//! The scale on synthetic rules: the real data of 1337 with a crusade
//! handed to an existing faction, so the tests do not depend on the
//! values (or the faction) of `data/rules/crusade.json`.

use std::sync::OnceLock;

use data_model::test_support::{fac, game_data, prov};

use super::*;
use crate::orders::OrderError;
use crate::state::{Army, ArmyPosition};

const CRUSADERS: &str = "fac_cyprus";
const HOLDER: &str = "fac_mamluks";
const BASE: &str = "set_famagusta";
const TARGET: &str = "prov_jerusalem";

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
}

fn synthetic_rules() -> CrusadeRules {
    serde_json::from_value(serde_json::json!({
        "faction": CRUSADERS,
        "base_settlement": BASE,
        "target_province": TARGET,
        "holy_land": [TARGET, "prov_gaza", "prov_safad"],
        "coastal_holy_land": ["prov_gaza", "prov_safad"],
        "fervor": {
            "start": 60,
            "decay_per_turn": 1,
            "decay_above_high": 2,
            "battle_won_other_faith": 6,
            "battle_lost": -8,
            "holy_land_settlement_taken": 10,
            "target_taken": 40,
            "target_floor": 50,
            "preach": 10,
            "war_declared_same_faith": -30,
            "battle_same_faith": -10,
            "truce_with_target_holder_per_turn": -2
        },
        "alms": { "base": 100, "per_fervor": 10 },
        "passage": {
            "cost": 1000,
            "cooldown_turns": 6,
            "delay_turns": 2,
            "units_base": 1,
            "fervor_per_extra_unit": 25,
            "max_units": 4,
            "unit_table": [
                { "unit": "unit_crossbowmen", "weight": 3 },
                { "unit": "unit_knights", "weight": 1 }
            ]
        },
        "zeal": {
            "high_threshold": 70,
            "high_morale": 10,
            "low_threshold": 30,
            "low_morale": -10
        },
        "desertion": { "threshold": 20, "men_percent_per_turn": 5, "zero_multiplier": 2 },
        "starting_army": ["unit_knights", "unit_crossbowmen", "unit_crossbowmen"],
        "target_taken_prestige": 40,
        "relief": {
            "units": 3,
            "cooldown_turns": 5,
            "unit_table": [{ "unit": "unit_urban_militia", "weight": 1 }]
        }
    }))
    .expect("synthetic rules are well formed")
}

fn data() -> &'static GameData {
    static DATA: OnceLock<GameData> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut data = game_data().clone();
        data.crusade_rules = Some(synthetic_rules());
        // The precomputed JR4b fit (ADR 0233) is made for the real crusaders;
        // these synthetic ones start unfitted, as the fit skips them.
        data.starting_fit = None;
        data
    })
}

/// A campaign played by the synthetic crusaders, at war with the holder
/// of the target (no peace penalty unless a test asks for it).
fn campaign() -> CampaignState {
    let mut state = CampaignState::new_1337(data(), fac(CRUSADERS), 7).expect("campaign");
    for (a, b) in [(CRUSADERS, HOLDER), (HOLDER, CRUSADERS)] {
        let f = state.factions.get_mut(&fac(a)).unwrap();
        f.at_war_with.insert(fac(b));
        f.truces.remove(&fac(b));
    }
    state
}

/// Units of the crusaders: garrisons of their places and armies.
fn crusader_units(state: &CampaignState) -> usize {
    let garrisons: usize = state
        .settlements
        .values()
        .filter(|s| s.controller == fac(CRUSADERS))
        .map(|s| s.garrison.len())
        .sum();
    let armies: usize = state
        .armies
        .values()
        .filter(|a| a.faction == fac(CRUSADERS))
        .map(|a| a.units.len())
        .sum();
    garrisons + armies
}

fn set_fervor(state: &mut CampaignState, fervor: u8) {
    state.crusade.as_mut().unwrap().fervor = fervor;
}

fn fervor(state: &CampaignState) -> u8 {
    state.crusade.as_ref().unwrap().fervor
}

fn end_of_turn(state: &mut CampaignState) -> Vec<GameEvent> {
    let mut events = Vec::new();
    resolve_crusade(state, data(), &mut events);
    state.turn += 1;
    events
}

fn hand(state: &mut CampaignState, settlement: &SettlementId, to: &str) {
    let s = state.settlements.get_mut(settlement).unwrap();
    s.controller = fac(to);
}

fn target_city(state: &CampaignState) -> SettlementId {
    state.province_city_id(&prov(TARGET)).unwrap().clone()
}

#[test]
fn setup_opens_the_crusade_and_bases_the_army() {
    let state = campaign();
    let crusade = state.crusade.as_ref().expect("opened at setup");
    assert_eq!(crusade.fervor, 60);
    assert!(!crusade.target_taken);
    assert!(is_crusader(&state, data(), &fac(CRUSADERS)));
    assert!(!is_crusader(&state, data(), &fac(HOLDER)));
    let army = state
        .armies
        .values()
        .find(|a| a.faction == fac(CRUSADERS))
        .expect("starting army");
    assert!(army.is_at(&set(BASE)), "based in the rules' settlement");
    let types: Vec<&str> = army.units.iter().map(|u| u.unit_type.as_str()).collect();
    assert_eq!(
        types,
        ["unit_knights", "unit_crossbowmen", "unit_crossbowmen"]
    );
}

#[test]
fn fervor_wears_off_and_stays_within_bounds() {
    let mut state = campaign();
    end_of_turn(&mut state);
    assert_eq!(fervor(&state), 59, "the vow wears off");
    assert_eq!(
        state.crusade.as_ref().unwrap().last_changes,
        vec![("Le vœu s'use".to_owned(), -1)]
    );
    // 0 is a floor, 100 a ceiling.
    set_fervor(&mut state, 0);
    end_of_turn(&mut state);
    assert_eq!(fervor(&state), 0);
    set_fervor(&mut state, 98);
    on_battle(
        &mut state,
        data(),
        &fac(CRUSADERS),
        &fac(HOLDER),
        &fac(CRUSADERS),
    );
    assert_eq!(fervor(&state), 100);
    // Only the points really gained are noted, under the turn's causes.
    let changes = &state.crusade.as_ref().unwrap().last_changes;
    assert_eq!(changes, &vec![("Victoire sur une autre foi".to_owned(), 2)]);
}

#[test]
fn exaltation_falls_back_faster_above_the_high_threshold() {
    let mut state = campaign();
    set_fervor(&mut state, 80);
    end_of_turn(&mut state);
    assert_eq!(fervor(&state), 77, "1 of wear + 2 of exaltation");
    assert!(state
        .crusade
        .as_ref()
        .unwrap()
        .last_changes
        .contains(&("L'exaltation retombe".to_owned(), -2)));
    // Below the threshold, the plain wear only.
    set_fervor(&mut state, 69);
    end_of_turn(&mut state);
    assert_eq!(fervor(&state), 68);
}

#[test]
fn a_contingent_respects_the_garrison_cap() {
    let mut state = campaign();
    let rules = synthetic_rules();
    let base = set(BASE);
    let cap = data()
        .settlement_rules
        .as_ref()
        .and_then(|r| r.garrison_cap.get(&state.settlements[&base].kind).copied())
        .expect("a cap");
    // Every port full: the volunteers form an army in the booked port.
    let militia = data()
        .unit_types
        .get(&UnitTypeId::new("unit_urban_militia").unwrap())
        .unwrap();
    for port in held_ports(&state, data(), &rules) {
        let place = state.settlements.get_mut(&port).unwrap();
        let kind_cap = data()
            .settlement_rules
            .as_ref()
            .and_then(|r| r.garrison_cap.get(&place.kind).copied())
            .unwrap();
        while place.garrison.len() < kind_cap {
            place.garrison.push(Unit::fresh(militia));
        }
    }
    state.armies.retain(|_, a| a.faction != fac(CRUSADERS));
    let before = crusader_units(&state);
    state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
    preach_passage(&mut state, data(), &fac(CRUSADERS)).expect("preached");
    end_of_turn(&mut state);
    end_of_turn(&mut state);
    assert_eq!(state.settlements[&base].garrison.len(), cap);
    assert_eq!(crusader_units(&state), before + 3);
    let army: Vec<_> = state
        .armies
        .values()
        .filter(|a| a.faction == fac(CRUSADERS))
        .collect();
    assert_eq!(army.len(), 1);
    assert_eq!(army[0].units.len(), 3);
    assert_eq!(army[0].settlement(), Some(&base));
}

#[test]
fn the_target_and_each_place_lift_the_fervour_once() {
    let mut state = campaign();
    let city = target_city(&state);
    let ruler = state.factions[&fac(CRUSADERS)].ruler.clone().unwrap();
    let mut events = Vec::new();
    hand(&mut state, &city, CRUSADERS);
    on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut events);
    assert_eq!(fervor(&state), 100);
    let prestige = state.characters[&ruler].prestige;
    // Lost and delivered again: the floor and the seat, no new gain.
    hand(&mut state, &city, HOLDER);
    on_settlement_taken(&mut state, data(), &fac(HOLDER), &city, &mut events);
    set_fervor(&mut state, 30);
    hand(&mut state, &city, CRUSADERS);
    on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut events);
    assert_eq!(fervor(&state), 50, "lifted to the floor only");
    assert_eq!(state.characters[&ruler].prestige, prestige);
    assert_eq!(state.factions[&fac(CRUSADERS)].capital, prov(TARGET));
    // A place of the Holy Land counts once, however often retaken.
    set_fervor(&mut state, 60);
    let acre = set("set_acre");
    for (taker, gain) in [(CRUSADERS, 10), (HOLDER, 0), (CRUSADERS, 0)] {
        let before = fervor(&state);
        hand(&mut state, &acre, taker);
        on_settlement_taken(&mut state, data(), &fac(taker), &acre, &mut events);
        assert_eq!(i32::from(fervor(&state)) - i32::from(before), gain);
    }
}

#[test]
fn the_capital_goes_back_when_the_target_is_lost() {
    let mut state = campaign();
    let seat = state.factions[&fac(CRUSADERS)].capital.clone();
    let city = target_city(&state);
    let mut events = Vec::new();
    hand(&mut state, &city, CRUSADERS);
    on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut events);
    assert_eq!(state.factions[&fac(CRUSADERS)].capital, prov(TARGET));
    hand(&mut state, &city, HOLDER);
    on_settlement_taken(&mut state, data(), &fac(HOLDER), &city, &mut events);
    assert_eq!(state.factions[&fac(CRUSADERS)].capital, seat);
    let lost = events.last().unwrap();
    assert!(lost.public && lost.loss, "{lost:?}");
}

#[test]
fn the_master_of_a_besieged_holy_place_calls_its_defence_once() {
    let mut state = campaign();
    let city = target_city(&state);
    let besiege = |state: &mut CampaignState| {
        state.settlements.get_mut(&city).unwrap().siege = Some(
            serde_json::from_value(serde_json::json!({
                "attacker": CRUSADERS,
                "turns_left": 9
            }))
            .unwrap(),
        );
    };
    // A rebel place besieged first in the order does not use the call.
    let acre = set("set_acre");
    assert!(acre < city);
    hand(&mut state, &acre, "fac_rebels");
    state.settlements.get_mut(&acre).unwrap().siege = Some(
        serde_json::from_value(serde_json::json!({
            "attacker": CRUSADERS,
            "turns_left": 9
        }))
        .unwrap(),
    );
    let rebels = state.settlements[&acre].garrison.len();
    besiege(&mut state);
    let before = state.settlements[&city].garrison.len();
    let events = end_of_turn(&mut state);
    assert_eq!(state.settlements[&acre].garrison.len(), rebels);
    state.settlements.get_mut(&acre).unwrap().siege = None;
    let after = state.settlements[&city].garrison.len();
    assert_eq!(after, before + 3, "{events:?}");
    let call: Vec<_> = events
        .iter()
        .filter(|e| e.text_fr.contains("Appel à défendre"))
        .collect();
    assert_eq!(call.len(), 1, "{events:?}");
    assert_eq!(call[0].faction, Some(fac(HOLDER)));
    assert!(call[0].public, "the besieging crusade reads it too");
    assert_eq!(call[0].province, Some(prov(TARGET)));
    // Once per siege.
    end_of_turn(&mut state);
    assert_eq!(state.settlements[&city].garrison.len(), after);
    // A new siege waits for the cooldown.
    state.settlements.get_mut(&city).unwrap().siege = None;
    end_of_turn(&mut state);
    besiege(&mut state);
    end_of_turn(&mut state);
    assert_eq!(state.settlements[&city].garrison.len(), after);
    for _ in 0..3 {
        end_of_turn(&mut state);
    }
    assert!(state.settlements[&city].garrison.len() > after);
    // A place outside the Holy Land is never relieved.
    let elsewhere = set("set_limassol");
    let mut other = campaign();
    other.settlements.get_mut(&elsewhere).unwrap().controller = fac(HOLDER);
    other.settlements.get_mut(&elsewhere).unwrap().siege = Some(
        serde_json::from_value(serde_json::json!({
            "attacker": CRUSADERS,
            "turns_left": 9
        }))
        .unwrap(),
    );
    let size = other.settlements[&elsewhere].garrison.len();
    end_of_turn(&mut other);
    assert_eq!(other.settlements[&elsewhere].garrison.len(), size);
}

#[test]
fn peace_with_the_holder_of_the_target_costs_fervor() {
    let mut state = campaign();
    for (a, b) in [(CRUSADERS, HOLDER), (HOLDER, CRUSADERS)] {
        let f = state.factions.get_mut(&fac(a)).unwrap();
        f.at_war_with.remove(&fac(b));
    }
    end_of_turn(&mut state);
    assert_eq!(fervor(&state), 60 - 1 - 2);
}

#[test]
fn battles_and_wars_move_the_gauge() {
    let mut state = campaign();
    let brothers = fac("fac_hospitallers");
    // Lost to another faith: only the defeat.
    on_battle(
        &mut state,
        data(),
        &fac(HOLDER),
        &fac(CRUSADERS),
        &fac(HOLDER),
    );
    assert_eq!(fervor(&state), 52);
    // Won against the same faith it attacked: the scandal, no gain.
    on_battle(
        &mut state,
        data(),
        &fac(CRUSADERS),
        &brothers,
        &fac(CRUSADERS),
    );
    assert_eq!(fervor(&state), 42);
    // Lost against the same faith it attacked: both.
    on_battle(
        &mut state,
        data(),
        &brothers,
        &fac(CRUSADERS),
        &fac(CRUSADERS),
    );
    assert_eq!(fervor(&state), 24);
    // Attacked by brothers in faith, it only defends itself: no
    // scandal, won or lost (the defeat still counts).
    on_battle(&mut state, data(), &fac(CRUSADERS), &brothers, &brothers);
    assert_eq!(fervor(&state), 24);
    on_battle(&mut state, data(), &brothers, &fac(CRUSADERS), &brothers);
    assert_eq!(fervor(&state), 16);
    set_fervor(&mut state, 24);
    // Other factions' battles and wars are none of its business.
    on_battle(&mut state, data(), &fac(HOLDER), &brothers, &fac(HOLDER));
    on_war_declared(&mut state, data(), &fac(HOLDER), &fac(CRUSADERS));
    on_war_declared(&mut state, data(), &fac(CRUSADERS), &fac(HOLDER));
    assert_eq!(fervor(&state), 24);
    set_fervor(&mut state, 60);
    on_war_declared(
        &mut state,
        data(),
        &fac(CRUSADERS),
        &fac("fac_hospitallers"),
    );
    assert_eq!(fervor(&state), 30);
    // Through the real order too.
    set_fervor(&mut state, 60);
    state
        .declare_war(data(), &fac(CRUSADERS), &fac("fac_papacy"))
        .expect("war declared");
    assert_eq!(fervor(&state), 30);
}

#[test]
fn holy_land_places_and_the_target_floor() {
    let mut state = campaign();
    let mut events = Vec::new();
    // A place of the Holy Land.
    hand(&mut state, &set("set_acre"), CRUSADERS);
    on_settlement_taken(
        &mut state,
        data(),
        &fac(CRUSADERS),
        &set("set_acre"),
        &mut events,
    );
    assert_eq!(fervor(&state), 70);
    assert!(events.is_empty());
    // A place elsewhere gives nothing.
    on_settlement_taken(
        &mut state,
        data(),
        &fac(CRUSADERS),
        &set("set_limassol"),
        &mut events,
    );
    assert_eq!(fervor(&state), 70);
    // The city of the vow: +40 (not +10 more), capped, with its event.
    set_fervor(&mut state, 5);
    let ruler = state.factions[&fac(CRUSADERS)].ruler.clone().unwrap();
    let prestige = state.characters[&ruler].prestige;
    let city = target_city(&state);
    hand(&mut state, &city, CRUSADERS);
    on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut events);
    assert_eq!(fervor(&state), 50, "5 + 40, lifted to the floor");
    assert!(state.crusade.as_ref().unwrap().target_taken);
    assert_eq!(state.factions[&fac(CRUSADERS)].capital, prov(TARGET));
    assert_eq!(state.characters[&ruler].prestige, prestige + 40);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::Crusade);
    assert_eq!(events[0].province, Some(prov(TARGET)));
    assert!(events[0].public && !events[0].loss, "{}", events[0].text_fr);
    // The floor holds against defeats and wear.
    set_fervor(&mut state, 53);
    on_battle(
        &mut state,
        data(),
        &fac(HOLDER),
        &fac(CRUSADERS),
        &fac(HOLDER),
    );
    assert_eq!(fervor(&state), 50);
    end_of_turn(&mut state);
    assert_eq!(fervor(&state), 50);
    assert_eq!(
        crusade_view(&state, data(), &fac(CRUSADERS)).unwrap().floor,
        50
    );
    // Delivered only once: a later capture there gives nothing more.
    let mut again = Vec::new();
    on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut again);
    assert!(again.is_empty());
    assert_eq!(
        fervor(&state),
        50,
        "the city was counted at its deliverance"
    );
    // Lost: the floor goes, with an event.
    hand(&mut state, &city, HOLDER);
    let lost = end_of_turn(&mut state);
    assert!(!state.crusade.as_ref().unwrap().target_taken);
    assert_eq!(fervor(&state), 49);
    // The loss is news for every faction too.
    assert!(lost.iter().any(|e| e.public && e.loss), "{lost:?}");
}

#[test]
fn the_capture_of_the_target_city_goes_through_the_hook() {
    let mut state = campaign();
    let city = target_city(&state);
    let mut events = Vec::new();
    crate::siege::capture(&mut state, data(), &city, &fac(CRUSADERS), &mut events);
    assert!(state.crusade.as_ref().unwrap().target_taken);
    assert_eq!(fervor(&state), 100);
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Crusade && e.text_fr.contains("délivrée")));
    // Retaken by its former master: the floor goes at once.
    crate::siege::capture(&mut state, data(), &city, &fac(HOLDER), &mut events);
    assert!(!state.crusade.as_ref().unwrap().target_taken);
}

#[test]
fn alms_follow_fervor_and_are_income() {
    let mut state = campaign();
    assert_eq!(alms(&state, data(), &fac(CRUSADERS)), 100 + 10 * 60);
    assert_eq!(alms(&state, data(), &fac(HOLDER)), 0);
    let with = state.faction_income(data(), &fac(CRUSADERS));
    set_fervor(&mut state, 10);
    let poorer = state.faction_income(data(), &fac(CRUSADERS));
    assert_eq!(with - poorer, 500, "alms are part of the income");
    let view = crusade_view(&state, data(), &fac(CRUSADERS)).unwrap();
    assert_eq!(view.alms, 200);
    end_of_turn(&mut state);
    assert_eq!(state.crusade.as_ref().unwrap().alms_last_turn, 200);
    // A dead crusade pays nothing.
    state.factions.get_mut(&fac(CRUSADERS)).unwrap().alive = false;
    assert_eq!(alms(&state, data(), &fac(CRUSADERS)), 0);
}

#[test]
fn preach_is_refused_with_a_reason() {
    let mut state = campaign();
    // Not the crusader faction.
    assert_eq!(
        preach_passage(&mut state, data(), &fac(HOLDER)),
        Err(CrusadeError::NotCrusaders)
    );
    assert!(crusade_view(&state, data(), &fac(HOLDER)).is_none());
    // Not enough money.
    state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 999;
    assert_eq!(
        preach_blocker(&state, data(), &fac(CRUSADERS)),
        Some(CrusadeError::InsufficientFunds {
            needed: 1000,
            available: 999
        })
    );
    let view = crusade_view(&state, data(), &fac(CRUSADERS)).unwrap();
    assert!(!view.passage_available);
    assert!(view.passage_blocker.contains("Trésor insuffisant"));
    assert!(ai_preach(&state, data(), &fac(CRUSADERS)).is_empty());
    // Through the order, in French.
    match state.submit_order(data(), Order::PreachPassage) {
        Err(OrderError::Crusade(CrusadeError::InsufficientFunds { .. })) => {}
        other => panic!("unexpected {other:?}"),
    }
    // No port held.
    state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
    let ports: Vec<SettlementId> = held_ports(&state, data(), &synthetic_rules());
    assert_eq!(ports.first(), Some(&set(BASE)), "the base comes first");
    for port in &ports {
        hand(&mut state, port, HOLDER);
    }
    assert_eq!(
        preach_blocker(&state, data(), &fac(CRUSADERS)),
        Some(CrusadeError::NoPort)
    );
    for port in &ports {
        hand(&mut state, port, CRUSADERS);
    }
    // Accepted once, then on cooldown.
    assert_eq!(
        ai_preach(&state, data(), &fac(CRUSADERS)),
        vec![Order::PreachPassage]
    );
    state
        .submit_order(data(), Order::PreachPassage)
        .expect("preached");
    assert_eq!(state.factions[&fac(CRUSADERS)].treasury, 4000);
    assert_eq!(fervor(&state), 70);
    assert_eq!(
        preach_blocker(&state, data(), &fac(CRUSADERS)),
        Some(CrusadeError::Cooldown(6))
    );
    assert_eq!(
        CrusadeError::Cooldown(6).to_string(),
        "Passage déjà prêché : nouvel appel dans 6 tours."
    );
    assert_eq!(
        CrusadeError::Cooldown(1).to_string(),
        "Passage déjà prêché : nouvel appel dans 1 tour."
    );
    for _ in 0..6 {
        end_of_turn(&mut state);
    }
    assert_eq!(preach_blocker(&state, data(), &fac(CRUSADERS)), None);
}

#[test]
fn contingent_size_follows_fervor() {
    let passage = synthetic_rules().passage;
    assert_eq!(passage.units_at(0), 1);
    assert_eq!(passage.units_at(24), 1);
    assert_eq!(passage.units_at(25), 2);
    assert_eq!(passage.units_at(60), 3);
    assert_eq!(passage.units_at(100), 4, "capped by max_units");
    let state = campaign();
    let view = crusade_view(&state, data(), &fac(CRUSADERS)).unwrap();
    assert_eq!(view.passage_units, 3, "sized at 60 + 10");
    assert_eq!(view.passage_cost, 1000);
    assert_eq!(view.passage_delay, 2);
}

#[test]
fn contingent_lands_two_turns_later() {
    let mut state = campaign();
    state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
    preach_passage(&mut state, data(), &fac(CRUSADERS)).expect("preached");
    let pending = state.crusade.as_ref().unwrap().pending_passages.clone();
    assert_eq!(
        pending,
        vec![PendingPassage {
            arrival_turn: state.turn + 2,
            port: set(BASE),
            units: 3
        }]
    );
    let view = crusade_view(&state, data(), &fac(CRUSADERS)).unwrap();
    assert_eq!(view.pending.len(), 1);
    assert_eq!(view.pending[0].turns_left, 2);
    let before = crusader_units(&state);
    // End of the turn of the call: still at sea.
    end_of_turn(&mut state);
    assert_eq!(crusader_units(&state), before);
    // End of the next one: ashore for the second turn after the call
    // (in the port's garrison within its cap, the rest elsewhere).
    let events = end_of_turn(&mut state);
    assert_eq!(crusader_units(&state), before + 3);
    assert!(state.crusade.as_ref().unwrap().pending_passages.is_empty());
    assert!(events.iter().any(|e| e.kind == EventKind::Crusade));
}

#[test]
fn contingent_finds_another_port_or_is_lost() {
    let mut state = campaign();
    let rules = synthetic_rules();
    state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
    preach_passage(&mut state, data(), &fac(CRUSADERS)).expect("preached");
    // The booked port falls: another held port receives the volunteers.
    hand(&mut state, &set(BASE), HOLDER);
    let fallback = held_ports(&state, data(), &rules)
        .into_iter()
        .next()
        .expect("another port");
    let before = crusader_units(&state);
    let garrison = state.settlements[&fallback].garrison.len();
    end_of_turn(&mut state);
    end_of_turn(&mut state);
    assert_eq!(crusader_units(&state), before + 3);
    assert!(state.settlements[&fallback].garrison.len() > garrison);

    // No port at all: lost, with an event.
    let mut state = campaign();
    state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
    preach_passage(&mut state, data(), &fac(CRUSADERS)).expect("preached");
    for port in held_ports(&state, data(), &rules) {
        hand(&mut state, &port, HOLDER);
    }
    end_of_turn(&mut state);
    let events = end_of_turn(&mut state);
    assert!(state.crusade.as_ref().unwrap().pending_passages.is_empty());
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Crusade && e.text_fr.contains("se disperse")));
}

#[test]
fn zeal_and_desertion_follow_the_thresholds() {
    let mut state = campaign();
    let crusaders = fac(CRUSADERS);
    assert_eq!(zeal_morale(&state, data(), &crusaders), 0);
    set_fervor(&mut state, 70);
    assert_eq!(zeal_morale(&state, data(), &crusaders), 10);
    assert_eq!(zeal_morale(&state, data(), &fac(HOLDER)), 0);
    set_fervor(&mut state, 29);
    assert_eq!(zeal_morale(&state, data(), &crusaders), -10);

    // One army of 1 000 men in one unit.
    let unit_type = &data().unit_types[&UnitTypeId::new("unit_crossbowmen").unwrap()];
    let mut unit = Unit::fresh(unit_type);
    unit.strength = 1000;
    unit.max_strength = 1000;
    state.armies.retain(|_, a| a.faction != crusaders);
    let id = state.allocate_army_id();
    state.armies.insert(
        id.clone(),
        Army::new(
            crusaders.clone(),
            ArmyPosition::Settlement(set(BASE)),
            vec![unit],
        ),
    );
    // 21 → 20 after the wear: still at the threshold, nobody leaves.
    set_fervor(&mut state, 21);
    end_of_turn(&mut state);
    assert_eq!(state.armies[&id].total_strength(), 1000);
    // 20 → 19: 5 % go home.
    let events = end_of_turn(&mut state);
    assert_eq!(state.armies[&id].total_strength(), 950);
    assert!(events.iter().any(|e| e.text_fr.contains("Débandade")));
    assert_eq!(
        crusade_view(&state, data(), &crusaders)
            .unwrap()
            .desertion_percent,
        5
    );
    // At 0 the share doubles.
    set_fervor(&mut state, 0);
    end_of_turn(&mut state);
    assert_eq!(state.armies[&id].total_strength(), 855);
    // Other factions' armies are untouched.
    let other: u32 = state
        .armies
        .values()
        .filter(|a| a.faction == fac(HOLDER))
        .map(Army::total_strength)
        .sum();
    end_of_turn(&mut state);
    let after: u32 = state
        .armies
        .values()
        .filter(|a| a.faction == fac(HOLDER))
        .map(Army::total_strength)
        .sum();
    assert_eq!(other, after);
}

#[test]
fn inert_without_rules_state_or_a_living_faction() {
    // Older save: no `crusade` field.
    let mut state = campaign();
    let mut json: serde_json::Value = serde_json::from_str(&state.save_json()).unwrap();
    assert!(json.as_object_mut().unwrap().remove("crusade").is_some());
    let mut old = CampaignState::load_json(&json.to_string()).expect("older save loads");
    assert!(old.crusade.is_none());
    assert!(end_of_turn(&mut old).is_empty());
    assert_eq!(alms(&old, data(), &fac(CRUSADERS)), 0);
    assert_eq!(
        preach_blocker(&old, data(), &fac(CRUSADERS)),
        Some(CrusadeError::NotCrusaders)
    );
    // The state survives a save, causes included.
    end_of_turn(&mut state);
    let reloaded = CampaignState::load_json(&state.save_json()).expect("save loads");
    assert_eq!(reloaded.crusade, state.crusade);
    // Dead faction: frozen.
    state.factions.get_mut(&fac(CRUSADERS)).unwrap().alive = false;
    let frozen = state.crusade.clone();
    assert!(end_of_turn(&mut state).is_empty());
    on_battle(
        &mut state,
        data(),
        &fac(HOLDER),
        &fac(CRUSADERS),
        &fac(HOLDER),
    );
    assert_eq!(state.crusade, frozen);
}
