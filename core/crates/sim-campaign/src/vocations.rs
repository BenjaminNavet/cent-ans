//! Settlement vocations (RG10, ADR 0344).
//!
//! A settlement may specialise (`SettlementState::vocation`, rules in
//! `data/settlements/vocations.json`). The vocation never touches the
//! building list: it adds a *delta* of effects ([`delta_effects`]: the bonus
//! on the concerned buildings' effects plus the cumulative coherence tiers)
//! that [`CampaignState::settlement_effects`], the province effects and the
//! tax computation sum next to the buildings' own effects, and it adjusts the
//! construction cost ([`adjusted_cost`]). A settlement with no vocation has
//! no delta and no cost change.

use data_model::{Building, BuildingId, Effect, GameData, SettlementId, Vocation};
use serde::Serialize;

use crate::buildings::EffectTotals;
use crate::orders::OrderError;
use crate::state::{CampaignState, SettlementState};

/// Vocation of `state`, when set and still defined by the data.
pub fn vocation_of<'a>(data: &'a GameData, state: &SettlementState) -> Option<&'a Vocation> {
    state
        .vocation
        .as_deref()
        .and_then(|id| data.vocation_rules.get(id))
}

/// Completed buildings of `state` that `vocation` concerns.
pub fn concerned_count(data: &GameData, state: &SettlementState, vocation: &Vocation) -> u32 {
    state
        .buildings
        .iter()
        .filter_map(|id| data.buildings.get(id))
        .filter(|b| vocation.concerns(b))
        .count() as u32
}

/// Adds `effect` to `lines`, merging it with a line of the same target and mode.
fn push_merged(lines: &mut Vec<Effect>, effect: Effect) {
    if let Some(line) = lines.iter_mut().find(|l| {
        l.effect == effect.effect
            && l.mode == effect.mode
            && l.class == effect.class
            && l.unit_category == effect.unit_category
    }) {
        line.value += effect.value;
    } else {
        lines.push(effect);
    }
}

/// Effects `vocation` adds to a settlement holding `buildings`: the bonus
/// share of the concerned buildings' effects, then the coherence tiers
/// reached (cumulative). Zero-valued lines are dropped.
pub fn delta_effects(
    data: &GameData,
    buildings: &[BuildingId],
    vocation: &Vocation,
) -> Vec<Effect> {
    let rules = &data.vocation_rules;
    let mut lines: Vec<Effect> = Vec::new();
    let mut count = 0u32;
    for building in buildings.iter().filter_map(|id| data.buildings.get(id)) {
        if !vocation.concerns(building) {
            continue;
        }
        count += 1;
        for effect in &building.effects {
            if rules.bonus_excluded_effects.contains(&effect.effect) {
                continue;
            }
            let mut scaled = effect.clone();
            scaled.value = effect.value * vocation.effect_bonus_percent / 100.0;
            push_merged(&mut lines, scaled);
        }
    }
    for tier in vocation
        .coherence
        .iter()
        .filter(|t| count >= t.min_buildings)
    {
        for effect in &tier.effects {
            push_merged(&mut lines, effect.clone());
        }
    }
    lines.retain(|l| l.value.abs() > f64::EPSILON);
    lines
}

/// The delta of `state`'s own vocation as totals (empty without vocation).
pub fn vocation_delta(data: &GameData, state: &SettlementState) -> EffectTotals {
    let mut totals = EffectTotals::default();
    if let Some(vocation) = vocation_of(data, state) {
        for effect in delta_effects(data, &state.buildings, vocation) {
            totals.add_effect(&effect);
        }
    }
    totals
}

/// Money cost of building `building` in `state` (`base` livres before the
/// vocation): cheaper when the vocation concerns it, dearer when it does not.
pub fn adjusted_cost(
    data: &GameData,
    state: &SettlementState,
    building: &Building,
    base: i64,
) -> i64 {
    let Some(vocation) = vocation_of(data, state) else {
        return base;
    };
    let percent = if vocation.concerns(building) {
        -vocation.cost_reduction_percent
    } else {
        vocation.off_vocation_surcharge_percent
    };
    (base as f64 * (1.0 + percent / 100.0)).round() as i64
}

/// A coherence tier as shown to the UI.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TierView {
    pub min_buildings: u32,
    pub label: String,
    pub reached: bool,
    pub effects: Vec<Effect>,
}

fn tier_views(vocation: &Vocation, count: u32) -> Vec<TierView> {
    vocation
        .coherence
        .iter()
        .map(|t| TierView {
            min_buildings: t.min_buildings,
            label: t.label.clone(),
            reached: count >= t.min_buildings,
            effects: t.effects.clone(),
        })
        .collect()
}

/// `get_settlement_vocation`: a settlement's vocation, coherence and figures.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VocationView {
    /// Vocation id, `None` without vocation.
    pub vocation: Option<String>,
    pub name: String,
    pub icon: String,
    pub description: String,
    /// Concerned buildings standing in the settlement.
    pub count: u32,
    pub reached_tiers: u32,
    pub next_tier: Option<TierView>,
    pub tiers: Vec<TierView>,
    pub effect_bonus_percent: f64,
    pub cost_reduction_percent: f64,
    pub off_vocation_surcharge_percent: f64,
    /// Effects the vocation currently adds (bonus + reached tiers).
    pub active_effects: Vec<Effect>,
    /// Turn from which the vocation may be changed (`None`: now).
    pub locked_until: Option<u32>,
    pub can_change: bool,
    pub change_reason: Option<String>,
}

/// `get_vocation_options`: one row per vocation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VocationOption {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    /// The settlement's current vocation.
    pub current: bool,
    /// Livres the change costs now.
    pub cost: u32,
    pub available: bool,
    pub reason: Option<String>,
    /// Concerned buildings already standing (fit with the present buildings).
    pub fit: u32,
    /// Buildings standing in total.
    pub total_buildings: u32,
    pub effect_bonus_percent: f64,
    pub cost_reduction_percent: f64,
    pub off_vocation_surcharge_percent: f64,
    pub tiers: Vec<TierView>,
    /// What the vocation would add with the present buildings.
    pub preview_effects: Vec<Effect>,
    /// Turns the choice then locks the vocation.
    pub lock_turns: u32,
}

impl CampaignState {
    /// Turn from which the vocation of `settlement` may be changed.
    fn vocation_locked_until(&self, data: &GameData, state: &SettlementState) -> Option<u32> {
        let lock = data.vocation_rules.change_lock_turns;
        state
            .vocation_set_turn
            .map(|set| set + lock)
            .filter(|until| *until > self.turn)
    }

    /// Why `vocation` cannot be adopted now in `settlement`, if so.
    fn vocation_blocker(
        &self,
        data: &GameData,
        state: &SettlementState,
        vocation: &str,
    ) -> Option<String> {
        if data.vocation_rules.get(vocation).is_none() {
            return Some("vocation inconnue".to_owned());
        }
        if state.vocation.as_deref() == Some(vocation) {
            return Some("vocation déjà adoptée".to_owned());
        }
        if let Some(until) = self.vocation_locked_until(data, state) {
            return Some(format!(
                "la vocation ne peut plus changer avant le tour {until} (dans {} tours)",
                until - self.turn
            ));
        }
        None
    }

    fn vocation_cost(&self, data: &GameData, state: &SettlementState) -> u32 {
        data.vocation_rules
            .cost_for(state.kind, state.vocation_set_turn.is_none())
    }

    /// The vocation of `settlement`, its coherence and figures.
    pub fn settlement_vocation(
        &self,
        data: &GameData,
        settlement: &SettlementId,
    ) -> Option<VocationView> {
        let state = self.settlements.get(settlement)?;
        let locked_until = self.vocation_locked_until(data, state);
        let Some(vocation) = vocation_of(data, state) else {
            return Some(VocationView {
                vocation: None,
                name: String::new(),
                icon: String::new(),
                description: String::new(),
                count: 0,
                reached_tiers: 0,
                next_tier: None,
                tiers: Vec::new(),
                effect_bonus_percent: 0.0,
                cost_reduction_percent: 0.0,
                off_vocation_surcharge_percent: 0.0,
                active_effects: Vec::new(),
                locked_until,
                can_change: locked_until.is_none(),
                change_reason: None,
            });
        };
        let count = concerned_count(data, state, vocation);
        let tiers = tier_views(vocation, count);
        Some(VocationView {
            vocation: Some(vocation.id.clone()),
            name: vocation.name.display.clone(),
            icon: vocation.icon.clone().unwrap_or_default(),
            description: vocation.description.clone().unwrap_or_default(),
            count,
            reached_tiers: tiers.iter().filter(|t| t.reached).count() as u32,
            next_tier: tiers.iter().find(|t| !t.reached).cloned(),
            tiers,
            effect_bonus_percent: vocation.effect_bonus_percent,
            cost_reduction_percent: vocation.cost_reduction_percent,
            off_vocation_surcharge_percent: vocation.off_vocation_surcharge_percent,
            active_effects: delta_effects(data, &state.buildings, vocation),
            locked_until,
            can_change: locked_until.is_none(),
            change_reason: locked_until
                .map(|until| format!("changement possible à partir du tour {until}")),
        })
    }

    /// Every vocation as an option for `settlement`: cost, preview, fit.
    pub fn vocation_options(
        &self,
        data: &GameData,
        settlement: &SettlementId,
    ) -> Vec<VocationOption> {
        let Some(state) = self.settlements.get(settlement) else {
            return Vec::new();
        };
        let rules = &data.vocation_rules;
        let cost = self.vocation_cost(data, state);
        let treasury = self
            .factions
            .get(&state.controller)
            .map_or(0, |f| f.treasury);
        rules
            .vocations
            .iter()
            .map(|vocation| {
                let fit = concerned_count(data, state, vocation);
                let mut reason = self.vocation_blocker(data, state, &vocation.id);
                if reason.is_none() && i64::from(cost) > treasury {
                    reason = Some("trésor insuffisant".to_owned());
                }
                VocationOption {
                    id: vocation.id.clone(),
                    name: vocation.name.display.clone(),
                    icon: vocation.icon.clone().unwrap_or_default(),
                    description: vocation.description.clone().unwrap_or_default(),
                    current: state.vocation.as_deref() == Some(vocation.id.as_str()),
                    cost,
                    available: reason.is_none(),
                    reason,
                    fit,
                    total_buildings: state.buildings.len() as u32,
                    effect_bonus_percent: vocation.effect_bonus_percent,
                    cost_reduction_percent: vocation.cost_reduction_percent,
                    off_vocation_surcharge_percent: vocation.off_vocation_surcharge_percent,
                    tiers: tier_views(vocation, fit),
                    preview_effects: delta_effects(data, &state.buildings, vocation),
                    lock_turns: rules.change_lock_turns,
                }
            })
            .collect()
    }

    /// `SetSettlementVocation`: adopts `vocation` in `settlement` for its price.
    pub(crate) fn order_set_vocation(
        &mut self,
        data: &GameData,
        faction: &data_model::FactionId,
        settlement: &SettlementId,
        vocation: &str,
    ) -> Result<(), OrderError> {
        self.own_settlement(faction, settlement)?;
        let state = &self.settlements[settlement];
        if let Some(reason) = self.vocation_blocker(data, state, vocation) {
            return Err(OrderError::VocationRefused(reason));
        }
        let cost = i64::from(self.vocation_cost(data, state));
        let treasury = self.factions.get(faction).map_or(0, |f| f.treasury);
        if cost > treasury {
            return Err(OrderError::InsufficientFunds {
                needed: cost,
                available: treasury,
            });
        }
        if let Some(f) = self.factions.get_mut(faction) {
            f.treasury -= cost;
        }
        let turn = self.turn;
        let state = self.settlements.get_mut(settlement).expect("checked above");
        state.vocation = Some(vocation.to_owned());
        state.vocation_set_turn = Some(turn);
        Ok(())
    }

    /// RG10 AI: the vocation a planner should adopt in `settlement` (best fit
    /// with the standing buildings, the one under construction counting for
    /// one), or `None` to leave it as it is. Deterministic: ties go to the
    /// vocation listed first. A settlement with a vocation changes only for a
    /// vocation fitting `ai_switch_margin` buildings better, once unlocked.
    pub fn suggest_vocation(&self, data: &GameData, settlement: &SettlementId) -> Option<String> {
        let state = self.settlements.get(settlement)?;
        let rules = &data.vocation_rules;
        if self.vocation_locked_until(data, state).is_some() {
            return None;
        }
        let building_under_way = state
            .construction
            .as_ref()
            .and_then(|c| data.buildings.get(&c.building));
        let fit = |vocation: &Vocation| {
            concerned_count(data, state, vocation)
                + u32::from(building_under_way.is_some_and(|b| vocation.concerns(b)))
        };
        let mut best: Option<(&Vocation, u32)> = None;
        for vocation in &rules.vocations {
            let value = fit(vocation);
            if best.is_none_or(|(_, top)| value > top) {
                best = Some((vocation, value));
            }
        }
        let (best, best_fit) = best?;
        if best_fit < rules.ai_min_buildings.max(1) {
            return None;
        }
        match vocation_of(data, state) {
            None => Some(best.id.clone()),
            Some(current) if current.id == best.id => None,
            Some(current) => {
                (best_fit >= fit(current) + rules.ai_switch_margin.max(1)).then(|| best.id.clone())
            }
        }
    }

    /// Livres the AI pays to adopt a vocation in `settlement`.
    pub fn vocation_price(&self, data: &GameData, settlement: &SettlementId) -> u32 {
        self.settlements
            .get(settlement)
            .map_or(0, |s| self.vocation_cost(data, s))
    }
}
