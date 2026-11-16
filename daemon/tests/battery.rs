mod common;
use common::*;

/// The battery files as the driver exposes them once battery care is enabled for this board.
async fn with_battery_care() -> Rig {
    let rig = Rig::start().await;
    rig.write(&format!("{BAT}/charge_types"), "[Standard] Long_Life Trickle");
    rig.write(&format!("{BAT}/charge_control_end_threshold"), "100");
    rig
}

#[tokio::test]
async fn the_charging_profile_is_shown_and_set_by_the_firmwares_names() {
    let rig = with_battery_care().await;
    assert_eq!(rig.client.status().await.unwrap().battery.charging_profile.as_deref(), Some("high-capacity"));

    rig.client.set_charging_profile("stationary").await.unwrap();
    assert_eq!(rig.read(&format!("{BAT}/charge_types")), "Trickle");

    // the kernel marks the selected one with brackets
    rig.write(&format!("{BAT}/charge_types"), "Standard [Long_Life] Trickle");
    assert_eq!(rig.client.status().await.unwrap().battery.charging_profile.as_deref(), Some("balanced"));

    let err = rig.client.set_charging_profile("fast").await.unwrap_err();
    assert!(format!("{err:#}").contains("high-capacity, balanced or stationary"), "{err:#}");
}

#[tokio::test]
async fn the_charge_cap_is_shown_and_set() {
    let rig = with_battery_care().await;
    assert_eq!(rig.client.status().await.unwrap().battery.charge_cap_percent, Some(100));

    rig.client.set_charge_cap(80).await.unwrap();
    assert_eq!(rig.read(&format!("{BAT}/charge_control_end_threshold")), "80");

    for bad in [0, 19, 101] {
        assert!(rig.client.set_charge_cap(bad).await.is_err(), "{bad}");
    }
    assert_eq!(rig.read(&format!("{BAT}/charge_control_end_threshold")), "80");
}

#[tokio::test]
async fn battery_care_the_driver_does_not_expose_is_unavailable() {
    let rig = Rig::start().await; // this board today: neither file

    let status = rig.client.status().await.unwrap();
    assert_eq!((status.battery.charging_profile, status.battery.charge_cap_percent), (None, None));
    assert!(status.unavailable.contains(&"charging_profile".to_string()));
    assert!(status.unavailable.contains(&"charge_cap".to_string()));
    for err in [rig.client.set_charging_profile("balanced").await.unwrap_err(), rig.client.set_charge_cap(80).await.unwrap_err()] {
        assert!(format!("{err:#}").contains("not exposed"), "{err:#}");
    }
}

#[tokio::test]
async fn only_the_active_desktop_user_may_change_battery_care() {
    let rig = with_battery_care().await;
    *rig.sessions.active_local_user.lock().unwrap() = None;

    assert!(rig.client.set_charging_profile("balanced").await.is_err());
    assert!(rig.client.set_charge_cap(80).await.is_err());
    assert_eq!(rig.read(&format!("{BAT}/charge_control_end_threshold")), "100");
}
