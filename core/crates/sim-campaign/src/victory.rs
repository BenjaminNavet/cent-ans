//! Campaign objectives and outcome (M10): every playable faction has
//! historical objectives in `data/factions/*.json` (`victory`). Meeting them
//! all before `end_year` wins; losing every province loses; reaching the end
//! year closes the campaign with a score. The player may keep playing.

use data_model::{FactionId, GameData, ObjectiveCondition};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    Victory,
    Defeat,
    /// The end year was reached without victory.
    Ended,
}

/// How the campaign ended for the player (kept once set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub kind: OutcomeKind,
    pub turn: u32,
    pub text_fr: String,
    pub score: i64,
}

/// Progress of one objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveStatus {
    pub id: String,
    pub title: String,
    pub description: String,
    pub done: bool,
    /// Short French progress text ("3/7 provinces").
    pub progress: String,
}

impl CampaignState {
    /// Objectives of `faction` with their progress (empty if it has none).
    pub fn objectives(&self, data: &GameData, faction: &FactionId) -> Vec<ObjectiveStatus> {
        let Some(victory) = data.factions.get(faction).and_then(|f| f.victory.as_ref()) else {
            return Vec::new();
        };
        victory
            .objectives
            .iter()
            .map(|objective| {
                let (done, progress) = self.condition_status(faction, &objective.condition);
                ObjectiveStatus {
                    id: objective.id.clone(),
                    title: objective.title.clone(),
                    description: objective.description.clone(),
                    done,
                    progress,
                }
            })
            .collect()
    }

    fn condition_status(
        &self,
        faction: &FactionId,
        condition: &ObjectiveCondition,
    ) -> (bool, String) {
        let controls = |p: &data_model::ProvinceId| {
            self.provinces
                .get(p)
                .is_some_and(|s| &s.controller == faction)
        };
        match condition {
            ObjectiveCondition::ControlAll { provinces } => {
                let held = provinces.iter().filter(|p| controls(p)).count();
                (
                    held == provinces.len(),
                    format!("{held}/{} provinces", provinces.len()),
                )
            }
            ObjectiveCondition::ControlCount { provinces, count } => {
                let held = provinces.iter().filter(|p| controls(p)).count() as u32;
                (held >= *count, format!("{held}/{count} provinces"))
            }
            ObjectiveCondition::NoForeignControl {
                faction: foreign,
                provinces,
            } => {
                let held = provinces
                    .iter()
                    .filter(|p| {
                        self.provinces
                            .get(*p)
                            .is_some_and(|s| &s.controller == foreign)
                    })
                    .count();
                (held == 0, format!("{held} province(s) encore occupée(s)"))
            }
            ObjectiveCondition::Independent => {
                let free = self
                    .factions
                    .get(faction)
                    .is_some_and(|f| f.suzerain.is_none());
                (
                    free,
                    if free {
                        "indépendant".to_owned()
                    } else {
                        "vassal".to_owned()
                    },
                )
            }
            ObjectiveCondition::Subjugate { faction: target } => {
                let done = self
                    .factions
                    .get(target)
                    .is_none_or(|f| !f.alive || f.suzerain.as_ref() == Some(faction));
                (
                    done,
                    if done {
                        "soumis".to_owned()
                    } else {
                        "insoumis".to_owned()
                    },
                )
            }
        }
    }

    /// Campaign score of `faction`: land, objectives, prestige, treasury.
    pub fn campaign_score(&self, data: &GameData, faction: &FactionId) -> i64 {
        let provinces = self
            .provinces
            .values()
            .filter(|p| &p.controller == faction)
            .count() as i64;
        let objectives = self
            .objectives(data, faction)
            .iter()
            .filter(|o| o.done)
            .count() as i64;
        let prestige = self
            .factions
            .get(faction)
            .and_then(|f| f.ruler.as_ref())
            .and_then(|r| self.characters.get(r))
            .map_or(0, |c| i64::from(c.prestige));
        let treasury = self.factions.get(faction).map_or(0, |f| f.treasury.max(0));
        provinces * 10 + objectives * 100 + prestige + treasury / 2000
    }
}

/// Phase (last): sets the player's outcome once.
pub(crate) fn resolve_victory(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    if state.outcome.is_some() {
        return;
    }
    let player = state.player_faction.clone();
    let name = crate::diplomacy::faction_name(data, &player);
    let alive = state.factions.get(&player).is_some_and(|f| f.alive)
        && state.provinces.values().any(|p| p.controller == player);
    let objectives = state.objectives(data, &player);
    let end_year = data
        .factions
        .get(&player)
        .and_then(|f| f.victory.as_ref())
        .map(|v| v.end_year);
    let (kind, text) = if !alive {
        (
            OutcomeKind::Defeat,
            format!("Défaite : {name} a perdu toutes ses terres."),
        )
    } else if !objectives.is_empty() && objectives.iter().all(|o| o.done) {
        (
            OutcomeKind::Victory,
            format!("Victoire ! {name} a accompli tous ses objectifs historiques."),
        )
    } else if end_year.is_some_and(|y| state.year > y) {
        let done = objectives.iter().filter(|o| o.done).count();
        (
            OutcomeKind::Ended,
            format!(
                "Fin de la campagne : {done} objectif(s) sur {} accompli(s) par {name}.",
                objectives.len()
            ),
        )
    } else {
        return;
    };
    let score = state.campaign_score(data, &player);
    let event_kind = match kind {
        OutcomeKind::Victory => EventKind::Victory,
        OutcomeKind::Defeat => EventKind::Defeat,
        OutcomeKind::Ended => EventKind::CampaignEnded,
    };
    events.push(GameEvent::new(event_kind, format!("{text} Score : {score}.")).faction(&player));
    state.outcome = Some(Outcome {
        kind,
        turn: state.turn,
        text_fr: text,
        score,
    });
}
