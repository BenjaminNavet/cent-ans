//! `turtle`: defends, never expands, never declares war.

use data_model::{BuildingCategory, GameData};
use sim_campaign::rng::CampaignRng;
use sim_campaign::{Order, Stance};

use super::draw_weighted;
use crate::bot::Bot;
use crate::candidates::{legal_candidates, Candidate, Family};
use crate::view::BotView;

/// Defensive stances, recruits, fortifications; no march, no war.
#[derive(Debug, Default)]
pub struct Turtle;

impl Bot for Turtle {
    fn name(&self) -> &'static str {
        "turtle"
    }

    fn plan(&mut self, view: &BotView, data: &GameData, rng: &mut CampaignRng) -> Vec<Order> {
        let pool: Vec<Candidate> = legal_candidates(view, data)
            .into_iter()
            .filter(|c| match &c.order {
                Order::MoveArmy { .. } | Order::DeclareWar { .. } | Order::Assault { .. } => false,
                Order::SetStance { stance, .. } => {
                    matches!(stance, Stance::Entrenched | Stance::Ambush)
                }
                _ => true,
            })
            .collect();
        let fortification = |c: &Candidate| match &c.order {
            Order::Build { building, .. } => data.buildings.get(building).is_some_and(|b| {
                matches!(
                    b.category,
                    BuildingCategory::Fortification | BuildingCategory::Military
                )
            }),
            _ => false,
        };
        let weight = |c: &Candidate| match &c.order {
            Order::SetStance { .. } => 25,
            Order::Recruit { .. } => 15,
            Order::Build { .. } if fortification(c) => 25,
            Order::Build { .. } => 2,
            Order::ProposePeace { .. } | Order::AnswerOffer { accept: true, .. } => 6,
            _ => match c.family {
                Family::Economy => 2,
                _ => 1,
            },
        };
        draw_weighted(pool, weight, 6, rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bots::test_support::{check, play};

    #[test]
    #[ignore = "slow in debug (~8 s): cargo test -p seed-bots -- --ignored"]
    fn defends_and_never_marches() {
        check(
            || Box::new(Turtle),
            |o| {
                matches!(
                    o,
                    Order::SetStance { .. } | Order::Recruit { .. } | Order::Build { .. }
                )
            },
        );
        let log = play(&mut Turtle, 6, 5);
        assert!(!log
            .iter()
            .flatten()
            .any(|o| matches!(o, Order::MoveArmy { .. } | Order::DeclareWar { .. })));
    }
}
