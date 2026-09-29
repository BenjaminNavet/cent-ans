//! M8 § 2: interactive assaults become pending siege battles (3D or auto).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId};
use sim_battle::{BattleSim, SideId};
use sim_campaign::{ArmyId, CampaignState, Order, Stance};

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

/// The city of Guyenne (lot C4: sieges target settlements).
fn guyenne(state: &CampaignState) -> SettlementId {
    state
        .province_city_id(&prov("prov_guyenne"))
        .unwrap()
        .clone()
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

/// France's main army besieging English Guyenne (siege started).
fn besiege_guyenne(data: &GameData, seed: u64) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).unwrap();
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let city = guyenne(&state);
    let kent = state.province_city_id(&prov("prov_kent")).unwrap().clone();
    let english: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| {
            a.settlement().and_then(|s| state.settlement_province(s)) == Some(&prov("prov_guyenne"))
                && a.faction != fac("fac_france")
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in english {
        state.armies.get_mut(&id).unwrap().position =
            sim_campaign::ArmyPosition::Settlement(kent.clone());
    }
    let a = state.armies.get_mut(&army).unwrap();
    a.position = sim_campaign::ArmyPosition::Settlement(city);
    a.stance = Stance::Siege;
    a.clear_plan();
    state.end_turn_with(data, idle);
    assert!(state
        .settlement_state(&guyenne(&state))
        .unwrap()
        .siege
        .is_some());
    let place = guyenne(&state);
    ladders_ready(&mut state, data, &place);
    (state, army)
}

/// NT5 (N7): the besiegers' ladders are built (an assault behind standing
/// walls needs one ready engine).
fn ladders_ready(state: &mut CampaignState, data: &GameData, place: &SettlementId) {
    if let Some(siege) = state
        .settlements
        .get_mut(place)
        .and_then(|s| s.siege.as_mut())
    {
        siege.engine_work = siege
            .engine_work
            .max(data.siege_engine_rules.engines[0].work);
    }
}

#[test]
fn player_assault_waits_as_a_pending_siege_battle() {
    let data = data();
    let (mut state, army) = besiege_guyenne(&data, 4);
    let garrison = state
        .settlement_state(&guyenne(&state))
        .unwrap()
        .garrison
        .len();
    state
        .submit_order(&data, Order::Assault { army: army.clone() })
        .unwrap();
    // Asking twice does not stack two battles.
    state
        .submit_order(&data, Order::Assault { army: army.clone() })
        .unwrap();
    assert_eq!(state.pending_battles.len(), 1);
    assert!(state.pending_battles[0].siege);
    let views = state.pending_battle_views(&data);
    assert!(views[0].siege);
    assert_eq!(views[0].player_side, Some(SideId::Attacker));
    assert!(!views[0].defender_name.is_empty());
    let setup = state.battle_setup(&data, 0).unwrap();
    let siege = setup.siege.as_ref().expect("siege battle");
    assert_eq!(
        siege.fortification,
        state.fortification_level(&data, &guyenne(&state))
    );
    assert_eq!(setup.defender.units.len(), garrison);
    assert_eq!(setup.player_side, Some(SideId::Attacker));
    assert!(!setup.river);
    // The town is not taken yet.
    assert_eq!(
        state.settlement_state(&guyenne(&state)).unwrap().controller,
        fac("fac_england")
    );
}

#[test]
fn a_3d_siege_victory_takes_the_town() {
    let data = data();
    let (mut state, army) = besiege_guyenne(&data, 6);
    state
        .submit_order(&data, Order::Assault { army: army.clone() })
        .unwrap();
    let setup = state.battle_setup(&data, 0).unwrap();
    // Fight it with the battle AI on both sides; the garrison has lost
    // heart (the test is about applying the outcome, not the odds).
    let mut battle_setup = setup.clone();
    battle_setup.player_side = None;
    let mut sim = BattleSim::new(battle_setup, 1).unwrap();
    for unit in sim.units_mut() {
        if unit.side == SideId::Defender {
            unit.morale = 21.0;
            unit.morale_cap = 21.0;
        }
    }
    while !sim.is_finished() {
        sim.step();
    }
    let outcome = sim.outcome().unwrap();
    assert_eq!(outcome.winner, SideId::Attacker);
    assert_eq!(outcome.defender.losses.len(), setup.defender.units.len());
    let events = state.resolve_pending_battle(&data, 0, &outcome).unwrap();
    assert!(state.pending_battles.is_empty());
    let province = state.settlement_state(&guyenne(&state)).unwrap();
    assert_eq!(province.controller, fac("fac_france"));
    assert!(province.siege.is_none());
    assert!(events.iter().any(|e| e.text_fr.contains("Assaut")));
}

#[test]
fn pending_assaults_are_auto_resolved_and_saved() {
    let data = data();
    let (mut state, army) = besiege_guyenne(&data, 8);
    let before: u32 = state.armies[&army].units.iter().map(|u| u.strength).sum();
    state
        .submit_order(&data, Order::Assault { army: army.clone() })
        .unwrap();
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded.pending_battles, state.pending_battles);
    assert!(json.contains("\"siege\":true"));
    let events = state.auto_resolve_pending(&data, 0).unwrap();
    assert!(events.iter().any(|e| e.text_fr.contains("Assaut")));
    let taken = state.settlement_state(&guyenne(&state)).unwrap().controller == fac("fac_france");
    let after: u32 = state
        .armies
        .get(&army)
        .map_or(0, |a| a.units.iter().map(|u| u.strength).sum());
    assert!(taken || after < before);
    // Left pending, the next turn resolves it first.
    let (mut state, army) = besiege_guyenne(&data, 9);
    state.submit_order(&data, Order::Assault { army }).unwrap();
    let events = state.end_turn_with(&data, idle);
    assert!(state.pending_battles.iter().all(|r| !r.siege));
    assert!(events.iter().any(|e| e.text_fr.contains("Assaut")));
}
