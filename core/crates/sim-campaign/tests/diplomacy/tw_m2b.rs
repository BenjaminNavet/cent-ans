//! Lot TW m2b: wider excommunication and interdict, throne claim by marriage,
//! progressive conversion of provinces, papal crusade (ADR 0326, 0327).

use data_model::test_support::{fac, game_data, prov};
use data_model::{CharacterId, ClaimKind, GameData, ReligionId, Sex};
use sim_campaign::religion::{is_excommunicated, is_interdicted};
use sim_campaign::test_support::{idle, start, start_quiet};
use sim_campaign::{CampaignState, EventKind, Order};

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

fn war_with_truce(state: &mut CampaignState, a: &str, b: &str) {
    let until = state.turn() + 10;
    state
        .factions
        .get_mut(&fac(a))
        .unwrap()
        .truces
        .insert(fac(b), until);
    state
        .factions
        .get_mut(&fac(b))
        .unwrap()
        .truces
        .insert(fac(a), until);
}

// ----- top4: excommunication and interdict -----------------------------------

#[test]
fn breaking_a_truce_with_a_catholic_excommunicates() {
    let data = game_data();
    let mut state = start(data, "fac_aragon", 14);
    war_with_truce(&mut state, "fac_aragon", "fac_navarre");
    state
        .factions
        .get_mut(&fac("fac_aragon"))
        .unwrap()
        .papal_favor = 70;
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_navarre"),
            },
        )
        .unwrap();
    assert!(is_excommunicated(&state, &fac("fac_aragon")));
}

#[test]
fn a_clean_war_does_not_excommunicate_a_pious_prince() {
    let data = game_data();
    let mut state = start(data, "fac_aragon", 14);
    state
        .factions
        .get_mut(&fac("fac_aragon"))
        .unwrap()
        .papal_favor = 70;
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_navarre"),
            },
        )
        .unwrap();
    assert!(!is_excommunicated(&state, &fac("fac_aragon")));
}

#[test]
fn executing_a_christian_captive_costs_favour_and_may_excommunicate() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_england", 1);
    let england = fac("fac_england");
    sim_campaign::chronicle::capture_character(
        &mut state,
        data,
        &chr("chr_jean_de_normandie"),
        &england,
        &mut Vec::new(),
    );
    let rules = data.religion_rules.as_ref().unwrap();
    state.factions.get_mut(&england).unwrap().papal_favor =
        rules.excommunication.captive_favor_below;
    state
        .submit_order(
            data,
            Order::ExecuteCaptive {
                character: chr("chr_jean_de_normandie"),
            },
        )
        .unwrap();
    assert!(is_excommunicated(&state, &england));
    assert!(state.factions[&england].papal_favor < rules.excommunication.captive_favor_below);
}

#[test]
fn deep_disfavour_brings_the_interdict_then_a_donation_lifts_it() {
    let data = game_data();
    let mut state = start(data, "fac_aragon", 14);
    let aragon = fac("fac_aragon");
    state.factions.get_mut(&aragon).unwrap().papal_favor = 5;
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_navarre"),
            },
        )
        .unwrap();
    assert!(is_excommunicated(&state, &aragon));
    assert!(is_interdicted(&state, &aragon));
    let rules = data.religion_rules.as_ref().unwrap();
    assert_eq!(
        state.interdict_unrest(data, &aragon),
        rules.interdict.unrest
    );
    assert_eq!(state.interdict_unrest(data, &fac("fac_navarre")), 0.0);
    // Lasts `turns`, no longer.
    let until = state.factions[&aragon].interdict_until.unwrap();
    assert_eq!(until, state.turn() + rules.interdict.turns);
    // A small gift does not lift it, a great one does.
    state.factions.get_mut(&aragon).unwrap().treasury = rules.interdict.lift_donation * 2;
    state
        .donate_to_church(data, &aragon, rules.interdict.lift_donation - 1)
        .unwrap();
    assert!(is_interdicted(&state, &aragon));
    state
        .donate_to_church(data, &aragon, rules.interdict.lift_donation)
        .unwrap();
    assert!(!is_interdicted(&state, &aragon));
}

#[test]
fn the_interdict_ends_with_the_excommunication() {
    let data = game_data();
    let mut state = start(data, "fac_aragon", 14);
    let aragon = fac("fac_aragon");
    state.factions.get_mut(&aragon).unwrap().papal_favor = 5;
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_navarre"),
            },
        )
        .unwrap();
    assert!(is_interdicted(&state, &aragon));
    // The Pope forgives: favour above the lifting level.
    state.factions.get_mut(&aragon).unwrap().papal_favor = 100;
    state.end_turn_with(data, idle);
    assert!(!is_excommunicated(&state, &aragon));
    assert!(!is_interdicted(&state, &aragon));
}

// ----- top5: throne claim by marriage -----------------------------------------

fn child_of(
    state: &mut CampaignState,
    data: &GameData,
    father_faction: &str,
    mother_faction: &str,
) -> data_model::CharacterId {
    use sim_campaign::CharacterState;
    let ruler_house = |state: &CampaignState, f: &str| {
        let ruler = state.factions[&fac(f)].ruler.clone().unwrap();
        state.characters[&ruler].house.clone()
    };
    let (father_house, mother_house) = (
        ruler_house(state, father_faction),
        ruler_house(state, mother_faction),
    );
    let skills =
        state.characters[&state.factions[&fac(father_faction)].ruler.clone().unwrap()].skills;
    let mk = |faction: &str, sex, house: String, skills| {
        CharacterState::new(fac(faction), 1310, sex, house, skills)
    };
    let father = chr("chr_test_father");
    let mother = chr("chr_test_mother");
    let child = chr("chr_test_child");
    state.characters.insert(
        father.clone(),
        mk(father_faction, Sex::Male, father_house.clone(), skills),
    );
    state.characters.insert(
        mother.clone(),
        mk(mother_faction, Sex::Female, mother_house, skills),
    );
    state.characters.insert(
        child.clone(),
        CharacterState {
            father: Some(father),
            mother: Some(mother),
            ..CharacterState::new(fac(father_faction), 1337, Sex::Male, father_house, skills)
        },
    );
    let _ = data;
    child
}

#[test]
fn a_child_of_two_crowns_claims_the_throne_of_its_mother() {
    let data = game_data();
    let mut state = start(data, "fac_france", 3);
    let (castile, aragon) = (fac("fac_castile"), fac("fac_aragon"));
    assert!(state.casus_belli(data, &castile, &aragon).is_none());
    let child = child_of(&mut state, data, "fac_castile", "fac_aragon");
    let mut events = Vec::new();
    sim_campaign::dynasty::claim_by_marriage(&mut state, data, &child, &mut events);
    let claim = state.factions[&castile]
        .claims
        .iter()
        .find(|c| c.kind == ClaimKind::Throne && c.faction.as_ref() == Some(&aragon))
        .expect("a claim on Aragon");
    assert!(claim.text_fr.contains("prétention"));
    assert!(state
        .casus_belli(data, &castile, &aragon)
        .unwrap()
        .contains("trône"));
    assert!(sim_campaign::diplomacy::claim_stakes(&state, &castile, &aragon).throne);
    assert!(events.iter().any(|e| e.kind == EventKind::Diplomacy));
    // Aragon gains nothing, and a second birth adds no duplicate.
    assert!(state.factions[&aragon]
        .claims
        .iter()
        .all(|c| c.faction.as_ref() != Some(&castile)));
    sim_campaign::dynasty::claim_by_marriage(&mut state, data, &child, &mut events);
    assert_eq!(
        state.factions[&castile]
            .claims
            .iter()
            .filter(|c| c.faction.as_ref() == Some(&aragon))
            .count(),
        1
    );
}

#[test]
fn a_child_of_a_mother_outside_the_ruling_house_claims_nothing() {
    let data = game_data();
    let mut state = start(data, "fac_france", 3);
    let child = child_of(&mut state, data, "fac_castile", "fac_aragon");
    let mother = state.characters[&child].mother.clone().unwrap();
    state.characters.get_mut(&mother).unwrap().house = "house_unknown".to_owned();
    sim_campaign::dynasty::claim_by_marriage(&mut state, data, &child, &mut Vec::new());
    assert!(state.factions[&fac("fac_castile")]
        .claims
        .iter()
        .all(|c| c.faction.as_ref() != Some(&fac("fac_aragon"))));
}

// ----- top6: conversion --------------------------------------------------------

#[test]
fn a_province_of_another_faith_converts_in_a_few_years() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let province = prov("prov_ile_de_france");
    let own = state.province_faith(data, &province).unwrap();
    let islam = ReligionId::new("rel_islam").unwrap();
    state.provinces.get_mut(&province).unwrap().faith_override = Some(islam.clone());
    assert_eq!(state.province_faith(data, &province), Some(islam.clone()));
    assert!(state.conversion_speed(data, &province) > 0);
    let mut seasons = 0;
    while state.provinces[&province].faith_override.is_some() && seasons < 120 {
        state.end_turn_with(data, idle);
        seasons += 1;
    }
    assert_eq!(state.province_faith(data, &province), Some(own));
    assert!(seasons > 4, "not at once: {seasons}");
    assert!(seasons < 120, "converted: {seasons}");
    assert_eq!(state.provinces[&province].conversion_progress, 0);
}

#[test]
fn a_province_of_its_lords_faith_never_converts() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let province = prov("prov_ile_de_france");
    assert_eq!(state.conversion_speed(data, &province), 0);
    for _ in 0..8 {
        state.end_turn_with(data, idle);
    }
    let p = &state.provinces[&province];
    assert!(p.faith_override.is_none() && p.conversion_progress == 0);
}

#[test]
fn unrest_stalls_the_conversion() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let province = prov("prov_ile_de_france");
    state.provinces.get_mut(&province).unwrap().faith_override =
        Some(ReligionId::new("rel_islam").unwrap());
    let calm = state.conversion_speed(data, &province);
    state.provinces.get_mut(&province).unwrap().unrest = 100;
    assert!(calm > 0 && state.conversion_speed(data, &province) == 0);
}

// ----- top7: papal crusade -------------------------------------------------------

fn crusade_state(seed: u64) -> (CampaignState, &'static GameData) {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", seed);
    let rules = data.religion_rules.as_ref().unwrap();
    state.turn = rules.papal_crusade.first_call_turn;
    (state, data)
}

#[test]
fn the_pope_calls_a_crusade_once_per_window() {
    let (mut state, data) = crusade_state(7);
    let rules = &data.religion_rules.as_ref().unwrap().papal_crusade;
    let events = state.end_turn_with(data, idle);
    let call = state.papal_crusade.clone().expect("a call");
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Crusade && e.text_fr.contains("croisade")));
    assert_eq!(call.expires_turn, call.called_turn + rules.window_turns);
    // No second call while one runs.
    let target = call.target.clone();
    state.end_turn_with(data, idle);
    assert_eq!(
        state.papal_crusade.as_ref().map(|c| &c.target),
        Some(&target)
    );
    assert_eq!(
        state.papal_crusade.as_ref().unwrap().called_turn,
        call.called_turn
    );
}

#[test]
fn the_call_lapses_then_waits_for_the_interval() {
    let (mut state, data) = crusade_state(7);
    let rules = &data.religion_rules.as_ref().unwrap().papal_crusade;
    state.end_turn_with(data, idle);
    let expires = state.papal_crusade.as_ref().unwrap().expires_turn;
    state.turn = expires;
    let events = state.end_turn_with(data, idle);
    assert!(state.papal_crusade.is_none());
    assert!(events.iter().any(|e| e.text_fr.contains("sans suite")));
    let end = state.last_papal_call_end.unwrap();
    state.turn = end + rules.interval_turns - 2;
    state.end_turn_with(data, idle);
    assert!(state.papal_crusade.is_none(), "too soon");
}

#[test]
fn joining_the_crusade_earns_favour_and_the_first_taker_is_rewarded() {
    let (mut state, data) = crusade_state(7);
    let rules = &data.religion_rules.as_ref().unwrap().papal_crusade;
    state.end_turn_with(data, idle);
    let call = state.papal_crusade.clone().expect("a call");
    let holder = state.province_controller(&call.target).cloned().unwrap();
    // France declares war on the holder of the target: favour and prestige.
    let france = fac("fac_france");
    state.factions.get_mut(&france).unwrap().papal_favor = 50;
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .truces
        .remove(&holder);
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .allies
        .remove(&holder);
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: holder.clone(),
            },
        )
        .unwrap();
    assert!(state
        .papal_crusade
        .as_ref()
        .unwrap()
        .participants
        .contains(&france));
    assert_eq!(state.factions[&france].papal_favor, 50 + rules.join_favor);
    // France takes the target's city: the reward comes at the next season.
    let city = state.province_city_id(&call.target).unwrap().clone();
    for id in state.provinces[&call.target].settlements.clone() {
        state.settlements.get_mut(&id).unwrap().controller = france.clone();
    }
    let _ = city;
    let favor = state.factions[&france].papal_favor;
    let events = state.end_turn_with(data, idle);
    assert!(state.papal_crusade.is_none());
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Crusade && e.text_fr.contains("victorieuse")));
    assert!(
        state.factions[&france].papal_favor
            >= favor
                .saturating_add(rules.reward_favor)
                .min(100)
                .saturating_sub(4)
    );
}

#[test]
fn the_call_is_saved_and_old_saves_load() {
    let (mut state, data) = crusade_state(7);
    state.end_turn_with(data, idle);
    let json = serde_json::to_string(&state).unwrap();
    let back: CampaignState = serde_json::from_str(&json).unwrap();
    assert_eq!(back.papal_crusade, state.papal_crusade);
    // An old save has none of the new fields.
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value.as_object_mut().unwrap().remove("papal_crusade");
    value.as_object_mut().unwrap().remove("last_papal_call_end");
    let old: CampaignState = serde_json::from_value(value).unwrap();
    assert!(old.papal_crusade.is_none());
}
