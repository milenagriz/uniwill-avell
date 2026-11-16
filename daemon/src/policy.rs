//! The policy: what the machine should be doing, computed from what the daemon knows. Pure: no
//! I/O and no clock, so the engine feeds it readings and writes out what it returns.

use std::time::Duration;

/// The TGP offset to set: the bundle's on AC, none on battery, where the graphics card would
/// only drain it faster.
pub fn tgp_offset_w(bundle_w: u32, on_ac: Option<bool>) -> u32 {
    if on_ac == Some(false) { 0 } else { bundle_w }
}

/// A fan curve: (temperature in °C, fan speed in percent) points, by rising temperature.
pub type Curve = [[u32; 2]];

/// The curve's speed at a temperature: linear between points, flat before the first and after
/// the last.
pub fn curve_percent(curve: &Curve, temp_c: i64) -> u32 {
    let (Some(first), Some(last)) = (curve.first(), curve.last()) else {
        return 100;
    };
    if temp_c <= i64::from(first[0]) {
        return first[1];
    }
    if temp_c >= i64::from(last[0]) {
        return last[1];
    }
    let i = curve.iter().position(|p| i64::from(p[0]) > temp_c).expect("below the last point");
    let ([t0, p0], [t1, p1]) = (curve[i - 1], curve[i]);
    let share = (temp_c - i64::from(t0)) as f64 / f64::from(t1 - t0);
    (f64::from(p0) + share * (f64::from(p1) - f64::from(p0))).round() as u32
}

/// What the fan curve remembers between readings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FanState {
    /// The temperature the current speed was chosen for.
    held_c: Option<i64>,
}

/// The fan speed in percent for a temperature. Warming is followed at once; cooling only once
/// the temperature is `hysteresis_c` below where the speed was last chosen.
pub fn fan_percent(curve: &Curve, hysteresis_c: u32, temp_c: i64, state: &mut FanState) -> u32 {
    let held = match state.held_c {
        Some(held) if temp_c < held && temp_c > held - i64::from(hysteresis_c) => held,
        _ => temp_c,
    };
    state.held_c = Some(held);
    curve_percent(curve, held)
}

/// A fan speed in percent as the 0-255 duty the kernel takes.
pub fn duty(percent: u32) -> u32 {
    (percent.min(100) * 255 + 50) / 100
}

/// Where game detection stands.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Game {
    #[default]
    Idle,
    /// The graphics card has been busy since then, not yet long enough for a hold.
    Busy { since: Duration },
    /// Performance is held; `quiet_since` is set while the load is low.
    Holding { quiet_since: Option<Duration> },
    /// The owner chose another profile during a hold: no new hold until this load has ended.
    Overridden { quiet_since: Option<Duration> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameAction {
    Hold,
    Release,
}

/// The settings game detection goes by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameRules {
    pub load_percent: u32,
    pub hold_after: Duration,
    pub release_after: Duration,
}

/// One reading of the graphics card's load (None: asleep, so not asked) at `now`. `allowed` is
/// false when detection is off or the machine is on battery.
pub fn game_step(state: Game, rules: GameRules, allowed: bool, load_percent: Option<u32>, now: Duration) -> (Game, Option<GameAction>) {
    let busy = load_percent.is_some_and(|l| l >= rules.load_percent);
    let ended = |quiet_since: Duration| now.saturating_sub(quiet_since) >= rules.release_after;
    if !allowed {
        let action = matches!(state, Game::Holding { .. }).then_some(GameAction::Release);
        return (Game::Idle, action);
    }
    match state {
        Game::Idle if busy => game_step(Game::Busy { since: now }, rules, allowed, load_percent, now),
        Game::Idle => (Game::Idle, None),
        Game::Busy { .. } if !busy => (Game::Idle, None),
        Game::Busy { since } if now.saturating_sub(since) >= rules.hold_after => {
            (Game::Holding { quiet_since: None }, Some(GameAction::Hold))
        }
        Game::Busy { since } => (Game::Busy { since }, None),
        Game::Holding { .. } if busy => (Game::Holding { quiet_since: None }, None),
        Game::Holding { quiet_since } => {
            let quiet_since = quiet_since.unwrap_or(now);
            if ended(quiet_since) { (Game::Idle, Some(GameAction::Release)) } else { (Game::Holding { quiet_since: Some(quiet_since) }, None) }
        }
        Game::Overridden { .. } if busy => (Game::Overridden { quiet_since: None }, None),
        Game::Overridden { quiet_since } => {
            let quiet_since = quiet_since.unwrap_or(now);
            if ended(quiet_since) { (Game::Idle, None) } else { (Game::Overridden { quiet_since: Some(quiet_since) }, None) }
        }
    }
}
