//! `builder`: builds, taxes, researches; recruits little; accepts peace.

use data_model::GameData;
use sim_campaign::rng::CampaignRng;
use sim_campaign::Order;

use super::draw_weighted;
use crate::bot::Bot;
use crate::candidates::{legal_candidates, Candidate, Family};
use crate::view::BotView;

/// Economy first, few soldiers, peace welcome.
#[derive(Debug, Default)]
pub struct Builder;

impl Bot for Builder {
    fn name(&self) -> &'static str {
        "builder"
    }

    fn plan(&mut self, view: &BotView, data: &GameData, rng: &mut CampaignRng) -> Vec<Order> {
        let pool: Vec<Candidate> = legal_candidates(view, data)
            .into_iter()
            .filter(|c| {
                !matches!(c.order, Order::DeclareWar { .. })
                    && !matches!(c.order, Order::AnswerOffer { accept: false, .. })
            })
            .collect();
        let weight = |c: &Candidate| match &c.order {
            Order::Build { .. } => 30,
            Order::Research { .. } => 14,
            Order::SetTaxRate { .. } | Order::SetEdict { .. } | Order::SetDiet { .. } => 8,
            Order::ProposePeace { .. } | Order::AnswerOffer { .. } => 12,
            Order::Recruit { .. } => 1,
            _ => match c.family {
                Family::Economy => 6,
                Family::Court => 3,
                Family::Diplomacy => 2,
                Family::Military => 1,
                Family::Recruitment => 1,
            },
        };
        draw_weighted(pool, weight, 8, rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bots::test_support::check;

    #[test]
    #[ignore = "slow in debug (~8 s): cargo test -p seed-bots -- --ignored"]
    fn builds_and_manages() {
        check(
            || Box::new(Builder),
            |o| matches!(o, Order::Build { .. } | Order::Research { .. }),
        );
    }
}
