//! Dev hook `CENT_ANS_PROLOGUE_DEMO=1`: plays every step of the battle tutorial through the real
//! order messages (selection, `Command`s, camera pose, pause, the continue buttons) so that an
//! agent can see the whole traversal in the dump without a mouse. Section
//! `battle_prologue.demo`. The victory itself is played by the AI of the player's side, sped up.

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_battle::Command;

use super::PrologueRun;
use crate::battle::orders::SelectBattleUnits;
use crate::battle::orders::input::IssueBattleCommand;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCameraState;
use crate::ui::kit::ButtonPressed;

/// Frames after the start of a step before the scripted gesture (the baseline is taken first).
const DELAY: u32 = 8;

#[derive(Resource, Default, Debug)]
pub struct PrologueDemo {
    enabled: Option<bool>,
    step: Option<usize>,
    frames: u32,
    acted: bool,
    pub log: Vec<(String, String)>,
}

pub fn report(world: &World) -> Value {
    let Some(demo) = world.get_resource::<PrologueDemo>() else {
        return json!({ "enabled": false });
    };
    json!({
        "enabled": demo.enabled.unwrap_or(false),
        "log": demo.log.iter().map(|(s, n)| json!({"step": s, "did": n})).collect::<Vec<_>>(),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn run_demo(
    mut demo: ResMut<PrologueDemo>,
    run: Res<PrologueRun>,
    mut session: ResMut<BattleSession>,
    camera: Option<ResMut<BattleCameraState>>,
    mut selects: MessageWriter<SelectBattleUnits>,
    mut orders: MessageWriter<IssueBattleCommand>,
    mut pressed: MessageWriter<ButtonPressed>,
) {
    let enabled = *demo
        .enabled
        .get_or_insert_with(|| std::env::var("CENT_ANS_PROLOGUE_DEMO").is_ok_and(|v| v != "0"));
    if !enabled || run.done || run.defeated || !run.passive_set {
        return;
    }
    if demo.step != Some(run.step) {
        demo.step = Some(run.step);
        demo.frames = 0;
        demo.acted = false;
    }
    demo.frames += 1;
    if demo.acted || demo.frames < DELAY || run.done_timer.is_some() {
        return;
    }
    demo.acted = true;
    let Some(player) = session.player else { return };
    let id = run.current().id.clone();
    if session.speed < 4.0 && !session.paused {
        session.speed = 4.0;
    }
    let sim = session.sim_mut();
    let own: Vec<(u32, data_model::UnitCategory, f64, f64, f64)> = sim
        .units()
        .iter()
        .filter(|u| u.side == player && u.present() && !u.synthetic)
        .map(|u| (u.id, u.category, u.x, u.z, u.extent().0))
        .collect();
    let enemies: Vec<(u32, f64, f64)> = sim
        .units()
        .iter()
        .filter(|u| u.side != player && u.present() && !u.synthetic)
        .map(|u| (u.id, u.x, u.z))
        .collect();
    let first = |category: data_model::UnitCategory| own.iter().find(|u| u.1 == category).copied();
    let Some(any_enemy) = enemies.first().copied() else {
        return;
    };
    let did: String = match id.as_str() {
        "intro" | "outro" => {
            pressed.write(ButtonPressed {
                entity: Entity::PLACEHOLDER,
                name: "prologue:next".to_owned(),
            });
            "button prologue:next".to_owned()
        }
        "camera" => {
            if let Some(mut camera) = camera {
                camera.target.x += 80.0;
            }
            "camera pan +80 m".to_owned()
        }
        "select" => {
            let ids: Vec<u32> = own.iter().map(|u| u.0).take(1).collect();
            selects.write(SelectBattleUnits {
                units: ids.clone(),
                additive: false,
            });
            format!("select {ids:?}")
        }
        "move" => {
            let Some(unit) = first(data_model::UnitCategory::Infantry) else {
                return;
            };
            let dir = (any_enemy.2 - unit.3).signum();
            orders.write(IssueBattleCommand(Command::Move {
                units: vec![unit.0],
                x: unit.2,
                z: unit.3 + 45.0 * dir,
                run: true,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            }));
            format!("move {} by 45 m", unit.0)
        }
        "formation" => {
            let Some(unit) = first(data_model::UnitCategory::Infantry) else {
                return;
            };
            orders.write(IssueBattleCommand(Command::Move {
                units: vec![unit.0],
                x: unit.2,
                z: unit.3,
                run: false,
                facing: None,
                queue: false,
                width: Some(unit.4 * 0.5),
                match_speed: false,
                group_tag: None,
            }));
            format!("narrow the front of {} to {:.0} m", unit.0, unit.4 * 0.5)
        }
        "charge" => {
            let Some(unit) = first(data_model::UnitCategory::Cavalry) else {
                return;
            };
            let nearest = enemies
                .iter()
                .min_by(|a, b| {
                    let d = |e: &&(u32, f64, f64)| (e.1 - unit.2).hypot(e.2 - unit.3);
                    d(a).total_cmp(&d(b))
                })
                .map_or(any_enemy.0, |e| e.0);
            orders.write(IssueBattleCommand(Command::Attack {
                units: vec![unit.0],
                target: nearest,
                run: true,
                queue: false,
            }));
            format!("charge {} on {nearest}", unit.0)
        }
        "fire" => {
            let archers: Vec<u32> = own
                .iter()
                .filter(|u| u.1 == data_model::UnitCategory::Ranged)
                .map(|u| u.0)
                .collect();
            orders.write(IssueBattleCommand(Command::Attack {
                units: archers.clone(),
                target: any_enemy.0,
                run: false,
                queue: false,
            }));
            format!("archers {archers:?} on {}", any_enemy.0)
        }
        "pause" => {
            session.paused = true;
            "pause".to_owned()
        }
        "victory" => {
            session.paused = false;
            session.speed = 4.0;
            session.sim_mut().set_ai(player, true);
            "resume at x4, AI plays the player's side".to_owned()
        }
        _ => return,
    };
    demo.log.push((id, did));
}
