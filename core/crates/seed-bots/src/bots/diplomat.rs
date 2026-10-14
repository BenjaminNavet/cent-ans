//! `diplomat`: peace, alliances, gifts, favourable answers; never declares war.

use data_model::GameData;
use sim_campaign::rng::CampaignRng;
use sim_campaign::Order;

use super::draw_weighted;
use crate::bot::Bot;
use crate::candidates::{legal_candidates, Candidate, Family};
use crate::view::BotView;

/// Diplomacy only in spirit: marriages, vassalage and treaties have no
/// prerequisite query (see `docs/wip/seed-bots.md`), so it draws on peace,
/// alliance, gift and offer answers.
#[derive(Debug, Default)]
pub struct Diplomat;

impl Bot for Diplomat {
    fn name(&self) -> &'static str {
        "diplomat"
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
            Order::AnswerOffer { .. } => 40,
            Order::ProposePeace { .. } => 25,
            Order::ProposeAlliance { .. } => 20,
            Order::SendGift { .. } => 10,
            _ => match c.family {
                Family::Diplomacy => 10,
                Family::Court => 3,
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
    fn diplomacy_without_war() {
        check(
            || Box::new(Diplomat),
            |o| {
                matches!(
                    o,
                    Order::ProposeAlliance { .. }
                        | Order::SendGift { .. }
                        | Order::ProposePeace { .. }
                )
            },
        );
        let log = play(&mut Diplomat, 4, 5);
        assert!(!log
            .iter()
            .flatten()
            .any(|o| matches!(o, Order::DeclareWar { .. })));
    }
}
