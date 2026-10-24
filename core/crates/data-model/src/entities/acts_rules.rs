//! The campaign arc in three acts, mirroring `data/schemas/acts_rules.schema.json`
//! (`data/rules/acts.json`, ADR 0784): the acts and their dates, the objectives of a
//! house per act, the pressures that change from act to act, the policies the player
//! chooses at each transition, and the weights of the reign score.
//! The whole file is optional: without it the campaign has no acts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{FactionId, ProvinceId, TitleId};

/// A condition of an act objective or pressure, evaluated on the state of the player's realm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActCondition {
    /// Every listed province is held.
    ControlAll { provinces: Vec<ProvinceId> },
    /// At least `count` of the listed provinces are held.
    ControlCount {
        provinces: Vec<ProvinceId>,
        count: u32,
    },
    /// `faction` holds none of the listed provinces.
    NoForeignControl {
        faction: FactionId,
        provinces: Vec<ProvinceId>,
    },
    /// `faction` holds at least `count` of the listed provinces (pressure: the foreigner in the realm).
    ForeignControlAtLeast {
        faction: FactionId,
        provinces: Vec<ProvinceId>,
        count: u32,
    },
    /// At least `count` provinces are held.
    ProvinceCount { count: u32 },
    /// At least `percent` of the provinces held at the start of the campaign are still held.
    DomainPercent { percent: u32 },
    /// The ruler's prestige is at least `min`.
    Prestige { min: u32 },
    /// The treasury holds at least `min` livres.
    Treasury { min: i64 },
    /// The bankers are owed at most `max` livres.
    DebtBelow { max: i64 },
    /// At least `count` allies.
    AlliesAtLeast { count: u32 },
    /// A free ruler (not captive) with an heir, and no regency.
    DynastySecure,
    /// The faction holds the title.
    HoldTitle { title: TitleId },
    /// The faction has no suzerain.
    Independent,
    /// `faction` is a vassal of the realm, or gone.
    Subjugate { faction: FactionId },
    /// At war with `faction`.
    AtWar { faction: FactionId },
}

/// One objective of the house during an act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActObjective {
    pub id: String,
    pub title: String,
    pub description: String,
    /// Category (`domain`, `treasury`, `dynasty`, `prestige`, `alliance`): policies favour one.
    pub category: String,
    /// Weight of the objective in the act score.
    pub weight: u32,
    pub condition: ActCondition,
}

/// Who a pressure applies to (empty lists: everybody).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ActAudience {
    pub factions: Vec<FactionId>,
    /// Government names (`kingdom`, `duchy`...).
    pub governments: Vec<String>,
}

/// A pressure of the act: a standing condition that adds or removes score points when it holds
/// at the end of the act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActPressure {
    pub id: String,
    pub title: String,
    /// Text shown while the pressure holds.
    pub text: String,
    #[serde(default)]
    pub audience: ActAudience,
    pub condition: ActCondition,
    /// Score points (act score, 0-100) added when the condition holds (negative: a penalty).
    pub points: i32,
}

/// A policy of the household chosen at the start of an act: it counts one category of
/// objectives `factor_percent` percent as much.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub factor_percent: u32,
}

/// Early ends of an act (besides its closing year); `0` disables a test.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ActEarlyEnd {
    /// Seasons at peace (no war) that close the act.
    pub peace_seasons: u32,
    /// An act never ends by peace before it ran this many seasons.
    pub min_seasons: u32,
    /// Collapse: fewer than this percent of the provinces held at the start of the act.
    pub collapse_domain_percent: u32,
    /// Collapse: this many royal defaults in the act.
    pub collapse_defaults: u32,
}

/// One act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActRules {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub start_year: i32,
    /// Last year of the act: it closes when the year is past it.
    pub end_year: i32,
    /// Weight of the act in the reign score.
    pub weight: u32,
    /// Opening text (transition screen).
    pub intro: String,
    /// Closing text per way the act ended: `year`, `peace`, `collapse`.
    pub closing: BTreeMap<String, String>,
    #[serde(default)]
    pub early_end: ActEarlyEnd,
    pub policies: Vec<ActPolicy>,
    #[serde(default)]
    pub pressures: Vec<ActPressure>,
}

/// A grade of the reign (score thresholds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReignGrade {
    pub min_score: u32,
    pub title: String,
    pub text: String,
}

/// Defeat conditions of the realm (`0` disables a test).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RuinRules {
    /// Irreversible royal bankruptcy: this many royal defaults, treasury still negative.
    pub defaults: u32,
    /// The capital is lost and the crown has no heir for this many seasons.
    pub capital_lost_seasons: u32,
    /// Governments whose line ends with the dynasty (elective ones are spared).
    pub dynastic_governments: Vec<String>,
}

/// Contents of `data/rules/acts.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ActsRules {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub acts: Vec<ActRules>,
    /// Display name of each objective category.
    pub categories: BTreeMap<String, String>,
    /// Objectives per house: key `fac_...` (one faction), `gov_...` (a government) or
    /// `default`; then per act id.
    pub objective_sets: BTreeMap<String, BTreeMap<String, Vec<ActObjective>>>,
    /// Reign grades, any order (the highest threshold reached wins).
    pub grades: Vec<ReignGrade>,
    /// Reign score (0-100) from which the campaign is won.
    pub victory_min_score: u32,
    pub ruin: RuinRules,
    /// Interface texts.
    pub texts: BTreeMap<String, String>,
}

impl ActsRules {
    /// Whether the arc is active (at least one act).
    pub fn enabled(&self) -> bool {
        !self.acts.is_empty()
    }

    /// Objectives of `act` for a house: its faction set, else its government's, else the default.
    pub fn objectives(&self, faction: &FactionId, government: &str, act: &str) -> &[ActObjective] {
        let keys = [
            faction.to_string(),
            format!("gov_{government}"),
            "default".to_owned(),
        ];
        for key in keys {
            if let Some(list) = self.objective_sets.get(&key).and_then(|set| set.get(act)) {
                return list;
            }
        }
        &[]
    }

    /// Grade reached by a reign score.
    pub fn grade(&self, score: u32) -> Option<&ReignGrade> {
        self.grades
            .iter()
            .filter(|g| g.min_score <= score)
            .max_by_key(|g| g.min_score)
    }
}
