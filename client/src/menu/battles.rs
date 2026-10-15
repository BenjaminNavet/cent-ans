//! « Batailles » of the main menu (game/scripts/ui/{battles_menu, custom_battle_screen,
//! historical_battles_menu, battle_demos_menu}.gd): the three screens that build a
//! [`PendingBattle`] and enter [`AppState::Battle`].
//!
//! - `custom_battle`: custom battle (armies, budgets, field, siege), kept armies (`presets`).
//! - `historical`: Crécy, Poitiers, Azincourt (`data/battle_maps/`), lead a side or watch.
//! - `demos`: `data/ui/battle_demos.json`, with the hour of the battle.
//!
//! Every screen sends [`LaunchBattle`]; the system here fills [`PendingBattle`] (the battle scene
//! consumes it on entry) and changes state. Not ported: the battle tutorial invite
//! (`battle_prologue_invite.gd`), the scene fader, « Rejeux » (needs a campaign).

use std::path::PathBuf;

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_battle::custom::CustomBattle;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::PendingBattle;
use crate::paths;

/// Which screen to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleScreen {
    Custom,
    Historical,
    Demos,
}

/// Opens one of the battle screens above the « Batailles » menu.
#[derive(Message, Debug, Clone, Copy)]
pub struct OpenBattleScreen(pub BattleScreen);

/// A screen asks for a battle: [`PendingBattle`] is filled and the battle screen opens.
#[derive(Message, Debug, Clone)]
pub struct LaunchBattle {
    pub config: CustomBattle,
    pub seed: u64,
    /// What was launched (`custom`, `historical:crecy:defender`, `demo:siege_paris`), for the dump.
    pub label: String,
}

/// A historical battle was chosen: the battle scene may build the scripted scenario of
/// `data/battle_maps/<id>.json` (orders of battle, waves, relief) instead of the custom-battle
/// approximation in [`PendingBattle`]. Removed by whoever consumes it.
#[derive(Resource, Debug, Clone)]
pub struct PendingHistorical {
    pub id: String,
    /// `attacker`, `defender` or empty (AI against AI).
    pub side: String,
}

/// What the screens remember between openings and what the last launch was.
#[derive(Resource, Default, Debug)]
pub struct BattleMemory {
    pub last_launch: Option<String>,
    /// Hour chosen on the demos screen (phase key, empty = drawn), kept from one demo to the next.
    pub demo_hour: String,
}

/// A phase of the day (`data/rules/battle_time_of_day.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct DayPhase {
    pub key: String,
    pub label: String,
    pub from_hour: f64,
    pub to_hour: f64,
    pub selectable: bool,
}

impl DayPhase {
    fn contains(&self, hour: f64) -> bool {
        if self.from_hour <= self.to_hour {
            hour >= self.from_hour && hour < self.to_hour
        } else {
            hour >= self.from_hour || hour < self.to_hour
        }
    }
}

/// Phases of the day, in file order (empty when the file is unreadable).
pub fn day_phases(data_dir: &std::path::Path) -> Vec<DayPhase> {
    let parsed = std::fs::read_to_string(data_dir.join("rules/battle_time_of_day.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let Some(phases) = parsed.as_ref().and_then(|v| v["phases"].as_array()) else {
        return Vec::new();
    };
    phases
        .iter()
        .filter_map(|p| {
            Some(DayPhase {
                key: p["key"].as_str()?.to_owned(),
                label: p["label"].as_str()?.to_owned(),
                from_hour: p["from_hour"].as_f64()?,
                to_hour: p["to_hour"].as_f64()?,
                selectable: p["selectable"].as_bool().unwrap_or(true),
            })
        })
        .collect()
}

/// Key of the phase of the day containing `hour` (empty when none does).
pub fn phase_of_hour(phases: &[DayPhase], hour: f64) -> String {
    phases
        .iter()
        .find(|p| p.contains(hour))
        .map(|p| p.key.clone())
        .unwrap_or_default()
}

/// `data/` of the repository, when found.
pub fn data_dir() -> Option<PathBuf> {
    paths::find_root().ok().map(|root| paths::data_dir(&root))
}

/// Folder of the per-user files (kept armies, last composition): the `Cent Ans` folder holding
/// the saves folder, or `CENT_ANS_PREFS` when set.
pub fn prefs_dir(save_dir: &std::path::Path) -> PathBuf {
    if let Some(dir) = std::env::var_os("CENT_ANS_PREFS") {
        return PathBuf::from(dir);
    }
    save_dir
        .parent()
        .map_or_else(|| save_dir.to_owned(), Into::into)
}

pub struct BattlesPlugin;

impl Plugin for BattlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenBattleScreen>()
            .add_message::<LaunchBattle>()
            .init_resource::<BattleMemory>()
            .register_dump(dump_battles)
            .add_plugins((
                super::custom_battle::CustomBattlePlugin,
                super::historical::HistoricalPlugin,
                super::demos::DemosPlugin,
            ))
            .add_systems(Update, launch_battle.run_if(in_state(AppState::MainMenu)));
    }
}

fn launch_battle(
    mut commands: Commands,
    mut launches: MessageReader<LaunchBattle>,
    mut memory: ResMut<BattleMemory>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(launch) = launches.read().last().cloned() else {
        return;
    };
    info!("battle launch: {}", launch.label);
    commands.insert_resource(PendingBattle {
        config: launch.config,
        seed: launch.seed,
    });
    commands.remove_resource::<PendingHistorical>();
    if let Some(rest) = launch.label.strip_prefix("historical:") {
        let mut parts = rest.split(':');
        commands.insert_resource(PendingHistorical {
            id: parts.next().unwrap_or_default().to_owned(),
            side: parts.next().unwrap_or_default().to_owned(),
        });
    }
    memory.last_launch = Some(launch.label);
    next.set(AppState::Battle);
}

fn dump_battles(world: &World) -> (String, Value) {
    let memory = world.resource::<BattleMemory>();
    let pending = world.get_resource::<PendingBattle>().map(|p| {
        json!({
            "attacker": p.config.attacker.faction,
            "defender": p.config.defender.faction,
            "attacker_units": p.config.attacker.units.len(),
            "defender_units": p.config.defender.units.len(),
            "terrain": p.config.terrain,
            "season": p.config.season,
            "weather": p.config.weather,
            "hour": p.config.hour,
            "siege": p.config.siege,
            "player_side": p.config.player_side,
            "year": p.config.year,
            "seed": p.seed,
        })
    });
    let value = json!({
        "last_launch": memory.last_launch,
        "demo_hour": memory.demo_hour,
        "pending_battle": pending,
        "pending_historical": world.get_resource::<PendingHistorical>().map(|h| json!({"id": h.id, "side": h.side})),
        "custom": super::custom_battle::dump(world),
        "historical": super::historical::dump(world),
        "demos": super::demos::dump(world),
    });
    ("menu_battles".to_owned(), value)
}
