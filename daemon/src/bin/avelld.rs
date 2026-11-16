//! avelld: the Avell A52 MOB control daemon (runs as root, started by systemd).

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = std::env::var_os("AVELLD_ROOT").map(Into::into).unwrap_or_else(|| "/".into());
    let conn = zbus::Connection::system().await?;
    let daemon = avell::engine::start(avell::real::port(&conn, root).await).await?;
    avell::api::serve_system(&conn, daemon.clone()).await?;
    eprintln!("avelld: serving {} on the system bus", avell::BUS_NAME);

    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        _ = term.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    eprintln!("avelld: stopping");
    daemon.shutdown().await;
    Ok(())
}
