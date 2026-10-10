//! Loyalty rules of generals and governors (`data/rules/loyalty.json`,
//! `loyalty_rules.schema.json`, WH charsb, ADR 0284). Read by
//! `sim-campaign::loyalty`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoyaltyRules {
    /// Loyalty a general or governor drifts towards, before modifiers.
    pub base_target: i32,
    /// Points moved towards the target each season.
    pub drift: i32,
    /// Target points per point of the character's `loyalty` effect (traits, skills).
    pub effect_weight: i32,
    /// Target bonus of a member of an active chivalric order.
    pub order_bonus: i32,
    /// Governed province unrest (weighted average) from which the governor resents it.
    pub unrest_threshold: i32,
    pub unrest_penalty: i32,
    /// Target malus while the faction's treasury is negative.
    pub debt_penalty: i32,
    /// Target malus after a lost battle, for `defeat_window_turns` seasons.
    pub defeat_penalty: i32,
    pub defeat_window_turns: u32,
    /// A warning reaches the journal when the loyalty falls under this.
    pub warn_below: u8,
    /// Under this, a general or governor may defect each season.
    pub defect_below: u8,
    pub defect_permille: u32,
    /// Loyalty set after a defection that found no enemy to join, or after
    /// the governor's dismissal.
    pub recover_to: u8,
    /// Unrest added to every class of the province whose governor defects.
    pub defect_unrest: u8,
    /// Above this loyalty, the units the general leads gain `high_morale`.
    pub high_above: u8,
    pub high_morale: f64,
    /// Loyalty a character starts with (new characters and the 1337 setup).
    #[serde(default)]
    pub start_base: u8,
    /// Added at setup for a member of the ruler's house.
    #[serde(default)]
    pub start_kin_bonus: u8,
    /// Deterministic spread (± points) around the starting loyalty.
    #[serde(default)]
    pub start_jitter: u8,
    /// Trait id of the ambitious: lower starting loyalty, jealous of rivals.
    #[serde(default)]
    pub ambition_trait: String,
    #[serde(default)]
    pub start_ambition_malus: u8,
    /// Seasonal loss of a captive whose lord could pay his ransom and does not.
    #[serde(default)]
    pub ransom_refused_loss: u8,
    /// Loss of a general or governor passed over when a title is granted to a peer.
    #[serde(default)]
    pub rival_loss: u8,
    /// Prestige distance under which the grantee counts as the rival's peer.
    #[serde(default)]
    pub rival_prestige_gap: i32,
    /// Extra loss for an ambitious character.
    #[serde(default)]
    pub rival_ambition_loss: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

crate::bundled_rules!(LoyaltyRules, "rules/loyalty.json", default);
