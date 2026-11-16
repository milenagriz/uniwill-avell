//! The control API: `org.avell.Control1`.

use zbus::interface;
use zbus::message::Header;

use crate::engine::Handle;
use crate::status;

pub struct Control {
    daemon: Handle,
}

fn failed(e: anyhow::Error) -> zbus::fdo::Error {
    zbus::fdo::Error::Failed(format!("{e:#}"))
}

/// The uid of the process that sent a method call: from the socket on a peer-to-peer connection,
/// from the bus daemon otherwise.
async fn caller_uid(conn: &zbus::Connection, hdr: &Header<'_>) -> zbus::fdo::Result<u32> {
    let uid = if conn.is_bus() {
        let sender = hdr.sender().ok_or_else(|| zbus::fdo::Error::AccessDenied("no sender".into()))?;
        zbus::fdo::DBusProxy::new(conn).await?.get_connection_unix_user(sender.clone().into()).await?
    } else {
        conn.peer_creds().await.ok().and_then(|c| c.unix_user_id())
            .ok_or_else(|| zbus::fdo::Error::AccessDenied("caller credentials unknown".into()))?
    };
    Ok(uid)
}

impl Control {
    /// Changes are allowed for the user of the active local desktop session only.
    async fn authorize(&self, conn: &zbus::Connection, hdr: &Header<'_>) -> zbus::fdo::Result<()> {
        let uid = caller_uid(conn, hdr).await?;
        if self.daemon.port().sessions.is_active_local_user(uid).await.map_err(failed)? {
            Ok(())
        } else {
            Err(zbus::fdo::Error::AccessDenied(format!(
                "uid {uid} is not the user of the active local desktop session"
            )))
        }
    }
}

#[interface(name = "org.avell.Control1")]
impl Control {
    /// The whole machine state, as JSON.
    async fn get_status(&self) -> zbus::fdo::Result<String> {
        let s = status::read(self.daemon.port()).await.map_err(failed)?;
        serde_json::to_string(&s).map_err(|e| failed(e.into()))
    }

    /// Sets the Power profile through the desktop's power-profile service
    /// ("power-saver", "balanced" or "performance").
    async fn set_profile(
        &self,
        profile: &str,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
    ) -> zbus::fdo::Result<()> {
        self.authorize(conn, &hdr).await?;
        self.daemon.set_profile(profile).await.map_err(failed)
    }

    /// The configuration in force, as TOML.
    async fn get_config(&self) -> zbus::fdo::Result<String> {
        self.daemon.config().await.map_err(failed)
    }

    /// Replaces the configuration (TOML). An invalid one is refused with the reason and the
    /// previous one stays in force; a valid one is saved and applied at once.
    async fn set_config(
        &self,
        toml: &str,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
    ) -> zbus::fdo::Result<()> {
        self.authorize(conn, &hdr).await?;
        self.daemon.set_config(toml).await.map_err(|e| zbus::fdo::Error::InvalidArgs(format!("{e:#}")))
    }
}

/// Serves the control API on the system bus under its well-known name.
pub async fn serve_system(conn: &zbus::Connection, daemon: Handle) -> anyhow::Result<()> {
    conn.object_server().at(crate::OBJECT_PATH, Control { daemon }).await?;
    conn.request_name(crate::BUS_NAME).await?;
    Ok(())
}

/// Serves the control API on one end of a private peer-to-peer connection.
pub async fn serve_p2p(stream: std::os::unix::net::UnixStream, daemon: Handle) -> zbus::Result<zbus::Connection> {
    zbus::connection::Builder::unix_stream(stream)
        .server(zbus::Guid::generate())
        .p2p()
        .serve_at(crate::OBJECT_PATH, Control { daemon })
        .build()
        .await
}
