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

/// Shared fixtures of the unit tests: the real `data/` and a 1337 France state.
#[cfg(test)]
pub(crate) mod fixtures {
    use std::sync::OnceLock;

    use data_model::{FactionId, GameData};
    use sim_campaign::CampaignState;

    /// The real game data, loaded once.
    pub fn data() -> &'static GameData {
        static DATA: OnceLock<GameData> = OnceLock::new();
        DATA.get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
            GameData::load(&root).expect("data").0
        })
    }

    /// The France faction id.
    pub fn france() -> FactionId {
        FactionId::new("fac_france").expect("id")
    }

    /// A fresh 1337 campaign as France.
    pub fn state() -> CampaignState {
        CampaignState::new_1337(data(), france(), 7).expect("state")
    }
}
