//! Diplomacy rules, schema
//! `data/schemas/diplomacy_rules.schema.json` (`data/rules/diplomacy.json`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A motive of opinion modifier whose running total is capped (RS-C).
///
/// The simulation maps each motive to the French reason it writes on the
/// modifier; an unknown motive in the data fails to load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpinionMotive {
    /// « Mariage entre nos maisons » (a marriage between the ruling houses).
    Marriage,
    /// « Ambassade d'un héraut » (a herald's embassy or truce mission).
    HeraldEmbassy,
    /// « Présents diplomatiques » (a gift of money).
    Gift,
    /// « Traité signé » (a negotiated treaty).
    Treaty,
}

/// One band of the qualitative attitude label (WH `diploa`): an attitude of
/// at least `min` reads `label`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttitudeBand {
    pub min: i32,
    pub label: String,
}

/// Withdrawing a military access: the opinion hit the holder suffers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeAccessRules {
    /// Opinion modifier the guest holds about the host (negative).
    pub attitude: i32,
    /// Turns it lasts.
    pub duration: u32,
}

/// Forecast of an ally's answer to a call to arms (war preview).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallForecastRules {
    /// An ally not crippled whose attitude towards the defender is at least
    /// this (and not above 0) hesitates instead of refusing.
    pub hesitate_floor: i32,
}

/// Severity of a diplomatic event (M6, ADR 0721), after WH MAJOR / MODERATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Major,
    Moderate,
}

/// A named diplomatic event with a window (M6, ADR 0721): breaking a word
/// writes this opinion modifier on every lord and costs the author prestige.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiplomacyEvent {
    pub id: String,
    pub severity: EventSeverity,
    /// Reason text written on the opinion modifier.
    pub reason_fr: String,
    /// Opinion change every other lord holds about the author (negative).
    pub opinion: i32,
    /// Turns (seasons) the opinion change lasts.
    pub duration: u32,
    /// Prestige change of the author's ruler (negative).
    #[serde(default)]
    pub prestige: i32,
    /// Extra prestige lost when the author's chivalric order stands (the
    /// knight forsworn); negative or zero.
    #[serde(default)]
    pub chivalry_prestige: i32,
}

/// Seasons since the signing within which breaking a truce is major.
fn default_window() -> u32 {
    6
}

/// Contents of `data/rules/diplomacy.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiplomacyRules {
    /// Largest total (in absolute value) of the running opinion modifiers of
    /// one motive held by a faction about another; a new modifier beyond it
    /// only renews the running ones. Motives absent here are not capped.
    #[serde(default)]
    pub opinion_caps: BTreeMap<OpinionMotive, i32>,
    /// Qualitative attitude labels, any order (the highest `min` not above
    /// the attitude wins; below all, the lowest band).
    #[serde(default)]
    pub attitude_bands: Vec<AttitudeBand>,
    #[serde(default)]
    pub revoke_access: RevokeAccessRules,
    #[serde(default)]
    pub call_forecast: CallForecastRules,
    /// Seasons since a truce was signed within which breaking it is the
    /// recent-truce event (M6).
    #[serde(default = "default_window")]
    pub truce_window: u32,
    /// Windowed diplomatic events (M6, ADR 0721).
    #[serde(default)]
    pub events: Vec<DiplomacyEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl DiplomacyRules {
    /// The event `id`, if the data defines it.
    pub fn event(&self, id: &str) -> Option<&DiplomacyEvent> {
        self.events.iter().find(|e| e.id == id)
    }

    /// Cap of `motive`, if any.
    pub fn opinion_cap(&self, motive: OpinionMotive) -> Option<i32> {
        self.opinion_caps.get(&motive).copied()
    }

    /// Qualitative label of an attitude (empty without bands in the data).
    pub fn attitude_band(&self, attitude: i32) -> &str {
        self.attitude_bands
            .iter()
            .filter(|b| b.min <= attitude)
            .max_by_key(|b| b.min)
            .or_else(|| self.attitude_bands.iter().min_by_key(|b| b.min))
            .map_or("", |b| b.label.as_str())
    }
}

crate::bundled_rules!(DiplomacyRules, "rules/diplomacy.json", default);
