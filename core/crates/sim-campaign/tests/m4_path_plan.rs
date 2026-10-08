//! Lot M4: path preview split by turn (`CampaignState::plan_path`) and the
//! march report of an attack (spec `docs/design/2026-09-24-mouvement-libre.md`
//! § 6), on the real data with an all-plain synthetic grid.
use data_model::test_support::fac;

use std::path::PathBuf;

use data_model::{GameData, MapRasters, NavGrid, IMPASSABLE, PLAIN_COST};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, OrderOutcome, StopReason};

fn data_with_grid(edit: impl FnOnce(&mut NavGrid)) -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let mut data = GameData::load(&root).expect("game data loads").0;
    let mut grid = NavGrid::uniform(2048, 2048, 2, 1.438, PLAIN_COST);
    edit(&mut grid);
    grid.refresh_min_cost();
    data.set_map_rasters(MapRasters {
        navgrid: grid,
        provinces: None,
    });
    data
}

fn px_per_km(data: &GameData) -> f32 {
    data.navgrid().px_per_km() as f32
}

fn distance_km(data: &GameData, a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() / px_per_km(data)
}

/// A point of central France with no settlement within `radius_km`.
fn empty_spot(data: &GameData, radius_km: f32) -> [f32; 2] {
    let settlements: Vec<[f32; 2]> = data
        .settlements
        .keys()
        .filter_map(|id| data.settlement_point(id))
        .collect();
    for y in (3280..3880).step_by(8) {
        for x in (1900..2500).step_by(8) {
            let p = [x as f32, y as f32];
            if settlements
                .iter()
                .all(|s| distance_km(data, *s, p) > radius_km)
            {
                return p;
            }
        }
    }
    panic!("no empty spot");
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

/// France and England at war, only their main armies kept, in the field.
fn duel(
    data: &GameData,
    french_at: [f32; 2],
    english_at: [f32; 2],
) -> (CampaignState, ArmyId, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.retain(|id, _| *id == french || *id == english);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(french_at);
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field(english_at);
    for id in [&french, &english] {
        let allowance = state.army_grid_allowance(data, &state.armies[id]);
        state.armies.get_mut(id).unwrap().movement_left = allowance;
    }
    (state, french, english)
}

#[test]
fn a_short_march_fits_in_one_turn() {
    let data = data_with_grid(|_| {});
    let start = empty_spot(&data, 25.0);
    let (state, french, _) = duel(&data, start, [10.0, 10.0]);
    let target = [start[0] + 40.0 * px_per_km(&data), start[1] + 10.0];
    let plan = state.plan_path(&data, &french, target).expect("reachable");
    assert_eq!(plan.points[0], start);
    assert_eq!(*plan.points.last().unwrap(), target);
    assert_eq!(plan.turns(), 1);
    assert!(plan.reachable_this_turn());
    assert_eq!(plan.stop_index(), plan.points.len() - 1);
    assert_eq!(plan.cost, plan.cost_this_turn);
    assert!(plan.cost <= state.armies[&french].movement_left);
}

#[test]
fn a_long_march_is_split_by_turn_and_matches_the_real_march() {
    let data = data_with_grid(|_| {});
    let start = empty_spot(&data, 25.0);
    let (mut state, french, _) = duel(&data, start, [10.0, 10.0]);
    // Lot DC1 (ADR 0082): about 105 km of plain a season.
    let target = [start[0], start[1] - 225.0 * px_per_km(&data)];
    let plan = state.plan_path(&data, &french, target).expect("reachable");
    assert_eq!(plan.turns(), 3, "225 km take three turns: {plan:?}");
    assert!(!plan.reachable_this_turn());
    assert!(plan.stop_index() > 0 && plan.stop_index() < plan.points.len() - 1);
    assert!(plan.turn_ends.windows(2).all(|w| w[0] < w[1]));
    assert!(plan.cost_this_turn <= state.armies[&french].movement_left);
    // The preview stops where the march really stops this turn.
    let stop = plan.points[plan.stop_index()];
    let report = match state
        .submit_order_outcome(&data, Order::move_to_point(french.clone(), target))
        .unwrap()
    {
        OrderOutcome::Moved(report) => report,
        other => panic!("expected a march, got {other:?}"),
    };
    assert_eq!(report.stop, StopReason::OutOfMovement);
    assert_eq!(report.cost, plan.cost_this_turn);
    let reached = state.army_point(&data, &state.armies[&french]);
    assert!(
        distance_km(&data, reached, stop) < 0.01,
        "{reached:?} vs {stop:?}"
    );
}

#[test]
fn an_unreachable_target_has_no_plan() {
    // A wall of water cuts the map in two.
    let data = data_with_grid(|grid| {
        for y in 0..grid.height {
            let index = (y * grid.width + 1000) as usize;
            grid.costs[index] = IMPASSABLE;
        }
    });
    let start = [1900.0, 2300.0];
    let (state, french, _) = duel(&data, start, [10.0, 10.0]);
    assert!(state.plan_path(&data, &french, [2100.0, 2300.0]).is_none());
    // Same side: fine.
    assert!(state.plan_path(&data, &french, [1950.0, 2300.0]).is_some());
}

#[test]
fn an_army_without_points_plans_from_next_turn() {
    let data = data_with_grid(|_| {});
    let start = empty_spot(&data, 25.0);
    let (mut state, french, _) = duel(&data, start, [10.0, 10.0]);
    state.armies.get_mut(&french).unwrap().movement_left = 0;
    let target = [start[0] + 40.0 * px_per_km(&data), start[1]];
    let plan = state.plan_path(&data, &french, target).expect("reachable");
    assert_eq!(plan.stop_index(), 0);
    assert_eq!(plan.cost_this_turn, 0);
    assert_eq!(plan.turns(), 2);
}

#[test]
fn an_attack_reports_the_march_and_the_engagement() {
    let data = data_with_grid(|_| {});
    let start = empty_spot(&data, 25.0);
    let (mut state, french, english) =
        duel(&data, start, [start[0] + 40.0 * px_per_km(&data), start[1]]);
    let outcome = state
        .submit_order_outcome(
            &data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            },
        )
        .unwrap();
    let OrderOutcome::Moved(report) = outcome else {
        panic!("an attack reports its march");
    };
    assert_eq!(report.stop, StopReason::Engaged { army: english });
    assert!(!report.walked.is_empty());
    assert!(report.cost > 0);
}
