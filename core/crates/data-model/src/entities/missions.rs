//! Short-term campaign missions (lot NT3, ADR 0127), mirroring
//! `data/schemas/missions.schema.json` (`data/missions.json`).

use crate::key_enum;
use serde::{Deserialize, Serialize};

/// Contents of `data/missions.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Missions active at most at the same time (1-2).
    pub max_active: u32,
    /// First turn a mission may be offered.
    pub first_turn: u32,
    /// Turns to wait after a success or a failure before a new offer.
    pub offer_cooldown_turns: u32,
    /// Ruler prestige on failure (0 or a small negative number).
    pub failure_prestige: i32,
    pub templates: Vec<MissionTemplate>,
}

crate::bundled_rules!(MissionRules, "missions.json", default);

key_enum! {
/// What a mission asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionKind {
    /// Take a neighbouring province held by a faction at war with the player.
    TakeProvince => "take_province",
    /// Win `count` battles.
    WinBattle => "win_battle",
    /// Complete a given building in a given place.
    ConstructBuilding => "construct_building",
    /// Recruit or hire `count` units.
    RecruitUnits => "recruit_units",
    /// Sign a new peace, alliance or vassalage.
    ConcludeTreaty => "conclude_treaty",
    /// Keep a threatened border place until the deadline.
    HoldPlace => "hold_place",
}
}

/// One mission template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionTemplate {
    pub id: String,
    pub kind: MissionKind,
    /// French title; `{cible}`, `{lieu}` and `{n}` are filled in.
    pub title: String,
    /// French objective text; same placeholders.
    pub objective: String,
    /// Turns before the deadline (3-12).
    pub duration: u32,
    /// Number asked for (battles, units).
    #[serde(default = "one")]
    pub count: u32,
    /// Relative weight of the template among the plausible ones.
    #[serde(default = "one")]
    pub weight: u32,
    pub reward: MissionReward,
}

fn one() -> u32 {
    1
}

/// What a success brings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionReward {
    /// Livres paid into the treasury.
    #[serde(default)]
    pub gold: i64,
    /// Ruler prestige.
    #[serde(default)]
    pub prestige: i32,
    /// Unrest removed from the target province (or the capital).
    #[serde(default)]
    pub public_order: u8,
    /// A free company joins the capital's garrison.
    #[serde(default)]
    pub free_unit: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_file_parses_with_every_kind() {
        let rules = MissionRules::default();
        assert!((1..=2).contains(&rules.max_active));
        for kind in [
            MissionKind::TakeProvince,
            MissionKind::WinBattle,
            MissionKind::ConstructBuilding,
            MissionKind::RecruitUnits,
            MissionKind::ConcludeTreaty,
            MissionKind::HoldPlace,
        ] {
            assert!(
                rules.templates.iter().any(|t| t.kind == kind),
                "{}",
                kind.key()
            );
        }
        for t in &rules.templates {
            assert!((3..=12).contains(&t.duration), "{}", t.id);
        }
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<MissionKind>();
    }
}
