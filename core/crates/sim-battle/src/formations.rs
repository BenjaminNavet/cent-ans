//! Regiment formations as data (lot RJ-a, ADR 0174, revises ADR 0095).
//!
//! `data/rules/unit_formations.json` lists every formation a regiment can
//! take: historical name and context (tooltip), the troops allowed, the
//! geometry (a [`FormationShape`] plus ranks, files and spacing) and the
//! modifiers it gives. [`Formation`] is an index into that table, written
//! as its key in JSON (commands, scenarios, replays).
//!
//! A formation order is not instantaneous: the regiment enters a
//! [`Reform`] for `reform_s` seconds (scaled by its size and experience,
//! block `reform`) during which every man walks from his old place to his
//! new one, the regiment moving and fighting worse (`reform` modifiers).

use std::fmt;
use data_model::UnitCategory;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::unit::Unit;

/// Outline of a formation; the numbers come from its [`FormationDef`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationShape {
    /// Straight front of `ranks` ranks (also the deep block).
    Line,
    /// Narrow front of `files` files, as deep as needed.
    Column,
    /// As many files as ranks; no flank.
    Square,
    /// Row k (0 = tip) holds 2k + 1 men.
    Wedge,
    /// Line whose odd ranks stand in the gaps of the rank ahead (quincunx).
    Herse,
}

/// What the battle AI uses a formation for (`ai_role`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiRole {
    /// The formation regiments come back to.
    Default,
    /// Long marches away from the enemy.
    March,
    /// Pikemen facing horse.
    AntiCavalry,
    /// Lancers charging.
    Charge,
}

/// Ranks of a line-shaped formation by class of troops (`None`: not used).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankTable {
    pub infantry: Option<u32>,
    pub ranged: Option<u32>,
    pub cavalry: Option<u32>,
    pub siege: Option<u32>,
}

/// Files of a column, on foot and mounted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileTable {
    pub foot: u32,
    pub mounted: u32,
}

impl Default for FileTable {
    fn default() -> Self {
        FileTable {
            foot: 6,
            mounted: 4,
        }
    }
}

/// Multipliers of the base spacing between men.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpacingFactors {
    #[serde(default = "one")]
    pub lateral: f64,
    #[serde(default = "one")]
    pub depth: f64,
}

impl Default for SpacingFactors {
    fn default() -> Self {
        SpacingFactors {
            lateral: 1.0,
            depth: 1.0,
        }
    }
}

fn one() -> f64 {
    1.0
}

fn two() -> f64 {
    2.0
}

/// Effects of a formation (1 = none).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormationModifiers {
    /// Pace multiplier.
    #[serde(default = "one")]
    pub speed: f64,
    /// Multiplier of the charge bonus and weight.
    #[serde(default = "one")]
    pub charge: f64,
    /// Multiplier of the melee damage taken.
    #[serde(default = "one")]
    pub melee_taken: f64,
    /// Multiplier of the missile casualties taken.
    #[serde(default = "one")]
    pub missile_taken: f64,
    /// Multiplier of the casualties the regiment's shots inflict.
    #[serde(default = "one")]
    pub shooting: f64,
    /// Multiplier of the morale lost to casualties.
    #[serde(default = "one")]
    pub morale_loss: f64,
    /// Multiplier of the push the regiment exerts in melee.
    #[serde(default = "one")]
    pub push_drive: f64,
    /// Multiplier of the push it withstands.
    #[serde(default = "one")]
    pub push_resistance: f64,
    /// Multiplier of the blows of horsemen at it (braced formations only).
    #[serde(default = "one")]
    pub horse_blows: f64,
    /// Men fighting per file of the front.
    #[serde(default = "two")]
    pub fighting_ranks: f64,
}

impl Default for FormationModifiers {
    fn default() -> Self {
        serde_json::from_str("{}").expect("modifier defaults")
    }
}

/// One formation of `data/rules/unit_formations.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormationDef {
    pub key: String,
    /// Historical name (French).
    pub name: String,
    /// Short label for buttons.
    pub short: String,
    /// Historical context for the tooltip (French).
    pub description: String,
    pub categories: Vec<UnitCategory>,
    #[serde(default)]
    pub foot_only: bool,
    #[serde(default)]
    pub mounted_only: bool,
    pub shape: FormationShape,
    #[serde(default)]
    pub ranks: RankTable,
    #[serde(default)]
    pub files: FileTable,
    #[serde(default)]
    pub spacing: SpacingFactors,
    #[serde(default)]
    pub modifiers: FormationModifiers,
    /// No flank nor rear: every blow lands on the front.
    #[serde(default)]
    pub all_round: bool,
    /// Braced against horse: no man knocked down, no shock to morale, levelled
    /// pikes stop a charge from any side; cavalry avoids charging it.
    #[serde(default)]
    pub braced: bool,
    /// Takes the road bonus of `battle_water.json` (`road_column`).
    #[serde(default)]
    pub road_march: bool,
    /// A right-drag sets its width (CB1) without leaving it.
    #[serde(default)]
    pub width_adjustable: bool,
    #[serde(default)]
    pub ai_role: Option<AiRole>,
    /// Base seconds to take this formation (scaled by `reform`).
    pub reform_s: f64,
}

impl FormationDef {
    /// The regiment may take this formation.
    pub fn allows(&self, unit: &Unit) -> bool {
        self.categories.contains(&unit.category)
            && !(self.foot_only && unit.mounted)
            && !(self.mounted_only && !unit.mounted)
    }

    /// Ranks of a line-shaped formation for this regiment's class.
    pub fn ranks_for(&self, unit: &Unit) -> u32 {
        let class = if unit.category == UnitCategory::Siege {
            self.ranks.siege
        } else if unit.mounted {
            self.ranks.cavalry
        } else if unit.category == UnitCategory::Ranged {
            self.ranks.ranged
        } else {
            self.ranks.infantry
        };
        class
            .or(self.ranks.infantry)
            .or(self.ranks.ranged)
            .or(self.ranks.cavalry)
            .unwrap_or(1)
            .max(1)
    }
}

/// Seconds, slow-down and weakness of a regiment changing formation.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReformRules {
    /// Regiment size at which `reform_s` applies as is.
    pub size_reference: f64,
    /// Duration × (soldiers / size_reference) ^ size_exponent ...
    pub size_exponent: f64,
    /// ... within [size_min, size_max].
    pub size_min: f64,
    pub size_max: f64,
    /// Duration × (1 − experience_step × experience), at least experience_min.
    pub experience_step: f64,
    pub experience_min: f64,
    /// Walking pace of the men to their new place (m/s), raised when needed
    /// to arrive in time.
    pub walk_mps: WalkPace,
    /// Multipliers while reforming.
    pub speed: f64,
    pub melee_taken: f64,
    pub missile_taken: f64,
    pub morale_loss: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkPace {
    pub foot: f64,
    pub mounted: f64,
}

/// `data/rules/unit_formations.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormationRules {
    #[serde(default)]
    pub description: String,
    /// Key of the formation every regiment starts in.
    pub default: String,
    pub reform: ReformRules,
    pub formations: Vec<FormationDef>,
}

data_model::bundled_rules!(FormationRules, "rules/unit_formations.json");

impl FormationRules {

    /// Consistency the schema cannot express.
    pub fn check(&self) -> Result<(), String> {
        if self.formations.is_empty() || self.formations.len() > usize::from(u8::MAX) {
            return Err("1 to 255 formations".into());
        }
        for (i, f) in self.formations.iter().enumerate() {
            if self.formations[..i].iter().any(|g| g.key == f.key) {
                return Err(format!("duplicate formation {}", f.key));
            }
            if f.width_adjustable && f.shape != FormationShape::Line {
                return Err(format!("{}: only a line takes a dragged width", f.key));
            }
            if f.reform_s < 0.0 {
                return Err(format!("{}: negative reform_s", f.key));
            }
        }
        let default = self
            .formations
            .iter()
            .find(|f| f.key == self.default)
            .ok_or_else(|| format!("default formation {} missing", self.default))?;
        for category in [
            UnitCategory::Infantry,
            UnitCategory::Ranged,
            UnitCategory::Cavalry,
            UnitCategory::Siege,
        ] {
            if !default.categories.contains(&category) || default.foot_only || default.mounted_only
            {
                return Err("the default formation must allow every regiment".into());
            }
        }
        Ok(())
    }

    pub fn get(&self, formation: Formation) -> &FormationDef {
        &self.formations[usize::from(formation.0)]
    }

    /// Duration of a change to `to` for `unit` (seconds).
    pub fn reform_duration(&self, unit: &Unit, to: Formation) -> f64 {
        let r = &self.reform;
        let size = (f64::from(unit.soldiers().max(1)) / r.size_reference.max(1.0))
            .powf(r.size_exponent)
            .clamp(r.size_min, r.size_max);
        let veteran = (1.0 - r.experience_step * f64::from(unit.experience)).max(r.experience_min);
        self.get(to).reform_s * size * veteran
    }
}

/// A formation of [`FormationRules::bundled`], written as its key.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Formation(u8);

impl Formation {
    /// The formation with this key, if the data has one.
    pub fn try_of(key: &str) -> Option<Formation> {
        FormationRules::bundled()
            .formations
            .iter()
            .position(|f| f.key == key)
            .map(|i| Formation(i as u8))
    }

    /// The formation with this key (panics when the data has none: tests
    /// and the few keys the rules name).
    pub fn of(key: &str) -> Formation {
        Self::try_of(key).unwrap_or_else(|| panic!("no formation {key} in unit_formations.json"))
    }

    /// The formation every regiment starts in.
    pub fn default_formation() -> Formation {
        Self::of(&FormationRules::bundled().default)
    }

    /// Every formation, in data order.
    pub fn all() -> impl Iterator<Item = Formation> {
        (0..FormationRules::bundled().formations.len()).map(|i| Formation(i as u8))
    }

    /// The first formation of `role` that `unit` may take.
    pub fn for_role(role: AiRole, unit: &Unit) -> Option<Formation> {
        Self::all().find(|f| f.def().ai_role == Some(role) && f.def().allows(unit))
    }

    pub fn def(self) -> &'static FormationDef {
        FormationRules::bundled().get(self)
    }

    pub fn key(self) -> &'static str {
        &self.def().key
    }

    pub fn label_fr(self) -> &'static str {
        &self.def().name
    }

    pub fn shape(self) -> FormationShape {
        self.def().shape
    }

    pub fn role(self) -> Option<AiRole> {
        self.def().ai_role
    }
}

impl Default for Formation {
    fn default() -> Self {
        Self::default_formation()
    }
}

impl fmt::Debug for Formation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

impl Serialize for Formation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.key())
    }
}

impl<'de> Deserialize<'de> for Formation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let key = String::deserialize(deserializer)?;
        Formation::try_of(&key)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown formation {key}")))
    }
}

/// A regiment changing formation: its men walk from their places in `from`
/// (with the dragged width `from_files`) to those of the current formation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Reform {
    pub from: Formation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_files: Option<u32>,
    pub elapsed: f64,
    pub duration: f64,
}

impl Reform {
    /// Share of the change done (0-1).
    pub fn progress(&self) -> f64 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }
}
