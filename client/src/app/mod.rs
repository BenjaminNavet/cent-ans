//! Application states (ADR 0357 § 4) and the Boot -> first screen transition.
//!
//! Will port the scene switching of game/scripts/ui/ (start menu, faction select) and the
//! campaign <-> battle hand-over of game/scripts/map/campaign_map.gd. Screen entities carry
//! `DespawnOnExit(state)`.

pub mod window_keepalive;

use bevy::prelude::*;

use crate::agent::cli::Cli;
use crate::agent::dump::DumpAppExt;
use crate::sim::Sim;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Boot,
    MainMenu,
    FactionSelect,
    Campaign,
    Battle,
    Codex,
}

impl AppState {
    pub const ALL: [AppState; 6] = [
        AppState::Boot,
        AppState::MainMenu,
        AppState::FactionSelect,
        AppState::Campaign,
        AppState::Battle,
        AppState::Codex,
    ];

    /// Parses a state name as typed on the command line or in a script (case-insensitive).
    pub fn from_name(name: &str) -> Option<AppState> {
        let wanted = name.trim();
        Self::ALL
            .into_iter()
            .find(|state| format!("{state:?}").eq_ignore_ascii_case(wanted))
    }
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(Update, leave_boot.run_if(in_state(AppState::Boot)))
            .add_systems(PostUpdate, window_keepalive::keep_primary_window)
            .register_dump(|_| {
                (
                    "window_keepalive".into(),
                    serde_json::json!({ "kept": window_keepalive::kept_windows() }),
                )
            });
    }
}

/// Leaves `Boot` once the simulation data is loaded: first screen is `--state` or the main menu.
fn first_screen(cli: Option<&Cli>) -> AppState {
    cli.and_then(|cli| cli.state)
        .filter(|state| *state != AppState::Boot)
        .unwrap_or(AppState::MainMenu)
}

fn leave_boot(sim: Res<Sim>, cli: Option<Res<Cli>>, mut next: ResMut<NextState<AppState>>) {
    if sim.loaded() {
        next.set(first_screen(cli.as_deref()));
    }
}
