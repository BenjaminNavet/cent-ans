//! Entry point: parses the command line and registers every plugin (ADR 0357 § 3). A lot never
//! edits this file; plugins are registered here from the skeleton on.

use std::process::ExitCode;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use cent_ans_client::{
    agent, app, audio, battle, campaign, codex, dataassets, flow, fx, menu, paths, settings, sim,
    ui,
};
use data_model::FactionId;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match agent::cli::Cli::parse(&args) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("AGENT_FATAL: {message}");
            return ExitCode::FAILURE;
        }
    };
    let root = match paths::find_root() {
        Ok(root) => root,
        Err(message) => {
            eprintln!("AGENT_FATAL: {message}");
            return ExitCode::FAILURE;
        }
    };
    let Ok(player) = FactionId::new(cli.faction.clone()) else {
        eprintln!("AGENT_FATAL: invalid faction id {}", cli.faction);
        return ExitCode::FAILURE;
    };

    let mut bevy_app = App::new();
    // `data://` asset source: before `DefaultPlugins` (ADR 0363).
    bevy_app.add_plugins(dataassets::DataSourcePlugin {
        data_dir: paths::data_dir(&root),
    });
    bevy_app.add_plugins(agent::devtools::tune_default_plugins(
        DefaultPlugins
            .set(agent::profile::log_plugin())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Cent Ans".into(),
                    resolution: WindowResolution::new(agent::VIEW_SIZE.0, agent::VIEW_SIZE.1)
                        .with_scale_factor_override(1.0),
                    present_mode: PresentMode::AutoVsync,
                    // Never take the player's focus when an agent launches the client.
                    focused: false,
                    // Agent runs never show a window (player rule: no focus).
                    visible: cli.window_visible(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: paths::asset_root(&root).to_string_lossy().into_owned(),
                ..default()
            }),
    ));
    bevy_app
        .add_plugins((
            app::AppPlugin,
            sim::SimPlugin {
                player,
                seed: cli.seed,
            },
            agent::AgentPlugin { cli: cli.clone() },
            ui::kit::UiKitPlugin,
            menu::MenuPlugin,
            audio::AudioPlugin,
            codex::CodexPlugin,
            dataassets::DataAssetsPlugin,
            settings::SettingsPlugin,
        ))
        .add_plugins((
            campaign::camera::CampaignCameraPlugin,
            campaign::terrain::CampaignTerrainPlugin,
            campaign::provinces::CampaignProvincesPlugin,
            campaign::water::CampaignWaterPlugin,
            campaign::markers::CampaignMarkersPlugin,
            campaign::armies::CampaignArmiesPlugin,
            campaign::hud::CampaignHudPlugin,
            campaign::panels::CampaignPanelsPlugin,
            campaign::vegetation::CampaignVegetationPlugin,
            campaign::life::CampaignLifePlugin,
            campaign::weather::CampaignWeatherPlugin,
            campaign::towns::CampaignTownsPlugin,
            campaign::countryside::CampaignCountrysidePlugin,
            campaign::progress::CampaignProgressPlugin,
            campaign::parchment::CampaignParchmentPlugin,
        ))
        .add_plugins((
            battle::scene::BattleScenePlugin,
            battle::units::BattleUnitsPlugin,
            battle::orders::BattleOrdersPlugin,
            battle::hud::BattleHudPlugin,
            battle::decor::BattleDecorPlugin,
            battle::siege::BattleSiegePlugin,
            battle::effects::BattleEffectsPlugin,
            flow::FlowPlugin,
        ))
        .add_plugins(fx::FlipbookPlugin)
        .add_plugins(fx::ModelMipsPlugin)
        .add_plugins(campaign::perf_switch::PerfSwitchPlugin)
        .insert_resource(cli);
    match bevy_app.run() {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(_) => ExitCode::FAILURE,
    }
}
