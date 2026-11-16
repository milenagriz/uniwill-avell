//! The daemon's configuration: one bundle of settings per Power profile.

use serde::{Deserialize, Serialize};

/// The Power profiles, as the power-profile service names them.
pub const PROFILES: [&str; 3] = ["power-saver", "balanced", "performance"];
/// The firmware table's maximum TGP offset for this machine.
pub const MAX_TGP_OFFSET_W: u32 = 20;
/// From a floor that keeps the machine usable up to the processor's own limit register (120 W).
pub const CPU_LIMIT_W: std::ops::RangeInclusive<u32> = 5..=120;

pub const MAX_CURVE_TEMP_C: u32 = 110;
pub const MAX_HYSTERESIS_C: u32 = 20;

/// What one Power profile applies to the machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    /// TGP offset in watts, on top of the graphics card's base power budget.
    pub tgp_offset_w: u32,
    /// Dynamic Boost: whether nvidia-powerd runs.
    pub dynamic_boost: bool,
    /// Long-term and short-term CPU package power limits (PL1 / PL2), in watts.
    pub cpu_long_term_w: u32,
    pub cpu_short_term_w: u32,
    /// The fan curve: [temperature in °C, fan speed in percent] points, by rising temperature.
    pub fan_curve: Vec<[u32; 2]>,
}

/// Who runs the fans when the daemon starts.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FanMode {
    /// The daemon, along the active profile's curve.
    Curve,
    /// The firmware's own curve.
    Automatic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fans {
    pub mode: FanMode,
    /// The temperature has to fall this far below where the fans last sped up before they slow
    /// down again.
    pub hysteresis_c: u32,
}

impl Default for Fans {
    fn default() -> Self {
        Fans { mode: FanMode::Curve, hysteresis_c: 4 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profiles {
    #[serde(rename = "power-saver")]
    pub power_saver: Bundle,
    pub balanced: Bundle,
    pub performance: Bundle,
}

impl Default for Profiles {
    fn default() -> Self {
        let b = |tgp_offset_w, dynamic_boost, cpu_long_term_w, cpu_short_term_w, fan_curve: &[[u32; 2]]| Bundle {
            tgp_offset_w,
            dynamic_boost,
            cpu_long_term_w,
            cpu_short_term_w,
            fan_curve: fan_curve.to_vec(),
        };
        Profiles {
            power_saver: b(0, false, 20, 30, &[[50, 20], [65, 35], [75, 50], [85, 75], [92, 100]]),
            balanced: b(10, true, 45, 65, &[[50, 30], [60, 40], [70, 60], [80, 80], [88, 100]]),
            // the firmware table's own limits for this machine
            performance: b(20, true, 80, 80, &[[45, 35], [55, 50], [65, 70], [75, 90], [80, 100]]),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub charger: Charger,
    pub fans: Fans,
    pub game_detection: GameDetection,
    pub profiles: Profiles,
}

/// The charger rules: which Power profile each power source gets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Charger {
    /// Off: plugging and unplugging change nothing.
    pub enabled: bool,
    /// The profile set when the charger is plugged in, and when it is unplugged. A manual change
    /// of profile replaces the one for the power source in use.
    pub on_ac: String,
    pub on_battery: String,
}

impl Default for Charger {
    fn default() -> Self {
        Charger { enabled: true, on_ac: "performance".into(), on_battery: "power-saver".into() }
    }
}

impl Charger {
    pub fn profile(&self, on_ac: bool) -> &str {
        if on_ac { &self.on_ac } else { &self.on_battery }
    }
}

/// Game detection: the performance profile is held while the graphics card is busy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDetection {
    pub enabled: bool,
    /// The graphics card counts as busy at this utilisation or above.
    pub load_percent: u32,
    /// How long it has to stay busy before performance is held, and how long it has to stay
    /// below the threshold before the hold is released, in seconds.
    pub hold_after_s: u32,
    pub release_after_s: u32,
}

impl Default for GameDetection {
    fn default() -> Self {
        GameDetection { enabled: true, load_percent: 60, hold_after_s: 15, release_after_s: 60 }
    }
}

/// Lays `over` on top of `base`, table by table; anything else is replaced whole.
fn merge(base: &mut toml::Table, over: toml::Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge(b, o),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

impl Config {
    /// Reads a configuration and checks it against what this machine allows. Settings it leaves
    /// out keep their built-in values.
    pub fn parse(text: &str) -> anyhow::Result<Config> {
        let mut merged = toml::Table::try_from(Config::default())?;
        merge(&mut merged, text.parse()?);
        let config: Config = merged.try_into()?;
        anyhow::ensure!(
            config.fans.hysteresis_c <= MAX_HYSTERESIS_C,
            "fans.hysteresis_c is {}: allowed are 0 to {MAX_HYSTERESIS_C} °C",
            config.fans.hysteresis_c
        );
        for (key, profile) in [("on_ac", &config.charger.on_ac), ("on_battery", &config.charger.on_battery)] {
            anyhow::ensure!(
                PROFILES.contains(&profile.as_str()),
                "charger.{key} is {profile:?}: the Power profiles are {}",
                PROFILES.join(", ")
            );
        }
        let game = &config.game_detection;
        anyhow::ensure!(
            (1..=100).contains(&game.load_percent),
            "game_detection.load_percent is {}: allowed are 1 to 100",
            game.load_percent
        );
        for (key, s) in [("hold_after_s", game.hold_after_s), ("release_after_s", game.release_after_s)] {
            anyhow::ensure!(s <= 3600, "game_detection.{key} is {s}: allowed are 0 to 3600 seconds");
        }
        for name in PROFILES {
            let b = config.bundle(name).expect("every profile has a bundle");
            anyhow::ensure!(
                b.tgp_offset_w <= MAX_TGP_OFFSET_W,
                "profiles.{name}.tgp_offset_w is {}: the firmware allows 0 to {MAX_TGP_OFFSET_W} W on this machine",
                b.tgp_offset_w
            );
            for (key, w) in [("cpu_long_term_w", b.cpu_long_term_w), ("cpu_short_term_w", b.cpu_short_term_w)] {
                anyhow::ensure!(
                    CPU_LIMIT_W.contains(&w),
                    "profiles.{name}.{key} is {w}: allowed are {} to {} W",
                    CPU_LIMIT_W.start(),
                    CPU_LIMIT_W.end()
                );
            }
            let curve = &b.fan_curve;
            anyhow::ensure!(
                !curve.is_empty()
                    && curve.iter().all(|p| p[0] <= MAX_CURVE_TEMP_C && p[1] <= 100)
                    && curve.windows(2).all(|w| w[0][0] < w[1][0] && w[0][1] <= w[1][1]),
                "profiles.{name}.fan_curve needs [°C, percent] points with rising temperatures (up to \
                 {MAX_CURVE_TEMP_C} °C) and speeds of 0 to 100 percent that never fall"
            );
            anyhow::ensure!(
                b.cpu_short_term_w >= b.cpu_long_term_w,
                "profiles.{name}.cpu_short_term_w ({}) is below cpu_long_term_w ({})",
                b.cpu_short_term_w,
                b.cpu_long_term_w
            );
        }
        Ok(config)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("the configuration serialises")
    }

    /// The bundle of a Power profile, by the power-profile service's name for it.
    pub fn bundle(&self, profile: &str) -> Option<&Bundle> {
        match profile {
            "power-saver" => Some(&self.profiles.power_saver),
            "balanced" => Some(&self.profiles.balanced),
            "performance" => Some(&self.profiles.performance),
            _ => None,
        }
    }
}
