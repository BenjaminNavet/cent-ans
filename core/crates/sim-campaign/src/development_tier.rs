//! Development tier of a settlement (CO-C, ADR 0292): a purely visual 1..=6
//! reading of how built-up a settlement is, driving the illustration of the
//! « Bâtiments » tab. No game rule reads it.
//!
//! Score = tier of the highest building of each standing chain + the
//! settlement's fortification level; reachable maximum = the best
//! `building_slot_cap` chains of its kind (by top tier) + the fortification
//! maximum. Thresholds live in `settlements/rules.json` `development_tiers`.

use std::collections::BTreeMap;

use data_model::{BuildingId, GameData, SettlementId};

use crate::state::CampaignState;

/// Lowest development tier.
pub const MIN_TIER: u8 = 1;
/// Highest development tier.
pub const MAX_TIER: u8 = 6;

/// Tier (1..=6) for `score` out of `maximum` against ascending percent
/// `thresholds` (tier n+1 starts at `thresholds[n]` %).
pub fn tier_for(score: u32, maximum: u32, thresholds: &[u32]) -> u8 {
    if maximum == 0 {
        return MIN_TIER;
    }
    let percent = u64::from(score.min(maximum)) * 100;
    let passed = thresholds
        .iter()
        .take(usize::from(MAX_TIER - 1))
        .filter(|t| percent >= u64::from(**t) * u64::from(maximum))
        .count();
    MIN_TIER + passed as u8
}

/// Progress (0..=100 %) from the current tier's threshold towards the next
/// one; 100 at the top tier.
pub fn tier_progress_percent(score: u32, maximum: u32, thresholds: &[u32]) -> u8 {
    let tier = tier_for(score, maximum, thresholds);
    if tier >= MAX_TIER || maximum == 0 {
        return 100;
    }
    let index = usize::from(tier - MIN_TIER);
    let low = if index == 0 { 0 } else { thresholds[index - 1] };
    let Some(&high) = thresholds.get(index) else {
        return 100;
    };
    let percent = u64::from(score.min(maximum)) * 100 / u64::from(maximum);
    let span = u64::from(high.saturating_sub(low)).max(1);
    (percent.saturating_sub(u64::from(low)) * 100 / span).min(100) as u8
}

impl CampaignState {
    /// `(score, maximum)` of `settlement`'s development; `None` for an
    /// unknown settlement.
    pub fn development_score(
        &self,
        data: &GameData,
        settlement: &SettlementId,
    ) -> Option<(u32, u32)> {
        let state = self.settlement_state(settlement)?;
        let kind = state.kind;
        let tier_of = |id: &BuildingId| data.buildings.get(id).map_or(0, |b| u32::from(b.tier));
        let fort_max = data
            .settlement_rules
            .as_ref()
            .and_then(|r| r.development_tiers.as_ref())
            .map_or(0, |d| d.fortification_max);
        let built: u32 = data
            .normalize_building_tiers(&state.buildings)
            .iter()
            .map(tier_of)
            .sum();
        let score = built + u32::from(state.fortification_level).min(fort_max);
        // Top tier of every chain the kind allows, keyed by chain root.
        let mut chains: BTreeMap<BuildingId, u32> = BTreeMap::new();
        for building in data.buildings.values().filter(|b| b.allowed_in(kind)) {
            let mut root = building.id.clone();
            for _ in 0..16 {
                let parent = data
                    .buildings
                    .get(&root)
                    .and_then(|b| b.upgrades_from.clone())
                    .filter(|p| data.buildings.get(p).is_some_and(|b| b.allowed_in(kind)));
                match parent {
                    Some(parent) => root = parent,
                    None => break,
                }
            }
            let top = chains.entry(root).or_insert(0);
            *top = (*top).max(u32::from(building.tier));
        }
        let mut tops: Vec<u32> = chains.into_values().collect();
        tops.sort_unstable_by(|a, b| b.cmp(a));
        let cap = data
            .settlement_rules
            .as_ref()
            .and_then(|r| r.building_slot_cap.get(&kind))
            .copied()
            .unwrap_or(tops.len());
        let maximum = tops.iter().take(cap).sum::<u32>() + fort_max;
        Some((score.min(maximum), maximum))
    }

    /// Development tier of `settlement`, 1..=6 (1 for an unknown one or when
    /// the rules carry no `development_tiers`). Purely visual.
    pub fn settlement_tier(&self, data: &GameData, settlement: &SettlementId) -> u8 {
        let Some(tiers) = data
            .settlement_rules
            .as_ref()
            .and_then(|r| r.development_tiers.as_ref())
        else {
            return MIN_TIER;
        };
        self.development_score(data, settlement)
            .map_or(MIN_TIER, |(score, maximum)| {
                tier_for(score, maximum, &tiers.thresholds_percent)
            })
    }

    /// Progress (0..=100 %) of `settlement` towards its next development
    /// tier; 100 at the top tier, 0 without `development_tiers`.
    pub fn settlement_tier_progress(&self, data: &GameData, settlement: &SettlementId) -> u8 {
        let Some(tiers) = data
            .settlement_rules
            .as_ref()
            .and_then(|r| r.development_tiers.as_ref())
        else {
            return 0;
        };
        self.development_score(data, settlement)
            .map_or(0, |(score, maximum)| {
                tier_progress_percent(score, maximum, &tiers.thresholds_percent)
            })
    }
}
