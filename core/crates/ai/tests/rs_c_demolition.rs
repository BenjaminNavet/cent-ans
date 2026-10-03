//! Lot RS-C: a realm whose buildings outgrew its income razes the least
//! useful after a prolonged deficit (`economy.json` `ai_demolition`).

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, SettlementKind};
use sim_campaign::{CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

/// The Swiss with a university (and its collegiate church) and a cathedral
/// in every city and town: far more upkeep than their income bears.
fn overbuilt_swiss(data: &GameData) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).expect("1337 start");
    state.chronicle.disabled = true;
    let swiss = fac("fac_swiss");
    for place in state
        .settlements
        .values_mut()
        .filter(|s| s.controller == swiss)
        .filter(|s| matches!(s.kind, SettlementKind::City | SettlementKind::Town))
    {
        place.buildings = vec![
            bld("bld_parish_church"),
            bld("bld_collegiate_church"),
            bld("bld_university"),
            // LR-04: the lord's demesne raised the Swiss income; a cathedral
            // keeps the upkeep far above what it bears.
            bld("bld_cathedral"),
        ];
        place.construction = None;
    }
    state
}

fn demolitions(orders: &[Order]) -> Vec<(String, BuildingId)> {
    orders
        .iter()
        .filter_map(|o| match o {
            Order::Demolish {
                settlement,
                building,
            } => Some((format!("{settlement:?}"), building.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_short_deficit_razes_nothing() {
    let data = data();
    let mut state = overbuilt_swiss(&data);
    let swiss = fac("fac_swiss");
    state.factions.get_mut(&swiss).unwrap().deficit_seasons =
        data.economy_rules.ai_demolition.deficit_seasons - 1;
    let orders = ai::campaign::plan_turn(&state, &data, &swiss);
    assert!(demolitions(&orders).is_empty());
}

#[test]
fn a_prolonged_deficit_razes_the_least_useful_building() {
    let data = data();
    let mut state = overbuilt_swiss(&data);
    let swiss = fac("fac_swiss");
    state.factions.get_mut(&swiss).unwrap().deficit_seasons =
        data.economy_rules.ai_demolition.deficit_seasons;
    let orders = ai::campaign::plan_turn(&state, &data, &swiss);
    let razed = demolitions(&orders);
    assert!(!razed.is_empty(), "the Swiss raze a building");
    assert!(razed.len() <= data.economy_rules.ai_demolition.max_per_turn);
    // Never a building another one needs (the collegiate church under the
    // university), and the order applies.
    assert!(razed
        .iter()
        .all(|(_, b)| *b != bld("bld_collegiate_church") && *b != bld("bld_parish_church")));
    let treasury = state.factions[&swiss].treasury;
    for order in orders
        .into_iter()
        .filter(|o| matches!(o, Order::Demolish { .. }))
    {
        state
            .apply_order(&data, &swiss, order)
            .expect("demolition applies");
    }
    assert!(
        state.factions[&swiss].treasury > treasury,
        "refund received"
    );
}

#[test]
fn modest_buildings_survive_a_deficit() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 5).expect("1337 start");
    state.chronicle.disabled = true;
    let swiss = fac("fac_swiss");
    for place in state
        .settlements
        .values_mut()
        .filter(|s| s.controller == swiss)
    {
        place.buildings.retain(|b| *b == bld("bld_parish_church"));
    }
    state.factions.get_mut(&swiss).unwrap().deficit_seasons = 40;
    let orders = ai::campaign::plan_turn(&state, &data, &swiss);
    assert!(demolitions(&orders).is_empty());
}
