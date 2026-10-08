//! Taxes, upkeep, recruitment delivery and supply/attrition (spec § 1.3 steps 5-7).
//!
//! Province income is `Σ_class count × tax_per_head × wealth/50 × (1 - devastation/100)`
//! scaled by `tax_efficiency` (`data/rules/economy.json`, RS-B): the raw formula on the 1337 population gives
//! France ≈ 268 000 livres per season, the target being 20 000-30 000.
//! Unit upkeep in `data/unit_types` is a monthly figure; a season bills
//! `upkeep_months_per_season` months (`economy.json`).
//!
//! Lot C4: the tax of a province is shared between the controllers of its
//! settlements in proportion to their normalised `weight` (a besieged
//! settlement yields nothing that turn); holding every settlement of a
//! province adds `full_province_bonus.income_percent` (`rules.json`). The
//! share of a settlement is taxed with the buildings of the city (they serve
//! the whole province) plus its own ([`CampaignState::settlement_tax`]).
//! Garrisons, recruits and building upkeep are counted per settlement.

use std::collections::BTreeMap;

use data_model::{
    BuildingId, EconomyRules, FactionId, GameData, ProvinceId, ResourceCategory, ResourceId,
    SettlementId, SocialClass,
};
use serde::{Deserialize, Serialize};

use crate::buildings::{effects_of, goods_map, province_building_upkeep, EffectTotals};
use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState, ProvinceState, Season, SettlementState, Unit};

/// Tax bracket a faction can pick (spec § 1.4): `×0.7 / ×1.0 / ×1.4` on the
/// tax base, also used as the unrest multiplier of § 1.1 (values in
/// `economy.json` `tax_rates`, RS-B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxRate {
    Low,
    #[default]
    Normal,
    High,
}

impl TaxRate {
    /// The bracket's figures in `rules`.
    pub fn bracket(self, rules: &EconomyRules) -> data_model::TaxBracket {
        match self {
            TaxRate::Low => rules.tax_rates.low,
            TaxRate::Normal => rules.tax_rates.normal,
            TaxRate::High => rules.tax_rates.high,
        }
    }

    /// Multiplier of the tax base (and of unrest).
    pub fn multiplier(self, rules: &EconomyRules) -> f64 {
        self.bracket(rules).multiplier
    }

    /// Share of the population's wealth taken by the crown (0-1): the
    /// "taux d'imposition" of the wealth and unrest formulas (spec § 1.1).
    pub fn burden(self, rules: &EconomyRules) -> f64 {
        self.bracket(rules).burden
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
    /// Court and administration (M10 balance), see
    /// [`data_model::EconomyRules::administration_rate`].
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
    /// C5: trade routes' income this season if resolved now (not yet in
    /// `income`/`projected_income`, which are the tax income alone).
    #[serde(default)]
    pub trade_income: i64,
    /// C5: trade income paid during the last resolved turn.
    #[serde(default)]
    pub trade_income_last_turn: i64,
}

/// Livres per head and per season, by social class (`economy.json` `tax_per_head`).
pub fn tax_per_head(rules: &EconomyRules, class: SocialClass) -> f64 {
    rules.tax_per_head.of(class)
}

// SV4/RS-B: the supply losses, recovery, starvation, devastation decay, tax
// scale, upkeep months and garrison shares are in `data/rules/economy.json`
// (`EconomyRules`).

/// Seasonal tax income of a province (livres).
pub fn province_income(rules: &EconomyRules, province: &ProvinceState) -> f64 {
    let base: f64 = province
        .population
        .iter()
        .map(|(class, entry)| {
            entry.count as f64 * tax_per_head(rules, class) * f64::from(entry.wealth) / 50.0
        })
        .sum();
    base * (1.0 - f64::from(province.devastation) / 100.0) * rules.tax_efficiency
}

/// Seasonal tax income of a province including the tax bracket and building
/// `TaxIncome`/`TradeIncome` effects (spec § 1.4); used to actually collect
/// income (`resolve_economy`), unlike the legacy [`province_income`] which
/// [`CampaignState::faction_income`] keeps using.
pub fn province_income_effective(
    data: &GameData,
    province: &ProvinceState,
    buildings: &[BuildingId],
    tax_rate: TaxRate,
) -> f64 {
    province_income_with(
        data,
        province,
        buildings,
        tax_rate,
        &EffectTotals::default(),
    )
}

/// [`province_income_effective`] with `extra` effects (the controller's
/// technologies, M6) merged on top of the buildings'.
pub fn province_income_with(
    data: &GameData,
    province: &ProvinceState,
    buildings: &[BuildingId],
    tax_rate: TaxRate,
    extra: &EffectTotals,
) -> f64 {
    let rules = &data.economy_rules;
    let mut effects = effects_of(data, buildings);
    effects.merge(extra);
    let mut base = 0.0;
    let mut burgher_base = 0.0;
    // F1 `Production` (mills, forges, workshops; water/wind mill
    // technologies): the produce of peasants and burghers is worth more,
    // half of the gain reaching the crown (`production_tax_share`).
    let production = 1.0 + rules.production_tax_share * effects.production.percent / 100.0;
    for (class, entry) in province.population.iter() {
        let mut share =
            entry.count as f64 * tax_per_head(rules, class) * f64::from(entry.wealth) / 50.0;
        if matches!(class, SocialClass::Peasants | SocialClass::Burghers) {
            share *= production.max(0.0);
        }
        base += share;
        if class == SocialClass::Burghers {
            burgher_base = share;
        }
    }
    base += effects.production.flat;
    base *= tax_rate.multiplier(rules);
    base *= 1.0 + effects.tax_income.percent / 100.0;
    base += effects.tax_income.flat;
    let trade = burgher_base * (effects.trade_income.percent / 100.0) + effects.trade_income.flat;
    ((base + trade) * (1.0 - f64::from(province.devastation) / 100.0) * rules.tax_efficiency)
        .max(0.0)
}

/// Share of a garrison's upkeep paid by its controller, in per cent (lot C4:
/// `garrison_upkeep_percent` of `rules.json` by settlement kind — town
/// militias and castellans were mostly paid locally —, otherwise
/// `economy.json` `garrison_upkeep_percent`).
pub fn garrison_upkeep_percent(data: &GameData, kind: data_model::SettlementKind) -> i64 {
    data.settlement_rules
        .as_ref()
        .and_then(|rules| rules.garrison_upkeep_percent.get(&kind).copied())
        .unwrap_or(data.economy_rules.garrison_upkeep_percent)
}

/// Livres of a garrison's upkeep paid by its controller, before reliefs:
/// each unit pays its settlement kind's share ([`garrison_upkeep_percent`]),
/// except in the capital city (`capital`), where the cheapest
/// `capital_guard.units` form the lord's household guard and pay
/// `capital_guard.upkeep_percent` (lot OMR R3, ADR 0117: a one-province lordship
/// could not pay the one unit the AI never dismisses).
pub fn garrison_share(
    data: &GameData,
    kind: data_model::SettlementKind,
    capital: bool,
    unit_costs: impl IntoIterator<Item = i64>,
) -> i64 {
    let percent = garrison_upkeep_percent(data, kind);
    let guard = data
        .settlement_rules
        .as_ref()
        .and_then(|rules| rules.capital_guard.as_ref())
        .filter(|_| capital);
    let Some(guard) = guard else {
        return unit_costs.into_iter().sum::<i64>() * percent / 100;
    };
    let mut costs: Vec<i64> = unit_costs.into_iter().collect();
    costs.sort_unstable();
    let split = guard.units.min(costs.len());
    let (household, others) = costs.split_at(split);
    household.iter().sum::<i64>() * guard.upkeep_percent / 100
        + others.iter().sum::<i64>() * percent / 100
}

/// Share (per cent) of the upkeep of a settlement's buildings paid by its
/// controller, by settlement kind (lot C7a, `rules.json`
/// `building_upkeep_percent`; 100 when absent).
pub fn building_upkeep_percent(data: &GameData, kind: data_model::SettlementKind) -> i64 {
    data.settlement_rules
        .as_ref()
        .and_then(|rules| rules.building_upkeep_percent.get(&kind).copied())
        .unwrap_or(100)
}

/// Seasonal upkeep of one unit (livres).
pub fn unit_upkeep(data: &GameData, unit: &Unit) -> i64 {
    data.unit_types
        .get(&unit.unit_type)
        .map_or(0, |t| i64::from(t.upkeep))
        * data.economy_rules.upkeep_months_per_season
}

/// Seasonal upkeep of a list of field units.
pub fn units_upkeep(data: &GameData, units: &[Unit]) -> i64 {
    units.iter().map(|unit| unit_upkeep(data, unit)).sum()
}

/// Seasonal upkeep of a garrison (`economy.json` `garrison_upkeep_percent`).
pub fn garrison_upkeep(data: &GameData, units: &[Unit]) -> i64 {
    units_upkeep(data, units) * data.economy_rules.garrison_upkeep_percent / 100
}

/// Upkeep relief (per cent) of a settlement's garrison (F1 `Garrison`): the
/// town itself pays `garrison_relief_percent_per_point` per point of effect
/// (walls and castles house and feed their garrison), up to
/// `garrison_relief_max_percent` (`economy.json`).
pub fn garrison_relief_percent(rules: &EconomyRules, effects: &EffectTotals) -> i64 {
    let points = effects.garrison.apply(0.0).max(0.0).round() as i64;
    (points * rules.garrison_relief_percent_per_point).min(rules.garrison_relief_max_percent)
}

impl CampaignState {
    /// Share (0-1) of the tax of `province` that `faction` collects this
    /// turn: the normalised weights of the settlements it controls that are
    /// not besieged (lot C4). Needs the data for the weights.
    pub fn province_tax_share(
        &self,
        data: &GameData,
        province: &ProvinceId,
        faction: &FactionId,
    ) -> f64 {
        self.settlements_of(province)
            .filter(|(_, s)| &s.controller == faction && s.siege.is_none())
            .map(|(id, _)| crate::settlements::weight_share(data, id))
            .sum()
    }

    /// Seasonal tax a settlement's share of its province yields (lot C4):
    /// the province's tax under the buildings of its city and of the
    /// settlement itself, the governor (M4) and `tech` (M6), times the
    /// settlement's normalised weight.
    pub fn settlement_tax(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        tax_rate: TaxRate,
        tech: &EffectTotals,
    ) -> f64 {
        let Some(state) = self.settlements.get(settlement) else {
            return 0.0;
        };
        let mut extra = self.governor_effects(data, &state.province);
        extra.merge(tech);
        self.settlement_tax_with(data, settlement, tax_rate, &extra)
    }

    /// [`CampaignState::settlement_tax`] with the governor and technology effects of the
    /// settlement's province already merged in `extra` (lot DC3: computed once per province).
    fn settlement_tax_with(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        tax_rate: TaxRate,
        extra: &EffectTotals,
    ) -> f64 {
        let Some(state) = self.settlements.get(settlement) else {
            return 0.0;
        };
        let Some(province) = self.provinces.get(&state.province) else {
            return 0.0;
        };
        let mut buildings: Vec<BuildingId> = self
            .settlements
            .get(&province.city)
            .map(|city| city.buildings.clone())
            .unwrap_or_default();
        if settlement != &province.city {
            buildings.extend(state.buildings.iter().cloned());
        }
        province_income_with(data, province, &buildings, tax_rate, extra)
            * crate::settlements::weight_share(data, settlement)
    }

    /// Income multiplier of the full-province bonus (spec § 4.3).
    pub fn full_province_income_factor(
        &self,
        data: &GameData,
        province: &ProvinceId,
        faction: &FactionId,
    ) -> f64 {
        let percent = data
            .settlement_rules
            .as_ref()
            .map_or(0, |r| r.full_province_bonus.income_percent);
        if percent != 0 && self.holds_whole_province(faction, province) {
            1.0 + f64::from(percent) / 100.0
        } else {
            1.0
        }
    }

    /// Income the faction would collect this turn (legacy formula without
    /// buildings: its share of each province, unbesieged settlements).
    pub fn faction_income(&self, data: &GameData, faction: &FactionId) -> i64 {
        self.provinces
            .iter()
            .map(|(id, p)| {
                let share = self.province_tax_share(data, id, faction);
                if share <= 0.0 {
                    return 0;
                }
                (province_income(&data.economy_rules, p)
                    * share
                    * self.full_province_income_factor(data, id, faction))
                .round() as i64
            })
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
        // C7: a Lombard banker in the general's retinue eases his army's pay.
        let armies: i64 = self
            .armies
            .values()
            .filter(|a| &a.faction == faction)
            .map(|a| {
                let raw: i64 = a.units.iter().map(unit_cost).sum();
                let relief = a.general.as_ref().map_or(0.0, |g| {
                    crate::skills::character_effects(self, data, g)
                        .army_upkeep
                        .percent
                });
                if relief == 0.0 {
                    raw
                } else {
                    (raw as f64 * (1.0 + relief.max(-50.0) / 100.0)).round() as i64
                }
            })
            .sum();
        let capital = self.faction_capital_city(faction);
        let garrisons: i64 = self
            .settlements
            .iter()
            .filter(|(_, s)| &s.controller == faction && !s.garrison.is_empty())
            .map(|(id, s)| {
                let relief = garrison_relief_percent(
                    &data.economy_rules,
                    &self.settlement_effects(data, id),
                );
                let share = garrison_share(
                    data,
                    s.kind,
                    capital == Some(id),
                    s.garrison.iter().map(unit_cost),
                );
                share * (100 - relief) / 100
            })
            .sum();
        // DF1: the AI's armies cost less at higher difficulty.
        let upkeep = crate::difficulty::scale_i64(
            armies + garrisons,
            self.difficulty_upkeep_percent(data, faction),
        );
        // H5: prices follow the coinage.
        crate::coinage::priced(self, faction, upkeep)
    }

    /// The city of the faction's capital province, when the faction holds it
    /// (lot OMR R3: home of the household guard, [`garrison_share`]).
    pub fn faction_capital_city(&self, faction: &FactionId) -> Option<&SettlementId> {
        let capital = &self.factions.get(faction)?.capital;
        self.province_city_id(capital).filter(|city| {
            self.settlements
                .get(*city)
                .is_some_and(|s| &s.controller == faction)
        })
    }

    /// Upkeep of the army (field armies + garrisons), without buildings.
    pub fn faction_army_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        self.faction_upkeep(data, faction)
    }

    /// Upkeep of every completed building of the faction's settlements (spec § 1.2).
    ///
    /// F4: a besieged settlement pays nothing (it pays no taxes either) and a
    /// devastated province pays less (half its devastation, in per cent):
    /// small realms under raids no longer sink into debt for idle buildings.
    pub fn faction_building_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        let raw: i64 = self
            .settlements
            .values()
            .filter(|s| &s.controller == faction && s.siege.is_none())
            .map(|s| {
                let devastation = self
                    .provinces
                    .get(&s.province)
                    .map_or(0, |p| p.devastation.min(100));
                province_building_upkeep(data, &s.buildings) * building_upkeep_percent(data, s.kind)
                    / 100
                    * (100 - i64::from(devastation) / 2)
                    / 100
            })
            .sum();
        crate::coinage::priced(self, faction, raw)
    }

    /// Income the faction would collect this turn under its current tax rate
    /// and buildings (spec § 1.4); unlike [`CampaignState::faction_income`]
    /// this is what `resolve_economy` actually applies to the treasury.
    pub fn faction_income_effective(&self, data: &GameData, faction: &FactionId) -> i64 {
        // OMR R1: asked several times per plan (economy, coinage, agents,
        // subsidies): a memo while a planning scope is open.
        if let Some(derived) = self.derived() {
            return derived.income(data, faction, || {
                self.faction_income_effective_walk(data, faction)
            });
        }
        self.faction_income_effective_walk(data, faction)
    }

    /// [`Self::faction_income_effective`] computed afresh.
    pub fn faction_income_effective_walk(&self, data: &GameData, faction: &FactionId) -> i64 {
        let tax_rate = self
            .factions
            .get(faction)
            .map(|f| f.tax_rate)
            .unwrap_or_default();
        let tech = crate::research::faction_province_tech_effects(self, data, faction);
        let gross: i64 = self
            .provinces
            .keys()
            .map(|id| self.province_gross_income(data, id, faction, tax_rate, &tech))
            .sum();
        // Embargoes (M5) cut trade.
        let net = (gross as f64 * self.embargo_income_factor(faction)).round() as i64;
        // LR-04: the lord's demesne, wherever his seat is held.
        let domain = self.faction_domain_income(data, faction);
        // DF1: difficulty (AI or player income).
        crate::difficulty::scale_i64(
            net + domain,
            self.difficulty_income_percent(data, faction),
        )
            // JR1: the alms of the crusade (0 for every other faction).
            + crate::crusade::alms(self, data, faction)
    }

    /// Seasonal revenue of the lord's own demesne (`economy.json`
    /// `domain_income`, lot LR-04): paid while the faction holds its capital
    /// city, nothing otherwise (a landless or exiled lord has no domain).
    pub fn faction_domain_income(&self, data: &GameData, faction: &FactionId) -> i64 {
        let amount = data.economy_rules.domain_income;
        if amount <= 0 {
            return 0;
        }
        let Some(city) = self.faction_capital_city(faction) else {
            return 0;
        };
        if self
            .settlements
            .get(city)
            .is_some_and(|s| s.siege.is_some())
        {
            return 0;
        }
        amount
    }

    /// Seasonal tax `faction` collects in `province` before embargoes and
    /// difficulty (the per-province part of
    /// [`CampaignState::faction_income_effective`]; `tech` is the faction's
    /// `faction_province_tech_effects`). IB5: read by the tooltip previews.
    pub fn province_gross_income(
        &self,
        data: &GameData,
        province: &ProvinceId,
        faction: &FactionId,
        tax_rate: TaxRate,
        tech: &EffectTotals,
    ) -> i64 {
        // DC3: provinces where the faction collects nothing are skipped, and the
        // governor's effects are merged once per province.
        let mut held = self
            .settlements_of(province)
            .filter(|(_, s)| &s.controller == faction && s.siege.is_none())
            .peekable();
        if held.peek().is_none() {
            return 0;
        }
        let mut extra = self.governor_effects(data, province);
        extra.merge(tech);
        let tax: f64 = held
            .map(|(sid, _)| self.settlement_tax_with(data, sid, tax_rate, &extra))
            .sum();
        (tax * self.full_province_income_factor(data, province, faction)).round() as i64
    }

    /// Court and administration costs of the season: a share of income that
    /// grows with the number of provinces held, plus a share of any treasury
    /// above some seasons of income (M10 balance, F4; B7a:
    /// `data/rules/economy.json`, 20 % above six seasons).
    pub fn faction_administration_upkeep(&self, data: &GameData, faction: &FactionId) -> i64 {
        let income = self.faction_income_effective(data, faction);
        self.administration_upkeep_for(data, faction, income)
    }

    /// [`Self::faction_administration_upkeep`] for an `income` already
    /// computed (`faction_income_effective`, without seigniorage).
    pub fn administration_upkeep_for(
        &self,
        data: &GameData,
        faction: &FactionId,
        income: i64,
    ) -> i64 {
        let rules = &data.economy_rules;
        let provinces = self.controlled_provinces(faction).len();
        let share = (income as f64 * rules.administration_rate(provinces)).round() as i64;
        // An idle hoard feeds court luxury, patronage and embezzlement.
        let treasury = self.factions.get(faction).map_or(0, |f| f.treasury);
        let opulence = (treasury - rules.opulence_seasons * income.max(0)).max(0)
            * rules.opulence_percent
            / 100;
        share + opulence
    }

    /// Full economic snapshot for the bridge (spec § 2).
    pub fn faction_economy(&self, data: &GameData, id: &FactionId) -> Option<FactionEconomy> {
        let faction = self.factions.get(id)?;
        let seigniorage = crate::coinage::seigniorage(self, data, id);
        let recoinage = crate::coinage::recoinage(self, data, id);
        let base_income = self.faction_income_effective(data, id);
        let income = base_income + seigniorage;
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
            administration_upkeep: self.administration_upkeep_for(data, id, base_income)
                + recoinage,
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
            trade_income: crate::trade::faction_trade_income(self, data, id),
            trade_income_last_turn: faction.trade_income_last_turn,
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
        // EQ1: the rebels are not a realm: they levy no taxes, pay no
        // upkeep and cannot go bankrupt (their garrisons live off the land).
        if !state.factions[&faction_id].alive || faction_id.is_rebels() {
            continue;
        }
        // H5: seigniorage is income, the recoinage of strong money upkeep.
        let seigniorage = crate::coinage::seigniorage(state, data, &faction_id);
        let recoinage = crate::coinage::recoinage(state, data, &faction_id);
        let base_income = state.faction_income_effective(data, &faction_id);
        let income = base_income + seigniorage;
        let army_upkeep = state.faction_army_upkeep(data, &faction_id);
        let building_upkeep = state.faction_building_upkeep(data, &faction_id);
        let administration =
            state.administration_upkeep_for(data, &faction_id, base_income) + recoinage;
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
        faction.deficit_seasons = if income < upkeep {
            faction.deficit_seasons.saturating_add(1)
        } else {
            0
        };
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
            // B7a: the troops grumble (morale), they do not desert.
            let penalty = data.economy_rules.bankruptcy_morale_penalty;
            for army in state
                .armies
                .values_mut()
                .filter(|a| a.faction == faction_id)
            {
                for unit in &mut army.units {
                    unit.morale = unit.morale.saturating_sub(penalty);
                }
            }
            for settlement in state
                .settlements
                .values_mut()
                .filter(|s| s.controller == faction_id)
            {
                for unit in &mut settlement.garrison {
                    unit.morale = unit.morale.saturating_sub(penalty);
                }
            }
            events.push(
                GameEvent::new(
                    EventKind::Bankruptcy,
                    format!("Le trésor est vide : les troupes grondent (moral -{penalty})."),
                )
                .faction(&faction_id),
            );
        }
    }

    let player = state.player_faction.clone();
    // F1: effects read by the recruitment delivery (recruits' experience)
    // and by the garrison reinforcement, computed before the mutable pass.
    let local_effects: BTreeMap<SettlementId, EffectTotals> = state
        .settlements
        .iter()
        .filter(|(_, s)| !s.recruit_queue.is_empty() || !s.garrison.is_empty())
        .map(|(id, s)| {
            let mut effects = state.settlement_effects(data, id);
            effects.merge(&crate::research::faction_tech_effects(
                state,
                data,
                &s.controller,
            ));
            (id.clone(), effects)
        })
        .collect();
    // H3: a hearty diet raises the morale of the troops levied there.
    let morale_bonus: BTreeMap<SettlementId, f64> = state
        .settlements
        .iter()
        .filter(|(_, s)| !s.recruit_queue.is_empty())
        .map(|(id, s)| {
            (
                id.clone(),
                crate::table::recruit_morale_bonus(state, data, &s.province),
            )
        })
        .collect();
    for (settlement_id, settlement) in state.settlements.iter_mut() {
        let Some(effects) = local_effects.get(settlement_id) else {
            continue;
        };
        reinforce_garrison(&data.economy_rules, settlement, effects);
        // B7b: every recruit trains one more turn; those done join the
        // garrison, the others stay in the queue.
        let queue = std::mem::take(&mut settlement.recruit_queue);
        let (ready, training): (Vec<_>, Vec<_>) = queue
            .into_iter()
            .map(|mut r| {
                r.turns_left = r.turns_left.saturating_sub(1);
                r
            })
            .partition(|r| r.turns_left == 0);
        settlement.recruit_queue = training;
        let bonus = morale_bonus.get(settlement_id).copied().unwrap_or(0.0);
        for unit_type_id in ready.into_iter().map(|r| r.unit_type) {
            let Some(unit_type) = data.unit_types.get(&unit_type_id) else {
                continue;
            };
            let mut unit = Unit::fresh(unit_type);
            unit.experience = recruit_experience(effects, unit_type.category);
            (unit.levy_armor, unit.levy_ranged) =
                crate::buildings::levy_bonus(data, &settlement.buildings, unit_type.category);
            unit.morale = crate::research::boosted(unit.morale, bonus, 100);
            settlement.garrison.push(unit);
            if settlement.controller == player {
                events.push(
                    GameEvent::new(
                        EventKind::Recruited,
                        format!(
                            "{} rejoignent la garnison de {}.",
                            unit_type.name.display,
                            data.settlement_name(settlement_id)
                        ),
                    )
                    .province(&settlement.province)
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
/// levies that bring its garrison back towards full strength
/// (`garrison_reinforce_*` of `economy.json`, per season and per point).
fn reinforce_garrison(
    rules: &EconomyRules,
    settlement: &mut SettlementState,
    effects: &EffectTotals,
) {
    let points = effects.garrison.apply(0.0).max(0.0).round() as u32;
    if points == 0 || settlement.siege.is_some() || settlement.owner != settlement.controller {
        return;
    }
    let percent = (points * rules.garrison_reinforce_percent_per_point)
        .min(rules.garrison_reinforce_max_percent);
    for unit in &mut settlement.garrison {
        let gain = (unit.max_strength * percent)
            .div_ceil(100)
            .min(unit.max_strength.saturating_sub(unit.strength));
        // TW2-T5: local levies dilute the garrison's experience pro rata.
        crate::traditions::add_recruits(unit, gain, 0);
    }
}

/// F1 `Supply`: `(recovery bonus, loss relief %)` of an army. In friendly
/// territory the province's buildings (ports) and the general's flat
/// `Supply` add to the seasonal recovery; outside it the general's `Supply`
/// and `AttritionResistance` percents shrink the supply lost.
///
/// B7a: the armies live off the land, so a devastated province feeds them
/// badly (design § ravitaillement, « attrition … en territoire ravagé »):
/// in friendly territory it cuts the recovery, outside it worsens the loss,
/// both in proportion to the devastation (`data/rules/economy.json`).
fn supply_modifiers(
    state: &CampaignState,
    data: &GameData,
    location: &ProvinceId,
    friendly: bool,
    general: Option<&data_model::CharacterId>,
) -> (f64, f64) {
    let general_fx = general
        .map(|g| crate::skills::character_effects(state, data, g))
        .unwrap_or_default();
    let rules = &data.economy_rules;
    let devastation = state
        .provinces
        .get(location)
        .map_or(0.0, |p| f64::from(p.devastation.min(100)) / 100.0);
    if friendly {
        let province_fx = state.province_effects(data, location);
        let bonus = province_fx.supply.apply(0.0) + general_fx.supply.flat;
        let cut = (f64::from(rules.supply_recovery) + bonus).max(0.0)
            * devastation
            * rules.supply_devastation_recovery_cut_percent
            / 100.0;
        (bonus - cut, 0.0)
    } else {
        let relief = general_fx.supply.percent
            + general_fx.attrition_resistance.percent
            + general_fx.attrition_resistance.flat;
        // A negative relief is an extra loss (-100: the loss doubles).
        let ravaged = devastation * rules.supply_devastation_loss_percent;
        (0.0, (relief.clamp(-100.0, 90.0) - ravaged).max(-100.0))
    }
}

/// Seasonal supply change (percentage points) of an army standing in
/// `location`: positive recovery in friendly territory, negative loss
/// outside it (heavier in winter). Shared by [`resolve_attrition`] and the
/// supply map filter (`crate::map_lens`), which passes no general.
///
/// OM3 (ADR 0116): the province terrain scales the forage (`terrain_supply`
/// of `economy.json`: thin on the steppe, very thin in the desert) and the
/// desert costs `summer_loss` more every summer, friendly territory included.
pub fn seasonal_supply_change(
    state: &CampaignState,
    data: &GameData,
    location: &ProvinceId,
    friendly: bool,
    general: Option<&data_model::CharacterId>,
    season: Season,
) -> i8 {
    let (recovery_bonus, loss_relief) = supply_modifiers(state, data, location, friendly, general);
    let rules = &data.economy_rules;
    let forage = data
        .provinces
        .get(location)
        .map(|p| rules.terrain_supply(p.terrain))
        .unwrap_or_default();
    let summer = if season == Season::Summer {
        f64::from(forage.summer_loss)
    } else {
        0.0
    };
    if friendly {
        let recovery = (f64::from(rules.supply_recovery) + recovery_bonus).max(0.0)
            * forage.recovery_percent.max(0.0)
            / 100.0;
        return (recovery - summer).round().clamp(-100.0, 100.0) as i8;
    }
    let base_loss = if season == Season::Winter {
        rules.supply_loss_winter
    } else {
        rules.supply_loss
    };
    let loss = f64::from(base_loss) * forage.loss_percent.max(0.0) / 100.0
        * (1.0 - loss_relief / 100.0)
        + summer;
    -(loss.round().clamp(0.0, 100.0) as i8)
}

/// Phase 6: supply and attrition for field armies.
pub(crate) fn resolve_attrition(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ArmyId> = state.armies.keys().cloned().collect();
    let season = state.season;
    for army_id in ids {
        let (faction, settlement, general, province) = {
            let army = &state.armies[&army_id];
            (
                army.faction.clone(),
                army.settlement().cloned(),
                army.general.clone(),
                state.army_province(data, army),
            )
        };
        let Some(location) = province else {
            continue;
        };
        // Lot C4: supplied in friendly territory or on a friendly settlement.
        let friendly = state.is_friendly_territory(&faction, &location)
            || settlement
                .as_ref()
                .is_some_and(|s| state.is_friendly_settlement(&faction, s));
        let change =
            seasonal_supply_change(state, data, &location, friendly, general.as_ref(), season);
        let army_label = crate::events::capitalize(&state.army_name(data, &army_id));
        let army = state.armies.get_mut(&army_id).expect("exists");
        if friendly && change >= 0 {
            army.supply = army.supply.saturating_add(change.unsigned_abs()).min(100);
            continue;
        }
        // CV3: an entrenched camp saves part of the loss.
        let loss = crate::posture::entrenched_loss(data, army, change.unsigned_abs());
        army.supply = army.supply.saturating_sub(loss);
        if army.supply == 0 {
            let mut lost = 0;
            let starvation = data.economy_rules.starvation_loss_percent;
            for unit in &mut army.units {
                let casualties = (unit.strength * starvation).div_ceil(100);
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
                        "{army_label} souffre de la disette en {} : {lost} hommes perdus.",
                        data.province_name(&location)
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

/// Phase 7: unrest and devastation slowly recover; a province whose
/// settlements are all held by one faction calms down by
/// `full_province_bonus.unrest_per_season` (lot C4).
pub(crate) fn resolve_decay(state: &mut CampaignState, data: &GameData) {
    let bonus = data
        .settlement_rules
        .as_ref()
        .map_or(0, |r| r.full_province_bonus.unrest_per_season);
    let whole: Vec<ProvinceId> = state
        .provinces
        .keys()
        .filter(|id| {
            state
                .province_controller(id)
                .is_some_and(|c| state.holds_whole_province(c, id))
        })
        .cloned()
        .collect();
    let rules = &data.population_rules;
    let devastation_decay = data.economy_rules.devastation_decay;
    for (id, province) in state.provinces.iter_mut() {
        province.devastation = province.devastation.saturating_sub(devastation_decay);
        // EQ2: the decay grows with the gauge (it stayed at 100 for years in
        // provinces taken and retaken, at 2 points a season).
        let decay = u32::from(rules.disorder_decay_flat)
            + u32::from(province.unrest) * u32::from(rules.disorder_decay_percent) / 100;
        province.unrest = province
            .unrest
            .saturating_sub(decay.min(u32::from(u8::MAX)) as u8);
        if bonus != 0 && whole.contains(id) {
            province.unrest = (i32::from(province.unrest) + bonus).clamp(0, 100) as u8;
        }
    }
}
