//! `BattleSim`: scalar state, outcome and the event streams.

use godot::prelude::*;

use crate::battle_sim::{parse_side, BattleSim};
use crate::convert::{json_to_variant, to_dict};

#[godot_api(secondary)]
impl BattleSim {
    /// The setup the battle was built from (names, factions, general).
    #[func]
    fn get_setup(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .map(|s| to_dict(s.setup()))
            .unwrap_or_default()
    }

    /// Head count still able to fight on `side`.
    #[func]
    fn get_strength(&self, side: GString) -> i64 {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => i64::from(sim.strength(side)),
            _ => 0,
        }
    }

    /// Simulated seconds since the start.
    #[func]
    fn get_elapsed(&self) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.elapsed())
    }

    #[func]
    fn get_ticks(&self) -> i64 {
        self.sim.as_ref().map_or(0, |s| s.ticks() as i64)
    }

    #[func]
    fn is_finished(&self) -> bool {
        self.sim.as_ref().is_some_and(|s| s.is_finished())
    }

    /// Result in the format accepted by `CampaignSim.resolve_battle` (empty
    /// while the battle goes on).
    #[func]
    fn get_outcome(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .and_then(|s| s.outcome())
            .map(|o| to_dict(&o))
            .unwrap_or_default()
    }

    /// SG1: siege assault events for the 3D view, added since the last call:
    /// `[{time, kind, ...}]` with `kind` among `engine_shot {unit, piece, x, z,
    /// height (0-1), breached}`, `ram_strike {unit, piece, breached}`,
    /// `tower_volley {tower, target}`, `ladders_raised {unit, piece}`,
    /// `tower_docked {unit, piece}`, `tower_undocked {unit, piece}`,
    /// `on_wall {unit, piece}`, `boiling_oil {piece, x, z, targets: [ids]}`,
    /// `gate_broken {piece}`, `wall_breached {piece}`, `defenders_fall_back`.
    /// Rendering only: the rules are already applied.
    #[func]
    fn get_siege_events(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_new_siege_fx()
            .iter()
            .map(|fx| {
                serde_json::to_value(fx)
                    .map(|json| json_to_variant(&json))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Journal entries `[{time, text_fr, side}]` added since the last call.
    #[func]
    fn get_events(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_new_events()
            .iter()
            .map(|event| {
                vdict! {
                    "time" => event.time,
                    "text_fr" => event.text_fr.as_str(),
                    "side" => event.side.map_or("", |s| s.key()),
                }
                .to_variant()
            })
            .collect()
    }

    /// CB5: typed alerts `[{kind, time, x, z, side, unit}]` added since the
    /// last call. `kind` is one of `rout`, `general_down`, `flanked`,
    /// `reinforcements`, `ammo_out`, `wall_breached`, `gate_destroyed`, `square_threatened` (T4).
    /// `side` is `""` and `unit` is `-1` for a wall/gate piece. Output only:
    /// reading it never changes the simulation.
    #[func]
    fn get_alerts(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_new_alerts()
            .iter()
            .map(|alert| {
                vdict! {
                    "kind" => alert.kind.key(),
                    "time" => alert.time,
                    "x" => alert.x,
                    "z" => alert.z,
                    "side" => alert.side.map_or("", |s| s.key()),
                    "unit" => alert.unit.map_or(-1, |u| u as i64),
                }
                .to_variant()
            })
            .collect()
    }
}
