//! Path preview for the campaign map (lot M4, spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 6): the polyline a march
//! would follow, split by turn. Pure query, the state is not touched.
//!
//! The walk mirrors `march.rs`: the smoothed corners of
//! [`CampaignState::find_path`] joined by 8-connected lines, spending
//! [`crate::Army::movement_left`] on the first turn and the full allowance
//! ([`CampaignState::army_grid_allowance`]) on each later one. Zones of
//! control are not simulated: the preview shows the intended route.

use data_model::GameData;

use crate::navigation::{line_cells, step_cost, Cell};
use crate::state::{ArmyId, CampaignState};

/// Turn-by-turn plan of a march towards a map point.
#[derive(Debug, Clone, PartialEq)]
pub struct PathPlan {
    /// Map-pixel polyline: the army's point, the corners of the path, the
    /// point where each turn ends, and the target.
    pub points: Vec<[f32; 2]>,
    /// Index in `points` where each turn ends, in order; the last entry is
    /// the end of the march (`points.len() - 1`).
    pub turn_ends: Vec<usize>,
    /// Total cost of the march (grid movement points).
    pub cost: u32,
    /// Cost spent on the first turn.
    pub cost_this_turn: u32,
}

impl PathPlan {
    /// Index in `points` where the march stops this turn.
    pub fn stop_index(&self) -> usize {
        self.turn_ends.first().copied().unwrap_or(0)
    }

    /// Number of turns the march takes (0 when there is nothing to walk).
    pub fn turns(&self) -> usize {
        if self.points.len() < 2 {
            0
        } else {
            self.turn_ends.len()
        }
    }

    /// `true` when the whole march fits in the current turn.
    pub fn reachable_this_turn(&self) -> bool {
        self.turns() == 1
    }
}

impl CampaignState {
    /// Turn-by-turn plan of `army` marching to map pixel `target`; `None`
    /// when the army is unknown or the target unreachable. A march that
    /// would need an allowance of zero stops at the end of the first turn.
    pub fn plan_path(&self, data: &GameData, army: &ArmyId, target: [f32; 2]) -> Option<PathPlan> {
        let entry = self.armies.get(army)?;
        let grid = data.navgrid();
        let path = self.find_path(data, army, target)?;
        let start = self.army_point(data, entry);
        let allowance = self.army_grid_allowance(data, entry);
        let mut plan = PathPlan {
            points: vec![start],
            turn_ends: Vec::new(),
            cost: 0,
            cost_this_turn: 0,
        };
        let mut left = entry.movement_left;
        let mut current = self.army_cell(data, entry);
        let mut first_turn = true;
        'walk: for waypoint in &path.waypoints {
            for next in line_cells(current, *waypoint) {
                let Some(step) = step_cost(grid, current, next) else {
                    break 'walk;
                };
                if step > left {
                    // The turn ends here: the next one starts afresh.
                    let point = if plan.cost == 0 {
                        start
                    } else {
                        current.center(grid)
                    };
                    close_turn(&mut plan, point);
                    if first_turn {
                        plan.cost_this_turn = plan.cost;
                        first_turn = false;
                    }
                    if step > allowance {
                        return Some(plan);
                    }
                    left = allowance;
                }
                left -= step;
                plan.cost += step;
                current = next;
            }
            plan.points.push(current.center(grid));
        }
        // The exact target point stands for the centre of its cell.
        if current == Cell::of_point(grid, target) && plan.points.len() > 1 {
            let last = plan.points.len() - 1;
            plan.points[last] = target;
        }
        if first_turn {
            plan.cost_this_turn = plan.cost;
        }
        let end = plan.points.len() - 1;
        if plan.turn_ends.last() != Some(&end) {
            plan.turn_ends.push(end);
        }
        Some(plan)
    }
}

/// Ends a turn at `point` (added unless it is already the last point).
fn close_turn(plan: &mut PathPlan, point: [f32; 2]) {
    if plan.points.last() != Some(&point) {
        plan.points.push(point);
    }
    plan.turn_ends.push(plan.points.len() - 1);
}
