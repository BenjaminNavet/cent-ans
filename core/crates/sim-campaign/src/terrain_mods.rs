//! RG9 (ADR 0343): per-army cost grid.
//!
//! The baked `navgrid.png` is shared by every army and never edited. What
//! depends on the state or on the army is applied at path-finding time, on a
//! *derived* grid ([`data_model::NavGrid::derive`], same shape, same
//! impassable cells, cached per set of modifiers by
//! [`GameData::derived_navgrid`]):
//!
//! - **Province roads**: the `RoadMovement` effects of the buildings of a
//!   province (Chemin empierré −15 %, Route royale −30 %) scale the cost of
//!   every cell of the province, on top of the baked roads.
//! - **Passes**: the cells along a `pass` of `crossings.json` cost
//!   `pass_cost_factor` of the mountain cost they were forced to.
//! - **Winter**: mountain and pass cells cost `winter_mountain_factor`
//!   (softened by the general's `WinterMovement` effects: Montagnard).
//!
//! Roads and passes cumulate down to `road_floor_factor` of the grid cost.

use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use data_model::{
    EffectKind, FreeMovementRules, GameData, NavGrid, ProvinceId, ProvinceRaster, IMPASSABLE,
};
use serde::{Deserialize, Serialize};

use crate::navigation::{diagonal_cost, line_cells, Cell};
use crate::state::{Army, CampaignState, Season};

/// A cost grid for one army: the shared baked grid, or a derived one.
pub enum ArmyGrid<'a> {
    Base(&'a NavGrid),
    Derived(Arc<NavGrid>),
}

impl Deref for ArmyGrid<'_> {
    type Target = NavGrid;

    fn deref(&self) -> &NavGrid {
        match self {
            ArmyGrid::Base(grid) => grid,
            ArmyGrid::Derived(grid) => grid,
        }
    }
}

/// What changes the cost of the cells for an army.
#[derive(Debug, Clone, PartialEq)]
pub struct CostMods {
    /// Cost multiplier of every cell of a province (roads), below 1.
    pub routes: BTreeMap<ProvinceId, f64>,
    /// Winter multiplier of mountain and pass cells (1 outside winter or
    /// for a general who ignores it).
    pub winter: f64,
}

impl CostMods {
    fn key(&self, rules: &FreeMovementRules) -> u64 {
        let mut hasher = DefaultHasher::new();
        milli(self.winter).hash(&mut hasher);
        for (province, factor) in &self.routes {
            province.hash(&mut hasher);
            milli(*factor).hash(&mut hasher);
        }
        milli(rules.pass_cost_factor).hash(&mut hasher);
        milli(rules.road_floor_factor).hash(&mut hasher);
        rules.winter_mountain_min_cost.hash(&mut hasher);
        hasher.finish()
    }
}

fn milli(value: f64) -> i64 {
    (value * 1000.0).round() as i64
}

/// Decomposition of a march cost into its causes (map tooltip).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Cost on the baked grid (terrain, baked roads and minor rivers).
    pub terrain: u32,
    /// Saved by the roads of the provinces crossed (≤ 0).
    pub route: i32,
    /// Saved on mountain passes (≤ 0).
    pub pass: i32,
    /// Added by winter on mountains and passes (≥ 0).
    pub winter: i32,
}

impl CostBreakdown {
    /// Cost on the derived grid: what the march really costs.
    pub fn total(&self) -> u32 {
        (self.terrain as i64
            + i64::from(self.route)
            + i64::from(self.pass)
            + i64::from(self.winter))
        .max(0) as u32
    }
}

/// Cost of a cell of grid cost `base` under the modifiers (never above 254,
/// never below 1; impassable stays impassable).
pub fn effective_cost(
    rules: &FreeMovementRules,
    base: u8,
    route: f64,
    pass: bool,
    winter: f64,
) -> u8 {
    if base == IMPASSABLE {
        return base;
    }
    let mut reduction = route;
    if pass {
        reduction *= rules.pass_cost_factor;
    }
    if reduction < 1.0 {
        reduction = reduction.max(rules.road_floor_factor.min(1.0));
    }
    let seasonal = if pass || base >= rules.winter_mountain_min_cost {
        winter
    } else {
        1.0
    };
    let cost = (f64::from(base) * reduction * seasonal).round();
    (cost as i64).clamp(1, i64::from(IMPASSABLE) - 1) as u8
}

/// Grid cells of the passes of `data` (the polyline, one cell on each
/// side), as indices into the cost array.
pub fn pass_cells(data: &GameData, grid: &NavGrid) -> Vec<usize> {
    let Some(map) = data.map.as_ref() else {
        return Vec::new();
    };
    let mut cells = Vec::new();
    for pass in &data.passes {
        let points: Vec<Cell> = pass
            .route
            .iter()
            .filter_map(|[lon, lat]| map.lonlat_to_px(*lon, *lat))
            .map(|[x, y]| Cell::of_point(grid, [x as f32, y as f32]))
            .collect();
        let mut line = Vec::new();
        for pair in points.windows(2) {
            line.push(pair[0]);
            line.extend(line_cells(pair[0], pair[1]));
        }
        if points.len() == 1 {
            line.push(points[0]);
        }
        for cell in line {
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (x, y) = (i64::from(cell.x) + dx, i64::from(cell.y) + dy);
                    if grid.contains(x, y) && grid.passable(x, y) {
                        cells.push(grid.index(x as u32, y as u32));
                    }
                }
            }
        }
    }
    cells.sort_unstable();
    cells.dedup();
    cells
}

/// Whether cell `index` is on a pass, with the cells kept sorted.
fn on_pass(passes: &[usize], index: usize, base: u8, rules: &FreeMovementRules) -> bool {
    // Only mountain-like cells of the corridor get the pass discount: the
    // dilated corridor also touches valley cells.
    base >= rules.winter_mountain_min_cost && passes.binary_search(&index).is_ok()
}

fn build_grid(
    data: &GameData,
    base: &NavGrid,
    provinces: Option<&ProvinceRaster>,
    mods: &CostMods,
) -> NavGrid {
    let rules = data.free_movement_rules();
    let passes = if rules.pass_cost_factor < 1.0 || mods.winter > 1.0 {
        pass_cells(data, base)
    } else {
        Vec::new()
    };
    // Route multiplier by raster index.
    let by_index: Vec<f64> = match provinces {
        Some(raster) if !mods.routes.is_empty() => raster
            .ids
            .iter()
            .map(|id| {
                id.as_ref()
                    .and_then(|id| mods.routes.get(id))
                    .copied()
                    .unwrap_or(1.0)
            })
            .collect(),
        _ => Vec::new(),
    };
    let scale = base.scale as usize;
    let mut costs = base.costs.clone();
    for (index, cost) in costs.iter_mut().enumerate() {
        let value = *cost;
        if value == IMPASSABLE {
            continue;
        }
        let route = match provinces {
            Some(raster) if !by_index.is_empty() => {
                let (cx, cy) = (index % base.width as usize, index / base.width as usize);
                let (px, py) = (cx * scale + scale / 2, cy * scale + scale / 2);
                if px < raster.width as usize && py < raster.height as usize {
                    let raster_index = raster.indices[py * raster.width as usize + px];
                    by_index
                        .get(usize::from(raster_index))
                        .copied()
                        .unwrap_or(1.0)
                } else {
                    1.0
                }
            }
            _ => 1.0,
        };
        let pass = on_pass(&passes, index, value, rules);
        if route < 1.0 || pass || (mods.winter > 1.0 && value >= rules.winter_mountain_min_cost) {
            *cost = effective_cost(rules, value, route, pass, mods.winter);
        }
    }
    base.derive(costs)
}

impl CampaignState {
    /// Cost multiplier of the cells of every province with a road building
    /// (`RoadMovement` effects of its buildings, weighed by place kind).
    pub fn province_route_factors(&self, data: &GameData) -> BTreeMap<ProvinceId, f64> {
        let mut percents: BTreeMap<ProvinceId, f64> = BTreeMap::new();
        for settlement in self.settlements.values() {
            let weight = f64::from(crate::buildings::province_effect_percent(
                data,
                settlement.kind,
            )) / 100.0;
            for building in settlement
                .buildings
                .iter()
                .filter_map(|id| data.buildings.get(id))
            {
                for effect in &building.effects {
                    if effect.effect == EffectKind::RoadMovement {
                        *percents.entry(settlement.province.clone()).or_default() +=
                            effect.value * weight;
                    }
                }
            }
        }
        percents
            .into_iter()
            .filter(|(_, percent)| *percent != 0.0)
            .map(|(province, percent)| (province, (1.0 + percent / 100.0).max(0.1)))
            .collect()
    }

    /// Winter multiplier of mountain and pass cells for `army`: 1 outside
    /// winter; the general's `WinterMovement` percent (Montagnard: −100)
    /// scales the malus.
    pub fn army_winter_factor(&self, data: &GameData, army: &Army) -> f64 {
        if self.season != Season::Winter {
            return 1.0;
        }
        let malus = data.free_movement_rules().winter_mountain_factor.max(1.0) - 1.0;
        let percent = army.general.as_ref().map_or(0.0, |id| {
            crate::skills::character_effects(self, data, id)[EffectKind::WinterMovement].percent
        });
        1.0 + malus * (1.0 + percent / 100.0).max(0.0)
    }

    /// The modifiers that apply to `army` now.
    pub fn army_cost_mods(&self, data: &GameData, army: &Army) -> CostMods {
        CostMods {
            routes: self.province_route_factors(data),
            winter: self.army_winter_factor(data, army),
        }
    }

    /// The cost grid `army` marches on: the baked grid with the province
    /// roads, passes and winter of the moment (cached).
    pub fn army_navgrid<'a>(&self, data: &'a GameData, army: &Army) -> ArmyGrid<'a> {
        let mods = self.army_cost_mods(data, army);
        grid_for(data, &mods)
    }

    /// Cost of the cells of `path` (the cells entered, in order, from
    /// `start`) split by cause, for `army`.
    pub fn breakdown_of_cells(
        &self,
        data: &GameData,
        army: &Army,
        start: Cell,
        cells: &[Cell],
    ) -> CostBreakdown {
        let mods = self.army_cost_mods(data, army);
        let rules = data.free_movement_rules();
        let base = data.navgrid();
        let passes = if rules.pass_cost_factor < 1.0 || mods.winter > 1.0 {
            pass_cells(data, base)
        } else {
            Vec::new()
        };
        let route_of = |cell: Cell| -> f64 {
            let [px, py] = cell.center(base);
            data.province_at_point(px, py)
                .and_then(|id| mods.routes.get(id))
                .copied()
                .unwrap_or(1.0)
        };
        let mut out = CostBreakdown::default();
        let mut previous = start;
        for &cell in cells {
            let index = base.index(u32::from(cell.x), u32::from(cell.y));
            let value = base.costs[index];
            if value == IMPASSABLE {
                break;
            }
            let diagonal = cell.x != previous.x && cell.y != previous.y;
            let step = |v: u8| -> i64 {
                if diagonal {
                    i64::from(diagonal_cost(v))
                } else {
                    i64::from(v)
                }
            };
            let route = route_of(cell);
            let pass = on_pass(&passes, index, value, rules);
            let after_route = effective_cost(rules, value, route, false, 1.0);
            let after_pass = effective_cost(rules, value, route, pass, 1.0);
            let after_winter = effective_cost(rules, value, route, pass, mods.winter);
            out.terrain += step(value) as u32;
            out.route += (step(after_route) - step(value)) as i32;
            out.pass += (step(after_pass) - step(after_route)) as i32;
            out.winter += (step(after_winter) - step(after_pass)) as i32;
            previous = cell;
        }
        out
    }
}

/// The grid for the modifiers `mods` (the base grid when they change nothing).
pub fn grid_for<'a>(data: &'a GameData, mods: &CostMods) -> ArmyGrid<'a> {
    let rules = data.free_movement_rules();
    let nothing = mods.routes.is_empty()
        && mods.winter <= 1.0
        && (rules.pass_cost_factor >= 1.0 || data.passes.is_empty());
    if nothing {
        return ArmyGrid::Base(data.navgrid());
    }
    let key = mods.key(rules);
    ArmyGrid::Derived(data.derived_navgrid(key, |base, provinces| {
        build_grid(data, base, provinces, mods)
    }))
}
