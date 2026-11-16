mod common;
use common::*;
use std::time::Duration;

const TICK: Duration = Duration::from_secs(1);

async fn plug(rig: &Rig, online: bool) {
    rig.write(&format!("{AC}/online"), if online { "1" } else { "0" });
    rig.advance(TICK).await;
}

fn active(rig: &Rig) -> String {
    rig.profiles.active.lock().unwrap().clone()
}

async fn choose_in_the_power_menu(rig: &Rig, profile: &str) {
    *rig.profiles.active.lock().unwrap() = profile.into();
    rig.advance(TICK).await;
}

#[tokio::test]
async fn unplugging_switches_to_the_battery_profile_and_plugging_in_switches_back() {
    let rig = Rig::start().await;

    plug(&rig, false).await;
    assert_eq!(active(&rig), "power-saver");
    assert_eq!(watts(&rig, 0), 20); // and its bundle is applied

    plug(&rig, true).await;
    assert_eq!(active(&rig), "performance");
    assert_eq!(watts(&rig, 0), 80);
}

#[tokio::test]
async fn the_last_manual_choice_is_remembered_for_each_power_source() {
    let rig = Rig::start().await;

    choose_in_the_power_menu(&rig, "balanced").await; // on AC
    plug(&rig, false).await;
    assert_eq!(active(&rig), "power-saver");
    rig.client.set_profile("performance").await.unwrap(); // on battery, through the control API

    plug(&rig, true).await;
    assert_eq!(active(&rig), "balanced");
    plug(&rig, false).await;
    assert_eq!(active(&rig), "performance");

    // the choices are in the configuration, so they outlive the daemon
    let config: toml::Table = rig.read(Rig::config_rel()).parse().unwrap();
    assert_eq!(config["charger"]["on_ac"].as_str(), Some("balanced"));
    assert_eq!(config["charger"]["on_battery"].as_str(), Some("performance"));
}

#[tokio::test]
async fn on_battery_the_tgp_offset_is_0_whatever_the_bundle_says() {
    let rig = Rig::start().await;
    rig.client.set_config(&TEST_CONFIG.replacen("on_battery = \"power-saver\"", "on_battery = \"performance\"", 1)).await.unwrap();
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "20");

    plug(&rig, false).await;
    assert_eq!(active(&rig), "performance");
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "0");
    assert_eq!(watts(&rig, 0), 80); // the rest of the bundle stays

    plug(&rig, true).await;
    assert_eq!(rig.read(&format!("{PLATFORM}/ctgp_offset")), "20");
}

#[tokio::test]
async fn with_the_rules_turned_off_the_power_source_changes_nothing() {
    let rig = Rig::start().await;
    rig.client.set_config(&TEST_CONFIG.replacen("enabled = true", "enabled = false", 1)).await.unwrap();

    plug(&rig, false).await;
    choose_in_the_power_menu(&rig, "balanced").await;
    plug(&rig, true).await;
    plug(&rig, false).await;

    assert_eq!(active(&rig), "balanced");
    assert!(rig.profiles.sets.lock().unwrap().is_empty());
    let config: toml::Table = rig.client.config().await.unwrap().parse().unwrap();
    assert_eq!(config["charger"]["on_battery"].as_str(), Some("power-saver")); // nothing remembered either
}

#[tokio::test]
async fn a_charger_rule_naming_an_unknown_profile_is_refused() {
    let rig = Rig::start().await;
    let err = rig.client.set_config(&TEST_CONFIG.replacen("on_ac = \"performance\"", "on_ac = \"turbo\"", 1)).await.unwrap_err();
    assert!(format!("{err:#}").contains("charger.on_ac"), "{err:#}");
}
