//! P1: « pas de quartier » in the campaign — a victor who gave no quarter
//! takes no noble prisoner (no ransom) and its commander loses piety.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_battle::{BattleOutcome, BattleSim, SideId};
use sim_campaign::{ArmyId, CampaignState, EventKind, Order, NO_QUARTER_PIETY};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies()
        .iter()
        .filter(|(_, a)| a.faction == faction && a.general.is_some())
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has a led army")
}

/// France attacks an English army in Normandy; the battle waits for the 3D
/// result. Returns the state, both armies and a finished outcome where the
/// French won and the English general was caught.
fn french_victory(data: &GameData) -> (CampaignState, ArmyId, ArmyId, BattleOutcome) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    // Lot C4: armies stand on settlements; the province path heads for
    // the city of Normandy.
    let rouen = state.provinces[&prov("prov_normandie")].city.clone();
    state.armies.get_mut(&english).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(rouen);
    // Lot M2: the French stand next to Rouen and attack at once.
    let point = data
        .settlement_point(&state.provinces[&prov("prov_normandie")].city)
        .unwrap();
    state.armies.get_mut(&french).unwrap().position =
        sim_campaign::ArmyPosition::field([point[0] + 4.0, point[1]]);
    state
        .submit_order(
            data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            },
        )
        .unwrap();
    assert_eq!(state.pending_battles.len(), 1);
    let setup = state.battle_setup(data, 0).unwrap();
    let mut battle = BattleSim::new(setup, 4).unwrap();
    while !battle.is_finished() {
        battle.tick(10.0);
    }
    let mut outcome = battle.outcome().unwrap();
    outcome.winner = SideId::Attacker;
    outcome.attacker.routed = false;
    outcome.attacker.general_killed = false;
    outcome.attacker.general_captured = false;
    outcome.attacker.no_quarter = false;
    outcome.defender.routed = true;
    outcome.defender.general_killed = false;
    outcome.defender.general_captured = true;
    outcome.defender.no_quarter = false;
    (state, french, english, outcome)
}

#[test]
fn caught_general_is_held_for_ransom_without_the_order() {
    let data = data();
    let (mut state, french, english, outcome) = french_victory(&data);
    let english_general = state.army(&english).unwrap().general.clone().unwrap();
    let french_general = state.army(&french).unwrap().general.clone().unwrap();
    let piety = state.character(&french_general).unwrap().piety;
    state.resolve_pending_battle(&data, 0, &outcome).unwrap();
    let captive = state.character(&english_general).unwrap();
    assert!(captive.alive);
    assert!(captive.captive);
    assert_eq!(captive.captor, Some(fac("fac_france")));
    assert_eq!(state.character(&french_general).unwrap().piety, piety);
}

#[test]
fn no_quarter_victors_slay_the_caught_general_and_lose_piety() {
    let data = data();
    let (mut state, french, english, mut outcome) = french_victory(&data);
    outcome.attacker.no_quarter = true;
    let english_general = state.army(&english).unwrap().general.clone().unwrap();
    let french_general = state.army(&french).unwrap().general.clone().unwrap();
    let piety = state.character(&french_general).unwrap().piety;
    let events = state.resolve_pending_battle(&data, 0, &outcome).unwrap();

    let slain = state.character(&english_general).unwrap();
    assert!(!slain.alive, "no prisoner: the caught general is slain");
    assert!(!slain.captive);
    assert_eq!(slain.captor, None);
    assert!(
        state
            .characters
            .values()
            .all(|c| c.captor.as_ref() != Some(&fac("fac_france"))),
        "France holds no prisoner, hence no ransom"
    );
    let victor = state.character(&french_general).unwrap();
    if victor.alive {
        assert_eq!(
            i32::from(victor.piety),
            (i32::from(piety) - NO_QUARTER_PIETY).max(0)
        );
    }
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Battle && e.text_fr.starts_with("Pas de quartier")));
    assert!(events
        .iter()
        .any(|e| e.text_fr.contains("tombé sur le champ de bataille")));
}

#[test]
fn no_quarter_of_the_loser_changes_nothing() {
    let data = data();
    let (mut state, _french, english, mut outcome) = french_victory(&data);
    // The beaten English raised the dragon banner: they took no prisoner,
    // but the French victors may still ransom their general.
    outcome.defender.no_quarter = true;
    let english_general = state.army(&english).unwrap().general.clone().unwrap();
    let events = state.resolve_pending_battle(&data, 0, &outcome).unwrap();
    let captive = state.character(&english_general).unwrap();
    assert!(captive.alive && captive.captive);
    assert!(!events
        .iter()
        .any(|e| e.text_fr.starts_with("Pas de quartier")));
}
