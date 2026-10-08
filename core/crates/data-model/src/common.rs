//! Shared value types mirroring `data/schemas/common.schema.json`.

use crate::key_enum;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::ResourceId;

/// French display name plus the period name in the local language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizedName {
    /// Name shown in the UI (French).
    pub display: String,
    /// Period name in the local language (Old French, Middle English, Gascon...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<String>,
    /// Language of `local` (e.g. "ancien français").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_language: Option<String>,
}

/// Partial ISO date (`YYYY`, `YYYY-MM` or `YYYY-MM-DD`, Julian calendar) with an
/// uncertainty flag. `uncertain: true` marks an approximate or disputed date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalDate {
    pub value: String,
    #[serde(default)]
    pub uncertain: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl HistoricalDate {
    /// Year component of the date, if it parses.
    pub fn year(&self) -> Option<i32> {
        self.value.get(..4)?.parse().ok()
    }
}

/// Estimated integer with an uncertainty flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UncertainInteger {
    pub value: u64,
    #[serde(default)]
    pub uncertain: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Integer gauge from 0 to 100.
pub type Percent = u8;

key_enum! {
/// The four social classes of the population model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialClass {
    Peasants => "peasants",
    Burghers => "burghers",
    Clergy => "clergy",
    Nobility => "nobility",
}
}

/// Battle unit family, shared by unit types and effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitCategory {
    Infantry,
    Ranged,
    Cavalry,
    Siege,
}

/// Closed list of modifiers an effect may target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    Wealth,
    Health,
    Unrest,
    GoodsSatisfaction,
    Growth,
    TaxIncome,
    TradeIncome,
    Production,
    ResearchCivil,
    ResearchMilitary,
    RecruitSlots,
    RecruitCost,
    ArmyUpkeep,
    ArmyExperience,
    ArmyMorale,
    ArmyArmor,
    ArmyRanged,
    /// M6: flat melee bonus per `unit_category` (technologies).
    ArmyMelee,
    Garrison,
    FortificationLevel,
    SiegeResistance,
    Piety,
    Prestige,
    Supply,
    Movement,
    AttritionResistance,
    Loyalty,
    SiegeSpeed,
    ConstructionSpeed,
    Diplomacy,
    Intrigue,
    Fertility,
    BattleCharge,
    BattleRanged,
    BattleDefense,
    /// M6: research points per turn (research buildings, technologies).
    ResearchPoints,
    /// H4: resistance to plague and epidemics, in percent points (reduces
    /// both the chance and the severity; capped by the simulation).
    PlagueResistance,
    /// H4: percent of a battle's casualties recovered as wounded by the
    /// surviving units (capped by the simulation).
    WoundRecovery,
    /// H3/H4: percent bonus to the `Health` effects of province diets
    /// (*Regimen sanitatis*).
    DietHealth,
}

impl EffectKind {
    /// Every kind, in declaration order (`ALL[kind.index()] == kind`).
    pub const ALL: [EffectKind; 39] = [
        EffectKind::Wealth,
        EffectKind::Health,
        EffectKind::Unrest,
        EffectKind::GoodsSatisfaction,
        EffectKind::Growth,
        EffectKind::TaxIncome,
        EffectKind::TradeIncome,
        EffectKind::Production,
        EffectKind::ResearchCivil,
        EffectKind::ResearchMilitary,
        EffectKind::RecruitSlots,
        EffectKind::RecruitCost,
        EffectKind::ArmyUpkeep,
        EffectKind::ArmyExperience,
        EffectKind::ArmyMorale,
        EffectKind::ArmyArmor,
        EffectKind::ArmyRanged,
        EffectKind::ArmyMelee,
        EffectKind::Garrison,
        EffectKind::FortificationLevel,
        EffectKind::SiegeResistance,
        EffectKind::Piety,
        EffectKind::Prestige,
        EffectKind::Supply,
        EffectKind::Movement,
        EffectKind::AttritionResistance,
        EffectKind::Loyalty,
        EffectKind::SiegeSpeed,
        EffectKind::ConstructionSpeed,
        EffectKind::Diplomacy,
        EffectKind::Intrigue,
        EffectKind::Fertility,
        EffectKind::BattleCharge,
        EffectKind::BattleRanged,
        EffectKind::BattleDefense,
        EffectKind::ResearchPoints,
        EffectKind::PlagueResistance,
        EffectKind::WoundRecovery,
        EffectKind::DietHealth,
    ];
    /// Number of kinds: the length of any per-kind table.
    pub const COUNT: usize = Self::ALL.len();

    /// Position of the kind in [`EffectKind::ALL`], for per-kind tables.
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// How an effect value combines with the base value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectMode {
    #[default]
    Add,
    Percent,
}

/// Modifier applied to a province, a social class or a faction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub effect: EffectKind,
    /// Targeted class for population effects; `None` = whole province.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<SocialClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_category: Option<UnitCategory>,
    pub value: f64,
    #[serde(default)]
    pub mode: EffectMode,
}

/// Cost in livres tournois plus optional resources.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    pub money: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resources: BTreeMap<ResourceId, u32>,
}

/// Titles of Wikipedia pages or book references backing an entity.
pub type Sources = Vec<String>;

#[cfg(test)]
mod effect_kind_tests {
    use super::EffectKind;

    #[test]
    fn all_is_in_declaration_order() {
        for (position, kind) in EffectKind::ALL.iter().enumerate() {
            assert_eq!(kind.index(), position);
        }
        assert_eq!(EffectKind::DietHealth.index() + 1, EffectKind::COUNT);
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<SocialClass>();
    }
}
