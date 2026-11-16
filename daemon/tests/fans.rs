mod common;
use common::*;
use std::time::Duration;

/// The fan heartbeat: the daemon writes the duty this often (the kernel watchdog allows 5 s).
const BEAT: Duration = Duration::from_secs(2);

fn set_temps(rig: &Rig, cpu_c: u32, gpu_c: u32) {
    rig.write(&format!("{HWMON}/temp1_input"), &format!("{}", cpu_c * 1000));
    rig.write(&format!("{HWMON}/temp2_input"), &format!("{}", gpu_c * 1000));
}

fn duties(rig: &Rig) -> (String, String, String) {
    (rig.read(&format!("{HWMON}/pwm1_enable")), rig.read(&format!("{HWMON}/pwm1")), rig.read(&format!("{HWMON}/pwm2")))
}

// TEST_CONFIG, performance: 50 °C -> 20 %, 70 °C -> 60 %, 90 °C -> 100 %; hysteresis 5 °C

#[tokio::test]
async fn fan_speed_follows_the_curve_at_the_hotter_of_the_two_temperatures() {
    let rig = Rig::start().await;

    set_temps(&rig, 60, 80); // the GPU is hotter: 80 °C is 80 %
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig), ("1".into(), "204".into(), "204".into()));

    set_temps(&rig, 90, 80); // now the CPU is: 100 %
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig), ("1".into(), "255".into(), "255".into()));

    set_temps(&rig, 40, 30); // below the first point: its speed, 20 %
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig), ("1".into(), "51".into(), "51".into()));
}

#[tokio::test]
async fn hysteresis_keeps_the_fans_from_hunting_around_a_temperature() {
    let rig = Rig::start().await;
    set_temps(&rig, 80, 40);
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig).1, "204"); // 80 %

    // cooling by less than the hysteresis changes nothing
    for cpu in [78, 76, 79, 76] {
        set_temps(&rig, cpu, 40);
        rig.advance(BEAT).await;
        assert_eq!(duties(&rig).1, "204", "at {cpu} °C");
    }
    // cooling by more does: 70 °C is 60 %
    set_temps(&rig, 70, 40);
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig).1, "153");
    // and warming is followed at once: 75 °C is 70 %
    set_temps(&rig, 75, 40);
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig).1, "179");
}

#[tokio::test]
async fn switching_the_power_profile_switches_the_curve() {
    let rig = Rig::start().await;
    set_temps(&rig, 60, 40);
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig).1, "102"); // performance: 60 °C is 40 %

    rig.client.set_profile("power-saver").await.unwrap();

    assert_eq!(duties(&rig).1, "128"); // power-saver: 60 °C is 50 %
}

#[tokio::test]
async fn the_duty_is_written_every_beat_even_when_it_has_not_changed() {
    let rig = Rig::start().await;
    for _ in 0..3 {
        // stands in for the kernel's "last write" time: wipe the file, expect it back
        rig.write(&format!("{HWMON}/pwm1"), "");
        rig.write(&format!("{HWMON}/pwm2"), "");
        rig.advance(BEAT).await;
        assert_eq!(duties(&rig), ("1".into(), "92".into(), "92".into())); // 58 °C is 36 %
    }
}

#[tokio::test]
async fn the_daemon_takes_the_fans_back_after_the_kernel_watchdog_fired() {
    let rig = Rig::start().await;
    // the daemon was stalled: the watchdog put the fans on automatic, the firmware set its duty
    rig.write(&format!("{HWMON}/pwm1_enable"), "2");
    rig.write(&format!("{HWMON}/pwm1"), "76");

    rig.advance(BEAT).await;

    assert_eq!(duties(&rig), ("1".into(), "92".into(), "92".into()));
}

#[tokio::test]
async fn a_clean_stop_leaves_the_fans_on_automatic() {
    let rig = Rig::start().await;
    assert_eq!(duties(&rig).0, "1");

    rig.daemon.shutdown().await;

    assert_eq!(duties(&rig).0, "2");
}

#[tokio::test]
async fn a_fixed_speed_is_held_whatever_the_temperature() {
    let rig = Rig::start().await;

    rig.client.set_fan("fixed", 40).await.unwrap();
    assert_eq!(duties(&rig), ("1".into(), "102".into(), "102".into()));

    set_temps(&rig, 85, 85);
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig), ("1".into(), "102".into(), "102".into()));
    let fans = rig.client.status().await.unwrap().fans;
    assert_eq!((fans.control.as_deref(), fans.target_percent), (Some("fixed"), Some(40)));
}

#[tokio::test]
async fn max_fan_and_automatic_are_left_alone_until_the_curve_is_asked_for_again() {
    let rig = Rig::start().await;

    rig.client.set_fan("max", 0).await.unwrap();
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig).0, "0");
    assert_eq!(rig.client.status().await.unwrap().fans.control.as_deref(), Some("max"));

    rig.client.set_fan("automatic", 0).await.unwrap();
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig).0, "2");
    assert_eq!(rig.client.status().await.unwrap().fans.control.as_deref(), Some("automatic"));

    rig.client.set_fan("curve", 0).await.unwrap();
    assert_eq!(duties(&rig), ("1".into(), "92".into(), "92".into()));
    let fans = rig.client.status().await.unwrap().fans;
    assert_eq!((fans.control.as_deref(), fans.target_percent), (Some("curve"), Some(36)));
}

#[tokio::test]
async fn max_fan_turned_on_by_someone_else_is_not_overridden() {
    let rig = Rig::start().await;
    // the Fn key, or another program writing the fan mode
    rig.write(&format!("{HWMON}/pwm1_enable"), "0");
    rig.write(&format!("{HWMON}/pwm1"), "255");

    rig.advance(BEAT).await;
    assert_eq!(duties(&rig).0, "0");
    assert_eq!(duties(&rig).1, "255");

    // released: the curve is back
    rig.write(&format!("{HWMON}/pwm1_enable"), "2");
    rig.advance(BEAT).await;
    assert_eq!(duties(&rig), ("1".into(), "92".into(), "92".into()));
}

#[tokio::test]
async fn a_nonsense_fan_request_or_curve_is_refused() {
    let rig = Rig::start().await;

    assert!(rig.client.set_fan("fixed", 101).await.is_err());
    assert!(rig.client.set_fan("turbo", 0).await.is_err());
    for bad in [
        "fan_curve = [[70, 60], [50, 20]]",   // temperatures must rise
        "fan_curve = [[50, 60], [70, 20]]",   // speeds must not fall
        "fan_curve = [[50, 20], [70, 120]]",  // percent
        "fan_curve = []",
    ] {
        let config = TEST_CONFIG.replacen("fan_curve = [[50, 20], [70, 60], [90, 100]]", bad, 1);
        let err = rig.client.set_config(&config).await.unwrap_err();
        assert!(format!("{err:#}").contains("fan_curve"), "{bad}: {err:#}");
    }
    assert!(rig.client.set_config(&TEST_CONFIG.replacen("hysteresis_c = 5", "hysteresis_c = 50", 1)).await.is_err());

    assert_eq!(duties(&rig), ("1".into(), "92".into(), "92".into()));
}

#[tokio::test]
async fn with_fans_set_to_automatic_in_the_configuration_the_daemon_leaves_them_to_the_firmware() {
    let root = Rig::laptop();
    common::write(root.path(), Rig::config_rel(), "[fans]\nmode = \"automatic\"");
    let rig = Rig::start_with(root).await;

    rig.advance(BEAT).await;

    assert_eq!(duties(&rig), ("2".into(), "76".into(), "70".into()));
}
