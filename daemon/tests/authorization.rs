mod common;
use common::*;

#[tokio::test]
async fn the_active_desktop_user_can_change_the_profile() {
    let rig = Rig::start().await;

    rig.client.set_profile("power-saver").await.unwrap();

    assert_eq!(*rig.profiles.sets.lock().unwrap(), vec!["power-saver".to_string()]);
}

#[tokio::test]
async fn other_callers_are_refused_and_nothing_changes() {
    let rig = Rig::start().await;
    *rig.sessions.active_local_user.lock().unwrap() = Some(my_uid() + 1);

    let err = rig.client.set_profile("power-saver").await.unwrap_err();

    assert!(format!("{err:#}").contains("AccessDenied"), "{err:#}");
    assert!(rig.profiles.sets.lock().unwrap().is_empty());
}

#[tokio::test]
async fn anyone_can_read_the_status() {
    let rig = Rig::start().await;
    *rig.sessions.active_local_user.lock().unwrap() = None;

    assert_eq!(rig.client.status().await.unwrap().profile, "performance");
}
