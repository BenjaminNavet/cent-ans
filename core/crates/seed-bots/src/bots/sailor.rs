//! `sailor`: heads for ports, crosses the sea, sails the rivers.

use data_model::{GameData, SettlementId};
use sim_campaign::rng::CampaignRng;
use sim_campaign::voyage::sea_voyages;
use sim_campaign::ArmyId;
use sim_campaign::Order;

use super::{army_point, dist2, draw_weighted, moves_closer, nearest};
use crate::bot::Bot;
use crate::candidates::{legal_candidates, Candidate, Family};
use crate::view::BotView;

/// Favours moves towards ports and every sea or river crossing.
///
/// A port is a settlement with a `sea` edge in the movement graph (the
/// data the voyage rules read). `Embark` and `SailRiver` are built from the
/// `sea_voyages` and `river_voyages` queries; they are not in
/// `legal_candidates`.
#[derive(Debug, Default)]
pub struct Sailor;

/// Whether the settlement graph gives `id` a sea edge.
pub(crate) fn is_port(data: &GameData, id: &SettlementId) -> bool {
    data.movement_graph.edges(id).iter().any(|e| e.sea)
}

fn naval_candidates(view: &BotView, data: &GameData) -> Vec<Candidate> {
    let state = view.state;
    let mut out = Vec::new();
    let armies: Vec<(&ArmyId, &sim_campaign::Army)> = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == view.faction)
        .collect();
    let ports: Vec<(SettlementId, [f32; 2])> = state
        .settlements
        .iter()
        .filter(|(id, s)| &s.owner == view.faction && is_port(data, id))
        .filter_map(|(id, _)| Some((id.clone(), data.settlement_point(id)?)))
        .collect();
    let port_points: Vec<[f32; 2]> = ports.iter().map(|(_, p)| *p).collect();
    for (army_id, army) in armies {
        let Some(here) = army_point(view, data, army_id) else {
            continue;
        };
        if let Some(from) = army.settlement() {
            if army.movement_left > 0 {
                // The farthest port reachable by sea: a landing far away.
                let voyages = sea_voyages(state, data, view.faction, from);
                let far = voyages
                    .keys()
                    .filter_map(|port| Some((port, data.settlement_point(port)?)))
                    .max_by(|a, b| dist2(here, a.1).total_cmp(&dist2(here, b.1)));
                if let Some((port, _)) = far {
                    out.push(Candidate {
                        order: Order::Embark {
                            army: army_id.clone(),
                            to_port: port.clone(),
                        },
                        family: Family::Military,
                    });
                }
                let rivers = state.river_voyages(data, army_id);
                let far = rivers.iter().max_by_key(|(_, v)| v.cost);
                if let Some((to, _)) = far {
                    out.push(Candidate {
                        order: Order::SailRiver {
                            army: army_id.clone(),
                            to: to.clone(),
                        },
                        family: Family::Military,
                    });
                }
            }
            if is_port(data, from) {
                continue;
            }
        }
        if let Some(goal) = nearest(here, &port_points) {
            if let Some((port, _)) = ports.iter().find(|(_, p)| *p == goal) {
                out.push(Candidate {
                    order: Order::move_to(army_id.clone(), port.clone()),
                    family: Family::Military,
                });
            }
        }
    }
    out
}

impl Bot for Sailor {
    fn name(&self) -> &'static str {
        "sailor"
    }

    fn plan(&mut self, view: &BotView, data: &GameData, rng: &mut CampaignRng) -> Vec<Order> {
        let state = view.state;
        let port_points: Vec<[f32; 2]> = state
            .settlements
            .iter()
            .filter(|(id, s)| &s.owner == view.faction && is_port(data, id))
            .filter_map(|(id, _)| data.settlement_point(id))
            .collect();
        let mut pool = legal_candidates(view, data);
        pool.extend(naval_candidates(view, data));
        let weight = |c: &Candidate| match &c.order {
            Order::Embark { .. } => 60,
            Order::SailRiver { .. } => 40,
            Order::MoveArmy { .. } if moves_closer(view, data, &c.order, &port_points) => 25,
            Order::MoveArmy { .. } => 8,
            _ => match c.family {
                Family::Economy | Family::Recruitment => 2,
                _ => 1,
            },
        };
        draw_weighted(pool, weight, 6, rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bots::test_support::check;

    #[test]
    #[ignore = "slow in debug (~8 s): cargo test -p seed-bots -- --ignored"]
    fn heads_for_ports_and_water() {
        check(
            || Box::new(Sailor),
            |o| {
                matches!(
                    o,
                    Order::MoveArmy { .. } | Order::Embark { .. } | Order::SailRiver { .. }
                )
            },
        );
    }
}
