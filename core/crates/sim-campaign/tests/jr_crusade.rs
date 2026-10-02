//! Lot JR1 (ADR 0165): the crusader faction and its fervour on the real
//! data of 1337 (`data/rules/crusade.json`, the faction of lot JR2).

use std::path::PathBuf;

use data_model::{CrusadeRules, FactionId, GameData};
use sim_campaign::crusade::{self, crusade_view};
use sim_campaign::{CampaignState, EventKind, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn rules(data: &GameData) -> &CrusadeRules {
    data.crusade_rules
        .as_ref()
        .expect("data/rules/crusade.json is loaded")
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

/// A campaign played by the crusader faction of the rules.
fn crusade(data: &GameData, seed: u64) -> (CampaignState, FactionId) {
    let faction = rules(data).faction.clone();
    let state = CampaignState::new_1337(data, faction.clone(), seed).expect("1337 start");
    (state, faction)
}

#[test]
fn the_rules_name_existing_things() {
    let data = data();
    let rules = rules(&data);
    assert!(data.factions.contains_key(&rules.faction));
    assert!(data
        .settlements
        .get(&rules.base_settlement)
        .is_some_and(|s| s.port));
    assert!(data.provinces.contains_key(&rules.target_province));
    for province in rules.holy_land.iter().chain(&rules.coastal_holy_land) {
        assert!(data.provinces.contains_key(province), "{province}");
    }
    for unit in rules
        .starting_army
        .iter()
        .chain(rules.passage.unit_table.iter().map(|e| &e.unit))
    {
        assert!(data.unit_types.contains_key(unit), "{unit}");
    }
}

#[test]
fn the_starting_army_stands_in_the_base_settlement() {
    let data = data();
    let (state, faction) = crusade(&data, 1);
    let rules = rules(&data);
    let base = &state.settlements[&rules.base_settlement];
    assert_eq!(base.owner, faction);
    assert_eq!(base.controller, faction);
    let armies: Vec<_> = state
        .armies
        .values()
        .filter(|a| a.faction == faction)
        .collect();
    assert_eq!(armies.len(), 1);
    let army = armies[0];
    assert!(
        army.is_at(&rules.base_settlement),
        "not in the city of the capital: {:?}",
        army.position
    );
    let types: Vec<_> = army.units.iter().map(|u| u.unit_type.clone()).collect();
    assert_eq!(types, rules.starting_army);
    assert_eq!(
        army.general, state.factions[&faction].ruler,
        "led by its ruler"
    );
    // The crusade is open, at the starting fervour; the city of the capital
    // province is not the faction's.
    let crusade = state.crusade.as_ref().expect("crusade opened");
    assert_eq!(crusade.fervor, rules.fervor.start);
    let capital = &state.factions[&faction].capital;
    assert_ne!(state.province_controller(capital), Some(&faction));
    // Only the crusaders see the panel.
    assert!(crusade_view(&state, &data, &faction).is_some());
    assert!(crusade_view(&state, &data, &fac("fac_france")).is_none());
}

#[test]
fn preaching_lands_a_contingent_on_time() {
    let data = data();
    let (mut state, faction) = crusade(&data, 2);
    let rules = rules(&data);
    let view = crusade_view(&state, &data, &faction).expect("view");
    assert!(view.passage_available, "{}", view.passage_blocker);
    let expected_units = view.passage_units;
    let treasury = state.factions[&faction].treasury;
    state
        .submit_order(&data, Order::PreachPassage)
        .expect("preached");
    assert_eq!(
        state.factions[&faction].treasury,
        treasury - view.passage_cost
    );
    let after = crusade_view(&state, &data, &faction).expect("view");
    assert_eq!(
        i32::from(after.fervor),
        i32::from(view.fervor) + rules.fervor.preach
    );
    assert!(!after.passage_available);
    assert!(!after.passage_blocker.is_empty());
    assert_eq!(after.pending.len(), 1);
    assert_eq!(after.pending[0].units, expected_units);
    assert_eq!(after.pending[0].turns_left, rules.passage.delay_turns);
    assert_eq!(after.pending[0].port, rules.base_settlement);
    // A second call is refused, in French.
    let refused = state
        .submit_order(&data, Order::PreachPassage)
        .expect_err("on cooldown");
    assert!(refused.to_string().contains("passage"), "{refused}");

    let garrison = |state: &CampaignState| state.settlements[&rules.base_settlement].garrison.len();
    let before = garrison(&state);
    let mut landed_after = None;
    let mut events = Vec::new();
    for turn in 1..=rules.passage.delay_turns {
        events = state.end_turn(&data);
        if garrison(&state) > before && landed_after.is_none() {
            landed_after = Some(turn);
        }
    }
    assert_eq!(landed_after, Some(rules.passage.delay_turns));
    assert_eq!(garrison(&state), before + expected_units as usize);
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Crusade && e.faction.as_ref() == Some(&faction)));
    let view = crusade_view(&state, &data, &faction).expect("view");
    assert!(view.pending.is_empty());
    // The alms are part of the season's income.
    assert!(view.alms_last_turn > 0);
    assert!(state.factions[&faction].income_last_turn >= view.alms_last_turn);
}

#[test]
fn an_older_save_without_crusade_loads() {
    let data = data();
    let (state, faction) = crusade(&data, 3);
    let mut json: serde_json::Value = serde_json::from_str(&state.save_json()).unwrap();
    assert!(json.as_object_mut().unwrap().remove("crusade").is_some());
    let mut old = CampaignState::load_json(&json.to_string()).expect("older save loads");
    assert!(old.crusade.is_none());
    assert_eq!(old.state_version, state.state_version);
    assert!(crusade_view(&old, &data, &faction).is_none());
    assert!(old.submit_order(&data, Order::PreachPassage).is_err());
    // The mechanic is inert, the game goes on.
    for _ in 0..3 {
        old.end_turn(&data);
    }
    assert!(old.crusade.is_none());
    // A current save keeps the crusade.
    let reloaded = CampaignState::load_json(&state.save_json()).expect("save loads");
    assert_eq!(reloaded.crusade, state.crusade);
}

#[test]
fn forty_ai_turns_without_panic_and_the_crusaders_live() {
    let data = data();
    let faction = rules(&data).faction.clone();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 4).expect("1337 start");
    state.interactive_battles = false;
    let mut preached = false;
    for _ in 0..40 {
        let events = state.end_turn(&data);
        preached |= events.iter().any(|e| e.kind == EventKind::Crusade);
        let crusade = state.crusade.as_ref().expect("crusade kept");
        assert!(crusade.fervor <= 100);
    }
    assert!(
        state.factions[&faction].alive,
        "the crusaders are still there"
    );
    assert!(preached, "the AI preached the passage");
    assert!(crusade_view(&state, &data, &faction).is_some());
}

#[test]
fn taking_the_target_province_delivers_it() {
    let data = data();
    let (mut state, faction) = crusade(&data, 5);
    let rules = rules(&data);
    let city = state
        .province_city_id(&rules.target_province)
        .expect("target city")
        .clone();
    state.crusade.as_mut().unwrap().fervor = 5;
    state.settlements.get_mut(&city).unwrap().controller = faction.clone();
    let mut events = Vec::new();
    crusade::on_settlement_taken(&mut state, &data, &faction, &city, &mut events);
    let delivered: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::Crusade)
        .collect();
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].province.as_ref(), Some(&rules.target_province));
    assert!(
        delivered[0].text_fr.contains("délivrée"),
        "{}",
        delivered[0].text_fr
    );
    let view = crusade_view(&state, &data, &faction).expect("view");
    assert!(view.target_taken);
    assert_eq!(view.floor, rules.fervor.target_floor);
    assert!(view.fervor >= rules.fervor.target_floor);
    assert_eq!(state.factions[&faction].capital, rules.target_province);
    // The floor holds through the ends of turn while the city is held.
    state.crusade.as_mut().unwrap().fervor = rules.fervor.target_floor;
    let mut journal = Vec::new();
    state.resolve_end_of_turn(&data, &mut journal);
    let crusade = state.crusade.as_ref().unwrap();
    if state.province_controller(&rules.target_province) == Some(&faction) {
        assert!(crusade.fervor >= rules.fervor.target_floor);
        assert!(crusade.target_taken);
    }
}
