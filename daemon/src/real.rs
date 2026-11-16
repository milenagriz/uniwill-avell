//! The real system port: this machine's kernel files, power-profiles-daemon, nvidia-smi, logind.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::port::{Clock, GpuLoad, GpuProbe, Port, PowerProfiles, Services, Sessions};

/// power-profiles-daemon on the system bus (its current name, then the legacy one).
pub struct Ppd {
    conn: zbus::Connection,
}

const PPD: [(&str, &str, &str); 2] = [
    ("org.freedesktop.UPower.PowerProfiles", "/org/freedesktop/UPower/PowerProfiles", "org.freedesktop.UPower.PowerProfiles"),
    ("net.hadess.PowerProfiles", "/net/hadess/PowerProfiles", "net.hadess.PowerProfiles"),
];

impl Ppd {
    async fn proxy(&self) -> anyhow::Result<zbus::Proxy<'static>> {
        let mut last = None;
        for (dest, path, iface) in PPD {
            match zbus::Proxy::new(&self.conn, dest, path, iface).await {
                Ok(p) => match p.get_property::<String>("ActiveProfile").await {
                    Ok(_) => return Ok(p),
                    Err(e) => last = Some(e),
                },
                Err(e) => last = Some(e),
            }
        }
        Err(anyhow::anyhow!("power-profiles-daemon is not reachable: {}", last.map(|e| e.to_string()).unwrap_or_default()))
    }
}

#[async_trait::async_trait]
impl PowerProfiles for Ppd {
    async fn active(&self) -> anyhow::Result<String> {
        Ok(self.proxy().await?.get_property("ActiveProfile").await?)
    }
    async fn set_active(&self, profile: &str) -> anyhow::Result<()> {
        Ok(self.proxy().await?.set_property("ActiveProfile", profile).await?)
    }
}

/// Asks the NVIDIA driver; only ever called when the kernel has the card awake.
pub struct NvidiaSmi;

#[async_trait::async_trait]
impl GpuProbe for NvidiaSmi {
    async fn query(&self) -> anyhow::Result<GpuLoad> {
        let out = tokio::process::Command::new("nvidia-smi")
            .args(["--query-gpu=utilization.gpu,power.draw,enforced.power.limit", "--format=csv,noheader,nounits"])
            .output()
            .await?;
        anyhow::ensure!(out.status.success(), "nvidia-smi failed");
        let line = String::from_utf8_lossy(&out.stdout);
        let v: Vec<&str> = line.trim().split(',').map(str::trim).collect();
        anyhow::ensure!(v.len() == 3, "unexpected nvidia-smi output: {line}");
        Ok(GpuLoad {
            utilization_percent: v[0].parse().unwrap_or(0),
            power_draw_w: v[1].parse().unwrap_or(0.0),
            power_limit_w: v[2].parse().unwrap_or(0.0),
        })
    }
}

pub struct MonotonicClock(Instant);

#[async_trait::async_trait]
impl Clock for MonotonicClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
    fn boot_time(&self) -> Duration {
        // /proc/uptime counts suspended time; Instant (CLOCK_MONOTONIC) does not
        std::fs::read_to_string("/proc/uptime").ok()
            .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
            .map(Duration::from_secs_f64)
            .unwrap_or_else(|| self.now())
    }
    async fn sleep_until(&self, deadline: Duration) {
        tokio::time::sleep(deadline.saturating_sub(self.now())).await
    }
}

/// systemd on the system bus.
pub struct Systemd {
    conn: zbus::Connection,
}

#[async_trait::async_trait]
impl Services for Systemd {
    async fn set_running(&self, unit: &str, running: bool) -> anyhow::Result<()> {
        let mgr = zbus::Proxy::new(&self.conn, "org.freedesktop.systemd1", "/org/freedesktop/systemd1",
                                   "org.freedesktop.systemd1.Manager").await?;
        let method = if running { "StartUnit" } else { "StopUnit" };
        let _job: zbus::OwnedObjectPath = mgr.call(method, &(unit, "replace")).await?;
        Ok(())
    }
}

/// logind: a caller may change settings when it owns an active, local, graphical-seat session.
pub struct Logind {
    conn: zbus::Connection,
}

#[async_trait::async_trait]
impl Sessions for Logind {
    async fn is_active_local_user(&self, uid: u32) -> anyhow::Result<bool> {
        let mgr = zbus::Proxy::new(&self.conn, "org.freedesktop.login1", "/org/freedesktop/login1",
                                   "org.freedesktop.login1.Manager").await?;
        let sessions: Vec<(String, u32, String, String, zbus::OwnedObjectPath)> =
            mgr.call("ListSessions", &()).await?;
        for (_, suid, _, seat, path) in sessions {
            if suid != uid || seat.is_empty() {
                continue;
            }
            let s = zbus::Proxy::new(&self.conn, "org.freedesktop.login1", path, "org.freedesktop.login1.Session").await?;
            let active: bool = s.get_property("Active").await?;
            let remote: bool = s.get_property("Remote").await?;
            let class: String = s.get_property("Class").await?;
            if active && !remote && class == "user" {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// The port for this machine, with the kernel files under `root` (normally "/").
pub async fn port(conn: &zbus::Connection, root: std::path::PathBuf) -> Port {
    Port {
        profiles: Arc::new(Ppd { conn: conn.clone() }),
        gpu: Arc::new(NvidiaSmi),
        clock: Arc::new(MonotonicClock(Instant::now())),
        sessions: Arc::new(Logind { conn: conn.clone() }),
        services: Arc::new(Systemd { conn: conn.clone() }),
        config_path: root.join("etc/avelld/config.toml"),
        root,
    }
}
