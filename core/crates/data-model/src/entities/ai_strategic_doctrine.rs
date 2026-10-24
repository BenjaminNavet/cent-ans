//! Strategic doctrines of the campaign AI (ADR 0782), mirroring
//! `data/schemas/ai_strategic_doctrine.schema.json` (`data/ai/strategic_doctrines.json`).
//!
//! A doctrine is a named, readable personality: it weighs the objectives of a realm, its taste for
//! risk in battle, sieges and chevauchées, its diplomacy and its finances. Every percentage is
//! neutral at 100 and every objective weight at 50, so the `balanced` doctrine changes nothing.
//! Which doctrine a faction follows (by faction, sovereign, era or after defeats) is the
//! `assignments` list, resolved in `sim_campaign::doctrines`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{CharacterId, FactionId};

/// Weight of each aim of a realm, 0-100 (50 is neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrineObjectives {
    /// Value of taking cities and provinces (siege targets, the claimed crown).
    pub conquest: u8,
    /// Value of recovering lost places and fighting those who beat the realm.
    pub revenge: u8,
    /// Value of keeping the own settlements (defence, garrisons).
    pub security: u8,
    /// Value of the income of a target and of a sound treasury.
    pub treasury: u8,
    /// Value of the enemy capital and of the throne claimed.
    pub prestige: u8,
}

impl DoctrineObjectives {
    /// A weight 0-100 as a multiplier: 50 is neutral (1.0), 0 cancels, 100 doubles.
    pub fn factor(weight: u8) -> f64 {
        f64::from(weight.min(100)) / 50.0
    }
}

/// Taste for risk in the field (percentages, neutral at 100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrineBattle {
    /// Superiority demanded before attacking an enemy army, % of normal (below 100: seeks battle).
    pub engage_ratio_percent: u32,
    /// Share of strength under which an army falls back, % of normal (above 100: prudent).
    pub retreat_percent: u32,
    /// Superiority demanded over the defence of a place before besieging it, % of normal.
    pub siege_superiority_percent: u32,
    /// Assault odds demanded before storming the walls, points added.
    pub assault_odds_delta: i32,
}

/// Appetite for war and the way it is waged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrineWar {
    /// Points added to the aggression of the faction (chevauchées, ambushes, wars declared).
    pub aggression_delta: i32,
    /// Power ratio demanded before declaring a war, % of normal (below 100: bolder).
    pub war_ratio_percent: u32,
    /// The chevauchée comes before the siege when true (Edward III in 1346).
    pub raid_first: bool,
}

/// Diplomacy (percentages, neutral at 100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrineDiplomacy {
    /// Frequency of the search for alliances, % of normal.
    pub alliance_percent: u32,
    /// Frequency of the marriages arranged, % of normal (below 100: every other year).
    pub marriage_percent: u32,
}

/// Finances (percentages, neutral at 100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrineFinance {
    /// Below 50, the faction borrows only when its treasury is empty.
    pub borrow_percent: u32,
    /// Below 50, the faction raises an extraordinary tax only when its treasury is empty.
    pub levy_percent: u32,
}

/// One doctrine of the catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategicDoctrine {
    /// French name shown to the player.
    pub label: String,
    /// French description (intentions of the faction).
    pub description: String,
    pub objectives: DoctrineObjectives,
    pub battle: DoctrineBattle,
    pub war: DoctrineWar,
    pub diplomacy: DoctrineDiplomacy,
    pub finance: DoctrineFinance,
}

/// Condition under which a doctrine applies; every given condition must hold.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrineCondition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
    /// The sovereign of the faction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruler: Option<CharacterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_year: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_year: Option<i32>,
    /// The faction has lost at least this many field battles in all (a defeat changes the plan).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_defeats: Option<u32>,
}

/// One line of the assignment list: the first line whose condition holds names the doctrine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrineAssignment {
    #[serde(rename = "if")]
    pub condition: DoctrineCondition,
    pub doctrine: String,
}

/// Constants shared by all doctrines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctrineTuning {
    /// Field battles a rival must have won over the faction, beyond those the faction won over
    /// it, to be a target of revenge.
    pub revenge_min_net_defeats: u32,
    /// Discount (per cent, at a revenge weight of 100) on the power ratio demanded before a war
    /// declared on a rival that beat the realm.
    pub revenge_war_discount_percent: u32,
    /// Value added to a siege target held by a rival of revenge (at a revenge weight of 50).
    pub revenge_target_bonus: f64,
}

/// Contents of `data/ai/strategic_doctrines.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategicDoctrines {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Doctrine of a faction no assignment names.
    pub default: String,
    pub doctrines: BTreeMap<String, StrategicDoctrine>,
    pub assignments: Vec<DoctrineAssignment>,
    pub tuning: DoctrineTuning,
}

crate::bundled_rules!(StrategicDoctrines, "ai/strategic_doctrines.json", default);

#[cfg(test)]
mod tests {
    use super::StrategicDoctrines;

    #[test]
    fn bundled_file_parses_and_is_consistent() {
        let d = StrategicDoctrines::bundled();
        assert!(d.doctrines.contains_key(&d.default));
        for a in &d.assignments {
            assert!(d.doctrines.contains_key(&a.doctrine), "{}", a.doctrine);
        }
    }
}
