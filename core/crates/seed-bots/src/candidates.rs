//! Legal candidate orders: the orders the requirement and preview queries
//! declare possible for the player this turn.

use data_model::GameData;
use sim_campaign::Order;

use crate::view::BotView;

/// Family of an order, used by bots to weigh candidates by personality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Moves, attacks, sieges, embarking.
    Military,
    /// Recruitment and army organisation.
    Recruitment,
    /// Buildings, taxes, edicts, research.
    Economy,
    /// War, peace, alliances, treaties, marriages, vassals.
    Diplomacy,
    /// Characters, agents, council, captives, events.
    Court,
}

/// A candidate order with its family.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// The order, which the queries say the rules should accept.
    pub order: Order,
    /// Its family.
    pub family: Family,
}

/// Every order the queries declare possible for the bot's faction this turn.
pub fn legal_candidates(_view: &BotView, _data: &GameData) -> Vec<Candidate> {
    todo!("lot B")
}
