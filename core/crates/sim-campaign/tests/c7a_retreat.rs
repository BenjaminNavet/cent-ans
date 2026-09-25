//! Where the loser of a battle goes: lot C7a rule (`rules.json` §
//! `retreat`) on the navigation grid (lot M2, spec § 3.3). Friendly
//! settlement within reach, else a fallback away from the victor with
//! stragglers lost, else a rout (heavy losses, rally far away or dispersal).

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::movement::{retreat_target, Retreat};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, EventKind, Order};

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

/// The English main army teleported to Saint-Denis, next to Paris.
fn english_at_saint_denis(data: &GameData) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.interactive_battles = false;
    let english = main_army(&state, "fac_england");
    state.armies.get_mut(&english).unwrap().position =
        ArmyPosition::Settlement(set("set_saint_denis"));
    assert!(state.is_at_war(&fac("fac_england"), &fac("fac_france")));
    (state, english)
}

fn paris(data: &GameData) -> [f32; 2] {
    data.settlement_point(&set("set_paris")).unwrap()
}

fn distance_km(data: &GameData, a: [f32; 2], b: [f32; 2]) -> f64 {
    let px = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
    f64::from(px) / data.navgrid().px_per_km()
}

#[test]
fn rules_are_read_from_data() {
    let data = data();
    let rules = data.retreat_rules();
    assert!(rules.friendly_radius_steps > 0.0);
    assert!(rules.rout_loss_percent > rules.neutral_loss_percent);
    assert_eq!(data.free_movement_rules().retreat_fallback_km, 15.0);
}

#[test]
fn a_french_army_falls_back_on_a_french_place() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    let french = main_army(&state, "fac_france");
    state.armies.get_mut(&french).unwrap().position =
        ArmyPosition::Settlement(set("set_saint_denis"));
    let battlefield = data.settlement_point(&set("set_saint_denis")).unwrap();
    match retreat_target(&state, &data, &french, battlefield) {
        Some(Retreat::Friendly(target)) => {
            assert_ne!(target, set("set_saint_denis"));
            assert!(state.is_friendly_settlement(&fac("fac_france"), &target));
        }
        other => panic!("expected a friendly retreat, got {other:?}"),
    }
}

#[test]
fn an_english_army_cut_off_near_paris_falls_back_away_from_the_victor() {
    let mut data = data();
    // Abbeville (English Ponthieu) lies within two steps of Saint-Denis:
    // shrink the friendly radius so that no English place is in reach.
    data.settlement_rules
        .as_mut()
        .unwrap()
        .retreat
        .friendly_radius_steps = 0.5;
    let (state, english) = english_at_saint_denis(&data);
    // The French victor came from Paris, south of Saint-Denis.
    match retreat_target(&state, &data, &english, paris(&data)) {
        Some(Retreat::Fallback(point)) => {
            let start = data.settlement_point(&set("set_saint_denis")).unwrap();
            let moved = distance_km(&data, start, point);
            assert!((12.0..=18.0).contains(&moved), "fell back {moved} km");
            assert!(
                distance_km(&data, point, paris(&data)) > distance_km(&data, start, paris(&data)),
                "away from the victor"
            );
        }
        other => panic!("expected a fallback, got {other:?}"),
    }
}

#[test]
fn an_allied_or_own_place_next_door_is_the_refuge() {
    let data = data();
    let (mut state, english) = english_at_saint_denis(&data);
    let start = data.settlement_point(&set("set_saint_denis")).unwrap();
    let neighbour = sim_campaign::march::nearest_settlement_where(&data, start, |id| {
        id != &set("set_saint_denis") && id != &set("set_paris")
    })
    .unwrap();
    state.settlements.get_mut(&neighbour).unwrap().controller = fac("fac_england");
    assert_eq!(
        retreat_target(&state, &data, &english, paris(&data)),
        Some(Retreat::Friendly(neighbour))
    );
}

#[test]
fn a_distant_friendly_place_is_a_rally_point_after_a_rout() {
    let mut data = data();
    let rules = data.settlement_rules.as_mut().unwrap();
    rules.retreat.friendly_radius_steps = 0.0;
    // A fallback beyond the sea in every direction tried: none is found.
    let mut free = data.free_movement_rules().clone();
    free.retreat_fallback_km = 0.1;
    data.free_movement = Some(free);
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    let french = main_army(&state, "fac_france");
    state.armies.get_mut(&french).unwrap().position =
        ArmyPosition::Settlement(set("set_saint_denis"));
    // A fallback of 0.1 km stays in the start cell (a French place: not
    // blocked for the French), so block it with an enemy place.
    state
        .settlements
        .get_mut(&set("set_saint_denis"))
        .unwrap()
        .controller = fac("fac_england");
    let battlefield = data.settlement_point(&set("set_saint_denis")).unwrap();
    match retreat_target(&state, &data, &french, battlefield) {
        Some(Retreat::Rout(Some(target))) => {
            assert!(state.is_friendly_settlement(&fac("fac_france"), &target));
        }
        other => panic!("expected a rout with a rally point, got {other:?}"),
    }
}

/// End to end: France attacks the English army at Saint-Denis. Whoever
/// loses leaves the battlefield or disperses, and the outcome is
/// deterministic.
#[test]
fn the_loser_never_stays_on_the_battlefield() {
    let data = data();
    let run = |boost: u32| {
        let (mut state, english) = english_at_saint_denis(&data);
        let french = main_army(&state, "fac_france");
        // Give France the upper hand so that England loses.
        for _ in 0..boost {
            let extra = state.armies[&french].units.clone();
            state.armies.get_mut(&french).unwrap().units.extend(extra);
        }
        state
            .submit_order(
                &data,
                Order::Attack {
                    army: french.clone(),
                    target_army: english.clone(),
                },
            )
            .unwrap();
        assert!(state
            .pending_events
            .iter()
            .any(|e| e.kind == EventKind::Battle));
        let english_left = state.armies.get(&english).map(|a| a.position.clone());
        let french_at = state.armies.get(&french).map(|a| a.position.clone());
        (english_left, french_at, state.pending_events.len())
    };
    let (english_left, french_at, count) = run(2);
    assert_ne!(
        english_left,
        Some(ArmyPosition::Settlement(set("set_saint_denis"))),
        "the beaten English left Saint-Denis"
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
    let before = state.armies[&english].total_strength();
    state
        .submit_order(
            &data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            },
        )
        .unwrap();
    match state.armies.get(&english) {
        // Fell back with stragglers lost.
        Some(army) => {
            assert!(army.total_strength() < before);
            assert_eq!(army.movement_left, 0);
            assert!(!army.is_at(&set("set_saint_denis")));
        }
        None => assert!(state
            .pending_events
            .iter()
            .any(|e| e.kind == EventKind::ArmyDestroyed && e.army.as_ref() == Some(&english))),
    }
}
