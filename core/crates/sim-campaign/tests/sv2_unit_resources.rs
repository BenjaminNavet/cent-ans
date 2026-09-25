//! Lot SV2: the `cost.resources` of a unit (siege engines: wood, iron) are
//! drawn at recruitment from the faction's producing provinces and reserved
//! while the recruit trains; what is lacking is imported and paid (B7c rule,
//! ADR 0053).

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::{
    BuildingId, FactionId, GameData, ProvinceId, ResourceId, SettlementId, UnitTypeId,
};
use sim_campaign::state::QueuedRecruit;
use sim_campaign::{CampaignState, Order, OrderError};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn wood() -> ResourceId {
    ResourceId::new("res_wood").unwrap()
}

fn trebuchet() -> UnitTypeId {
    UnitTypeId::new("unit_trebuchet").unwrap()
}

/// France at Paris with the siege workshop, the engineering technology, a
/// full treasury and an empty recruitment queue.
fn setup(data: &GameData) -> (CampaignState, FactionId, SettlementId) {
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(data, france.clone(), 11).unwrap();
    let paris = state
        .province_city_id(&ProvinceId::new("prov_ile_de_france").unwrap())
        .unwrap()
        .clone();
    let unit = &data.unit_types[&trebuchet()];
    let f = state.factions.get_mut(&france).unwrap();
    if let Some(tech) = &unit.required_technology {
        f.technologies.insert(tech.clone());
    }
    f.treasury = 1_000_000;
    let settlement = state.settlements.get_mut(&paris).unwrap();
    settlement.recruit_queue.clear();
    let workshop = BuildingId::new("bld_siege_workshop").unwrap();
    if !settlement.buildings.contains(&workshop) {
        settlement.buildings.push(workshop);
    }
    (state, france, paris)
}

/// Reserves `amount` wood in a recruit of an earlier turn (no slot used).
fn reserve_wood(state: &mut CampaignState, settlement: &SettlementId, amount: u32) {
    state
        .settlements
        .get_mut(settlement)
        .unwrap()
        .recruit_queue
        .push(QueuedRecruit {
            unit_type: UnitTypeId::new("unit_mangonel").unwrap(),
            turns_left: 5,
            ordered_turn: 0,
            drawn: BTreeMap::from([(wood(), amount)]),
        });
}

fn free_wood(state: &CampaignState, data: &GameData, faction: &FactionId) -> u32 {
    state
        .free_supply(data, faction)
        .get(&wood())
        .copied()
        .unwrap_or(0)
}

/// Wood a trebuchet needs (from the data).
fn need(data: &GameData) -> u32 {
    data.unit_types[&trebuchet()].cost.resources[&wood()]
}

fn import_price(state: &CampaignState, data: &GameData, faction: &FactionId, units: u32) -> u32 {
    let livres = i64::from(data.resources[&wood()].base_price)
        * i64::from(data.economy_rules.resource_import_multiplier)
        * i64::from(units);
    sim_campaign::coinage::priced(state, faction, livres) as u32
}

#[test]
fn siege_engines_declare_resources() {
    let data = data();
    let (state, france, paris) = setup(&data);
    let option = state
        .recruit_option(&data, &france, &paris, &trebuchet())
        .unwrap();
    assert!(need(&data) >= 2, "a trebuchet needs wood");
    assert_eq!(option.resources.get(&wood()), Some(&need(&data)));
}

#[test]
fn wood_is_drawn_and_reserved_while_the_recruit_trains() {
    let data = data();
    let (mut state, france, paris) = setup(&data);
    let supply = free_wood(&state, &data, &france);
    let need = need(&data);
    assert!(supply >= need, "France reaches {supply} wood provinces");

    let option = state
        .recruit_option(&data, &france, &paris, &trebuchet())
        .unwrap();
    assert!(option.available, "{:?}", option.reason);
    assert_eq!(option.import_cost, 0);
    assert!(option.imported.is_empty());
    let unit = &data.unit_types[&trebuchet()];
    assert_eq!(
        option.cost,
        state.recruit_cost(&data, &france, &paris, unit),
        "no import: the money cost alone"
    );

    let before = state.factions[&france].treasury;
    state
        .submit_order(
            &data,
            Order::Recruit {
                settlement: paris.clone().into(),
                unit_type: trebuchet(),
            },
        )
        .unwrap();
    assert_eq!(
        state.factions[&france].treasury,
        before - i64::from(option.cost)
    );
    let queued = state.settlements[&paris]
        .recruit_queue
        .last()
        .unwrap()
        .clone();
    assert_eq!(queued.drawn.get(&wood()), Some(&need));
    assert_eq!(free_wood(&state, &data, &france), supply - need);

    // The unit joins the garrison: the wood is free again.
    state
        .settlements
        .get_mut(&paris)
        .unwrap()
        .recruit_queue
        .clear();
    assert_eq!(free_wood(&state, &data, &france), supply);
}

#[test]
fn missing_wood_is_imported_and_paid() {
    let data = data();
    let (mut state, france, paris) = setup(&data);
    let supply = free_wood(&state, &data, &france);
    // Only one unit of wood left: the trebuchet imports the rest.
    let missing = need(&data) - 1;
    reserve_wood(&mut state, &paris, supply - 1);
    let option = state
        .recruit_option(&data, &france, &paris, &trebuchet())
        .unwrap();
    assert!(option.available, "{:?}", option.reason);
    assert_eq!(option.imported.get(&wood()), Some(&missing));
    assert_eq!(
        option.import_cost,
        import_price(&state, &data, &france, missing)
    );
    let unit = &data.unit_types[&trebuchet()];
    assert_eq!(
        option.cost,
        state.recruit_cost(&data, &france, &paris, unit) + option.import_cost
    );
    let before = state.factions[&france].treasury;
    state
        .submit_order(
            &data,
            Order::Recruit {
                settlement: paris.clone().into(),
                unit_type: trebuchet(),
            },
        )
        .unwrap();
    assert_eq!(
        state.factions[&france].treasury,
        before - i64::from(option.cost)
    );
    let queued = state.settlements[&paris]
        .recruit_queue
        .last()
        .unwrap()
        .clone();
    assert_eq!(queued.drawn.get(&wood()), Some(&1), "only what was owned");
    assert_eq!(free_wood(&state, &data, &france), 0);
}

#[test]
fn a_treasury_short_of_the_import_refuses_the_recruit() {
    let data = data();
    let (mut state, france, paris) = setup(&data);
    let supply = free_wood(&state, &data, &france);
    reserve_wood(&mut state, &paris, supply);
    let unit = &data.unit_types[&trebuchet()];
    let money = state.recruit_cost(&data, &france, &paris, unit);
    // Enough for the engine, not for its imported wood.
    state.factions.get_mut(&france).unwrap().treasury = i64::from(money);
    let option = state
        .recruit_option(&data, &france, &paris, &trebuchet())
        .unwrap();
    assert!(!option.available);
    assert!(
        option
            .reason
            .as_deref()
            .is_some_and(|r| r.starts_with("trésor insuffisant")),
        "{:?}",
        option.reason
    );
    let refused = state.submit_order(
        &data,
        Order::Recruit {
            settlement: paris.clone().into(),
            unit_type: trebuchet(),
        },
    );
    assert!(
        matches!(refused, Err(OrderError::InsufficientFunds { needed, .. }) if needed == i64::from(option.cost)),
        "{refused:?}"
    );
    assert_eq!(state.factions[&france].treasury, i64::from(money));
}

#[test]
fn units_without_resources_are_unchanged() {
    let data = data();
    let (state, france, paris) = setup(&data);
    let militia = UnitTypeId::new("unit_urban_militia").unwrap();
    let option = state
        .recruit_option(&data, &france, &paris, &militia)
        .unwrap();
    assert!(option.resources.is_empty());
    assert_eq!(option.import_cost, 0);
    assert_eq!(
        option.cost,
        state.recruit_cost(&data, &france, &paris, &data.unit_types[&militia])
    );
}

#[test]
fn queue_entries_of_older_saves_hold_no_resources() {
    let entry: QueuedRecruit =
        serde_json::from_str(r#"{"unit_type":"unit_trebuchet","turns_left":2,"ordered_turn":4}"#)
            .unwrap();
    assert!(entry.drawn.is_empty());
    let back: QueuedRecruit = serde_json::from_str(
        &serde_json::to_string(&QueuedRecruit {
            drawn: BTreeMap::from([(wood(), 2)]),
            ..entry
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(back.drawn.get(&wood()), Some(&2));
}
