//! Lot RS-N: read-only demolition preview (`buildings::demolition_preview`)
//! for the UI's « Raser » button, on the real data of 1337.

use data_model::{BuildingId, GameData, SettlementId};

use crate::buildings::demolition_preview;
use crate::state::CampaignState;

use data_model::test_support::{fac, game_data};

fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

fn campaign(data: &GameData) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    state.chronicle.disabled = true;
    state
}

/// A settlement of `faction` with `buildings` and no construction nor siege.
fn place_with(state: &mut CampaignState, faction: &str, buildings: &[&str]) -> SettlementId {
    let faction = fac(faction);
    let id = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == faction && s.siege.is_none())
        .map(|(id, _)| id.clone())
        .expect("a settlement");
    let place = state.settlements.get_mut(&id).unwrap();
    place.buildings = buildings.iter().map(|b| bld(b)).collect();
    place.construction = None;
    place.siege = None;
    id
}

#[test]
fn a_free_standing_building_can_be_razed() {
    let data = game_data();
    let mut state = campaign(data);
    let place = place_with(&mut state, "fac_france", &["bld_market", "bld_apothecary"]);
    let preview = demolition_preview(&state, data, &place, &bld("bld_apothecary"));
    assert!(preview.can_demolish);
    assert!(preview.reason.is_none());
    let apothecary = &data.buildings[&bld("bld_apothecary")];
    let expected_refund = i64::from(apothecary.cost.money)
        * i64::from(data.economy_rules.demolition_refund_percent)
        / 100;
    assert_eq!(preview.refund, expected_refund);
    assert!(preview.refund > 0);
    assert_eq!(
        preview.upkeep_saved,
        i64::from(apothecary.upkeep.unwrap_or(0))
    );
}

#[test]
fn a_dependency_blocks_the_preview_but_still_gives_the_refund() {
    let data = game_data();
    let mut state = campaign(data);
    let place = place_with(&mut state, "fac_france", &["bld_market", "bld_apothecary"]);
    let preview = demolition_preview(&state, data, &place, &bld("bld_market"));
    assert!(!preview.can_demolish);
    assert!(preview.reason.is_some());
    let market = &data.buildings[&bld("bld_market")];
    let expected_refund = i64::from(market.cost.money)
        * i64::from(data.economy_rules.demolition_refund_percent)
        / 100;
    assert_eq!(preview.refund, expected_refund);
}

#[test]
fn a_besieged_settlement_blocks_the_preview() {
    let data = game_data();
    let mut state = campaign(data);
    let place = place_with(&mut state, "fac_france", &["bld_market"]);
    let turn = state.turn;
    state.settlements.get_mut(&place).unwrap().siege = Some(crate::state::SiegeState {
        attacker: fac("fac_england"),
        turns_left: 3,
        turns_elapsed: 1,
        supplies: 10,
        breach: 0,
        started_turn: turn,
        engine_work: 0,
    });
    let preview = demolition_preview(&state, data, &place, &bld("bld_market"));
    assert!(!preview.can_demolish);
    assert_eq!(preview.reason.as_deref(), Some("la colonie est assiégée"));
}

#[test]
fn the_preview_and_the_actual_demolition_agree_on_the_refund() {
    let data = game_data();
    let mut state = campaign(data);
    let place = place_with(&mut state, "fac_france", &["bld_market", "bld_apothecary"]);
    let preview = demolition_preview(&state, data, &place, &bld("bld_apothecary"));
    let treasury = state.factions[&fac("fac_france")].treasury;
    state
        .submit_order(
            data,
            crate::orders::Order::Demolish {
                settlement: place.clone().into(),
                building: bld("bld_apothecary"),
            },
        )
        .unwrap();
    assert_eq!(
        state.factions[&fac("fac_france")].treasury,
        treasury + preview.refund
    );
}
