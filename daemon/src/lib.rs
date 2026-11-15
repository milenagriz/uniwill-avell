//! The Avell A52 MOB control stack: the daemon (`avelld`) and its CLI (`avellctl`).
//!
//! All I/O goes through one system port ([`port::Port`]); the control API ([`api`]) is the
//! system-bus service `org.avell.Control1`, and [`client`] is the one client of it, used by the
//! CLI and by the tests.

pub mod api;
pub mod client;
pub mod port;
pub mod real;
pub mod status;
pub mod sysfs;

/// Well-known bus name and object path of the control API.
pub const BUS_NAME: &str = "org.avell.Control1";
pub const OBJECT_PATH: &str = "/org/avell/Control1";
