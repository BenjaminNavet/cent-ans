//! Buildings: construction, effects and the province "city" view (spec § 1.2).
//!
//! A province holds at most one [`Construction`] at a time. Effects of every
//! completed building are summed by [`province_effects`] into an
//! [`EffectTotals`], which the rest of the crate (taxes, population, sieges)
//! reads instead of touching `Building::effects` directly.

use std::collections::BTreeMap;

use data_model::{BuildingId, EffectKind, EffectMode, GameData, ProvinceId, ResourceId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{CampaignState, Construction, ProvinceState};

/// Population count a province can sustain before health suffers (spec § 1.1).
pub const BASE_CAPACITY: u64 = 40_000;
/// Share of a construction's money cost refunded by `cancel_build`.
pub const CANCEL_REFUND_PERCENT: u32 = 50;

/// One `flat` + `percent` pair of a summed [`EffectKind`].
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EffectValue {
    pub flat: f64,
    pub percent: f64,
}

impl EffectValue {
    fn add(&mut self, effect_mode: EffectMode, value: f64) {
        match effect_mode {
            EffectMode::Add => self.flat += value,
            EffectMode::Percent => self.percent += value,
        }
    }

    /// Applies `flat` then `percent` on top of `base`.
    pub fn apply(&self, base: f64) -> f64 {
        (base + self.flat) * (1.0 + self.percent / 100.0)
    }
}

/// Sum of every building effect of a province, one entry per [`EffectKind`]
/// the simulation reads (spec § 1.2): `TaxIncome`, `TradeIncome`, `Health`,
/// `Unrest`, `Wealth`, `GoodsSatisfaction`, `Growth`, `Garrison`,
/// `FortificationLevel`, `RecruitCost`, `Supply`. Other kinds are ignored.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EffectTotals {
    pub tax_income: EffectValue,
    pub trade_income: EffectValue,
    pub health: EffectValue,
    pub unrest: EffectValue,
    pub wealth: EffectValue,
    pub goods_satisfaction: EffectValue,
    pub growth: EffectValue,
    pub garrison: EffectValue,
    pub fortification_level: EffectValue,
    pub recruit_cost: EffectValue,
    pub supply: EffectValue,
}

impl EffectTotals {
    fn add(&mut self, kind: EffectKind, mode: EffectMode, value: f64) {
        let slot = match kind {
            EffectKind::TaxIncome => &mut self.tax_income,
            EffectKind::TradeIncome => &mut self.trade_income,
            EffectKind::Health => &mut self.health,
            EffectKind::Unrest => &mut self.unrest,
            EffectKind::Wealth => &mut self.wealth,
            EffectKind::GoodsSatisfaction => &mut self.goods_satisfaction,
            EffectKind::Growth => &mut self.growth,
            EffectKind::Garrison => &mut self.garrison,
            EffectKind::FortificationLevel => &mut self.fortification_level,
            EffectKind::RecruitCost => &mut self.recruit_cost,
            EffectKind::Supply => &mut self.supply,
            _ => return,
        };
        slot.add(mode, value);
    }
}

/// Sums the effects of every completed building of `buildings` (data-only:
/// no [`CampaignState`] needed, so `orders.rs` and the population/economy
/// modules can call it on any building list).
pub fn effects_of(data: &GameData, buildings: &[BuildingId]) -> EffectTotals {
    let mut totals = EffectTotals::default();
    for id in buildings {
        let Some(building) = data.buildings.get(id) else {
            continue;
        };
        for effect in &building.effects {
            totals.add(effect.effect, effect.mode, effect.value);
        }
    }
    totals
}

/// Population capacity of a province: [`BASE_CAPACITY`] times one plus the
/// sum of the tiers of its `production`-category buildings (spec § 1.1).
pub fn capacity(data: &GameData, buildings: &[BuildingId]) -> u64 {
    let bonus_tiers: u64 = buildings
        .iter()
        .filter_map(|id| data.buildings.get(id))
        .filter(|b| b.category == data_model::BuildingCategory::Production)
        .map(|b| u64::from(b.tier))
        .sum();
    BASE_CAPACITY * (1 + bonus_tiers)
}

/// Total upkeep of the completed buildings of a province (spec § 1.2).
pub fn province_building_upkeep(data: &GameData, buildings: &[BuildingId]) -> i64 {
    buildings
        .iter()
        .filter_map(|id| data.buildings.get(id))
        .map(|b| i64::from(b.upkeep.unwrap_or(0)))
        .sum()
}

/// One line of the "what can I build here" panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildOption {
    pub building: BuildingId,
    pub name: String,
    pub cost: u32,
    pub turns: u32,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Snapshot of a province's city panel (bridge input, spec § 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvinceCity {
    pub classes: data_model::PopulationClasses,
    pub buildings: Vec<BuildingId>,
    pub construction: Option<Construction>,
    pub fortification_level: u32,
    pub capacity: u64,
    pub buildable: Vec<BuildOption>,
    pub resources: Vec<ResourceId>,
    pub effects: EffectTotals,
}

impl CampaignState {
    /// Sum of the building effects currently active in `province`.
    pub fn province_effects(&self, data: &GameData, province: &ProvinceId) -> EffectTotals {
        self.provinces
            .get(province)
            .map(|p| effects_of(data, &p.buildings))
            .unwrap_or_default()
    }

    /// Fortification level used by sieges: the province's base level (data)
    /// plus building effects (spec § 1.2, replaces the former constant).
    pub fn fortification_level(&self, data: &GameData, province: &ProvinceId) -> u32 {
        let base = data
            .provinces
            .get(province)
            .and_then(|p| p.fortification_level)
            .unwrap_or(0);
        let effects = self.province_effects(data, province);
        effects.fortification_level.apply(f64::from(base)).max(0.0) as u32
    }

    /// Buildable options of `province` for its controller (spec § 1.2).
    pub fn buildable(&self, data: &GameData, province: &ProvinceId) -> Vec<BuildOption> {
        let Some(state) = self.provinces.get(province) else {
            return Vec::new();
        };
        let Some(province_data) = data.provinces.get(province) else {
            return Vec::new();
        };
        let Some(faction) = self.factions.get(&state.controller) else {
            return Vec::new();
        };
        data.buildings
            .values()
            .map(|building| {
                let mut option = BuildOption {
                    building: building.id.clone(),
                    name: building.name.display.clone(),
                    cost: building.cost.money,
                    turns: building.build_time_turns,
                    available: true,
                    reason: None,
                };
                if let Some(reason) =
                    self.build_blocker(data, state, province_data, faction, building)
                {
                    option.available = false;
                    option.reason = Some(reason);
                }
                option
            })
            .collect()
    }

    fn build_blocker(
        &self,
        data: &GameData,
        state: &ProvinceState,
        province_data: &data_model::Province,
        faction: &crate::state::FactionState,
        building: &data_model::Building,
    ) -> Option<String> {
        if state.owner != state.controller {
            return Some("la province doit être possédée et contrôlée".to_owned());
        }
        if state.construction.is_some() {
            return Some("une construction est déjà en cours".to_owned());
        }
        if state.buildings.contains(&building.id) {
            return Some("déjà construit".to_owned());
        }
        if let Some(from) = &building.upgrades_from {
            if !state.buildings.contains(from) {
                let name = data
                    .buildings
                    .get(from)
                    .map_or_else(|| from.to_string(), |b| b.name.display.clone());
                return Some(format!("nécessite {name}"));
            }
        }
        if let Some(required) = &building.required_building {
            if !state.buildings.contains(required) {
                let name = data
                    .buildings
                    .get(required)
                    .map_or_else(|| required.to_string(), |b| b.name.display.clone());
                return Some(format!("bâtiment requis : {name}"));
            }
        }
        if let Some(tech) = &building.required_technology {
            if !faction.technologies.contains(tech) {
                let name = data
                    .technologies
                    .get(tech)
                    .map_or_else(|| tech.to_string(), |t| t.name.display.clone());
                return Some(format!("technologie requise : {name}"));
            }
        }
        if let Some(resource) = &building.required_resource {
            if !province_data.resources.contains(resource) {
                return Some("ressource requise absente".to_owned());
            }
        }
        if building.requires_coastal && !province_data.coastal {
            return Some("nécessite une côte".to_owned());
        }
        if building.requires_river && province_data.rivers.is_empty() {
            return Some("nécessite une rivière".to_owned());
        }
        if building.unique_per_faction
            && self.provinces.values().any(|p| {
                p.controller == state.controller
                    && !std::ptr::eq(p, state)
                    && p.buildings.contains(&building.id)
            })
        {
            return Some("unique pour la faction (déjà construit ailleurs)".to_owned());
        }
        if faction.treasury < i64::from(building.cost.money) {
            return Some(format!(
                "trésor insuffisant ({} livres nécessaires)",
                building.cost.money
            ));
        }
        None
    }

    /// Full city snapshot for the bridge (spec § 2).
    pub fn province_city(&self, data: &GameData, id: &ProvinceId) -> Option<ProvinceCity> {
        let state = self.provinces.get(id)?;
        let province_data = data.provinces.get(id)?;
        Some(ProvinceCity {
            classes: state.population.clone(),
            buildings: state.buildings.clone(),
            construction: state.construction.clone(),
            fortification_level: self.fortification_level(data, id),
            capacity: capacity(data, &state.buildings),
            buildable: self.buildable(data, id),
            resources: province_data.resources.clone(),
            effects: self.province_effects(data, id),
        })
    }
}

/// Phase: progresses (and completes) constructions in every province.
pub(crate) fn resolve_construction(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let player = state.player_faction.clone();
    let ids: Vec<ProvinceId> = state.provinces.keys().cloned().collect();
    for id in ids {
        let province = state.provinces.get_mut(&id).expect("exists");
        let Some(construction) = &mut province.construction else {
            continue;
        };
        construction.turns_left = construction.turns_left.saturating_sub(1);
        if construction.turns_left > 0 {
            continue;
        }
        let Construction { building, .. } = province.construction.take().expect("checked above");
        if let Some(from) = data
            .buildings
            .get(&building)
            .and_then(|b| b.upgrades_from.clone())
        {
            province.buildings.retain(|b| b != &from);
        }
        province.buildings.push(building.clone());
        let controller = province.controller.clone();
        if controller == player {
            let name = data
                .buildings
                .get(&building)
                .map_or_else(|| building.to_string(), |b| b.name.display.clone());
            let province_name = data
                .provinces
                .get(&id)
                .map_or_else(|| id.to_string(), |p| p.name.display.clone());
            events.push(
                GameEvent::new(
                    EventKind::BuildingCompleted,
                    format!("{name} achevé à {province_name}."),
                )
                .province(&id)
                .faction(&controller),
            );
        }
    }
}

/// Resources accessible to `faction`: one point per producing province it
/// controls or is allied with (spec § 1.3).
pub(crate) fn goods_map(
    data: &GameData,
    state: &CampaignState,
    faction: &data_model::FactionId,
) -> BTreeMap<ResourceId, u32> {
    let mut goods: BTreeMap<ResourceId, u32> = BTreeMap::new();
    for (id, province) in &state.provinces {
        if !state.is_allied(faction, &province.controller) {
            continue;
        }
        let Some(province_data) = data.provinces.get(id) else {
            continue;
        };
        for resource in &province_data.resources {
            *goods.entry(resource.clone()).or_insert(0) += 1;
        }
    }
    goods
}
