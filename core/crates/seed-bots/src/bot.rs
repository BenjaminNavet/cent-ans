//! The bot trait: one personality choosing the player's orders each turn.

use data_model::GameData;
use sim_campaign::rng::CampaignRng;
use sim_campaign::Order;

use crate::view::BotView;

/// A bot plays the player's faction: each turn it returns the orders to submit.
pub trait Bot: Send {
    /// Stable name, used in reports and on the command line (`random-legal`, `chaos`…).
    fn name(&self) -> &'static str;

    /// The orders for this turn, in submission order.
    fn plan(&mut self, view: &BotView, data: &GameData, rng: &mut CampaignRng) -> Vec<Order>;
}
