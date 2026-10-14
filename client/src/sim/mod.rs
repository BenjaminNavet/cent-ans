//! Simulation resource: game data and campaign state held by the client, never any rule.
//!
//! Will port game/scripts/sim/ (the GDScript wrappers around the `CampaignSim` bridge). The
//! campaign starts exactly like `godot-bridge` `campaign_sim.rs::new_campaign`.

use std::sync::Arc;

use bevy::prelude::*;
use data_model::{FactionId, GameData};
use serde_json::{Value, json};
use sim_campaign::CampaignState;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::paths;

/// Default seed of a new campaign.
pub const DEFAULT_SEED: u64 = 1337;
/// Default player faction.
pub const DEFAULT_FACTION: &str = "fac_france";

#[derive(Resource)]
pub struct Sim {
    /// `None` until Boot has loaded `data/`.
    pub data: Option<Arc<GameData>>,
    /// `None` outside a campaign; reset to `None` by "new game".
    pub state: Option<CampaignState>,
    pub player: FactionId,
    pub seed: u64,
}

impl Sim {
    pub fn loaded(&self) -> bool {
        self.data.is_some()
    }

    /// Starts a 1337 campaign for `player` (title holdings synced as in the Godot bridge).
    pub fn new_campaign(&mut self) -> Result<(), String> {
        let data = self.data.clone().ok_or("game data not loaded")?;
        let mut state = CampaignState::new_1337(&data, self.player.clone(), self.seed)
            .map_err(|e| e.to_string())?;
        sim_campaign::feudal::sync_suzerains(&mut state, &data);
        self.state = Some(state);
        Ok(())
    }
}

pub struct SimPlugin {
    pub player: FactionId,
    pub seed: u64,
}

impl Default for SimPlugin {
    fn default() -> Self {
        Self {
            player: FactionId::new(DEFAULT_FACTION).expect("valid default faction id"),
            seed: DEFAULT_SEED,
        }
    }
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Sim {
            data: None,
            state: None,
            player: self.player.clone(),
            seed: self.seed,
        })
        .register_dump(dump_sim)
        .add_systems(Startup, load_data)
        .add_systems(OnEnter(AppState::Campaign), start_campaign);
    }
}

fn load_data(mut sim: ResMut<Sim>, mut exit: MessageWriter<AppExit>) {
    let loaded = paths::find_root().and_then(|root| {
        GameData::load(&paths::data_dir(&root))
            .map(|(data, warnings)| {
                info!("game data loaded ({} warnings)", warnings.len());
                data
            })
            .map_err(|e| e.to_string())
    });
    match loaded {
        Ok(data) => sim.data = Some(Arc::new(data)),
        Err(message) => {
            error!("AGENT_FATAL: game data: {message}");
            exit.write(AppExit::error());
        }
    }
}

fn start_campaign(mut sim: ResMut<Sim>, mut exit: MessageWriter<AppExit>) {
    if sim.state.is_some() {
        return;
    }
    if let Err(message) = sim.new_campaign() {
        error!("AGENT_FATAL: new campaign: {message}");
        exit.write(AppExit::error());
    }
}

/// `--dump-state` section `sim`: player faction summary (from `game-api`) and the turn.
fn dump_sim(world: &World) -> (String, Value) {
    let sim = world.resource::<Sim>();
    let value = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => json!({
            "player": sim.player.as_str(),
            "seed": sim.seed,
            "turn": state.turn,
            "year": state.year,
            "faction": game_api::faction_summary(data, state, &sim.player),
        }),
        (Some(_), None) => json!({ "player": sim.player.as_str(), "campaign": null }),
        _ => json!({ "loaded": false }),
    };
    ("sim".to_owned(), value)
}
