//! Lot M2: free army movement on the navigation grid (spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 3 and § 7).
//!
//! The rules run on the real game data, but most tests swap the navigation
//! grid for a small synthetic one (a plain, a river with a bridge, a
//! pocket...) so that the geometry is known. The v5 save refusal is tested
//! in `campaign.rs` (`save_load_round_trip`), determinism in
//! `twenty_turns_with_ai_are_deterministic` there and below.

use std::path::PathBuf;

use data_model::{FactionId, GameData, MapRasters, NavGrid, SettlementId, IMPASSABLE, PLAIN_COST};
use sim_campaign::march::km_to_grid_points;
use sim_campaign::movement::{retreat_target, retreat_target_after, Retreat};
use sim_campaign::navigation::Cell;
use sim_campaign::{
    ArmyId, ArmyPosition, CampaignState, EventKind, MoveTarget, Order, OrderError, OrderOutcome,
    StopReason,
};

fn real_data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

/// The real data on an all-plain 2048² grid, edited by `edit`.
fn data_with_grid(edit: impl FnOnce(&mut NavGrid)) -> GameData {
    let mut data = real_data();
    let mut grid = NavGrid::uniform(2048, 2048, 2, 1.438, PLAIN_COST);
    edit(&mut grid);
    grid.refresh_min_cost();
    data.set_map_rasters(MapRasters {
        navgrid: grid,
        provinces: None,
    });
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
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

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn px_per_km(data: &GameData) -> f32 {
    data.navgrid().px_per_km() as f32
}

fn distance_km(data: &GameData, a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() / px_per_km(data)
}

/// A map point `km` kilometres east of `point`.
fn east(data: &GameData, point: [f32; 2], km: f32) -> [f32; 2] {
    [point[0] + km * px_per_km(data), point[1]]
}

/// A point of central France with no settlement within `radius_km`
/// (deterministic scan).
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

/// France and England at war; only the French and English main armies are
/// kept, both in the field.
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
    assert!(state.is_at_war(&fac("fac_france"), &fac("fac_england")));
    (state, french, english)
}

fn march(state: &mut CampaignState, data: &GameData, order: Order) -> sim_campaign::MoveReport {
    match state
        .submit_order_outcome(data, order)
        .expect("move accepted")
    {
        OrderOutcome::Moved(report) => report,
        other => panic!("expected a march, got {other:?}"),
    }
}

#[test]
fn a_path_goes_round_a_river_by_the_bridge() {
    // A river two cells wide, north-south, with one bridge.
    let probe = real_data();
    let start = empty_spot(&probe, 25.0);
    let grid_start = Cell::of_point(probe.navgrid(), start);
    let river_x = i64::from(grid_start.x) + 10;
    let bridge_y = i64::from(grid_start.y) + 40;
    let data = data_with_grid(|grid| {
        for y in 0..2048 {
            if y != bridge_y {
                grid.set(river_x, y, IMPASSABLE);
                grid.set(river_x + 1, y, IMPASSABLE);
            }
        }
    });
    let (state, french, _) = duel(&data, start, [10.0, 10.0]);
    let target = east(&data, start, 40.0);
    let path = state.find_path(&data, &french, target).expect("a path");
    let bridge = [
        Cell::new(river_x as u32, bridge_y as u32),
        Cell::new(river_x as u32 + 1, bridge_y as u32),
    ];
    assert!(
        bridge.iter().all(|c| path.cells.contains(c)),
        "the path crosses at the bridge"
    );
    assert!(path.cells.iter().all(|c| {
        let (x, y) = (i64::from(c.x), i64::from(c.y));
        !(x == river_x || x == river_x + 1) || y == bridge_y
    }));
    // The detour costs more than the straight line (40 km ≈ 28 cells).
    assert!(path.cost > km_to_grid_points(&data, 40.0));
    // Smoothed: a few corners, not one per cell.
    assert!(path.waypoints.len() <= 6, "{:?}", path.waypoints);
    // Without the bridge, no way across.
    let closed = data_with_grid(|grid| {
        for y in 0..2048 {
            grid.set(river_x, y, IMPASSABLE);
            grid.set(river_x + 1, y, IMPASSABLE);
        }
    });
    let (state, french, _) = duel(&closed, start, [10.0, 10.0]);
    assert!(state.find_path(&closed, &french, target).is_none());
}

#[test]
fn an_army_stops_in_an_enemy_zone_of_control() {
    let data = data_with_grid(|_| {});
    let start = empty_spot(&data, 25.0);
    let enemy_at = east(&data, start, 30.0);
    let (mut state, french, english) = duel(&data, start, enemy_at);
    let report = march(
        &mut state,
        &data,
        Order::move_to_point(french.clone(), east(&data, start, 60.0)),
    );
    assert_eq!(
        report.stop,
        StopReason::EnemyZoneOfControl {
            army: english.clone()
        }
    );
    let at = state.army_point(&data, &state.armies[&french]);
    let zoc = data.free_movement_rules().zoc_radius_km as f32;
    let d = distance_km(&data, at, enemy_at);
    assert!(d <= zoc && d > zoc - 2.0, "stopped {d} km from the enemy");
    assert!(state.armies[&french].planned_path.is_empty());
    assert!(state.armies[&french].movement_left > 0, "points left over");
    // Already inside the zone, the army may walk out of it.
    let report = march(
        &mut state,
        &data,
        Order::move_to_point(french.clone(), east(&data, start, -20.0)),
    );
    assert_eq!(report.stop, StopReason::Arrived);
}

#[test]
fn a_march_spanning_several_turns_resumes_next_turn() {
    let data = data_with_grid(|_| {});
    let start = empty_spot(&data, 25.0);
    let (mut state, french, _) = duel(&data, start, [10.0, 10.0]);
    let allowance = state.armies[&french].movement_left;
    // Lot DC1 (ADR 0082): about 105 km of plain a season.
    let target = [start[0], start[1] - 225.0 * px_per_km(&data)];
    let report = march(
        &mut state,
        &data,
        Order::move_to_point(french.clone(), target),
    );
    assert_eq!(report.stop, StopReason::OutOfMovement);
    assert!(report.cost <= allowance);
    assert!(!report.walked.is_empty());
    assert!(!state.armies[&french].planned_path.is_empty());
    assert_eq!(
        state.armies[&french].destination,
        Some(MoveTarget::Point {
            x: target[0],
            y: target[1]
        })
    );
    let first = state.army_point(&data, &state.armies[&french]);
    let covered = distance_km(&data, start, first);
    assert!((75.0..=108.0).contains(&covered), "{covered} km in spring");
    // No more points this turn.
    assert!(matches!(
        state.submit_order(&data, Order::move_to_point(french.clone(), target)),
        Ok(()) | Err(OrderError::NoMovementLeft)
    ));
    let mut turns = 0;
    while state.armies[&french].position != ArmyPosition::field(target) {
        let before = state.army_point(&data, &state.armies[&french]);
        state.end_turn_with(&data, idle);
        turns += 1;
        let after = state.army_point(&data, &state.armies[&french]);
        let moved = distance_km(&data, before, after);
        assert!(
            moved > 75.0 || state.armies[&french].planned_path.is_empty(),
            "the march resumed: {moved} km"
        );
        assert!(turns <= 3, "225 km take three turns");
    }
    assert!(state.armies[&french].planned_path.is_empty());
    assert!(state.armies[&french].destination.is_none());
}

#[test]
fn attack_within_reach_and_out_of_reach() {
    let data = data_with_grid(|_| {});
    let start = empty_spot(&data, 25.0);
    // Out of reach this turn: refused, nothing changes.
    let (mut state, french, english) = duel(&data, start, east(&data, start, 400.0));
    let before = state.save_json();
    assert_eq!(
        state.submit_order(
            &data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            }
        ),
        Err(OrderError::OutOfRange)
    );
    assert_eq!(state.save_json(), before);
    // Within reach: the attacker closes in (through the target's own zone
    // of control) and fights at once.
    let (mut state, french, english) = duel(&data, start, east(&data, start, 40.0));
    let english_before = state.armies[&english].total_strength();
    state
        .submit_order(
            &data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            },
        )
        .unwrap();
    assert!(state
        .pending_events
        .iter()
        .any(|e| e.kind == EventKind::Battle && e.text_fr.contains("Vainqueur")));
    assert_eq!(
        state.armies[&french].movement_left, 0,
        "no move after a battle"
    );
    assert!(state
        .armies
        .get(&english)
        .is_none_or(|a| a.total_strength() < english_before));
    // Not at war: refused.
    let (mut state, french, english) = duel(&data, start, east(&data, start, 3.0));
    let england = fac("fac_england");
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .at_war_with
        .remove(&england);
    state
        .factions
        .get_mut(&england)
        .unwrap()
        .at_war_with
        .remove(&fac("fac_france"));
    assert_eq!(
        state.submit_order(
            &data,
            Order::Attack {
                army: french,
                target_army: english,
            }
        ),
        Err(OrderError::NotAtWar)
    );
}

#[test]
fn entering_an_enemy_place_lays_siege() {
    let data = data_with_grid(|_| {});
    let bordeaux = set("set_bordeaux");
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 5).unwrap();
    assert!(state.is_hostile_settlement(&fac("fac_france"), &bordeaux));
    assert!(!state.settlements[&bordeaux].garrison.is_empty());
    let french = main_army(&state, "fac_france");
    let english: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| a.faction != fac("fac_france"))
        .map(|(id, _)| id.clone())
        .collect();
    for id in english {
        state.armies.remove(&id);
    }
    let point = data.settlement_point(&bordeaux).unwrap();
    state.armies.get_mut(&french).unwrap().position =
        ArmyPosition::field(east(&data, point, -12.0));
    let report = march(
        &mut state,
        &data,
        Order::move_to(french.clone(), bordeaux.clone()),
    );
    assert_eq!(
        report.stop,
        StopReason::SiegeStarted {
            settlement: bordeaux.clone()
        }
    );
    let army = &state.armies[&french];
    assert!(army.is_at(&bordeaux));
    assert_eq!(army.stance, sim_campaign::Stance::Siege);
    let siege = state.settlements[&bordeaux]
        .siege
        .clone()
        .expect("besieged");
    assert_eq!(siege.attacker, fac("fac_france"));
    assert_eq!(siege.started_turn, state.turn());
    assert!(state
        .pending_events
        .iter()
        .any(|e| e.kind == EventKind::SiegeStarted));
    // The siege counts from the next end of turn, not twice.
    state.end_turn_with(&data, idle);
    let siege = state.settlements[&bordeaux]
        .siege
        .clone()
        .expect("still besieged");
    assert_eq!(siege.turns_elapsed, 0);
    state.end_turn_with(&data, idle);
    if let Some(siege) = &state.settlements[&bordeaux].siege {
        assert_eq!(siege.turns_elapsed, 1);
    }
}

#[test]
fn an_ungarrisoned_place_is_taken_and_a_friendly_one_is_a_stop() {
    let data = data_with_grid(|_| {});
    let bordeaux = set("set_bordeaux");
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 5).unwrap();
    let french = main_army(&state, "fac_france");
    state
        .armies
        .retain(|id, a| *id == french || a.faction == fac("fac_france"));
    state
        .settlements
        .get_mut(&bordeaux)
        .unwrap()
        .garrison
        .clear();
    let point = data.settlement_point(&bordeaux).unwrap();
    state.armies.get_mut(&french).unwrap().position =
        ArmyPosition::field(east(&data, point, -12.0));
    let report = march(
        &mut state,
        &data,
        Order::move_to(french.clone(), bordeaux.clone()),
    );
    assert_eq!(
        report.stop,
        StopReason::SettlementTaken {
            settlement: bordeaux.clone()
        }
    );
    assert_eq!(state.settlements[&bordeaux].controller, fac("fac_france"));
    // Now friendly: the army leaves and comes back the same turn.
    let out = march(
        &mut state,
        &data,
        Order::move_to_point(french.clone(), east(&data, point, 10.0)),
    );
    assert_eq!(out.stop, StopReason::Arrived);
    let back = march(
        &mut state,
        &data,
        Order::move_to(french.clone(), bordeaux.clone()),
    );
    assert_eq!(
        back.stop,
        StopReason::Stationed {
            settlement: bordeaux.clone()
        }
    );
    // A garrison can be left there (lot C7d).
    state
        .submit_order(
            &data,
            Order::GarrisonUnits {
                army: french.clone(),
                unit_indices: vec![0],
            },
        )
        .unwrap();
    assert_eq!(state.settlements[&bordeaux].garrison.len(), 1);
}

#[test]
fn hostile_places_are_walked_around() {
    let data = data_with_grid(|_| {});
    let bordeaux = set("set_bordeaux");
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 5).unwrap();
    let french = main_army(&state, "fac_france");
    state.armies.retain(|id, _| *id == french);
    let point = data.settlement_point(&bordeaux).unwrap();
    let from = east(&data, point, -10.0);
    let to = east(&data, point, 10.0);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(from);
    let report = march(&mut state, &data, Order::move_to_point(french.clone(), to));
    assert_eq!(report.stop, StopReason::Arrived);
    let grid = data.navgrid();
    let hostile = Cell::of_point(grid, point);
    assert!(report
        .walked
        .iter()
        .all(|p| Cell::of_point(grid, *p) != hostile));
    assert!(state.settlements[&bordeaux].siege.is_none());
}

#[test]
fn the_loser_falls_back_on_the_grid() {
    let spot = empty_spot(&real_data(), 25.0);
    let victor_at = east(&real_data(), spot, -4.0);
    // No friendly place within reach; a refuge within the former 140 km
    // (two steps of 70 km since lot DC1, ADR 0082).
    let setup = |data: &mut GameData| {
        let retreat = &mut data.settlement_rules.as_mut().unwrap().retreat;
        retreat.friendly_radius_steps = 0.0;
        retreat.neutral_radius_steps = 2.0;
    };
    let mut open = data_with_grid(|_| {});
    setup(&mut open);
    let (state, _, english) = duel(&open, victor_at, spot);
    match retreat_target(&state, &open, &english, victor_at) {
        Some(Retreat::Fallback(point)) => {
            let moved = distance_km(&open, spot, point);
            assert!((13.0..=17.0).contains(&moved), "{moved} km");
            assert!(point[0] > spot[0], "away from the victor, to the east");
        }
        other => panic!("expected a fallback, got {other:?}"),
    }
    // A wall to the east: the army falls back aside.
    let wall_x = i64::from(Cell::of_point(open.navgrid(), spot).x) + 3;
    let mut walled = data_with_grid(|grid| {
        for y in 0..2048 {
            grid.set(wall_x, y, IMPASSABLE);
        }
    });
    setup(&mut walled);
    // DC3: on the densified map the empty spot moved and no refuge lies within two
    // steps on this side of the wall (a rout at 2): the radius is widened.
    walled
        .settlement_rules
        .as_mut()
        .unwrap()
        .retreat
        .neutral_radius_steps = 4.0;
    let (state, _, english) = duel(&walled, victor_at, spot);
    match retreat_target(&state, &walled, &english, victor_at) {
        Some(Retreat::Fallback(point)) => {
            let cell = Cell::of_point(walled.navgrid(), point);
            assert!(i64::from(cell.x) < wall_x, "not across the wall");
        }
        other => panic!("expected a fallback aside, got {other:?}"),
    }
    // Shut in a pocket: a rout, and with no friendly place on its land, the
    // army disperses when beaten.
    let (cx, cy) = {
        let c = Cell::of_point(open.navgrid(), spot);
        (i64::from(c.x), i64::from(c.y))
    };
    let mut pocket = data_with_grid(|grid| {
        for y in 0..2048i64 {
            for x in 0..2048i64 {
                if (x - cx).abs() > 2 || (y - cy).abs() > 2 {
                    grid.set(x, y, IMPASSABLE);
                }
            }
        }
    });
    setup(&mut pocket);
    let (state, _, english) = duel(&pocket, victor_at, spot);
    assert_eq!(
        retreat_target(&state, &pocket, &english, victor_at),
        Some(Retreat::Rout(None))
    );
}

/// Lot M5b: without a friendly place in reach, a crushed army or one with
/// no refuge left routs instead of falling back.
#[test]
fn a_crushed_or_cornered_loser_routs() {
    let spot = empty_spot(&real_data(), 25.0);
    let victor_at = east(&real_data(), spot, -4.0);
    let mut data = data_with_grid(|_| {});
    let rules = &mut data.settlement_rules.as_mut().unwrap().retreat;
    rules.friendly_radius_steps = 0.0;
    // A refuge within the former 140 km (lot DC1 halved the step).
    rules.neutral_radius_steps = 2.0;
    rules.heavy_defeat_losses_percent = 42;
    let (state, _, english) = duel(&data, victor_at, spot);
    assert!(matches!(
        retreat_target_after(&state, &data, &english, victor_at, 41),
        Some(Retreat::Fallback(_))
    ));
    assert!(matches!(
        retreat_target_after(&state, &data, &english, victor_at, 42),
        Some(Retreat::Rout(_))
    ));
    // No settlement free of the enemy within the refuge radius: a rout.
    data.settlement_rules
        .as_mut()
        .unwrap()
        .retreat
        .neutral_radius_steps = 0.0;
    assert!(matches!(
        retreat_target(&state, &data, &english, victor_at),
        Some(Retreat::Rout(_))
    ));
}

#[test]
fn free_movement_is_deterministic() {
    let data = real_data();
    let run = || {
        let mut state = CampaignState::new_1337(&data, fac("fac_france"), 77).unwrap();
        let french = main_army(&state, "fac_france");
        let toulouse = state
            .province_city_id(&data_model::ProvinceId::new("prov_toulousain").unwrap())
            .cloned()
            .unwrap();
        state
            .submit_order(&data, Order::move_to(french, toulouse))
            .unwrap();
        for _ in 0..4 {
            state.end_turn(&data);
        }
        state.save_json()
    };
    assert_eq!(run(), run());
}

/// Performance of the grid searches on the real grid (lot M3 targets under
/// 50 ms per AI faction and turn): a multi-turn path across France and a
/// reachable area.
#[test]
fn grid_searches_are_fast_enough() {
    let data = real_data();
    let state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let french = main_army(&state, "fac_france");
    let bayonne = data.settlement_point(&set("set_bayonne")).unwrap();
    let grid = data.navgrid();
    let start = std::time::Instant::now();
    let _ = grid.component(0, 0);
    println!("components labelled in {:?}", start.elapsed());
    let start = std::time::Instant::now();
    let path = state
        .find_path(&data, &french, bayonne)
        .expect("Paris to Bayonne");
    let path_time = start.elapsed();

    let start = std::time::Instant::now();
    let area = state.reachable_area(&data, &french);
    let area_time = start.elapsed();
    println!(
        "Paris-Bayonne: {} cells, cost {}, {path_time:?}; bubble: {} cells, {area_time:?}",
        path.cells.len(),
        path.cost,
        area.len()
    );
    assert!(!area.is_empty());
    // Generous bounds: debug builds are ~10× slower than release.
    assert!(path_time.as_millis() < 3_000, "{path_time:?}");
    assert!(area_time.as_millis() < 3_000, "{area_time:?}");
}
