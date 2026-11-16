mod common;
use common::*;

#[tokio::test]
async fn status_shows_the_active_power_profile() {
    let rig = Rig::start().await;
    *rig.profiles.active.lock().unwrap() = "balanced".into();

    let status = rig.client.status().await.unwrap();

    assert_eq!(status.profile, "balanced");
}

#[tokio::test]
async fn status_shows_both_temperatures_and_both_fans() {
    let rig = Rig::start().await;
    rig.write(&format!("{HWMON}/temp1_input"), "61000");
    rig.write(&format!("{HWMON}/fan1_input"), "5500");
    rig.write(&format!("{HWMON}/pwm1"), "255");
    rig.write(&format!("{HWMON}/pwm2"), "70");
    rig.write(&format!("{HWMON}/pwm1_enable"), "0");
    rig.write(&format!("{HWMON}/pwm2_enable"), "0");

    let status = rig.client.status().await.unwrap();

    assert_eq!(status.temperatures.cpu_c, Some(61));
    assert_eq!(status.temperatures.gpu_c, Some(45));
    assert_eq!(status.fans.mode.as_deref(), Some("max"));
    assert_eq!(status.fans.fans.len(), 2);
    assert_eq!(status.fans.fans[0].rpm, 5500);
    assert_eq!(status.fans.fans[0].duty, 255);
    assert_eq!(status.fans.fans[1].rpm, 1622);
    assert_eq!(status.fans.fans[1].duty, 70);
}

#[tokio::test]
async fn status_never_wakes_a_sleeping_graphics_card() {
    let rig = Rig::start().await;
    rig.write(GPU_PM, "suspended");
    rig.write(&format!("{PLATFORM}/ctgp_offset"), "20");

    let status = rig.client.status().await.unwrap();

    assert_eq!(status.gpu.state, "asleep");
    assert_eq!(status.gpu.tgp_offset_w, Some(20));
    assert_eq!(status.gpu.power_draw_w, None);
    assert_eq!(*rig.gpu.queries.lock().unwrap(), 0);
}

#[tokio::test]
async fn status_shows_gpu_power_when_the_card_is_awake() {
    let rig = Rig::start().await;
    rig.write(GPU_PM, "active");
    *rig.gpu.load.lock().unwrap() = Some(avell::port::GpuLoad { utilization_percent: 88, power_draw_w: 67.0, power_limit_w: 95.0 });

    let status = rig.client.status().await.unwrap();

    assert_eq!(status.gpu.state, "awake");
    assert_eq!(status.gpu.power_draw_w, Some(67.0));
    assert_eq!(status.gpu.power_limit_w, Some(95.0));
    assert_eq!(status.gpu.utilization_percent, Some(88));
    assert_eq!(*rig.gpu.queries.lock().unwrap(), 1);
}

#[tokio::test]
async fn status_shows_battery_charge_cycles_and_wear() {
    let rig = Rig::start().await;
    rig.write(&format!("{BAT}/capacity"), "87");
    rig.write(&format!("{BAT}/status"), "Discharging");
    rig.write(&format!("{AC}/online"), "0");

    let status = rig.client.status().await.unwrap();

    assert_eq!(status.battery.capacity_percent, Some(87));
    assert_eq!(status.battery.state.as_deref(), Some("Discharging"));
    assert_eq!(status.battery.on_ac, Some(false));
    // The driver's EC count wins over ACPI's 0
    assert_eq!(status.battery.cycle_count, Some(308));
    assert_eq!(status.battery.health_percent, Some(70));
}

#[tokio::test]
async fn battery_cycle_count_falls_back_to_acpi_without_the_driver() {
    let rig = Rig::start().await;
    rig.remove(&format!("{PLATFORM}/battery_cycle_count"));
    rig.write(&format!("{BAT}/cycle_count"), "12");

    let status = rig.client.status().await.unwrap();

    assert_eq!(status.battery.cycle_count, Some(12));
}

#[tokio::test]
async fn status_shows_cpu_power_limits_and_keyboard_backlight() {
    let rig = Rig::start().await;
    rig.write(&format!("{RAPL}/constraint_0_power_limit_uw"), "45000000");
    rig.write(&format!("{RAPL}/constraint_1_power_limit_uw"), "65000000");
    rig.write(&format!("{KBD}/brightness"), "3");

    let status = rig.client.status().await.unwrap();

    assert_eq!(status.cpu.long_term_w, Some(45));
    assert_eq!(status.cpu.short_term_w, Some(65));
    assert_eq!(status.backlight.brightness, Some(3));
    assert_eq!(status.backlight.max_brightness, Some(4));
    assert_eq!(status.backlight.color, Some([0, 255, 255])); // 0 50 50 of 50, as 0-255
}

#[tokio::test]
async fn features_the_driver_does_not_expose_are_listed_as_unavailable() {
    // The stock kernel driver: no fan control, no TGP offset, no backlight, no powercap
    let rig = Rig::start().await;
    rig.remove(&format!("{HWMON}/pwm1_enable"));
    rig.remove(&format!("{PLATFORM}/ctgp_offset"));
    rig.remove(&format!("{KBD}/brightness"));
    rig.remove(&format!("{RAPL}/constraint_0_power_limit_uw"));

    let status = rig.client.status().await.unwrap();

    for feature in ["fan_control", "tgp_offset", "keyboard_backlight", "cpu_power_limits"] {
        assert!(status.unavailable.iter().any(|f| f == feature), "{feature} not listed in {:?}", status.unavailable);
    }
    assert!(!status.unavailable.iter().any(|f| f == "temperatures"));
    assert_eq!(status.fans.mode, None);
    assert_eq!(status.temperatures.cpu_c, Some(58));
}
