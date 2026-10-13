//! Read-only view of the campaign for a bot: what a player may see.

use data_model::FactionId;
use sim_campaign::CampaignState;

/// What the bot sees of the game: the state, read-only, and its own faction.
///
/// Bots only read through the queries the interface already uses, so they
/// see what a player sees.
pub struct BotView<'a> {
    /// The campaign state.
    pub state: &'a CampaignState,
    /// The faction the bot plays.
    pub faction: &'a FactionId,
}
