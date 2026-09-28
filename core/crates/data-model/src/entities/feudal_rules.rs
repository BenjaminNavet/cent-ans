//! Feudal tuning (lot FE, ADR 0098), mirroring
//! `data/schemas/feudal_rules.schema.json` (`data/rules/feudal.json`).
//!
//! The defaults are the values of the former `sim_campaign::diplomacy`
//! constants, so a data tree without the file behaves as before.

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/feudal.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FeudalRules {
    /// Share of a vassal's income paid to its direct suzerain each turn.
    pub vassal_tribute_percent: i64,
    /// Vassals below this loyalty may rebel.
    pub rebellion_loyalty: u8,
    /// Vassals below this loyalty ignore their suzerain's calls to arms.
    pub call_to_arms_loyalty: u8,
    /// Chance per turn (‰) that a disloyal vassal rebels.
    pub rebellion_permille: u32,
    /// Minimum power ratio to demand homage.
    pub vassalage_power_ratio: f64,
    /// Below this loyalty a vassal may refuse the host, revolt or pay
    /// homage to another liege (spec § 4.2).
    pub disloyal_threshold: u8,
    /// Turns a felony case stays open to a declaration of forfeiture (§ 4.4).
    pub felony_window_turns: u32,
    /// Turns of independence held for the generic victory (§ 4.8).
    pub independence_turns: u32,
    /// Turns as the realm's first vassal for the generic victory (§ 4.8).
    pub ascension_turns: u32,
    pub loyalty: LoyaltyWeights,
}

impl Default for FeudalRules {
    fn default() -> Self {
        FeudalRules {
            vassal_tribute_percent: 10,
            rebellion_loyalty: 20,
            call_to_arms_loyalty: 30,
            rebellion_permille: 250,
            vassalage_power_ratio: 3.0,
            disloyal_threshold: 30,
            felony_window_turns: 8,
            independence_turns: 20,
            ascension_turns: 20,
            loyalty: LoyaltyWeights::default(),
        }
    }
}

/// Terms of a vassal's target loyalty towards its direct suzerain (§ 4.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LoyaltyWeights {
    /// Starting point before any term.
    pub base: i32,
    /// Suzerain more than twice as powerful as the vassal...
    pub power_favourable: i32,
    /// ... or not.
    pub power_unfavourable: i32,
    /// Excommunicated suzerain.
    pub excommunicated_liege: i32,
    /// Vassal embargoed by an enemy of its suzerain.
    pub embargo_squeeze: i32,
    /// Protection granted when the vassal was attacked (decays).
    pub protection_granted: i32,
    /// Protection refused (decays).
    pub protection_refused: i32,
    /// Rulers of the two factions are kin or married into each other.
    pub family_tie: i32,
    /// Vassal and suzerain share a culture.
    pub shared_culture: i32,
    /// The suzerain granted the vassal a title.
    pub title_granted: i32,
    /// A peer of the vassal was struck by forfeiture.
    pub peer_forfeiture: i32,
    /// Each recent defeat of the suzerain.
    pub liege_defeat: i32,
    /// A rival claimant to the suzerain's primary title exists.
    pub rival_claimant: i32,
}

impl Default for LoyaltyWeights {
    fn default() -> Self {
        LoyaltyWeights {
            base: 55,
            power_favourable: 10,
            power_unfavourable: -10,
            excommunicated_liege: -20,
            embargo_squeeze: -25,
            protection_granted: 15,
            protection_refused: -20,
            family_tie: 10,
            shared_culture: 5,
            title_granted: 10,
            peer_forfeiture: -10,
            liege_defeat: -5,
            rival_claimant: -15,
        }
    }
}
