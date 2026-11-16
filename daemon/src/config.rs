//! The daemon's configuration: one bundle of settings per Power profile.

use serde::{Deserialize, Serialize};

/// The Power profiles, as the power-profile service names them.
pub const PROFILES: [&str; 3] = ["power-saver", "balanced", "performance"];
/// The firmware table's maximum TGP offset for this machine.
pub const MAX_TGP_OFFSET_W: u32 = 20;
/// From a floor that keeps the machine usable up to the processor's own limit register (120 W).
pub const CPU_LIMIT_W: std::ops::RangeInclusive<u32> = 5..=120;

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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Profiles {
    #[serde(rename = "power-saver")]
    pub power_saver: Bundle,
    pub balanced: Bundle,
    pub performance: Bundle,
}

impl Default for Profiles {
    fn default() -> Self {
        let b = |tgp_offset_w, dynamic_boost, cpu_long_term_w, cpu_short_term_w| Bundle {
            tgp_offset_w,
            dynamic_boost,
            cpu_long_term_w,
            cpu_short_term_w,
        };
        Profiles {
            power_saver: b(0, false, 20, 30),
            balanced: b(10, true, 45, 65),
            // the firmware table's own limits for this machine
            performance: b(20, true, 80, 80),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub profiles: Profiles,
}

impl Config {
    /// Reads a configuration and checks it against what this machine allows.
    pub fn parse(text: &str) -> anyhow::Result<Config> {
        let config: Config = toml::from_str(text)?;
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
