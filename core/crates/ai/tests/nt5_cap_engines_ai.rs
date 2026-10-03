//! NT5 (ADR 0128): the campaign AI respects the army unit cap (N6) and
//! storms a walled place only once an engine is built (N7).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId, UnitTypeId};
use sim_campaign::{ArmyId, CampaignState, Order, Stance, Unit};

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

fn units(data: &GameData, unit_type: &str, n: usize) -> Vec<Unit> {
    let t = &data.unit_types[&UnitTypeId::new(unit_type).unwrap()];
    (0..n).map(|_| Unit::fresh(t)).collect()
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn city(state: &CampaignState, province: &str) -> SettlementId {
    state.province_city_id(&prov(province)).unwrap().clone()
}

fn french_armies(state: &CampaignState) -> Vec<ArmyId> {
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .collect()
}

#[test]
fn the_ai_forms_armies_within_the_cap() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 5).unwrap();
    let paris = city(&state, "prov_ile_de_france");
    state.settlements.get_mut(&paris).unwrap().garrison = units(&data, "unit_crossbowmen", 100);
    let orders = ai::plan_turn(&state, &data, &fac("fac_france"));
    let forming: Vec<usize> = orders
        .iter()
        .filter_map(|o| match o {
            Order::CreateArmy {
                settlement,
                units_from_garrison,
                ..
            } if *settlement == paris.clone().into() => Some(units_from_garrison.len()),
            _ => None,
        })
        .collect();
    assert!(forming.len() >= 2, "several armies: {forming:?}");
    assert!(forming.iter().all(|n| *n <= 40), "{forming:?}");
    // Applied in order, every order is accepted.
    for order in orders
        .iter()
        .filter(|o| matches!(o, Order::CreateArmy { .. }))
    {
        state
            .apply_order(&data, &fac("fac_france"), order.clone())
            .unwrap();
    }
    assert!(state.armies.values().all(|a| a.units.len() <= 40));
}

#[test]
fn the_ai_does_not_merge_beyond_the_cap() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 5).unwrap();
    let armies = french_armies(&state);
    let first = armies[0].clone();
    let place = state.armies[&first].settlement().unwrap().clone();
    state.armies.get_mut(&first).unwrap().units = units(&data, "unit_crossbowmen", 30);
    let mut second = state.armies[&first].clone();
    second.general = None;
    second.units = units(&data, "unit_crossbowmen", 22);
    let second_id = ArmyId::from_index(9_998);
    state.armies.insert(second_id.clone(), second);
    let orders = ai::plan_turn(&state, &data, &fac("fac_france"));
    let merges: Vec<(ArmyId, ArmyId)> = orders
        .iter()
        .filter_map(|o| match o {
            Order::MergeArmies { source, target } => Some((source.clone(), target.clone())),
            _ => None,
        })
        .collect();
    assert!(
        !merges
            .iter()
            .any(|(s, t)| [s, t].contains(&&first) && [s, t].contains(&&second_id)),
        "30 + 22 > 40 at {place}: {merges:?}"
    );
}

#[test]
fn the_ai_storms_once_an_engine_is_ready() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 4).unwrap();
    let army = french_armies(&state)[0].clone();
    let target = city(&state, "prov_guyenne");
    let kent = city(&state, "prov_kent");
    let english: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| a.faction != fac("fac_france"))
        .filter(|(_, a)| {
            a.settlement().and_then(|s| state.settlement_province(s)) == Some(&prov("prov_guyenne"))
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in english {
        state.armies.get_mut(&id).unwrap().position =
            sim_campaign::ArmyPosition::Settlement(kent.clone());
    }
    let a = state.armies.get_mut(&army).unwrap();
    a.position = sim_campaign::ArmyPosition::Settlement(target.clone());
    a.stance = Stance::Siege;
    a.units = units(&data, "unit_knights", 20);
    a.clear_plan();
    state.settlements.get_mut(&target).unwrap().garrison = units(&data, "unit_urban_militia", 1);
    state.end_turn_with(&data, idle);
    assert!(state.settlement_state(&target).unwrap().siege.is_some());
    let assaults = |state: &CampaignState| {
        ai::plan_turn(state, &data, &fac("fac_france"))
            .iter()
            .any(|o| matches!(o, Order::Assault { army: a } if *a == army))
    };
    assert!(state.assault_blocker(&data, &army).is_some());
    assert!(!assaults(&state), "no assault without an engine");
    // A6-L2: the ladders cost more against high walls.
    let ladders = data.siege_engine_rules.engines[0].cost(
        data.siege_engine_rules.scaling_min_wall_level,
        state.fortification_level(&data, &target),
    );
    state
        .settlements
        .get_mut(&target)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .engine_work = ladders;
    assert!(state.assault_blocker(&data, &army).is_none());
    assert!(assaults(&state), "ladders ready, odds good: assault");
}
