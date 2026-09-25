//! NV1: naval war on the campaign map — interception of crossings, pending
//! naval battles, ships taken and sunk, control of the sea, blockade.

use std::path::PathBuf;

use data_model::{FactionId, GameData, SeaZoneId, SettlementId};
use sim_battle::naval::{NavalSim, ShipFate};
use sim_battle::SideId;
use sim_campaign::naval::SeaControl;
use sim_campaign::{ArmyId, CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn channel() -> SeaZoneId {
    SeaZoneId::new("sea_channel").unwrap()
}

/// England at spring 1337, its main army in a port with a crossing to
/// France, the French holding the Channel.
fn crossing(data: &GameData, seed: u64) -> (CampaignState, ArmyId, SettlementId, SettlementId) {
    let mut state = CampaignState::new_1337(data, fac("fac_england"), seed).unwrap();
    let (from, to) = data
        .movement_graph
        .adjacency
        .iter()
        .flat_map(|(from, edges)| edges.iter().map(move |e| (from, e)))
        .filter(|(_, e)| e.sea)
        .find(|(from, edge)| {
            state.is_friendly_settlement(&fac("fac_england"), from)
                && state
                    .settlement_province(&edge.to)
                    .is_some_and(|p| p.as_str() == "prov_boulonnais")
        })
        .map(|(from, edge)| (from.clone(), edge.to.clone()))
        .expect("a Channel crossing");
    let army = state
        .armies()
        .iter()
        .filter(|(_, a)| a.faction == fac("fac_england"))
        .max_by_key(|(id, a)| (a.total_strength(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .unwrap();
    state.armies.get_mut(&army).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(from.clone());
    state.naval.ensure(data);
    state.naval.control.insert(
        channel(),
        SeaControl {
            faction: fac("fac_france"),
            level: 100,
        },
    );
    assert!(state.is_at_war(&fac("fac_england"), &fac("fac_france")));
    (state, army, from, to)
}

/// Crosses until a French squadron intercepts (the chance is 70 %).
fn intercepted(data: &GameData) -> (CampaignState, ArmyId, SettlementId, SettlementId) {
    for seed in 1..40 {
        let (mut state, army, from, to) = crossing(data, seed);
        state
            .submit_order(
                data,
                Order::Embark {
                    army: army.clone(),
                    to_port: to.clone(),
                },
            )
            .unwrap();
        if !state.naval.pending.is_empty() {
            return (state, army, from, to);
        }
    }
    panic!("no interception in 40 seeds");
}

#[test]
fn an_intercepted_player_crossing_waits_in_port() {
    let data = data();
    let (state, army, from, _) = intercepted(&data);
    let a = state.army(&army).unwrap();
    assert!(a.is_at(&from), "the army waits in port");
    assert_eq!(a.movement_left, 0);
    let views = state.pending_naval_views(&data);
    assert_eq!(views.len(), 1);
    let view = &views[0];
    assert_eq!(view.interceptor, fac("fac_france"));
    assert_eq!(view.player_side, SideId::Defender);
    assert_eq!(view.sea_name, "la Manche");
    assert!(view.interceptor_ships > 0 && view.transport_ships > 0);
    assert_eq!(view.army_men, a.total_strength());
    // The setup plays in the real-time battle.
    let setup = state.naval_battle_setup(&data, 0).unwrap();
    assert_eq!(setup.defender.units.len(), a.units.len());
    assert_eq!(setup.defender.men(), a.total_strength());
    let mut sim = NavalSim::new(setup, 1).unwrap();
    sim.set_ai(SideId::Defender, true);
    sim.advance(10.0);
    assert!(!sim.ships.is_empty());
}

#[test]
fn withdrawing_keeps_the_army_home() {
    let data = data();
    let (mut state, army, from, _) = intercepted(&data);
    let before = state.army(&army).unwrap().total_strength();
    state.withdraw_naval_battle(&data, 0).unwrap();
    assert!(state.naval.pending.is_empty());
    let a = state.army(&army).unwrap();
    assert!(a.is_at(&from));
    assert_eq!(a.total_strength(), before);
}

#[test]
fn a_naval_battle_moves_ships_losses_and_the_sea() {
    let data = data();
    let (mut state, army, from, to) = intercepted(&data);
    let before_men = state.army(&army).unwrap().total_strength();
    let setup = state.naval_battle_setup(&data, 0).unwrap();
    let english_before = state.naval.ships_of(&fac("fac_england"));
    let french_before = state.naval.ships_of(&fac("fac_france"));
    let mut sim = NavalSim::new(setup, state.naval_battle_seed(0).unwrap()).unwrap();
    sim.set_ai(SideId::Defender, true);
    let outcome = sim.run_to_end();
    state.resolve_naval_battle(&data, 0, &outcome).unwrap();
    assert!(state.naval.pending.is_empty());
    let lost: u32 = outcome.defender.unit_losses.iter().sum();
    match state.army(&army) {
        Some(a) => {
            // Landing on a hostile shore costs more men after the fight.
            assert!(a.total_strength() + lost <= before_men);
            if outcome.winner == Some(SideId::Attacker) {
                assert!(a.is_at(&from), "beaten back to port");
            } else {
                assert!(a.is_at(&to) || !a.is_at(&from), "the crossing goes on");
            }
        }
        None => assert!(lost >= before_men),
    }
    // Ships taken change pools; the winner holds the Channel.
    let english_taken =
        outcome.defender.count(ShipFate::Captured) + outcome.defender.count(ShipFate::Sunk);
    let english_prizes = outcome.defender.prizes.len();
    let french_prizes = outcome.attacker.prizes.len();
    if english_taken > 0 || english_prizes > 0 {
        assert!(
            state.naval.ships_of(&fac("fac_england")) <= english_before + english_prizes as u32
        );
    }
    assert!(state.naval.ships_of(&fac("fac_france")) <= french_before + french_prizes as u32);
    if outcome.winner == Some(SideId::Defender) {
        let control = &state.naval.control[&channel()];
        assert_eq!(control.level, 60, "100 French − 40");
    }
}

#[test]
fn auto_resolve_of_a_pending_naval_battle() {
    let data = data();
    let (mut state, _, _, _) = intercepted(&data);
    let events = state.auto_resolve_naval_battle(&data, 0).unwrap();
    assert!(events
        .iter()
        .any(|e| e.text_fr.contains("Bataille navale dans la Manche")));
    assert!(state.naval.pending.is_empty());
}

#[test]
fn ai_crossings_are_fought_at_once() {
    let data = data();
    let mut fought = false;
    for seed in 1..40 {
        let (mut state, army, _, to) = crossing(&data, seed);
        state.interactive_battles = false;
        state
            .submit_order(
                &data,
                Order::Embark {
                    army: army.clone(),
                    to_port: to.clone(),
                },
            )
            .unwrap();
        assert!(state.naval.pending.is_empty());
        if state
            .pending_events
            .iter()
            .any(|e| e.text_fr.starts_with("Bataille navale"))
        {
            fought = true;
            break;
        }
    }
    assert!(fought);
}

#[test]
fn a_held_sea_blockades_enemy_ports_and_fades() {
    let data = data();
    let (mut state, _, _, _) = crossing(&data, 3);
    let treasury = state.factions[&fac("fac_england")].treasury;
    let mut events = Vec::new();
    state
        .naval
        .fleets
        .get_mut(&fac("fac_france"))
        .unwrap()
        .clear();
    let idle = |_: &CampaignState, _: &GameData, _: &FactionId| Vec::new();
    events.extend(state.end_turn_with(&data, idle));
    assert!(
        !state.naval.blockaded.is_empty(),
        "English Channel ports blockaded"
    );
    assert!(events.iter().any(|e| e.text_fr.starts_with("Blocus")));
    let _ = treasury;
    assert_eq!(state.naval.control[&channel()].level, 90);
    // The French shipyards rebuild.
    assert!(state.naval.ships_of(&fac("fac_france")) > 0);
}

#[test]
fn older_saves_without_naval_state_load() {
    let data = data();
    let (state, _, _, _) = crossing(&data, 1);
    let mut json = serde_json::to_value(&state).unwrap();
    json.as_object_mut().unwrap().remove("naval");
    let back: CampaignState = serde_json::from_value(json).unwrap();
    assert!(!back.naval.initialised);
}
