//! Taxes, upkeep, recruitment delivery and supply/attrition (spec § 1.3 steps 5-7).
//!
//! Province income is `Σ_class count × tax_per_head × wealth/50 × (1 - devastation/100)`
//! scaled by [`TAX_EFFICIENCY`]: the raw formula on the 1337 population gives
//! France ≈ 268 000 livres per season, the target being 20 000-30 000.
//! Unit upkeep in `data/unit_types` is a monthly figure; a season bills
//! [`UPKEEP_MONTHS_PER_SEASON`] months.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, ResourceCategory, ResourceId, SocialClass};
use serde::{Deserialize, Serialize};

use crate::buildings::{effects_of, goods_map, province_building_upkeep};
use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState, ProvinceState, Season, Unit};

/// Tax bracket a faction can pick (spec § 1.4): `×0.7 / ×1.0 / ×1.4` on the
/// tax base, also used as the unrest multiplier of § 1.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxRate {
    Low,
    #[default]
    Normal,
    High,
}

impl TaxRate {
    pub fn multiplier(self) -> f64 {
        match self {
            TaxRate::Low => 0.7,
            TaxRate::Normal => 1.0,
            TaxRate::High => 1.4,
        }
    }
}

/// Faction-level economic snapshot for the bridge (spec § 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionEconomy {
    pub treasury: i64,
    pub income: i64,
    pub projected_income: i64,
    pub army_upkeep: i64,
    pub building_upkeep: i64,
    pub tax_rate: TaxRate,
    pub goods: BTreeMap<ResourceId, u32>,
    pub goods_categories: Vec<ResourceCategory>,
}

/// Livres per head and per season, by social class.
pub fn tax_per_head(class: SocialClass) -> f64 {
    match class {
        SocialClass::Peasants => 0.02,
        SocialClass::Burghers => 0.08,
        SocialClass::Clergy => 0.01,
        SocialClass::Nobility => 0.03,
    }
}

/// Share of the theoretical tax base the crown actually collects.
pub const TAX_EFFICIENCY: f64 = 0.09;
/// Months of upkeep billed per season turn.
pub const UPKEEP_MONTHS_PER_SEASON: i64 = 4;
/// Garrison units are part-time local levies: they cost this share of field upkeep.
pub const GARRISON_UPKEEP_PERCENT: i64 = 50;
/// Morale lost by every unit when the treasury is negative.
pub const BANKRUPTCY_MORALE_PENALTY: u8 = 10;
/// Supply lost per turn outside friendly territory.
pub const ATTRITION_SUPPLY_LOSS: u8 = 20;
/// Supply lost per winter turn outside friendly territory.
pub const ATTRITION_SUPPLY_LOSS_WINTER: u8 = 35;
/// Supply regained per turn in friendly territory.
pub const SUPPLY_RECOVERY: u8 = 40;
/// Strength lost (per cent) each turn an army sits at zero supply.
pub const STARVATION_LOSS_PERCENT: u32 = 10;
/// Devastation healed per turn.
pub const DEVASTATION_DECAY: u8 = 5;
/// Unrest healed per turn.
pub const UNREST_DECAY: u8 = 2;

/// Seasonal tax income of a province (livres).
pub fn province_income(province: &ProvinceState) -> f64 {
    let base: f64 = province
        .population
        .iter()
        .map(|(class, entry)| {
            entry.count as f64 * tax_per_head(class) * f64::from(entry.wealth) / 50.0
        })
        .sum();
    base * (1.0 - f64::from(province.devastation) / 100.0) * TAX_EFFICIENCY
}

/// Seasonal tax income of a province including the tax bracket and building
/// `TaxIncome`/`TradeIncome` effects (spec § 1.4); used to actually collect
/// income (`resolve_economy`), unlike the legacy [`province_income`] which
/// [`CampaignState::faction_income`] keeps using.
pub fn province_income_effective(
    data: &GameData,
    province: &ProvinceState,
    tax_rate: TaxRate,
) -> f64 {
    let effects = effects_of(data, &province.buildings);
    let mut base = 0.0;
    let mut burgher_base = 0.0;
    for (class, entry) in province.population.iter() {
        let share = entry.count as f64 * tax_per_head(class) * f64::from(entry.wealth) / 50.0;
        base += share;
        if class == SocialClass::Burghers {
            burgher_base = share;
        }
    }
    base *= tax_rate.multiplier();
    base *= 1.0 + effects.tax_income.percent / 100.0;
    base += effects.tax_income.flat;
    let trade = burgher_base * (effects.trade_income.percent / 100.0) + effects.trade_income.flat;
    ((base + trade) * (1.0 - f64::from(province.devastation) / 100.0) * TAX_EFFICIENCY).max(0.0)
}

/// Seasonal upkeep of one unit (livres).
pub fn unit_upkeep(data: &GameData, unit: &Unit) -> i64 {
    data.unit_types
        .get(&unit.unit_type)
        .map_or(0, |t| i64::from(t.upkeep))
        * UPKEEP_MONTHS_PER_SEASON
}

/// Seasonal upkeep of a list of field units.
pub fn units_upkeep(data: &GameData, units: &[Unit]) -> i64 {
    units.iter().map(|unit| unit_upkeep(data, unit)).sum()
}

/// Seasonal upkeep of a garrison (see [`GARRISON_UPKEEP_PERCENT`]).
pub fn garrison_upkeep(data: &GameData, units: &[Unit]) -> i64 {
    units_upkeep(data, units) * GARRISON_UPKEEP_PERCENT / 100
}

impl CampaignState {
    /// Income the faction would collect this turn (controlled, unbesieged provinces).
    pub fn faction_income(&self, faction: &data_model::FactionId) -> i64 {
        self.provinces
            .values()
            .filter(|p| &p.controller == faction && p.siege.is_none())
            .map(|p| province_income(p).round() as i64)
            .sum()
    }

    /// Upkeep of every army and garrison of the faction.
    pub fn faction_upkeep(&self, data: &GameData, faction: &data_model::FactionId) -> i64 {
        let armies: i64 = self
            .armies
            .values()
            .filter(|a| &a.faction == faction)
            .map(|a| units_upkeep(data, &a.units))
            .sum();
        let garrisons: i64 = self
            .provinces
            .values()
            .filter(|p| &p.controller == faction)
            .map(|p| garrison_upkeep(data, &p.garrison))
            .sum();
        armies + garrisons
    }

    /// Upkeep of the army (field armies + garrisons), without buildings.
    pub fn faction_army_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        self.faction_upkeep(data, faction)
    }

    /// Upkeep of every completed building of the faction's provinces (spec § 1.2).
    pub fn faction_building_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        self.provinces
            .values()
            .filter(|p| &p.controller == faction)
            .map(|p| province_building_upkeep(data, &p.buildings))
            .sum()
    }

    /// Income the faction would collect this turn under its current tax rate
    /// and buildings (spec § 1.4); unlike [`CampaignState::faction_income`]
    /// this is what `resolve_economy` actually applies to the treasury.
    pub fn faction_income_effective(&self, data: &GameData, faction: &FactionId) -> i64 {
        let tax_rate = self
            .factions
            .get(faction)
            .map(|f| f.tax_rate)
            .unwrap_or_default();
        self.provinces
            .values()
            .filter(|p| &p.controller == faction && p.siege.is_none())
            .map(|p| province_income_effective(data, p, tax_rate).round() as i64)
            .sum()
    }

    /// Full economic snapshot for the bridge (spec § 2).
    pub fn faction_economy(&self, data: &GameData, id: &FactionId) -> Option<FactionEconomy> {
        let faction = self.factions.get(id)?;
        let income = self.faction_income_effective(data, id);
        let army_upkeep = self.faction_army_upkeep(data, id);
        let building_upkeep = self.faction_building_upkeep(data, id);
        let mut goods_categories: Vec<ResourceCategory> = Vec::new();
        for resource_id in faction.goods.keys() {
            if let Some(resource) = data.resources.get(resource_id) {
                if !goods_categories.contains(&resource.category) {
                    goods_categories.push(resource.category);
                }
            }
        }
        Some(FactionEconomy {
            treasury: faction.treasury,
            income: faction.income_last_turn,
            projected_income: income,
            army_upkeep,
            building_upkeep,
            tax_rate: faction.tax_rate,
            goods: faction.goods.clone(),
            goods_categories,
        })
    }
}

/// Phase 5: taxes, upkeep, bankruptcy morale penalty, delivery of recruits.
pub(crate) fn resolve_economy(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let factions: Vec<_> = state.factions.keys().cloned().collect();
    for faction_id in factions {
        if !state.factions[&faction_id].alive {
            continue;
        }
        let income = state.faction_income_effective(data, &faction_id);
        let army_upkeep = state.faction_army_upkeep(data, &faction_id);
        let building_upkeep = state.faction_building_upkeep(data, &faction_id);
        let upkeep = army_upkeep + building_upkeep;
        let faction = state.factions.get_mut(&faction_id).expect("exists");
        faction.treasury += income - upkeep;
        faction.income_last_turn = income;
        faction.upkeep_last_turn = upkeep;
        faction.army_upkeep_last_turn = army_upkeep;
        faction.building_upkeep_last_turn = building_upkeep;
        faction.projected_income = income;
        let treasury = faction.treasury;
        if faction_id == state.player_faction {
            events.push(
                GameEvent::new(
                    EventKind::Income,
                    format!("Revenus : {income} livres, entretien : {upkeep} livres, trésor : {treasury} livres."),
                )
                .faction(&faction_id),
            );
        }
        if treasury < 0 {
            for army in state
                .armies
                .values_mut()
                .filter(|a| a.faction == faction_id)
            {
                for unit in &mut army.units {
                    unit.morale = unit.morale.saturating_sub(BANKRUPTCY_MORALE_PENALTY);
                }
            }
            for province in state
                .provinces
                .values_mut()
                .filter(|p| p.controller == faction_id)
            {
                for unit in &mut province.garrison {
                    unit.morale = unit.morale.saturating_sub(BANKRUPTCY_MORALE_PENALTY);
                }
            }
            events.push(
                GameEvent::new(
                    EventKind::Bankruptcy,
                    "Le trésor est vide : les troupes grondent (moral -10).",
                )
                .faction(&faction_id),
            );
        }
    }

    let player = state.player_faction.clone();
    for (province_id, province) in state.provinces.iter_mut() {
        let queue = std::mem::take(&mut province.recruit_queue);
        for unit_type_id in queue {
            let Some(unit_type) = data.unit_types.get(&unit_type_id) else {
                continue;
            };
            province.garrison.push(Unit::fresh(unit_type));
            if province.controller == player {
                events.push(
                    GameEvent::new(
                        EventKind::Recruited,
                        format!(
                            "{} rejoignent la garnison de {}.",
                            unit_type.name.display,
                            data.provinces.get(province_id).map_or_else(
                                || province_id.to_string(),
                                |p| p.name.display.clone()
                            )
                        ),
                    )
                    .province(province_id)
                    .faction(&player),
                );
            }
        }
    }
}

/// Phase 6: supply and attrition for field armies.
pub(crate) fn resolve_attrition(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ArmyId> = state.armies.keys().cloned().collect();
    let winter = state.season == Season::Winter;
    for army_id in ids {
        let (faction, location) = {
            let army = &state.armies[&army_id];
            (army.faction.clone(), army.location.clone())
        };
        let friendly = state.is_friendly_territory(&faction, &location);
        let army = state.armies.get_mut(&army_id).expect("exists");
        if friendly {
            army.supply = army.supply.saturating_add(SUPPLY_RECOVERY).min(100);
            continue;
        }
        let loss = if winter {
            ATTRITION_SUPPLY_LOSS_WINTER
        } else {
            ATTRITION_SUPPLY_LOSS
        };
        army.supply = army.supply.saturating_sub(loss);
        if army.supply == 0 {
            let mut lost = 0;
            for unit in &mut army.units {
                let casualties = (unit.strength * STARVATION_LOSS_PERCENT).div_ceil(100);
                unit.strength = unit.strength.saturating_sub(casualties);
                lost += casualties;
            }
            army.units.retain(|u| u.strength > 0);
            let destroyed = army.units.is_empty();
            let general = army.general.clone();
            if destroyed {
                if let Some(general) = general {
                    state.detach_general(&general);
                }
                state.armies.remove(&army_id);
            }
            events.push(
                GameEvent::new(
                    if destroyed {
                        EventKind::ArmyDestroyed
                    } else {
                        EventKind::Attrition
                    },
                    format!(
                        "L'armée {army_id} souffre de la disette en {} : {lost} hommes perdus.",
                        data.provinces
                            .get(&location)
                            .map_or_else(|| location.to_string(), |p| p.name.display.clone())
                    ),
                )
                .province(&location)
                .army(&army_id)
                .faction(&faction),
            );
        }
    }
}

/// Recomputes `FactionState::goods` of every faction (spec § 1.3).
pub(crate) fn resolve_goods(state: &mut CampaignState, data: &GameData) {
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for id in ids {
        let goods = goods_map(data, state, &id);
        state.factions.get_mut(&id).expect("exists").goods = goods;
    }
}

/// Phase 7: unrest and devastation slowly recover.
pub(crate) fn resolve_decay(state: &mut CampaignState) {
    for province in state.provinces.values_mut() {
        province.devastation = province.devastation.saturating_sub(DEVASTATION_DECAY);
        province.unrest = province.unrest.saturating_sub(UNREST_DECAY);
    }
}
