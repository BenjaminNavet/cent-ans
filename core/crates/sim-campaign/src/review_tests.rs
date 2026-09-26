//! Regression tests for the fixes of the 2026-09-26 code review
//! (`docs/wip/revue-code.md`, section « Corrections sim-campaign »).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId};

use crate::battle_auto::{BattleResult, SideOutcome, Winner};
use crate::state::{ArmyId, ArmyPosition, CampaignState, Stance};

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

fn city(state: &CampaignState, province: &str) -> SettlementId {
    state.province_city_id(&prov(province)).unwrap().clone()
}

/// The first army of `faction` led by a general.
fn led_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction) && a.general.is_some())
        .map(|(id, _)| id.clone())
        .expect("a led army")
}

/// `army` besieges `place` (siege stance, siege state begun).
fn besiege(state: &mut CampaignState, data: &GameData, army: &ArmyId, place: &SettlementId) {
    let a = state.armies.get_mut(army).unwrap();
    a.position = ArmyPosition::Settlement(place.clone());
    a.stance = Stance::Siege;
    a.clear_plan();
    let faction = a.faction.clone();
    let mut events = Vec::new();
    crate::siege::begin_siege(state, data, place, &faction, army, &mut events);
}

fn outcome(units: usize, general_captured: bool) -> SideOutcome {
    SideOutcome {
        power: 1.0,
        losses: vec![0; units],
        total_losses: 0,
        morale_delta: 0,
        routed: false,
        general_captured,
    }
}

/// Fix 1: a general taken in a failed assault is held by the defender.
#[test]
fn general_captured_in_assault_has_a_captor() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let army = led_army(&state, "fac_france");
    let general = state.armies[&army].general.clone().unwrap();
    let guyenne = city(&state, "prov_guyenne");
    besiege(&mut state, &data, &army, &guyenne);
    let controller = state.settlements[&guyenne].controller.clone();
    let units = state.armies[&army].units.len();
    let garrison = state.settlements[&guyenne].garrison.len();
    let result = BattleResult {
        winner: Winner::Defender,
        attacker: outcome(units, true),
        defender: outcome(garrison, false),
    };
    let mut events = Vec::new();
    crate::siege::apply_assault_result(
        &mut state,
        &data,
        std::slice::from_ref(&army),
        &guyenne,
        &result,
        true,
        &mut events,
    );
    let c = &state.characters[&general];
    assert!(c.captive);
    assert_eq!(c.captor.as_ref(), Some(&controller));
}

/// Fix 2: a besieged garrison cannot form an army (it would lift the siege).
#[test]
fn besieged_garrison_cannot_create_an_army() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let army = led_army(&state, "fac_france");
    let guyenne = city(&state, "prov_guyenne");
    besiege(&mut state, &data, &army, &guyenne);
    let controller = state.settlements[&guyenne].controller.clone();
    assert!(!state.settlements[&guyenne].garrison.is_empty());
    let order = crate::orders::Order::CreateArmy {
        settlement: crate::orders::Place::Settlement(guyenne.clone()),
        units_from_garrison: vec![0],
        general: None,
    };
    assert_eq!(
        state.apply_order(&data, &controller, order),
        Err(crate::orders::OrderError::SettlementBesieged)
    );
}
