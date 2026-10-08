//! Diplomacy rules (lot RS-C), schema
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

/// Contents of `data/rules/diplomacy.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiplomacyRules {
    /// Largest total (in absolute value) of the running opinion modifiers of
    /// one motive held by a faction about another; a new modifier beyond it
    /// only renews the running ones. Motives absent here are not capped.
    #[serde(default)]
    pub opinion_caps: BTreeMap<OpinionMotive, i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl DiplomacyRules {
    /// Cap of `motive`, if any.
    pub fn opinion_cap(&self, motive: OpinionMotive) -> Option<i32> {
        self.opinion_caps.get(&motive).copied()
    }
}

crate::bundled_rules!(DiplomacyRules, "rules/diplomacy.json", default);
