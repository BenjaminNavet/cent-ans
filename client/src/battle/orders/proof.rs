//! Dev hook (agents): `CENT_ANS_BDEPLOY_DEMO=1` plays the deployment lot end to end without a
//! mouse: place two regiments, propose and validate a preset formation, start the battle, queue
//! two moves (numbered waypoints and arrival ghosts), Alt+1 (ability), Ctrl+F2 / F2 (camera
//! bookmark). Results in the dump section `battle_bdeploy_demo` (and `battle_orders`).

use bevy::prelude::*;
use sim_battle::{Command, SideId};

use super::BattleSelection;
use super::SelectBattleUnits;
use super::deploy::Deployment;
use super::input::{IssueBattleCommand, MoveSelectionTo, OrderLog};
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCameraState;
use crate::ui::kit::ButtonPressed;

#[derive(Resource, Debug, Default)]
pub struct BdeployDemo {
    enabled: Option<bool>,
    frame: u32,
    pub notes: Vec<(String, serde_json::Value)>,
}

impl BdeployDemo {
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.enabled.unwrap_or(false),
            "frame": self.frame,
            "notes": self.notes.iter().map(|(k, v)| serde_json::json!({ "step": k, "value": v })).collect::<Vec<_>>(),
        })
    }
}

fn press(pressed: &mut MessageWriter<ButtonPressed>, name: &str) {
    pressed.write(ButtonPressed {
        entity: Entity::PLACEHOLDER,
        name: name.to_owned(),
    });
}

#[allow(clippy::too_many_arguments)]
pub fn run_bdeploy_demo(
    mut demo: ResMut<BdeployDemo>,
    mut session: ResMut<BattleSession>,
    deployment: Res<Deployment>,
    selection: Res<BattleSelection>,
    log: Res<OrderLog>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut selects: MessageWriter<SelectBattleUnits>,
    mut moves: MessageWriter<MoveSelectionTo>,
    mut pressed: MessageWriter<ButtonPressed>,
    mut orders: MessageWriter<IssueBattleCommand>,
    camera: Option<ResMut<BattleCameraState>>,
) {
    let enabled = *demo
        .enabled
        .get_or_insert_with(|| std::env::var("CENT_ANS_BDEPLOY_DEMO").is_ok_and(|v| v != "0"));
    if !enabled {
        return;
    }
    demo.frame += 1;
    let frame = demo.frame;
    let Some(player) = session.player else { return };
    let sim = session.sim_mut();
    let mine: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == player && u.present() && !u.synthetic)
        .map(|u| u.id)
        .collect();
    let positions = |sim: &sim_battle::BattleSim, ids: &[u32]| {
        serde_json::json!(
            ids.iter()
                .filter_map(|id| sim.unit(*id))
                .map(|u| serde_json::json!({ "id": u.id, "x": u.x.round(), "z": u.z.round(), "facing": (u.facing * 100.0).round() / 100.0 }))
                .collect::<Vec<_>>()
        )
    };
    match frame {
        20 => {
            let pair: Vec<u32> = mine.iter().copied().take(2).collect();
            demo.notes
                .push(("before_place".into(), positions(sim, &pair)));
            demo.notes.push((
                "deploying".into(),
                serde_json::json!({ "active": deployment.active, "zones": deployment.zones, "ticks": sim.ticks() }),
            ));
            selects.write(SelectBattleUnits {
                units: pair,
                additive: false,
            });
        }
        25 => {
            if let Some(zone) = deployment.zones.first() {
                let point = Vec3::new(
                    (zone[0] + zone[2]) * 0.5,
                    0.0,
                    if player == SideId::Attacker {
                        zone[3] - 40.0
                    } else {
                        zone[1] + 40.0
                    },
                );
                moves.write(MoveSelectionTo {
                    point,
                    queued: false,
                    run: false,
                });
            }
        }
        30 => {
            let pair = selection.units.clone();
            demo.notes
                .push(("after_place".into(), positions(sim, &pair)));
            press(&mut pressed, "formation:harrow");
        }
        32 => press(&mut pressed, "formation:place"),
        34 => {
            demo.notes.push((
                "after_proposal_ticks".into(),
                serde_json::json!(sim.ticks()),
            ));
            press(&mut pressed, "formation:place");
        }
        38 => {
            demo.notes
                .push(("after_preset_deploy".into(), positions(sim, &mine)));
            press(&mut pressed, "formation:harrow"); // preset off
        }
        40 => press(&mut pressed, "deploy:start"),
        60 => {
            demo.notes.push((
                "started".into(),
                serde_json::json!({ "ticks": sim.ticks(), "deploying": sim.is_deploying() }),
            ));
            if let Some(first) = mine.first() {
                selects.write(SelectBattleUnits {
                    units: vec![*first],
                    additive: false,
                });
            }
        }
        65 => {
            if let Some(unit) = mine.first().and_then(|id| sim.unit(*id)) {
                let ahead = if player == SideId::Attacker {
                    1.0
                } else {
                    -1.0
                };
                for step in 1..=2 {
                    orders.write(IssueBattleCommand(Command::Move {
                        units: vec![unit.id],
                        x: unit.x + 30.0 * f64::from(step),
                        z: unit.z + ahead * 40.0 * f64::from(step),
                        run: false,
                        facing: None,
                        queue: step > 1 || unit.destination.is_some(),
                        width: None,
                        match_speed: false,
                        group_tag: None,
                    }));
                }
            }
        }
        80 => keys.press(KeyCode::AltLeft),
        81 => keys.press(KeyCode::Digit1),
        82 => {
            keys.release(KeyCode::Digit1);
            keys.release(KeyCode::AltLeft);
        }
        90 => {
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::F2);
        }
        91 => {
            keys.release(KeyCode::F2);
            keys.release(KeyCode::ControlLeft);
            if let Some(mut state) = camera {
                demo.notes.push((
                    "bookmark_saved_at".into(),
                    serde_json::json!([state.target.x.round(), state.target.z.round()]),
                ));
                state.target += Vec3::new(120.0, 0.0, 0.0);
            }
        }
        100 => keys.press(KeyCode::F2),
        101 => {
            keys.release(KeyCode::F2);
        }
        140 => {
            if let Some(state) = camera {
                demo.notes.push((
                    "bookmark_after_recall".into(),
                    serde_json::json!([state.target.x.round(), state.target.z.round()]),
                ));
            }
            demo.notes.push((
                "last_orders".into(),
                serde_json::json!(
                    log.recent
                        .iter()
                        .map(
                            |o| serde_json::json!({ "cmd": o.command["type"], "result": o.result })
                        )
                        .collect::<Vec<_>>()
                ),
            ));
        }
        _ => {}
    }
}
