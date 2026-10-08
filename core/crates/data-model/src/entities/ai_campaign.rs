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
