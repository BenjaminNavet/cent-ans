//! Recruitment doctrines (lot E1, audit A2 § 5.1).
//!
//! Each faction aims at a mix of unit types (`data/ai/doctrines.json`):
//! England archers and dismounted men-at-arms, France knights and
//! crossbowmen, Flanders pikes... At every recruitment the AI takes, among
//! the units it can raise and afford on the spot, the one furthest below its
//! share of the field regiments; the unit's value only breaks ties. Without
//! doctrines the AI keeps ranking units by value alone.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, UnitTypeId};
use sim_campaign::orders::RecruitOption;
use sim_campaign::CampaignState;

/// Weight of the unit's value against the doctrine's deficit (tie-break).
const VALUE_WEIGHT: f64 = 1e-3;

/// Field regiments of `faction`, by unit type (garrisons left out: they
/// are the realm's fixed cost, not its army).
pub fn field_composition(state: &CampaignState, faction: &FactionId) -> BTreeMap<UnitTypeId, u32> {
    let mut counts = BTreeMap::new();
    for army in state.armies.values().filter(|a| &a.faction == faction) {
        for unit in &army.units {
            *counts.entry(unit.unit_type.clone()).or_default() += 1;
        }
    }
    counts
}

/// Picks the recruit among `options` (all available and affordable): the
/// unit type furthest below its doctrine share, `value` breaking ties. The
/// doctrine's weights are renormalised over `options`; with no doctrine, or
/// none of its types on offer, the most valuable option wins.
pub fn pick_recruit<'a>(
    data: &GameData,
    faction: &FactionId,
    options: &[&'a RecruitOption],
    composition: &BTreeMap<UnitTypeId, u32>,
    value: impl Fn(&RecruitOption) -> f64,
) -> Option<&'a RecruitOption> {
    // A6-L3b: a type above its share cap (data/ai/doctrines.json) is not
    // recruited; `composition` then counts garrisons too (see campaign.rs).
    let uncapped: Vec<&'a RecruitOption>;
    let options: &[&'a RecruitOption] = match data.ai_doctrines.as_ref() {
        Some(d) if !d.share_caps.is_empty() => {
            let field_total: u32 = composition.values().sum();
            uncapped = options
                .iter()
                .copied()
                .filter(|o| {
                    d.share_caps.get(&o.unit_type).is_none_or(|cap| {
                        // Share counting the recruit itself.
                        let share =
                            f64::from(composition.get(&o.unit_type).copied().unwrap_or(0) + 1)
                                / f64::from(field_total + 1);
                        field_total < cap.min_field_units || share <= cap.max_share
                    })
                })
                .collect();
            &uncapped
        }
        _ => options,
    };
    let by_value = || {
        options.iter().copied().max_by(|a, b| {
            value(a)
                .total_cmp(&value(b))
                .then_with(|| b.unit_type.cmp(&a.unit_type))
        })
    };
    let Some(doctrine) = data.ai_doctrines.as_ref().map(|d| d.of(faction)) else {
        return by_value();
    };
    let weight =
        |o: &RecruitOption| f64::from(doctrine.mix.get(&o.unit_type).copied().unwrap_or(0));
    let total_weight: f64 = options.iter().map(|o| weight(o)).sum();
    if total_weight <= 0.0 {
        return by_value();
    }
    let on_offer: f64 = options
        .iter()
        .map(|o| f64::from(composition.get(&o.unit_type).copied().unwrap_or(0)))
        .sum();
    let max_value = options
        .iter()
        .map(|o| value(o))
        .fold(f64::EPSILON, f64::max);
    let score = |o: &RecruitOption| {
        let target = weight(o) / total_weight;
        // The share among the types on offer, counting the recruit itself.
        let current =
            f64::from(composition.get(&o.unit_type).copied().unwrap_or(0)) / (on_offer + 1.0);
        let deficit = if weight(o) > 0.0 {
            target - current
        } else {
            -1.0
        };
        deficit + VALUE_WEIGHT * value(o) / max_value
    };
    options.iter().copied().max_by(|a, b| {
        score(a)
            .total_cmp(&score(b))
            .then_with(|| b.unit_type.cmp(&a.unit_type))
    })
}
