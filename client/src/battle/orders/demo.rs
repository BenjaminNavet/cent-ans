//! Dev hook: `CENT_ANS_ORDERS_DEMO=1` plays a scripted gesture sequence through the real pointer
//! code (click, box, right click, right drag, attack) so that an agent can check selection and
//! orders without a mouse (`--dump-state` then shows the order the simulation received).

use bevy::prelude::*;

use super::BattleSelection;
use super::drag::Modifiers;
use super::input::{BattleViews, OrderLog, PointerEvent, PointerKind};
use super::picking::UnitView;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

/// Frames between two gestures (the pointer messages are applied on the next frame).
const GAP: u32 = 8;
/// Frames before the first gesture (camera and soldiers settled).
const START: u32 = 45;

#[derive(Resource, Debug, Default)]
pub struct Demo {
    frame: u32,
    step: usize,
    pub results: Vec<(String, bool, String)>,
    enabled: Option<bool>,
}

impl Demo {
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.enabled.unwrap_or(false),
            "steps_done": self.step,
            "results": self.results.iter().map(|(name, ok, note)| serde_json::json!({ "step": name, "ok": ok, "note": note })).collect::<Vec<_>>(),
        })
    }

    fn record(&mut self, name: &str, ok: bool, note: String) {
        self.results.push((name.to_owned(), ok, note));
    }
}

fn screen_of(camera: &Camera, transform: &GlobalTransform, world: Vec3) -> Option<Vec2> {
    let p = camera.world_to_viewport(transform, world).ok()?;
    let size = camera.logical_viewport_size()?;
    (p.x > 20.0 && p.y > 20.0 && p.x < size.x - 20.0 && p.y < size.y - 20.0).then_some(p)
}

fn event(kind: PointerKind, pos: Vec2) -> PointerEvent {
    PointerEvent {
        kind,
        pos,
        mods: Modifiers::default(),
    }
}

pub fn run_demo(
    mut demo: ResMut<Demo>,
    mut events: MessageWriter<PointerEvent>,
    views: Res<BattleViews>,
    selection: Res<BattleSelection>,
    log: Res<OrderLog>,
    session: Res<BattleSession>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
) {
    let enabled = *demo
        .enabled
        .get_or_insert_with(|| std::env::var("CENT_ANS_ORDERS_DEMO").is_ok_and(|v| v != "0"));
    if !enabled || demo.step >= 7 {
        return;
    }
    demo.frame += 1;
    if demo.frame < START || !(demo.frame - START).is_multiple_of(GAP) {
        return;
    }
    let Some(player) = session.player else { return };
    let Ok((camera, transform)) = cameras.single() else {
        return;
    };
    let own: Vec<&UnitView> = views
        .0
        .iter()
        .filter(|u| u.side == player && u.present)
        .collect();
    let foes: Vec<&UnitView> = views
        .0
        .iter()
        .filter(|u| u.side == player.other() && u.present)
        .collect();
    let seen = |u: &&UnitView| screen_of(camera, transform, u.pos).map(|p| (u.id, p));
    let last = |log: &OrderLog| {
        log.recent.back().map(|o| {
            (
                o.command["type"].as_str().unwrap_or("").to_owned(),
                o.result.clone(),
                o.command.clone(),
            )
        })
    };
    match demo.step {
        0 => {
            // Click on the first visible own regiment.
            let Some((id, at)) = own.iter().find_map(seen) else {
                demo.record("click_select", false, "no own regiment on screen".into());
                demo.step = 7;
                return;
            };
            demo.record("click_select", true, format!("clicked unit {id}"));
            events.write(event(PointerKind::LeftDown, at));
            events.write(event(PointerKind::LeftUp, at));
        }
        1 => {
            let ok = selection.units.len() == 1;
            demo.record(
                "click_select_result",
                ok,
                format!("selection {:?}", selection.units),
            );
            // Box over every visible own regiment.
            let points: Vec<Vec2> = own.iter().filter_map(seen).map(|(_, p)| p).collect();
            if points.is_empty() {
                demo.step = 7;
                return;
            }
            let min =
                points.iter().fold(Vec2::splat(f32::MAX), |a, p| a.min(*p)) - Vec2::splat(15.0);
            let max =
                points.iter().fold(Vec2::splat(f32::MIN), |a, p| a.max(*p)) + Vec2::splat(15.0);
            events.write(event(PointerKind::LeftDown, min));
            events.write(event(PointerKind::Move, (min + max) * 0.5));
            events.write(event(PointerKind::Move, max));
            events.write(event(PointerKind::LeftUp, max));
        }
        2 => {
            demo.record(
                "box_select_result",
                selection.units.len() > 1,
                format!("selection {:?}", selection.units),
            );
            // Right click on the ground ahead of the selection.
            let centre = own.iter().fold(Vec3::ZERO, |a, u| a + u.pos) / own.len().max(1) as f32;
            let toward = foes
                .first()
                .map_or(Vec3::Z, |f| (f.pos - centre).normalize_or_zero());
            let goal = centre + toward * 40.0;
            let Some(at) = screen_of(camera, transform, goal) else {
                demo.record("move", false, "goal off screen".into());
                demo.step = 7;
                return;
            };
            events.write(event(PointerKind::RightDown, at));
            events.write(event(PointerKind::RightUp, at));
        }
        3 => {
            let l = last(&log);
            demo.record(
                "move_result",
                l.as_ref().is_some_and(|(t, r, _)| t == "move" && r == "ok"),
                format!("{l:?}"),
            );
            // Right drag: a line to the side of the first own regiment.
            let centre = own.iter().fold(Vec3::ZERO, |a, u| a + u.pos) / own.len().max(1) as f32;
            let a = centre + Vec3::new(-30.0, 0.0, 70.0);
            let b = centre + Vec3::new(30.0, 0.0, 70.0);
            let (Some(pa), Some(pb)) = (
                screen_of(camera, transform, a),
                screen_of(camera, transform, b),
            ) else {
                demo.record("drag", false, "line off screen".into());
                demo.step = 7;
                return;
            };
            events.write(event(PointerKind::RightDown, pa));
            events.write(event(PointerKind::Move, (pa + pb) * 0.5));
            events.write(event(PointerKind::Move, pb));
            events.write(event(PointerKind::RightUp, pb));
        }
        4 => {
            let l = last(&log);
            let has_width = l.as_ref().is_some_and(|(_, _, c)| c.get("width").is_some());
            demo.record(
                "drag_result",
                l.as_ref().is_some_and(|(t, r, _)| {
                    (t == "move" && r == "ok" && has_width) || t == "move"
                }),
                format!("{l:?}"),
            );
            // Attack: right click on a visible enemy.
            let Some((id, at)) = foes.iter().find_map(seen) else {
                demo.record("attack", false, "no enemy on screen".into());
                demo.step = 7;
                return;
            };
            demo.record("attack", true, format!("clicked enemy {id}"));
            events.write(event(PointerKind::Move, at));
            events.write(event(PointerKind::RightDown, at));
            events.write(event(PointerKind::RightUp, at));
        }
        5 => {
            let l = last(&log);
            demo.record(
                "attack_result",
                l.as_ref()
                    .is_some_and(|(t, _, _)| t == "attack" || t == "pursue"),
                format!("{l:?}"),
            );
            // Park the cursor over an own regiment so the hover state and range arcs show.
            if let Some((_, at)) = own.iter().find_map(seen) {
                events.write(event(PointerKind::Move, at));
            }
        }
        _ => {}
    }
    demo.step += 1;
}
