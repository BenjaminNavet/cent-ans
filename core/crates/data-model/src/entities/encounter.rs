//! Map encounters (lot CV3-3, spec `docs/design/2026-09-27-campagne-vivante.md`
//! § 2): points of interest that appear on the campaign map, mirroring
//! `data/schemas/encounter.schema.json` (`data/encounters/enc_*.json`) and
//! `data/schemas/encounter_rules.schema.json` (`data/rules/encounters.json`).
//!
//! Conditions and effects reuse the chronicle's typed [`Condition`] and
//! [`EventEffect`]; effects on the army that met the encounter use the typed
//! [`Effect`] (`army_morale`, `supply`, `army_experience`). The rules live in
//! `sim-campaign::encounter`.

use serde::{Deserialize, Serialize};

use crate::common::{Effect, EffectKind, Sources};
use crate::entities::event::{Condition, EventEffect, EventSeason};
use crate::entities::province::Terrain;
use crate::ids::{EncounterId, ProvinceId, UnitTypeId};

/// Effect kinds an `army_effects` entry may use.
pub const ARMY_EFFECT_KINDS: [EffectKind; 3] = [
    EffectKind::ArmyMorale,
    EffectKind::Supply,
    EffectKind::ArmyExperience,
];

/// War requirement on the controller of the spawn province.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnWar {
    /// Peace or war alike.
    #[default]
    Any,
    /// The controller is at war with someone (rebels aside).
    War,
    /// The controller is at peace with everyone (rebels aside).
    Peace,
}

/// Where and when an encounter may appear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterSpawn {
    /// First year the encounter may appear (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_year: Option<i32>,
    /// Last year the encounter may appear (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_year: Option<i32>,
    /// Seasons it may appear in (empty: all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seasons: Vec<EventSeason>,
    /// Terrains of the province (empty: all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrains: Vec<Terrain>,
    /// Regions of the province (`Province::region`; empty: all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<String>,
    /// Explicit provinces (empty: all); combined with `regions` as a union.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provinces: Vec<ProvinceId>,
    /// War state of the province's controller.
    #[serde(default)]
    pub war: SpawnWar,
    /// Chronicle conditions, evaluated for the province and its controller.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
    /// Relative weight among the eligible encounters.
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// Seasons the site stays on the map, drawn in `[min, max]`.
    pub lifetime: [u32; 2],
}

fn default_weight() -> u32 {
    10
}

/// A number of regiments of one unit type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterUnits {
    pub unit_type: UnitTypeId,
    pub count: u32,
}

/// Consequences of a battle's result (`on_win` / `on_loss`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterResult {
    /// French line told in the chronicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EventEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub army_effects: Vec<Effect>,
}

/// Special outcome of an option: a battle against a neutral troop, or
/// regiments joining the army.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EncounterOutcome {
    /// A `fac_rebels` troop is raised beside the army and fought (3D or auto).
    Battle {
        units: Vec<EncounterUnits>,
        #[serde(default)]
        on_win: EncounterResult,
        #[serde(default)]
        on_loss: EncounterResult,
    },
    /// Regiments join the army, within the army size limit.
    Join { units: Vec<EncounterUnits> },
}

/// One choice of an encounter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterOption {
    /// French label.
    pub label: String,
    /// All must hold for the deciding faction (e.g. `treasury_above`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EventEffect>,
    /// Effects on the army that met the encounter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub army_effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<EncounterOutcome>,
    /// Relative weight of the option for the AI (weighted draw).
    #[serde(default = "default_ai_weight")]
    pub ai_weight: u32,
    /// Taken when the player leaves the encounter unanswered (at most one;
    /// the first option otherwise).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
}

fn default_ai_weight() -> u32 {
    1
}

/// A map encounter (`data/encounters/enc_*.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub id: EncounterId,
    /// French title.
    pub title: String,
    /// French text in a period tone.
    pub text: String,
    pub spawn: EncounterSpawn,
    pub options: Vec<EncounterOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl Encounter {
    /// Index of the default option: the one flagged `default`, else 0.
    pub fn default_option(&self) -> usize {
        self.options.iter().position(|o| o.default).unwrap_or(0)
    }
}

/// Tuning of the encounters (`data/rules/encounters.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterRules {
    /// Sites on the map at most.
    pub max_active: u32,
    /// New sites per season at most.
    pub spawn_per_season: u32,
    /// Distance (km) from a site at which a march ending triggers it.
    pub trigger_radius_km: f64,
    /// Minimal distance (km) between two sites.
    pub min_site_distance_km: f64,
    /// Minimal distance (km) between a site and any settlement.
    pub min_settlement_distance_km: f64,
    /// Radius (km) around a settlement of the province where a site is drawn.
    pub spawn_radius_km: f64,
    /// Cells tried before giving up a spawn.
    pub spawn_attempts: u32,
    /// Chance (‰) that a new site is drawn among the provinces the player
    /// controls or borders, when any is eligible.
    pub near_player_permille: u32,
    /// Regiments an army may hold at most (`join` outcomes stop there).
    pub max_army_units: u32,
}

impl Default for EncounterRules {
    fn default() -> Self {
        EncounterRules {
            max_active: 6,
            spawn_per_season: 2,
            trigger_radius_km: 4.0,
            min_site_distance_km: 60.0,
            min_settlement_distance_km: 3.0,
            spawn_radius_km: 25.0,
            spawn_attempts: 24,
            near_player_permille: 500,
            max_army_units: 20,
        }
    }
}
