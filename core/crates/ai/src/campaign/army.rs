//! Military planning: one objective per army (defend, besiege, raid,
//! regroup, retreat), after the stances, the merges and the garrisons.
//!
//! [`Fleet`] holds what the faction's armies share during the step (who is
//! already defended or targeted, the enemies already engaged); [`ArmyTurn`]
//! holds what one army knows about itself. [`Fleet::plan_army`] runs the
//! named steps in order; a step that decides the army's turn returns
//! `ControlFlow::Break`.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;
use std::sync::Arc;

use data_model::entities::ai_campaign::MilitaryRules;
use data_model::{ProvinceId, SettlementId, SettlementKind};
use sim_campaign::movement::{is_sea_crossing, points_per_step, Reach};
use sim_campaign::passage;
use sim_campaign::{Army, ArmyId, Order, Season, Stance};

use super::Context;
use crate::grid::Table;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Objective {
    Defend,
    Siege,
    Raid,
    Regroup,
    Retreat,
}

/// Everything one army knows about its own turn.
struct ArmyTurn<'a> {
    id: &'a ArmyId,
    army: &'a Army,
    /// The settlement standing for the army on the settlement graph.
    anchor: SettlementId,
    /// Power of the armies anchored on the same settlement.
    power: f64,
    /// Movement allowance of the turn.
    cap: u32,
    /// Routes from the anchor within the planning range.
    table: Arc<Table>,
    besieging: bool,
    /// Under [`MilitaryRules::retreat_strength`] of its maximum strength.
    broken: bool,
    /// Province the army stands in.
    here: Option<ProvinceId>,
    /// Standing without right of passage in the lands of a realm at peace.
    trespassing: bool,
    /// A siege that holds out far beyond patience.
    hopeless: bool,
    /// An attack order was issued this turn.
    attacked: bool,
    choice: Option<(Objective, SettlementId)>,
}

/// What the faction's armies share while they are planned.
struct Fleet<'c, 'a, 'o> {
    ctx: &'c Context<'a>,
    rules: &'a MilitaryRules,
    orders: &'o mut Vec<Order>,
    /// Own armies, strongest first.
    armies: Vec<(ArmyId, f64)>,
    /// Total power of the own armies anchored on each settlement.
    power_by_anchor: BTreeMap<SettlementId, f64>,
    /// Armies folded into another one this turn.
    merged: BTreeSet<ArmyId>,
    /// Armies whose units are being dismissed this turn (their unit indices
    /// stay untouched).
    disbanding: BTreeSet<ArmyId>,
    /// Armies losing every unit this turn: gone before any later order.
    emptied: BTreeSet<ArmyId>,
    /// Planning range, in movement points.
    range: u32,
    /// Movement points of one province step.
    step: f64,
    /// A throne claim makes every province of that crown a claimed target.
    claims: BTreeSet<ProvinceId>,
    /// Last provinces of an enemy realm: left to the peace table.
    last_bastions: BTreeSet<ProvinceId>,
    enemy_capitals: BTreeSet<SettlementId>,
    my_capital: Option<SettlementId>,
    largest: Option<(ArmyId, f64)>,
    /// A siege of ours or against us is in progress (no encounter detours).
    realm_besieged: bool,
    defended: BTreeSet<SettlementId>,
    targeted: BTreeSet<SettlementId>,
    /// Enemies another of our armies attacks this turn (one attack each).
    engaged: Vec<[f32; 2]>,
}

pub(super) fn plan_armies(ctx: &Context, orders: &mut Vec<Order>) {
    let mut fleet = Fleet::new(ctx, orders);
    fleet.merge_colocated();
    fleet.prefetch_routes();
    let ids: Vec<ArmyId> = fleet
        .armies
        .iter()
        .map(|(id, _)| id)
        .filter(|id| !fleet.merged.contains(*id) && !fleet.emptied.contains(*id))
        .cloned()
        .collect();
    for id in &ids {
        let _ = fleet.plan_army(id);
    }
}

impl<'c, 'a, 'o> Fleet<'c, 'a, 'o> {
    fn new(ctx: &'c Context<'a>, orders: &'o mut Vec<Order>) -> Self {
        let state = ctx.state;
        let data = ctx.data;
        let mut armies: Vec<(ArmyId, f64)> = state
            .armies
            .iter()
            .filter(|(_, a)| &a.faction == ctx.faction)
            .map(|(id, _)| (id.clone(), state.army_power(data, id)))
            .collect();
        armies.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut power_by_anchor: BTreeMap<SettlementId, f64> = BTreeMap::new();
        for (id, power) in &armies {
            if let Some(anchor) = ctx.anchors.get(id) {
                *power_by_anchor.entry(anchor.clone()).or_insert(0.0) += power;
            }
        }
        let rules = &ctx.rules.military;
        let step = points_per_step(data).max(1.0);
        let range = (f64::from(rules.planning_range) * step).round() as u32;
        let last_bastions = ctx
            .enemies
            .iter()
            .filter(|e| !e.is_rebels())
            .flat_map(|enemy| {
                let held: Vec<ProvinceId> = state
                    .provinces
                    .keys()
                    .filter(|id| state.holds_province(enemy, id))
                    .cloned()
                    .collect();
                if held.len() <= super::LAST_BASTIONS {
                    held
                } else {
                    Vec::new()
                }
            })
            .collect();
        let enemy_capitals = ctx
            .enemies
            .iter()
            .filter_map(|e| state.faction_capital_city(e).cloned())
            .collect();
        let realm_besieged = state.settlements.values().any(|s| {
            s.siege
                .as_ref()
                .is_some_and(|siege| &s.controller == ctx.faction || &siege.attacker == ctx.faction)
        });
        let disbanding: BTreeSet<ArmyId> = orders
            .iter()
            .filter_map(|o| match o {
                Order::DisbandUnit {
                    army: Some(army), ..
                } => Some(army.clone()),
                _ => None,
            })
            .collect();
        let emptied = disbanding
            .iter()
            .filter(|army| {
                let dismissed = orders
                    .iter()
                    .filter(|o| matches!(o, Order::DisbandUnit { army: Some(a), .. } if a == *army))
                    .count();
                state
                    .armies
                    .get(*army)
                    .is_some_and(|a| dismissed >= a.units.len())
            })
            .cloned()
            .collect();
        Fleet {
            ctx,
            rules,
            orders,
            largest: armies.first().cloned(),
            armies,
            power_by_anchor,
            merged: BTreeSet::new(),
            disbanding,
            emptied,
            range,
            step,
            claims: sim_campaign::diplomacy::claimed_provinces(state, ctx.faction),
            last_bastions,
            enemy_capitals,
            my_capital: state.faction_seat(ctx.faction),
            realm_besieged,
            defended: BTreeSet::new(),
            targeted: BTreeSet::new(),
            engaged: Vec::new(),
        }
    }

    /// Merges armies stationed together into the strongest one (within the
    /// unit cap; an army that does not fit stays apart and takes the next
    /// ones).
    fn merge_colocated(&mut self) {
        let state = self.ctx.state;
        let cap = self.ctx.data.army_rules.cap();
        let mut by_location: BTreeMap<SettlementId, Vec<(ArmyId, usize)>> = BTreeMap::new();
        for (id, _) in &self.armies {
            let Some(location) = state.armies[id].settlement().cloned() else {
                continue;
            };
            let size = state.armies[id].units.len();
            let targets = by_location.entry(location).or_default();
            match targets.iter_mut().find(|(_, units)| *units + size <= cap) {
                Some((target, units)) => {
                    self.orders.push(Order::MergeArmies {
                        source: id.clone(),
                        target: target.clone(),
                    });
                    *units += size;
                    self.merged.insert(id.clone());
                }
                None => targets.push((id.clone(), size)),
            }
        }
    }

    /// PB3f: the route tables the loop asks for, computed ahead on the
    /// planner's pool (a memo of the grid planner: same tables).
    fn prefetch_routes(&self) {
        let ctx = self.ctx;
        let keys: Vec<(SettlementId, u32, u32, f64)> = self
            .armies
            .iter()
            .filter(|(id, _)| !self.merged.contains(id))
            .filter_map(|(id, _)| {
                let anchor = ctx.anchors.get(id)?;
                let cap = ctx
                    .state
                    .army_movement_allowance(ctx.data, &ctx.state.armies[id]);
                Some((anchor.clone(), self.range, cap, self.power_at(anchor)))
            })
            .collect();
        ctx.grid.prefetch_tables(ctx.mode, &keys);
    }

    fn power_at(&self, settlement: &SettlementId) -> f64 {
        self.power_by_anchor.get(settlement).copied().unwrap_or(0.0)
    }

    /// Province steps of a route cost.
    fn steps(&self, cost: u32) -> f64 {
        f64::from(cost) / self.step
    }

    fn push_stance(&mut self, army: &ArmyId, stance: Stance) {
        self.orders.push(Order::SetStance {
            army: army.clone(),
            stance,
        });
    }

    /// Orders `stance` unless the army already holds it.
    fn ensure_stance(&mut self, turn: &ArmyTurn, stance: Stance) {
        if turn.army.stance != stance {
            self.push_stance(turn.id, stance);
        }
    }

    fn threat_here(&self, turn: &ArmyTurn) -> f64 {
        turn.here.as_ref().map_or(0.0, |p| self.ctx.threat(p))
    }

    /// The army's turn, step by step; `Break` as soon as a step settles it.
    fn plan_army(&mut self, id: &ArmyId) -> ControlFlow<()> {
        let mut turn = self.open_turn(id)?;
        self.pick_retreat(&mut turn);
        self.give_up_hopeless(&mut turn);
        self.leave_ambush(&turn)?;
        self.rest_or_leave_camp(&turn)?;
        self.attack_in_bubble(&mut turn);
        self.lie_in_wait(&turn)?;
        self.keep_siege(&turn)?;
        self.pick_defence(&mut turn);
        self.pick_siege(&mut turn);
        self.pick_raid(&mut turn);
        self.go_home_from_trespass(&turn)?;
        self.pick_regroup(&mut turn);
        self.pick_return_home(&mut turn);
        self.entrench(&turn)?;
        self.detour_to_encounter(&turn)?;
        let (objective, target) = self.settle_idle(&mut turn)?;
        self.march(&turn, objective, target)
    }

    /// Garrison duty of an army standing in a place of its own, then its
    /// context. `Break` when the army has no anchor or joined a garrison.
    fn open_turn(&mut self, id: &ArmyId) -> ControlFlow<(), ArmyTurn<'a>> {
        let ctx = self.ctx;
        let (state, data) = (ctx.state, ctx.data);
        let (id, army) = state.armies.get_key_value(id).expect("own army");
        let trespassing = state
            .army_province(data, army)
            .and_then(|p| passage::trespassed_owner(state, ctx.faction, &p))
            .is_some();
        if trespassing && !self.disbanding.contains(id) && !ctx.at_war() {
            if let Some(order) = trespasser_garrison_order(ctx, id) {
                self.orders.push(order);
                return ControlFlow::Break(());
            }
        }
        if !self.disbanding.contains(id) {
            if let Some(order) = garrison_order(ctx, id) {
                self.orders.push(order);
            }
        }
        let Some(anchor) = ctx.anchors.get(id).cloned() else {
            return ControlFlow::Break(());
        };
        let power = self.power_at(&anchor);
        let strength: u32 = army.units.iter().map(|u| u.strength).sum();
        let max_strength: u32 = army.units.iter().map(|u| u.max_strength).sum();
        let cap = state.army_movement_allowance(data, army);
        // Lot M3: cached for the turn; stronger enemy armies are avoided.
        let table = ctx.grid.table(&anchor, self.range, cap, power);
        let besieging = army
            .settlement()
            .and_then(|s| state.settlements.get(s))
            .and_then(|s| s.siege.as_ref())
            .is_some_and(|s| &s.attacker == ctx.faction);
        ControlFlow::Continue(ArmyTurn {
            id,
            army,
            anchor,
            power,
            cap,
            table,
            besieging,
            broken: max_strength > 0
                && f64::from(strength) < self.rules.retreat_strength * f64::from(max_strength),
            here: state.army_province(data, army),
            trespassing,
            hopeless: false,
            attacked: false,
            choice: None,
        })
    }

    /// Retreat when broken or starving in winter: the nearest place of its
    /// own the enemy cannot overrun.
    fn pick_retreat(&self, turn: &mut ArmyTurn) {
        let ctx = self.ctx;
        let starving = ctx.state.season == Season::Winter
            && turn.army.supply < self.rules.starving_supply
            && turn
                .here
                .as_ref()
                .is_some_and(|p| ctx.state.is_hostile_territory(ctx.faction, p));
        if (turn.broken || starving) && !turn.besieging {
            let home = turn
                .table
                .iter()
                .filter(|(id, _)| ctx.owns_settlement(id) && ctx.threat_at(id) < turn.power)
                .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
                .map(|(id, _)| id.clone());
            if let Some(home) = home {
                turn.choice = Some((Objective::Retreat, home));
            }
        }
    }

    /// Lot C7a: give up a fortress that holds out far beyond patience.
    fn give_up_hopeless(&mut self, turn: &mut ArmyTurn) {
        let ctx = self.ctx;
        let (state, data) = (ctx.state, ctx.data);
        turn.hopeless = turn.besieging
            && state.fortification_level(data, &turn.anchor) >= self.rules.fortress_level
            && state
                .settlements
                .get(&turn.anchor)
                .and_then(|s| s.siege.as_ref())
                .is_some_and(|s| {
                    s.turns_elapsed >= self.rules.siege_patience_turns && s.turns_left > 2
                })
            && state
                .assault_odds(data, turn.id)
                .is_none_or(|(odds, _)| odds < self.rules.assault_odds);
        if turn.hopeless {
            self.targeted.insert(turn.anchor.clone());
        }
    }

    /// CV3-6: an army in ambush waits while its prey still comes and it is
    /// not discovered; it does not attack by itself.
    fn leave_ambush(&mut self, turn: &ArmyTurn) -> ControlFlow<()> {
        if turn.army.stance != Stance::Ambush {
            return ControlFlow::Continue(());
        }
        let ctx = self.ctx;
        if crate::stances::keep_ambush(ctx.state, ctx.data, ctx.faction, turn.id) {
            return ControlFlow::Break(());
        }
        self.push_stance(turn.id, Stance::Normal);
        ControlFlow::Continue(())
    }

    /// NT6c: a weakened army on friendly ground with no enemy near rests to
    /// be replenished; it leaves the rest camp once rebuilt.
    fn rest_or_leave_camp(&mut self, turn: &ArmyTurn) -> ControlFlow<()> {
        if turn.choice.is_some() || turn.broken || turn.besieging {
            return ControlFlow::Continue(());
        }
        let ctx = self.ctx;
        match crate::stances::rest_plan(ctx.state, ctx.data, ctx.faction, turn.id) {
            crate::stances::RestPlan::Rest { entrench } => {
                if entrench {
                    self.ensure_stance(turn, Stance::Entrenched);
                }
                return ControlFlow::Break(());
            }
            crate::stances::RestPlan::Leave => {
                let threat = self.threat_here(turn);
                if !crate::stances::should_entrench(
                    ctx.state,
                    ctx.data,
                    ctx.faction,
                    turn.id,
                    threat,
                ) {
                    self.push_stance(turn.id, Stance::Normal);
                }
            }
            crate::stances::RestPlan::None => {}
        }
        ControlFlow::Continue(())
    }

    /// Lot M3: engage an enemy army within the bubble when the odds are
    /// good. The strategic orders below still follow: they fail harmlessly
    /// once the battle has spent the army's movement, and apply when the
    /// attack was refused (target out of reach).
    fn attack_in_bubble(&mut self, turn: &mut ArmyTurn) {
        if turn.broken || turn.besieging {
            return;
        }
        if let Some(order) = self
            .ctx
            .grid
            .attack_order_sparing(turn.id, &mut self.engaged)
        {
            self.orders.push(order);
            turn.attacked = true;
        }
    }

    /// CV3-6: lie in wait for a stronger enemy marching on our lands.
    fn lie_in_wait(&mut self, turn: &ArmyTurn) -> ControlFlow<()> {
        let ctx = self.ctx;
        if turn.choice.is_some() || turn.broken || turn.besieging || turn.attacked || !ctx.at_war()
        {
            return ControlFlow::Continue(());
        }
        match crate::stances::ambush_orders(
            ctx.state,
            ctx.data,
            ctx.faction,
            turn.id,
            ctx.aggression,
        ) {
            Some(ambush) => {
                self.orders.extend(ambush);
                ControlFlow::Break(())
            }
            None => ControlFlow::Continue(()),
        }
    }

    /// Keep a siege that is going our way: storm the walls when the odds
    /// are good (M8) and, behind standing walls, an engine is ready (NT5,
    /// N7: built meanwhile).
    fn keep_siege(&mut self, turn: &ArmyTurn) -> ControlFlow<()> {
        let ctx = self.ctx;
        let holds = turn.choice.is_none()
            && turn.besieging
            && !turn.hopeless
            && ctx.threat_at(&turn.anchor) < turn.power * self.rules.siege_hold_threat_factor;
        if !holds {
            return ControlFlow::Continue(());
        }
        self.targeted.insert(turn.anchor.clone());
        let (state, data) = (ctx.state, ctx.data);
        if state
            .assault_odds(data, turn.id)
            .is_some_and(|(odds, _)| odds >= self.rules.assault_odds)
            && state.assault_blocker(data, turn.id).is_none()
        {
            self.orders.push(Order::Assault {
                army: turn.id.clone(),
            });
            return ControlFlow::Break(());
        }
        self.ensure_stance(turn, Stance::Siege);
        ControlFlow::Break(())
    }

    /// Defence of a threatened friendly settlement.
    fn pick_defence(&self, turn: &mut ArmyTurn) {
        if turn.choice.is_some() {
            return;
        }
        let ctx = self.ctx;
        let (state, data, rules) = (ctx.state, ctx.data, self.rules);
        turn.choice = turn
            .table
            .iter()
            .filter(|(id, _)| {
                state.is_friendly_settlement(ctx.faction, id) && !self.defended.contains(*id)
            })
            .filter_map(|(id, reach)| {
                let threat = ctx.threat_at(id);
                let local = state.settlement_defensive_power(data, id);
                (threat > local * rules.defence_local_factor
                    && turn.power >= rules.defence_ratio * threat)
                    .then(|| {
                        let weight = if Some(id) == self.my_capital.as_ref() {
                            rules.defend_weight_capital
                        } else if ctx.is_city(id) {
                            rules.defend_weight_city
                        } else {
                            rules.defend_weight_other
                        };
                        let value = weight * (ctx.settlement_income(id) + threat)
                            / (1.0 + self.steps(reach.cost));
                        (value, id.clone())
                    })
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
            .map(|(_, id)| (Objective::Defend, id));
    }

    /// Siege: the most valuable weak enemy settlement, cities first (they
    /// hand over the province), then the others by weight and
    /// fortification.
    fn pick_siege(&self, turn: &mut ArmyTurn) {
        if turn.choice.is_some() {
            return;
        }
        let ctx = self.ctx;
        let (state, data, rules) = (ctx.state, ctx.data, self.rules);
        let table = &turn.table;
        turn.choice = table
            .iter()
            .filter(|(id, _)| {
                state.is_hostile_settlement(ctx.faction, id) && !self.targeted.contains(*id)
            })
            .filter(|(id, _)| {
                state.settlement_defensive_power(data, id) * rules.siege_superiority < turn.power
            })
            .filter_map(|(id, reach)| {
                state
                    .settlement_province(id)
                    .map(|p| (id, reach, p.clone()))
            })
            // Landings only for claimed provinces (England in France, not
            // the reverse).
            .filter(|(id, _, p)| {
                self.claims.contains(p) || ctx.owned_de_jure(id) || !crosses_sea(data, table, id)
            })
            // F4: the last strongholds of a realm we hold no claim on are
            // left to the peace table (Scotland survives Edward III).
            .filter(|(id, _, p)| {
                self.claims.contains(p) || ctx.owned_de_jure(id) || !self.last_bastions.contains(p)
            })
            .map(|(id, reach, p)| {
                let mut value = ctx.settlement_income(id) / 100.0 + 10.0;
                if ctx.is_city(id) {
                    value += rules.city_target_bonus;
                }
                if ctx.owned_de_jure(id) {
                    value += rules.reclaim_target_bonus;
                }
                value -= rules.fortification_target_penalty
                    * f64::from(state.fortification_level(data, id));
                if self.enemy_capitals.contains(id) {
                    value += rules.enemy_capital_target_bonus;
                }
                if self.claims.contains(&p) {
                    value += rules.throne_claim_target_bonus;
                }
                if crosses_sea(data, table, id) {
                    value *= rules.sea_invasion_factor;
                }
                (
                    value.max(1.0) / (1.0 + self.steps(reach.cost) / 2.0),
                    id.clone(),
                )
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
            .map(|(_, id)| (Objective::Siege, id));
    }

    /// Chevauchée when too weak to besiege (aggressive factions).
    fn pick_raid(&self, turn: &mut ArmyTurn) {
        let ctx = self.ctx;
        if turn.choice.is_some() || ctx.aggression < self.rules.raid_min_aggression || !ctx.at_war()
        {
            return;
        }
        let (state, data, rules) = (ctx.state, ctx.data, self.rules);
        let table = &turn.table;
        turn.choice = table
            .iter()
            .filter_map(|(id, reach)| {
                state
                    .settlement_province(id)
                    .map(|p| (id, reach, p.clone()))
            })
            .filter(|(_, _, p)| state.is_hostile_territory(ctx.faction, p))
            .filter(|(id, _, p)| {
                state.provinces[p].devastation < rules.raid_max_devastation
                    && (self.claims.contains(p) || !self.last_bastions.contains(p))
                    && state.defensive_power(data, p) < turn.power * rules.raid_defence_factor
                    && ctx.threat(p) < turn.power
                    && !crosses_sea(data, table, id)
            })
            .map(|(id, reach, _)| (id, reach))
            .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
            .map(|(id, _)| (Objective::Raid, id.clone()));
    }

    /// EQ5: with nothing to fight for here, an army trespassing on the
    /// lands of a realm at peace goes home: a place of its own outside
    /// closed lands first (within the planning range, then farther across
    /// those lands), else any place of its own, and only a place the grid
    /// can actually reach (a road over water is no way home).
    fn go_home_from_trespass(&mut self, turn: &ArmyTurn) -> ControlFlow<()> {
        if turn.choice.is_some() || turn.besieging || !turn.trespassing {
            return ControlFlow::Continue(());
        }
        let ctx = self.ctx;
        let (state, data) = (ctx.state, ctx.data);
        let far = ctx.grid.homeward_table(
            &turn.anchor,
            self.range * self.rules.homeward_range_factor,
            turn.cap,
            turn.power,
        );
        let homes = |table: &Table, abroad_ok: bool| -> Vec<SettlementId> {
            let mut homes: Vec<(u32, SettlementId)> = table
                .iter()
                .filter(|(id, _)| {
                    ctx.owns_settlement(id)
                        && state.hostile_armies_at(ctx.faction, id).is_empty()
                        && (abroad_ok
                            || ctx.province_of(id).is_none_or(|p| {
                                passage::trespassed_owner(state, ctx.faction, p).is_none()
                            }))
                })
                .map(|(id, reach)| (reach.cost, id.clone()))
                .collect();
            homes.sort();
            homes.into_iter().map(|(_, id)| id).collect()
        };
        // Same land mass on the grid (a cheap check: no path search).
        let grid = data.navgrid();
        let cell = state.army_cell(data, turn.army);
        let land = grid.component(i64::from(cell.x), i64::from(cell.y));
        let reachable = |target: &SettlementId| {
            data.settlement_point(target).is_some_and(|p| {
                let cell = sim_campaign::Cell::of_point(grid, p);
                let (x, y) = (i64::from(cell.x), i64::from(cell.y));
                grid.passable(x, y) && grid.component(x, y) == land
            })
        };
        let near: &Table = &turn.table;
        let found = [(near, false), (&far, false), (near, true), (&far, true)]
            .into_iter()
            .find_map(|(route, abroad_ok)| {
                homes(route, abroad_ok)
                    .into_iter()
                    .take(self.rules.homeward_candidates)
                    .find(|id| reachable(id))
                    .map(|id| (route, id))
            });
        let Some((route, target)) = found else {
            return ControlFlow::Continue(());
        };
        self.ensure_stance(turn, Stance::Normal);
        let march = ctx
            .grid
            .march_orders(turn.id, turn.army, &turn.anchor, &target, route);
        self.orders.extend(march);
        ControlFlow::Break(())
    }

    /// Regroup with the main army when much weaker.
    fn pick_regroup(&self, turn: &mut ArmyTurn) {
        if turn.choice.is_some() {
            return;
        }
        let Some((main, main_power)) = &self.largest else {
            return;
        };
        if main != turn.id && turn.power < self.rules.regroup_power_share * main_power {
            if let Some(location) = self.ctx.anchors.get(main) {
                if turn.table.contains_key(location) {
                    turn.choice = Some((Objective::Regroup, location.clone()));
                }
            }
        }
    }

    /// Lot C7a: an idle army outside friendly places (after a peace, a lost
    /// siege...) goes back to the nearest place of its own.
    fn pick_return_home(&self, turn: &mut ArmyTurn) {
        let ctx = self.ctx;
        if turn.choice.is_some()
            || turn.besieging
            || !turn
                .army
                .settlement()
                .is_none_or(|s| !ctx.state.is_friendly_settlement(ctx.faction, s))
        {
            return;
        }
        turn.choice = turn
            .table
            .iter()
            .filter(|(id, _)| {
                ctx.owns_settlement(id) && ctx.state.hostile_armies_at(ctx.faction, id).is_empty()
            })
            .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
            .map(|(id, _)| (Objective::Regroup, id.clone()));
    }

    /// CV3-6: outnumbered in the field on a threatened border, with no
    /// shelter within this turn's reach: an entrenched camp.
    fn entrench(&mut self, turn: &ArmyTurn) -> ControlFlow<()> {
        let sheltered = turn.choice.as_ref().is_some_and(|(objective, target)| {
            !matches!(objective, Objective::Regroup | Objective::Retreat)
                || turn.table.get(target).is_some_and(|r| r.cost <= turn.cap)
        });
        if sheltered || turn.besieging || turn.attacked {
            return ControlFlow::Continue(());
        }
        let ctx = self.ctx;
        let threat = self.threat_here(turn);
        if crate::stances::should_entrench(ctx.state, ctx.data, ctx.faction, turn.id, threat) {
            self.ensure_stance(turn, Stance::Entrenched);
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }

    /// CV3-6: with nothing urgent (at peace, or at war when the tuning
    /// allows it with no threat here; no siege of ours or against us), a
    /// short detour to an encounter site.
    fn detour_to_encounter(&mut self, turn: &ArmyTurn) -> ControlFlow<()> {
        let ctx = self.ctx;
        let idle = turn
            .choice
            .as_ref()
            .is_none_or(|(objective, _)| *objective == Objective::Regroup);
        if !idle || turn.besieging || turn.attacked || self.realm_besieged {
            return ControlFlow::Continue(());
        }
        let quiet = !ctx.at_war()
            || (ctx.data.ai_grid.encounters.detour_at_war
                && turn.here.as_ref().is_none_or(|p| ctx.threat(p) <= 0.0));
        if !quiet {
            return ControlFlow::Continue(());
        }
        match crate::stances::encounter_detour(ctx.state, ctx.data, ctx.faction, turn.id) {
            Some(order) => {
                self.ensure_stance(turn, Stance::Normal);
                self.orders.push(order);
                ControlFlow::Break(())
            }
            None => ControlFlow::Continue(()),
        }
    }

    /// The objective chosen, or `Break` after the idle army's stance is
    /// set (normal at home, kept in hostile lands).
    fn settle_idle(&mut self, turn: &mut ArmyTurn) -> ControlFlow<(), (Objective, SettlementId)> {
        let ctx = self.ctx;
        match turn.choice.take() {
            Some(choice) => ControlFlow::Continue(choice),
            None => {
                if !turn
                    .here
                    .as_ref()
                    .is_some_and(|p| ctx.state.is_hostile_territory(ctx.faction, p))
                {
                    self.ensure_stance(turn, Stance::Normal);
                }
                ControlFlow::Break(())
            }
        }
    }

    /// Marks the objective, then marches (a forced march first when the
    /// place is just beyond normal reach) in the stance the objective wants.
    fn march(
        &mut self,
        turn: &ArmyTurn,
        objective: Objective,
        target: SettlementId,
    ) -> ControlFlow<()> {
        let ctx = self.ctx;
        match objective {
            Objective::Defend => {
                self.defended.insert(target.clone());
            }
            Objective::Siege => {
                self.targeted.insert(target.clone());
            }
            _ => {}
        }
        // CV3-6: a forced march to relieve a besieged place or join a siege
        // just beyond normal reach.
        if matches!(objective, Objective::Defend | Objective::Siege) && !turn.attacked {
            if let Some(cost) = turn.table.get(&target).map(|r| r.cost) {
                let forced = crate::stances::forced_march_orders(
                    ctx.state,
                    ctx.data,
                    ctx.faction,
                    turn.id,
                    &target,
                    (cost, turn.cap),
                    |c| {
                        ctx.grid.march_orders_with_cap(
                            turn.id,
                            turn.army,
                            &turn.anchor,
                            &target,
                            &turn.table,
                            c,
                        )
                    },
                );
                if let Some(forced) = forced {
                    self.orders.extend(forced);
                    return ControlFlow::Break(());
                }
            }
        }
        let stance = match objective {
            Objective::Siege => Stance::Siege,
            Objective::Raid => Stance::Raid,
            _ => Stance::Normal,
        };
        self.ensure_stance(turn, stance);
        let march = ctx
            .grid
            .march_orders(turn.id, turn.army, &turn.anchor, &target, &turn.table);
        self.orders.extend(march);
        ControlFlow::Break(())
    }
}

/// Lot C7a: one unit (the cheapest to keep) left as the garrison of an
/// empty, unbesieged place the army holds, when the place was taken from
/// an enemy or lies on a threatened border, and the army can spare it.
pub(super) fn garrison_order(ctx: &Context, army_id: &ArmyId) -> Option<Order> {
    let state = ctx.state;
    let army = state.armies.get(army_id)?;
    if army.units.len() < ctx.rules.military.garrison_min_army_units {
        return None;
    }
    let place = state.settlements.get(army.settlement()?)?;
    if &place.controller != ctx.faction
        || place.siege.is_some()
        || !place.garrison.is_empty()
        || place.kind == SettlementKind::Village
    {
        return None;
    }
    let conquered = &place.owner != ctx.faction;
    let exposed = ctx.is_border(&place.province) && ctx.threat(&place.province) > 0.0;
    if !conquered && !exposed {
        return None;
    }
    let index = army
        .units
        .iter()
        .enumerate()
        .min_by_key(|(i, u)| (sim_campaign::economy::unit_upkeep(ctx.data, u), *i))
        .map(|(i, _)| i)?;
    Some(Order::GarrisonUnits {
        army: army_id.clone(),
        unit_indices: vec![index],
    })
}

/// EQ5: at peace, an army standing in a place of its own inside the lands
/// of another realm (a castle held in a foreign province) joins the
/// garrison when the walls can hold it all, instead of camping there as a
/// field army without right of passage.
pub(super) fn trespasser_garrison_order(ctx: &Context, army_id: &ArmyId) -> Option<Order> {
    let state = ctx.state;
    let army = state.armies.get(army_id)?;
    let place = state.settlements.get(army.settlement()?)?;
    if &place.controller != ctx.faction || place.siege.is_some() || army.units.is_empty() {
        return None;
    }
    let cap = ctx
        .data
        .settlement_rules
        .as_ref()
        .and_then(|r| r.garrison_cap.get(&place.kind))
        .copied()
        .unwrap_or(usize::MAX);
    if place.garrison.len() + army.units.len() > cap {
        return None;
    }
    Some(Order::GarrisonUnits {
        army: army_id.clone(),
        unit_indices: (0..army.units.len()).collect(),
    })
}

/// True when the planned path to `target` includes a sea crossing.
fn crosses_sea(
    data: &data_model::GameData,
    table: &BTreeMap<SettlementId, Reach>,
    target: &SettlementId,
) -> bool {
    let mut current = target.clone();
    while let Some(previous) = table.get(&current).and_then(|r| r.previous.clone()) {
        if is_sea_crossing(data, &previous, &current) {
            return true;
        }
        current = previous;
    }
    false
}
