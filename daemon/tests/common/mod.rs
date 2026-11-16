//! The automated test point (spec #1): drive the daemon through its control API, over a private
//! peer-to-peer D-Bus connection, against a fake system port.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use avell::client::Client;
use avell::port::{Clock, GpuLoad, GpuProbe, Port, PowerProfiles, Services, Sessions};

/// Stands in for power-profiles-daemon.
#[derive(Clone, Default)]
pub struct FakeProfiles {
    pub active: Arc<Mutex<String>>,
    pub sets: Arc<Mutex<Vec<String>>>,
    /// Holds in force: (cookie, reason, the profile to return to).
    pub holds: Arc<Mutex<Vec<(u32, String, String)>>>,
}

impl FakeProfiles {
    /// A choice in the power menu. As in power-profiles-daemon, it cancels every hold.
    pub fn choose(&self, profile: &str) {
        self.holds.lock().unwrap().clear();
        *self.active.lock().unwrap() = profile.to_string();
    }
}

#[async_trait::async_trait]
impl PowerProfiles for FakeProfiles {
    async fn active(&self) -> anyhow::Result<String> {
        Ok(self.active.lock().unwrap().clone())
    }
    async fn set_active(&self, profile: &str) -> anyhow::Result<()> {
        self.sets.lock().unwrap().push(profile.to_string());
        self.choose(profile);
        Ok(())
    }
    async fn hold(&self, profile: &str, reason: &str) -> anyhow::Result<u32> {
        let mut holds = self.holds.lock().unwrap();
        let cookie = holds.len() as u32 + 1;
        let mut active = self.active.lock().unwrap();
        holds.push((cookie, reason.to_string(), active.clone()));
        *active = profile.to_string();
        Ok(cookie)
    }
    async fn release(&self, cookie: u32) -> anyhow::Result<()> {
        let mut holds = self.holds.lock().unwrap();
        if let Some(i) = holds.iter().position(|h| h.0 == cookie) {
            let (_, _, previous) = holds.remove(i);
            *self.active.lock().unwrap() = previous;
        }
        Ok(())
    }
}

/// Stands in for the graphics card's load query (nvidia-smi); counts how often it is asked.
#[derive(Clone, Default)]
pub struct FakeGpu {
    pub load: Arc<Mutex<Option<GpuLoad>>>,
    pub queries: Arc<Mutex<u32>>,
}

#[async_trait::async_trait]
impl GpuProbe for FakeGpu {
    async fn query(&self) -> anyhow::Result<GpuLoad> {
        *self.queries.lock().unwrap() += 1;
        Ok(self.load.lock().unwrap().clone().unwrap_or_default())
    }
}

/// A clock that only moves when the test says so. `Rig::advance` returns once the daemon has
/// handled everything due and is waiting for a later deadline.
#[derive(Clone)]
pub struct FakeClock {
    /// (monotonic, boot time): boot time also moves while "suspended".
    pub time: tokio::sync::watch::Sender<(Duration, Duration)>,
    /// (times the daemon went idle, the deadline it is waiting for)
    pub idle: tokio::sync::watch::Sender<(u64, Duration)>,
}

impl Default for FakeClock {
    fn default() -> Self {
        FakeClock {
            time: tokio::sync::watch::channel((Duration::ZERO, Duration::ZERO)).0,
            idle: tokio::sync::watch::channel((0, Duration::ZERO)).0,
        }
    }
}

#[async_trait::async_trait]
impl Clock for FakeClock {
    fn now(&self) -> Duration {
        self.time.borrow().0
    }
    fn boot_time(&self) -> Duration {
        self.time.borrow().1
    }
    async fn sleep_until(&self, deadline: Duration) {
        let mut rx = self.time.subscribe();
        loop {
            if rx.borrow_and_update().0 >= deadline {
                return;
            }
            self.idle.send_modify(|i| *i = (i.0 + 1, deadline));
            if rx.changed().await.is_err() {
                return std::future::pending().await;
            }
        }
    }
}

/// Stands in for systemd: which units the daemon started or stopped.
#[derive(Clone, Default)]
pub struct FakeServices {
    pub running: Arc<Mutex<BTreeMap<String, bool>>>,
}

#[async_trait::async_trait]
impl Services for FakeServices {
    async fn set_running(&self, unit: &str, running: bool) -> anyhow::Result<()> {
        self.running.lock().unwrap().insert(unit.to_string(), running);
        Ok(())
    }
}

/// Stands in for logind: which user owns the active local desktop session.
#[derive(Clone)]
pub struct FakeSessions {
    pub active_local_user: Arc<Mutex<Option<u32>>>,
}

#[async_trait::async_trait]
impl Sessions for FakeSessions {
    async fn is_active_local_user(&self, uid: u32) -> anyhow::Result<bool> {
        Ok(*self.active_local_user.lock().unwrap() == Some(uid))
    }
}

pub struct Rig {
    pub root: tempfile::TempDir,
    pub profiles: FakeProfiles,
    pub gpu: FakeGpu,
    pub clock: FakeClock,
    pub sessions: FakeSessions,
    pub services: FakeServices,
    pub client: Client,
    pub daemon: avell::engine::Handle,
    _server: zbus::Connection,
}

/// The uid of the test process, which is the caller the daemon sees.
pub fn my_uid() -> u32 {
    std::fs::metadata("/proc/self").map(|m| std::os::unix::fs::MetadataExt::uid(&m)).unwrap()
}

impl Rig {
    /// A machine laid out like this laptop: the patched driver's files under a temporary root.
    pub async fn start() -> Rig {
        Self::start_with(Self::laptop()).await
    }

    /// A temporary root laid out like this laptop, with the test configuration.
    pub fn laptop() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for (path, content) in &laptop_files() {
            write(root.path(), path, content);
        }
        write(root.path(), "etc/avelld/config.toml", TEST_CONFIG);
        root
    }

    pub async fn start_with(root: tempfile::TempDir) -> Rig {
        let profiles = FakeProfiles::default();
        *profiles.active.lock().unwrap() = "performance".into();
        let gpu = FakeGpu::default();
        let clock = FakeClock::default();
        let services = FakeServices::default();
        let sessions = FakeSessions { active_local_user: Arc::new(Mutex::new(Some(my_uid()))) };
        let port = Port {
            root: root.path().to_path_buf(),
            profiles: Arc::new(profiles.clone()),
            gpu: Arc::new(gpu.clone()),
            clock: Arc::new(clock.clone()),
            sessions: Arc::new(sessions.clone()),
            services: Arc::new(services.clone()),
            config_path: root.path().join("etc/avelld/config.toml"),
        };
        let daemon = avell::engine::start(port).await.unwrap();
        let (a, b) = std::os::unix::net::UnixStream::pair().unwrap();
        let (server, client) = tokio::join!(avell::api::serve_p2p(a, daemon.clone()), Client::connect_p2p(b));
        let rig = Rig { root, profiles, gpu, clock, sessions, services, client: client.unwrap(), daemon, _server: server.unwrap() };
        rig.advance(Duration::ZERO).await;
        rig
    }

    /// Moves the clock on and returns when the daemon has caught up.
    pub async fn advance(&self, by: Duration) {
        let target = self.clock.now() + by;
        self.clock.time.send_modify(|t| *t = (t.0 + by, t.1 + by));
        let mut rx = self.clock.idle.subscribe();
        loop {
            if rx.borrow_and_update().1 > target {
                return;
            }
            rx.changed().await.unwrap();
        }
    }

    /// Suspends the machine for a while (boot time moves, monotonic time does not), then lets
    /// the daemon run for a second after resume.
    pub async fn suspend(&self, for_: Duration) {
        self.clock.time.send_modify(|t| t.1 += for_);
        self.advance(Duration::from_secs(1)).await;
    }

    /// The bundle file as a test wants it, before start.
    pub fn config_rel() -> &'static str {
        "etc/avelld/config.toml"
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.path().join(rel)
    }

    pub fn write(&self, rel: &str, content: &str) {
        write(self.root.path(), rel, content);
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.path(rel)).unwrap().trim().to_string()
    }

    pub fn remove(&self, rel: &str) {
        std::fs::remove_file(self.path(rel)).unwrap();
    }
}

pub fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, format!("{content}\n")).unwrap();
}

pub const HWMON: &str = "sys/class/hwmon/hwmon8";
pub const PLATFORM: &str = "sys/bus/platform/devices/INOU0000:00";
pub const GPU_PM: &str = "sys/bus/pci/devices/0000:01:00.0/power/runtime_status";
pub const BAT: &str = "sys/class/power_supply/BAT0";
pub const AC: &str = "sys/class/power_supply/AC0";
pub const KBD: &str = "sys/class/leds/uniwill:multicolor:kbd_backlight";
pub const RAPL: &str = "sys/class/powercap/intel-rapl:0";

/// The control files this laptop exposes with the patched driver (values as observed on it).
pub fn laptop_files() -> BTreeMap<&'static str, String> {
    let mut f = BTreeMap::new();
    let h = |n: &str| format!("{HWMON}/{n}");
    for (k, v) in [("name", "uniwill"), ("temp1_input", "58000"), ("temp1_label", "CPU"), ("temp2_input", "45000"),
                   ("temp2_label", "GPU"), ("fan1_input", "2274"), ("fan2_input", "1622"), ("pwm1", "76"),
                   ("pwm2", "70"), ("pwm1_enable", "2"), ("pwm2_enable", "2")] {
        f.insert(Box::leak(h(k).into_boxed_str()) as &str, v.to_string());
    }
    f.insert(Box::leak(format!("{PLATFORM}/ctgp_offset").into_boxed_str()), "0".into());
    f.insert(Box::leak(format!("{PLATFORM}/battery_cycle_count").into_boxed_str()), "308".into());
    f.insert(GPU_PM, "suspended".into());
    for (k, v) in [("status", "Full"), ("capacity", "100"), ("charge_full", "2900000"), ("charge_full_design", "4100000"),
                   ("cycle_count", "0")] {
        f.insert(Box::leak(format!("{BAT}/{k}").into_boxed_str()), v.into());
    }
    f.insert(Box::leak(format!("{AC}/online").into_boxed_str()), "1".into());
    for (k, v) in [("brightness", "1"), ("max_brightness", "4"), ("multi_intensity", "0 50 50"), ("multi_index", "red green blue")] {
        f.insert(Box::leak(format!("{KBD}/{k}").into_boxed_str()), v.into());
    }
    f.insert(Box::leak(format!("{RAPL}/constraint_0_power_limit_uw").into_boxed_str()), "120000000".into());
    f.insert(Box::leak(format!("{RAPL}/constraint_1_power_limit_uw").into_boxed_str()), "120000000".into());
    f.insert("sys/firmware/acpi/platform_profile", "performance".into());
    f
}

/// Bundles with values that differ per profile, so a test can tell which one was applied.
pub const TEST_CONFIG: &str = r#"
[fans]
hysteresis_c = 5

[game_detection]
enabled = true
load_percent = 50
hold_after_s = 10
release_after_s = 30

[charger]
enabled = true
on_ac = "performance"
on_battery = "power-saver"

[profiles.power-saver]
tgp_offset_w = 0
dynamic_boost = false
cpu_long_term_w = 20
cpu_short_term_w = 30
fan_curve = [[40, 0], [80, 100]]

[profiles.balanced]
tgp_offset_w = 10
dynamic_boost = true
cpu_long_term_w = 45
cpu_short_term_w = 65

[profiles.performance]
tgp_offset_w = 20
dynamic_boost = true
cpu_long_term_w = 80
cpu_short_term_w = 80
fan_curve = [[50, 20], [70, 60], [90, 100]]
"#;

pub fn watts(rig: &Rig, constraint: u32) -> u64 {
    rig.read(&format!("{RAPL}/constraint_{constraint}_power_limit_uw")).parse::<u64>().unwrap() / 1_000_000
}
