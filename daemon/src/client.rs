//! The one client of the control API (used by avellctl and by the tests).

use zbus::proxy;

use crate::status::Status;

#[proxy(interface = "org.avell.Control1", default_service = "org.avell.Control1", default_path = "/org/avell/Control1")]
trait Control {
    fn get_status(&self) -> zbus::Result<String>;
    fn set_profile(&self, profile: &str) -> zbus::Result<()>;
    fn set_fan(&self, mode: &str, percent: u32) -> zbus::Result<()>;
    fn get_config(&self) -> zbus::Result<String>;
    fn set_config(&self, toml: &str) -> zbus::Result<()>;
}

pub struct Client {
    proxy: ControlProxy<'static>,
}

impl Client {
    /// Connects to the daemon on the system bus.
    pub async fn connect_system() -> zbus::Result<Client> {
        let conn = zbus::Connection::system().await?;
        Ok(Client { proxy: ControlProxy::new(&conn).await? })
    }

    /// Connects over a private peer-to-peer connection (tests).
    pub async fn connect_p2p(stream: std::os::unix::net::UnixStream) -> zbus::Result<Client> {
        let conn = zbus::connection::Builder::unix_stream(stream).p2p().build().await?;
        let proxy = ControlProxy::builder(&conn).destination(crate::BUS_NAME).build().await?;
        Ok(Client { proxy })
    }

    pub async fn set_profile(&self, profile: &str) -> anyhow::Result<()> {
        Ok(self.proxy.set_profile(profile).await?)
    }

    /// Hands the fans to "curve", "fixed" (with a percentage), "max" or "automatic".
    pub async fn set_fan(&self, mode: &str, percent: u32) -> anyhow::Result<()> {
        Ok(self.proxy.set_fan(mode, percent).await?)
    }

    /// The configuration in force, as TOML.
    pub async fn config(&self) -> anyhow::Result<String> {
        Ok(self.proxy.get_config().await?)
    }

    /// Replaces the configuration. An invalid one is refused and nothing changes.
    pub async fn set_config(&self, toml: &str) -> anyhow::Result<()> {
        Ok(self.proxy.set_config(toml).await?)
    }

    pub async fn status(&self) -> anyhow::Result<Status> {
        Ok(serde_json::from_str(&self.proxy.get_status().await?)?)
    }
}
