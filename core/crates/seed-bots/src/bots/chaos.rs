//! `chaos`: boundary and contradictory orders, to provoke panics and broken invariants.

use data_model::{GameData, SettlementId};
use sim_campaign::orders::Place;
use sim_campaign::rng::CampaignRng;
use sim_campaign::ArmyId;
use sim_campaign::Order;

use super::MAX_ORDERS;
use crate::bot::Bot;
use crate::candidates::legal_candidates;
use crate::view::BotView;

/// Legal candidates mixed with hand-built orders on its own armies and
/// settlements: split, merge, cancel, disband, in contradictory sequences.
///
/// The hand-built orders are unchecked and may be refused; that is expected.
/// `Debug*` orders are never used.
#[derive(Debug, Default)]
pub struct Chaos;

/// Own armies and own settlements, in id order.
fn own_things(view: &BotView) -> (Vec<ArmyId>, Vec<SettlementId>) {
    let armies = view
        .state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == view.faction)
        .map(|(id, _)| id.clone())
        .collect();
    let settlements = view
        .state
        .settlements
        .iter()
        .filter(|(_, s)| &s.owner == view.faction)
        .map(|(id, _)| id.clone())
        .collect();
    (armies, settlements)
}

fn pick<'a, T>(items: &'a [T], rng: &mut CampaignRng) -> Option<&'a T> {
    if items.is_empty() {
        None
    } else {
        items.get(rng.below(items.len() as u32) as usize)
    }
}

/// One scripted burst of boundary orders (1 to 3 orders).
fn burst(
    view: &BotView,
    armies: &[ArmyId],
    settlements: &[SettlementId],
    rng: &mut CampaignRng,
) -> Vec<Order> {
    let army = pick(armies, rng).cloned();
    let other = pick(armies, rng).cloned();
    let place = pick(settlements, rng).cloned().map(Place::Settlement);
    let size = army
        .as_ref()
        .and_then(|id| view.state.armies.get(id))
        .map_or(0, |a| a.units.len());
    let index = if rng.chance_permille(300) {
        rng.below(3) as usize + size
    } else {
        rng.below(size.max(1) as u32) as usize
    };
    match (rng.below(12), army, other, place) {
        (0, Some(army), _, _) => vec![Order::SplitArmy {
            army,
            unit_indices: vec![index],
        }],
        (1, Some(army), _, _) => vec![Order::SplitArmy {
            army,
            unit_indices: (0..size).collect(),
        }],
        (2, Some(army), _, _) => vec![Order::SplitArmy {
            army,
            unit_indices: Vec::new(),
        }],
        (3, Some(source), Some(target), _) => vec![Order::MergeArmies { source, target }],
        (4, Some(source), _, _) => vec![Order::MergeArmies {
            target: source.clone(),
            source,
        }],
        (5, _, _, Some(settlement)) => vec![
            Order::CancelRecruit {
                settlement: settlement.clone(),
                index,
            },
            Order::CancelBuild { settlement },
        ],
        (6, Some(army), _, _) => vec![
            Order::DisbandUnit {
                army: Some(army.clone()),
                settlement: None,
                unit_index: index,
            },
            Order::DisbandUnit {
                army: Some(army),
                settlement: None,
                unit_index: index,
            },
        ],
        (7, _, _, Some(settlement)) => vec![Order::DisbandUnit {
            army: None,
            settlement: Some(settlement),
            unit_index: index,
        }],
        // Move then split / disband the same army, whatever happened.
        (8, Some(army), _, Some(Place::Settlement(to))) => vec![
            Order::move_to(army.clone(), to),
            Order::SplitArmy {
                army,
                unit_indices: vec![0],
            },
        ],
        // Split then merge the halves back, then split again.
        (9, Some(army), Some(other), _) => vec![
            Order::SplitArmy {
                army: army.clone(),
                unit_indices: vec![0],
            },
            Order::MergeArmies {
                source: other.clone(),
                target: army.clone(),
            },
            Order::SplitArmy {
                army,
                unit_indices: vec![0],
            },
        ],
        // Both a settlement and an army named in the same disband.
        (10, Some(army), _, Some(settlement)) => vec![Order::DisbandUnit {
            army: Some(army),
            settlement: Some(settlement),
            unit_index: index,
        }],
        _ => Vec::new(),
    }
}

impl Bot for Chaos {
    fn name(&self) -> &'static str {
        "chaos"
    }

    fn plan(&mut self, view: &BotView, data: &GameData, rng: &mut CampaignRng) -> Vec<Order> {
        let (armies, settlements) = own_things(view);
        let mut pool = legal_candidates(view, data);
        let mut orders = Vec::new();
        for _ in 0..rng.below(4) {
            if pool.is_empty() {
                break;
            }
            let index = rng.below(pool.len() as u32) as usize;
            orders.push(pool.swap_remove(index).order);
        }
        for _ in 0..1 + rng.below(3) {
            orders.extend(burst(view, &armies, &settlements, rng));
        }
        // Shuffle (Fisher-Yates) so that contradictions interleave with candidates.
        for i in (1..orders.len()).rev() {
            let j = rng.below(i as u32 + 1) as usize;
            if rng.chance_permille(500) {
                orders.swap(i, j);
            }
        }
        orders.truncate(MAX_ORDERS as usize);
        orders
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bots::test_support::check;

    #[test]
    #[ignore = "slow in debug (~8 s): cargo test -p seed-bots -- --ignored"]
    fn emits_boundary_orders() {
        check(
            || Box::new(Chaos),
            |o| {
                matches!(
                    o,
                    Order::SplitArmy { .. }
                        | Order::MergeArmies { .. }
                        | Order::CancelRecruit { .. }
                        | Order::CancelBuild { .. }
                        | Order::DisbandUnit { .. }
                )
            },
        );
    }
}
