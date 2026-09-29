//! Lot TW2-T3 (ADR 0103): the campaign AI hires mercenary companies when it
//! is rich and one of its armies is threatened (`data/rules/mercenaries.json`,
//! `ai`).
//!
//! - Rich: a treasury of at least `rich_income_percent` of a season of gross income and
//!   `rich_min_treasury` livres, after this turn's other spending.
//! - Threatened: the hostile power near the army (its anchor settlement or
//!   one edge away) is at least `threat_ratio_percent` of its own.
//! - It hires the best companies per livre offered where the army stands
//!   until its power reaches `target_ratio_percent` of the threat, keeping
//!   `keep_income_percent` of income in the treasury, within the limits per
//!   army and per faction and the regional reserves (the same checks as the
//!   player's, counted here so that no order is refused).

use std::collections::BTreeMap;

use data_model::{FactionId, GameData};
use sim_campaign::state::unit_power;
use sim_campaign::{ArmyId, CampaignState, Order, Unit};

/// Hiring orders of `faction` this turn. `treasury` is what its other
/// orders of the turn leave; `gross_income` its seasonal gross income;
/// `threat_of` the hostile power near an army.
pub(crate) fn plan_hires(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    treasury: i64,
    gross_income: i64,
    threat_of: impl Fn(&ArmyId) -> f64,
) -> Vec<Order> {
    let rules = &data.mercenary_rules;
    let ai = &rules.ai;
    if !state.factions.contains_key(faction) {
        return Vec::new();
    }
    let income = gross_income.max(0);
    if treasury
        < ai.rich_min_treasury
            .max(ai.rich_income_percent * income / 100)
    {
        return Vec::new();
    }
    let mut budget = treasury - ai.keep_income_percent * income / 100;
    let mut faction_left = rules
        .hires_per_faction_per_turn
        .saturating_sub(faction_hires(state, faction));
    // Companies drawn this turn from each reserve (band, region).
    let mut drawn: BTreeMap<(String, String), u32> = BTreeMap::new();
    let mut orders = Vec::new();
    let armies: Vec<&ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction)
        .map(|(id, _)| id)
        .collect();
    for army_id in armies {
        if faction_left == 0 || budget <= 0 {
            break;
        }
        let threat = threat_of(army_id);
        let mut power = state.army_power(data, army_id);
        if threat <= 0.0 || threat * 100.0 < power * f64::from(ai.threat_ratio_percent) {
            continue;
        }
        let target = threat * f64::from(ai.target_ratio_percent) / 100.0;
        let Some(market) = state.mercenary_market(data, army_id) else {
            continue;
        };
        let Some(region) = market.region.clone() else {
            continue;
        };
        if market.blocked.is_some() {
            continue;
        }
        // Best power per livre first.
        let mut options: Vec<(f64, f64, &sim_campaign::MercenaryOption)> = market
            .options
            .iter()
            .filter(|o| o.available)
            .filter_map(|o| {
                let unit = data.unit_types.get(&o.unit_type)?;
                let company = unit_power(data, &[Unit::fresh(unit)]);
                Some((company / f64::from(o.cost.max(1)), company, o))
            })
            .collect();
        options.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then_with(|| a.2.unit_type.cmp(&b.2.unit_type))
        });
        let mut army_left = market.hires_left;
        for (_, company, option) in options {
            let key = (option.band.clone(), region.clone());
            loop {
                let used = drawn.get(&key).copied().unwrap_or(0);
                if army_left == 0
                    || faction_left == 0
                    || power >= target
                    || used >= option.pool.available
                    || budget < i64::from(option.cost)
                {
                    break;
                }
                orders.push(Order::HireMercenary {
                    army: army_id.clone(),
                    unit: option.unit_type.clone(),
                });
                drawn.insert(key.clone(), used + 1);
                army_left -= 1;
                faction_left -= 1;
                budget -= i64::from(option.cost);
                power += company;
            }
        }
    }
    orders
}

/// Companies `faction` already hired this turn.
fn faction_hires(state: &CampaignState, faction: &FactionId) -> u32 {
    let hires = &state.mercenaries;
    if hires.hires_turn != state.turn {
        return 0;
    }
    hires.faction_hires.get(faction).copied().unwrap_or(0)
}
