//! The campaign arc in three acts, mirroring `data/schemas/acts_rules.schema.json`
//! (`data/rules/acts.json`, ADR 0784): the acts and their dates, the objectives of a
//! house per act, the pressures that change from act to act, the ambitions the player
//! chooses at each transition (R1-2), the renown of the house (R1-1) and the weights of the
//! reign score. The whole file is optional: without it the campaign has no acts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Effect, FactionId, ProvinceId, TitleId};

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

/// A pressure of the act: a standing condition that adds renown while it holds during the act
/// (R1-1: added when it starts to hold, removed when it ends).
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
    /// Renown while the condition holds (negative: a penalty), scaled by the `pressure` source.
    pub points: i32,
}

/// R1-2: an ambition of the household, chosen at the opening of an act (`ambitions_to_choose`).
/// It changes the simulation through the shared effect vocabulary (merged into the realm's
/// effects like a technology) and earns more renown from the facts and objectives it names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActAmbition {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Effects on the realm while the ambition is held (never empty: a pure weighting is refused).
    pub effects: Vec<Effect>,
    /// Extra renown, in percent, from the sources and objective categories below.
    pub renown_bonus_percent: u32,
    /// Renown sources the bonus applies to (`battle_won`, `place_taken`...).
    #[serde(default)]
    pub renown_sources: Vec<String>,
    /// Objective categories the bonus applies to (`domain`, `treasury`...).
    #[serde(default)]
    pub objective_categories: Vec<String>,
}

/// R1-1: one source of renown: points per fact and the most a season can bring (absolute value;
/// 0: no cap). For `objective_met`, `objective_lost` and `pressure`, `points` is a percent of the
/// objective weight or of the pressure points.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenownSource {
    pub points: i32,
    #[serde(default)]
    pub season_cap: u32,
}

/// R1-1: the renown of the house, accumulated season by season from facts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RenownRules {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Sources by name (`battle_won`, `battle_lost`, `place_taken`, `place_lost`, `raid_season`,
    /// `ransom_received`, `ransom_paid`, `truce_imposed`, `objective_met`, `objective_lost`,
    /// `pressure`).
    pub sources: BTreeMap<String, RenownSource>,
    /// Wear of the renown of facts of the act in progress, per mille per season.
    pub decay_permille_per_season: u32,
    /// Facts kept in the journal (the oldest leave).
    pub journal_len: u32,
    /// A treaty ending a war is a truce imposed when the house's war score is at least this.
    pub truce_min_war_score: i32,
    /// French label of each source (journal lines).
    pub labels: BTreeMap<String, String>,
}

impl RenownRules {
    /// The rule of a source (`None`: the source earns nothing).
    pub fn source(&self, name: &str) -> Option<&RenownSource> {
        self.sources.get(name)
    }
}

/// One act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActRules {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub start_year: i32,
    /// Last year of the act: it closes when the year is past it, and only then (R1-1).
    pub end_year: i32,
    /// Weight of the act in the reign score.
    pub weight: u32,
    /// Opening text (transition screen).
    pub intro: String,
    /// Closing text per way the act ended (R1-1: `year` only).
    pub closing: BTreeMap<String, String>,
    /// R1-1: renown earned in the act that makes an act score of 100.
    pub renown_target: u32,
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
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
    /// R1-2: ambitions per house (same keys as `objective_sets`), then per act id.
    pub ambition_sets: BTreeMap<String, BTreeMap<String, Vec<ActAmbition>>>,
    /// R1-2: ambitions the player holds per act.
    pub ambitions_to_choose: u32,
    /// R1-2: seasons after the opening of an act during which chosen ambitions may still change.
    pub ambition_choice_seasons: u32,
    /// R1-1: renown sources, caps and wear.
    pub renown: RenownRules,
    /// Reign grades, any order (the highest threshold reached wins).
    pub grades: Vec<ReignGrade>,
    /// Reign score (0-100) from which the campaign is won (in 1453 only).
    pub victory_min_score: u32,
    pub ruin: RuinRules,
    /// Interface texts.
    pub texts: BTreeMap<String, String>,
}

fn lookup<'a, T>(
    sets: &'a BTreeMap<String, BTreeMap<String, Vec<T>>>,
    faction: &FactionId,
    government: &str,
    act: &str,
) -> &'a [T] {
    let keys = [
        faction.to_string(),
        format!("gov_{government}"),
        "default".to_owned(),
    ];
    for key in keys {
        if let Some(list) = sets.get(&key).and_then(|set| set.get(act)) {
            return list;
        }
    }
    &[]
}

impl ActsRules {
    /// Whether the arc is active (at least one act).
    pub fn enabled(&self) -> bool {
        !self.acts.is_empty()
    }

    /// Objectives of `act` for a house: its faction set, else its government's, else the default.
    pub fn objectives(&self, faction: &FactionId, government: &str, act: &str) -> &[ActObjective] {
        lookup(&self.objective_sets, faction, government, act)
    }

    /// R1-2: ambitions of `act` for a house: its faction set, else its government's, else the
    /// default.
    pub fn ambitions(&self, faction: &FactionId, government: &str, act: &str) -> &[ActAmbition] {
        lookup(&self.ambition_sets, faction, government, act)
    }

    /// Grade reached by a reign score.
    pub fn grade(&self, score: u32) -> Option<&ReignGrade> {
        self.grades
            .iter()
            .filter(|g| g.min_score <= score)
            .max_by_key(|g| g.min_score)
    }
}
