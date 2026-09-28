//! Mercenary companies (lot TW2-T3, ADR 0103), mirroring
//! `data/schemas/mercenary_rules.schema.json` (`data/rules/mercenaries.json`).
//! Spec `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T3.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::UnitTypeId;

/// The rules file as bundled at build time: the fallback when
/// `data/rules/mercenaries.json` is absent (test fixtures).
const BUNDLED: &str = include_str!("../../../../../data/rules/mercenaries.json");

/// Contents of `data/rules/mercenaries.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercenaryRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Hiring price, percent of the unit type's `cost.money`.
    pub hire_cost_percent: u32,
    /// Upkeep of a mercenary unit, percent of its type's `upkeep` (the
    /// share above 100 % is billed by `sim_campaign::mercenaries`).
    pub upkeep_percent: u32,
    /// Companies one army may hire in a turn.
    pub hires_per_army_per_turn: u32,
    /// Companies one faction may hire in a turn.
    pub hires_per_faction_per_turn: u32,
    /// `false`: no hiring on lands held by a faction at war with the army's.
    pub hostile_territory_allowed: bool,
    pub arrears: MercenaryArrears,
    pub ai: MercenaryAi,
    /// French name of each province `region`.
    pub region_names: BTreeMap<String, String>,
    pub bands: Vec<MercenaryBand>,
}

impl Default for MercenaryRules {
    fn default() -> Self {
        serde_json::from_str(BUNDLED).expect("bundled data/rules/mercenaries.json is valid")
    }
}

impl MercenaryRules {
    /// The band `id`.
    pub fn band(&self, id: &str) -> Option<&MercenaryBand> {
        self.bands.iter().find(|b| b.id == id)
    }

    /// French name of `region` (the id itself when unnamed).
    pub fn region_name<'a>(&'a self, region: &'a str) -> &'a str {
        self.region_names.get(region).map_or(region, String::as_str)
    }
}

/// Unpaid companies (treasury below zero after the season's upkeep).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercenaryArrears {
    /// Chance (percent) an unpaid company deserts; otherwise it pillages.
    pub desert_percent: u32,
    /// Unrest added to the province a company pillages.
    pub pillage_unrest: u8,
    /// Devastation added to the province a company pillages.
    pub pillage_devastation: u8,
}

/// When the campaign AI hires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercenaryAi {
    /// « Rich »: treasury of at least this percent of a season of gross income...
    pub rich_income_percent: i64,
    /// ... and at least this many livres.
    pub rich_min_treasury: i64,
    /// « Threatened »: hostile power near the army at least this percent of
    /// its own.
    pub threat_ratio_percent: u32,
    /// Hires until the army's power reaches this percent of the threat.
    pub target_ratio_percent: u32,
    /// Treasury kept after hiring, percent of a season of gross income.
    pub keep_income_percent: i64,
}

/// A company offered in some regions over a period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercenaryBand {
    pub id: String,
    /// French name shown in the panel.
    pub name: String,
    /// Unit type hired (marked `mercenary`).
    pub unit: UnitTypeId,
    /// Province `region`s where it can be hired.
    pub regions: Vec<String>,
    /// First and last year (inclusive).
    pub from: i32,
    pub until: i32,
    /// Companies each region holds at most.
    pub cap: u32,
    /// Thousandths of a company each region regains per season.
    pub refill_milli: u32,
    /// Experience of a hired company (0-10).
    #[serde(default)]
    pub experience: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl MercenaryBand {
    /// `true` in `year` (bounds included).
    pub fn active(&self, year: i32) -> bool {
        (self.from..=self.until).contains(&year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_parse() {
        let rules = MercenaryRules::default();
        assert!(rules.upkeep_percent >= 150 && rules.upkeep_percent <= 200);
        assert!(rules.hire_cost_percent > 100);
        let companies = rules.band("grandes_compagnies").expect("routiers");
        assert!(companies.active(1360) && !companies.active(1359));
    }
}
