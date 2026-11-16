//! The engine: one task that owns the daemon's state and keeps the machine in line with the
//! active Power profile's bundle. Everything else talks to it through a [`Handle`].

use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use crate::config::{Config, FanMode};
use crate::policy;
use crate::port::Port;
use crate::status::{self, PLATFORM, RAPL, Status};
use crate::sysfs::Sysfs;

const TICK: Duration = Duration::from_secs(1);
/// How often the fan duty is written, changed or not: well inside the kernel watchdog's 5 s.
const FAN_BEAT: Duration = Duration::from_secs(2);
/// The two clocks are read a moment apart: a gap growing by less than this is not a suspend.
const RESUME_SLACK: Duration = Duration::from_millis(500);
/// Dynamic Boost is on while this service runs.
pub const DYNAMIC_BOOST_UNIT: &str = "nvidia-powerd.service";

type Reply<T> = oneshot::Sender<T>;

/// Who runs the fans right now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FanControl {
    /// The daemon, along the active profile's curve.
    Curve,
    /// The daemon, at one speed in percent.
    Fixed(u32),
    /// Max fan.
    Max,
    /// The firmware's own curve.
    Automatic,
}

impl FanControl {
    pub fn parse(mode: &str, percent: u32) -> anyhow::Result<FanControl> {
        match mode {
            "curve" => Ok(FanControl::Curve),
            "fixed" if percent <= 100 => Ok(FanControl::Fixed(percent)),
            "fixed" => anyhow::bail!("a fixed fan speed is 0 to 100 percent, not {percent}"),
            "max" => Ok(FanControl::Max),
            "automatic" => Ok(FanControl::Automatic),
            _ => anyhow::bail!("unknown fan mode {mode:?}: curve, fixed, max or automatic"),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FanControl::Curve => "curve",
            FanControl::Fixed(_) => "fixed",
            FanControl::Max => "max",
            FanControl::Automatic => "automatic",
        }
    }
}

impl From<FanMode> for FanControl {
    fn from(mode: FanMode) -> Self {
        match mode {
            FanMode::Curve => FanControl::Curve,
            FanMode::Automatic => FanControl::Automatic,
        }
    }
}

enum Command {
    Status(Reply<anyhow::Result<Status>>),
    SetFan(FanControl, Reply<()>),
    SetProfile(String, Reply<anyhow::Result<()>>),
    GetConfig(Reply<String>),
    SetConfig(String, Reply<anyhow::Result<()>>),
    Shutdown(Reply<()>),
}

#[derive(Clone)]
pub struct Handle {
    port: Port,
    tx: mpsc::Sender<Command>,
}

impl Handle {
    pub fn port(&self) -> &Port {
        &self.port
    }

    async fn ask<T>(&self, command: impl FnOnce(Reply<T>) -> Command) -> anyhow::Result<T> {
        let (tx, rx) = oneshot::channel();
        let stopped = || anyhow::anyhow!("the daemon is stopping");
        self.tx.send(command(tx)).await.map_err(|_| stopped())?;
        rx.await.map_err(|_| stopped())
    }

    /// The whole machine state.
    pub async fn status(&self) -> anyhow::Result<Status> {
        self.ask(Command::Status).await?
    }

    /// Hands the fans to the curve, a fixed speed, Max fan or the firmware.
    pub async fn set_fan(&self, control: FanControl) -> anyhow::Result<()> {
        self.ask(|r| Command::SetFan(control, r)).await
    }

    /// The configuration in force, as TOML.
    pub async fn config(&self) -> anyhow::Result<String> {
        self.ask(Command::GetConfig).await
    }

    /// Replaces the configuration. An invalid one is refused and the previous one stays.
    pub async fn set_config(&self, toml: &str) -> anyhow::Result<()> {
        self.ask(|r| Command::SetConfig(toml.to_string(), r)).await?
    }

    /// Stops the engine, leaving the machine in the firmware's state.
    pub async fn shutdown(&self) {
        let _ = self.ask(Command::Shutdown).await;
    }

    /// Sets the Power profile through the power-profile service and applies its bundle at once.
    pub async fn set_profile(&self, profile: &str) -> anyhow::Result<()> {
        self.ask(|r| Command::SetProfile(profile.to_string(), r)).await?
    }
}

/// Replaces the file in one step, so a crash never leaves half a configuration.
fn save(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("toml.new");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

struct Engine {
    port: Port,
    config: Config,
    /// The profile whose bundle was applied last.
    applied: Option<String>,
    /// How far boot time is ahead of monotonic time; it grows by the length of every suspend.
    suspended: Duration,
    /// The CPU power limits as they were when the daemon started.
    firmware_cpu_limits_uw: [Option<String>; 2],
    fan_control: FanControl,
    /// The speed last written, in percent.
    fan_target: Option<u32>,
    fan_state: policy::FanState,
    /// When the fan duty is due again.
    next_fan_beat: Duration,
}

fn cpu_limit(constraint: u32) -> String {
    format!("{RAPL}/constraint_{constraint}_power_limit_uw")
}

impl Engine {
    fn load_config(port: &Port) -> Config {
        match std::fs::read_to_string(&port.config_path) {
            Ok(text) => Config::parse(&text).unwrap_or_else(|e| {
                eprintln!("avelld: {} is invalid, using the defaults: {e:#}", port.config_path.display());
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    async fn set_config(&mut self, text: &str) -> anyhow::Result<()> {
        let config = Config::parse(text).map_err(|e| anyhow::anyhow!("invalid configuration: {e:#}"))?;
        save(&self.port.config_path, &config.to_toml())
            .map_err(|e| anyhow::anyhow!("saving {}: {e}", self.port.config_path.display()))?;
        if config.fans.mode != self.config.fans.mode {
            self.fan_control = config.fans.mode.into();
        }
        let fan_control = self.fan_control;
        self.config = config;
        self.set_fan(fan_control);
        if let Some(profile) = self.applied.clone() {
            self.apply(&profile).await;
        }
        Ok(())
    }

    fn suspended_now(&self) -> Duration {
        self.port.clock.boot_time().saturating_sub(self.port.clock.now())
    }

    async fn tick(&mut self) {
        let profile = match self.port.profiles.active().await {
            Ok(p) => p,
            Err(e) => return eprintln!("avelld: reading the Power profile: {e:#}"),
        };
        // the firmware resets its registers over a suspend
        let suspended = self.suspended_now();
        let resumed = suspended > self.suspended + RESUME_SLACK;
        self.suspended = suspended;
        if resumed || self.applied.as_deref() != Some(&profile) {
            self.apply(&profile).await;
        }
        if self.port.clock.now() >= self.next_fan_beat {
            self.fan_beat();
        }
    }

    /// The driver's fan files, when it exposes fan control.
    fn fan_files(&self) -> Option<String> {
        let fs = Sysfs::new(&self.port.root);
        fs.hwmon("uniwill").filter(|h| fs.exists(&format!("{h}/pwm1_enable")))
    }

    fn write_fan_mode(&self, hwmon: &str, mode: &str) {
        if let Err(e) = Sysfs::new(&self.port.root).write(&format!("{hwmon}/pwm1_enable"), mode) {
            eprintln!("avelld: {e:#}");
        }
    }

    /// Hands the fans over. Max fan and automatic are set once and then left alone; the curve
    /// and a fixed speed are written on every beat.
    fn set_fan(&mut self, control: FanControl) {
        self.fan_control = control;
        self.fan_state = policy::FanState::default();
        self.fan_target = None;
        let Some(hwmon) = self.fan_files() else { return };
        match control {
            FanControl::Max => self.write_fan_mode(&hwmon, "0"),
            FanControl::Automatic => self.write_fan_mode(&hwmon, "2"),
            FanControl::Curve | FanControl::Fixed(_) => {
                self.write_fan_mode(&hwmon, "1");
                self.fan_beat();
            }
        }
    }

    /// Computes the fan speed and writes it, changed or not: the kernel watchdog hands the fans
    /// back to the firmware when the writes stop.
    fn fan_beat(&mut self) {
        self.next_fan_beat = self.port.clock.now() + FAN_BEAT;
        let Some(hwmon) = self.fan_files() else { return };
        let fs = Sysfs::new(&self.port.root);
        let percent = match self.fan_control {
            FanControl::Automatic | FanControl::Max => return,
            FanControl::Fixed(percent) => percent,
            FanControl::Curve => {
                let temps = [1, 2].map(|n| fs.read_num::<i64>(&format!("{hwmon}/temp{n}_input")));
                let (Some(hottest), Some(bundle)) =
                    (temps.into_iter().flatten().max(), self.applied.as_deref().and_then(|p| self.config.bundle(p)))
                else {
                    return; // nothing to go by: the watchdog leaves the fans to the firmware
                };
                policy::fan_percent(&bundle.fan_curve, self.config.fans.hysteresis_c, hottest / 1000, &mut self.fan_state)
            }
        };
        match fs.read(&format!("{hwmon}/pwm1_enable")).as_deref() {
            Some("1") => {}
            // Max fan turned on by someone else (the Fn key, another program): theirs until released
            Some("0") => {
                self.fan_target = None;
                return;
            }
            // automatic: the watchdog fired while the daemon was stalled, or the machine resumed
            _ => self.write_fan_mode(&hwmon, "1"),
        }
        let duty = policy::duty(percent).to_string();
        for n in [1, 2] {
            if let Err(e) = fs.write(&format!("{hwmon}/pwm{n}"), &duty) {
                eprintln!("avelld: {e:#}");
            }
        }
        self.fan_target = Some(percent);
    }

    /// The machine state, with what only the engine knows added.
    async fn status(&self) -> anyhow::Result<Status> {
        let mut s = status::read(&self.port).await?;
        if s.fans.mode.is_some() {
            s.fans.control = Some(self.fan_control.name().to_string());
            s.fans.target_percent = self.fan_target;
        }
        Ok(s)
    }

    /// Applies a profile's bundle. A feature the driver does not expose is skipped.
    async fn apply(&mut self, profile: &str) {
        self.applied = Some(profile.to_string());
        let Some(bundle) = self.config.bundle(profile).cloned() else {
            return eprintln!("avelld: no bundle for Power profile {profile:?}");
        };
        let uw = |w: u32| Some((u64::from(w) * 1_000_000).to_string());
        self.write_power(bundle.tgp_offset_w, [uw(bundle.cpu_long_term_w), uw(bundle.cpu_short_term_w)]);
        if let Err(e) = self.port.services.set_running(DYNAMIC_BOOST_UNIT, bundle.dynamic_boost).await {
            eprintln!("avelld: Dynamic Boost ({DYNAMIC_BOOST_UNIT}): {e:#}");
        }
        // a new curve starts afresh
        self.fan_state = policy::FanState::default();
        self.fan_beat();
    }

    /// Writes the TGP offset (watts) and the two CPU power limits (microwatts, as the kernel
    /// takes them), each only where the kernel exposes the file.
    fn write_power(&self, tgp_offset_w: u32, cpu_limits_uw: [Option<String>; 2]) {
        let fs = Sysfs::new(&self.port.root);
        let [long, short] = cpu_limits_uw;
        let writes = [(format!("{PLATFORM}/ctgp_offset"), Some(tgp_offset_w.to_string())), (cpu_limit(0), long), (cpu_limit(1), short)];
        for (file, value) in &writes {
            if let Some(value) = value
                && fs.exists(file)
                    && let Err(e) = fs.write(file, value) {
                        eprintln!("avelld: {e:#}");
                    }
        }
    }

    /// The firmware's state: fans on automatic, no TGP offset, and the CPU limits found at start.
    fn restore(&self) {
        if let Some(hwmon) = self.fan_files() {
            self.write_fan_mode(&hwmon, "2");
        }
        self.write_power(0, self.firmware_cpu_limits_uw.clone());
    }

    async fn run(mut self, mut rx: mpsc::Receiver<Command>) {
        let clock = self.port.clock.clone();
        let mut next = clock.now() + TICK;
        loop {
            tokio::select! {
                biased;
                _ = clock.sleep_until(next) => {
                    self.tick().await;
                    next = clock.now() + TICK;
                }
                command = rx.recv() => match command {
                    None => break,
                    Some(Command::Shutdown(reply)) => {
                        self.restore();
                        let _ = reply.send(());
                        break;
                    }
                    Some(Command::Status(reply)) => {
                        let _ = reply.send(self.status().await);
                    }
                    Some(Command::SetFan(control, reply)) => {
                        self.set_fan(control);
                        let _ = reply.send(());
                    }
                    Some(Command::GetConfig(reply)) => {
                        let _ = reply.send(self.config.to_toml());
                    }
                    Some(Command::SetConfig(text, reply)) => {
                        let _ = reply.send(self.set_config(&text).await);
                    }
                    Some(Command::SetProfile(profile, reply)) => {
                        let result = self.port.profiles.set_active(&profile).await;
                        if result.is_ok() {
                            self.apply(&profile).await;
                        }
                        let _ = reply.send(result);
                    }
                },
            }
        }
    }
}

/// Starts the engine: applies the active profile's bundle, then keeps watching.
pub async fn start(port: Port) -> anyhow::Result<Handle> {
    let fs = Sysfs::new(&port.root);
    let config = Engine::load_config(&port);
    let mut engine = Engine {
        fan_control: config.fans.mode.into(),
        fan_target: None,
        fan_state: policy::FanState::default(),
        next_fan_beat: Duration::ZERO,
        config,
        firmware_cpu_limits_uw: [fs.read(&cpu_limit(0)), fs.read(&cpu_limit(1))],
        port: port.clone(),
        applied: None,
        suspended: Duration::ZERO,
    };
    engine.suspended = engine.suspended_now();
    engine.tick().await;
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(engine.run(rx));
    Ok(Handle { port, tx })
}
