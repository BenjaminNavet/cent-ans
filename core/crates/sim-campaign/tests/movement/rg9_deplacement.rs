//! Lot RG9 (ADR 0343): roads of the provinces, passes and winter, the
//! general's experience, army skills and river navigation.
use std::path::PathBuf;

use data_model::test_support::fac;
use data_model::{
    FreeMovementRules, GameData, MapRasters, NavGrid, ProvinceId, ProvinceRaster, SettlementId,
    SettlementKind, PLAIN_COST,
};
use sim_campaign::march::px_per_km;
use sim_campaign::state::Season;
use sim_campaign::terrain_mods::{effective_cost, pass_cells};
use sim_campaign::test_support::{bld, main_army};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, OrderOutcome, StopReason};

const GRID: u32 = 2048;
const SCALE: u32 = 2;

/// Real data with an all-plain grid and one province covering the whole map.
fn data_with(edit: impl FnOnce(&mut NavGrid), province: &ProvinceId) -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let mut data = GameData::load(&root).expect("game data loads").0;
    let mut grid = NavGrid::uniform(GRID, GRID, SCALE, 1.438, PLAIN_COST);
    edit(&mut grid);
    grid.refresh_min_cost();
    let side = (GRID * SCALE) as usize;
    let raster = ProvinceRaster {
        width: side as u32,
        height: side as u32,
        indices: vec![1; side * side],
        ids: vec![None, Some(province.clone())],
    };
    data.set_map_rasters(MapRasters {
        navgrid: grid,
        provinces: Some(raster),
    });
    data
}

fn rules(data: &GameData) -> &FreeMovementRules {
    data.free_movement_rules()
}

/// France and England at war, only their main armies kept, in the field.
fn duel(data: &GameData, at: [f32; 2]) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.retain(|id, _| *id == french || *id == english);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(at);
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field([10.0, 10.0]);
    refill(&mut state, data, &french);
    (state, french)
}

fn refill(state: &mut CampaignState, data: &GameData, army: &ArmyId) {
    let allowance = state.army_grid_allowance(data, &state.armies[army]);
    state.armies.get_mut(army).unwrap().movement_left = allowance;
}

fn some_province(data: &GameData) -> ProvinceId {
    let state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state
        .settlements
        .values()
        .find(|s| s.kind == SettlementKind::City)
        .map(|s| s.province.clone())
        .expect("a city")
}

fn build_in(state: &mut CampaignState, province: &ProvinceId, building: &str) {
    let settlement = state
        .settlements
        .values_mut()
        .find(|s| &s.province == province && s.kind == SettlementKind::City)
        .expect("city of the province");
    settlement
        .buildings
        .retain(|b| !b.as_str().ends_with("_road"));
    settlement.buildings.push(bld(building));
}

/// A point of the plain, away from every settlement, and a target 30 cells east.
fn march_line(data: &GameData) -> ([f32; 2], [f32; 2]) {
    let grid = data.navgrid();
    let start = grid.cell_center(900, 1200);
    let target = grid.cell_center(930, 1200);
    (start, target)
}

#[test]
fn road_buildings_scale_the_cost_of_the_province() {
    let province = {
        let data = GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data"))
            .unwrap()
            .0;
        some_province(&data)
    };
    let data = data_with(|_| {}, &province);
    let (start, target) = march_line(&data);
    let (mut state, french) = duel(&data, start);
    let base = state.plan_path(&data, &french, target).expect("path");
    assert_eq!(base.cost, 300, "30 plain cells");
    assert_eq!(base.breakdown.route, 0);

    build_in(&mut state, &province, "bld_gravel_road");
    let gravel = state.plan_path(&data, &french, target).expect("path");
    assert_eq!(gravel.cost, 30 * 9, "x0.85 rounds 10 to 9 per cell");
    assert_eq!(gravel.breakdown.terrain, 300);
    assert_eq!(gravel.breakdown.route, -30);
    assert_eq!(gravel.breakdown.total(), gravel.cost);

    build_in(&mut state, &province, "bld_royal_road");
    let royal = state.plan_path(&data, &french, target).expect("path");
    assert_eq!(royal.cost, 30 * 7, "x0.7");
    assert_eq!(royal.breakdown.route, -90);

    // The real march spends what the preview announced.
    let report = match state
        .submit_order_outcome(&data, Order::move_to_point(french.clone(), target))
        .unwrap()
    {
        OrderOutcome::Moved(report) => report,
        other => panic!("expected a march, got {other:?}"),
    };
    assert_eq!(report.stop, StopReason::Arrived);
    assert_eq!(report.cost, royal.cost);
}

#[test]
fn road_cost_stacks_with_baked_roads_down_to_the_floor() {
    let data = data_with(
        |_| {},
        &some_province(
            &GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data"))
                .unwrap()
                .0,
        ),
    );
    let rules = rules(&data);
    // A baked road cell (7) under a royal road (x0.7): 5, never below half.
    assert_eq!(effective_cost(rules, 7, 0.7, false, 1.0), 5);
    // Pass (x0.7) under a royal road (x0.7) = 0.49, held at the 0.5 floor.
    assert_eq!(effective_cost(rules, 30, 0.7, true, 1.0), 15);
    // No road: the cost is untouched.
    assert_eq!(effective_cost(rules, 18, 1.0, false, 1.0), 18);
    // Impassable stays so.
    assert_eq!(effective_cost(rules, 255, 0.7, true, 1.5), 255);
}

#[test]
fn passes_are_cheaper_and_winter_hits_mountains_and_passes() {
    let province = some_province(
        &GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data"))
            .unwrap()
            .0,
    );
    // Everything is mountain (30), as around a baked pass.
    let data = data_with(
        |grid| {
            grid.costs.fill(30);
        },
        &province,
    );
    let corridor = pass_cells(&data, data.navgrid());
    assert!(
        !corridor.is_empty(),
        "the passes of crossings.json land on the grid"
    );
    let on_pass = corridor[corridor.len() / 2];
    let off_pass = (0..data.navgrid().costs.len())
        .find(|i| corridor.binary_search(i).is_err())
        .unwrap();

    let (mut state, french) = duel(&data, data.navgrid().cell_center(10, 10));
    let cost_at = |state: &CampaignState, index: usize| -> u8 {
        let army = &state.armies[&french];
        state.army_navgrid(&data, army).costs[index]
    };
    // Summer: the pass costs 0.7 of the mountain, the mountain stays 30.
    state.season = Season::Summer;
    assert_eq!(cost_at(&state, on_pass), 21);
    assert_eq!(cost_at(&state, off_pass), 30);
    // Winter: x1.5 on both.
    state.season = Season::Winter;
    assert_eq!(cost_at(&state, off_pass), 45);
    assert_eq!(cost_at(&state, on_pass), 32, "30 x 0.7 x 1.5 = 31.5");

    // Montagnard: no winter malus.
    let general = state.armies[&french].general.clone().expect("a general");
    state
        .characters
        .get_mut(&general)
        .unwrap()
        .skills_learned
        .insert(data_model::SkillId::new("skill_montagnard").unwrap());
    assert_eq!(cost_at(&state, off_pass), 30);
    assert_eq!(cost_at(&state, on_pass), 21);
}

#[test]
fn the_generals_level_adds_movement_up_to_the_cap() {
    let province = some_province(
        &GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data"))
            .unwrap()
            .0,
    );
    let data = data_with(|_| {}, &province);
    let (mut state, french) = duel(&data, data.navgrid().cell_center(900, 1200));
    let general = state.armies[&french].general.clone().expect("a general");
    let set_level = |state: &mut CampaignState, level: u32| {
        let c = state.characters.get_mut(&general).unwrap();
        c.skills_learned.clear();
        c.skill_points = level - 1;
    };
    let allowance =
        |state: &CampaignState| state.army_movement_allowance(&data, &state.armies[&french]);
    set_level(&mut state, 1);
    let level_one = f64::from(allowance(&state));
    set_level(&mut state, 6);
    let level_six = f64::from(allowance(&state));
    set_level(&mut state, 40);
    let capped = f64::from(allowance(&state));
    let r = rules(&data);
    assert!((level_six / level_one - (1.0 + 5.0 * r.general_level_percent / 100.0)).abs() < 0.01);
    assert!((capped / level_one - (1.0 + r.general_level_cap_percent / 100.0)).abs() < 0.01);
}

#[test]
fn the_scout_skill_adds_ten_percent_of_movement() {
    let province = some_province(
        &GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data"))
            .unwrap()
            .0,
    );
    let data = data_with(|_| {}, &province);
    let (mut state, french) = duel(&data, data.navgrid().cell_center(900, 1200));
    let general = state.armies[&french].general.clone().expect("a general");
    {
        let c = state.characters.get_mut(&general).unwrap();
        c.skills_learned.clear();
        c.skill_points = 0;
    }
    let before = f64::from(state.army_movement_allowance(&data, &state.armies[&french]));
    state
        .characters
        .get_mut(&general)
        .unwrap()
        .skills_learned
        .insert(data_model::SkillId::new("skill_eclaireur").unwrap());
    // The skill point it cost counts as a level too: compare on the skill alone.
    let level_bonus = state.general_level_percent(&data, &state.armies[&french]);
    let after = f64::from(state.army_movement_allowance(&data, &state.armies[&french]));
    let expected = before * (1.0 + (10.0 + level_bonus) / 100.0);
    assert!((after - expected).abs() <= 1.0, "{after} vs {expected}");
}

fn riverside_state() -> (GameData, CampaignState, ArmyId) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("game data loads").0;
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 5).unwrap();
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    state.armies.retain(|id, _| *id == french);
    let paris = SettlementId::new("set_paris").unwrap();
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::Settlement(paris);
    // Peace with everyone for the voyage.
    let allowance = state.army_grid_allowance(&data, &state.armies[&french]);
    state.armies.get_mut(&french).unwrap().movement_left = allowance;
    (data, state, french)
}

#[test]
fn rivers_are_cheaper_downstream_and_the_batelier_helps() {
    let (data, mut state, french) = riverside_state();
    let rouen = SettlementId::new("set_rouen").unwrap();
    let troyes = SettlementId::new("set_troyes").unwrap();
    for place in [&rouen, &troyes] {
        let faction = fac("fac_france");
        state.settlements.get_mut(place).unwrap().controller = faction;
    }
    let voyages = state.river_voyages_all(&data, &french);
    let down = voyages.get(&rouen).expect("Paris to Rouen by the Seine");
    let up = voyages.get(&troyes).expect("Paris to Troyes by the Seine");
    assert!(down.downstream && !up.downstream);
    assert_eq!(down.river, "riv_seine");
    let per_km_down = f64::from(down.cost) / down.km;
    let per_km_up = f64::from(up.cost) / up.km;
    let r = rules(&data);
    assert!(
        (per_km_up / per_km_down - r.river_upstream_factor / r.river_downstream_factor).abs()
            < 0.02
    );
    assert!(down.cost < down.road_cost);

    // Batelier: -25 %.
    let general = state.armies[&french].general.clone().expect("a general");
    state
        .characters
        .get_mut(&general)
        .unwrap()
        .skills_learned
        .insert(data_model::SkillId::new("skill_batelier").unwrap());
    let boatman = state.river_voyages_all(&data, &french)[&rouen].cost;
    assert!((f64::from(boatman) / f64::from(down.cost) - 0.75).abs() < 0.01);

    // Too far for the points left: refused, nothing changes.
    state.armies.get_mut(&french).unwrap().movement_left = boatman - 1;
    let sail = Order::SailRiver {
        army: french.clone(),
        to: rouen.clone(),
    };
    assert!(matches!(
        state.submit_order_outcome(&data, sail.clone()),
        Err(sim_campaign::OrderError::RiverTooFar { .. })
    ));
    assert!(!state.armies[&french].is_at(&rouen));
    // With the exact points: the order spends them and moves the army.
    state.armies.get_mut(&french).unwrap().movement_left = boatman + 7;
    let outcome = state.submit_order_outcome(&data, sail).expect("sailing");
    assert!(matches!(outcome, OrderOutcome::Moved(_)));
    assert!(state.armies[&french].is_at(&rouen));
    assert_eq!(state.armies[&french].movement_left, 7);
}

#[test]
fn an_enemy_place_bars_the_river_and_other_rivers_do_not_connect() {
    let (data, mut state, french) = riverside_state();
    let rouen = SettlementId::new("set_rouen").unwrap();
    let saint_denis = SettlementId::new("set_saint_denis").unwrap();
    state.settlements.get_mut(&saint_denis).unwrap().controller = fac("fac_england");
    state.settlements.get_mut(&rouen).unwrap().controller = fac("fac_france");
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    let voyages = state.river_voyages_all(&data, &french);
    assert!(
        !voyages.contains_key(&saint_denis),
        "cannot enter a hostile place"
    );
    assert!(!voyages.contains_key(&rouen), "cannot pass a hostile place");
    // Orleans is on the Loire, not on the Seine.
    assert!(!voyages.contains_key(&SettlementId::new("set_orleans").unwrap()));
    let err = state.submit_order_outcome(
        &data,
        Order::SailRiver {
            army: french.clone(),
            to: rouen,
        },
    );
    assert!(err.is_err());
}

#[test]
fn every_river_place_is_a_known_settlement_in_downstream_order() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("game data loads").0;
    // CC C3: 9 rivers survive (those with fewer than two places were dropped).
    assert!(data.navigable_rivers.rivers.len() >= 9);
    for river in &data.navigable_rivers.rivers {
        assert!(river.places.len() >= 2, "{}", river.id);
        for pair in river.places.windows(2) {
            assert!(pair[0].km < pair[1].km, "{} order", river.id);
        }
        for place in &river.places {
            assert!(
                data.settlements.contains_key(&place.settlement),
                "{} {}",
                river.id,
                place.settlement
            );
        }
    }
    let _ = px_per_km(&data);
}
