//! Journal of causes (ADR 0748): « why did I lose / win? ».
//!
//! Every major event of the campaign (lost battle, place taken, bankruptcy, revolt, war declared,
//! alliance broken, succession, end of the game) can carry a [`Cause`]: a kind, the faction hit
//! and the faction acting, a severity and ranked numeric [`Factor`]s. Causes hold keys and
//! numbers only; the French sentences live in `data/ui/causes.fr.json`, read by `game-api`.
//!
//! Producers attach a cause to the [`GameEvent`] they already emit (`GameEvent::cause`);
//! [`harvest`] moves them into the bounded [`CauseJournal`] at the end of the turn and stamps the
//! event with `cause_id`, so a notification can point to its cause. The journal only reads the
//! state at the time of the event: it changes no rule and uses no randomness.

use std::collections::VecDeque;

use data_model::{FactionId, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::events::GameEvent;
use crate::state::{CampaignState, Season};

/// Records kept in the journal (the oldest leave first).
pub const MAX_RECORDS: usize = 300;

/// Ceiling of a factor weight and of a severity.
pub const MAX_WEIGHT: u8 = 100;

/// What kind of event a cause explains (stable snake_case names, keys of `causes.fr.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CauseKind {
    /// A battle: `subject` is the loser, `actor` the winner.
    Battle,
    /// A place changes hands: `subject` the previous holder, `actor` the new one.
    PlaceTaken,
    /// The treasury is empty at the end of a season.
    Bankruptcy,
    /// An uprising: the province is shaken (`subject` is its lord).
    Revolt,
    /// The tension of a province crosses a warning level.
    UnrestWarning,
    /// A war is declared: `subject` is the target, `actor` the aggressor.
    WarDeclared,
    /// An alliance is broken: `subject` is the abandoned ally, `actor` who breaks it.
    AllianceBroken,
    /// A ruler dies and the crown passes (or is lost).
    Succession,
    /// The crown stops paying its bankers (royal default, ADR 0777).
    RoyalDefault,
    /// An extraordinary tax is raised (`subject` is the taxing faction).
    ExtraordinaryTax,
    /// The money is debased (`subject` is the minting faction).
    Debasement,
    /// The player's campaign is won.
    GameWon,
    /// The player's campaign is lost.
    GameLost,
}

impl CauseKind {
    /// Stable name (the serde name), key of the texts.
    pub fn as_str(self) -> &'static str {
        match self {
            CauseKind::Battle => "battle",
            CauseKind::PlaceTaken => "place_taken",
            CauseKind::Bankruptcy => "bankruptcy",
            CauseKind::Revolt => "revolt",
            CauseKind::UnrestWarning => "unrest_warning",
            CauseKind::WarDeclared => "war_declared",
            CauseKind::AllianceBroken => "alliance_broken",
            CauseKind::Succession => "succession",
            CauseKind::RoyalDefault => "royal_default",
            CauseKind::ExtraordinaryTax => "extraordinary_tax",
            CauseKind::Debasement => "debasement",
            CauseKind::GameWon => "game_won",
            CauseKind::GameLost => "game_lost",
        }
    }
}

/// One ranked contributing factor: a key of the texts, an integer value (a percentage or a
/// number of points, the unit being in the text) and a weight (0-100) that ranks it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Factor {
    pub key: String,
    pub value: i64,
    pub weight: u8,
}

/// The named reason of an AI decision (doctrine or objective), a key of the texts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiReason {
    pub key: String,
}

/// Why something happened, structured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cause {
    pub kind: CauseKind,
    /// The faction hit (the loser, the target, the ruler of the shaken province).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<FactionId>,
    /// The faction acting (the winner, the aggressor, the new holder).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    /// Weight of the event for the ranking of the « main causes » (0-100).
    pub severity: u8,
    /// Factors, heaviest first.
    pub factors: Vec<Factor>,
    /// The AI's named reason when the decision is the AI's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<AiReason>,
}

impl Cause {
    pub fn new(kind: CauseKind, severity: u8) -> Self {
        Cause {
            kind,
            subject: None,
            actor: None,
            province: None,
            severity: severity.min(MAX_WEIGHT),
            factors: Vec::new(),
            reason: None,
        }
    }

    pub fn subject(mut self, faction: &FactionId) -> Self {
        self.subject = Some(faction.clone());
        self
    }

    pub fn actor(mut self, faction: &FactionId) -> Self {
        self.actor = Some(faction.clone());
        self
    }

    pub fn province(mut self, province: &ProvinceId) -> Self {
        self.province = Some(province.clone());
        self
    }

    /// Adds a factor; a zero weight is dropped, the rest is ranked by [`Cause::ranked`].
    pub fn factor(mut self, key: &str, value: i64, weight: i64) -> Self {
        let weight = weight.clamp(0, i64::from(MAX_WEIGHT)) as u8;
        if weight > 0 {
            self.factors.push(Factor {
                key: key.to_owned(),
                value,
                weight,
            });
        }
        self
    }

    pub fn reason(mut self, key: &str) -> Self {
        self.reason = Some(AiReason {
            key: key.to_owned(),
        });
        self
    }

    /// Sorts the factors, heaviest first (the key breaks ties: deterministic).
    pub fn ranked(mut self) -> Self {
        self.factors
            .sort_by(|a, b| b.weight.cmp(&a.weight).then_with(|| a.key.cmp(&b.key)));
        self
    }

    /// Whether the cause concerns `faction` (hit or acting).
    pub fn concerns(&self, faction: &FactionId) -> bool {
        self.subject.as_ref() == Some(faction) || self.actor.as_ref() == Some(faction)
    }
}

/// A cause stamped with its identifier and date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CauseRecord {
    pub id: u32,
    pub turn: u32,
    pub season: Season,
    pub year: i32,
    pub cause: Cause,
}

/// The bounded journal kept in the campaign state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CauseJournal {
    #[serde(default)]
    next_id: u32,
    #[serde(default)]
    records: VecDeque<CauseRecord>,
}

impl CauseJournal {
    /// Records, oldest first.
    pub fn records(&self) -> impl DoubleEndedIterator<Item = &CauseRecord> {
        self.records.iter()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&CauseRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    fn push(&mut self, turn: u32, season: Season, year: i32, cause: Cause) -> u32 {
        self.next_id += 1;
        let id = self.next_id;
        self.records.push_back(CauseRecord {
            id,
            turn,
            season,
            year,
            cause: cause.ranked(),
        });
        while self.records.len() > MAX_RECORDS {
            self.records.pop_front();
        }
        id
    }
}

impl CampaignState {
    /// Records `cause` now (date of the state) and returns its identifier.
    pub fn note_cause(&mut self, cause: Cause) -> u32 {
        let (turn, season, year) = (self.turn, self.season, self.year);
        self.causes.push(turn, season, year, cause)
    }

    /// The journal of causes.
    pub fn cause_journal(&self) -> &CauseJournal {
        &self.causes
    }
}

/// Moves the causes carried by `events` into the journal and stamps the events with their
/// `cause_id` (end of turn).
pub(crate) fn harvest(state: &mut CampaignState, events: &mut [GameEvent]) {
    for event in events.iter_mut() {
        if let Some(cause) = event.cause.take() {
            event.cause_id = Some(state.note_cause(*cause));
        }
    }
}

/// Migration v9 -> v10: the journal of causes appears, empty.
pub(crate) fn migrate_v9_to_v10(value: &mut serde_json::Value) {
    if let Some(map) = value.as_object_mut() {
        map.entry("causes")
            .or_insert_with(|| serde_json::json!({"next_id": 0, "records": []}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(id: &str) -> FactionId {
        FactionId::new(id).unwrap()
    }

    #[test]
    fn factors_are_ranked_heaviest_first_and_zero_weights_dropped() {
        let cause = Cause::new(CauseKind::Battle, 50)
            .factor("b", 1, 20)
            .factor("a", 2, 20)
            .factor("c", 3, 80)
            .factor("zero", 4, 0)
            .ranked();
        let keys: Vec<&str> = cause.factors.iter().map(|x| x.key.as_str()).collect();
        assert_eq!(keys, ["c", "a", "b"]);
    }

    #[test]
    fn weights_and_severity_are_clamped() {
        let cause = Cause::new(CauseKind::Revolt, 250).factor("x", 1, 999);
        assert_eq!(cause.severity, MAX_WEIGHT);
        assert_eq!(cause.factors[0].weight, MAX_WEIGHT);
    }

    #[test]
    fn journal_is_bounded_and_ids_keep_growing() {
        let mut journal = CauseJournal::default();
        for _ in 0..MAX_RECORDS + 20 {
            journal.push(
                1,
                Season::Spring,
                1337,
                Cause::new(CauseKind::Bankruptcy, 10),
            );
        }
        assert_eq!(journal.len(), MAX_RECORDS);
        assert_eq!(journal.records().next().unwrap().id, 21);
        assert!(journal.get(1).is_none());
        assert!(journal.get(MAX_RECORDS as u32 + 20).is_some());
    }

    #[test]
    fn concerns_matches_subject_or_actor() {
        let cause = Cause::new(CauseKind::WarDeclared, 40)
            .subject(&f("fac_france"))
            .actor(&f("fac_england"));
        assert!(cause.concerns(&f("fac_france")));
        assert!(cause.concerns(&f("fac_england")));
        assert!(!cause.concerns(&f("fac_burgundy")));
    }

    #[test]
    fn migration_adds_an_empty_journal_and_keeps_an_existing_one() {
        let mut v = serde_json::json!({"state_version": 9});
        migrate_v9_to_v10(&mut v);
        assert_eq!(v["causes"]["records"], serde_json::json!([]));
        let mut w = serde_json::json!({"causes": {"next_id": 3, "records": []}});
        migrate_v9_to_v10(&mut w);
        assert_eq!(w["causes"]["next_id"], 3);
    }
}
