//! Contextual cursor and face-to-face comparison (lot CB-M2, spec
//! `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`,
//! « Curseur contextuel » and « Comparaison au survol »).
//!
//! [`BattleSim::hover_context`] tells the interface what a right click at
//! (x, z) would do with the current selection: move, close in, shoot (or
//! not: out of range, no line of sight), batter or scale the walls, nothing
//! (forbidden ground, a friend). Range, line of sight and what may attack a
//! wall are the rules of the simulation, evaluated here and never in
//! GDScript. Read-only and cheap (no path search): called as the mouse
//! moves. Numbers from `data/rules/battle_hover.json` ([`HoverRules`]).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::setup::SideId;
use crate::sim::BattleSim;

/// What a right click under the cursor would do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoverKind {
    /// Walk there.
    Move,
    /// Close with the enemy (crossed swords).
    Melee,
    /// Shoot at the enemy.
    Ranged,
    /// A shooter's target out of range or out of sight.
    RangedBlocked,
    /// Batter, ram or scale the wall, tower or gate.
    Siege,
    /// Impassable ground, off the field or outside the deployment zone.
    Forbidden,
    /// A friend, or nothing selected.
    None,
}

impl HoverKind {
    /// Key of the context for the interface (`move`, `ranged_blocked`...).
    pub fn key(self) -> &'static str {
        match self {
            HoverKind::Move => "move",
            HoverKind::Melee => "melee",
            HoverKind::Ranged => "ranged",
            HoverKind::RangedBlocked => "ranged_blocked",
            HoverKind::Siege => "siege",
            HoverKind::Forbidden => "forbidden",
            HoverKind::None => "none",
        }
    }
}

/// One regiment's figures in the comparison (same format on both sides).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompareSide {
    pub unit: u32,
    /// Living soldiers.
    pub soldiers: u32,
    pub melee: f64,
    /// Armour plus the general's defence bonus.
    pub defense: f64,
    pub charge: f64,
    pub ranged: f64,
    /// Effective range against the other regiment (0 without missiles).
    pub range: f64,
    pub morale: f64,
    pub fatigue: f64,
    /// Multiplier of the blows struck at the other regiment by the matchup
    /// of the types (pikes against horse...), in percent (100 = none).
    pub bonus_vs: f64,
}

/// A line of the comparison where one side has a net advantage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Advantage {
    Even,
    /// The selected regiment.
    Ours,
    /// The enemy under the cursor.
    Theirs,
}

/// Face-to-face figures of the selected regiment and the hovered enemy,
/// with the net advantage of each line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Compare {
    pub ours: CompareSide,
    pub theirs: CompareSide,
    /// `(line, advantage)` for each line, in display order (`soldiers`,
    /// `melee`, `defense`, `charge`, `ranged`, `range`, `morale`, `fatigue`,
    /// `bonus_vs`); fatigue is an advantage when lower.
    pub advantages: Vec<(String, Advantage)>,
}

/// What the cursor is over and what a right click would do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverContext {
    pub context: HoverKind,
    /// The regiment under the cursor (friend or enemy).
    pub target: Option<u32>,
    /// The wall piece (wall, tower or gate) under the cursor.
    pub piece: Option<usize>,
    /// With a single regiment selected and an enemy under the cursor.
    pub compare: Option<Compare>,
}

/// `data/rules/battle_hover.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoverRules {
    #[serde(default)]
    pub description: String,
    pub pick: PickRules,
    pub compare: CompareRules,
    pub preview: PreviewRules,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickRules {
    /// Margin around a regiment's rectangle (metres).
    pub unit_margin_m: f64,
    /// Margin beyond half the wall's thickness (metres).
    pub piece_margin_m: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompareRules {
    /// Share of the larger value the gap must exceed.
    pub net_advantage_ratio: f64,
    /// Absolute gap the line must exceed.
    pub net_advantage_min: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewRules {
    /// Recompute the path when the aimed point moved farther (metres).
    pub recompute_distance_m: f64,
    /// At most this many recomputes per second.
    pub max_recomputes_per_s: f64,
    /// Beyond this many selected regiments, one path from the centre.
    pub max_individual_paths: u32,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_hover.json");

impl HoverRules {
    /// `data/rules/battle_hover.json` as compiled into the crate.
    pub fn bundled() -> &'static HoverRules {
        static RULES: OnceLock<HoverRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_hover.json is valid")
        })
    }
}

impl BattleSim {
    /// CB-M2: what a right click at (x, z) would do with regiments
    /// `selected` of `side` (the player's side).
    pub fn hover_context(&self, x: f64, z: f64, selected: &[u32], side: SideId) -> HoverContext {
        let _ = (x, z, selected, side);
        HoverContext {
            context: HoverKind::None,
            target: None,
            piece: None,
            compare: None,
        }
    }
}
