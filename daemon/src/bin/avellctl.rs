//! avellctl: command-line client of the avelld control API.

use avell::client::Client;
use avell::status::Status;

const USAGE: &str = "usage: avellctl [--json] <command>

commands:
  status                 the whole machine state
  profile                the active Power profile
  profile set <name>     power-saver, balanced or performance
  config                 the configuration in force (TOML)
  config set <key> <value>
                         change one setting, e.g. profiles.balanced.tgp_offset_w 15

--json prints machine-readable output for every command.";

fn opt<T: std::fmt::Display>(v: &Option<T>, unit: &str) -> String {
    v.as_ref().map(|v| format!("{v}{unit}")).unwrap_or_else(|| "unavailable".into())
}

fn print_status(s: &Status) {
    println!("Power profile    {}", s.profile);
    println!("Temperatures     CPU {}  GPU {}", opt(&s.temperatures.cpu_c, " °C"), opt(&s.temperatures.gpu_c, " °C"));
    let fans: Vec<String> = s.fans.fans.iter().enumerate()
        .map(|(i, f)| format!("fan{} {} rpm ({}/255)", i + 1, f.rpm, f.duty)).collect();
    println!("Fans             {}  {}", opt(&s.fans.mode, ""), if fans.is_empty() { "unavailable".into() } else { fans.join("  ") });
    let g = &s.gpu;
    let power = match (g.power_draw_w, g.power_limit_w) {
        (Some(d), Some(l)) => format!("  {d:.1} W of {l:.0} W, {}% busy", g.utilization_percent.unwrap_or(0)),
        _ => String::new(),
    };
    println!("GPU              {}  TGP offset {}{}", g.state, opt(&g.tgp_offset_w, " W"), power);
    println!("CPU limits       long-term {}  short-term {}", opt(&s.cpu.long_term_w, " W"), opt(&s.cpu.short_term_w, " W"));
    let b = &s.battery;
    println!("Battery          {}  {}  {}  cycles {}  health {}", opt(&b.capacity_percent, "%"), opt(&b.state, ""),
             match b.on_ac { Some(true) => "on AC", Some(false) => "on battery", None => "power source unavailable" },
             opt(&b.cycle_count, ""), opt(&b.health_percent, "%"));
    let k = &s.backlight;
    println!("Keyboard         {}{}", match (k.brightness, k.max_brightness) {
        (Some(b), Some(m)) => format!("brightness {b}/{m}"),
        _ => "unavailable".into(),
    }, k.color.map(|c| format!("  colour {} {} {}", c[0], c[1], c[2])).unwrap_or_default());
    if !s.unavailable.is_empty() {
        println!("Unavailable      {} (not exposed by the driver on this machine)", s.unavailable.join(", "));
    }
}

/// Turns D-Bus errors into messages a person can act on.
fn explain(e: anyhow::Error) -> anyhow::Error {
    let text = format!("{e:#}");
    if text.contains("ServiceUnknown") || text.contains("was not provided by any .service files") {
        anyhow::anyhow!("avelld is not running (check: systemctl status avelld)")
    } else if let Some((_, reason)) = text.rsplit_once("invalid configuration: ") {
        anyhow::anyhow!("invalid configuration: {reason}")
    } else if text.contains("AccessDenied") {
        anyhow::anyhow!("not allowed: {}", text.rsplit(':').next().unwrap_or(&text).trim())
    } else {
        e
    }
}

/// Sets one setting by its dotted key. The value is read as TOML (15, true), else as a string.
fn set_key(config: &mut toml::Table, key: &str, value: &str) -> anyhow::Result<()> {
    let mut parts: Vec<&str> = key.split('.').collect();
    let last = parts.pop().unwrap_or_default();
    let mut table = config;
    for part in parts {
        table = table.get_mut(part).and_then(toml::Value::as_table_mut)
            .ok_or_else(|| anyhow::anyhow!("no setting {key:?} (see: avellctl config)"))?;
    }
    anyhow::ensure!(table.contains_key(last) && !table[last].is_table(), "no setting {key:?} (see: avellctl config)");
    let parsed = format!("v = {value}").parse::<toml::Table>().ok().and_then(|mut t| t.remove("v"));
    table.insert(last.to_string(), parsed.unwrap_or_else(|| toml::Value::String(value.to_string())));
    Ok(())
}

async fn run(json: bool, args: &[String]) -> anyhow::Result<()> {
    let client = Client::connect_system().await.map_err(|e| explain(e.into()))?;
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["status"] => {
            let s = client.status().await.map_err(explain)?;
            if json { println!("{}", serde_json::to_string_pretty(&s)?) } else { print_status(&s) }
        }
        ["profile"] => {
            let p = client.status().await.map_err(explain)?.profile;
            if json { println!("{}", serde_json::json!({ "profile": p })) } else { println!("{p}") }
        }
        ["profile", "set", name] => {
            client.set_profile(name).await.map_err(explain)?;
            if json { println!("{}", serde_json::json!({ "profile": name })) }
        }
        ["config"] => {
            let text = client.config().await.map_err(explain)?;
            if json { println!("{}", serde_json::to_string_pretty(&text.parse::<toml::Table>()?)?) } else { print!("{text}") }
        }
        ["config", "set", key, value] => {
            let mut config: toml::Table = client.config().await.map_err(explain)?.parse()?;
            set_key(&mut config, key, value)?;
            client.set_config(&config.to_string()).await.map_err(explain)?;
            if json { println!("{}", serde_json::to_string_pretty(&config)?) }
        }
        _ => anyhow::bail!("{USAGE}"),
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    args.retain(|a| a != "--json");
    if let Err(e) = run(json, &args).await {
        if json {
            println!("{}", serde_json::json!({ "error": format!("{e:#}") }));
        } else {
            eprintln!("avellctl: {e:#}");
        }
        std::process::exit(1);
    }
}
