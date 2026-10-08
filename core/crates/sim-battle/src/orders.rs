//! Leader's orders ("ordres du chef", F10b, spec `docs/design/battle-orders.md`).
//!
//! The catalogue comes from `data/battle_orders/` through
//! [`crate::BattleSetup::orders`]; every number (radius, cooldown, duration,
//! effect) is read from it. Each side keeps its own uses and cooldowns.
//!
//! - **war_cry**: morale bonus to the regiments around the general, for a
//!   while (the bonus also raises the recovery ceiling until it expires).
//! - **no_quarter**: morale bonus to the whole army for the battle, once;
//!   the side is flagged `no_quarter` in the [`crate::BattleOutcome`].
//! - **dismount**: heavy horse fight on foot, irreversibly ([`Unit::dismount`]).
//! - **pavise**: crossbowmen stand still behind their pavises and take far
//!   fewer missile casualties, until their next move order.
//! - **rally**: each routing regiment near the general answers the call with
//!   a chance that grows with his command.

use std::collections::BTreeMap;

use data_model::{BattleOrder, BattleOrderKind, BattleOrderScope};
use serde::{Deserialize, Serialize};

use crate::command::CommandError;
use crate::morale::MoraleRules;
use crate::setup::SideId;
use crate::sim::{of_faction, BattleSim};
use crate::unit::{Unit, UnitState};

/// Uses of one order by one side.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct OrderUse {
    pub uses: u32,
    /// Simulated time from which the order can be given again.
    pub ready_at: f64,
}

/// One order as shown in the order bar of `side`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderView {
    pub id: String,
    pub kind: BattleOrderKind,
    pub rank: u8,
    /// Generic name ("Cri de guerre").
    pub name: String,
    /// Wording of the side's faction ("Montjoie ! Saint-Denis !").
    pub label: String,
    pub description: String,
    pub icon: String,
    pub available: bool,
    /// Why the order cannot be given now (French), empty when available.
    pub reason: String,
    pub cooldown: f64,
    pub cooldown_remaining: f64,
    pub uses: u32,
    pub uses_per_battle: Option<u32>,
}

/// Replaces `{key}` placeholders.
fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut text = template.to_owned();
    for (key, value) in values {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text
}

/// Does `unit` match the order's `eligible` filter?
pub fn eligible(order: &BattleOrder, unit: &Unit) -> bool {
    let filter = &order.eligible;
    (filter.categories.is_empty() || filter.categories.contains(&unit.category))
        && filter.mounted.is_none_or(|m| m == unit.mounted)
        && filter.abilities.iter().all(|a| unit.has(*a))
        && !unit.synthetic
}

impl BattleSim {
    /// The battle's order catalogue.
    pub fn order_catalog(&self) -> &[BattleOrder] {
        &self.setup().orders
    }

    pub fn find_order(&self, id: &str) -> Option<&BattleOrder> {
        self.setup().orders.iter().find(|o| o.id == id)
    }

    /// Uses of `order` by `side` so far.
    pub fn order_use(&self, side: SideId, order: &str) -> OrderUse {
        self.order_uses[side.index()]
            .get(order)
            .copied()
            .unwrap_or_default()
    }

    /// Did `side` give the "no quarter" order?
    pub fn no_quarter(&self, side: SideId) -> bool {
        self.no_quarter[side.index()]
    }

    /// Index of the general's regiment while he still commands (alive, on
    /// the field, not routing).
    pub fn commanding_general(&self, side: SideId) -> Option<usize> {
        if !self.general_alive(side) {
            return None;
        }
        self.units()
            .iter()
            .position(|u| u.side == side && u.is_general && u.able())
    }

    /// Regiments of `side` the order reaches now; `units` restricts a
    /// `selected` order (empty: every eligible regiment).
    pub fn order_targets(&self, side: SideId, order: &BattleOrder, units: &[u32]) -> Vec<usize> {
        let rally = order.kind == BattleOrderKind::Rally;
        let wanted = |u: &Unit| {
            u.side == side
                && eligible(order, u)
                && if rally {
                    u.present() && u.state == UnitState::Routing
                } else {
                    u.able()
                }
                && !(order.kind == BattleOrderKind::Pavise && u.pavise.is_some())
        };
        let all = self.units();
        match order.scope {
            BattleOrderScope::Army => (0..all.len()).filter(|&i| wanted(&all[i])).collect(),
            BattleOrderScope::Radius => {
                let Some(g) = self.commanding_general(side) else {
                    return Vec::new();
                };
                let (gx, gz) = (all[g].x, all[g].z);
                let r2 = order.radius.unwrap_or(0.0).powi(2);
                (0..all.len())
                    .filter(|&i| wanted(&all[i]))
                    .filter(|&i| (all[i].x - gx).powi(2) + (all[i].z - gz).powi(2) <= r2)
                    .collect()
            }
            BattleOrderScope::Selected => {
                if units.is_empty() {
                    (0..all.len()).filter(|&i| wanted(&all[i])).collect()
                } else {
                    let mut picked: Vec<usize> = units
                        .iter()
                        .map(|&id| id as usize)
                        .filter(|&i| i < all.len() && wanted(&all[i]))
                        .collect();
                    picked.sort_unstable();
                    picked.dedup();
                    picked
                }
            }
        }
    }

    /// Why `side` cannot give `order` now (`None`: it can).
    pub fn order_unavailable(&self, side: SideId, order: &BattleOrder) -> Option<String> {
        if self.is_finished() {
            return Some("la bataille est terminée".to_owned());
        }
        if order.requires_general && self.commanding_general(side).is_none() {
            let reason = if self.setup().side(side).general.is_none() {
                "aucun chef à la tête de l'armée"
            } else if !self.general_alive(side) {
                "le chef est tombé"
            } else {
                "le chef ne commande plus"
            };
            return Some(reason.to_owned());
        }
        let used = self.order_use(side, &order.id);
        if order.uses_per_battle.is_some_and(|max| used.uses >= max) {
            return Some("déjà donné dans cette bataille".to_owned());
        }
        let remaining = used.ready_at - self.elapsed();
        if remaining > 1e-9 {
            return Some(format!("recharge, encore {} s", remaining.ceil() as i64));
        }
        if self.order_targets(side, order, &[]).is_empty() {
            let reason = match order.kind {
                BattleOrderKind::WarCry => "aucun régiment autour du chef",
                BattleOrderKind::NoQuarter => "plus aucun régiment en état de combattre",
                BattleOrderKind::Dismount => "aucune cavalerie lourde à démonter",
                BattleOrderKind::Pavise => "aucun arbalétrier à pavois disponible",
                BattleOrderKind::Rally => "aucun régiment en déroute près du chef",
            };
            return Some(reason.to_owned());
        }
        None
    }

    /// The order bar of `side`, by rank.
    pub fn leader_orders(&self, side: SideId) -> Vec<OrderView> {
        let faction = &self.setup().side(side).faction;
        let mut views: Vec<OrderView> = self
            .order_catalog()
            .iter()
            .map(|order| {
                let used = self.order_use(side, &order.id);
                let reason = self.order_unavailable(side, order);
                OrderView {
                    id: order.id.clone(),
                    kind: order.kind,
                    rank: order.rank,
                    name: order.name.clone(),
                    label: order.label_for(faction).to_owned(),
                    description: order.description.clone(),
                    icon: order.icon.clone().unwrap_or_default(),
                    available: reason.is_none(),
                    reason: reason.unwrap_or_default(),
                    cooldown: order.cooldown,
                    cooldown_remaining: (used.ready_at - self.elapsed()).max(0.0),
                    uses: used.uses,
                    uses_per_battle: order.uses_per_battle,
                }
            })
            .collect();
        views.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.id.cmp(&b.id)));
        views
    }

    /// Validates and applies a leader's order of `side`.
    pub(crate) fn give_order(
        &mut self,
        side: SideId,
        id: &str,
        units: &[u32],
    ) -> Result<(), CommandError> {
        let order = self
            .find_order(id)
            .cloned()
            .ok_or_else(|| CommandError::UnknownOrder(id.to_owned()))?;
        let faction = self.setup().side(side).faction.clone();
        let label = order.label_for(&faction).to_owned();
        // Named regiments must be ours and able to obey.
        for &uid in units {
            let unit = self
                .units()
                .get(uid as usize)
                .ok_or(CommandError::UnknownUnit(uid))?;
            if unit.side != side {
                return Err(CommandError::NotYours(uid));
            }
            if !unit.present() {
                return Err(CommandError::Unavailable(uid));
            }
        }
        if let Some(reason) = self.order_unavailable(side, &order) {
            return Err(CommandError::OrderUnavailable {
                order: label,
                reason,
            });
        }
        let targets = self.order_targets(side, &order, units);
        if targets.is_empty() {
            return Err(CommandError::OrderUnavailable {
                order: label,
                reason: "aucun des régiments désignés ne peut l'exécuter".to_owned(),
            });
        }
        let faction_name = self.setup().side(side).faction_name.clone();
        let general = self
            .setup()
            .side(side)
            .general
            .as_ref()
            .map_or_else(|| "le chef".to_owned(), |g| g.name.clone());
        let of = of_faction(&faction_name);
        let count = targets.len().to_string();
        let base = [
            ("label", label.as_str()),
            ("faction", faction_name.as_str()),
            ("of_faction", of.as_str()),
            ("general", general.as_str()),
            ("count", count.as_str()),
        ];
        let effects = order.effects.clone();
        match order.kind {
            BattleOrderKind::WarCry => {
                for &i in &targets {
                    let unit = &mut self.units_mut()[i];
                    // A new cry replaces the previous one (no stacking).
                    unit.morale_cap -= unit.order_morale;
                    unit.order_morale = effects.morale;
                    unit.order_morale_timer = order.duration;
                    unit.morale_cap += effects.morale;
                    unit.morale = (unit.morale + effects.morale).clamp(0.0, 100.0);
                }
                self.log(fill(&order.journal, &base), Some(side));
            }
            BattleOrderKind::NoQuarter => {
                for &i in &targets {
                    let unit = &mut self.units_mut()[i];
                    unit.morale_cap = (unit.morale_cap + effects.morale).min(100.0);
                    unit.morale = (unit.morale + effects.morale).clamp(0.0, 100.0);
                }
                self.no_quarter[side.index()] = true;
                self.log(fill(&order.journal, &base), Some(side));
            }
            BattleOrderKind::Dismount => {
                let speed = effects.speed_max.unwrap_or(u8::MAX);
                for &i in &targets {
                    self.units_mut()[i].dismount(speed, effects.armor);
                    let name = self.unit_label(i);
                    let mut values = base.to_vec();
                    values.push(("unit", name.as_str()));
                    self.log(fill(&order.journal, &values), Some(side));
                }
            }
            BattleOrderKind::Pavise => {
                let factor = effects.missile_damage_factor.unwrap_or(1.0);
                for &i in &targets {
                    let unit = &mut self.units_mut()[i];
                    unit.pavise = Some(factor);
                    unit.destination = None;
                    unit.order_queue.clear();
                    unit.destination_facing = None;
                    unit.running = false;
                    if unit.state == UnitState::Marching {
                        unit.state = UnitState::Idle;
                    }
                    let name = self.unit_label(i);
                    let mut values = base.to_vec();
                    values.push(("unit", name.as_str()));
                    self.log(fill(&order.journal, &values), Some(side));
                }
            }
            BattleOrderKind::Rally => {
                let command = self
                    .setup()
                    .side(side)
                    .general
                    .as_ref()
                    .map_or(0.0, |g| f64::from(g.command));
                let chance = (effects.rally_chance + effects.rally_chance_per_command * command)
                    .clamp(0.0, 1.0);
                for &i in &targets {
                    let success = self.rng.unit() < chance;
                    let name = self.unit_label(i);
                    let mut values = base.to_vec();
                    values.push(("unit", name.as_str()));
                    if success {
                        let unit = &mut self.units_mut()[i];
                        unit.state = UnitState::Rallied;
                        unit.rally_timer = MoraleRules::bundled().rally.pause_s;
                        unit.order_queue.clear();
                        unit.morale = unit.morale.max(effects.rally_morale);
                        unit.target = None;
                        unit.destination = None;
                        unit.running = false;
                        self.log(fill(&order.journal, &values), Some(side));
                    } else if let Some(failure) = &order.journal_failure {
                        self.log(fill(failure, &values), Some(side));
                    }
                }
            }
        }
        let elapsed = self.elapsed();
        let used = self.order_uses[side.index()]
            .entry(order.id.clone())
            .or_default();
        used.uses += 1;
        used.ready_at = elapsed + order.cooldown;
        Ok(())
    }

    /// One step of the timed order effects (war cry expiry, missile timer).
    pub(crate) fn tick_orders(&mut self, dt: f64) {
        for unit in self.units_mut() {
            unit.missile_timer += dt;
            if unit.order_morale_timer > 0.0 {
                unit.order_morale_timer -= dt;
                if unit.order_morale_timer <= 0.0 {
                    unit.order_morale_timer = 0.0;
                    unit.morale_cap -= unit.order_morale;
                    unit.order_morale = 0.0;
                }
            }
        }
    }
}

/// Per-side, per-order uses (kept by [`BattleSim`]).
pub(crate) type OrderUses = [BTreeMap<String, OrderUse>; 2];
