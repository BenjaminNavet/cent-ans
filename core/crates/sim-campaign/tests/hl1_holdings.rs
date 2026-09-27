//! Lot HL1 (settlements quick-access list, touche B): `holdings_overview`.
//! See `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` § 1.

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId, SettlementKind};
use sim_campaign::holdings::holdings_overview;
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 7).expect("1337 start")
}

/// First settlement of `kind` owned and controlled by France, by id.
fn french_settlement(state: &CampaignState, kind: SettlementKind) -> SettlementId {
    let france = FactionId::new("fac_france").unwrap();
    state
        .settlements
        .iter()
        .find(|(_, s)| s.kind == kind && s.owner == france && s.controller == france)
        .map(|(id, _)| id.clone())
        .expect("France holds a settlement of this kind")
}

#[test]
#[ignore]
fn idle_settlement_with_enough_treasury_can_upgrade() {
    todo!()
}

#[test]
#[ignore]
fn idle_settlement_without_enough_treasury_has_no_options() {
    todo!()
}

#[test]
#[ignore]
fn settlement_under_construction_is_not_idle() {
    todo!()
}

#[test]
#[ignore]
fn besieged_settlement_has_no_income_and_is_endangered() {
    todo!()
}

#[test]
#[ignore]
fn occupied_settlement_is_listed_and_excluded_from_slots_total() {
    todo!()
}

#[test]
#[ignore]
fn province_income_is_the_sum_of_its_settlements() {
    todo!()
}

#[test]
#[ignore]
fn unknown_faction_has_an_empty_overview() {
    let data = data();
    let state = start(&data);
    let unknown = FactionId::new("fac_does_not_exist").unwrap();
    let overview = holdings_overview(&state, &data, &unknown);
    assert!(overview.provinces.is_empty());
    assert_eq!(overview.treasury, 0);
}
