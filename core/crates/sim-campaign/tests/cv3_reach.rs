//! Lot CV3-5: two-tone reachable area (spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 4). The first ring is what
//! the army reaches with its movement left, the second what it reaches by
//! the end of the next turn (full movement, no forced march bonus), with the
//! costs and zones of control of the real march.
//!
//! The rules run on the real game data on a synthetic all-plain grid, the
//! armies standing at sea (no settlement around) so that the geometry is
//! known.

use std::collections::BTreeSet;
use std::path::PathBuf;

use data_model::{FactionId, GameData, MapRasters, NavGrid, PLAIN_COST};
use sim_campaign::navigation::{step_cost, Cell};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, OrderOutcome, Stance, StopReason};

fn real_data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

/// The real data on an all-plain 2048² grid.
fn plain_data() -> GameData {
    let mut data = real_data();
    let mut grid = NavGrid::uniform(2048, 2048, 2, 1.438, PLAIN_COST);
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

fn px_per_km(data: &GameData) -> f32 {
    data.navgrid().px_per_km() as f32
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

/// A point with no settlement within `radius_km` (deterministic scan, open
/// sea west of France on the real map).
fn empty_spot(data: &GameData, radius_km: f32) -> [f32; 2] {
    let settlements: Vec<[f32; 2]> = data
        .settlements
        .keys()
        .filter_map(|id| data.settlement_point(id))
        .collect();
    let radius = radius_km * px_per_km(data);
    for y in (1600..3600).step_by(16) {
        for x in (400..2400).step_by(16) {
            let p = [x as f32, y as f32];
            if settlements
                .iter()
                .all(|s| ((s[0] - p[0]).powi(2) + (s[1] - p[1]).powi(2)).sqrt() > radius)
            {
                return p;
            }
        }
    }
    panic!("no empty spot");
}

/// France and England at war, only their main armies kept, both in the
/// field with their full movement; England stands at `english_at` (far
/// away when `None`).
fn duel(
    data: &GameData,
    french_at: [f32; 2],
    english_at: Option<[f32; 2]>,
) -> (CampaignState, ArmyId, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.retain(|id, _| *id == french || *id == english);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(french_at);
    state.armies.get_mut(&english).unwrap().position =
        ArmyPosition::field(english_at.unwrap_or([10.0, 10.0]));
    for id in [&french, &english] {
        let allowance = state.army_grid_allowance(data, &state.armies[id]);
        state.armies.get_mut(id).unwrap().movement_left = allowance;
    }
    assert!(state.is_at_war(&fac("fac_france"), &fac("fac_england")));
    (state, french, english)
}

/// Octile cost on the uniform plain.
fn octile(data: &GameData, a: Cell, b: Cell) -> u32 {
    let grid = data.navgrid();
    let straight = step_cost(grid, Cell::new(10, 10), Cell::new(11, 10)).unwrap();
    let diagonal = step_cost(grid, Cell::new(10, 10), Cell::new(11, 11)).unwrap();
    let dx = (i64::from(a.x) - i64::from(b.x)).unsigned_abs() as u32;
    let dy = (i64::from(a.y) - i64::from(b.y)).unsigned_abs() as u32;
    straight * (dx.max(dy) - dx.min(dy)) + diagonal * dx.min(dy)
}

/// The real march towards the centre of `cell`: an order this turn, and
/// again on a fresh next turn (full movement, forced march over) when it
/// fell short. `true` when the army ends on `cell`.
fn real_march_reaches(state: &CampaignState, data: &GameData, army: &ArmyId, cell: Cell) -> bool {
    let grid = data.navgrid();
    let mut state = state.clone();
    let target = cell.center(grid);
    for turn in 0..2 {
        if turn == 1 {
            let entry = state.armies.get_mut(army).unwrap();
            if entry.stance == Stance::ForcedMarch {
                entry.stance = Stance::Normal;
            }
            let base = state.army_base_grid_allowance(data, &state.armies[army]);
            state.armies.get_mut(army).unwrap().movement_left = base;
        }
        if let Ok(OrderOutcome::Moved(report)) =
            state.submit_order_outcome(data, Order::move_to_point(army.clone(), target))
        {
            if report.stop == StopReason::Arrived {
                return true;
            }
        }
        if state.army_cell(data, &state.armies[army]) == cell {
            return true;
        }
    }
    false
}

/// Passable cells next to the area (both rings) but outside it.
fn outer_border(area: &BTreeSet<Cell>) -> Vec<Cell> {
    let mut border = BTreeSet::new();
    for cell in area {
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let (x, y) = (i32::from(cell.x) + dx, i32::from(cell.y) + dy);
                if x < 0 || y < 0 {
                    continue;
                }
                let next = Cell::new(x as u32, y as u32);
                if !area.contains(&next) {
                    border.insert(next);
                }
            }
        }
    }
    border.into_iter().collect()
}

#[test]
fn second_ring_is_exactly_what_two_turns_of_marching_cover() {
    let data = plain_data();
    let start_point = empty_spot(&data, 150.0);
    let (mut state, french, _) = duel(&data, start_point, None);
    // A march already begun: part of the turn spent.
    let base = state.army_base_grid_allowance(&data, &state.armies[&french]);
    state.armies.get_mut(&french).unwrap().movement_left = base * 3 / 5;
    let area = state.reachable_cells(&data, &french);
    let left = area.budget;
    assert_eq!(left, base * 3 / 5);
    assert_eq!(area.next_budget, base);
    let start = state.army_cell(&data, &state.armies[&french]);
    println!(
        "left {left}, base {base}: {} + {} cells",
        area.this_turn.len(),
        area.next_turn.len()
    );

    // Ring 1: every cell within `left` (octile distance on the plain).
    let first: BTreeSet<Cell> = area.this_turn.iter().map(|(c, _)| *c).collect();
    for (cell, cost) in &area.this_turn {
        assert_eq!(*cost, octile(&data, start, *cell));
    }
    // Reference: a cell is reachable by the end of next turn when some end
    // point `p` of this turn is within `left` of the start and the cell
    // within `base` of `p` (the turn may end anywhere).
    let reach = i64::from((left + base) / 10 + 2);
    let mut expected_next = BTreeSet::new();
    let mut expected_first = BTreeSet::new();
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let cell = Cell::new(
                (i64::from(start.x) + dx) as u32,
                (i64::from(start.y) + dy) as u32,
            );
            if octile(&data, start, cell) <= left {
                expected_first.insert(cell);
            } else if first.iter().any(|p| octile(&data, *p, cell) <= base) {
                expected_next.insert(cell);
            }
        }
    }
    assert_eq!(first, expected_first);
    let next: BTreeSet<Cell> = area.next_turn.iter().copied().collect();
    assert_eq!(next, expected_next);

    // The real march agrees: cells of the second ring comfortably inside
    // are reached in two turns (a turn wastes less than one diagonal step),
    // and no cell just outside the area is.
    let diagonal = octile(&data, Cell::new(0, 0), Cell::new(1, 1));
    let inside: Vec<Cell> = area
        .next_turn
        .iter()
        .copied()
        .filter(|c| octile(&data, start, *c) + diagonal <= left + base)
        .collect();
    for cell in inside.iter().step_by((inside.len() / 24).max(1)) {
        assert!(
            real_march_reaches(&state, &data, &french, *cell),
            "{cell:?} of the second ring is reached in two turns"
        );
    }
    let all: BTreeSet<Cell> = first.union(&next).copied().collect();
    let border = outer_border(&all);
    for cell in border.iter().step_by((border.len() / 24).max(1)) {
        assert!(
            !real_march_reaches(&state, &data, &french, *cell),
            "{cell:?} outside the area is not reached in two turns"
        );
    }
}

#[test]
fn an_enemy_zone_of_control_ends_the_first_turn_not_the_second() {
    let data = plain_data();
    let start_point = empty_spot(&data, 150.0);
    let base = {
        let (state, french, _) = duel(&data, start_point, None);
        state.army_base_grid_allowance(&data, &state.armies[&french])
    };
    let base_km = f64::from(base) / 10.0 * data.navgrid().cell_km;
    let zoc_km = data.free_movement_rules().zoc_radius_km;
    // The enemy east, seen (within the army's sight) and its zone of
    // control inside the first ring.
    let enemy_km = (data.free_movement_rules().vision_army_km * 0.7).min(base_km * 0.5);
    let enemy_at = [
        start_point[0] + enemy_km as f32 * px_per_km(&data),
        start_point[1],
    ];
    let (free_state, free_french, _) = duel(&data, start_point, None);
    let free = free_state.reachable_cells(&data, &free_french);
    let (state, french, _) = duel(&data, start_point, Some(enemy_at));
    let area = state.reachable_cells(&data, &french);
    let grid = data.navgrid();
    let zoc_px = zoc_km as f32 * px_per_km(&data);
    let in_zoc = |c: &Cell| {
        let p = c.center(grid);
        ((p[0] - enemy_at[0]).powi(2) + (p[1] - enemy_at[1]).powi(2)).sqrt() <= zoc_px
    };
    println!(
        "free {} + {}, with an enemy {} + {}",
        free.this_turn.len(),
        free.next_turn.len(),
        area.this_turn.len(),
        area.next_turn.len()
    );
    // The zone is entered this turn, but the march stops there: the first
    // ring is smaller, and the cell right behind the enemy is out of it.
    assert!(area.this_turn.iter().any(|(c, _)| in_zoc(c)));
    assert!(area.this_turn.len() < free.this_turn.len());
    let behind = Cell::of_point(
        grid,
        [enemy_at[0] + (zoc_px + 2.0 * px_per_km(&data)), enemy_at[1]],
    );
    let first: BTreeSet<Cell> = area.this_turn.iter().map(|(c, _)| *c).collect();
    let next: BTreeSet<Cell> = area.next_turn.iter().copied().collect();
    let start = state.army_cell(&data, &state.armies[&french]);
    assert!(
        octile(&data, start, behind) <= area.budget,
        "behind the enemy is in reach on an open plain"
    );
    // Going round the zone costs more than the straight line: out of the
    // first ring when the detour does not fit.
    // Next turn: a march starting inside the zone walks out of it.
    assert!(first.contains(&behind) || next.contains(&behind));
    // No cell of the zone ends the first turn and lets it go on: every
    // first-ring cell beyond the zone was reached round it (its cost is at
    // least the straight line).
    for (cell, cost) in &area.this_turn {
        assert!(*cost >= octile(&data, start, *cell));
    }
    // The real march never beats the area.
    let all: BTreeSet<Cell> = first.union(&next).copied().collect();
    let border = outer_border(&all);
    for cell in border.iter().step_by((border.len() / 24).max(1)) {
        assert!(
            !real_march_reaches(&state, &data, &french, *cell),
            "{cell:?} outside the area is not reached in two turns"
        );
    }
    // Straight through the zone: the march stops in it, and goes on the
    // next turn.
    let far_behind = Cell::of_point(
        grid,
        [enemy_at[0] + (zoc_px + 1.0 * px_per_km(&data)), enemy_at[1]],
    );
    let mut walker = state.clone();
    let report = match walker
        .submit_order_outcome(
            &data,
            Order::move_to_point(french.clone(), far_behind.center(grid)),
        )
        .unwrap()
    {
        OrderOutcome::Moved(report) => report,
        other => panic!("expected a march, got {other:?}"),
    };
    assert!(matches!(report.stop, StopReason::EnemyZoneOfControl { .. }));
    let stopped = walker.army_cell(&data, &walker.armies[&french]);
    assert!(first.contains(&stopped));
    assert!(real_march_reaches(&state, &data, &french, far_behind));
    assert!(first.contains(&far_behind) || next.contains(&far_behind));
}

#[test]
fn forced_march_widens_the_first_ring_only() {
    let data = plain_data();
    let start_point = empty_spot(&data, 150.0);
    let (mut state, french, _) = duel(&data, start_point, None);
    let normal = state.reachable_cells(&data, &french);
    state
        .submit_order(
            &data,
            Order::SetStance {
                army: french.clone(),
                stance: Stance::ForcedMarch,
            },
        )
        .expect("forced march allowed with full movement");
    let forced = state.reachable_cells(&data, &french);
    assert!(forced.budget > normal.budget);
    assert_eq!(forced.next_budget, normal.next_budget);
    assert!(forced.this_turn.len() > normal.this_turn.len());
    let normal_first: BTreeSet<Cell> = normal.this_turn.iter().map(|(c, _)| *c).collect();
    assert!(forced
        .this_turn
        .iter()
        .all(|(c, _)| normal_first.contains(c) || normal.next_turn.contains(c)));
    // Both rings together reach further with the bonus.
    assert!(
        forced.this_turn.len() + forced.next_turn.len()
            > normal.this_turn.len() + normal.next_turn.len()
    );
}

#[test]
fn an_army_without_movement_left_only_has_the_second_ring() {
    let data = plain_data();
    let start_point = empty_spot(&data, 150.0);
    let (mut state, french, _) = duel(&data, start_point, None);
    state.armies.get_mut(&french).unwrap().movement_left = 0;
    let area = state.reachable_cells(&data, &french);
    assert_eq!(area.this_turn.len(), 1);
    assert!(!area.next_turn.is_empty());
}
