//! Lot C7: the general's retinue (acquisition, effects, cap, inheritance,
//! transfer), the year of death, and old saves without the new fields.

use data_model::{AcquisitionTrigger, BuildingId, CharacterId, CompanionId, GameData};
use sim_campaign::state::ArmyId;
use sim_campaign::{characters, retinue, skills, CampaignState, Order, OrderError};

use data_model::test_support::{fac, game_data};

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

fn ret(id: &str) -> CompanionId {
    CompanionId::new(id).unwrap()
}

fn start(data: &GameData, faction: &str, seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac(faction), seed).expect("1337 start");
    state.chronicle.disabled = true;
    state
}

/// Companions `id` gains over `turns` victories, one roll per turn.
fn victories(data: &GameData, seed: u64, id: &str, turns: u32) -> Vec<CompanionId> {
    let mut state = start(data, "fac_england", seed);
    let id = chr(id);
    let mut events = Vec::new();
    for turn in 0..turns {
        state.turn = turn;
        retinue::try_acquire(
            &mut state,
            data,
            &id,
            AcquisitionTrigger::BattleWon,
            &[],
            &mut events,
        );
    }
    state.characters[&id].retinue.clone()
}

#[test]
fn the_catalogue_loads() {
    let data = game_data();
    let catalogue = data.retinue.as_ref().expect("data/retinue.json");
    assert_eq!(catalogue.max_per_character, 8);
    assert!(catalogue.companions.len() >= 12);
    assert!(catalogue
        .companion(&ret("ret_barbier_chirurgien"))
        .is_some());
    assert_eq!(retinue::max_per_character(data), 8);
}

#[test]
fn acquisition_is_deterministic_per_seed_and_capped() {
    let data = game_data();
    let first = victories(data, 7, "chr_edward_iii", 60);
    let again = victories(data, 7, "chr_edward_iii", 60);
    assert_eq!(first, again, "same seed, same retinue");
    assert!(!first.is_empty(), "sixty victories bring companions");
    assert!(first.len() <= 8, "cap of 8: {first:?}");
    let other = victories(data, 8, "chr_edward_iii", 60);
    let short_a = victories(data, 7, "chr_edward_iii", 6);
    let short_b = victories(data, 8, "chr_edward_iii", 6);
    assert!(
        other != first || short_a != short_b,
        "another seed gives another story"
    );
    // No duplicates.
    let mut sorted = first.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), first.len());
}

#[test]
fn acquisition_does_not_touch_the_main_random_stream() {
    let data = game_data();
    let mut state = start(data, "fac_england", 3);
    let before = state.rng.clone();
    let mut events = Vec::new();
    for turn in 0..20 {
        state.turn = turn;
        retinue::try_acquire(
            &mut state,
            data,
            &chr("chr_edward_iii"),
            AcquisitionTrigger::BattleFought,
            &[],
            &mut events,
        );
    }
    assert_eq!(state.rng, before);
}

#[test]
fn faction_conditions_and_buildings_are_honoured() {
    let data = game_data();
    // The vintenar only serves England.
    let mut state = start(data, "fac_france", 11);
    let philippe = chr("chr_philippe_vi");
    assert!(!retinue::can_gain(
        &state,
        data,
        &philippe,
        &ret("ret_vintenier")
    ));
    assert!(retinue::can_gain(
        &state,
        data,
        &chr("chr_edward_iii"),
        &ret("ret_vintenier")
    ));
    // A season without the right building brings no companion; with an
    // abbey, a confessor comes sooner or later.
    let mut events = Vec::new();
    for turn in 0..40 {
        state.turn = turn;
        retinue::try_acquire(
            &mut state,
            data,
            &philippe,
            AcquisitionTrigger::SeasonInSettlement,
            &[],
            &mut events,
        );
    }
    assert!(state.characters[&philippe].retinue.is_empty());
    let abbey = [BuildingId::new("bld_abbey").unwrap()];
    for turn in 0..80 {
        state.turn = turn;
        retinue::try_acquire(
            &mut state,
            data,
            &philippe,
            AcquisitionTrigger::SeasonInSettlement,
            &abbey,
            &mut events,
        );
    }
    assert_eq!(
        state.characters[&philippe].retinue,
        vec![ret("ret_confesseur")]
    );
    assert!(!events.is_empty(), "the arrival is announced");
}

#[test]
fn companions_add_their_effects() {
    let data = game_data();
    let mut state = start(data, "fac_england", 1);
    let edward = chr("chr_edward_iii");
    let before = skills::character_effects(&state, data, &edward);
    state
        .characters
        .get_mut(&edward)
        .unwrap()
        .retinue
        .extend([ret("ret_maitre_engins"), ret("ret_heraut")]);
    let after = skills::character_effects(&state, data, &edward);
    assert_eq!(after.siege_speed.percent - before.siege_speed.percent, 15.0);
    // Prestige is paid yearly, not folded into the effects.
    assert_eq!(after.prestige, before.prestige);
    assert_eq!(retinue::yearly_prestige(&state, data, &edward), 2);
}

#[test]
fn the_heir_gathers_the_inheritable_companions() {
    let data = game_data();
    let mut state = start(data, "fac_france", 5);
    let philippe = chr("chr_philippe_vi");
    let jean = chr("chr_jean_de_normandie");
    state
        .characters
        .get_mut(&philippe)
        .unwrap()
        .retinue
        .extend([ret("ret_heraut"), ret("ret_confesseur")]);
    let mut events = Vec::new();
    characters::kill(&mut state, data, &philippe, &mut events);
    let dead = &state.characters[&philippe];
    assert!(!dead.alive);
    assert_eq!(dead.death_year, Some(state.year));
    assert!(dead.retinue.is_empty());
    assert_eq!(state.characters[&jean].retinue, vec![ret("ret_heraut")]);
}

#[test]
fn a_childless_general_loses_his_retinue() {
    let data = game_data();
    let mut state = start(data, "fac_england", 5);
    let mauny = chr("chr_gautier_de_mauny");
    state
        .characters
        .get_mut(&mauny)
        .unwrap()
        .retinue
        .push(ret("ret_heraut"));
    characters::kill(&mut state, data, &mauny, &mut Vec::new());
    let holders = state
        .characters
        .values()
        .filter(|c| c.retinue.contains(&ret("ret_heraut")))
        .count();
    assert_eq!(holders, 0);
}

#[test]
fn death_year_of_characters_already_dead_in_1337() {
    let data = game_data();
    let state = start(data, "fac_france", 1);
    for (id, c) in &state.characters {
        if c.alive {
            assert_eq!(c.death_year, None, "{id}");
        } else {
            assert!(c.death_year.is_some_and(|y| y <= 1337), "{id}");
        }
    }
}

fn place_general(state: &mut CampaignState, army: &ArmyId, general: &CharacterId) {
    state.armies.get_mut(army).unwrap().general = Some(general.clone());
    state.characters.get_mut(general).unwrap().army = Some(army.clone());
}

#[test]
fn companions_move_between_generals_standing_together() {
    let data = game_data();
    let mut state = start(data, "fac_england", 2);
    let england = fac("fac_england");
    let first = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == england)
        .map(|(id, _)| id.clone())
        .expect("England has an army");
    // A second army, one step away from the first.
    let second = ArmyId::from_index(9_999);
    let mut copy = state.armies[&first].clone();
    let away = state
        .settlements
        .keys()
        .find(|s| !copy.is_at(s))
        .unwrap()
        .clone();
    copy.position = sim_campaign::ArmyPosition::Settlement(away);
    state.armies.insert(second.clone(), copy);
    let armies = [first, second];
    let edward = chr("chr_edward_iii");
    // FE: Henry of Grosmont serves Lancaster now; Bohun is an English captain.
    let bohun = chr("chr_william_de_bohun");
    for army in &armies {
        state.armies.get_mut(army).unwrap().general = None;
    }
    place_general(&mut state, &armies[0], &edward);
    place_general(&mut state, &armies[1], &bohun);
    state
        .characters
        .get_mut(&edward)
        .unwrap()
        .retinue
        .push(ret("ret_ecuyer"));
    let order = || Order::TransferCompanion {
        from: edward.clone(),
        to: bohun.clone(),
        companion: ret("ret_ecuyer"),
    };
    // Apart: refused.
    let far = state.armies[&armies[0]].settlement().cloned().unwrap();
    assert!(matches!(
        state.submit_order(data, order()),
        Err(OrderError::Retinue(retinue::RetinueError::NotTogether))
    ));
    // Together: accepted.
    state.armies.get_mut(&armies[1]).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(far);
    state.submit_order(data, order()).expect("transfer");
    assert!(state.characters[&edward].retinue.is_empty());
    assert_eq!(state.characters[&bohun].retinue, vec![ret("ret_ecuyer")]);
    // Not his any more.
    assert!(state.submit_order(data, order()).is_err());
}

#[test]
fn old_saves_without_the_new_fields_still_load() {
    let data = game_data();
    let mut state = start(data, "fac_france", 9);
    state
        .characters
        .get_mut(&chr("chr_philippe_vi"))
        .unwrap()
        .retinue
        .push(ret("ret_heraut"));
    characters::kill(&mut state, data, &chr("chr_edward_iii"), &mut Vec::new());
    let saved = state.save_json();
    assert!(saved.contains("\"retinue\""));
    assert!(saved.contains("\"death_year\""));
    let restored = CampaignState::load_json(&saved).expect("round trip");
    assert_eq!(
        restored.characters[&chr("chr_philippe_vi")].retinue,
        vec![ret("ret_heraut")]
    );
    assert_eq!(
        restored.characters[&chr("chr_edward_iii")].death_year,
        Some(state.year)
    );

    let mut json: serde_json::Value = serde_json::from_str(&saved).unwrap();
    for character in json["characters"].as_object_mut().unwrap().values_mut() {
        let object = character.as_object_mut().unwrap();
        object.remove("retinue");
        object.remove("death_year");
    }
    let old = CampaignState::load_json(&json.to_string()).expect("pre-C7 save loads");
    assert!(old.characters.values().all(|c| c.retinue.is_empty()));
    assert!(old.characters.values().all(|c| c.death_year.is_none()));
    assert_eq!(old.state_version, state.state_version);
}

#[test]
fn the_ai_gains_companions_during_a_campaign() {
    let data = game_data();
    let mut state = start(data, "fac_france", 21);
    for _ in 0..16 {
        state.end_turn(data);
    }
    let holders = state
        .characters
        .values()
        .filter(|c| !c.retinue.is_empty() && c.faction != state.player_faction)
        .count();
    assert!(holders > 0, "AI generals build a retinue over four years");
    let cap = retinue::max_per_character(data);
    assert!(state.characters.values().all(|c| c.retinue.len() <= cap));
}
