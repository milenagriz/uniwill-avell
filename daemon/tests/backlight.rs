mod common;
use common::*;

#[tokio::test]
async fn the_keyboard_brightness_can_be_set_up_to_the_keyboards_maximum() {
    let rig = Rig::start().await;

    rig.client.set_keyboard_brightness(2).await.unwrap();
    assert_eq!(rig.read(&format!("{KBD}/brightness")), "2");
    assert_eq!(rig.client.status().await.unwrap().backlight.brightness, Some(2));

    let err = rig.client.set_keyboard_brightness(5).await.unwrap_err();
    assert!(format!("{err:#}").contains("0 to 4"), "{err:#}");
    assert_eq!(rig.read(&format!("{KBD}/brightness")), "2");
}

#[tokio::test]
async fn the_keyboard_colour_is_given_as_rgb_and_scaled_to_what_the_keyboard_takes() {
    let rig = Rig::start().await;

    rig.client.set_keyboard_color([255, 128, 0]).await.unwrap();

    // this keyboard takes 0-50 per channel
    assert_eq!(rig.read(&format!("{KBD}/multi_intensity")), "50 25 0");
    assert_eq!(rig.client.status().await.unwrap().backlight.color, Some([255, 128, 0]));
}

#[tokio::test]
async fn without_a_keyboard_backlight_setting_it_says_so() {
    let rig = Rig::start().await;
    for f in ["brightness", "max_brightness", "multi_intensity", "multi_index", "multi_max_intensity"] {
        rig.remove(&format!("{KBD}/{f}"));
    }

    let err = rig.client.set_keyboard_brightness(1).await.unwrap_err();

    assert!(format!("{err:#}").contains("not exposed"), "{err:#}");
}

#[tokio::test]
async fn only_the_active_desktop_user_may_change_the_backlight() {
    let rig = Rig::start().await;
    *rig.sessions.active_local_user.lock().unwrap() = None;

    assert!(rig.client.set_keyboard_brightness(0).await.is_err());
    assert!(rig.client.set_keyboard_color([1, 2, 3]).await.is_err());
    assert_eq!(rig.read(&format!("{KBD}/brightness")), "1");
}
