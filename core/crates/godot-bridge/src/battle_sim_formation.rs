//! CB6: group formations on the GDExtension side (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`, « CB6 »).
//!
//! Two read-only queries: the presets of `data/rules/group_formations.json`
//! and the places one of them proposes. Nothing moves and nothing is
//! recorded: the interface turns the places into ordinary `move` orders
//! (CB1 width, `match_speed`, `group_tag`) or `deploy_unit` calls.
//!
//! RJ-a (ADR 0174): the regiment formations of
//! `data/rules/unit_formations.json` (menu and tooltips) and their fields in
//! `get_units`.

use godot::prelude::*;
use sim_battle::{FormationRules, Unit};

use crate::battle_sim::BattleSim;

/// RJ-a: adds to `dict` (one `get_units` entry): `formation_name` and
/// `formation_short` (labels of the current formation), `formations` (keys
/// the regiment may take, data order), `reforming`, `reform_progress` (0-1)
/// and `reform_from` (key being left, "" when formed).
pub(crate) fn add_formation_fields(unit: &Unit, dict: &mut VarDictionary) {
    let def = unit.formation.def();
    dict.set("formation_name", def.name.as_str());
    dict.set("formation_short", def.short.as_str());
    let allowed: PackedStringArray = sim_battle::Formation::all()
        .filter(|f| f.def().allows(unit))
        .map(|f| GString::from(f.key()))
        .collect();
    dict.set("formations", &allowed);
    dict.set("reforming", unit.reform.is_some());
    dict.set("reform_progress", unit.reform.map_or(1.0, |r| r.progress()));
    dict.set("reform_from", unit.reform.map_or("", |r| r.from.key()));
}

fn category_key(category: data_model::UnitCategory) -> &'static str {
    match category {
        data_model::UnitCategory::Infantry => "infantry",
        data_model::UnitCategory::Ranged => "ranged",
        data_model::UnitCategory::Cavalry => "cavalry",
        data_model::UnitCategory::Siege => "siege",
    }
}

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

    /// RJ-a: the regiment formations, in the data order: `{key, name, short,
    /// description, categories, foot_only, mounted_only, shape, all_round,
    /// braced, reform_s, modifiers: {speed, charge, melee_taken,
    /// missile_taken, shooting, morale_loss, push_drive, push_resistance,
    /// horse_blows, fighting_ranks}}`.
    #[func]
    fn unit_formations(&self) -> VarArray {
        let mut out = VarArray::new();
        for f in &FormationRules::bundled().formations {
            let m = &f.modifiers;
            let categories: PackedStringArray = f
                .categories
                .iter()
                .map(|&c| GString::from(category_key(c)))
                .collect();
            let modifiers = vdict! {
                "speed" => m.speed,
                "charge" => m.charge,
                "melee_taken" => m.melee_taken,
                "missile_taken" => m.missile_taken,
                "shooting" => m.shooting,
                "morale_loss" => m.morale_loss,
                "push_drive" => m.push_drive,
                "push_resistance" => m.push_resistance,
                "horse_blows" => m.horse_blows,
                "fighting_ranks" => m.fighting_ranks,
            };
            let mut entry = vdict! {
                "key" => f.key.as_str(),
                "name" => f.name.as_str(),
                "short" => f.short.as_str(),
                "description" => f.description.as_str(),
                "foot_only" => f.foot_only,
                "mounted_only" => f.mounted_only,
                "shape" => format!("{:?}", f.shape).to_lowercase(),
                "all_round" => f.all_round,
                "braced" => f.braced,
                "reform_s" => f.reform_s,
            };
            entry.set("categories", &categories);
            entry.set("modifiers", &modifiers);
            out.push(&entry.to_variant());
        }
        out
    }

    /// RJ-a: multipliers while a regiment changes formation: `{speed,
    /// melee_taken, missile_taken, morale_loss}`.
    #[func]
    fn formation_reform_rules(&self) -> VarDictionary {
        let r = &FormationRules::bundled().reform;
        vdict! {
            "speed" => r.speed,
            "melee_taken" => r.melee_taken,
            "missile_taken" => r.missile_taken,
            "morale_loss" => r.morale_loss,
        }
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
