//! The crusader faction's « Ferveur » (lot JR1, ADR 0165), mirroring
//! `data/schemas/crusade_rules.schema.json` (`data/rules/crusade.json`).
//! Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md` § 4.
//!
//! The file is optional: without it the mechanic is inert
//! ([`crate::GameData::crusade_rules`] is `None`). It names the faction, its
//! base and its goal, so the simulation holds no faction or province id.

use serde::{Deserialize, Serialize};

use crate::ids::{FactionId, ProvinceId, SettlementId, UnitTypeId};

/// Contents of `data/rules/crusade.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrusadeRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The faction that lives on fervour.
    pub faction: FactionId,
    /// Settlement its starting army stands in (it holds no city).
    pub base_settlement: SettlementId,
    /// The province of the vow (Jerusalem).
    pub target_province: ProvinceId,
    /// Provinces of the Holy Land: a settlement taken there raises fervour.
    pub holy_land: Vec<ProvinceId>,
    /// The coastal ones (« un port pour les pèlerins »): a held port there
    /// receives the contingents first.
    pub coastal_holy_land: Vec<ProvinceId>,
    pub fervor: CrusadeFervor,
    pub alms: CrusadeAlms,
    pub passage: CrusadePassage,
    pub zeal: CrusadeZeal,
    pub desertion: CrusadeDesertion,
    /// Unit types of the starting army, in order.
    pub starting_army: Vec<UnitTypeId>,
    /// Prestige the ruler gains when the target province is taken.
    pub target_taken_prestige: i32,
}

/// What moves the 0-100 gauge: signed points, except `start`, `target_floor`
/// (levels) and `decay_per_turn` (points lost each turn).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrusadeFervor {
    pub start: u8,
    pub decay_per_turn: u8,
    /// Extra points lost each turn at or above the high zeal threshold
    /// (« l'exaltation retombe »: keeps the gauge off its ceiling).
    #[serde(default)]
    pub decay_above_high: u8,
    pub battle_won_other_faith: i32,
    pub battle_lost: i32,
    pub holy_land_settlement_taken: i32,
    pub target_taken: i32,
    /// Fervour cannot fall below this while the target province is held.
    pub target_floor: u8,
    pub preach: i32,
    pub war_declared_same_faith: i32,
    pub battle_same_faith: i32,
    /// Each turn at peace or truce with the holder of the target province.
    pub truce_with_target_holder_per_turn: i32,
}

/// Alms of Christendom: `base + per_fervor × fervour` livres a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrusadeAlms {
    pub base: i64,
    pub per_fervor: i64,
}

/// « Prêcher le passage »: a paid call for volunteers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrusadePassage {
    /// Livres, before the faction's price level.
    pub cost: i64,
    /// Turns before the passage can be preached again.
    pub cooldown_turns: u32,
    /// Turns before the contingent lands.
    pub delay_turns: u32,
    /// Units of a contingent at fervour 0.
    pub units_base: u32,
    /// One more unit per this many points of fervour.
    pub fervor_per_extra_unit: u32,
    pub max_units: u32,
    /// Weighted draw of the contingent's unit types.
    pub unit_table: Vec<CrusadePassageUnit>,
}

impl CrusadePassage {
    /// Units of a contingent preached at `fervor`.
    pub fn units_at(&self, fervor: u8) -> u32 {
        let extra = u32::from(fervor)
            .checked_div(self.fervor_per_extra_unit)
            .unwrap_or(0);
        (self.units_base + extra).min(self.max_units)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrusadePassageUnit {
    pub unit: UnitTypeId,
    pub weight: u32,
}

/// « Élan de la Croix »: morale of the faction's armies (points of the
/// 0-100 morale).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrusadeZeal {
    /// Fervour at or above: `high_morale`.
    pub high_threshold: u8,
    pub high_morale: i32,
    /// Fervour below: `low_morale`.
    pub low_threshold: u8,
    pub low_morale: i32,
}

/// « Débandade »: below `threshold`, men go home every turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrusadeDesertion {
    pub threshold: u8,
    /// Percent of each unit's men lost a turn.
    pub men_percent_per_turn: u32,
    /// Multiplier of that share at fervour 0.
    pub zero_multiplier: u32,
}

impl CrusadeDesertion {
    /// Percent of the men who leave this turn at `fervor` (0: none).
    pub fn percent_at(&self, fervor: u8) -> u32 {
        if fervor >= self.threshold {
            0
        } else if fervor == 0 {
            (self.men_percent_per_turn * self.zero_multiplier).min(100)
        } else {
            self.men_percent_per_turn.min(100)
        }
    }
}
