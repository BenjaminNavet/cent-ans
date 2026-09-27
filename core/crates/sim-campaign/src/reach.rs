//! Two-tone reachable area of an army (lot CV3-5, spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 4): the cells it can
//! reach this turn, and those it can reach by the end of the next one.
//!
//! Same costs as the real march (`march.rs`): grid step costs, hostile
//! settlements never entered, and a march ends on entering the zone of
//! control of an enemy army. The first turn spends
//! [`crate::Army::movement_left`] (the forced march bonus is already in it);
//! the second one the full movement of a fresh turn without any stance bonus
//! ([`CampaignState::army_base_grid_allowance`]): forced marches end at the
//! start of a turn (`posture::start_of_turn`).
//!
//! A turn may end anywhere (the player can stop short), so a cell is
//! reachable next turn when some cell `p` of the first ring (or the army's
//! own) is within `base` of it. A turn that starts inside the zone of
//! control of an enemy is not stopped by that enemy (the march lets an army
//! walk out of a zone it stands in): the second search is run once per set
//! of enemies whose zone holds its starts.
//!
//! Only the enemies the army's faction sees count (`Vision::sees_army`):
//! the area must not betray a hidden ambush.

use std::collections::{BTreeMap, BTreeSet};

use data_model::GameData;

use crate::march::px_per_km;
use crate::navigation::{bounded_dijkstra_from, Cell, CellSet};
use crate::state::{ArmyId, CampaignState};

/// Reachable cells of an army over two turns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReachableCells {
    /// Cells reachable this turn with their cost, sorted by cell (the army's
    /// own cell included, at cost 0).
    pub this_turn: Vec<(Cell, u32)>,
    /// Cells reachable by the end of the next turn only (not in
    /// `this_turn`), sorted.
    pub next_turn: Vec<Cell>,
    /// Movement points of the first turn (`movement_left`).
    pub budget: u32,
    /// Full movement of the next turn (base allowance, no stance bonus).
    pub next_budget: u32,
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

impl CampaignState {
    /// Cells `army` can reach this turn and by the end of the next one (see
    /// the module doc). Empty for an unknown army.
    pub fn reachable_cells(&self, data: &GameData, army_id: &ArmyId) -> ReachableCells {
        let Some(army) = self.armies.get(army_id) else {
            return ReachableCells::default();
        };
        let grid = data.navgrid();
        let blocker: CellSet = self.hostile_blocker(data, &army.faction);
        let zoc_px = data.free_movement_rules().zoc_radius_km as f32 * px_per_km(data);
        let vision = self.vision(data, &army.faction);
        let enemies: Vec<[f32; 2]> = self
            .armies
            .iter()
            .filter(|(id, other)| {
                *id != army_id
                    && self.is_at_war(&army.faction, &other.faction)
                    && vision.sees_army(self, data, other)
            })
            .map(|(_, other)| self.army_point(data, other))
            .collect();
        // Enemies whose zone of control holds `point` (they do not stop a
        // turn starting there).
        let holding = |point: [f32; 2]| -> Vec<usize> {
            enemies
                .iter()
                .enumerate()
                .filter(|(_, p)| distance(**p, point) <= zoc_px)
                .map(|(i, _)| i)
                .collect()
        };
        let stops = |ignored: &[usize]| {
            let active: Vec<[f32; 2]> = enemies
                .iter()
                .enumerate()
                .filter(|(i, _)| !ignored.contains(i))
                .map(|(_, p)| *p)
                .collect();
            move |cell: Cell| {
                let point = cell.center(grid);
                active.iter().any(|p| distance(*p, point) <= zoc_px)
            }
        };

        let start = self.army_cell(data, army);
        let start_point = self.army_point(data, army);
        let budget = army.movement_left;
        let next_budget = self.army_base_grid_allowance(data, army);

        let mut this_turn: Vec<(Cell, u32)> = Vec::new();
        let first_stop = stops(&holding(start_point));
        bounded_dijkstra_from(grid, &[start], budget, &blocker, &first_stop, |cell, cost| {
            this_turn.push((cell, cost));
            false
        });
        this_turn.sort_unstable();

        // Starts of the second turn, grouped by the enemies they ignore.
        let mut groups: BTreeMap<Vec<usize>, Vec<Cell>> = BTreeMap::new();
        for (cell, _) in &this_turn {
            let point = if *cell == start {
                start_point
            } else {
                cell.center(grid)
            };
            groups.entry(holding(point)).or_default().push(*cell);
        }
        let first: BTreeSet<Cell> = this_turn.iter().map(|(c, _)| *c).collect();
        let mut next: BTreeSet<Cell> = BTreeSet::new();
        for (ignored, starts) in &groups {
            let stop = stops(ignored);
            bounded_dijkstra_from(grid, starts, next_budget, &blocker, &stop, |cell, _| {
                if !first.contains(&cell) {
                    next.insert(cell);
                }
                false
            });
        }
        ReachableCells {
            this_turn,
            next_turn: next.into_iter().collect(),
            budget,
            next_budget,
        }
    }
}
