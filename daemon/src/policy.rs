//! The policy: what the machine should be doing, computed from what the daemon knows. Pure: no
//! I/O and no clock, so the engine feeds it readings and writes out what it returns.

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
