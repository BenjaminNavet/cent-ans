//! Spring 1337 sheets of the playable factions, for the faction chooser (spec FE § 6).
//!
//! Port of `GameDataStore.get_feudal_start_sheets` (`godot-bridge`, `campaign_sim_feudal.rs`):
//! sovereigns first then by name; each sheet carries the feudal sheet (ruler, titles, suzerain,
//! objectives), the provinces owned in 1337, the key figures (treasury, projected income,
//! armies) and the kingdom (primary title of the sovereign). No rule here: everything comes from
//! `sim_campaign::feudal`.

use data_model::{FactionId, GameData, TitleId};
use serde::Serialize;
use sim_campaign::CampaignState;
use sim_campaign::feudal;

/// One objective title of a sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartObjective {
    pub title: String,
    pub description: String,
}

/// The 1337 starting situation of one playable faction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StartSheet {
    pub id: String,
    pub name: String,
    /// Display name of the ruler (empty when none).
    pub ruler: String,
    /// Id of the ruler's character (portrait file `portraits/<id>.png`), empty when none.
    pub ruler_id: String,
    /// Names of the held titles.
    pub titles: Vec<String>,
    /// `kingdom`, `duchy`, `county` or empty (landless).
    pub rank: String,
    /// Name of the suzerain, empty for a sovereign.
    pub liege_name: String,
    /// Id of the kingdom title (the sovereign's primary title), empty when landless.
    pub kingdom: String,
    pub kingdom_name: String,
    pub objectives: Vec<StartObjective>,
    /// Ids of the provinces owned in 1337.
    pub provinces: Vec<String>,
    pub treasury: i64,
    pub projected_income: i64,
    pub armies_count: usize,
}

fn title_name(data: &GameData, id: &TitleId) -> String {
    data.titles
        .get(id)
        .map_or_else(|| id.as_str().to_owned(), |t| t.name.display.clone())
}

/// Sheets of every playable faction (empty when the 1337 campaign cannot be built).
pub fn feudal_start_sheets(data: &GameData) -> Vec<StartSheet> {
    let Some(first) = data.factions.keys().next() else {
        return Vec::new();
    };
    let Ok(mut state) = CampaignState::new_1337(data, first.clone(), 0) else {
        return Vec::new();
    };
    feudal::sync_suzerains(&mut state, data);
    let mut playable: Vec<&FactionId> = data
        .factions
        .iter()
        .filter(|(_, f)| f.playable)
        .map(|(id, _)| id)
        .collect();
    playable.sort_by_cached_key(|id| {
        (
            feudal::liege_of(&state, data, id).is_some(),
            data.faction_name(id),
        )
    });
    playable
        .into_iter()
        .map(|id| sheet_of(data, &state, id))
        .collect()
}

fn sheet_of(data: &GameData, state: &CampaignState, id: &FactionId) -> StartSheet {
    let sheet = feudal::faction_sheet(state, data, id);
    let ruler_id = state
        .factions
        .get(id)
        .and_then(|f| f.ruler.as_ref())
        .map(|r| r.as_str().to_owned())
        .unwrap_or_default();
    let ruler = state
        .factions
        .get(id)
        .and_then(|f| f.ruler.as_ref())
        .map_or_else(String::new, |r| state.character_name(data, r));
    let sovereign = feudal::liege_chain(state, data, id)
        .last()
        .cloned()
        .unwrap_or_else(|| id.clone());
    let kingdom = state.feudal.primary.get(&sovereign).cloned();
    let summary = state.faction_summary(id);
    let economy = state.faction_economy(data, id);
    StartSheet {
        id: id.as_str().to_owned(),
        name: data.faction_name(id),
        ruler,
        ruler_id,
        titles: sheet.titles.iter().map(|t| title_name(data, t)).collect(),
        rank: sheet.rank.map_or("", |r| r.key()).to_owned(),
        liege_name: sheet
            .liege
            .as_ref()
            .map_or_else(String::new, |f| data.faction_name(f)),
        kingdom: kingdom
            .as_ref()
            .map_or_else(String::new, |t| t.as_str().to_owned()),
        kingdom_name: kingdom
            .as_ref()
            .map_or_else(String::new, |t| title_name(data, t)),
        objectives: sheet
            .objectives
            .iter()
            .map(|o| StartObjective {
                title: o.title.clone(),
                description: o.description.clone(),
            })
            .collect(),
        provinces: data
            .provinces
            .values()
            .filter(|p| &p.owner == id)
            .map(|p| p.id.as_str().to_owned())
            .collect(),
        treasury: economy.as_ref().map_or(0, |e| e.treasury),
        projected_income: economy.as_ref().map_or(0, |e| e.projected_income),
        armies_count: summary.map_or(0, |s| s.armies_count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_data_gives_no_sheets() {
        assert!(feudal_start_sheets(&GameData::default()).is_empty());
    }
}
