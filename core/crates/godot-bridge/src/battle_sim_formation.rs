//! CB6: group formations on the GDExtension side (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`, « CB6 »).
//!
//! Two read-only queries: the presets of `data/rules/group_formations.json`
//! and the places one of them proposes. Nothing moves and nothing is
//! recorded: the interface turns the places into ordinary `move` orders
//! (CB1 width, `match_speed`, `group_tag`) or `deploy_unit` calls.

use godot::prelude::*;

use crate::battle_sim::BattleSim;

#[godot_api(secondary)]
impl BattleSim {
    /// The presets, in the data order: `{id, name_fr, description_fr,
    /// stance}` (`stance`: `attack`, `defense` or `march`).
    #[func]
    fn formation_presets(&self) -> VarArray {
        let mut out = VarArray::new();
        for preset in sim_battle::group_formation::presets() {
            let entry = vdict! {
                "id" => preset.id.as_str(),
                "name_fr" => preset.name_fr.as_str(),
                "description_fr" => preset.description_fr.as_str(),
                "stance" => preset.stance.key(),
            };
            out.push(&entry.to_variant());
        }
        out
    }

    /// The places preset `preset_id` proposes for regiments `unit_ids`, the
    /// centre of their front line at (x, z), facing `facing` (radians):
    /// `[{id, x, z, facing, width}]`, `width` 0 when the regiment keeps its
    /// formation. Inside the deployment zone during the deployment, never
    /// in deep water. Empty for an unknown preset or without a battle.
    #[func]
    fn formation_slots(
        &self,
        preset_id: GString,
        unit_ids: PackedInt32Array,
        x: f64,
        z: f64,
        facing: f64,
    ) -> VarArray {
        let mut out = VarArray::new();
        let Some(sim) = &self.sim else {
            return out;
        };
        let ids = Self::ids_of(&unit_ids);
        for slot in sim.formation_slots(&preset_id.to_string(), &ids, x, z, facing) {
            let entry = vdict! {
                "id" => i64::from(slot.id),
                "x" => slot.x,
                "z" => slot.z,
                "facing" => slot.facing,
                "width" => slot.width.unwrap_or(0.0),
            };
            out.push(&entry.to_variant());
        }
        out
    }
}
