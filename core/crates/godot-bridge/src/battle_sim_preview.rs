//! CB-M2: path preview and contextual cursor on the GDExtension side (spec
//! `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`,
//! « Aperçu du trajet » and « Curseur contextuel »).
//!
//! Two read-only queries of the battle: they change nothing (no touch of
//! the poses, no replay note), so the battle and its replay stay the same
//! however often the interface asks.

use godot::prelude::*;
use sim_battle::{Advantage, Compare, CompareSide, PreviewError, SideId};

use crate::battle_sim::BattleSim;

/// French reason of a refused preview (tooltip of the `forbidden` cursor).
fn reason_fr(error: PreviewError) -> &'static str {
    match error {
        PreviewError::UnknownUnit | PreviewError::Unavailable => "Ce régiment ne répond plus",
        PreviewError::OutsideField => "Hors du champ de bataille",
        PreviewError::OutsideZone => "Hors de la zone de déploiement",
        PreviewError::Unreachable => "Aucun chemin jusque-là",
    }
}

fn advantage_key(a: Advantage) -> &'static str {
    match a {
        Advantage::Even => "even",
        Advantage::Ours => "ours",
        Advantage::Theirs => "theirs",
    }
}

fn side_dict(side: &CompareSide) -> VarDictionary {
    vdict! {
        "unit" => i64::from(side.unit),
        "soldiers" => i64::from(side.soldiers),
        "melee" => side.melee,
        "defense" => side.defense,
        "charge" => side.charge,
        "ranged" => side.ranged,
        "range" => side.range,
        "morale" => side.morale,
        "fatigue" => side.fatigue,
        "bonus_vs" => side.bonus_vs,
    }
}

fn compare_dict(compare: &Compare) -> VarDictionary {
    let mut advantages = VarDictionary::new();
    let mut lines = VarArray::new();
    for (line, advantage) in &compare.advantages {
        advantages.set(line.as_str(), advantage_key(*advantage));
        lines.push(&line.to_variant());
    }
    let mut dict = VarDictionary::new();
    dict.set("ours", &side_dict(&compare.ours));
    dict.set("theirs", &side_dict(&compare.theirs));
    dict.set("advantages", &advantages);
    dict.set("lines", &lines);
    dict
}

impl BattleSim {
    /// `from` then every waypoint, at the height one walks at.
    fn path_points(&self, from: (f64, f64), path: &[(f64, f64)]) -> PackedVector3Array {
        let Some(sim) = &self.sim else {
            return PackedVector3Array::new();
        };
        let field = sim.field();
        std::iter::once(from)
            .chain(path.iter().copied())
            .map(|(x, z)| Vector3::new(x as f32, field.walk_height(x, z) as f32, z as f32))
            .collect()
    }

    /// The ids of a GDScript array of ints (others skipped).
    fn ids_of(array: &VarArray) -> Vec<u32> {
        array
            .iter_shared()
            .filter_map(|v| v.try_to::<i64>().ok())
            .filter_map(|id| u32::try_from(id).ok())
            .collect()
    }
}

#[godot_api(secondary)]
impl BattleSim {
    /// CB-M2: the way regiment `unit_id` would walk if ordered to (x, z):
    /// its position, then each waypoint, the destination last (`y` = height
    /// one walks at). Empty when there is no way (the cursor is then
    /// `forbidden` and no order is sent). The same computation as the real
    /// order (`plan_route`), without changing the battle.
    #[func]
    fn preview_path(&self, unit_id: i64, x: f64, z: f64) -> PackedVector3Array {
        let Some(sim) = &self.sim else {
            return PackedVector3Array::new();
        };
        let Ok(id) = u32::try_from(unit_id) else {
            return PackedVector3Array::new();
        };
        match (sim.preview_path(id, x, z), sim.unit(id)) {
            (Ok(path), Some(unit)) => self.path_points((unit.x, unit.z), &path),
            _ => PackedVector3Array::new(),
        }
    }

    /// CB-M2: the ways of a group `move` of `unit_ids` to (x, z) (`facing`
    /// in radians, NaN for none: as the order spreads them), each
    /// `{unit, path, ok, reason}`: `path` as in [`Self::preview_path`];
    /// beyond `max_individual_paths` regiments (`data/rules/battle_hover.json`),
    /// a single entry from the group's centre with `unit` = -1. `reason` is
    /// the French cause of an empty path.
    #[func]
    fn preview_paths(&self, unit_ids: VarArray, x: f64, z: f64, facing: f64) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        let ids = Self::ids_of(&unit_ids);
        let facing = facing.is_finite().then_some(facing);
        sim.preview_group(&ids, x, z, facing)
            .iter()
            .map(|leg| {
                let (path, reason) = match &leg.path {
                    Ok(path) => (self.path_points(leg.from, path), ""),
                    Err(e) => (PackedVector3Array::new(), reason_fr(*e)),
                };
                let mut dict = vdict! {
                    "unit" => leg.unit.map_or(-1, i64::from),
                    "ok" => leg.path.is_ok(),
                    "reason" => reason,
                };
                dict.set("path", &path);
                dict.to_variant()
            })
            .collect()
    }

    /// CB-M2: what a right click at (x, z) would do with the player's
    /// regiments `selected_ids`: `{context, target, piece, compare}` —
    /// `context` one of `move`, `melee`, `ranged`, `ranged_blocked`,
    /// `siege`, `forbidden`, `none`; `target` the regiment under the cursor
    /// (-1: none), `piece` the wall piece (-1: none); `compare` (only with a
    /// single regiment selected and an enemy under the cursor) `{ours,
    /// theirs, advantages: {line: "ours"|"theirs"|"even"}, lines}`, each
    /// side `{unit, soldiers, melee, defense, charge, ranged, range, morale,
    /// fatigue, bonus_vs}`. Cheap (no path search): the interface calls it
    /// as the mouse moves.
    #[func]
    fn hover_context(&self, x: f64, z: f64, selected_ids: VarArray) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return vdict! { "context" => "none", "target" => -1, "piece" => -1 };
        };
        let side = sim.setup().player_side.unwrap_or(SideId::Attacker);
        let hover = sim.hover_context(x, z, &Self::ids_of(&selected_ids), side);
        let mut dict = vdict! {
            "context" => hover.context.key(),
            "target" => hover.target.map_or(-1, i64::from),
            "piece" => hover.piece.map_or(-1, |p| p as i64),
        };
        if let Some(compare) = &hover.compare {
            dict.set("compare", &compare_dict(compare));
        }
        dict
    }
}
