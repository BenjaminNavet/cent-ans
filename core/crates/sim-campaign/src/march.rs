//! Free army movement on the navigation grid, executed immediately (lot
//! M2, spec `docs/design/2026-09-24-mouvement-libre.md` § 3).
//!
//! An army stands at a free point of the map or in a settlement
//! ([`ArmyPosition`]). A move order walks it at once, cell by cell, along
//! the smoothed A* path of [`crate::navigation`], spending
//! [`Army::movement_left`] (grid costs, 10 = one plain cell). It stops:
//!
//! - at the destination (a settlement is entered: siege or capture when
//!   hostile, a stop otherwise);
//! - when its points run out (the rest of the path stays in
//!   [`Army::planned_path`] and resumes next turn);
//! - on entering the zone of control of an enemy army;
//! - before entering a hostile settlement other than its destination (the
//!   planner routes around them, so this only happens when the map changed).
//!
//! Attacks, embarkations and the continuation of multi-turn marches live
//! here too; the battles themselves are `movement::fight`.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{FactionId, GameData, NavGrid, ProvinceId, SettlementId, SettlementKind};
use serde::{Deserialize, Serialize};

use crate::events::GameEvent;
use crate::navigation::{self, line_cells, step_cost, Cell, CellSet, GridPath};
use crate::orders::OrderError;
use crate::state::{Army, ArmyId, ArmyPosition, CampaignState, MoveTarget, Stance};

/// Why a march stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StopReason {
    /// The destination point was reached.
    Arrived,
    /// The army entered a friendly or neutral settlement and stands there.
    Stationed { settlement: SettlementId },
    /// The army entered a hostile settlement and besieges it.
    SiegeStarted { settlement: SettlementId },
    /// The army entered a hostile settlement without defenders and took it.
    SettlementTaken { settlement: SettlementId },
    /// No movement points left; the rest of the path is kept for next turn.
    OutOfMovement,
    /// The army entered the zone of control of an enemy army.
    EnemyZoneOfControl { army: ArmyId },
    /// A hostile settlement bars the way.
    EnemySettlement { settlement: SettlementId },
    /// The path is no longer passable.
    Blocked,
}

/// What a move order did (lot M2): the points walked (map pixels, for the
/// animation), their cost and why the march stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveReport {
    pub army: ArmyId,
    /// Map-pixel points walked, in order (the start excluded).
    pub walked: Vec<[f32; 2]>,
    /// Movement points spent.
    pub cost: u32,
    pub stop: StopReason,
    /// Corners of the rest of the march (later turns).
    pub planned_path: Vec<Cell>,
}

/// Map pixels per kilometre on the grid of `data`.
pub fn px_per_km(data: &GameData) -> f32 {
    data.navgrid().px_per_km() as f32
}

/// Grid movement points worth `km` kilometres of plain.
pub fn km_to_grid_points(data: &GameData, km: f64) -> u32 {
    let grid = data.navgrid();
    (km / grid.cell_km.max(1e-6) * f64::from(data_model::PLAIN_COST)).round() as u32
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The settlement nearest to map pixel `point` (ties by id).
pub fn nearest_settlement(data: &GameData, point: [f32; 2]) -> Option<SettlementId> {
    nearest_settlement_where(data, point, |_| true)
}

/// The settlement nearest to `point` among those accepted by `accept`.
pub fn nearest_settlement_where(
    data: &GameData,
    point: [f32; 2],
    accept: impl Fn(&SettlementId) -> bool,
) -> Option<SettlementId> {
    data.settlements
        .keys()
        .filter(|id| accept(id))
        .filter_map(|id| data.settlement_point(id).map(|p| (distance(p, point), id)))
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)))
        .map(|(_, id)| id.clone())
}

impl CampaignState {
    /// Map-pixel point of `army`: its field position, or its settlement's.
    pub fn army_point(&self, data: &GameData, army: &Army) -> [f32; 2] {
        match &army.position {
            ArmyPosition::Field { x, y } => [*x, *y],
            ArmyPosition::Settlement(id) => data.settlement_point(id).unwrap_or([0.0, 0.0]),
        }
    }

    /// Province of `army`: its settlement's, or the one under its point in
    /// `province_ids.png` (the nearest settlement's at sea or without the
    /// raster).
    pub fn army_province(&self, data: &GameData, army: &Army) -> Option<ProvinceId> {
        match &army.position {
            ArmyPosition::Settlement(id) => self.settlement_province(id).cloned(),
            ArmyPosition::Field { x, y } => data
                .province_at_point(*x, *y)
                .filter(|p| self.provinces.contains_key(*p))
                .cloned()
                .or_else(|| {
                    nearest_settlement(data, [*x, *y])
                        .and_then(|s| self.settlement_province(&s).cloned())
                }),
        }
    }

    /// Grid cell of `army`.
    pub fn army_cell(&self, data: &GameData, army: &Army) -> Cell {
        Cell::of_point(data.navgrid(), self.army_point(data, army))
    }

    /// Settlement standing for `army` on the settlement graph (AI planning,
    /// agents): its own, or the nearest one.
    pub fn army_anchor(&self, data: &GameData, army: &Army) -> Option<SettlementId> {
        match &army.position {
            ArmyPosition::Settlement(id) => Some(id.clone()),
            ArmyPosition::Field { x, y } => nearest_settlement(data, [*x, *y]),
        }
    }

    /// Ids of the armies within `radius_km` of `point`, in id order.
    pub fn armies_near(&self, data: &GameData, point: [f32; 2], radius_km: f64) -> Vec<ArmyId> {
        let radius = radius_km as f32 * px_per_km(data);
        self.armies
            .iter()
            .filter(|(_, a)| distance(self.army_point(data, a), point) <= radius)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// `true` when two armies stand together: in the same settlement, or
    /// within `engage_radius_km` of each other (merges, retinue transfers).
    pub fn armies_together(&self, data: &GameData, a: &Army, b: &Army) -> bool {
        match (a.settlement(), b.settlement()) {
            (Some(x), Some(y)) => x == y,
            _ => self.army_distance_km(data, a, b) <= data.free_movement_rules().engage_radius_km,
        }
    }

    /// Distance in kilometres between two armies.
    pub fn army_distance_km(&self, data: &GameData, a: &Army, b: &Army) -> f64 {
        f64::from(distance(self.army_point(data, a), self.army_point(data, b)))
            / f64::from(px_per_km(data))
    }

    /// Movement points (grid costs) `army` receives at the start of a turn:
    /// its C7a allowance in kilometres of plain, converted to grid costs
    /// (10 per plain cell of about 1.44 km).
    pub fn army_grid_allowance(&self, data: &GameData, army: &Army) -> u32 {
        km_to_grid_points(data, f64::from(self.army_movement_allowance(data, army)))
    }

    /// Cells of the settlements held by an enemy of `faction`, with their
    /// ids (the first id of a shared cell).
    pub(crate) fn hostile_settlement_cells(
        &self,
        data: &GameData,
        faction: &FactionId,
    ) -> BTreeMap<Cell, SettlementId> {
        let grid = data.navgrid();
        let mut cells = BTreeMap::new();
        for id in self.settlements.keys() {
            if !self.is_hostile_settlement(faction, id) {
                continue;
            }
            if let Some(point) = data.settlement_point(id) {
                cells
                    .entry(Cell::of_point(grid, point))
                    .or_insert_with(|| id.clone());
            }
        }
        cells
    }

    /// The cells of [`CampaignState::hostile_settlement_cells`] as a
    /// search blocker.
    pub(crate) fn hostile_blocker(&self, data: &GameData, faction: &FactionId) -> CellSet {
        self.hostile_settlement_cells(data, faction)
            .into_keys()
            .collect()
    }

    /// Cheapest path of `army` to map pixel `target` on the grid (may span
    /// several turns), avoiding hostile settlements other than the one at
    /// the target. `None` when unreachable.
    pub fn find_path(&self, data: &GameData, army: &ArmyId, target: [f32; 2]) -> Option<GridPath> {
        let army = self.armies.get(army)?;
        let grid = data.navgrid();
        let blocker = self.hostile_blocker(data, &army.faction);
        navigation::find_path(
            grid,
            self.army_cell(data, army),
            Cell::of_point(grid, target),
            &blocker,
            None,
        )
    }

    /// Cells `army` can reach this turn with its movement points left, with
    /// their cost (the "bubble"; hostile settlements excluded).
    pub fn reachable_area(&self, data: &GameData, army: &ArmyId) -> Vec<(Cell, u32)> {
        let Some(army) = self.armies.get(army) else {
            return Vec::new();
        };
        let blocker = self.hostile_blocker(data, &army.faction);
        navigation::reachable_area(
            data.navgrid(),
            self.army_cell(data, army),
            army.movement_left,
            &blocker,
        )
    }

    /// Settlements `army` can reach (enter) this turn, with their cost: the
    /// non-hostile settlements inside the bubble, plus the hostile ones next
    /// to it (entering them starts a siege). The army's own is excluded.
    pub fn reachable(&self, data: &GameData, army: &ArmyId) -> BTreeMap<SettlementId, u32> {
        let Some(entry) = self.armies.get(army) else {
            return BTreeMap::new();
        };
        let grid = data.navgrid();
        let area: BTreeMap<Cell, u32> = self.reachable_area(data, army).into_iter().collect();
        let own = entry.settlement();
        let mut result = BTreeMap::new();
        for id in self.settlements.keys() {
            if Some(id) == own {
                continue;
            }
            let Some(point) = data.settlement_point(id) else {
                continue;
            };
            let cell = Cell::of_point(grid, point);
            let cost = if self.is_hostile_settlement(&entry.faction, id) {
                neighbours(cell)
                    .filter_map(|n| {
                        let base = area.get(&n)?;
                        Some(base + step_cost(grid, n, cell)?)
                    })
                    .filter(|c| *c <= entry.movement_left)
                    .min()
            } else {
                area.get(&cell).copied()
            };
            if let Some(cost) = cost {
                result.insert(id.clone(), cost);
            }
        }
        result
    }

    /// Provinces `army` can reach this turn: the cheapest reachable
    /// settlement of each province other than the army's own.
    pub fn reachable_provinces(&self, data: &GameData, army: &ArmyId) -> BTreeMap<ProvinceId, u32> {
        let own = self
            .armies
            .get(army)
            .and_then(|a| self.army_province(data, a));
        let mut result: BTreeMap<ProvinceId, u32> = BTreeMap::new();
        for (settlement, cost) in self.reachable(data, army) {
            let Some(province) = self.settlement_province(&settlement) else {
                continue;
            };
            if Some(province) == own.as_ref() {
                continue;
            }
            let entry = result.entry(province.clone()).or_insert(cost);
            *entry = (*entry).min(cost);
        }
        result
    }

    /// Map-pixel point of a move target.
    pub fn target_point(&self, data: &GameData, target: &MoveTarget) -> Option<[f32; 2]> {
        match target {
            MoveTarget::Point { x, y } => Some([*x, *y]),
            MoveTarget::Settlement(id) => data.settlement_point(id),
        }
    }
}

fn neighbours(cell: Cell) -> impl Iterator<Item = Cell> {
    (-1i32..=1)
        .flat_map(move |dy| (-1i32..=1).map(move |dx| (dx, dy)))
        .filter(|d| *d != (0, 0))
        .filter_map(move |(dx, dy)| {
            let x = i32::from(cell.x) + dx;
            let y = i32::from(cell.y) + dy;
            (x >= 0 && y >= 0).then(|| Cell::new(x as u32, y as u32))
        })
}

/// A march simulated without touching the state.
struct Walk {
    /// Cells entered, in order.
    cells: Vec<Cell>,
    cost: u32,
    stop: StopReason,
    /// Index of the waypoint being walked towards when the march stopped
    /// (`waypoints.len()` when every one was reached).
    next_waypoint: usize,
}

/// Extra condition ending a march early: within `radius` pixels of `point`.
struct Approach {
    point: [f32; 2],
    radius: f32,
}

/// Simulates the march of `army_id` along `waypoints`. `goal` is the
/// settlement the march may enter (hostile settlements otherwise stop it);
/// `ignore_zoc` is an enemy army whose zone of control does not stop it
/// (the target of an attack).
fn simulate(
    state: &CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    waypoints: &[Cell],
    goal: Option<&SettlementId>,
    ignore_zoc: Option<&ArmyId>,
    approach: Option<&Approach>,
) -> Option<Walk> {
    let army = state.armies.get(army_id)?;
    let grid: &NavGrid = data.navgrid();
    let hostile = state.hostile_settlement_cells(data, &army.faction);
    let zoc_px = data.free_movement_rules().zoc_radius_km as f32 * px_per_km(data);
    let start_point = state.army_point(data, army);
    let enemies: Vec<(ArmyId, [f32; 2])> = state
        .armies
        .iter()
        .filter(|(id, a)| {
            *id != army_id && Some(*id) != ignore_zoc && state.is_at_war(&army.faction, &a.faction)
        })
        .map(|(id, a)| (id.clone(), state.army_point(data, a)))
        // An army already inside a zone of control may walk out of it.
        .filter(|(_, p)| distance(*p, start_point) > zoc_px)
        .collect();
    let mut current = state.army_cell(data, army);
    let mut left = army.movement_left;
    let mut walk = Walk {
        cells: Vec::new(),
        cost: 0,
        stop: StopReason::Arrived,
        next_waypoint: 0,
    };
    if approach.is_some_and(|a| distance(start_point, a.point) <= a.radius) {
        return Some(walk);
    }
    while walk.next_waypoint < waypoints.len() {
        for next in line_cells(current, waypoints[walk.next_waypoint]) {
            let Some(step) = step_cost(grid, current, next) else {
                walk.stop = StopReason::Blocked;
                return Some(walk);
            };
            if let Some(settlement) = hostile.get(&next) {
                if Some(settlement) != goal {
                    walk.stop = StopReason::EnemySettlement {
                        settlement: settlement.clone(),
                    };
                    return Some(walk);
                }
            }
            if step > left {
                walk.stop = StopReason::OutOfMovement;
                return Some(walk);
            }
            left -= step;
            walk.cost += step;
            walk.cells.push(next);
            current = next;
            let point = next.center(grid);
            if let Some((enemy, _)) = enemies.iter().find(|(_, p)| distance(*p, point) <= zoc_px) {
                walk.stop = StopReason::EnemyZoneOfControl {
                    army: enemy.clone(),
                };
                return Some(walk);
            }
            if approach.is_some_and(|a| distance(point, a.point) <= a.radius) {
                return Some(walk);
            }
        }
        walk.next_waypoint += 1;
    }
    Some(walk)
}

/// Applies a simulated march: position, points, plan and general.
fn apply_walk(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    walk: &Walk,
    waypoints: &[Cell],
    destination: &MoveTarget,
) {
    let grid = data.navgrid();
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    army.movement_left = army.movement_left.saturating_sub(walk.cost);
    if let Some(last) = walk.cells.last() {
        army.position = ArmyPosition::field(last.center(grid));
    }
    if walk.stop == StopReason::OutOfMovement {
        army.planned_path = waypoints[walk.next_waypoint.min(waypoints.len())..].to_vec();
        army.destination = Some(destination.clone());
    } else {
        army.clear_plan();
    }
    if !walk.cells.is_empty() {
        crate::movement::move_general(state, data, army_id);
    }
}

/// Walks `army_id` along `waypoints` towards `destination` and resolves the
/// arrival (settlement entered, exact point reached).
fn march(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    waypoints: &[Cell],
    destination: &MoveTarget,
    events: &mut Vec<GameEvent>,
) -> Option<MoveReport> {
    let goal = match destination {
        MoveTarget::Settlement(id) => Some(id),
        MoveTarget::Point { .. } => None,
    };
    let walk = simulate(state, data, army_id, waypoints, goal, None, None)?;
    apply_walk(state, data, army_id, &walk, waypoints, destination);
    let grid = data.navgrid();
    let mut walked: Vec<[f32; 2]> = walk.cells.iter().map(|c| c.center(grid)).collect();
    let mut stop = walk.stop.clone();
    if stop == StopReason::Arrived {
        match destination {
            MoveTarget::Point { x, y } => {
                if let Some(army) = state.armies.get_mut(army_id) {
                    army.position = ArmyPosition::Field { x: *x, y: *y };
                }
                if walked.last() != Some(&[*x, *y]) {
                    walked.push([*x, *y]);
                }
            }
            MoveTarget::Settlement(id) => {
                if let Some(point) = data.settlement_point(id) {
                    walked.push(point);
                }
                stop = enter_settlement(state, data, army_id, id, events);
                if let Some(army) = state.armies.get_mut(army_id) {
                    army.clear_plan();
                }
            }
        }
    }
    let planned_path = state
        .armies
        .get(army_id)
        .map(|a| a.planned_path.clone())
        .unwrap_or_default();
    Some(MoveReport {
        army: army_id.clone(),
        walked,
        cost: walk.cost,
        stop,
        planned_path,
    })
}

/// `army_id` enters `settlement` (spec § 3.3): a hostile one is besieged,
/// or taken at once when nobody defends it (an unfortified village with a
/// garrison is stormed); any other one is a stop.
pub(crate) fn enter_settlement(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    settlement: &SettlementId,
    events: &mut Vec<GameEvent>,
) -> StopReason {
    let Some(army) = state.armies.get(army_id) else {
        return StopReason::Blocked;
    };
    let faction = army.faction.clone();
    let hostile = state.is_hostile_settlement(&faction, settlement);
    if hostile {
        if let Some(defender) = state.hostile_armies_at(&faction, settlement).first() {
            // An enemy army stands inside: stop at the gates.
            return StopReason::EnemyZoneOfControl {
                army: defender.clone(),
            };
        }
    }
    if let Some(army) = state.armies.get_mut(army_id) {
        army.position = ArmyPosition::Settlement(settlement.clone());
    }
    crate::movement::move_general(state, data, army_id);
    if !hostile {
        return StopReason::Stationed {
            settlement: settlement.clone(),
        };
    }
    let kind = state.settlement_kind(settlement);
    let garrison_empty = state
        .settlements
        .get(settlement)
        .is_none_or(|s| s.garrison.is_empty());
    if garrison_empty {
        crate::siege::capture(state, data, settlement, &faction, events);
        return StopReason::SettlementTaken {
            settlement: settlement.clone(),
        };
    }
    if kind == SettlementKind::Village {
        crate::siege::enter_village(state, data, army_id, events);
        if state
            .settlements
            .get(settlement)
            .is_some_and(|s| s.controller == faction)
        {
            return StopReason::SettlementTaken {
                settlement: settlement.clone(),
            };
        }
        return StopReason::SiegeStarted {
            settlement: settlement.clone(),
        };
    }
    if let Some(army) = state.armies.get_mut(army_id) {
        army.stance = Stance::Siege;
        army.movement_left = 0;
    }
    crate::siege::begin_siege(state, data, settlement, &faction, army_id, events);
    StopReason::SiegeStarted {
        settlement: settlement.clone(),
    }
}

impl CampaignState {
    fn own_army_checked(&self, faction: &FactionId, army: &ArmyId) -> Result<&Army, OrderError> {
        let entry = self
            .armies
            .get(army)
            .ok_or_else(|| OrderError::UnknownArmy(army.clone()))?;
        if &entry.faction != faction {
            return Err(OrderError::NotYourArmy(faction.clone()));
        }
        Ok(entry)
    }

    /// Order `MoveArmy` (lot M2): walks `army` towards `target` at once.
    pub(crate) fn order_move_army(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army: &ArmyId,
        target: MoveTarget,
        events: &mut Vec<GameEvent>,
    ) -> Result<MoveReport, OrderError> {
        let entry = self.own_army_checked(faction, army)?;
        if let MoveTarget::Settlement(id) = &target {
            if !self.settlements.contains_key(id) {
                return Err(OrderError::UnknownSettlement(id.clone()));
            }
            if entry.is_at(id) {
                return Ok(MoveReport {
                    army: army.clone(),
                    walked: Vec::new(),
                    cost: 0,
                    stop: StopReason::Stationed {
                        settlement: id.clone(),
                    },
                    planned_path: Vec::new(),
                });
            }
        }
        if entry.movement_left == 0 {
            return Err(OrderError::NoMovementLeft);
        }
        let point = self.target_point(data, &target).ok_or(OrderError::NoPath)?;
        let path = self
            .find_path(data, army, point)
            .ok_or(OrderError::NoPath)?;
        march(self, data, army, &path.waypoints, &target, events).ok_or(OrderError::NoPath)
    }

    /// Order `Attack` (lot M2): `army` closes in on `target` (whose own zone
    /// of control does not stop it) until within `engage_radius_km`, then
    /// fights at once. Refused when the target cannot be reached this turn.
    pub(crate) fn order_attack(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army: &ArmyId,
        target: &ArmyId,
        events: &mut Vec<GameEvent>,
    ) -> Result<(), OrderError> {
        let entry = self.own_army_checked(faction, army)?;
        let enemy = self
            .armies
            .get(target)
            .ok_or_else(|| OrderError::UnknownArmy(target.clone()))?;
        if !self.is_at_war(&entry.faction, &enemy.faction) {
            return Err(OrderError::NotAtWar);
        }
        if entry.movement_left == 0 {
            return Err(OrderError::NoMovementLeft);
        }
        let approach = Approach {
            point: self.army_point(data, enemy),
            radius: data.free_movement_rules().engage_radius_km as f32 * px_per_km(data),
        };
        let goal = enemy.settlement().cloned();
        let path = self
            .find_path(data, army, approach.point)
            .ok_or(OrderError::NoPath)?;
        let walk = simulate(
            self,
            data,
            army,
            &path.waypoints,
            goal.as_ref(),
            Some(target),
            Some(&approach),
        )
        .ok_or(OrderError::NoPath)?;
        let grid = data.navgrid();
        let end = walk.cells.last().map_or_else(
            || self.army_point(data, &self.armies[army]),
            |c| c.center(grid),
        );
        if distance(end, approach.point) > approach.radius {
            return Err(OrderError::OutOfRange);
        }
        let destination = MoveTarget::Point {
            x: end[0],
            y: end[1],
        };
        apply_walk(self, data, army, &walk, &path.waypoints, &destination);
        if let Some(a) = self.armies.get_mut(army) {
            a.movement_left = 0;
            a.clear_plan();
        }
        crate::movement::fight(self, data, army, target, events);
        Ok(())
    }

    /// Order `Embark` (lot M2): a port-to-port crossing along a `sea` edge
    /// of the settlement graph (C3); costs the whole turn.
    pub(crate) fn order_embark(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army: &ArmyId,
        to_port: &SettlementId,
        events: &mut Vec<GameEvent>,
    ) -> Result<(), OrderError> {
        let entry = self.own_army_checked(faction, army)?;
        let from = entry.settlement().cloned().ok_or(OrderError::NotInPort)?;
        if !data.settlements.contains_key(to_port) {
            return Err(OrderError::UnknownSettlement(to_port.clone()));
        }
        if !crate::movement::is_sea_crossing(data, &from, to_port) {
            return Err(OrderError::NotAdjacent {
                from: from.to_string(),
                to: to_port.to_string(),
            });
        }
        let full = self.army_grid_allowance(data, entry);
        if entry.movement_left < full {
            return Err(OrderError::NoMovementLeft);
        }
        let army_faction = entry.faction.clone();
        if let Some(a) = self.armies.get_mut(army) {
            a.movement_left = 0;
            a.clear_plan();
            a.position = ArmyPosition::Settlement(to_port.clone());
        }
        crate::movement::move_general(self, data, army);
        let province = self.settlement_province(to_port).cloned();
        if province
            .as_ref()
            .is_some_and(|p| self.is_hostile_territory(&army_faction, p))
        {
            crate::movement::land_on_hostile_shore(self, army, province.as_ref(), events);
        }
        if let Some(defender) =
            crate::movement::strongest(self, &self.hostile_armies_at(&army_faction, to_port))
        {
            // Land beside the port and fight the army holding it.
            if let (Some(point), Some(a)) =
                (data.settlement_point(to_port), self.armies.get_mut(army))
            {
                a.position = ArmyPosition::field(point);
            }
            crate::movement::fight(self, data, army, &defender, events);
        } else if self.is_hostile_settlement(&army_faction, to_port) {
            enter_settlement(self, data, army, to_port, events);
        }
        Ok(())
    }
}

/// Resumes the multi-turn marches (`planned_path`) of `faction`'s armies
/// (every faction when `None`), in army id order. A march whose path is no
/// longer passable is planned again towards its destination.
pub(crate) fn continue_marches(
    state: &mut CampaignState,
    data: &GameData,
    faction: Option<&FactionId>,
    events: &mut Vec<GameEvent>,
) -> Vec<MoveReport> {
    let ids: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| {
            !a.planned_path.is_empty()
                && a.movement_left > 0
                && faction.is_none_or(|f| &a.faction == f)
        })
        .map(|(id, _)| id.clone())
        .collect();
    let mut reports = Vec::new();
    for id in ids {
        let Some(army) = state.armies.get(&id) else {
            continue;
        };
        let waypoints = army.planned_path.clone();
        let destination = army.destination.clone().unwrap_or_else(|| {
            let last = waypoints.last().copied().unwrap_or(Cell::new(0, 0));
            let point = last.center(data.navgrid());
            MoveTarget::Point {
                x: point[0],
                y: point[1],
            }
        });
        let Some(mut report) = march(state, data, &id, &waypoints, &destination, events) else {
            continue;
        };
        if report.stop == StopReason::Blocked {
            let replanned = state
                .target_point(data, &destination)
                .and_then(|point| state.find_path(data, &id, point));
            match replanned {
                Some(path) => {
                    if let Some(again) =
                        march(state, data, &id, &path.waypoints, &destination, events)
                    {
                        report = again;
                    }
                }
                None => {
                    if let Some(a) = state.armies.get_mut(&id) {
                        a.clear_plan();
                    }
                }
            }
        }
        reports.push(report);
    }
    reports
}

/// Settlements whose cells lie within `radius_km` of `point` (vision,
/// contact), in id order.
pub fn settlements_near(
    data: &GameData,
    point: [f32; 2],
    radius_km: f64,
) -> BTreeSet<SettlementId> {
    let radius = radius_km as f32 * px_per_km(data);
    data.settlements
        .keys()
        .filter(|id| {
            data.settlement_point(id)
                .is_some_and(|p| distance(p, point) <= radius)
        })
        .cloned()
        .collect()
}
