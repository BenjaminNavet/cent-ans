//! `aggressor`: declares wars, marches on the enemy, attacks and besieges.

use data_model::{GameData, SettlementId};
use sim_campaign::rng::CampaignRng;
use sim_campaign::Order;

use super::{draw_weighted, moves_closer};
use crate::bot::Bot;
use crate::candidates::{legal_candidates, Candidate, Family};
use crate::view::BotView;

/// Favours military action and recruitment, declares war, never offers peace.
#[derive(Debug, Default)]
pub struct Aggressor;

/// Map points of the settlements to march on: those of the factions at war
/// with us, or of our neighbours when at peace with everyone.
pub(crate) fn enemy_points(view: &BotView, data: &GameData) -> Vec<[f32; 2]> {
    let state = view.state;
    let mut enemies: Vec<_> = state
        .factions
        .get(view.faction)
        .map(|f| f.at_war_with.iter().cloned().collect())
        .unwrap_or_default();
    if enemies.is_empty() {
        enemies.extend(state.neighbour_factions(data, view.faction));
    }
    let places: Vec<&SettlementId> = state
        .settlements
        .iter()
        .filter(|(_, s)| enemies.contains(&s.owner))
        .map(|(id, _)| id)
        .collect();
    places
        .into_iter()
        .filter_map(|id| data.settlement_point(id))
        .collect()
}

impl Bot for Aggressor {
    fn name(&self) -> &'static str {
        "aggressor"
    }

    fn plan(&mut self, view: &BotView, data: &GameData, rng: &mut CampaignRng) -> Vec<Order> {
        let targets = enemy_points(view, data);
        let pool: Vec<Candidate> = legal_candidates(view, data)
            .into_iter()
            .filter(|c| {
                !matches!(c.order, Order::ProposePeace { .. })
                    && !matches!(c.order, Order::AnswerOffer { accept: true, .. })
            })
            .collect();
        let weight = |c: &Candidate| match &c.order {
            Order::Assault { .. } => 40,
            Order::DeclareWar { .. } => 12,
            Order::MoveArmy { .. } if moves_closer(view, data, &c.order, &targets) => 20,
            Order::MoveArmy { .. } => 3,
            _ => match c.family {
                Family::Military => 6,
                Family::Recruitment => 8,
                Family::Economy => 1,
                _ => 1,
            },
        };
        draw_weighted(pool, weight, 10, rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bots::test_support::check;

    #[test]
    #[ignore = "slow in debug (~8 s): cargo test -p seed-bots -- --ignored"]
    fn aggressive_orders_and_no_peace() {
        check(
            || Box::new(Aggressor),
            |o| {
                matches!(
                    o,
                    Order::MoveArmy { .. } | Order::Recruit { .. } | Order::DeclareWar { .. }
                )
            },
        );
        let log = crate::bots::test_support::play(&mut Aggressor, 3, 5);
        assert!(!log
            .iter()
            .flatten()
            .any(|o| matches!(o, Order::ProposePeace { .. })));
    }
}
