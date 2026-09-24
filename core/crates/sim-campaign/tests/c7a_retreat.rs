//! Lot C7a: where the loser of a battle goes (`rules.json` § `retreat`).
//! Friendly settlement within reach, else neutral ground with stragglers
//! lost, else a rout (heavy losses, rally far away or dispersal).

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::movement::{edges, retreat_target, Retreat};
use sim_campaign::{ArmyId, CampaignState, EventKind, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

/// The English main army teleported to Saint-Denis, next to Paris.
fn english_at_saint_denis(data: &GameData) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.interactive_battles = false;
    let english = main_army(&state, "fac_england");
    state.armies.get_mut(&english).unwrap().location = set("set_saint_denis");
    assert!(state.is_at_war(&fac("fac_england"), &fac("fac_france")));
    (state, english)
}

/// Cheapest land neighbour of Saint-Denis.
fn nearest_neighbour(data: &GameData) -> SettlementId {
    edges(data, &set("set_saint_denis"))
        .into_iter()
        .filter(|(to, _)| {
            !data
                .movement_graph
                .edge(&set("set_saint_denis"), to)
                .unwrap()
                .sea
        })
        .min_by_key(|(to, cost)| (*cost, to.clone()))
        .map(|(to, _)| to)
        .expect("Saint-Denis has neighbours")
}

#[test]
fn rules_are_read_from_data() {
    let data = data();
    let rules = data.retreat_rules();
    assert!(rules.friendly_radius_steps > 0.0);
    assert!(rules.rout_loss_percent > rules.neutral_loss_percent);
}

#[test]
fn a_french_army_falls_back_on_a_french_place() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    let mut state = state;
    let french = main_army(&state, "fac_france");
    state.armies.get_mut(&french).unwrap().location = set("set_saint_denis");
    match retreat_target(&state, &data, &french) {
        Some(Retreat::Friendly(target)) => {
            assert_ne!(target, set("set_saint_denis"));
            assert!(state.is_friendly_settlement(&fac("fac_france"), &target));
        }
        other => panic!("expected a friendly retreat, got {other:?}"),
    }
}

#[test]
fn an_english_army_cut_off_near_paris_routs_and_disperses() {
    let data = data();
    let (state, english) = english_at_saint_denis(&data);
    // Every neighbour is French: no friendly place, no neutral ground.
    assert_eq!(
        retreat_target(&state, &data, &english),
        Some(Retreat::Rout(None))
    );
}

#[test]
fn an_allied_or_own_place_next_door_is_the_refuge() {
    let data = data();
    let (mut state, english) = english_at_saint_denis(&data);
    let neighbour = nearest_neighbour(&data);
    let s = state.settlements.get_mut(&neighbour).unwrap();
    s.controller = fac("fac_england");
    assert_eq!(
        retreat_target(&state, &data, &english),
        Some(Retreat::Friendly(neighbour))
    );
}

#[test]
fn neutral_ground_is_the_second_choice() {
    let data = data();
    let (mut state, english) = english_at_saint_denis(&data);
    let neighbour = nearest_neighbour(&data);
    let neutral = state
        .factions
        .keys()
        .find(|f| {
            f.as_str() != "fac_rebels"
                && !state.is_at_war(&fac("fac_england"), f)
                && !state.is_allied(&fac("fac_england"), f)
        })
        .cloned()
        .expect("a neutral faction");
    state.settlements.get_mut(&neighbour).unwrap().controller = neutral;
    assert_eq!(
        retreat_target(&state, &data, &english),
        Some(Retreat::Neutral(neighbour))
    );
}

#[test]
fn a_distant_friendly_place_is_a_rally_point_after_a_rout() {
    let mut data = data();
    let rules = data.settlement_rules.as_mut().unwrap();
    rules.retreat.friendly_radius_steps = 0.0;
    rules.retreat.neutral_radius_steps = 0.0;
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    let french = main_army(&state, "fac_france");
    state.armies.get_mut(&french).unwrap().location = set("set_saint_denis");
    match retreat_target(&state, &data, &french) {
        Some(Retreat::Rout(Some(target))) => {
            assert!(state.is_friendly_settlement(&fac("fac_france"), &target));
        }
        other => panic!("expected a rout with a rally point, got {other:?}"),
    }
}

/// End to end: France attacks the English army at Saint-Denis. Whoever
/// loses leaves the battlefield or disperses; two enemies never share the
/// settlement after the battle, and the outcome is deterministic.
#[test]
fn the_loser_never_stays_on_the_battlefield() {
    let data = data();
    let run = |seed_boost: u32| {
        let (mut state, english) = english_at_saint_denis(&data);
        let french = main_army(&state, "fac_france");
        // Give France the upper hand so that England loses.
        for _ in 0..seed_boost {
            let extra = state.armies[&french].units.clone();
            state.armies.get_mut(&french).unwrap().units.extend(extra);
        }
        state
            .submit_order(
                &data,
                Order::move_along(french.clone(), vec![set("set_saint_denis")]),
            )
            .unwrap();
        let events = state.end_turn_with(&data, idle);
        assert!(events.iter().any(|e| e.kind == EventKind::Battle));
        let english_left = state.armies.get(&english).map(|a| a.location.clone());
        let french_at = state.armies.get(&french).map(|a| a.location.clone());
        (english_left, french_at, events.len())
    };
    let (english_left, french_at, count) = run(2);
    let saint_denis = set("set_saint_denis");
    if french_at.as_ref() == Some(&saint_denis) {
        // France won: England retreated (nowhere) or dispersed.
        assert_ne!(english_left.as_ref(), Some(&saint_denis));
    }
    assert!(
        !(english_left.as_ref() == Some(&saint_denis) && french_at.as_ref() == Some(&saint_denis)),
        "two enemies left on the battlefield"
    );
    assert_eq!(run(2), (english_left, french_at, count), "deterministic");
}

#[test]
fn a_routed_army_takes_heavy_losses_or_disperses() {
    let data = data();
    let (mut state, english) = english_at_saint_denis(&data);
    let french = main_army(&state, "fac_france");
    for _ in 0..3 {
        let extra = state.armies[&french].units.clone();
        state.armies.get_mut(&french).unwrap().units.extend(extra);
    }
    state
        .submit_order(
            &data,
            Order::move_along(french.clone(), vec![set("set_saint_denis")]),
        )
        .unwrap();
    let events = state.end_turn_with(&data, idle);
    assert!(
        !state.armies.contains_key(&english),
        "cut off from its places, the beaten English army disperses"
    );
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::ArmyDestroyed && e.army.as_ref() == Some(&english)));
}
