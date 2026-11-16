mod common;
use avell::port::GpuLoad;
use common::*;
use std::time::Duration;

// TEST_CONFIG: a hold after 10 s at 50 % or more, released after 30 s below

async fn run(rig: &Rig, seconds: u64) {
    for _ in 0..seconds {
        rig.advance(Duration::from_secs(1)).await;
    }
}

fn load(rig: &Rig, percent: u32) {
    *rig.gpu.load.lock().unwrap() = Some(GpuLoad { utilization_percent: percent, power_draw_w: 50.0, power_limit_w: 95.0 });
}

fn active(rig: &Rig) -> String {
    rig.profiles.active.lock().unwrap().clone()
}

fn held(rig: &Rig) -> bool {
    !rig.profiles.holds.lock().unwrap().is_empty()
}

/// The owner on balanced, the graphics card awake and idle.
async fn desktop() -> Rig {
    let rig = Rig::start().await;
    rig.write(GPU_PM, "active");
    load(&rig, 3);
    rig.profiles.choose("balanced");
    run(&rig, 2).await;
    rig
}

#[tokio::test]
async fn sustained_gpu_load_holds_performance_and_the_owners_choice_returns_afterwards() {
    let rig = desktop().await;

    load(&rig, 95);
    run(&rig, 8).await;
    assert!(!held(&rig), "not yet: the load has to last");
    run(&rig, 6).await;
    assert!(held(&rig));
    assert_eq!(active(&rig), "performance");
    assert_eq!(watts(&rig, 0), 80); // with its bundle
    let status = rig.client.status().await.unwrap();
    assert_eq!(status.hold.as_deref(), Some("the graphics card is under sustained load"));
    assert_eq!(rig.profiles.holds.lock().unwrap()[0].1, "the graphics card is under sustained load");

    load(&rig, 5);
    run(&rig, 26).await;
    assert!(held(&rig), "not yet: the load has to stay low");
    run(&rig, 8).await;
    assert!(!held(&rig));
    assert_eq!(active(&rig), "balanced");
    assert_eq!(watts(&rig, 0), 45);
    assert_eq!(rig.client.status().await.unwrap().hold, None);

    // the hold was not the owner's choice: balanced is still what AC gets
    let config: toml::Table = rig.client.config().await.unwrap().parse().unwrap();
    assert_eq!(config["charger"]["on_ac"].as_str(), Some("balanced"));
    assert!(rig.profiles.sets.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_brief_spike_does_not_trigger_a_hold() {
    let rig = desktop().await;

    for _ in 0..5 {
        load(&rig, 95);
        run(&rig, 6).await;
        load(&rig, 5);
        run(&rig, 4).await;
    }

    assert!(!held(&rig));
    assert_eq!(active(&rig), "balanced");
}

#[tokio::test]
async fn a_dip_in_the_load_does_not_release_the_hold() {
    let rig = desktop().await;
    load(&rig, 95);
    run(&rig, 14).await;
    assert!(held(&rig));

    for _ in 0..4 {
        load(&rig, 5); // a loading screen
        run(&rig, 20).await;
        load(&rig, 95);
        run(&rig, 6).await;
    }

    assert!(held(&rig));
    assert_eq!(rig.profiles.holds.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_sleeping_graphics_card_is_never_queried() {
    let rig = desktop().await;
    rig.write(GPU_PM, "suspended");
    load(&rig, 95); // what it would say if it were woken
    let before = *rig.gpu.queries.lock().unwrap();

    run(&rig, 60).await;

    assert_eq!(*rig.gpu.queries.lock().unwrap(), before);
    assert!(!held(&rig));
}

#[tokio::test]
async fn a_manual_choice_during_a_hold_wins_until_the_load_has_ended() {
    let rig = desktop().await;
    load(&rig, 95);
    run(&rig, 14).await;
    assert!(held(&rig));

    rig.profiles.choose("power-saver"); // the power menu: this cancels the hold
    run(&rig, 40).await;
    assert_eq!(active(&rig), "power-saver");
    assert!(!held(&rig), "the same load must not take the profile back");

    load(&rig, 5); // the game ends
    run(&rig, 34).await;
    load(&rig, 95); // and another starts
    run(&rig, 14).await;
    assert!(held(&rig));
    assert_eq!(active(&rig), "performance");
}

#[tokio::test]
async fn with_detection_turned_off_nothing_is_held() {
    let rig = desktop().await;
    rig.client.set_config(&TEST_CONFIG.replacen("[game_detection]\nenabled = true", "[game_detection]\nenabled = false", 1)).await.unwrap();
    let before = *rig.gpu.queries.lock().unwrap();

    load(&rig, 95);
    run(&rig, 30).await;

    assert!(!held(&rig));
    assert_eq!(*rig.gpu.queries.lock().unwrap(), before);
}

#[tokio::test]
async fn nothing_is_held_on_battery_and_a_hold_ends_when_the_charger_is_unplugged() {
    let rig = desktop().await;
    load(&rig, 95);
    run(&rig, 14).await;
    assert!(held(&rig));

    rig.write(&format!("{AC}/online"), "0");
    run(&rig, 20).await;

    assert!(!held(&rig));
    assert_eq!(active(&rig), "power-saver");
}

#[tokio::test]
async fn a_clean_stop_releases_the_hold() {
    let rig = desktop().await;
    load(&rig, 95);
    run(&rig, 14).await;
    assert!(held(&rig));

    rig.daemon.shutdown().await;

    assert!(!held(&rig));
    assert_eq!(active(&rig), "balanced");
}
