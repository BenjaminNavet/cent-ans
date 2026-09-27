//! `GameDataStore.get_rule_constants` (lot SV4): the rule values the
//! interface quotes in its texts, by name, so that no GDScript copies a
//! number of the core or of `data/rules`.

use godot::prelude::*;

use crate::GameDataStore;

#[godot_api(secondary)]
impl GameDataStore {
    /// `{name: value}` of every rule value the interface may quote
    /// (`sim_campaign::rule_constants`, plus battle thresholds). Values are
    /// floats; percentages are in percent. Empty before `load`.
    #[func]
    fn get_rule_constants(&self) -> VarDictionary {
        let Some(data) = &self.data else {
            return VarDictionary::new();
        };
        let mut dict = VarDictionary::new();
        for (name, value) in sim_campaign::rule_constants::rule_constants(data) {
            dict.set(name, value);
        }
        dict.set("exhausted_fatigue", sim_battle::sim::EXHAUSTED_FATIGUE);
        dict.set(
            "breach_open_threshold",
            f64::from(sim_battle::siege::BREACH_ONE_GAP),
        );
        // CB-M2: path preview cadence (`data/rules/battle_hover.json`).
        let preview = &sim_battle::HoverRules::bundled().preview;
        dict.set("hover_preview_recompute_m", preview.recompute_distance_m);
        dict.set("hover_preview_max_per_s", preview.max_recomputes_per_s);
        dict.set(
            "hover_preview_max_paths",
            f64::from(preview.max_individual_paths),
        );
        // CB-M3: queued orders per regiment (`data/rules/battle_queue.json`).
        dict.set(
            "battle_queue_max",
            f64::from(sim_battle::QueueRules::bundled().max_queued_orders),
        );
        // CB2: unit modes (`data/rules/unit_modes.json`), quoted by the
        // tooltips of the mode buttons and the help.
        let modes = sim_battle::UnitModeRules::bundled();
        dict.set("unit_modes_skirmish_trigger_m", modes.skirmish.trigger_m);
        dict.set("unit_modes_skirmish_retreat_m", modes.skirmish.retreat_m);
        dict.set(
            "unit_modes_breach_damage_percent",
            (modes.breach.wall_damage("default") - 1.0) * 100.0,
        );
        dict.set(
            "unit_modes_breach_mangonel_percent",
            (modes.breach.wall_damage("unit_mangonel") - 1.0) * 100.0,
        );
        dict.set(
            "unit_modes_breach_reload_percent",
            (modes.breach.reload_multiplier - 1.0) * 100.0,
        );
        dict.set("unit_modes_wavering_morale", modes.status.wavering_morale);
        dict.set(
            "unit_modes_under_fire_seconds",
            modes.status.under_fire_seconds,
        );
        dict
    }
}
