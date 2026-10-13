//! `random-legal`: orders drawn uniformly among the legal candidates.

use data_model::GameData;
use sim_campaign::rng::CampaignRng;
use sim_campaign::Order;

use crate::bot::Bot;
use crate::candidates::legal_candidates;
use crate::view::BotView;

/// Most orders drawn in one turn.
const MAX_ORDERS: u32 = 8;

/// Draws 0..=[`MAX_ORDERS`] distinct candidates uniformly each turn.
#[derive(Debug, Default)]
pub struct RandomLegal;

impl Bot for RandomLegal {
    fn name(&self) -> &'static str {
        "random-legal"
    }

    fn plan(&mut self, view: &BotView, data: &GameData, rng: &mut CampaignRng) -> Vec<Order> {
        let mut pool = legal_candidates(view, data);
        let wanted = rng.below(MAX_ORDERS + 1) as usize;
        let mut orders = Vec::new();
        while orders.len() < wanted && !pool.is_empty() {
            let index = rng.below(pool.len() as u32) as usize;
            orders.push(pool.swap_remove(index).order);
        }
        orders
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::fixtures;

    #[test]
    fn same_seed_same_orders() {
        let data = fixtures::data();
        let state = fixtures::state();
        let faction = fixtures::france();
        let view = BotView {
            state: &state,
            faction: &faction,
        };
        let plan = |seed| RandomLegal.plan(&view, data, &mut CampaignRng::from_seed(seed));
        assert_eq!(plan(5), plan(5));
        assert!((0..20).any(|seed| plan(seed) != plan(seed + 100)));
    }
}
