//! Economy tuning (lots EQ1, B7a, SV4, RS-B), mirroring
//! `data/schemas/economy_rules.schema.json` (`data/rules/economy.json`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{SocialClass, Terrain};

/// RS-B: livres per head and per season, by social class (ex-`tax_per_head`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxPerHead {
    pub peasants: f64,
    pub burghers: f64,
    pub clergy: f64,
    pub nobility: f64,
}

impl TaxPerHead {
    /// Livres per head and per season of `class`.
    pub fn of(&self, class: SocialClass) -> f64 {
        match class {
            SocialClass::Peasants => self.peasants,
            SocialClass::Burghers => self.burghers,
            SocialClass::Clergy => self.clergy,
            SocialClass::Nobility => self.nobility,
        }
    }
}

impl Default for TaxPerHead {
    fn default() -> Self {
        TaxPerHead {
            peasants: 0.02,
            burghers: 0.08,
            clergy: 0.01,
            nobility: 0.03,
        }
    }
}

/// RS-B: one tax bracket (spec § 1.4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxBracket {
    /// Multiplier of the tax base, also the unrest multiplier of § 1.1.
    pub multiplier: f64,
    /// Share of the population's wealth taken by the crown (0-1).
    pub burden: f64,
}

/// RS-B: the three tax brackets a faction can pick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxBrackets {
    pub low: TaxBracket,
    pub normal: TaxBracket,
    pub high: TaxBracket,
}

impl Default for TaxBrackets {
    fn default() -> Self {
        TaxBrackets {
            low: TaxBracket {
                multiplier: 0.7,
                burden: 0.2,
            },
            normal: TaxBracket {
                multiplier: 1.0,
                burden: 0.35,
            },
            high: TaxBracket {
                multiplier: 1.4,
                burden: 0.5,
            },
        }
    }
}

/// Contents of `data/rules/economy.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EconomyRules {
    /// Seasonal income (livres) at and above which a chronicle event's
    /// treasury effect applies in full; a poorer faction pays (or gains) in
    /// proportion to its income...
    pub event_treasury_reference_income: i64,
    /// ... but never less than this share of the written amount (0-1).
    pub event_treasury_min_scale: f64,
    /// B7a: base share of income spent on the court and administration (0-1).
    #[serde(default = "default_administration_base")]
    pub administration_base: f64,
    /// B7a: extra share per province controlled (0-1).
    #[serde(default = "default_administration_per_province")]
    pub administration_per_province: f64,
    /// B7a: ceiling of the administration share (0-1).
    #[serde(default = "default_administration_max")]
    pub administration_max: f64,
    /// B7a (M10/F4): a treasury above this many seasons of income feeds an
    /// opulent court...
    #[serde(default = "default_opulence_seasons")]
    pub opulence_seasons: i64,
    /// ... which spends this percentage of the excess every season.
    #[serde(default = "default_opulence_percent")]
    pub opulence_percent: i64,
    /// B7a: morale lost every season by every unit (armies and garrisons)
    /// while the treasury is negative.
    #[serde(default = "default_bankruptcy_morale_penalty")]
    pub bankruptcy_morale_penalty: u8,
    /// B7a: outside friendly territory, extra supply loss (percent of the
    /// seasonal loss) in a province devastated at 100; proportional below.
    #[serde(default = "default_supply_devastation_loss_percent")]
    pub supply_devastation_loss_percent: f64,
    /// B7a: in friendly territory, share of the seasonal supply recovery
    /// (percent) lost in a province devastated at 100; proportional below.
    #[serde(default = "default_supply_devastation_recovery_cut_percent")]
    pub supply_devastation_recovery_cut_percent: f64,
    /// B7c: a construction's `cost.resources` come from the provinces
    /// producing them that the faction can reach (`goods`); each missing
    /// unit is imported at the resource's `base_price` times this.
    #[serde(default = "default_resource_import_multiplier")]
    pub resource_import_multiplier: u32,
    /// SV4: supply lost per season outside friendly territory (points).
    #[serde(default = "default_supply_loss")]
    pub supply_loss: u8,
    /// SV4: supply lost per winter season outside friendly territory.
    #[serde(default = "default_supply_loss_winter")]
    pub supply_loss_winter: u8,
    /// SV4: supply regained per season in friendly territory.
    #[serde(default = "default_supply_recovery")]
    pub supply_recovery: u8,
    /// SV4: strength lost (percent) each season an army sits at zero supply.
    #[serde(default = "default_starvation_loss_percent")]
    pub starvation_loss_percent: u32,
    /// SV4: devastation healed per season in every province.
    #[serde(default = "default_devastation_decay")]
    pub devastation_decay: u8,
    /// RS-B: share of the theoretical tax base the crown actually collects.
    #[serde(default = "default_tax_efficiency")]
    pub tax_efficiency: f64,
    /// RS-B: livres per head and per season, by social class.
    #[serde(default)]
    pub tax_per_head: TaxPerHead,
    /// RS-B: the tax brackets (multiplier and burden).
    #[serde(default)]
    pub tax_rates: TaxBrackets,
    /// RS-B (F1): share of a `Production` bonus that reaches the tax base (0-1).
    #[serde(default = "default_production_tax_share")]
    pub production_tax_share: f64,
    /// RS-B: months of unit upkeep (monthly in `data/unit_types`) billed per season.
    #[serde(default = "default_upkeep_months_per_season")]
    pub upkeep_months_per_season: i64,
    /// RS-B: share (percent) of field upkeep a garrison costs when
    /// `settlements/rules.json` gives none for the settlement kind.
    #[serde(default = "default_garrison_upkeep_percent")]
    pub garrison_upkeep_percent: i64,
    /// RS-B (F1): garrison upkeep relief (percent) per point of `Garrison` effect...
    #[serde(default = "default_garrison_relief_percent_per_point")]
    pub garrison_relief_percent_per_point: i64,
    /// ... up to this ceiling (percent).
    #[serde(default = "default_garrison_relief_max_percent")]
    pub garrison_relief_max_percent: i64,
    /// RS-B (F1): strength (percent of `max_strength`) a garrison regains each
    /// season per point of `Garrison` effect...
    #[serde(default = "default_garrison_reinforce_percent_per_point")]
    pub garrison_reinforce_percent_per_point: u32,
    /// ... up to this ceiling (percent).
    #[serde(default = "default_garrison_reinforce_max_percent")]
    pub garrison_reinforce_max_percent: u32,
    /// RS-C: share (percent) of a building's money cost refunded when it is
    /// demolished (`Order::Demolish`).
    #[serde(default = "default_demolition_refund_percent")]
    pub demolition_refund_percent: u32,
    /// RS-C: when the AI demolishes buildings it can no longer afford.
    #[serde(default)]
    pub ai_demolition: AiDemolition,
    /// OM3 (ADR 0116): forage by province terrain (steppe, desert); a
    /// terrain left out feeds an army normally.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub terrain_supply: BTreeMap<Terrain, TerrainSupply>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OM3: how well a terrain feeds an army.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainSupply {
    /// Share (percent) of the friendly seasonal recovery kept on this terrain.
    #[serde(default = "hundred")]
    pub recovery_percent: f64,
    /// Seasonal loss outside friendly territory (percent of the base loss).
    #[serde(default = "hundred")]
    pub loss_percent: f64,
    /// Extra supply lost every summer, friendly territory included (heat,
    /// dry wells): the desert's summer attrition.
    #[serde(default)]
    pub summer_loss: u8,
}

fn hundred() -> f64 {
    100.0
}

impl Default for TerrainSupply {
    fn default() -> Self {
        TerrainSupply {
            recovery_percent: 100.0,
            loss_percent: 100.0,
            summer_loss: 0,
        }
    }
}

impl EconomyRules {
    /// Forage of `terrain` (neutral when not listed).
    pub fn terrain_supply(&self, terrain: Terrain) -> TerrainSupply {
        self.terrain_supply
            .get(&terrain)
            .copied()
            .unwrap_or_default()
    }
}

/// RS-C: the AI demolishes buildings after `deficit_seasons` seasons in a row
/// in deficit, while their upkeep (those that do not pay for themselves)
/// exceeds `max_upkeep_percent` of its gross income, at most
/// `max_per_turn` a season.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiDemolition {
    pub deficit_seasons: u32,
    pub max_upkeep_percent: i64,
    pub max_per_turn: usize,
}

impl Default for AiDemolition {
    fn default() -> Self {
        AiDemolition {
            deficit_seasons: 4,
            max_upkeep_percent: 40,
            max_per_turn: 1,
        }
    }
}

fn default_administration_base() -> f64 {
    0.08
}
fn default_administration_per_province() -> f64 {
    0.01
}
fn default_administration_max() -> f64 {
    0.35
}
fn default_opulence_seasons() -> i64 {
    6
}
fn default_opulence_percent() -> i64 {
    20
}
fn default_bankruptcy_morale_penalty() -> u8 {
    10
}
fn default_supply_devastation_loss_percent() -> f64 {
    50.0
}
fn default_supply_devastation_recovery_cut_percent() -> f64 {
    50.0
}

impl EconomyRules {
    /// Share of income taken by the court and administration for a realm of
    /// `provinces` provinces.
    pub fn administration_rate(&self, provinces: usize) -> f64 {
        (self.administration_base + self.administration_per_province * provinces as f64)
            .min(self.administration_max)
    }
}

impl Default for EconomyRules {
    /// Fallback when `data/rules/economy.json` is absent; kept equal to that
    /// file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        EconomyRules {
            event_treasury_reference_income: 4000,
            event_treasury_min_scale: 0.25,
            administration_base: default_administration_base(),
            administration_per_province: default_administration_per_province(),
            administration_max: default_administration_max(),
            opulence_seasons: default_opulence_seasons(),
            opulence_percent: default_opulence_percent(),
            bankruptcy_morale_penalty: default_bankruptcy_morale_penalty(),
            supply_devastation_loss_percent: default_supply_devastation_loss_percent(),
            supply_devastation_recovery_cut_percent:
                default_supply_devastation_recovery_cut_percent(),
            resource_import_multiplier: default_resource_import_multiplier(),
            supply_loss: default_supply_loss(),
            supply_loss_winter: default_supply_loss_winter(),
            supply_recovery: default_supply_recovery(),
            starvation_loss_percent: default_starvation_loss_percent(),
            devastation_decay: default_devastation_decay(),
            tax_efficiency: default_tax_efficiency(),
            tax_per_head: TaxPerHead::default(),
            tax_rates: TaxBrackets::default(),
            production_tax_share: default_production_tax_share(),
            upkeep_months_per_season: default_upkeep_months_per_season(),
            garrison_upkeep_percent: default_garrison_upkeep_percent(),
            garrison_relief_percent_per_point: default_garrison_relief_percent_per_point(),
            garrison_relief_max_percent: default_garrison_relief_max_percent(),
            garrison_reinforce_percent_per_point: default_garrison_reinforce_percent_per_point(),
            garrison_reinforce_max_percent: default_garrison_reinforce_max_percent(),
            demolition_refund_percent: default_demolition_refund_percent(),
            ai_demolition: AiDemolition::default(),
            terrain_supply: BTreeMap::new(),
            description: None,
        }
    }
}

fn default_demolition_refund_percent() -> u32 {
    10
}
fn default_resource_import_multiplier() -> u32 {
    100
}
fn default_supply_loss() -> u8 {
    20
}
fn default_supply_loss_winter() -> u8 {
    35
}
fn default_supply_recovery() -> u8 {
    40
}
fn default_starvation_loss_percent() -> u32 {
    10
}
fn default_devastation_decay() -> u8 {
    5
}
/// Lot C4: 0.09 / 1.1 — in 1337 almost every province is held whole, so the
/// full-province bonus (+10 %, `settlements/rules.json`) would otherwise
/// inflate every treasury; the base is lowered to keep the v1 economy.
fn default_tax_efficiency() -> f64 {
    0.082
}
fn default_production_tax_share() -> f64 {
    0.5
}
fn default_upkeep_months_per_season() -> i64 {
    4
}
fn default_garrison_upkeep_percent() -> i64 {
    50
}
fn default_garrison_relief_percent_per_point() -> i64 {
    10
}
fn default_garrison_relief_max_percent() -> i64 {
    50
}
fn default_garrison_reinforce_percent_per_point() -> u32 {
    5
}
fn default_garrison_reinforce_max_percent() -> u32 {
    50
}
