//! CB4: regiments' active abilities on the GDExtension side (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § CB4).
//! The `abilities` entry of `get_units` (state of each ability of the
//! regiment), the catalogue texts (`BattleSim.get_ability_catalog`), and the
//! rule values of the tooltips (`GameDataStore.get_rule_constants`). An
//! ability is used by `issue_command` (`{type: "use_ability", units: [ids],
//! ability: "ability_pavise"}`) or `BattleSim.use_ability(units, ability)`.

use godot::prelude::*;
use sim_battle::Unit;

/// Adds `abilities` to `dict` (one `get_units` entry): by rank, `[{id,
/// kind, available, reason, active, effective, setup_remaining, remaining,
/// duration, cooldown, cooldown_remaining, ended_reason}]` (`reason`: why it
/// cannot be used now, empty when available; `ended_reason`: why a
/// condition ended it last). Empty for a regiment without ability.
pub(crate) fn add_ability_fields(
    sim: &sim_battle::BattleSim,
    unit: &Unit,
    dict: &mut VarDictionary,
) {
    let list: VarArray = sim
        .ability_views(unit)
        .iter()
        .map(|v| {
            vdict! {
                "id" => v.id.as_str(),
                "kind" => kind_key(v.kind),
                "available" => v.available,
                "reason" => v.reason.as_str(),
                "active" => v.active,
                "effective" => v.effective,
                "setup_remaining" => v.setup_remaining,
                "remaining" => v.remaining,
                "duration" => v.duration,
                "cooldown" => v.cooldown,
                "cooldown_remaining" => v.cooldown_remaining,
                "ended_reason" => v.ended_reason.as_str(),
            }
            .to_variant()
        })
        .collect();
    dict.set("abilities", &list);
}

/// `{id: {id, kind, rank, name, description, icon}}` of the battle's
/// catalogue.
pub(crate) fn catalog_dict(sim: &sim_battle::BattleSim) -> VarDictionary {
    let mut out = VarDictionary::new();
    for a in sim.ability_catalog() {
        out.set(
            a.id.as_str(),
            &vdict! {
                "id" => a.id.as_str(),
                "kind" => kind_key(a.kind),
                "rank" => i64::from(a.rank),
                "name" => a.name.as_str(),
                "description" => a.description.as_str(),
                "icon" => a.icon.as_deref().unwrap_or(""),
            },
        );
    }
    out
}

/// JSON key of an ability kind (`aimed_shot`, `pavise`…).
pub(crate) fn kind_key(kind: data_model::AbilityKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Rule values of the abilities quoted by the tooltips: for each ability
/// `<id>_cooldown`, `<id>_duration`, `<id>_setup_time` (seconds), each
/// effect factor as a change in percent (`<id>_range_percent` = −45 for a
/// range factor of 0.55) and each bonus as is (`<id>_morale_bonus`).
pub(crate) fn rule_constants(data: &data_model::GameData, dict: &mut VarDictionary) {
    for a in data.battle_abilities.values() {
        dict.set(format!("{}_cooldown", a.id).as_str(), a.cooldown);
        dict.set(format!("{}_duration", a.id).as_str(), a.duration);
        dict.set(format!("{}_setup_time", a.id).as_str(), a.setup_time);
        let Ok(serde_json::Value::Object(effects)) = serde_json::to_value(&a.effects) else {
            continue;
        };
        for (key, value) in effects {
            let Some(number) = value.as_f64() else {
                continue;
            };
            match key.strip_suffix("_factor") {
                Some(stem) => dict.set(
                    format!("{}_{stem}_percent", a.id).as_str(),
                    ((number - 1.0) * 100.0).round(),
                ),
                None => dict.set(format!("{}_{key}", a.id).as_str(), number),
            }
        }
    }
}
