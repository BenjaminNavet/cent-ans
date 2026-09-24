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

use crate::buildings::{effects_of, goods_map, province_building_upkeep, EffectTotals};
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

    /// Share of the population's wealth taken by the crown (0-1): the
    /// "taux d'imposition" of the wealth and unrest formulas (spec § 1.1).
    pub fn burden(self) -> f64 {
        match self {
            TaxRate::Low => 0.2,
            TaxRate::Normal => 0.35,
            TaxRate::High => 0.5,
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
    /// Court and administration (M10 balance), see [`administration_rate`].
    #[serde(default)]
    pub administration_upkeep: i64,
    /// H3 « Table »: diets of the controlled provinces this season.
    #[serde(default)]
    pub table_upkeep: i64,
    /// H3 « Table »: diets paid during the last resolved turn.
    #[serde(default)]
    pub table_upkeep_last_turn: i64,
    /// H5: current coinage.
    #[serde(default)]
    pub coinage: crate::coinage::CoinageLevel,
    /// H5: price level (100 = 1337 prices).
    #[serde(default)]
    pub price_level: u32,
    /// H5: seigniorage this season (included in `projected_income`).
    #[serde(default)]
    pub seigniorage: i64,
    /// H5: recoinage this season (included in `administration_upkeep`).
    #[serde(default)]
    pub recoinage: i64,
    #[serde(default)]
    pub seigniorage_last_turn: i64,
    #[serde(default)]
    pub recoinage_last_turn: i64,
    pub tax_rate: TaxRate,
    pub goods: BTreeMap<ResourceId, u32>,
    pub goods_categories: Vec<ResourceCategory>,
}

/// Base share of income spent on the court and administration.
pub const ADMINISTRATION_BASE: f64 = 0.08;
/// Extra share per province controlled.
pub const ADMINISTRATION_PER_PROVINCE: f64 = 0.01;
/// Ceiling of the administration share.
pub const ADMINISTRATION_MAX: f64 = 0.35;

/// A treasury above this many seasons of income feeds an opulent court (F4: 8 → 6).
pub const OPULENCE_SEASONS: i64 = 6;
/// Share of that excess spent by the court every season (percent; F4: 3 → 20).
pub const OPULENCE_PERCENT: i64 = 20;

/// Share of income taken by administration for a realm of `provinces`.
pub fn administration_rate(provinces: usize) -> f64 {
    (ADMINISTRATION_BASE + ADMINISTRATION_PER_PROVINCE * provinces as f64).min(ADMINISTRATION_MAX)
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
    province_income_with(data, province, tax_rate, &EffectTotals::default())
}

/// [`province_income_effective`] with `extra` effects (the controller's
/// technologies, M6) merged on top of the buildings'.
pub fn province_income_with(
    data: &GameData,
    province: &ProvinceState,
    tax_rate: TaxRate,
    extra: &EffectTotals,
) -> f64 {
    let mut effects = effects_of(data, &province.buildings);
    effects.merge(extra);
    let mut base = 0.0;
    let mut burgher_base = 0.0;
    // F1 `Production` (mills, forges, workshops; water/wind mill
    // technologies): the produce of peasants and burghers is worth more,
    // half of the gain reaching the crown ([`PRODUCTION_TAX_SHARE`]).
    let production = 1.0 + PRODUCTION_TAX_SHARE * effects.production.percent / 100.0;
    for (class, entry) in province.population.iter() {
        let mut share = entry.count as f64 * tax_per_head(class) * f64::from(entry.wealth) / 50.0;
        if matches!(class, SocialClass::Peasants | SocialClass::Burghers) {
            share *= production.max(0.0);
        }
        base += share;
        if class == SocialClass::Burghers {
            burgher_base = share;
        }
    }
    base += effects.production.flat;
    base *= tax_rate.multiplier();
    base *= 1.0 + effects.tax_income.percent / 100.0;
    base += effects.tax_income.flat;
    let trade = burgher_base * (effects.trade_income.percent / 100.0) + effects.trade_income.flat;
    ((base + trade) * (1.0 - f64::from(province.devastation) / 100.0) * TAX_EFFICIENCY).max(0.0)
}

/// Share of a `Production` bonus that reaches the tax base (F1).
pub const PRODUCTION_TAX_SHARE: f64 = 0.5;

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

/// Share (per cent) of a garrison's upkeep the town itself pays, per point
/// of `Garrison` effect (F1: walls and castles house and feed their
/// garrison, "unités de garnison gratuites" spread over the whole garrison).
pub const GARRISON_RELIEF_PERCENT_PER_POINT: i64 = 10;
/// Ceiling of that relief.
pub const GARRISON_RELIEF_MAX_PERCENT: i64 = 50;

/// Upkeep relief (per cent) of a province's garrison (F1 `Garrison`).
pub fn garrison_relief_percent(effects: &EffectTotals) -> i64 {
    let points = effects.garrison.apply(0.0).max(0.0).round() as i64;
    (points * GARRISON_RELIEF_PERCENT_PER_POINT).min(GARRISON_RELIEF_MAX_PERCENT)
}

/// Strength (per cent of `max_strength`) a garrison regains each season per
/// point of `Garrison` effect, when the town is neither besieged nor occupied.
pub const GARRISON_REINFORCE_PERCENT_PER_POINT: u32 = 5;

impl CampaignState {
    /// Income the faction would collect this turn (controlled, unbesieged provinces).
    pub fn faction_income(&self, faction: &data_model::FactionId) -> i64 {
        self.provinces
            .values()
            .filter(|p| &p.controller == faction && p.siege.is_none())
            .map(|p| province_income(p).round() as i64)
            .sum()
    }

    /// Upkeep of every army and garrison of the faction: fortified towns pay
    /// part of their garrison's upkeep (`Garrison`, F1), and the faction's `ArmyUpkeep`
    /// technologies (global or per unit family, F1) scale the bill.
    pub fn faction_upkeep(&self, data: &GameData, faction: &data_model::FactionId) -> i64 {
        let tech = crate::research::faction_tech_effects(self, data, faction);
        let unit_cost = |unit: &Unit| -> i64 {
            let mut percent = tech.army_upkeep.percent;
            if let Some(unit_type) = data.unit_types.get(&unit.unit_type) {
                percent += tech
                    .unit_categories
                    .get(unit_type.category)
                    .army_upkeep
                    .percent;
            }
            let raw = unit_upkeep(data, unit) as f64 * (1.0 + percent.max(-90.0) / 100.0);
            raw.round() as i64
        };
        let armies: i64 = self
            .armies
            .values()
            .filter(|a| &a.faction == faction)
            .flat_map(|a| a.units.iter())
            .map(unit_cost)
            .sum();
        let garrisons: i64 = self
            .provinces
            .iter()
            .filter(|(_, p)| &p.controller == faction && !p.garrison.is_empty())
            .map(|(id, p)| {
                let relief = garrison_relief_percent(&self.province_effects(data, id));
                let raw: i64 = p.garrison.iter().map(unit_cost).sum();
                raw * GARRISON_UPKEEP_PERCENT / 100 * (100 - relief) / 100
            })
            .sum();
        // H5: prices follow the coinage.
        crate::coinage::priced(self, faction, armies + garrisons)
    }

    /// Upkeep of the army (field armies + garrisons), without buildings.
    pub fn faction_army_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        self.faction_upkeep(data, faction)
    }

    /// Upkeep of every completed building of the faction's provinces (spec § 1.2).
    ///
    /// F4: a besieged province pays nothing (it pays no taxes either) and a
    /// devastated one pays less (half its devastation, in per cent): small
    /// realms under raids no longer sink into debt for idle buildings.
    pub fn faction_building_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        let raw: i64 = self
            .provinces
            .values()
            .filter(|p| &p.controller == faction && p.siege.is_none())
            .map(|p| {
                province_building_upkeep(data, &p.buildings)
                    * (100 - i64::from(p.devastation.min(100)) / 2)
                    / 100
            })
            .sum();
        crate::coinage::priced(self, faction, raw)
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
        let tech = crate::research::faction_province_tech_effects(self, data, faction);
        let gross: i64 = self
            .provinces
            .iter()
            .filter(|(_, p)| &p.controller == faction && p.siege.is_none())
            .map(|(id, p)| {
                // Buildings + governor (M4) + technologies (M6).
                let mut extra = self.governor_effects(data, id);
                extra.merge(&tech);
                province_income_with(data, p, tax_rate, &extra).round() as i64
            })
            .sum();
        // Embargoes (M5) cut trade.
        (gross as f64 * self.embargo_income_factor(faction)).round() as i64
    }

    /// Court and administration costs of the season: a share of income that
    /// grows with the number of provinces held, plus 20 % of any treasury
    /// above six seasons of income (M10 balance, F4).
    pub fn faction_administration_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        let provinces = self
            .provinces
            .values()
            .filter(|p| &p.controller == faction)
            .count();
        let income = self.faction_income_effective(data, faction);
        let share = (income as f64 * administration_rate(provinces)).round() as i64;
        // An idle hoard feeds court luxury, patronage and embezzlement.
        let treasury = self.factions.get(faction).map_or(0, |f| f.treasury);
        let opulence =
            (treasury - OPULENCE_SEASONS * income.max(0)).max(0) * OPULENCE_PERCENT / 100;
        share + opulence
    }

    /// Full economic snapshot for the bridge (spec § 2).
    pub fn faction_economy(&self, data: &GameData, id: &FactionId) -> Option<FactionEconomy> {
        let faction = self.factions.get(id)?;
        let seigniorage = crate::coinage::seigniorage(self, data, id);
        let recoinage = crate::coinage::recoinage(self, data, id);
        let income = self.faction_income_effective(data, id) + seigniorage;
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
            administration_upkeep: self.faction_administration_upkeep(data, id) + recoinage,
            table_upkeep: self.faction_table_upkeep(data, id),
            table_upkeep_last_turn: faction.table_upkeep_last_turn,
            coinage: faction.coinage,
            price_level: faction.price_level,
            seigniorage,
            recoinage,
            seigniorage_last_turn: faction.seigniorage_last_turn,
            recoinage_last_turn: faction.recoinage_last_turn,
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
        // H5: seigniorage is income, the recoinage of strong money upkeep.
        let seigniorage = crate::coinage::seigniorage(state, data, &faction_id);
        let recoinage = crate::coinage::recoinage(state, data, &faction_id);
        let income = state.faction_income_effective(data, &faction_id) + seigniorage;
        let army_upkeep = state.faction_army_upkeep(data, &faction_id);
        let building_upkeep = state.faction_building_upkeep(data, &faction_id);
        let administration = state.faction_administration_upkeep(data, &faction_id) + recoinage;
        let available = state.factions[&faction_id].treasury + income
            - (army_upkeep + building_upkeep + administration);
        // H3: the diets of the provinces (« Table »), those the purse cannot
        // cover fall back to the default.
        let table = crate::table::pay_table(state, data, &faction_id, available, events);
        let upkeep = army_upkeep + building_upkeep + administration + table;
        let faction = state.factions.get_mut(&faction_id).expect("exists");
        faction.table_upkeep_last_turn = table;
        faction.seigniorage_last_turn = seigniorage;
        faction.recoinage_last_turn = recoinage;
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
    // F1: effects read by the recruitment delivery (recruits' experience)
    // and by the garrison reinforcement, computed before the mutable pass.
    let local_effects: BTreeMap<data_model::ProvinceId, EffectTotals> = state
        .provinces
        .iter()
        .filter(|(_, p)| !p.recruit_queue.is_empty() || !p.garrison.is_empty())
        .map(|(id, p)| {
            let mut effects = state.province_effects(data, id);
            effects.merge(&crate::research::faction_tech_effects(
                state,
                data,
                &p.controller,
            ));
            (id.clone(), effects)
        })
        .collect();
    // H3: a hearty diet raises the morale of the troops levied there.
    let morale_bonus: BTreeMap<data_model::ProvinceId, f64> = state
        .provinces
        .iter()
        .filter(|(_, p)| !p.recruit_queue.is_empty())
        .map(|(id, _)| {
            (
                id.clone(),
                crate::table::recruit_morale_bonus(state, data, id),
            )
        })
        .collect();
    for (province_id, province) in state.provinces.iter_mut() {
        let Some(effects) = local_effects.get(province_id) else {
            continue;
        };
        reinforce_garrison(province, effects);
        let queue = std::mem::take(&mut province.recruit_queue);
        let bonus = morale_bonus.get(province_id).copied().unwrap_or(0.0);
        for unit_type_id in queue {
            let Some(unit_type) = data.unit_types.get(&unit_type_id) else {
                continue;
            };
            let mut unit = Unit::fresh(unit_type);
            unit.experience = recruit_experience(effects, unit_type.category);
            (unit.levy_armor, unit.levy_ranged) =
                crate::buildings::levy_bonus(data, &province.buildings, unit_type.category);
            unit.morale = crate::research::boosted(unit.morale, bonus, 100);
            province.garrison.push(unit);
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

/// Initial experience (0-10) of a recruit of `category` (F1): the flat
/// `ArmyExperience` of the province's buildings (archery butts, armoury), of
/// its governor and of the controller's technologies (standing companies).
pub fn recruit_experience(effects: &EffectTotals, category: data_model::UnitCategory) -> u8 {
    let targeted = effects.unit_categories.get(category).army_experience;
    (effects.army_experience.flat + targeted.flat)
        .round()
        .clamp(0.0, 10.0) as u8
}

/// F1 `Garrison`: a town held by its owner and not besieged musters local
/// levies that bring its garrison back towards full strength.
fn reinforce_garrison(province: &mut ProvinceState, effects: &EffectTotals) {
    let points = effects.garrison.apply(0.0).max(0.0).round() as u32;
    if points == 0 || province.siege.is_some() || province.owner != province.controller {
        return;
    }
    let percent = (points * GARRISON_REINFORCE_PERCENT_PER_POINT).min(50);
    for unit in &mut province.garrison {
        let gain = (unit.max_strength * percent).div_ceil(100);
        unit.strength = (unit.strength + gain).min(unit.max_strength.max(unit.strength));
    }
}

/// F1 `Supply`: `(recovery bonus, loss relief %)` of an army. In friendly
/// territory the province's buildings (ports) and the general's flat
/// `Supply` add to the seasonal recovery; outside it the general's `Supply`
/// and `AttritionResistance` percents shrink the supply lost.
fn supply_modifiers(
    state: &CampaignState,
    data: &GameData,
    location: &data_model::ProvinceId,
    friendly: bool,
    general: Option<&data_model::CharacterId>,
) -> (f64, f64) {
    let general_fx = general
        .map(|g| crate::skills::character_effects(state, data, g))
        .unwrap_or_default();
    if friendly {
        let province_fx = state.province_effects(data, location);
        let bonus = province_fx.supply.apply(0.0) + general_fx.supply.flat;
        (bonus, 0.0)
    } else {
        let relief = general_fx.supply.percent
            + general_fx.attrition_resistance.percent
            + general_fx.attrition_resistance.flat;
        (0.0, relief.clamp(-100.0, 90.0))
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
        let (faction, location, general) = {
            let army = &state.armies[&army_id];
            (
                army.faction.clone(),
                army.location.clone(),
                army.general.clone(),
            )
        };
        let friendly = state.is_friendly_territory(&faction, &location);
        let (recovery_bonus, loss_relief) =
            supply_modifiers(state, data, &location, friendly, general.as_ref());
        let army = state.armies.get_mut(&army_id).expect("exists");
        if friendly {
            let recovery = (f64::from(SUPPLY_RECOVERY) + recovery_bonus)
                .round()
                .clamp(0.0, 100.0) as u8;
            army.supply = army.supply.saturating_add(recovery).min(100);
            continue;
        }
        let base_loss = if winter {
            ATTRITION_SUPPLY_LOSS_WINTER
        } else {
            ATTRITION_SUPPLY_LOSS
        };
        let loss = (f64::from(base_loss) * (1.0 - loss_relief / 100.0))
            .round()
            .clamp(0.0, 100.0) as u8;
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
