//! Dev hooks (see the module doc of `flow`): stage a battle or an encounter, play a battle to
//! its end and leave it. Scripts and agents only; nothing here runs without its variable.

use bevy::prelude::*;
use sim_campaign::Order;
use sim_campaign::orders::MoveOrderTarget;

use super::{FlowState, LeaveBattle, Stage};
use crate::app::AppState;
use crate::battle::scene::{BattleSession, BattleSnapshot};
use crate::sim::Sim;

pub struct FlowDevPlugin;

impl Plugin for FlowDevPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            stage_on_campaign.run_if(in_state(AppState::Campaign)),
        )
        .add_systems(
            Update,
            sim_to_end
                .run_if(in_state(AppState::Battle))
                .run_if(resource_exists::<BattleSession>),
        );
    }
}

/// Stages once, a few frames after the campaign starts.
fn stage_on_campaign(mut sim: ResMut<Sim>, mut frames: Local<u32>, mut done: Local<bool>) {
    if *done {
        return;
    }
    *frames += 1;
    if *frames < 10 {
        return;
    }
    let (Some(data), Some(_)) = (sim.data.clone(), sim.state.as_ref()) else {
        return;
    };
    *done = true;
    let player = sim.player.clone();
    let Some(state) = sim.state.as_mut() else {
        return;
    };
    if std::env::var("CENT_ANS_STAGE_BATTLE").is_ok() {
        let mine: Vec<_> = state
            .armies
            .iter()
            .filter(|(_, a)| a.faction == player)
            .map(|(id, _)| id.clone())
            .collect();
        'search: for attacker in &mine {
            let enemies: Vec<_> = state
                .armies
                .iter()
                .filter(|(_, a)| a.faction != player && state.is_at_war(&player, &a.faction))
                .map(|(id, _)| id.clone())
                .collect();
            for defender in enemies {
                if state
                    .clone()
                    .debug_stage_battle(attacker, &defender)
                    .is_ok()
                {
                    match state.debug_stage_battle(attacker, &defender) {
                        Ok(index) => {
                            info!("flow dev: staged battle {index} ({attacker} vs {defender})")
                        }
                        Err(error) => warn!("flow dev: staging failed: {error}"),
                    }
                    break 'search;
                }
            }
        }
    }
    if let Ok(encounter) = std::env::var("CENT_ANS_STAGE_ENCOUNTER") {
        let encounter_id = if encounter == "1" || encounter == "first" {
            data.encounters.keys().next().cloned()
        } else {
            data_model::EncounterId::new(encounter.as_str()).ok()
        };
        let army = state
            .armies
            .iter()
            .find(|(_, a)| a.faction == player)
            .map(|(id, a)| (id.clone(), state.army_point(&data, a)));
        if let (Some(id), Some((army_id, point))) = (encounter_id, army) {
            // A point a few cells away that still lies inside a province.
            let site = [[0.0, -8.0], [8.0, 0.0], [0.0, 8.0], [-8.0, 0.0], [0.0, 0.0]]
                .iter()
                .map(|d| [point[0] + d[0], point[1] + d[1]])
                .find_map(|p| {
                    state
                        .debug_put_encounter_site(&data, &id, p)
                        .map(|s| (s, p))
                });
            match site {
                Some((site, p)) => {
                    let order = Order::MoveArmy {
                        army: army_id.clone(),
                        target: MoveOrderTarget::Point { x: p[0], y: p[1] },
                    };
                    let result = state.submit_order(&data, order);
                    info!("flow dev: staged site {site} at {p:?}, march {result:?}");
                }
                None => warn!("flow dev: no point for the encounter site"),
            }
        }
    }
}

/// `CENT_ANS_FLOW_SIM_TO_END`: steps the battle to its end at once and leaves it;
/// `CENT_ANS_FLOW_ABANDON`: leaves it straight away as abandoned.
fn sim_to_end(
    mut session: ResMut<BattleSession>,
    flow: Res<FlowState>,
    mut leave: MessageWriter<LeaveBattle>,
    mut frames: Local<u32>,
    mut done: Local<bool>,
) {
    if *done || flow.stage != Stage::InBattle {
        return;
    }
    *frames += 1;
    if *frames < 30 {
        return;
    }
    if std::env::var("CENT_ANS_FLOW_ABANDON").is_ok() {
        *done = true;
        leave.write(LeaveBattle { abandoned: true });
    } else if std::env::var("CENT_ANS_FLOW_SIM_TO_END").is_ok() {
        *done = true;
        let sim = session.sim_mut();
        let mut guard = 0;
        while !sim.is_finished() && guard < 200_000 {
            sim.step();
            guard += 1;
        }
        let snapshot = BattleSnapshot::of(session.sim_mut());
        session.snapshot = snapshot;
        info!("flow dev: battle played in {guard} steps");
        leave.write(LeaveBattle { abandoned: false });
    }
}
