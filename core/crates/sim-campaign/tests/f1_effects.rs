//! F1 « règles inertes » integration tests: building, technology and trait
//! effects that used to be displayed without effect, allied armies joining
//! battles, and the new chronicle effects (capture, ransom, delayed events).
//! See `docs/design/v2-finalisation.md` (lot F1).

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, ProvinceId, UnitTypeId};
use sim_campaign::{ArmyId, CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

fn unit(id: &str) -> UnitTypeId {
    UnitTypeId::new(id).unwrap()
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

/// France at spring 1337 without chronicle events (no random noise).
fn quiet_france(data: &GameData, seed: u64) -> CampaignState {
    let mut state = france(data, seed);
    state.chronicle.disabled = true;
    state
}

/// A planner that does nothing: only the orders the test submits apply.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn first_army_of(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction))
        .map(|(id, _)| id.clone())
        .expect("an army")
}

// =========================================================================
// 1. Buildings: Garrison, RecruitCost, Supply, class targeting
// =========================================================================

#[test]
fn class_targeted_building_effects_reach_only_their_class() {
    let data = data();
    let province = prov("prov_ile_de_france");
    let mut with = quiet_france(&data, 7);
    let mut without = with.clone();
    with.provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_guild_hall"));
    without
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .retain(|b| b != &bld("bld_guild_hall"));
    for _ in 0..4 {
        with.end_turn_with(&data, idle);
        without.end_turn_with(&data, idle);
    }
    let a = &with.provinces[&province].population;
    let b = &without.provinces[&province].population;
    assert!(
        a.burghers.wealth > b.burghers.wealth,
        "guild hall: burghers richer ({} vs {})",
        a.burghers.wealth,
        b.burghers.wealth
    );
    assert_eq!(a.peasants.wealth, b.peasants.wealth, "peasants untouched");
    assert_eq!(a.nobility.wealth, b.nobility.wealth, "nobility untouched");
}

#[test]
fn garrison_effect_makes_garrison_units_free() {
    let data = data();
    let mut state = quiet_france(&data, 3);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    let walls = ["bld_palisade", "bld_stone_walls", "bld_castle"].map(bld);
    let p = state.provinces.get_mut(&province).unwrap();
    assert!(!p.garrison.is_empty());
    p.buildings.retain(|b| !walls.contains(b));
    assert_eq!(state.province_effects(&data, &province).garrison.flat, 0.0);
    let before = state.faction_upkeep(&data, &france_id);
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_palisade"));
    let after = state.faction_upkeep(&data, &france_id);
    assert!(
        after < before,
        "palisade: one free unit ({before} -> {after})"
    );
}

#[test]
fn garrison_effect_reinforces_a_depleted_garrison() {
    let data = data();
    let mut state = quiet_france(&data, 4);
    let province = prov("prov_ile_de_france");
    let p = state.provinces.get_mut(&province).unwrap();
    assert!(!p.garrison.is_empty());
    p.buildings.push(bld("bld_castle"));
    for unit in &mut p.garrison {
        unit.strength = unit.max_strength / 2;
    }
    let before: u32 = p.garrison.iter().map(|u| u.strength).sum();
    state.end_turn_with(&data, idle);
    let after: u32 = state.provinces[&province]
        .garrison
        .iter()
        .map(|u| u.strength)
        .sum();
    assert!(after > before, "castle levies: {before} -> {after}");
}

#[test]
fn recruit_cost_effects_target_their_unit_family() {
    let data = data();
    let mut state = quiet_france(&data, 5);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    let cost = |state: &CampaignState, id: &str| {
        state
            .recruit_option(&data, &france_id, &province, &unit(id))
            .unwrap()
            .cost
    };
    let knights = cost(&state, "unit_knights");
    let militia = cost(&state, "unit_urban_militia");
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_stables"));
    // −10 % of the base price (percent effects add up).
    let base = data.unit_types[&unit("unit_knights")].cost.money;
    assert_eq!(cost(&state, "unit_knights"), knights - base / 10);
    assert_eq!(cost(&state, "unit_urban_militia"), militia);
    // The treasury pays the discounted price.
    let treasury = state.factions[&france_id].treasury;
    state
        .submit_order(
            &data,
            Order::Recruit {
                province: province.clone(),
                unit_type: unit("unit_knights"),
            },
        )
        .unwrap();
    assert_eq!(
        treasury - state.factions[&france_id].treasury,
        i64::from(cost(&state, "unit_knights"))
    );
}

#[test]
fn supply_buildings_speed_up_recovery_in_the_province() {
    let data = data();
    let base = quiet_france(&data, 6);
    let army = first_army_of(&base, "fac_france");
    let location = base.armies[&army].location.clone();
    let run = |port: bool| {
        let mut state = base.clone();
        state.armies.get_mut(&army).unwrap().supply = 10;
        let p = state.provinces.get_mut(&location).unwrap();
        p.buildings.retain(|b| b != &bld("bld_port"));
        if port {
            p.buildings.push(bld("bld_port"));
        }
        state.end_turn_with(&data, idle);
        state.armies[&army].supply
    };
    assert_eq!(run(true), run(false) + 10);
}
