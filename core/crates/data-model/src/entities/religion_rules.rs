//! Faith rules (ADR 0326, 0327), mirroring
//! `data/schemas/religion_rules.schema.json` (`data/rules/religion.json`).
//!
//! The file is optional: without it the new mechanics (wider excommunication,
//! interdict, conversion of provinces, papal crusade, dynastic claims by
//! marriage) are inert ([`crate::GameData::religion_rules`] is `None`).

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/religion.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReligionRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub excommunication: ExcommunicationRules,
    pub interdict: InterdictRules,
    pub conversion: ConversionRules,
    pub papal_crusade: PapalCrusadeRules,
    pub dynastic_claim: DynasticClaimRules,
}

/// Extra triggers of the excommunication (the war on a Catholic with a low
/// favour stays in the diplomacy code).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcommunicationRules {
    /// Breaking a truce or a non-aggression pact against a Catholic
    /// excommunicates a Catholic whose favour (after the perjury penalty) is
    /// below this value.
    pub perjury_favor_below: u8,
    /// Executing a Catholic captive: favour lost…
    pub captive_favor_loss: u8,
    /// …and excommunication when the favour then falls below this value.
    pub captive_favor_below: u8,
}

/// The interdict: a kingdom under excommunication whose favour is very low.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterdictRules {
    /// An excommunication pronounced with a favour below this value (or
    /// repeated while it lasts) comes with the interdict.
    pub favor_below: u8,
    /// Length in seasons.
    pub turns: u32,
    /// Flat unrest added to every province of the realm.
    pub unrest: f64,
    /// A donation of at least this many livres lifts the interdict.
    pub lift_donation: i64,
}

/// Progressive conversion of a province to the faith of its lord.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversionRules {
    /// Progress points per season.
    pub base_per_season: u8,
    /// One more point per this many points of the governor's (or ruler's) piety.
    pub piety_divisor: u8,
    /// Points per religious building of the province (weighted by tier).
    pub per_religious_building: f64,
    /// Speed (percent) when the lord only occupies the province.
    pub occupier_percent: u32,
    /// Speed (percent) when the two churches are kindred (schismatics).
    pub kindred_percent: u32,
    /// No progress while the unrest of the province is at least this.
    pub stall_unrest: u8,
    /// Points lost per season when the lord's faith is the province's own.
    pub decay_per_season: u8,
    /// Points added by a preacher's sermon (before his level bonus).
    pub preacher_gain: u8,
    /// Extra points per preacher level above the first.
    pub preacher_per_level: u8,
    /// Unrest (all classes) added when the faith changes.
    pub unrest_on_conversion: u8,
}

/// The pope's call to crusade against a target province.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PapalCrusadeRules {
    /// First turn a call may be made.
    pub first_call_turn: u32,
    /// Minimum seasons between the end of a call and the next.
    pub interval_turns: u32,
    /// Length of the call in seasons.
    pub window_turns: u32,
    /// Favour gained by a Catholic that goes to war with the holder of the target.
    pub join_favor: u8,
    pub join_prestige: i32,
    /// Reward of the first Catholic to hold the target.
    pub reward_favor: u8,
    pub reward_prestige: i32,
    /// Livres paid by the papal treasury to the victor (capped by the treasury).
    pub reward_gold: i64,
    /// AI: the lowest power ratio (own / target coalition) to answer the call.
    pub ai_min_ratio: f64,
    /// AI: the lowest papal favour of a Catholic that answers the call.
    pub ai_min_favor: u8,
    /// AI: no further Catholic AI joins once this many have.
    pub ai_max_participants: u32,
}

/// Throne claim born of a marriage between two factions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynasticClaimRules {
    /// The foreign parent must belong to the ruling house of its faction.
    pub require_ruling_house: bool,
}
