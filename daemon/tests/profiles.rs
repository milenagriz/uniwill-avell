mod common;
use common::*;
use std::time::Duration;

#[tokio::test]
async fn a_profile_change_in_the_power_menu_applies_that_profiles_bundle() {
    let rig = Rig::start().await;

    *rig.profiles.active.lock().unwrap() = "power-saver".into();
    rig.advance(Duration::from_secs(1)).await;

    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "0");
    assert_eq!(watts(&rig, 0), 20);
    assert_eq!(watts(&rig, 1), 30);
    assert_eq!(rig.services.running.lock().unwrap().get("nvidia-powerd.service"), Some(&false));
}

#[tokio::test]
async fn setting_the_profile_through_the_control_api_applies_its_bundle_at_once() {
    let rig = Rig::start().await;

    rig.client.set_profile("balanced").await.unwrap();

    assert_eq!(*rig.profiles.active.lock().unwrap(), "balanced");
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "10");
    assert_eq!(watts(&rig, 0), 45);
    assert_eq!(watts(&rig, 1), 65);
    assert_eq!(rig.services.running.lock().unwrap().get("nvidia-powerd.service"), Some(&true));
}

#[tokio::test]
async fn an_invalid_configuration_is_rejected_and_the_previous_one_stays_in_force() {
    let rig = Rig::start().await;
    let before = rig.client.config().await.unwrap();

    for (change, complaint) in [
        ("tgp_offset_w = 20", "tgp_offset_w = 21"),          // above the firmware table's maximum
        ("cpu_long_term_w = 80", "cpu_long_term_w = 200"),   // outside 5-120 W
        ("cpu_short_term_w = 80", "cpu_short_term_w = 60"),  // short-term below long-term
        ("dynamic_boost = false", "dynamic_boost = 3"),      // wrong type
        ("[profiles.balanced]", "[profiles.turbo]"),         // unknown profile
    ] {
        let bad = TEST_CONFIG.replacen(change, complaint, 1);
        assert_ne!(bad, TEST_CONFIG);
        let err = rig.client.set_config(&bad).await.unwrap_err();
        assert!(format!("{err:#}").contains("configuration"), "{complaint}: {err:#}");
    }

    assert_eq!(rig.client.config().await.unwrap(), before);
    assert_eq!(rig.read(Rig::config_rel()), TEST_CONFIG.trim());
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "20");
    assert_eq!(watts(&rig, 0), 80);
}

#[tokio::test]
async fn a_valid_configuration_is_saved_and_applied_at_once() {
    let rig = Rig::start().await;
    let quieter = TEST_CONFIG.replacen("tgp_offset_w = 20", "tgp_offset_w = 15", 1).replacen("cpu_long_term_w = 80", "cpu_long_term_w = 60", 1);

    rig.client.set_config(&quieter).await.unwrap();

    // applied to the active profile (performance)
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "15");
    assert_eq!(watts(&rig, 0), 60);
    assert_eq!(watts(&rig, 1), 80);
    // in force, and saved for the next start
    let now: toml::Table = rig.client.config().await.unwrap().parse().unwrap();
    assert_eq!(now["profiles"]["performance"]["tgp_offset_w"].as_integer(), Some(15));
    let saved: toml::Table = rig.read(Rig::config_rel()).parse().unwrap();
    assert_eq!(saved, now);
}

#[tokio::test]
async fn without_a_configuration_file_the_built_in_bundles_apply() {
    let root = Rig::laptop();
    std::fs::remove_file(root.path().join(Rig::config_rel())).unwrap();
    let rig = Rig::start_with(root).await;

    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "20");
    assert_eq!(watts(&rig, 0), 80);
    let config: toml::Table = rig.client.config().await.unwrap().parse().unwrap();
    assert_eq!(config["profiles"]["power-saver"]["cpu_long_term_w"].as_integer(), Some(20));
}

#[tokio::test]
async fn the_bundle_is_applied_again_after_resume() {
    let rig = Rig::start().await;
    // what the firmware does over a suspend: its own values come back
    rig.write(&format!("{PLATFORM}/ctgp_offset"), "0");
    rig.write(&format!("{RAPL}/constraint_0_power_limit_uw"), "120000000");
    rig.write(&format!("{RAPL}/constraint_1_power_limit_uw"), "120000000");

    rig.suspend(Duration::from_secs(3600)).await;

    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "20");
    assert_eq!(watts(&rig, 0), 80);
    assert_eq!(watts(&rig, 1), 80);
}

#[tokio::test]
async fn a_clean_stop_leaves_the_machine_in_the_firmwares_state() {
    let rig = Rig::start().await;
    assert_eq!(watts(&rig, 0), 80);

    rig.daemon.shutdown().await;

    // TGP offset 0, and the CPU limits the firmware had set before the daemon started
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "0");
    assert_eq!(watts(&rig, 0), 120);
    assert_eq!(watts(&rig, 1), 120);
}

#[tokio::test]
async fn the_configuration_can_be_read_and_replaced_as_json() {
    let rig = Rig::start().await;

    let mut config: serde_json::Value = serde_json::from_str(&rig.client.config_json().await.unwrap()).unwrap();
    assert_eq!(config["profiles"]["performance"]["tgp_offset_w"], 20);
    assert_eq!(config["profiles"]["performance"]["fan_curve"][1], serde_json::json!([70, 60]));

    config["profiles"]["performance"]["tgp_offset_w"] = 12.into();
    rig.client.set_config_json(&config.to_string()).await.unwrap();
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "12");
    let saved: toml::Table = rig.read(Rig::config_rel()).parse().unwrap();
    assert_eq!(saved["profiles"]["performance"]["tgp_offset_w"].as_integer(), Some(12));

    // the same checks as for TOML
    config["profiles"]["performance"]["tgp_offset_w"] = 21.into();
    let err = rig.client.set_config_json(&config.to_string()).await.unwrap_err();
    assert!(format!("{err:#}").contains("invalid configuration"), "{err:#}");
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "12");
}
