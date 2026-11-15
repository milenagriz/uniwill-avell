//! The system port: everything the daemon reads from or does to the machine.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The desktop's power-profile service (power-profiles-daemon), the single source of truth for
/// the active Power profile.
#[async_trait::async_trait]
pub trait PowerProfiles: Send + Sync {
    async fn active(&self) -> anyhow::Result<String>;
    async fn set_active(&self, profile: &str) -> anyhow::Result<()>;
}

/// What the graphics card reports when it is awake.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GpuLoad {
    pub utilization_percent: u32,
    pub power_draw_w: f64,
    pub power_limit_w: f64,
}

/// Queries the graphics card. Waking it is up to the caller: only call this when the kernel says
/// the card is already awake.
#[async_trait::async_trait]
pub trait GpuProbe: Send + Sync {
    async fn query(&self) -> anyhow::Result<GpuLoad>;
}

/// Monotonic time since an arbitrary start.
pub trait Clock: Send + Sync {
    fn now(&self) -> Duration;
}

/// Who is allowed to change things: the user of the active local desktop session.
#[async_trait::async_trait]
pub trait Sessions: Send + Sync {
    async fn is_active_local_user(&self, uid: u32) -> anyhow::Result<bool>;
}

/// The whole port. `root` is where the kernel's control files live (`/` on the real machine).
#[derive(Clone)]
pub struct Port {
    pub root: PathBuf,
    pub profiles: Arc<dyn PowerProfiles>,
    pub gpu: Arc<dyn GpuProbe>,
    pub clock: Arc<dyn Clock>,
    pub sessions: Arc<dyn Sessions>,
}
