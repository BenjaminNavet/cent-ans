//! The AI armies on the navigation grid (lot M3, spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 4).
//!
//! Strategy stays on the settlement graph (C3/C7a): [`GridPlanner::table`]
//! is its Dijkstra, cached for the faction's turn and keeping the zones of
//! control of stronger enemy armies out of the routes. Execution goes
//! through the grid: [`GridPlanner::march_orders`] turns the chosen route
//! into one `MoveArmy` per settlement-graph leg the army may walk this turn
//! (each leg a short A* on the grid), and an `Embark` at a port facing a sea
//! crossing. [`GridPlanner::attack_order`] engages an enemy army within the
//! army's bubble when the odds are good.
//!
//! Tuning: `data/ai/grid.json` ([`data_model::AiGrid`]).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::sync::{Arc, Mutex};

use data_model::{AiGrid, FactionId, GameData, SettlementId, PLAIN_COST};
use sim_campaign::movement::{edges, is_sea_crossing, path_to, Reach};
use sim_campaign::passage;
use sim_campaign::{Army, ArmyId, CampaignState, Order};

use crate::parallel::Mode;

/// A Dijkstra table of the settlement graph.
pub type Table = BTreeMap<SettlementId, Reach>;

/// An army at war with the planning faction.
struct Enemy {
    id: ArmyId,
    faction: FactionId,
    point: [f32; 2],
    power: f64,
    settlement: Option<SettlementId>,
    /// Settlements within `avoid_radius_km`.
    near: Vec<SettlementId>,
    /// EQ5: standing in lands the planning faction will not enter without
    /// right of passage (it is not pursued there).
    beyond_passage: bool,
}

/// Per faction and turn: the enemy armies, the settlements an army may not
/// pass through, and the cached route tables.
pub struct GridPlanner<'a> {
    state: &'a CampaignState,
    data: &'a GameData,
    faction: &'a FactionId,
    rules: &'a AiGrid,
    px_per_km: f32,
    enemies: Vec<Enemy>,
    /// Settlements held by an enemy or holding an enemy army: reached (a
    /// siege, a battle) but never passed through.
    stops: BTreeSet<SettlementId>,
    /// Lot DP2: settlements in the lands of a faction at peace that this
    /// faction's AI will not cross without right of passage
    /// (`passage::ai_may_trespass`), with that faction: never reached nor
    /// passed through. EQ5: places the faction holds itself are never
    /// forbidden, and a route starting in forbidden lands may cross those
    /// lands (the way out, [`GridPlanner::table`]).
    forbidden: BTreeMap<SettlementId, FactionId>,
    /// EQ5: settlements in lands of a faction at peace that this faction's
    /// AI may cross without right of passage: reached at a higher route
    /// cost (`AiGrid::trespass_route_factor`).
    crossable: BTreeSet<SettlementId>,
    /// EQ5: may this faction's AI cross the lands of each realm at peace
    /// without right of passage (`passage::ai_may_trespass`)?
    may_cross: Mutex<BTreeMap<FactionId, bool>>,
    /// EQ5: lands a road of the settlement graph runs through besides those
    /// of its two ends (sampled on the straight line, like the grid march):
    /// closed ones (owners) and whether it crosses lands open by temper.
    roads: Mutex<BTreeMap<(SettlementId, SettlementId), Arc<RoadLands>>>,
    /// Route tables by (start, budget, cap, avoided enemy armies).
    tables: Mutex<BTreeMap<TableKey, Arc<Table>>>,
}

/// (start, budget, cap, avoided enemy armies, homeward).
/// EQ5: foreign lands a road crosses on the way (see `GridPlanner::roads`).
#[derive(Default)]
struct RoadLands {
    closed: Vec<FactionId>,
    open: bool,
}

/// Sampling step (map pixels) of a road's straight line.
const ROAD_SAMPLE_PX: f32 = 8.0;

type TableKey = (SettlementId, u32, u32, Vec<usize>, bool);

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

impl<'a> GridPlanner<'a> {
    pub fn new(state: &'a CampaignState, data: &'a GameData, faction: &'a FactionId) -> Self {
        Self::with_mode(Mode::Sequential, state, data, faction)
    }

    /// [`GridPlanner::new`], the lands and enemy armies read on the
    /// planner's pool in [`Mode::Parallel`] (PB3f, same planner).
    pub fn with_mode(
        mode: Mode,
        state: &'a CampaignState,
        data: &'a GameData,
        faction: &'a FactionId,
    ) -> Self {
        let rules = &data.ai_grid;
        let px_per_km = sim_campaign::march::px_per_km(data);
        let avoid_px = rules.avoid_radius_km as f32 * px_per_km;
        // DC3: positions read in one walk (`settlement_px` is sorted like `settlements`).
        let points: Vec<(SettlementId, [f32; 2])> =
            if data.settlements.keys().eq(data.settlement_px.keys()) {
                data.settlement_px
                    .iter()
                    .map(|(id, p)| (id.clone(), *p))
                    .collect()
            } else {
                data.settlements
                    .keys()
                    .filter_map(|id| Some((id.clone(), data.settlement_point(id)?)))
                    .collect()
            };
        let mut crossing: BTreeMap<FactionId, bool> = BTreeMap::new();
        let mut may_cross = |owner: &FactionId| {
            *crossing
                .entry(owner.clone())
                .or_insert_with(|| passage::ai_may_trespass(state, data, faction, owner))
        };
        let mut forbidden: BTreeMap<SettlementId, FactionId> = BTreeMap::new();
        let mut crossable: BTreeSet<SettlementId> = BTreeSet::new();
        let foreign: Vec<&SettlementId> = state
            .settlements
            .iter()
            .filter(|(_, s)| &s.controller != faction)
            .map(|(id, _)| id)
            .collect();
        let owners = mode.map(&foreign, |id| {
            state
                .settlement_province(id)
                .and_then(|p| passage::trespassed_owner(state, faction, p))
        });
        for (id, owner) in foreign.into_iter().zip(owners) {
            let Some(owner) = owner else {
                continue;
            };
            if may_cross(&owner) {
                crossable.insert(id.clone());
            } else {
                forbidden.insert(id.clone(), owner);
            }
        }
        let hostile: Vec<(&ArmyId, &Army)> = state
            .armies
            .iter()
            .filter(|(_, a)| state.is_at_war(faction, &a.faction))
            // CV3-6: an army hidden in ambush is neither attacked nor
            // avoided: the AI does not know it is there.
            .filter(|(_, a)| !sim_campaign::posture::is_hidden_from(state, data, a, faction))
            .collect();
        let read = mode.map(&hostile, |(id, a)| {
            let point = state.army_point(data, a);
            let enemy = Enemy {
                id: (*id).clone(),
                faction: a.faction.clone(),
                point,
                power: state.army_power(data, id),
                settlement: a.settlement().cloned(),
                beyond_passage: false,
                near: points
                    .iter()
                    .filter(|(_, p)| distance(*p, point) <= avoid_px)
                    .map(|(s, _)| s.clone())
                    .collect(),
            };
            let owner = state
                .army_province(data, a)
                .and_then(|p| passage::trespassed_owner(state, faction, &p));
            (enemy, owner)
        });
        let enemies: Vec<Enemy> = read
            .into_iter()
            .map(|(mut enemy, owner)| {
                enemy.beyond_passage = owner.is_some_and(|owner| !may_cross(&owner));
                enemy
            })
            .collect();
        let mut stops: BTreeSet<SettlementId> = state
            .settlements
            .keys()
            .filter(|id| state.is_hostile_settlement(faction, id))
            .cloned()
            .collect();
        stops.extend(enemies.iter().filter_map(|e| e.settlement.clone()));
        GridPlanner {
            state,
            data,
            faction,
            rules,
            px_per_km,
            enemies,
            stops,
            forbidden,
            crossable,
            may_cross: Mutex::new(crossing),
            roads: Mutex::new(BTreeMap::new()),
            tables: Mutex::new(BTreeMap::new()),
        }
    }

    /// EQ5: may this faction's AI cross `owner`'s lands without passage?
    fn may_cross(&self, owner: &FactionId) -> bool {
        // PB3f: a pure memo, filled outside the lock (the prefetch threads
        // may ask at once; they compute the same answer).
        let known = self
            .may_cross
            .lock()
            .expect("planner cache")
            .get(owner)
            .copied();
        known.unwrap_or_else(|| {
            let open = passage::ai_may_trespass(self.state, self.data, self.faction, owner);
            self.may_cross
                .lock()
                .expect("planner cache")
                .insert(owner.clone(), open);
            open
        })
    }

    /// PB3f (ADR 0091): computes the route tables of `keys` (start, budget,
    /// cap, power) on the planner's pool ahead of the sequential army loop,
    /// which then finds them in the cache. The tables are a pure memo:
    /// filling it ahead changes no order.
    pub fn prefetch_tables(
        &self,
        mode: crate::parallel::Mode,
        keys: &[(SettlementId, u32, u32, f64)],
    ) where
        Self: Sync,
    {
        let mut seen = BTreeSet::new();
        let fresh: Vec<&(SettlementId, u32, u32, f64)> = keys
            .iter()
            .filter(|(start, budget, cap, power)| {
                seen.insert((start.clone(), *budget, *cap, self.avoided(*power)))
            })
            .collect();
        mode.map(&fresh, |(start, budget, cap, power)| {
            self.table(start, *budget, *cap, *power);
        });
    }

    /// EQ5: the foreign lands the road `from` → `to` runs through between
    /// its ends: a road between two open places may still clip a closed
    /// province (the Po valley roads through the Veronese).
    fn road_lands(&self, from: &SettlementId, to: &SettlementId) -> Arc<RoadLands> {
        let key = (from.clone(), to.clone());
        if let Some(road) = self.roads.lock().expect("planner cache").get(&key) {
            return Arc::clone(road);
        }
        let mut road = RoadLands::default();
        if let (Some(a), Some(b)) = (
            self.data.settlement_point(from),
            self.data.settlement_point(to),
        ) {
            let ends = [
                self.state.settlement_province(from),
                self.state.settlement_province(to),
            ];
            let length = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            let steps = (length / ROAD_SAMPLE_PX).ceil().max(1.0) as usize;
            let mut seen: BTreeSet<&data_model::ProvinceId> = BTreeSet::new();
            for i in 1..steps {
                let t = i as f32 / steps as f32;
                let Some(province) = self
                    .data
                    .province_at_point(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
                else {
                    continue;
                };
                if ends.contains(&Some(province)) || !seen.insert(province) {
                    continue;
                }
                let Some(owner) = passage::trespassed_owner(self.state, self.faction, province)
                else {
                    continue;
                };
                if self.may_cross(&owner) {
                    road.open = true;
                } else if !road.closed.contains(&owner) {
                    road.closed.push(owner);
                }
            }
        }
        let road = Arc::new(road);
        self.roads
            .lock()
            .expect("planner cache")
            .insert(key, Arc::clone(&road));
        road
    }

    /// Indices of the enemy armies an army of `power` keeps away from.
    fn avoided(&self, power: f64) -> Vec<usize> {
        self.enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.power > self.rules.avoid_ratio * power)
            .map(|(i, _)| i)
            .collect()
    }

    /// Shortest routes on the settlement graph from `start` for an army of
    /// `power` (cached for the turn). Like `movement::dijkstra`, hostile
    /// places and places holding an enemy army end a route; so do the
    /// places inside the zone of control of an enemy army stronger than
    /// `power` (lot M3): the army may head for them, never through them.
    /// EQ5: from a start in forbidden lands (an army caught there by a
    /// peace), the lands of that same owner are open: the army can leave.
    pub fn table(&self, start: &SettlementId, budget: u32, cap: u32, power: f64) -> Arc<Table> {
        self.routes(start, budget, cap, power, false)
    }

    /// EQ5: like [`GridPlanner::table`], but crossing the lands closed
    /// without right of passage: the way home of an army stranded abroad
    /// (a short trespass on the way beats camping there for years).
    pub fn homeward_table(
        &self,
        start: &SettlementId,
        budget: u32,
        cap: u32,
        power: f64,
    ) -> Arc<Table> {
        self.routes(start, budget, cap, power, true)
    }

    fn routes(
        &self,
        start: &SettlementId,
        budget: u32,
        cap: u32,
        power: f64,
        homeward: bool,
    ) -> Arc<Table> {
        let avoided = self.avoided(power);
        let key = (start.clone(), budget, cap, avoided, homeward);
        if let Some(table) = self.tables.lock().expect("planner cache").get(&key) {
            return Arc::clone(table);
        }
        let blocked: BTreeSet<&SettlementId> = key
            .3
            .iter()
            .flat_map(|i| self.enemies[*i].near.iter())
            .collect();
        let way_out = self
            .state
            .settlement_province(start)
            .and_then(|p| passage::trespassed_owner(self.state, self.faction, p));
        let way_out = way_out.as_ref();
        let mut best: Table = BTreeMap::new();
        let mut heap = BinaryHeap::new();
        best.insert(
            start.clone(),
            Reach {
                cost: 0,
                previous: None,
            },
        );
        heap.push(Reverse((0u32, start.clone())));
        while let Some(Reverse((cost, current))) = heap.pop() {
            if best.get(&current).is_some_and(|r| r.cost < cost) {
                continue;
            }
            if &current != start && (self.stops.contains(&current) || blocked.contains(&current)) {
                continue;
            }
            for (next, edge) in edges(self.data, &current) {
                if !homeward
                    && self
                        .forbidden
                        .get(&next)
                        .is_some_and(|owner| Some(owner) != way_out)
                {
                    continue;
                }
                let road = self.road_lands(&current, &next);
                if !homeward && road.closed.iter().any(|owner| Some(owner) != way_out) {
                    continue;
                }
                let mut step = edge.min(cap.max(1));
                if self.crossable.contains(&next) || road.open {
                    step = (f64::from(step) * self.rules.trespass_route_factor).round() as u32;
                }
                let total = cost + step;
                if total > budget {
                    continue;
                }
                if best.get(&next).is_none_or(|r| total < r.cost) {
                    best.insert(
                        next.clone(),
                        Reach {
                            cost: total,
                            previous: Some(current.clone()),
                        },
                    );
                    heap.push(Reverse((total, next)));
                }
            }
        }
        let table = Arc::new(best);
        self.tables
            .lock()
            .expect("planner cache")
            .insert(key, Arc::clone(&table));
        table
    }

    /// `Attack` on the nearest enemy army within `army_id`'s bubble that
    /// its side outweighs by `attack_ratio` (lot M3). Enemies behind the
    /// walls of a place hostile to us are left to the siege planner.
    ///
    /// Our side is `army_id` itself plus every friendly army within the
    /// engagement radius of the enemy: armies merely sharing its anchor are
    /// not counted beforehand (they would be counted twice).
    pub fn attack_order(&self, army_id: &ArmyId) -> Option<Order> {
        let state = self.state;
        let army = state.armies.get(army_id)?;
        let power = state.army_power(self.data, army_id);
        if army.movement_left == 0 || power <= 0.0 {
            return None;
        }
        let grid = self.data.navgrid();
        let reach_km = f64::from(army.movement_left) / f64::from(PLAIN_COST) * grid.cell_km;
        let reach_px = (reach_km * self.rules.attack_reach_share) as f32 * self.px_per_km;
        let engage_px = self.data.free_movement_rules().engage_radius_km as f32 * self.px_per_km;
        let here = state.army_point(self.data, army);
        self.enemies
            .iter()
            .filter(|e| !e.beyond_passage)
            .filter(|e| {
                e.settlement
                    .as_ref()
                    .is_none_or(|s| !state.is_hostile_settlement(self.faction, s))
            })
            .filter_map(|e| {
                let d = distance(here, e.point);
                if d > reach_px {
                    return None;
                }
                // Sides at the point of contact: the enemy's army and every
                // army of its side there; ours and our allies' there.
                let (mut ours, mut theirs) = (power, 0.0);
                for (id, other) in &state.armies {
                    if id == army_id
                        || distance(state.army_point(self.data, other), e.point) > engage_px
                    {
                        continue;
                    }
                    if state.is_allied(&e.faction, &other.faction)
                        && state.is_at_war(&other.faction, self.faction)
                    {
                        theirs += state.army_power(self.data, id);
                    } else if state.is_allied(self.faction, &other.faction)
                        && state.is_at_war(&other.faction, &e.faction)
                    {
                        ours += state.army_power(self.data, id);
                    }
                }
                let needed = self.rules.attack_ratio * theirs.max(e.power);
                if ours < needed {
                    return None;
                }
                // RC (ADR 0117): forcing a bridge, ford or ferry held on
                // the far bank weakens our side as in the resolver.
                if let Some(site) =
                    sim_campaign::river_crossing::crossing_between(self.data, here, e.point)
                {
                    ours *= site
                        .effect()
                        .attacker_factor(&self.data.river_crossing_rules);
                }
                (ours >= needed).then_some((d, &e.id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)))
            .map(|(_, target)| Order::Attack {
                army: army_id.clone(),
                target_army: target.clone(),
            })
    }

    /// Orders walking `army` from `anchor` towards `target` along `table`
    /// (lot M3): one `MoveArmy` per settlement-graph leg, up to the first
    /// leg beyond this season's allowance or `max_legs_per_turn`; the grid
    /// march stops by itself when the points run out. A sea crossing ends
    /// the legs: the army walks to the port, and embarks there once it
    /// stands in it with a full season ahead (`Embark` costs the turn).
    pub fn march_orders(
        &self,
        army_id: &ArmyId,
        army: &Army,
        anchor: &SettlementId,
        target: &SettlementId,
        table: &Table,
    ) -> Vec<Order> {
        let cap = self.state.army_movement_allowance(self.data, army);
        self.march_orders_with_cap(army_id, army, anchor, target, table, cap)
    }

    /// [`GridPlanner::march_orders`] with the allowance `cap` (CV3-6: a
    /// forced march walks further).
    pub fn march_orders_with_cap(
        &self,
        army_id: &ArmyId,
        army: &Army,
        anchor: &SettlementId,
        target: &SettlementId,
        table: &Table,
        cap: u32,
    ) -> Vec<Order> {
        if army.is_at(target) {
            return Vec::new();
        }
        let Some(path) = path_to(table, target) else {
            return Vec::new();
        };
        if path.is_empty() {
            // The target is the anchor itself (an army in the field nearby).
            return vec![Order::move_to(army_id.clone(), target.clone())];
        }
        let mut orders = Vec::new();
        let mut previous = anchor.clone();
        for node in path {
            if is_sea_crossing(self.data, &previous, &node) {
                if orders.is_empty() {
                    orders.push(if army.is_at(&previous) {
                        Order::Embark {
                            army: army_id.clone(),
                            to_port: node,
                        }
                    } else {
                        Order::move_to(army_id.clone(), previous)
                    });
                }
                break;
            }
            let beyond = table.get(&node).is_none_or(|r| r.cost > cap);
            orders.push(Order::move_to(army_id.clone(), node.clone()));
            if beyond || orders.len() >= self.rules.max_legs_per_turn {
                break;
            }
            previous = node;
        }
        orders
    }
}
