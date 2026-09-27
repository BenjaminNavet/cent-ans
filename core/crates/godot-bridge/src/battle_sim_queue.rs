//! CB-M3: queued orders on the GDExtension side (spec
//! `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`,
//! « Ordres en file »). The `queue` entry of `get_units`, and the path
//! previews that start from the last point of a regiment's queue. Read-only,
//! like the CB-M2 previews.

use godot::prelude::*;
use sim_battle::{QueuedOrder, Unit};

use crate::battle_sim::BattleSim;
use crate::battle_sim_preview::reason_fr;

/// `get_units()[i].queue`: the regiment's orders waiting behind the current
/// one, in order, each `{x, z}` (where it goes: an attack's target where it
/// stands now) plus `facing` (move with an arrival facing) or `target` (an
/// attack).
pub(crate) fn queue_array(sim: &sim_battle::BattleSim, unit: &Unit) -> VarArray {
    unit.order_queue
        .iter()
        .map(|order| {
            let dict = match order {
                QueuedOrder::Move { x, z, facing, .. } => {
                    let mut dict = vdict! { "x" => *x, "z" => *z };
                    if let Some(f) = facing {
                        dict.set("facing", *f);
                    }
                    dict
                }
                QueuedOrder::Attack { target, .. } => {
                    let (x, z) = sim.unit(*target).map_or((unit.x, unit.z), |t| (t.x, t.z));
                    vdict! { "x" => x, "z" => z, "target" => i64::from(*target) }
                }
            };
            dict.to_variant()
        })
        .collect()
}

#[godot_api(secondary)]
impl BattleSim {
    /// CB-M3: as `preview_path`, but the way starts at (`from_x`, `from_z`)
    /// (the last point of the regiment's queue): that point, then each
    /// waypoint, the destination last. Empty when there is no way.
    #[func]
    fn preview_path_from(
        &self,
        unit_id: i64,
        from_x: f64,
        from_z: f64,
        x: f64,
        z: f64,
    ) -> PackedVector3Array {
        let Some(sim) = &self.sim else {
            return PackedVector3Array::new();
        };
        let Ok(id) = u32::try_from(unit_id) else {
            return PackedVector3Array::new();
        };
        match sim.preview_path_from(id, Some((from_x, from_z)), x, z) {
            Ok(path) => self.path_points((from_x, from_z), &path),
            Err(_) => PackedVector3Array::new(),
        }
    }

    /// CB-M3: as `preview_paths`, for a queued `move` (Shift held): each way
    /// starts where the regiment's queue leaves it, the group spread around
    /// those points as the queued order does.
    #[func]
    fn preview_paths_queued(
        &self,
        unit_ids: PackedInt32Array,
        x: f64,
        z: f64,
        facing: f64,
    ) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        let ids = Self::ids_of(&unit_ids);
        let facing = facing.is_finite().then_some(facing);
        sim.preview_group_queued(&ids, x, z, facing)
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
}
