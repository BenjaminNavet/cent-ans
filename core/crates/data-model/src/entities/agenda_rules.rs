//! Attention budget of the campaign (XP-AUDIENCE, R1), mirroring
//! `data/schemas/agenda_rules.schema.json` (`data/rules/agenda.json`): how many matters the
//! Audience may push at the player each season, how they are ranked, and how many strong
//! chronicle decisions a season may open. The file is optional: without it the defaults below
//! (the shipped caps, no source table) apply.

use serde::{Deserialize, Serialize};

/// Where an agenda item lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgendaRank {
    /// Pushed at the player (the Audience), `max_affairs` at most.
    Affair,
    /// Listed under « À surveiller », `max_watch` at most.
    Watch,
    /// Only counted in the folded news.
    News,
}

/// How one kind of source (an alert kind, an offer, an ambition choice...) is ranked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgendaSource {
    /// Alert kind (`siege`, `enemy_army`, `debt`...) or one of the agenda's own kinds
    /// (`mission_offer`, `ambition_choice`, `treasury_swing`, `chronicle_incident`).
    pub kind: String,
    pub rank: AgendaRank,
    /// Sort key among the items of a rank: the smaller comes first.
    pub order: u32,
    /// Council office whose holder speaks (`chancellor`, `constable`, `treasurer`...); empty:
    /// the chronicler.
    #[serde(default)]
    pub office: String,
    /// Always an Affair, whatever the cap (a blocking decision, a mission offer).
    #[serde(default)]
    pub guaranteed: bool,
    /// Several items of the kind collapse into one line with a count.
    #[serde(default)]
    pub grouped: bool,
    /// Interface target of the main button (`open_chronicle`, `open_offer`, `center_province`...).
    #[serde(default)]
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AgendaRules {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Affairs pushed at the player per season, at most.
    pub max_affairs: u32,
    /// Items of « À surveiller », at most.
    pub max_watch: u32,
    /// Strong (non-crisis, blocking) chronicle decisions a season may open for the player.
    pub max_strong_decisions_per_season: u32,
    /// Events that are crises: they open at once, whatever the cap.
    pub crisis_events: Vec<String>,
    /// The treasurer speaks when the coming season's net balance falls below this (negative).
    pub treasury_affair_below: i64,
    pub sources: Vec<AgendaSource>,
}

impl Default for AgendaRules {
    fn default() -> Self {
        AgendaRules {
            description: String::new(),
            max_affairs: 3,
            max_watch: 5,
            max_strong_decisions_per_season: 1,
            crisis_events: Vec::new(),
            treasury_affair_below: -1,
            sources: Vec::new(),
        }
    }
}

impl AgendaRules {
    /// The source row of `kind`.
    pub fn source(&self, kind: &str) -> Option<&AgendaSource> {
        self.sources.iter().find(|s| s.kind == kind)
    }

    /// Whether `event` is a crisis (opens at once).
    pub fn is_crisis(&self, event: &str) -> bool {
        self.crisis_events.iter().any(|e| e == event)
    }
}
