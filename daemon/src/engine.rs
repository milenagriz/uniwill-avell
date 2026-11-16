//! The engine: one task that owns the daemon's state and keeps the machine in line with the
//! active Power profile's bundle. Everything else talks to it through a [`Handle`].

use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use crate::config::Config;
use crate::port::Port;
use crate::status::{PLATFORM, RAPL};
use crate::sysfs::Sysfs;

const TICK: Duration = Duration::from_secs(1);
/// The two clocks are read a moment apart: a gap growing by less than this is not a suspend.
const RESUME_SLACK: Duration = Duration::from_millis(500);
/// Dynamic Boost is on while this service runs.
pub const DYNAMIC_BOOST_UNIT: &str = "nvidia-powerd.service";

type Reply<T> = oneshot::Sender<T>;

enum Command {
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
        self.config = config;
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

    /// The firmware's state: no TGP offset, and the CPU limits found at start.
    fn restore(&self) {
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
    let mut engine = Engine {
        config: Engine::load_config(&port),
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
