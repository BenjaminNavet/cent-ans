//! Real (unmuted) code path of the audio director without a window or an output device:
//! loads an MP3, spawns the voices, probes the length, crossfades on a state change.

use bevy::audio::AudioPlugin as BevyAudioPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use cent_ans_client::app::AppState;
use cent_ans_client::audio::{AudioPlugin, PlaySfx};
use cent_ans_client::paths;

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn plays_and_crossfades_without_device() {
    let root = paths::find_root().expect("repo root");
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: paths::asset_root(&root).to_string_lossy().into(),
            ..default()
        },
        StatesPlugin,
        BevyAudioPlugin::default(),
    ))
    .init_state::<AppState>()
    .add_plugins(AudioPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    run(&mut app, 5);
    let menu_voices = count_music(&mut app);
    assert_eq!(menu_voices, 1, "one music voice in the menu");
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Battle);
    run(&mut app, 5);
    assert_eq!(count_music(&mut app), 2, "old track fading out + new one");
    app.world_mut().write_message(PlaySfx::new("ui_click"));
    run(&mut app, 3);
    let sfx = cent_ans_client::audio::dump_for_tests(app.world());
    assert_eq!(sfx["playlist"], "battle");
    assert!(
        sfx["recent_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e == "ui_click")
    );
    // after the fade (1.5 s) only the battle track remains
    for _ in 0..90 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(count_music(&mut app), 1);
}

fn count_music(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query::<(&Name, &bevy::audio::AudioPlayer)>();
    query
        .iter(app.world())
        .filter(|(name, _)| name.as_str().starts_with("Music "))
        .count()
}
