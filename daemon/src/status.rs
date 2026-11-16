//! The machine state the control API reports.

use serde::{Deserialize, Serialize};

use crate::port::Port;
use crate::sysfs::Sysfs;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Status {
    /// The active Power profile, as the power-profile service names it.
    pub profile: String,
    /// Why the daemon is holding the performance profile, when it is.
    #[serde(default)]
    pub hold: Option<String>,
    pub temperatures: Temperatures,
    pub fans: Fans,
    pub gpu: Gpu,
    pub battery: Battery,
    pub cpu: CpuLimits,
    pub backlight: Backlight,
    /// Features this machine's driver does not expose (not yet verified on this board, or the
    /// stock kernel driver is loaded): "temperatures", "fan_control", "tgp_offset", "gpu_state",
    /// "battery", "charging_profile", "charge_cap", "cpu_power_limits", "keyboard_backlight".
    pub unavailable: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CpuLimits {
    /// Package power limits from the kernel's power-capping interface (PL1 / PL2), in watts.
    pub long_term_w: Option<u32>,
    pub short_term_w: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Backlight {
    pub brightness: Option<u32>,
    pub max_brightness: Option<u32>,
    /// Red, green, blue, each 0-255 (None on a single-colour keyboard).
    pub color: Option<[u32; 3]>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Battery {
    pub capacity_percent: Option<u32>,
    /// The kernel's battery status: Charging, Discharging, Full, Not charging.
    pub state: Option<String>,
    pub on_ac: Option<bool>,
    pub cycle_count: Option<u32>,
    /// Full-charge capacity as a share of the design capacity.
    pub health_percent: Option<u32>,
    /// The firmware's charging profile: "high-capacity", "balanced" or "stationary".
    #[serde(default)]
    pub charging_profile: Option<String>,
    /// The level charging stops at, in percent.
    #[serde(default)]
    pub charge_cap_percent: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gpu {
    /// "asleep" (runtime-suspended: never queried), "awake", or "unknown".
    pub state: String,
    /// None when the driver does not expose the TGP offset.
    pub tgp_offset_w: Option<u32>,
    pub power_draw_w: Option<f64>,
    pub power_limit_w: Option<f64>,
    pub utilization_percent: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Temperatures {
    pub cpu_c: Option<i64>,
    pub gpu_c: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fans {
    /// "max" (Max fan), "manual" or "automatic"; None when the driver does not expose fan control.
    pub mode: Option<String>,
    /// Who runs the fans: "curve" or "fixed" (the daemon), "max" (Max fan) or "automatic" (the
    /// firmware). None when the driver does not expose fan control.
    #[serde(default)]
    pub control: Option<String>,
    /// The speed the daemon last asked for, in percent, under "curve" and "fixed".
    #[serde(default)]
    pub target_percent: Option<u32>,
    pub fans: Vec<Fan>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fan {
    pub rpm: u32,
    /// 0-255, as the driver reports it.
    pub duty: u32,
}

/// The kernel's control files this module reads, relative to the port's root.
pub const PLATFORM: &str = "sys/bus/platform/devices/INOU0000:00";
pub const GPU_RUNTIME_STATUS: &str = "sys/bus/pci/devices/0000:01:00.0/power/runtime_status";
pub const BATTERY: &str = "sys/class/power_supply/BAT0";
pub const AC: &str = "sys/class/power_supply/AC0";

pub const RAPL: &str = "sys/class/powercap/intel-rapl:0";
pub const KBD_LED: &str = "sys/class/leds/uniwill:multicolor:kbd_backlight";

fn cpu(fs: &Sysfs<'_>) -> CpuLimits {
    let w = |n: u32| fs.read_num::<u64>(&format!("{RAPL}/constraint_{n}_power_limit_uw")).map(|uw| (uw / 1_000_000) as u32);
    CpuLimits { long_term_w: w(0), short_term_w: w(1) }
}

/// Three numbers from an LED file such as `multi_intensity`.
fn triple(fs: &Sysfs<'_>, file: &str) -> Option<[u32; 3]> {
    let v: Vec<u32> = fs.read(&format!("{KBD_LED}/{file}"))?.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    v.try_into().ok()
}

/// The most each colour channel takes on this keyboard (255 when the kernel does not say).
fn max_intensity(fs: &Sysfs<'_>) -> [u32; 3] {
    triple(fs, "multi_max_intensity").filter(|m| m.iter().all(|&c| c > 0)).unwrap_or([255; 3])
}

fn backlight(fs: &Sysfs<'_>) -> Backlight {
    let max = max_intensity(fs);
    let color = triple(fs, "multi_intensity").map(|raw| [0, 1, 2].map(|i| (raw[i].min(max[i]) * 255 + max[i] / 2) / max[i]));
    Backlight {
        brightness: fs.read_num(&format!("{KBD_LED}/brightness")),
        max_brightness: fs.read_num(&format!("{KBD_LED}/max_brightness")),
        color,
    }
}

fn battery(fs: &Sysfs<'_>) -> Battery {
    let b = |f: &str| format!("{BATTERY}/{f}");
    let full: Option<u64> = fs.read_num(&b("charge_full"));
    let design: Option<u64> = fs.read_num(&b("charge_full_design"));
    Battery {
        capacity_percent: fs.read_num(&b("capacity")),
        state: fs.read(&b("status")),
        on_ac: fs.read(&format!("{AC}/online")).map(|v| v == "1"),
        // ACPI reports 0 on this board; the driver reads the real count from the EC
        cycle_count: fs.read_num(&format!("{PLATFORM}/battery_cycle_count")).or_else(|| fs.read_num(&b("cycle_count"))),
        health_percent: match (full, design) {
            (Some(f), Some(d)) if d > 0 => Some((f * 100 / d) as u32),
            _ => None,
        },
        charging_profile: fs.read(&b("charge_types")).and_then(|types| {
            let selected = types.split_whitespace().find_map(|t| t.strip_prefix('[')?.strip_suffix(']'))?;
            CHARGING_PROFILES.iter().find(|(_, kernel)| *kernel == selected).map(|(name, _)| name.to_string())
        }),
        charge_cap_percent: fs.read_num(&b("charge_control_end_threshold")),
    }
}

/// The firmware's charging profiles, and the kernel's charge type each one is.
const CHARGING_PROFILES: [(&str, &str); 3] = [("high-capacity", "Standard"), ("balanced", "Long_Life"), ("stationary", "Trickle")];
/// Below this a cap would leave the laptop close to useless off the charger.
const MIN_CHARGE_CAP: u32 = 20;

fn not_exposed(what: &str) -> anyhow::Error {
    anyhow::anyhow!("{what} is not exposed by the driver on this machine")
}

/// Sets the firmware's charging profile through the battery's `charge_types`.
pub fn set_charging_profile(port: &Port, profile: &str) -> anyhow::Result<()> {
    let fs = Sysfs::new(&port.root);
    let file = format!("{BATTERY}/charge_types");
    anyhow::ensure!(fs.exists(&file), not_exposed("the charging profile"));
    let (_, kernel) = CHARGING_PROFILES.iter().find(|(name, _)| *name == profile)
        .ok_or_else(|| anyhow::anyhow!("unknown charging profile {profile:?}: high-capacity, balanced or stationary"))?;
    fs.write(&file, kernel)
}

/// Caps the charge level through the battery's `charge_control_end_threshold`.
pub fn set_charge_cap(port: &Port, percent: u32) -> anyhow::Result<()> {
    let fs = Sysfs::new(&port.root);
    let file = format!("{BATTERY}/charge_control_end_threshold");
    anyhow::ensure!(fs.exists(&file), not_exposed("the charge cap"));
    anyhow::ensure!((MIN_CHARGE_CAP..=100).contains(&percent), "the charge cap is {MIN_CHARGE_CAP} to 100 percent, not {percent}");
    fs.write(&file, &percent.to_string())
}

fn no_backlight() -> anyhow::Error {
    not_exposed("the keyboard backlight")
}

/// Sets the keyboard backlight's brightness level, from 0 (off) to the keyboard's maximum.
pub fn set_keyboard_brightness(port: &Port, level: u32) -> anyhow::Result<()> {
    let fs = Sysfs::new(&port.root);
    let max: u32 = fs.read_num(&format!("{KBD_LED}/max_brightness")).ok_or_else(no_backlight)?;
    anyhow::ensure!(level <= max, "the keyboard brightness is 0 to {max}, not {level}");
    fs.write(&format!("{KBD_LED}/brightness"), &level.to_string())
}

/// Sets the keyboard backlight's colour (red, green, blue, each 0-255), scaled to what the
/// keyboard takes per channel.
pub fn set_keyboard_color(port: &Port, rgb: [u8; 3]) -> anyhow::Result<()> {
    let fs = Sysfs::new(&port.root);
    anyhow::ensure!(fs.exists(&format!("{KBD_LED}/multi_intensity")), no_backlight());
    let max = max_intensity(&fs);
    let raw = [0, 1, 2].map(|i| ((u32::from(rgb[i]) * max[i] + 127) / 255).to_string());
    fs.write(&format!("{KBD_LED}/multi_intensity"), &raw.join(" "))
}

/// Never wakes the card: its load is only queried when the kernel already has it awake.
async fn gpu(port: &Port, fs: &Sysfs<'_>) -> Gpu {
    let mut g = Gpu { tgp_offset_w: fs.read_num(&format!("{PLATFORM}/ctgp_offset")), ..Gpu::default() };
    g.state = match fs.read(GPU_RUNTIME_STATUS).as_deref() {
        Some("active") => "awake",
        Some("suspended") | Some("suspending") => "asleep",
        _ => "unknown",
    }
    .to_string();
    if g.state == "awake"
        && let Ok(load) = port.gpu.query().await {
            g.power_draw_w = Some(load.power_draw_w);
            g.power_limit_w = Some(load.power_limit_w);
            g.utilization_percent = Some(load.utilization_percent);
        }
    g
}

/// Reads the whole state from the port.
pub async fn read(port: &Port) -> anyhow::Result<Status> {
    let fs = Sysfs::new(&port.root);
    let hw = fs.hwmon("uniwill");
    let temp = |n: u32| hw.as_ref().and_then(|h| fs.read_num::<i64>(&format!("{h}/temp{n}_input"))).map(|m| m / 1000);
    let mut fans = Fans::default();
    if let Some(h) = &hw {
        fans.mode = fs.read(&format!("{h}/pwm1_enable")).map(|m| match m.as_str() {
            "0" => "max".to_string(),
            "1" => "manual".to_string(),
            _ => "automatic".to_string(),
        });
        for n in 1..=2 {
            if let Some(rpm) = fs.read_num(&format!("{h}/fan{n}_input")) {
                fans.fans.push(Fan { rpm, duty: fs.read_num(&format!("{h}/pwm{n}")).unwrap_or(0) });
            }
        }
    }
    let mut s = Status {
        profile: port.profiles.active().await?,
        hold: None,
        temperatures: Temperatures { cpu_c: temp(1), gpu_c: temp(2) },
        fans,
        gpu: gpu(port, &fs).await,
        battery: battery(&fs),
        cpu: cpu(&fs),
        backlight: backlight(&fs),
        unavailable: Vec::new(),
    };
    let missing = [
        ("temperatures", s.temperatures.cpu_c.is_none() && s.temperatures.gpu_c.is_none()),
        ("fan_control", s.fans.mode.is_none()),
        ("tgp_offset", s.gpu.tgp_offset_w.is_none()),
        ("gpu_state", s.gpu.state == "unknown"),
        ("battery", s.battery.capacity_percent.is_none()),
        ("charging_profile", s.battery.charging_profile.is_none()),
        ("charge_cap", s.battery.charge_cap_percent.is_none()),
        ("cpu_power_limits", s.cpu.long_term_w.is_none()),
        ("keyboard_backlight", s.backlight.brightness.is_none()),
    ];
    s.unavailable = missing.iter().filter(|(_, m)| *m).map(|(f, _)| f.to_string()).collect();
    Ok(s)
}
