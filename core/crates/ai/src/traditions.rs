//! Army traditions of the AI (lot TW2-T5, ADR 0112).
//!
//! Each rank an AI army reaches is spent at once on the tradition whose
//! branch scores best (`data/rules/army_traditions.json`, `ai`): a base
//! weight per branch, plus the share of shooters in the faction's
//! recruitment doctrine (shooting: England's longbows), the army's share of
//! missing men (stewardship), an ongoing siege (assault) and a branch
//! already started (deepen it). Ties go to the data order of the branches.

use data_model::{FactionId, GameData, TraditionBranch, UnitCategory};
use sim_campaign::traditions::{available_traditions, pending_choices};
use sim_campaign::{Army, ArmyId, CampaignState, Order, Stance};

/// Share (0-1) of shooting units in `faction`'s recruitment doctrine;
/// without a doctrine, in its field armies.
pub fn doctrine_ranged_share(state: &CampaignState, data: &GameData, faction: &FactionId) -> f64 {
    let is_ranged = |unit_type| {
        data.unit_types
            .get(unit_type)
            .is_some_and(|t| t.category == UnitCategory::Ranged)
    };
    let (ranged, total) = match data.ai_doctrines.as_ref().map(|d| d.of(faction)) {
        Some(doctrine) if !doctrine.mix.is_empty() => {
            doctrine
                .mix
                .iter()
                .fold((0u64, 0u64), |(ranged, total), (unit_type, weight)| {
                    let weight = u64::from(*weight);
                    (
                        ranged + if is_ranged(unit_type) { weight } else { 0 },
                        total + weight,
                    )
                })
        }
        _ => crate::doctrine::field_composition(state, faction)
            .iter()
            .fold((0u64, 0u64), |(ranged, total), (unit_type, count)| {
                let count = u64::from(*count);
                (
                    ranged + if is_ranged(unit_type) { count } else { 0 },
                    total + count,
                )
            }),
    };
    if total == 0 {
        0.0
    } else {
        ranged as f64 / total as f64
    }
}

/// Score of `branch` for `army` (higher is better).
fn branch_score(data: &GameData, army: &Army, ranged_share: f64, branch: TraditionBranch) -> f64 {
    let ai = &data.army_tradition_rules.ai;
    let mut score = f64::from(ai.base_weight.get(branch));
    match branch {
        TraditionBranch::Shooting => score += f64::from(ai.doctrine_ranged_weight) * ranged_share,
        TraditionBranch::Stewardship => {
            let (missing, max) = army.units.iter().fold((0u64, 0u64), |(m, t), u| {
                (
                    m + u64::from(u.max_strength.saturating_sub(u.strength)),
                    t + u64::from(u.max_strength),
                )
            });
            if max > 0 {
                score += f64::from(ai.missing_men_weight) * missing as f64 / max as f64;
            }
        }
        TraditionBranch::Assault if army.stance == Stance::Siege => {
            score += f64::from(ai.besieging_weight);
        }
        _ => {}
    }
    let started = army.traditions.chosen.iter().any(|id| {
        data.army_tradition_rules
            .tradition(id)
            .is_some_and(|t| t.branch == branch)
    });
    if started {
        score += f64::from(ai.started_branch_weight);
    }
    score
}

/// The tradition the AI takes for `army` now, if it has a rank to spend.
pub fn pick_tradition(state: &CampaignState, data: &GameData, army: &Army) -> Option<String> {
    if pending_choices(data, army) == 0 {
        return None;
    }
    let ranged_share = doctrine_ranged_share(state, data, &army.faction);
    available_traditions(data, army)
        .into_iter()
        .map(|t| (branch_score(data, army, ranged_share, t.branch), t))
        // Best score; ties to the lowest tier, then the data order.
        .fold(
            None,
            |best: Option<(f64, &data_model::ArmyTradition)>, (score, t)| match best {
                Some((best_score, best_t))
                    if best_score > score || (best_score == score && best_t.tier <= t.tier) =>
                {
                    Some((best_score, best_t))
                }
                _ => Some((score, t)),
            },
        )
        .map(|(_, t)| t.id.clone())
}

/// `ChooseArmyTradition` orders of `faction` for every rank its armies can
/// spend (several ranks of one army in a row).
pub fn plan_traditions(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let mut orders = Vec::new();
    let armies: Vec<(&ArmyId, &Army)> = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction && pending_choices(data, a) > 0)
        .collect();
    for (id, army) in armies {
        let mut planned = army.clone();
        while let Some(tradition) = pick_tradition(state, data, &planned) {
            planned.traditions.chosen.push(tradition.clone());
            orders.push(Order::ChooseArmyTradition {
                army: id.clone(),
                tradition,
            });
        }
    }
    orders
}
