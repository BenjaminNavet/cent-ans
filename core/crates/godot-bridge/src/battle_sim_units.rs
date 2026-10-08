//! `BattleSim`: the units and soldier buffers handed to the renderer.

use data_model::{Ability, UnitCategory};
use godot::prelude::*;
use sim_battle::{Unit, UnitState};

use crate::battle_pose_lerp::push_pose;
use crate::battle_sim::{parse_side, BattleSim};

/// Rendering family of a regiment: `infantry`, `archer`, `cavalry`, `siege`,
/// or `ram` / `tower` (siege battles: drawn as one machine, not soldiers).
fn render_key(unit: &Unit) -> &'static str {
    if unit.ram {
        return "ram";
    }
    if unit.siege_tower() {
        return "tower";
    }
    match unit.category {
        UnitCategory::Siege => "siege",
        _ if unit.mounted => "cavalry",
        UnitCategory::Ranged => "archer",
        UnitCategory::Cavalry => "cavalry",
        UnitCategory::Infantry => "infantry",
    }
}

fn category_key(category: UnitCategory) -> &'static str {
    match category {
        UnitCategory::Infantry => "infantry",
        UnitCategory::Ranged => "ranged",
        UnitCategory::Cavalry => "cavalry",
        UnitCategory::Siege => "siege",
    }
}

fn state_label_fr(state: UnitState) -> &'static str {
    match state {
        UnitState::Idle => "au repos",
        UnitState::Marching => "en marche",
        UnitState::Charging => "charge",
        UnitState::Melee => "mêlée",
        UnitState::Shooting => "tir",
        UnitState::Routing => "déroute",
        UnitState::Rallied => "rallié",
        UnitState::Climbing => "escalade",
    }
}

/// Godot-facing handle on one battle (spec § 3).

#[godot_api(secondary)]
impl BattleSim {
    /// One dictionary per regiment (spec § 3), plus rendering helpers.
    /// PB3c: rebuilt only after a simulation step or a change of the battle
    /// (same key as the figure poses); in between the same array comes back —
    /// read-only for the caller.
    #[func]
    /// RJ-b: `x`, `y`, `z`, `facing` blended between the two latest steps
    /// with `set_pose_lerp` (the dictionaries change in place each frame),
    /// `ground_speed` = m/s of the centre over the latest step.
    fn get_units(&mut self) -> VarArray {
        self.units_now()
    }

    /// The dictionaries of [`Self::get_units`].
    pub(crate) fn build_units(&self) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        sim.units()
            .iter()
            .map(|unit| {
                let (width, depth) = unit.extent();
                let mut dict = vdict! {
                    "id" => i64::from(unit.id),
                    "side" => unit.side.key(),
                    "type" => unit.unit_type.as_str(),
                    "name" => unit.name.as_str(),
                    "category" => category_key(unit.category),
                    "render" => render_key(unit),
                    "soldiers" => i64::from(unit.soldiers()),
                    "figures" => i64::from(unit.figure_count(self.figure_scale)),
                    "max_soldiers" => i64::from(unit.max_soldiers),
                    "initial_soldiers" => i64::from(unit.initial_soldiers),
                    "kills" => unit.kills.round() as i64,
                    "morale" => unit.morale,
                    "fatigue" => unit.fatigue,
                    "ammo" => i64::from(unit.ammo),
                    "max_ammo" => i64::from(if unit.can_shoot() { unit.stats.ammo } else { 0 }),
                    // SG2: seconds before the next shot and the full reload (engines
                    // are wound back over it).
                    "reload" => unit.reload.max(0.0),
                    "reload_period" => unit.reload_period(),
                    "state" => unit.state.key(),
                    "state_label" => state_label_fr(unit.state),
                    "formation" => unit.formation.key(),
                    "x" => unit.x,
                    "z" => unit.z,
                    "y" => sim.standing_height(unit, unit.x, unit.z),
                    "facing" => unit.facing,
                    "width" => width,
                    "depth" => depth,
                    "is_general" => unit.is_general,
                    // CB3: fog of war for the tactical view (Tab) — purely derived, not part of
                    // the core state (`state_digest`/replay): own side always spotted; no player
                    // side set (spectated battle) shows everyone, as the normal view already does.
                    "spotted" => sim
                        .setup()
                        .player_side
                        .is_none_or(|side| sim.spotted_by(unit, side)),
                    "present" => unit.present(),
                    "reserve" => unit.reserve,
                    "left_field" => unit.left_field,
                    // Q2: end-of-battle fate (destroyed, routed, withdrawn, reserve, held).
                    "fate" => unit.fate().key(),
                    "fire_at_will" => unit.fire_at_will,
                    "can_shoot" => unit.can_shoot(),
                    "running" => unit.running,
                    "withdrawing" => unit.withdrawing,
                    "stakes" => unit.stakes_planted,
                    "target" => unit.target.map_or(-1, i64::from),
                    "on_wall" => unit.on_wall,
                    "climbing" => unit.climbing.map_or(-1, |p| p as i64),
                    "climb_progress" => unit.climb_progress,
                    "ladders" => sim.on_ladders(unit),
                    "synthetic" => unit.synthetic,
                    "ram" => unit.ram,
                    "siege_tower" => unit.siege_tower(),
                    "wall_breaker" => unit.wall_breaker(),
                    "pavise" => unit.pavise.is_some(),
                    "pavise_cover" => unit.pavise.is_some()
                        || (unit.has(Ability::Pavise) && unit.state != UnitState::Marching),
                    "dismounted" => unit.dismounted,
                    "order_morale" => unit.order_morale,
                    // BV2: cause of the latest deaths (chooses the death drawn).
                    "loss_cause" => unit.loss_cause.key(),
                    "loss_by" => unit.loss_by.map_or(-1, i64::from),
                    "knocked" => if unit.knocked_timer > 0.0 { unit.knocked } else { 0.0 },
                    // CB-M4: range drawn on the ground (0 without missiles) and the
                    // half-angle of the drawn sector (radians, `battle_hover.json`).
                    "effective_range" => sim.ground_range(unit),
                    "fire_arc" => sim_battle::HoverRules::bundled().range_arc.fire_half_angle(),
                };
                if let Some((x, z)) = unit.destination {
                    dict.set("destination", Vector2::new(x as f32, z as f32));
                }
                // CB1: files of a dragged Line (-1: default depth), and the
                // tag of the grouped order it walks under (-1: none).
                dict.set("line_files", unit.line_files.map_or(-1, i64::from));
                dict.set("group_tag", unit.group_tag.map_or(-1, i64::from));
                dict.set("match_speed", unit.match_speed);
                // CB-M3: orders waiting behind the current one.
                dict.set("queue", &crate::battle_sim_queue::queue_array(sim, unit));
                // CB2: modes on, modes available, states for the badges.
                crate::battle_sim_modes::add_mode_fields(sim, unit, &mut dict);
                crate::battle_sim_formation::add_formation_fields(unit, &mut dict);
                // CB4: the regiment's abilities (buttons of its card).
                crate::battle_sim_abilities::add_ability_fields(sim, unit, &mut dict);
                // EP11: push of the lines in melee (m/s, > 0 driving the enemy
                // back, < 0 giving ground), compression (0-1), ground given (m).
                dict.set("push_speed", unit.push.speed);
                dict.set("compression", unit.push.compression);
                dict.set("ground_lost", unit.push.ground_lost);
                // EP5: the regiment's standard (`carried`, `fallen`, `captured`,
                // `lost`), where it lies on the ground, the regiment that took it,
                // and the figures of the buffer that carry it.
                let bearers = sim.standard_bearers(unit.id as usize);
                if bearers > 0 {
                    dict.set("standard", unit.standard.key());
                    if unit.is_general {
                        let sovereign = sim
                            .setup()
                            .side(unit.side)
                            .general
                            .as_ref()
                            .is_some_and(|g| g.sovereign);
                        dict.set("sovereign", sovereign);
                    }
                    if let Some((x, z)) = unit.standard.ground() {
                        dict.set("standard_x", x);
                        dict.set("standard_z", z);
                        dict.set("standard_y", sim.field().height(x, z));
                    }
                    if let sim_battle::unit::StandardState::Fallen { timer, .. } = unit.standard {
                        dict.set("standard_timer", timer);
                    }
                    if let sim_battle::unit::StandardState::Captured { by } = unit.standard {
                        dict.set(
                            "standard_by",
                            if by == u32::MAX { -1 } else { i64::from(by) },
                        );
                    }
                    let slots: PackedInt32Array = unit
                        .standard_slots(self.figure_scale, bearers)
                        .iter()
                        .map(|&s| s as i32)
                        .collect();
                    dict.set("bearer_slots", &slots);
                }
                // SG1: the first `climbers_shown` soldiers of the buffer are on
                // the ladders / bridge; `ladder_lines` = [foot, top] per ladder
                // as Vector3 pairs (ground and crenel heights).
                if unit.climbing.is_some() {
                    dict.set(
                        "climbers_shown",
                        sim.climbers_shown(unit, self.figure_scale) as i64,
                    );
                    let lines: PackedVector3Array = sim
                        .ladders(unit)
                        .iter()
                        .flat_map(|l| {
                            [
                                Vector3::new(l.foot.0 as f32, l.foot_y as f32, l.foot.1 as f32),
                                Vector3::new(l.top.0 as f32, l.top_y as f32, l.top.1 as f32),
                            ]
                        })
                        .collect();
                    dict.set("ladder_lines", &lines);
                }
                dict.to_variant()
            })
            .collect()
    }

    /// `(x, y, z, angle)` per living soldier of `side`.
    #[func]
    fn get_soldier_transforms(&self, side: GString) -> PackedFloat32Array {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return PackedFloat32Array::new();
        };
        sim.soldier_transforms(side, None)
            .iter()
            .flat_map(|t| t.map(|v| v as f32))
            .collect()
    }

    /// `MultiMesh.buffer` (12 floats per instance, rotation about Y) of the
    /// living soldiers of `side` whose regiment renders as `render`
    /// (`infantry`, `archer`, `cavalry`, `siege`). Uncached; the renderer uses
    /// [`Self::get_soldier_buffers`].
    #[func]
    fn get_soldier_buffer(&self, side: GString, render: GString) -> PackedFloat32Array {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return PackedFloat32Array::new();
        };
        let render = render.to_string();
        let mut buffer: Vec<f32> = Vec::new();
        for unit in sim
            .units()
            .iter()
            .filter(|u| u.side == side && render_key(u) == render)
        {
            // SG1: climbers drawn on their ladders / the tower bridge.
            for pose in sim.soldier_poses(unit, self.figure_scale) {
                push_pose(&mut buffer, pose);
            }
        }
        PackedFloat32Array::from(buffer.as_slice())
    }
}
