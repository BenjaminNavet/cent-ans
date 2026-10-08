//! Lot C7a: the AI on the settlements — it wins back lost places, leaves a
//! garrison in the places it takes, goes home when idle abroad; and the
//! `GarrisonUnits` order it relies on.

use data_model::SettlementId;
use sim_campaign::{ArmyId, CampaignState, Order, OrderError, Place, SiegeState};

use data_model::test_support::{fac, game_data};

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

/// Destination of the last move order of `army` (lot M3: one order per
/// leg of the route), if any.
fn destination(orders: &[Order], army: &ArmyId) -> Option<SettlementId> {
    orders.iter().rev().find_map(|o| match o {
        Order::MoveArmy { army: a, target } if a == army => match target {
            sim_campaign::MoveOrderTarget::Place(Place::Settlement(s)) => Some(s.clone()),
            sim_campaign::MoveOrderTarget::Path(path) => match path.last() {
                Some(Place::Settlement(s)) => Some(s.clone()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    })
}

#[test]
fn garrison_order_moves_units_within_the_cap() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let army = main_army(&state, "fac_france");
    let meaux = set("set_meaux");
    state.armies.get_mut(&army).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(meaux.clone());
    state.settlements.get_mut(&meaux).unwrap().garrison.clear();
    let units = state.armies[&army].units.len();
    assert!(units >= 3);
    state
        .submit_order(
            data,
            Order::GarrisonUnits {
                army: army.clone(),
                unit_indices: vec![0],
            },
        )
        .unwrap();
    assert_eq!(state.settlements[&meaux].garrison.len(), 1);
    assert_eq!(state.armies[&army].units.len(), units - 1);
    // A town holds 3 units at most (`rules.json` garrison_cap).
    let refused = state.submit_order(
        data,
        Order::GarrisonUnits {
            army: army.clone(),
            unit_indices: vec![0, 1, 2],
        },
    );
    assert!(matches!(refused, Err(OrderError::GarrisonFull { cap: 3 })));
    // Not in a besieged place.
    state.settlements.get_mut(&meaux).unwrap().siege = Some(SiegeState {
        attacker: fac("fac_england"),
        turns_left: 3,
        turns_elapsed: 1,
        supplies: 100,
        breach: 0,
        started_turn: 0,
        engine_work: 0,
    });
    assert!(matches!(
        state.submit_order(
            data,
            Order::GarrisonUnits {
                army: army.clone(),
                unit_indices: vec![0],
            },
        ),
        Err(OrderError::SettlementBesieged)
    ));
}

#[test]
fn giving_every_unit_dissolves_the_army() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let army = main_army(&state, "fac_france");
    let vincennes = set("set_vincennes");
    let a = state.armies.get_mut(&army).unwrap();
    a.position = sim_campaign::ArmyPosition::Settlement(vincennes.clone());
    a.units.truncate(2);
    state
        .settlements
        .get_mut(&vincennes)
        .unwrap()
        .garrison
        .clear();
    state
        .submit_order(
            data,
            Order::GarrisonUnits {
                army: army.clone(),
                unit_indices: vec![0, 1],
            },
        )
        .unwrap();
    assert!(!state.armies.contains_key(&army));
    assert_eq!(state.settlements[&vincennes].garrison.len(), 2);
}

#[test]
fn the_ai_wins_back_a_lost_place() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 1).unwrap();
    // England took Meaux (France keeps it de jure), without a garrison.
    let meaux = set("set_meaux");
    let s = state.settlements.get_mut(&meaux).unwrap();
    s.controller = fac("fac_england");
    s.garrison.clear();
    // Lot SL1: the English host in London stands one crossing from Calais
    // (route de l'Étape) and France rightly defends Calais first; keep it
    // in York so that only the lost place is at stake.
    let english = state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == fac("fac_england"))
        .filter(|(_, a)| a.settlement() == Some(&set("set_londres")))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in english {
        state.armies.get_mut(&id).unwrap().position =
            sim_campaign::ArmyPosition::Settlement(set("set_york"));
    }
    let orders = ai::plan_turn(&state, data, &fac("fac_france"));
    let targets: Vec<SettlementId> = state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == fac("fac_france"))
        .filter_map(|(id, _)| destination(&orders, id))
        .collect();
    assert!(
        targets.contains(&meaux),
        "a French army marches on Meaux: {targets:?}"
    );
}

#[test]
fn the_ai_leaves_a_garrison_in_a_conquered_place() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let army = main_army(&state, "fac_england");
    // The English army has just taken Meaux, left without a garrison.
    let place = set("set_meaux");
    let s = state.settlements.get_mut(&place).unwrap();
    s.controller = fac("fac_england");
    s.garrison.clear();
    state.armies.get_mut(&army).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(place.clone());
    assert!(state.armies[&army].units.len() >= 3);
    let orders = ai::plan_turn(&state, data, &fac("fac_england"));
    assert!(
        orders.iter().any(|o| matches!(
            o,
            Order::GarrisonUnits { army: a, unit_indices } if a == &army && unit_indices.len() == 1
        )),
        "one unit stays in Meaux"
    );
}

#[test]
fn an_idle_army_abroad_goes_home() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    // Peace everywhere: no target, no threat.
    for f in state.factions.values_mut() {
        f.at_war_with.clear();
    }
    let army = main_army(&state, "fac_france");
    // Standing in a neutral place next to a French one.
    let france = fac("fac_france");
    let neutral = state
        .settlements
        .keys()
        .find(|id| {
            !state.is_friendly_settlement(&france, id)
                && sim_campaign::movement::edges(data, id)
                    .iter()
                    .any(|(n, _)| state.settlements[n].controller == france)
        })
        .cloned()
        .expect("a neutral place on the French border");
    state.armies.get_mut(&army).unwrap().position = sim_campaign::ArmyPosition::Settlement(neutral);
    let orders = ai::plan_turn(&state, data, &fac("fac_france"));
    let home = destination(&orders, &army).expect("the army marches home");
    assert!(state.is_friendly_settlement(&fac("fac_france"), &home));
    // The minimal AI does the same.
    let orders = ai::plan_turn_minimal(&state, data, &fac("fac_france"));
    let home = destination(&orders, &army).expect("the army marches home (minimal AI)");
    assert!(state.is_friendly_settlement(&fac("fac_france"), &home));
}
