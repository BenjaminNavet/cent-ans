//! Leader's battle order ("ordres du chef", `battle_order.schema.json`,
//! `data/battle_orders/`). The rules live in `sim-battle`; this is only the
//! catalogue: names, texts and every number of the order.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::common::{Sources, UnitCategory};
use crate::entities::unit_type::Ability;
use crate::ids::FactionId;

/// What the order does (each kind has its own rule in `sim-battle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleOrderKind {
    /// Temporary morale bonus around the general.
    WarCry,
    /// Morale bonus to the whole army for the battle; no prisoners.
    NoQuarter,
    /// Heavy horse fight on foot (irreversible).
    Dismount,
    /// Crossbowmen raise their pavises: immobile, far less hurt by missiles.
    Pavise,
    /// The general tries to rally routing regiments near him.
    Rally,
}

/// Which regiments the order reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleOrderScope {
    /// Regiments within `radius` metres of the general.
    Radius,
    /// The whole army.
    Army,
    /// The regiments named in the command (every eligible one when none is).
    Selected,
}

/// Regiments an order may apply to (every listed condition must hold).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleOrderFilter {
    /// Allowed categories (any when empty).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<UnitCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mounted: Option<bool>,
    /// Required abilities (all of them).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<Ability>,
}

/// Numbers of the order's effect (only those of its kind are read).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleOrderEffects {
    /// Morale points added at once and to the recovery ceiling while the
    /// order lasts (war cry, no quarter).
    #[serde(default)]
    pub morale: f64,
    /// Dismount: speed ceiling of the regiment on foot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_max: Option<u8>,
    /// Dismount: armour points added (men fight in closer order, shields up).
    #[serde(default)]
    pub armor: u8,
    /// Pavise: multiplier of the missile casualties taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missile_damage_factor: Option<f64>,
    /// Rally: chance of success for a general of command 0 (0-1).
    #[serde(default)]
    pub rally_chance: f64,
    /// Rally: chance added per command point (0-10).
    #[serde(default)]
    pub rally_chance_per_command: f64,
    /// Rally: morale of a regiment that answers the call.
    #[serde(default)]
    pub rally_morale: f64,
}

/// When the battle AI gives the order.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleOrderAi {
    /// An enemy closer than this (metres) to a regiment reached by the order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_within: Option<f64>,
    /// At least this many regiments reached by the order.
    #[serde(default = "one")]
    pub min_units: u32,
    /// Own strength / enemy strength below this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_strength_ratio: Option<f64>,
    /// Only against a hereditary enemy (pairs of factions, either order).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rivals: Vec<[FactionId; 2]>,
    /// Pavise: the regiment took missile casualties within this many seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub under_fire_within: Option<f64>,
    /// Dismount: only when the side stands on the defensive.
    #[serde(default)]
    pub when_defensive: bool,
}

fn one() -> u32 {
    1
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleOrder {
    /// `order_*`, equal to the file name.
    pub id: String,
    pub kind: BattleOrderKind,
    /// Position in the order bar (ascending).
    #[serde(default)]
    pub rank: u8,
    /// Generic French name ("Cri de guerre", "Pas de quartier").
    pub name: String,
    /// Default wording of the order when the faction has none of its own
    /// ("Déployer la bannière rouge"); `name` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Faction-specific wording (the war cry of each host, the oriflamme).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels_by_faction: BTreeMap<FactionId, String>,
    /// French description with its historical reference.
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub scope: BattleOrderScope,
    /// Only while the general lives and commands.
    #[serde(default = "yes")]
    pub requires_general: bool,
    /// Seconds before the order can be given again.
    pub cooldown: f64,
    /// Seconds the effect lasts (0 = instantaneous or for the whole battle).
    #[serde(default)]
    pub duration: f64,
    /// Metres around the general (`scope: radius`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uses_per_battle: Option<u32>,
    #[serde(default)]
    pub eligible: BattleOrderFilter,
    #[serde(default)]
    pub effects: BattleOrderEffects,
    /// Journal line; placeholders `{label}`, `{faction}`, `{of_faction}`,
    /// `{general}`, `{unit}`, `{count}`.
    pub journal: String,
    /// Journal line when the order fails (rally).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_failure: Option<String>,
    /// Dismount: journal line of the automatic dismount before an assault.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_assault: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<BattleOrderAi>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl BattleOrder {
    /// The wording used by `faction` (its own war cry, banner...).
    pub fn label_for(&self, faction: &str) -> &str {
        self.labels_by_faction
            .iter()
            .find(|(id, _)| id.as_str() == faction)
            .map(|(_, label)| label.as_str())
            .or(self.label.as_deref())
            .unwrap_or(self.name.as_str())
    }
}
