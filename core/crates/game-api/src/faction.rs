//! Faction sheet: the first view that proves the `core` -> `game-api` -> client pattern.

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_campaign::CampaignState;

/// What a header or a faction list shows about one faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FactionSummary {
    pub id: String,
    /// French name shown in the UI.
    pub display_name: String,
    /// Livres tournois; may be negative.
    pub treasury: i64,
    /// Provinces the faction controls.
    pub province_count: usize,
}

/// Summary of `faction`, `None` when it is unknown to the data or to the campaign state.
pub fn faction_summary(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> Option<FactionSummary> {
    let definition = data.factions.get(faction)?;
    let summary = state.faction_summary(faction)?;
    Some(FactionSummary {
        id: faction.as_str().to_owned(),
        display_name: definition.name.display.clone(),
        treasury: summary.treasury,
        province_count: summary.provinces_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn france_summary_at_start() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let id = FactionId::new("fac_france").unwrap();
        let summary = faction_summary(data, &state, &id).expect("france exists");
        assert_eq!(summary.id, "fac_france");
        assert!(!summary.display_name.is_empty());
        assert!(summary.province_count > 0);
        assert_eq!(
            summary.treasury,
            state.faction_summary(&id).unwrap().treasury
        );
        assert!(serde_json::to_string(&summary)
            .unwrap()
            .contains("display_name"));
    }

    #[test]
    fn unknown_faction_is_none() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let id = FactionId::new("fac_nowhere").unwrap();
        assert!(faction_summary(data, &state, &id).is_none());
    }
}
