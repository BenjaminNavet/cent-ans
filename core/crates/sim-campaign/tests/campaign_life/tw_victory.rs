//! TW misc-camp (ADR 0332): short and long campaigns, generic objectives,
//! new objective conditions.

use data_model::test_support::{fac, game_data};
use data_model::{Objective, ObjectiveCondition, ObjectiveScope, ProvinceId, TitleId};
use sim_campaign::test_support::idle;
use sim_campaign::victory::{OutcomeKind, VictoryLength};
use sim_campaign::CampaignState;

#[test]
fn short_campaign_keeps_only_the_always_objectives() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let long = state.objectives(data, &fac("fac_france"));
    state.set_victory_length(VictoryLength::Short);
    let short = state.objectives(data, &fac("fac_france"));
    assert!(!short.is_empty() && short.len() < long.len(), "{short:?}");
    assert!(short.iter().all(|o| o.id != "obj_fr_expel"));
    assert!(state.victory_hold_turns(data, &fac("fac_france")) <= 4);
}

#[test]
fn factions_without_a_block_get_the_generic_objectives() {
    let data = game_data();
    let bourbon = fac("fac_bourbon");
    assert!(data.factions[&bourbon].victory.is_none());
    let mut state = CampaignState::new_1337(data, bourbon.clone(), 1).unwrap();
    let long = state.objectives(data, &bourbon);
    assert_eq!(
        long.len(),
        data.feudal_rules.victory.generic_objectives.len()
    );
    assert!(long.iter().all(|o| !o.done), "{long:?}");
    state.set_victory_length(VictoryLength::Short);
    let short = state.objectives(data, &bourbon);
    assert!(short.len() < long.len());
}

#[test]
fn every_new_faction_block_is_consistent() {
    let data = game_data();
    for id in [
        "fac_moscow",
        "fac_ottoman",
        "fac_hungary",
        "fac_poland",
        "fac_lithuania",
        "fac_teutonic",
        "fac_byzantium",
        "fac_mamluks",
        "fac_sicily",
    ] {
        let state = CampaignState::new_1337(data, fac(id), 1).unwrap();
        let long = state.objectives(data, &fac(id));
        assert!(long.len() >= 4, "{id}");
        assert!(!long.iter().all(|o| o.done), "{id} wins at the start");
        let v = data.factions[&fac(id)].victory.as_ref().unwrap();
        assert!(v
            .objectives
            .iter()
            .any(|o| o.scope == ObjectiveScope::LongOnly));
        assert!(v.short_end_year.is_some());
    }
}

/// Replaces France's objectives by `conditions` and returns their statuses.
fn statuses(
    state: &CampaignState,
    conditions: Vec<ObjectiveCondition>,
) -> Vec<sim_campaign::victory::ObjectiveStatus> {
    let mut data = game_data().clone();
    let france = fac("fac_france");
    let victory = data
        .factions
        .get_mut(&france)
        .unwrap()
        .victory
        .as_mut()
        .unwrap();
    victory.objectives = conditions
        .into_iter()
        .enumerate()
        .map(|(i, condition)| Objective {
            id: format!("obj_t{i}"),
            title: "t".into(),
            description: "d".into(),
            condition,
            scope: ObjectiveScope::Always,
        })
        .collect();
    state.objectives(&data, &france)
}

#[test]
fn new_conditions_report_their_progress() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let held = state.controlled_provinces(&france).count() as u32;
    let ruler = state.factions[&france].ruler.clone().unwrap();
    state.characters.get_mut(&ruler).unwrap().prestige = 70;
    state.factions.get_mut(&france).unwrap().treasury = 9_000;
    state.victory_start_provinces = held - 2;
    let title = TitleId::new("tit_france").unwrap();
    let missing = TitleId::new("tit_england").unwrap();
    let list = statuses(
        &state,
        vec![
            ObjectiveCondition::ProvinceCount { count: held },
            ObjectiveCondition::ProvinceCount { count: held + 1 },
            ObjectiveCondition::ProvinceGrowth { extra: 2 },
            ObjectiveCondition::ProvinceGrowth { extra: 3 },
            ObjectiveCondition::Prestige { min: 70 },
            ObjectiveCondition::Prestige { min: 71 },
            ObjectiveCondition::Treasury { min: 9_000 },
            ObjectiveCondition::Treasury { min: 9_001 },
            ObjectiveCondition::HoldTitle { title },
            ObjectiveCondition::HoldTitle { title: missing },
        ],
    );
    let done: Vec<bool> = list.iter().map(|o| o.done).collect();
    assert_eq!(
        done,
        vec![true, false, true, false, true, false, true, false, true, false]
    );
    assert!(list[4].progress.contains("70/70"));
    let _ = ProvinceId::new("prov_flandre");
}

#[test]
fn a_short_campaign_ends_at_its_own_year() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    state.set_victory_length(VictoryLength::Short);
    state.year = data.feudal_rules.victory.short_end_year;
    state.end_turn_with(data, idle);
    assert!(state.outcome.is_none(), "{:?}", state.outcome);
    state.year = data.feudal_rules.victory.short_end_year + 1;
    state.end_turn_with(data, idle);
    let outcome = state.outcome.clone().expect("the short campaign is over");
    assert_eq!(outcome.kind, OutcomeKind::Ended);
}
