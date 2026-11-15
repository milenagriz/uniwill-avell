//! The automated test point (spec #1): drive the daemon through its control API, over a private
//! peer-to-peer D-Bus connection, against a fake system port.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use avell::client::Client;
use avell::port::{Clock, GpuLoad, GpuProbe, Port, PowerProfiles, Sessions};

/// Stands in for power-profiles-daemon.
#[derive(Clone, Default)]
pub struct FakeProfiles {
    pub active: Arc<Mutex<String>>,
    pub sets: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl PowerProfiles for FakeProfiles {
    async fn active(&self) -> anyhow::Result<String> {
        Ok(self.active.lock().unwrap().clone())
    }
    async fn set_active(&self, profile: &str) -> anyhow::Result<()> {
        self.sets.lock().unwrap().push(profile.to_string());
        *self.active.lock().unwrap() = profile.to_string();
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

/// A clock that only moves when the test says so.
#[derive(Clone, Default)]
pub struct FakeClock {
    pub now: Arc<Mutex<Duration>>,
}

impl Clock for FakeClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
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
    pub client: Client,
    _server: zbus::Connection,
}

/// The uid of the test process, which is the caller the daemon sees.
pub fn my_uid() -> u32 {
    std::fs::metadata("/proc/self").map(|m| std::os::unix::fs::MetadataExt::uid(&m)).unwrap()
}

impl Rig {
    /// A machine laid out like this laptop: the patched driver's files under a temporary root.
    pub async fn start() -> Rig {
        let root = tempfile::tempdir().unwrap();
        let files = laptop_files();
        for (path, content) in &files {
            write(root.path(), path, content);
        }
        Self::start_with(root).await
    }

    pub async fn start_with(root: tempfile::TempDir) -> Rig {
        let profiles = FakeProfiles::default();
        *profiles.active.lock().unwrap() = "performance".into();
        let gpu = FakeGpu::default();
        let clock = FakeClock::default();
        let sessions = FakeSessions { active_local_user: Arc::new(Mutex::new(Some(my_uid()))) };
        let port = Port {
            root: root.path().to_path_buf(),
            profiles: Arc::new(profiles.clone()),
            gpu: Arc::new(gpu.clone()),
            clock: Arc::new(clock.clone()),
            sessions: Arc::new(sessions.clone()),
        };
        let (a, b) = std::os::unix::net::UnixStream::pair().unwrap();
        let (server, client) = tokio::join!(avell::api::serve_p2p(a, port), Client::connect_p2p(b));
        Rig { root, profiles, gpu, clock, sessions, client: client.unwrap(), _server: server.unwrap() }
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
