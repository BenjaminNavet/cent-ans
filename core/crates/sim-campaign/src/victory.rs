//! Campaign objectives and outcome (M10): every playable faction has
//! historical objectives in `data/factions/*.json` (`victory`). Meeting them
//! all before `end_year` wins; losing every province loses; reaching the end
//! year closes the campaign with a score. The player may keep playing.

use data_model::{FactionId, GameData, Objective, ObjectiveCondition, ObjectiveScope};
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

/// Length of the campaign the player chose (ADR 0332): a short one counts the
/// objectives of every scope up to the short end year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VictoryLength {
    Short,
    #[default]
    Long,
}

/// Objectives, end year and holding time that apply to a faction.
struct VictorySet<'a> {
    objectives: Vec<&'a Objective>,
    end_year: i32,
    hold_turns: u32,
    /// The set is the generic one (no `victory` block): it only wins from
    /// `generic_victory_min_year`.
    generic: bool,
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
    /// The objectives, end year and holding time of `faction` for the chosen
    /// campaign length: its own `victory` block, else the generic objectives
    /// of `feudal_rules.victory` (ADR 0332).
    fn victory_set<'a>(&self, data: &'a GameData, faction: &FactionId) -> VictorySet<'a> {
        let rules = &data.feudal_rules.victory;
        let short = self.victory_length == VictoryLength::Short;
        let own = data.factions.get(faction).and_then(|f| f.victory.as_ref());
        let (all, end_year, hold, generic): (&[Objective], i32, u32, bool) = match own {
            Some(v) => (
                &v.objectives,
                if short {
                    v.short_end_year
                        .unwrap_or(rules.short_end_year)
                        .min(v.end_year)
                } else {
                    v.end_year
                },
                v.hold_turns.unwrap_or(1),
                false,
            ),
            None => (
                &rules.generic_objectives,
                if short {
                    rules.short_end_year
                } else {
                    data.feudal_rules.default_end_year
                },
                rules.generic_hold_turns,
                true,
            ),
        };
        let mut objectives: Vec<&Objective> = all
            .iter()
            .filter(|o| !short || o.scope == ObjectiveScope::Always)
            .collect();
        if objectives.is_empty() {
            objectives = all.iter().collect();
        }
        VictorySet {
            objectives,
            end_year,
            hold_turns: if short {
                hold.min(rules.short_hold_turns)
            } else {
                hold
            }
            .max(1),
            generic,
        }
    }

    /// Objectives of `faction` with their progress, for the chosen campaign
    /// length (empty if the data give it none).
    pub fn objectives(&self, data: &GameData, faction: &FactionId) -> Vec<ObjectiveStatus> {
        self.victory_set(data, faction)
            .objectives
            .into_iter()
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
        let controls = |p: &data_model::ProvinceId| self.controls_province(faction, p);
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
                    .filter(|p| self.controls_province(foreign, p))
                    .count();
                let progress = match held {
                    0 => "aucune province occupée".to_owned(),
                    1 => "1 province encore occupée".to_owned(),
                    n => format!("{n} provinces encore occupées"),
                };
                (held == 0, progress)
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
            ObjectiveCondition::ProvinceCount { count } => {
                let held = self.controlled_provinces(faction).count() as u32;
                (held >= *count, format!("{held}/{count} provinces"))
            }
            ObjectiveCondition::ProvinceGrowth { extra } => {
                let held = self.controlled_provinces(faction).count() as u32;
                let start = if self.victory_start_provinces == 0 {
                    held
                } else {
                    self.victory_start_provinces
                };
                let gained = held.saturating_sub(start);
                (
                    gained >= *extra,
                    format!("{gained}/{extra} provinces gagnées"),
                )
            }
            ObjectiveCondition::Prestige { min } => {
                let prestige = self.victory_prestige(faction);
                (
                    prestige >= i64::from(*min),
                    format!("prestige {prestige}/{min}"),
                )
            }
            ObjectiveCondition::Treasury { min } => {
                let treasury = self.factions.get(faction).map_or(0, |f| f.treasury);
                (treasury >= *min, format!("{treasury}/{min} livres"))
            }
            ObjectiveCondition::HoldTitle { title } => {
                let held = crate::feudal::holder_of(self, title) == Some(faction);
                (
                    held,
                    if held {
                        "titre détenu".to_owned()
                    } else {
                        "titre à obtenir".to_owned()
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
                        "soumission obtenue".to_owned()
                    } else {
                        "soumission à obtenir".to_owned()
                    },
                )
            }
        }
    }

    /// Seasons all objectives of `faction` must hold in a row (chosen length).
    pub fn victory_hold_turns(&self, data: &GameData, faction: &FactionId) -> u32 {
        self.victory_set(data, faction).hold_turns
    }

    /// Chooses a short or long campaign (ADR 0332).
    pub fn set_victory_length(&mut self, length: VictoryLength) {
        self.victory_length = length;
        self.victory_streak = 0;
    }

    /// Prestige of the faction's ruler (0 without one).
    fn victory_prestige(&self, faction: &FactionId) -> i64 {
        self.factions
            .get(faction)
            .and_then(|f| f.ruler.as_ref())
            .and_then(|r| self.characters.get(r))
            .map_or(0, |c| i64::from(c.prestige))
    }

    /// Campaign score of `faction`: land, objectives, prestige, treasury.
    pub fn campaign_score(&self, data: &GameData, faction: &FactionId) -> i64 {
        let provinces = self.controlled_provinces(faction).count() as i64;
        let objectives = self
            .objectives(data, faction)
            .iter()
            .filter(|o| o.done)
            .count() as i64;
        let prestige = self.victory_prestige(faction);
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
    let name = data.faction_name(&player);
    // Lands as in `characters::resolve_faction_deaths`: any settlement held
    // (a castle or a town is enough, not only a province's city).
    let alive = state.factions.get(&player).is_some_and(|f| f.alive)
        && state.settlements.values().any(|s| s.controller == player);
    if state.victory_start_provinces == 0 {
        state.victory_start_provinces = (state.controlled_provinces(&player).count() as u32).max(1);
    }
    let objectives = state.objectives(data, &player);
    let set = state.victory_set(data, &player);
    let (end_year, hold) = (set.end_year, set.hold_turns);
    // ADR 0332: generic objectives win only from the generic minimum year.
    let too_early = set.generic && state.year < data.feudal_rules.generic_victory_min_year;
    let all_done = !objectives.is_empty() && objectives.iter().all(|o| o.done) && !too_early;
    if all_done {
        state.victory_streak += 1;
        if state.victory_streak == 1 && hold > 1 {
            events.push(
                GameEvent::new(
                    EventKind::Diplomacy,
                    format!(
                        "Tous les objectifs de {name} sont remplis : tenez-les {hold} saisons pour l'emporter."
                    ),
                )
                .faction(&player),
            );
        }
    } else {
        if state.victory_streak > 0 && hold > 1 {
            events.push(
                GameEvent::new(
                    EventKind::Diplomacy,
                    format!("{name} ne remplit plus tous ses objectifs : la victoire s'éloigne."),
                )
                .faction(&player),
            );
        }
        state.victory_streak = 0;
    }
    // FE (F3, spec § 4.8): objectives of the primary title, independence,
    // first vassal of the realm, crown of the suzerain.
    let feudal = crate::feudal::generic_victory(state, data, &player);
    let (kind, text) = if !alive {
        (
            OutcomeKind::Defeat,
            format!("Défaite : {name} a perdu toutes ses terres."),
        )
    } else if all_done && state.victory_streak >= hold {
        (
            OutcomeKind::Victory,
            format!("Victoire ! {name} a accompli tous ses objectifs historiques."),
        )
    } else if let Some(path) = feudal {
        (
            OutcomeKind::Victory,
            format!("Victoire ! {name} {}.", path.text_fr()),
        )
    } else if state.year > end_year {
        let done = objectives.iter().filter(|o| o.done).count();
        (
            OutcomeKind::Ended,
            format!(
                "Fin de la campagne : {name} a accompli {done} {} sur {}.",
                if done <= 1 { "objectif" } else { "objectifs" },
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
