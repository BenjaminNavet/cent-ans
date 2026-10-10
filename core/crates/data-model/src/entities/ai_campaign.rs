//! Numeric constants of the campaign AI, mirroring
//! `data/schemas/ai_campaign.schema.json` (`data/ai/campaign.json`).
//!
//! Read through [`AiCampaign::bundled`] (embedded in the crate): the planner
//! of the `ai` crate holds a reference for the whole plan.

use serde::{Deserialize, Serialize};

/// Contents of `data/ai/campaign.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiCampaign {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub military: MilitaryRules,
    pub economy: EconomyRules,
    /// WR ai-mil (ADR 0301): thresholds of the sortie, the surrender demand
    /// and the recruitment into armies.
    #[serde(default)]
    pub military_orders: MilitaryOrdersRules,
}

/// Army objectives: targets, defence, sieges, retreats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MilitaryRules {
    /// Maximum path cost of an objective, in province steps.
    pub planning_range: u32,
    pub city_target_bonus: f64,
    pub fortification_target_penalty: f64,
    pub siege_superiority: f64,
    pub sea_invasion_factor: f64,
    pub sea_threat_factor: f64,
    pub defence_ratio: f64,
    pub defence_local_factor: f64,
    pub assault_odds: u32,
    pub retreat_strength: f64,
    pub starving_supply: u8,
    pub homeward_range_factor: u32,
    pub homeward_candidates: usize,
    pub reclaim_target_bonus: f64,
    pub enemy_capital_target_bonus: f64,
    pub throne_claim_target_bonus: f64,
    pub siege_patience_turns: u32,
    pub fortress_level: u32,
    pub siege_hold_threat_factor: f64,
    pub garrison_min_army_units: usize,
    pub raid_min_aggression: i32,
    pub raid_max_devastation: u8,
    pub raid_defence_factor: f64,
    pub regroup_power_share: f64,
    pub defend_weight_capital: f64,
    pub defend_weight_city: f64,
    pub defend_weight_other: f64,
    /// RX iaplay: factor on the value of the settlement an army is already
    /// marching to besiege (its `destination`): it changes target only for
    /// one worth this many times more. 1.0 = no memory.
    #[serde(default = "one")]
    pub siege_target_persistence: f64,
    /// RX iaplay: same for the settlement an army marches to defend.
    #[serde(default = "one")]
    pub defence_target_persistence: f64,
    /// RX iaplay: turns a noisy (low decision quality) preference of the AI
    /// for one siege target lasts before it is redrawn.
    #[serde(default = "default_noise_epoch")]
    pub decision_noise_epoch_turns: u32,
}

/// WR ai-mil: when the AI recruits into a field army, sallies out of a
/// besieged place or calls a besieged garrison to surrender.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MilitaryOrdersRules {
    /// An army standing in the settlement below this many units receives
    /// the recruits (capped by the army size limit).
    pub recruit_into_target_units: usize,
    /// The garrison must keep this many units before recruits go to an army.
    pub recruit_into_min_garrison: usize,
    /// Sortie when the garrison holds at least this share (%) of the
    /// combined power of garrison and besiegers.
    pub sortie_odds: u32,
    /// Supplies (0-100) at or below which the garrison is about to starve.
    pub sortie_starving_supply: u8,
    /// A starving garrison sallies from this share (%) of the combined power.
    pub sortie_desperate_odds: u32,
    /// The besieger demands surrender from this `surrender_chance` (%).
    pub surrender_min_chance: u32,
    /// Turns between two demands on the same siege (0: every turn).
    pub surrender_retry_turns: u32,
}

impl Default for MilitaryOrdersRules {
    fn default() -> Self {
        Self {
            recruit_into_target_units: 10,
            recruit_into_min_garrison: 3,
            sortie_odds: 55,
            sortie_starving_supply: 15,
            sortie_desperate_odds: 25,
            surrender_min_chance: 35,
            surrender_retry_turns: 3,
        }
    }
}

fn one() -> f64 {
    1.0
}

fn default_noise_epoch() -> u32 {
    16
}

fn never_unrest() -> f64 {
    101.0
}

/// Taxes, recruitment, treasury and construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EconomyRules {
    pub war_military_share: f64,
    pub peace_military_share: f64,
    pub high_tax_max_unrest: f64,
    pub low_tax_min_unrest: f64,
    pub revolt_margin: f64,
    /// WH econ: weighted unrest above which the AI eases one province to
    /// « Bas » on its own (above 100: never).
    #[serde(default = "never_unrest")]
    pub province_tax_relief_unrest: f64,
    pub high_tax_hysteresis: f64,
    pub safety_margin_percent: i64,
    pub max_building_upkeep_percent: i64,
    pub debt_repayment_turns: i64,
    pub max_disbands_per_turn: usize,
    pub income_per_recruit: i64,
    pub reserve_seasons: i64,
    pub hoard_spending_turns: i64,
    pub hoard_livres_per_recruit: i64,
    pub max_recruits_per_turn: i64,
    pub war_runway_turns: i64,
    pub peace_runway_turns: i64,
    pub donation_percent: i64,
    pub donation_favor: u8,
    pub donation_min: i64,
    pub livres_per_extra_build: i64,
    pub max_builds_per_turn: usize,
}

crate::bundled_rules!(AiCampaign, "ai/campaign.json", default);

#[cfg(test)]
mod tests {
    use super::AiCampaign;

    #[test]
    fn bundled_file_parses() {
        assert_eq!(AiCampaign::bundled().military.planning_range, 10);
    }
}
