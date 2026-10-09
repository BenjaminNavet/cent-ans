//! WH econ: where a settlement's, a province's and a faction's tax income
//! comes from, source by source (poll tax per class, production, tax bracket,
//! building taxes, building trade, devastation, full-province bonus, then at
//! faction level embargo, demesne, difficulty and crusade alms).
//!
//! The decomposition mirrors the arithmetic of [`crate::economy::province_income`]
//! with float accumulators, then rounds each line and puts the rounding
//! remainder on the largest one, so the integer lines always add up to the
//! figure the rest of the game uses (`province_gross_income`,
//! `faction_income`): the invariant is tested.

use data_model::{
    BuildingId, EffectKind, FactionId, GameData, ProvinceId, SettlementId, SocialClass,
};

use crate::buildings::{effects_of, EffectTotals};
use crate::economy::{tax_per_head, TaxRate};
use crate::state::{CampaignState, ProvinceState};

/// Number of province-level sources.
pub const SOURCE_COUNT: usize = 10;

/// Keys of the sources, in display order (`IncomeBreakdown::lines`).
pub const SOURCE_KEYS: [&str; SOURCE_COUNT] = [
    "poll_peasants",
    "poll_burghers",
    "poll_clergy",
    "poll_nobility",
    "production",
    "tax_rate",
    "building_tax",
    "building_trade",
    "devastation",
    "full_province",
];

type Parts = [f64; SOURCE_COUNT];

/// Signed livres per source for a settlement or a province.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IncomeBreakdown {
    pub lines: [i64; SOURCE_COUNT],
}

impl IncomeBreakdown {
    pub fn total(&self) -> i64 {
        self.lines.iter().sum()
    }

    /// `(key, livres)` for the non-zero lines, in display order.
    pub fn named_lines(&self) -> Vec<(&'static str, i64)> {
        SOURCE_KEYS
            .iter()
            .copied()
            .zip(self.lines)
            .filter(|(_, value)| *value != 0)
            .collect()
    }

    fn add(&mut self, other: &IncomeBreakdown) {
        for (a, b) in self.lines.iter_mut().zip(other.lines) {
            *a += b;
        }
    }

    /// Rounds `parts`; the remainder against `target` goes on the largest line.
    fn from_parts(parts: &Parts, target: i64) -> IncomeBreakdown {
        let mut lines = [0i64; SOURCE_COUNT];
        for (line, part) in lines.iter_mut().zip(parts) {
            *line = part.round() as i64;
        }
        let largest = parts
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .map_or(0, |(index, _)| index);
        lines[largest] += target - lines.iter().sum::<i64>();
        IncomeBreakdown { lines }
    }
}

/// Faction level: the provinces' sources plus the adjustments after them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FactionIncomeBreakdown {
    /// Sum of the provinces' lines.
    pub provinces: IncomeBreakdown,
    /// Embargo cut (≤ 0).
    pub embargo: i64,
    /// The lord's demesne.
    pub domain: i64,
    /// Difficulty scaling of the income.
    pub difficulty: i64,
    /// Crusade alms.
    pub alms: i64,
}

impl FactionIncomeBreakdown {
    /// Equals `CampaignState::faction_income`.
    pub fn total(&self) -> i64 {
        self.provinces.total() + self.embargo + self.domain + self.difficulty + self.alms
    }
}

/// Float decomposition of `province_income` for one settlement's share of
/// the tax (before the settlement's weight).
fn income_parts(
    data: &GameData,
    province: &ProvinceState,
    buildings: &[BuildingId],
    tax_rate: TaxRate,
    extra: &EffectTotals,
) -> Parts {
    let rules = &data.economy_rules;
    let mut effects = effects_of(data, buildings);
    effects.merge(extra);
    let production = (1.0
        + rules.production_tax_share * effects[EffectKind::Production].percent / 100.0)
        .max(0.0);
    let mut parts: Parts = [0.0; SOURCE_COUNT];
    let mut poll = [0.0; 4];
    let mut production_gain = effects[EffectKind::Production].flat;
    let mut burgher_base = 0.0;
    for (class, entry) in province.population.iter() {
        let raw = entry.count as f64 * tax_per_head(rules, class) * f64::from(entry.wealth) / 50.0;
        let mut share = raw;
        if matches!(class, SocialClass::Peasants | SocialClass::Burghers) {
            share *= production;
        }
        production_gain += share - raw;
        if class == SocialClass::Burghers {
            burgher_base = share;
        }
        let slot = match class {
            SocialClass::Peasants => 0,
            SocialClass::Burghers => 1,
            SocialClass::Clergy => 2,
            SocialClass::Nobility => 3,
        };
        poll[slot] += raw;
    }
    let base1: f64 = poll.iter().sum::<f64>() + production_gain;
    let base2 = base1 * tax_rate.multiplier(rules);
    let tax_percent = effects[EffectKind::TaxIncome].percent / 100.0;
    let building_tax = base2 * tax_percent + effects[EffectKind::TaxIncome].flat;
    let trade = burgher_base * (effects[EffectKind::TradeIncome].percent / 100.0)
        + effects[EffectKind::TradeIncome].flat;
    parts[..4].copy_from_slice(&poll);
    parts[4] = production_gain;
    parts[5] = base1 * (tax_rate.multiplier(rules) - 1.0);
    parts[6] = building_tax;
    parts[7] = trade;
    // Collection efficiency scales every source; devastation is the loss.
    let scale = rules.tax_efficiency;
    let devastation = f64::from(province.devastation) / 100.0;
    let gross: f64 = parts.iter().sum::<f64>() * scale;
    for part in &mut parts {
        *part *= scale;
    }
    parts[8] = -gross * devastation;
    for part in &mut parts[..8] {
        *part *= 1.0 - devastation;
    }
    parts[8] = -(gross - parts[..8].iter().sum::<f64>());
    parts
}

impl CampaignState {
    /// Sources of a settlement's tax share this season; sums to
    /// `settlement_tax(..).round()` (0 while besieged).
    pub fn settlement_income_breakdown(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        tax_rate: TaxRate,
        tech: &EffectTotals,
    ) -> IncomeBreakdown {
        let Some(state) = self.settlements.get(settlement) else {
            return IncomeBreakdown::default();
        };
        let target = self
            .settlement_tax(data, settlement, tax_rate, tech)
            .round() as i64;
        if state.siege.is_some() {
            return IncomeBreakdown::default();
        }
        let parts = self.settlement_parts(data, settlement, tax_rate, tech);
        IncomeBreakdown::from_parts(&parts, target)
    }

    fn settlement_parts(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        tax_rate: TaxRate,
        tech: &EffectTotals,
    ) -> Parts {
        let mut parts: Parts = [0.0; SOURCE_COUNT];
        let Some(state) = self.settlements.get(settlement) else {
            return parts;
        };
        let Some(province) = self.provinces.get(&state.province) else {
            return parts;
        };
        let mut extra = self.governor_effects(data, &state.province);
        extra.merge(tech);
        let mut buildings: Vec<BuildingId> = self
            .settlements
            .get(&province.city)
            .map(|city| city.buildings.clone())
            .unwrap_or_default();
        if settlement != &province.city {
            buildings.extend(state.buildings.iter().cloned());
        }
        let rate = self.province_tax_rate(&state.province, tax_rate);
        let weight = crate::settlements::weight_share(data, settlement);
        for (part, value) in parts
            .iter_mut()
            .zip(income_parts(data, province, &buildings, rate, &extra))
        {
            *part = value * weight;
        }
        parts
    }

    /// Sources of `faction`'s tax in `province`; sums to
    /// [`CampaignState::province_gross_income`].
    pub fn province_income_breakdown(
        &self,
        data: &GameData,
        province: &ProvinceId,
        faction: &FactionId,
        tax_rate: TaxRate,
        tech: &EffectTotals,
    ) -> IncomeBreakdown {
        let target = self.province_gross_income(data, province, faction, tax_rate, tech);
        let mut parts: Parts = [0.0; SOURCE_COUNT];
        for (sid, s) in self.settlements_of(province) {
            if &s.controller != faction || s.siege.is_some() {
                continue;
            }
            for (part, value) in parts
                .iter_mut()
                .zip(self.settlement_parts(data, sid, tax_rate, tech))
            {
                *part += value;
            }
        }
        let factor = self.full_province_income_factor(data, province, faction);
        let subtotal: f64 = parts.iter().sum();
        for part in &mut parts[..9] {
            *part *= factor;
        }
        parts[9] = subtotal * (factor - 1.0);
        IncomeBreakdown::from_parts(&parts, target)
    }

    /// Sources of the faction's seasonal tax income; sums to
    /// [`CampaignState::faction_income`] (trade routes and seigniorage are
    /// outside of it).
    pub fn faction_income_breakdown(
        &self,
        data: &GameData,
        faction: &FactionId,
    ) -> FactionIncomeBreakdown {
        let tax_rate = self
            .factions
            .get(faction)
            .map(|f| f.tax_rate)
            .unwrap_or_default();
        let tech = crate::research::faction_province_tech_effects(self, data, faction);
        let mut provinces = IncomeBreakdown::default();
        for id in self.provinces.keys() {
            provinces.add(&self.province_income_breakdown(data, id, faction, tax_rate, &tech));
        }
        let gross = provinces.total();
        let net = (gross as f64 * self.embargo_income_factor(faction)).round() as i64;
        let domain = self.faction_domain_income(data, faction);
        let scaled = crate::difficulty::scale_i64(
            net + domain,
            self.difficulty_income_percent(data, faction),
        );
        FactionIncomeBreakdown {
            provinces,
            embargo: net - gross,
            domain,
            difficulty: scaled - (net + domain),
            alms: crate::crusade::alms(self, data, faction),
        }
    }
}
