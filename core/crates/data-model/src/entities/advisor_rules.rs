//! The counsellor's season advice, mirroring `data/schemas/advisor_rules.schema.json`
//! (`data/rules/advisor.json`, GAMEPLAY2, ADR 0789): when the counsellor speaks (an opportunity,
//! a danger, the end of the year), how often, and what he says (French texts in data), plus the
//! news line announcing a change of posture of a neighbouring AI (doctrine, ADR 0782).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What makes an advice hold, read from the state by `sim_campaign::advisor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub enum AdviceTrigger {
    /// At least `min_sites` settlements without a works under way, with one affordable option.
    IdleSites { min_sites: u32 },
    /// The treasury net of debts is above `seasons` seasons of income (and `min` livres).
    IdleHoard { seasons: u32, min: i64 },
    /// The season's balance is negative and the treasury covers at most `max_seasons` of it.
    RunwayShort { max_seasons: u32 },
    /// The last season of the year: a word on the act in progress.
    YearEnd,
    /// The last season of the year and the act score is below `below` (an idle house).
    ActScoreLow { below: u32 },
    /// ECO-R1d: the realm is at war, a house with assemblies has no subsidy running, no open session
    /// and one of its assemblies can be convened (the cash will run short without consent).
    WarWithoutSubsidy,
    /// ECO-R1d: a promise made to an assembly falls due (a truce to sign) within `within_seasons`.
    PromiseDue { within_seasons: u32 },
}

/// One advice of the counsellor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Advice {
    pub id: String,
    /// `opportunity`, `danger` or `season_end` (danger first when several hold).
    pub kind: String,
    pub trigger: AdviceTrigger,
    /// Seasons before the same advice may be said again.
    pub cooldown_seasons: u32,
    pub title: String,
    /// Placeholders: `{sites}`, `{treasury}`, `{levy}`, `{province}`, `{seasons}`, `{act}`,
    /// `{score}`, `{objective}`, `{assembly}`, `{promise}`, `{deadline}`.
    pub text: String,
}

/// The news line of a change of posture of a neighbouring or hostile house.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PostureNews {
    /// Placeholders `{faction}`, `{doctrine}` (label of the catalogue), `{intent}`.
    pub text: String,
    /// Phrase by doctrine id (what the house now seems to prepare).
    pub intents: BTreeMap<String, String>,
    /// Phrase when the doctrine has no entry in `intents`.
    pub default_intent: String,
}

/// Contents of `data/rules/advisor.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AdvisorRules {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Advices said in one season at most.
    pub max_per_season: u32,
    /// Seasons without any advice at the start of a campaign (the guide speaks first).
    pub quiet_seasons: u32,
    pub advices: Vec<Advice>,
    pub posture: PostureNews,
}

impl AdvisorRules {
    /// Whether the counsellor is configured.
    pub fn enabled(&self) -> bool {
        !self.advices.is_empty()
    }
}
