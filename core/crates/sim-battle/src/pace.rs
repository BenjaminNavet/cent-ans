//! A6-L13 (ADR 0184): pace of the fighting. Rules in
//! `data/rules/battle_pace.json` (schema
//! `data/schemas/battle_pace_rules.schema.json`): lethality of the melee and
//! of the missiles, morale lost per man killed. Field battles and sieges have
//! their own values (the sieges keep their original pace).

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/battle_pace.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaceRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub field: Pace,
    /// Historical maps (EP7), tuned with the original pace.
    pub historical: Pace,
    pub siege: Pace,
}

/// One set of fighting rates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pace {
    /// Deaths per second per melee fighter, before armour, attack value and cover.
    pub melee_rate: f64,
    /// Lethality factor of a volley (and of tower shots).
    pub ranged_rate: f64,
    /// Morale lost when a regiment loses all its men at once.
    pub loss_morale_factor: f64,
    /// Morale lost per second by a regiment struck on the flank.
    pub flank_morale_per_s: f64,
    /// Morale lost per second by a regiment struck in the rear.
    pub rear_morale_per_s: f64,
    /// A regiment whose morale falls under this routs.
    pub rout_morale: f64,
    /// A routing regiment out of danger whose morale rises above this rallies.
    pub rally_morale: f64,
    /// A6-L13b: factor on the walking speed of a regiment that is not running
    /// or routing and while the two armies are still farther apart than
    /// `approach_range_m` (the deliberate march of the approach; 1 = the stats
    /// of the unit types).
    pub move_speed_factor: f64,
    /// A6-L13b: gap between the two armies (nearest regiments) under which
    /// everyone moves at full speed, so the volleys of the approach keep their
    /// original exposure and the army arrives together.
    pub approach_range_m: f64,
    /// A6-L13b: factor on the speed of a run or a charge (not of a rout).
    pub run_speed_factor: f64,
    /// A6-L13b: fatigue gained per second by a regiment in melee.
    pub melee_fatigue_per_s: f64,
    /// A6-L13b: factor on the morale a routing neighbour costs (rout cascade).
    pub contagion_factor: f64,
    /// A6-L13b: morale regained per second by a regiment holding its ground in
    /// melee (not flanked, over half its men), up to its morale cap.
    pub melee_resolve_per_s: f64,
    /// A6-L13b: the patience clocks of the battle AI (wait, duel, assault and
    /// hold times of `ai.rs`) run this many times slower, so a longer approach
    /// does not make the AI leave its ground before the armies meet.
    pub ai_patience_factor: f64,
}

data_model::bundled_rules!(PaceRules, "rules/battle_pace.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load_and_sieges_keep_their_original_pace() {
        let rules = PaceRules::bundled();
        assert_eq!(rules.siege.melee_rate, 0.035);
        assert_eq!(rules.siege.ranged_rate, 0.3);
        assert_eq!(rules.siege.loss_morale_factor, 60.0);
        assert_eq!(rules.siege.rout_morale, 20.0);
        assert!(rules.field.rout_morale < rules.field.rally_morale);
    }
}
