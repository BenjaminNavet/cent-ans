//! `mirror`: the game's own AI plays the player's faction.

use data_model::GameData;
use sim_campaign::negotiation::evaluate_treaty;
use sim_campaign::rng::CampaignRng;
use sim_campaign::Order;

use crate::bot::Bot;
use crate::view::BotView;

/// Answers offers as the AI would, then returns `ai::plan_turn`.
#[derive(Debug, Default)]
pub struct Mirror;

impl Bot for Mirror {
    fn name(&self) -> &'static str {
        "mirror"
    }

    fn plan(&mut self, view: &BotView, data: &GameData, _rng: &mut CampaignRng) -> Vec<Order> {
        let state = view.state;
        let mut orders: Vec<Order> = state
            .factions
            .get(view.faction)
            .map(|f| f.offers.as_slice())
            .unwrap_or_default()
            .iter()
            .map(|offer| Order::AnswerOffer {
                offer: offer.id,
                accept: evaluate_treaty(
                    state,
                    data,
                    &offer.from,
                    view.faction,
                    &offer.proposal.articles,
                )
                .accept,
            })
            .collect();
        orders.extend(ai::plan_turn(state, data, view.faction));
        orders
    }
}
