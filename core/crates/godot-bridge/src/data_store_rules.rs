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
        // CB5: the alert column's on-screen duration and merge window, so
        // `battle_alerts_column.gd` quotes `data/rules/battle_alerts.json`
        // instead of copying its numbers.
        let alerts = sim_battle::alerts::AlertRules::bundled();
        dict.set("cb5_alert_duration_s", alerts.duration_s);
        dict.set("cb5_alert_merge_window_s", alerts.merge_window_s);
        dict.set("cb5_alert_merge_radius_m", alerts.merge_radius_m);
        dict.set("cb5_alert_max_shown", f64::from(alerts.max_shown));
        dict
    }
}
