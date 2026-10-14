//! Faction sheet: the first view that proves the `core` -> `game-api` -> client pattern.

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_campaign::CampaignState;

use crate::common::{faction_refs, FactionRef};

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

/// A title held by a faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TitleView {
    pub id: String,
    pub name: String,
    /// `kingdom`, `duchy`, `county`.
    pub rank: String,
}

/// A victory objective of the faction sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectiveView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub met: bool,
}

/// One other faction seen from the sheet's faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelationView {
    pub id: String,
    pub name: String,
    /// `war`, `truce`, `peace`, `alliance`, ... (relation key).
    pub status: String,
    pub attitude: i32,
    pub attitude_band: String,
    pub truce_turns_left: u32,
    pub war_score: i32,
    pub embargo_by_us: bool,
    pub embargo_on_us: bool,
    pub trade_agreement: bool,
}

/// Counts of relations by status plus the full list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelationsSummary {
    pub wars: usize,
    pub truces: usize,
    pub alliances: usize,
    pub vassals: usize,
    pub entries: Vec<RelationView>,
}

/// Faction sheet: ruler, feudal position, title holdings, relations.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FactionSheet {
    pub id: String,
    pub name: String,
    pub ruler: String,
    pub primary_title: Option<TitleView>,
    pub rank: String,
    /// Titles held, primary first.
    pub titles: Vec<TitleView>,
    pub liege: Option<FactionRef>,
    pub liege_chain: Vec<FactionRef>,
    pub sovereign: FactionRef,
    pub direct_vassals: Vec<FactionRef>,
    pub title_vassals: Vec<FactionRef>,
    /// Loyalty towards the liege, `None` when sovereign.
    pub loyalty: Option<u8>,
    pub status: String,
    pub status_label: String,
    pub objectives: Vec<ObjectiveView>,
    pub alive: bool,
    pub provinces_count: usize,
    pub treasury: i64,
    pub prestige: i32,
    pub relations: RelationsSummary,
}

fn title_view(data: &GameData, id: &data_model::TitleId) -> TitleView {
    let definition = data.titles.get(id);
    TitleView {
        id: id.as_str().to_owned(),
        name: definition.map_or_else(|| id.as_str().to_owned(), |t| t.name.display.clone()),
        rank: definition.map_or("", |t| t.rank.key()).to_owned(),
    }
}

/// Sheet of `faction`; `None` when unknown to the campaign.
pub fn faction_sheet(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> Option<FactionSheet> {
    let live = state.factions.get(faction)?;
    let summary = state.faction_summary(faction)?;
    let sheet = sim_campaign::feudal::faction_sheet(state, data, faction);
    let entries: Vec<RelationView> = state
        .diplomacy_view(data, faction)
        .iter()
        .map(|entry| RelationView {
            id: entry.faction.as_str().to_owned(),
            name: data.faction_name(&entry.faction),
            status: entry.relation.key().to_owned(),
            attitude: entry.attitude,
            attitude_band: entry.attitude_band.clone(),
            truce_turns_left: entry.truce_turns_left,
            war_score: entry.war_score,
            embargo_by_us: entry.embargo_by_us,
            embargo_on_us: entry.embargo_on_us,
            trade_agreement: entry.trade_agreement,
        })
        .collect();
    let count = |key: &str| entries.iter().filter(|e| e.status == key).count();
    let relations = RelationsSummary {
        wars: count("war"),
        truces: count("truce"),
        alliances: count("alliance"),
        vassals: sheet.direct_vassals.len(),
        entries,
    };
    Some(FactionSheet {
        id: faction.as_str().to_owned(),
        name: data.faction_name(faction),
        ruler: live
            .ruler
            .as_ref()
            .map_or_else(String::new, |r| state.character_name(data, r)),
        primary_title: sheet.primary.as_ref().map(|t| title_view(data, t)),
        rank: sheet.rank.map_or("", |r| r.key()).to_owned(),
        titles: sheet.titles.iter().map(|t| title_view(data, t)).collect(),
        liege: sheet
            .liege
            .as_ref()
            .map(|f| faction_refs(data, std::slice::from_ref(f)).remove(0)),
        liege_chain: faction_refs(data, &sheet.liege_chain),
        sovereign: faction_refs(data, std::slice::from_ref(&sheet.sovereign)).remove(0),
        direct_vassals: faction_refs(data, &sheet.direct_vassals),
        title_vassals: faction_refs(data, &sheet.title_vassals),
        loyalty: sheet.loyalty,
        status: sheet.status.map_or("", |s| s.key()).to_owned(),
        status_label: sheet.status.map_or("", |s| s.label_fr()).to_owned(),
        objectives: sheet
            .objectives
            .iter()
            .map(|o| ObjectiveView {
                id: o.id.clone(),
                title: o.title.clone(),
                description: o.description.clone(),
                met: o.met,
            })
            .collect(),
        alive: live.alive,
        provinces_count: summary.provinces_count,
        treasury: summary.treasury,
        prestige: summary.prestige,
        relations,
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
