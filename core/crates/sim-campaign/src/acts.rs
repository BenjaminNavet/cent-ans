//! The campaign arc in three acts (ADR 0784, 0785): « La querelle », « La reprise et la ruine »,
//! « Le sursaut ». Each act gives the house of the player objectives and pressures
//! (`data/rules/acts.json`); an act closes with the year, or earlier by a lasting peace or a
//! collapse; the bilan of the three acts is the reign score that decides the final victory,
//! while the extinction of the dynasty, an irreversible royal bankruptcy and the loss of the
//! capital without an heir end the campaign at once.
//!
//! Only the player's house is followed. Nothing here changes the economy or the AI: the
//! module reads the state and keeps its own bookkeeping ([`ActsState`]).

use data_model::{ActCondition, ActObjective, ActPressure, ActsRules, FactionId, GameData};
use serde::{Deserialize, Serialize};

use crate::causes::{CauseKind, CauseRecord};
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// Fraction of the weight an unmet objective still earns per unit of progress.
const PARTIAL_CREDIT: f64 = 0.25;
/// Causes kept in the bilan of an act.
const BILAN_CAUSES: usize = 3;

/// Result of one objective at a given date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveResult {
    pub id: String,
    pub met: bool,
    /// Progress, 0-100.
    pub percent: u32,
}

/// Result of one pressure at a given date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PressureResult {
    pub id: String,
    pub active: bool,
    pub points: i32,
}

/// The bilan of an act that is over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActReport {
    pub act_id: String,
    /// `year`, `peace` or `collapse`.
    pub ended_by: String,
    pub start_turn: u32,
    pub end_turn: u32,
    pub end_year: i32,
    /// Act score, 0-100.
    pub score: u32,
    /// Policy chosen for the act.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    pub objectives: Vec<ObjectiveResult>,
    pub pressures: Vec<PressureResult>,
    /// The main causes of the act (journal records, copied: the journal is bounded).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<CauseRecord>,
}

/// Why the campaign ends in defeat (besides losing every land).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ruin {
    /// The line of the ruling house is extinct.
    Dynasty,
    /// Repeated royal defaults, the treasury still in the red.
    Bankruptcy,
    /// The capital is lost and there is no heir.
    Capital,
}

impl Ruin {
    /// Key of the interface text (`texts` of `acts.json`).
    pub fn key(self) -> &'static str {
        match self {
            Ruin::Dynasty => "defeat_dynasty",
            Ruin::Bankruptcy => "defeat_bankruptcy",
            Ruin::Capital => "defeat_capital",
        }
    }
}

/// Bookkeeping of the arc, saved with the campaign (version 11).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ActsState {
    /// The arc has been entered (first season after loading).
    pub started: bool,
    /// Index in `acts_rules.acts` of the act in progress.
    pub current: usize,
    /// Every act is over (the campaign now awaits its final bilan).
    pub finished: bool,
    pub act_start_turn: u32,
    /// Provinces held when the act began.
    pub act_start_provinces: u32,
    /// Royal defaults of the house when the act began.
    pub act_start_defaults: u32,
    /// Policy chosen for the act in progress (`None`: still to choose).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    /// Consecutive seasons without a war.
    pub peace_streak: u32,
    /// House of the ruling line being followed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub house: String,
    /// Consecutive seasons with the capital lost and no heir.
    pub capital_lost_seasons: u32,
    /// Turn of the last transition, for the transition screen (`None`: none yet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_transition_turn: Option<u32>,
    /// The transition screen of `last_transition_turn` has been seen.
    pub transition_seen: bool,
    pub reports: Vec<ActReport>,
}

/// Errors of the player's act choices.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActError {
    #[error("the campaign has no act in progress")]
    NoAct,
    #[error("unknown policy {0}")]
    UnknownPolicy(String),
}

/// Live status of the act in progress (read-only view for the interface).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActStatus {
    pub index: usize,
    pub objectives: Vec<ObjectiveLine>,
    pub pressures: Vec<PressureLine>,
    pub score: u32,
}

/// An objective with its live progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveLine {
    pub objective: ActObjective,
    pub met: bool,
    pub percent: u32,
    pub progress: String,
    /// Multiplier of its weight from the chosen policy, in percent.
    pub factor_percent: u32,
}

/// A pressure with its live state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PressureLine {
    pub pressure: ActPressure,
    pub active: bool,
}

/// Migration v10 -> v11: the arc starts unentered.
pub(crate) fn migrate_v10_to_v11(value: &mut serde_json::Value) {
    if let Some(map) = value.as_object_mut() {
        map.entry("acts")
            .or_insert_with(|| serde_json::to_value(ActsState::default()).expect("serialisable"));
    }
}

fn government_key(data: &GameData, faction: &FactionId) -> &'static str {
    use data_model::Government::*;
    match data.factions.get(faction).map(|f| f.government) {
        Some(Kingdom) => "kingdom",
        Some(Duchy) => "duchy",
        Some(County) => "county",
        Some(Principality) => "principality",
        Some(Republic) => "republic",
        Some(Theocracy) => "theocracy",
        Some(Empire) => "empire",
        Some(Lordship) => "lordship",
        None => "none",
    }
}

fn ratio(held: usize, wanted: usize) -> f64 {
    if wanted == 0 {
        1.0
    } else {
        (held as f64 / wanted as f64).min(1.0)
    }
}

impl CampaignState {
    /// Ratio (0-1, 1 = met) of a condition for `faction` and a short French progress text.
    fn act_condition(&self, faction: &FactionId, condition: &ActCondition) -> (f64, String) {
        let controls = |p: &data_model::ProvinceId| self.controls_province(faction, p);
        let held_now = self.controlled_provinces(faction).count();
        match condition {
            ActCondition::ControlAll { provinces } => {
                let held = provinces.iter().filter(|p| controls(p)).count();
                (
                    ratio(held, provinces.len()),
                    format!("{held}/{} provinces", provinces.len()),
                )
            }
            ActCondition::ControlCount { provinces, count } => {
                let held = provinces.iter().filter(|p| controls(p)).count();
                (
                    ratio(held, *count as usize),
                    format!("{held}/{count} provinces"),
                )
            }
            ActCondition::NoForeignControl {
                faction: foreign,
                provinces,
            } => {
                let held = provinces
                    .iter()
                    .filter(|p| self.controls_province(foreign, p))
                    .count();
                let text = match held {
                    0 => "aucune province occupée".to_owned(),
                    1 => "1 province encore occupée".to_owned(),
                    n => format!("{n} provinces encore occupées"),
                };
                (1.0 - ratio(held, provinces.len()), text)
            }
            ActCondition::ForeignControlAtLeast {
                faction: foreign,
                provinces,
                count,
            } => {
                let held = provinces
                    .iter()
                    .filter(|p| self.controls_province(foreign, p))
                    .count() as u32;
                (f64::from(held >= *count), format!("{held} occupée(s)"))
            }
            ActCondition::ProvinceCount { count } => (
                ratio(held_now, *count as usize),
                format!("{held_now}/{count} provinces"),
            ),
            ActCondition::DomainPercent { percent } => {
                let start = self.victory_start_provinces.max(1) as usize;
                let share = held_now * 100 / start;
                (
                    ratio(share, *percent as usize),
                    format!("{share} % du domaine initial"),
                )
            }
            ActCondition::Prestige { min } => {
                let prestige = self.victory_prestige(faction).max(0);
                (
                    ratio(prestige as usize, *min as usize),
                    format!("prestige {prestige}/{min}"),
                )
            }
            ActCondition::Treasury { min } => {
                let treasury = self.factions.get(faction).map_or(0, |f| f.treasury);
                (
                    ratio(treasury.max(0) as usize, (*min).max(0) as usize),
                    format!("{treasury}/{min} livres"),
                )
            }
            ActCondition::DebtBelow { max } => {
                let debt = self.factions.get(faction).map_or(0, |f| f.finance.debt());
                let r = if debt <= *max {
                    1.0
                } else {
                    (*max as f64 / debt as f64).max(0.0)
                };
                (r, format!("dette {debt} livres (au plus {max})"))
            }
            ActCondition::AlliesAtLeast { count } => {
                let allies = self.factions.get(faction).map_or(0, |f| f.allies.len());
                (
                    ratio(allies, *count as usize),
                    format!("{allies}/{count} alliés"),
                )
            }
            ActCondition::DynastySecure => {
                let f = self.factions.get(faction);
                let ruler_free = f
                    .and_then(|f| f.ruler.as_ref())
                    .and_then(|r| self.characters.get(r))
                    .is_some_and(|c| c.alive && !c.captive);
                let heir = f.is_some_and(|f| f.heir.is_some());
                let no_regency = f.is_some_and(|f| !f.regency);
                let r = 0.4 * f64::from(ruler_free)
                    + 0.4 * f64::from(heir)
                    + 0.2 * f64::from(no_regency);
                let text = match (ruler_free, heir, no_regency) {
                    (true, true, true) => "lignée assurée".to_owned(),
                    (false, _, _) => "roi captif ou absent".to_owned(),
                    (_, false, _) => "aucun héritier".to_owned(),
                    _ => "régence en cours".to_owned(),
                };
                (if r > 0.999 { 1.0 } else { r }, text)
            }
            ActCondition::HoldTitle { title } => {
                let held = crate::feudal::holder_of(self, title) == Some(faction);
                (
                    f64::from(held),
                    if held {
                        "titre détenu"
                    } else {
                        "titre à obtenir"
                    }
                    .to_owned(),
                )
            }
            ActCondition::Independent => {
                let free = self
                    .factions
                    .get(faction)
                    .is_some_and(|f| f.suzerain.is_none());
                (
                    f64::from(free),
                    if free { "indépendant" } else { "vassal" }.to_owned(),
                )
            }
            ActCondition::Subjugate { faction: target } => {
                let done = self
                    .factions
                    .get(target)
                    .is_none_or(|f| !f.alive || f.suzerain.as_ref() == Some(faction));
                (
                    f64::from(done),
                    if done {
                        "soumission obtenue"
                    } else {
                        "soumission à obtenir"
                    }
                    .to_owned(),
                )
            }
            ActCondition::AtWar { faction: foe } => {
                let war = self
                    .factions
                    .get(faction)
                    .is_some_and(|f| f.at_war_with.contains(foe));
                (
                    f64::from(war),
                    if war { "en guerre" } else { "en paix" }.to_owned(),
                )
            }
        }
    }

    /// The pressures of `act` that apply to the player's house.
    fn act_pressures<'a>(
        &self,
        data: &'a GameData,
        rules: &'a ActsRules,
        act: usize,
    ) -> Vec<&'a ActPressure> {
        let player = &self.player_faction;
        let government = government_key(data, player);
        rules.acts[act]
            .pressures
            .iter()
            .filter(|p| {
                (p.audience.factions.is_empty() || p.audience.factions.contains(player))
                    && (p.audience.governments.is_empty()
                        || p.audience.governments.iter().any(|g| g == government))
            })
            .collect()
    }

    /// Factor (percent) the chosen policy gives to a category of the act.
    fn policy_factor(rules: &ActsRules, act: usize, policy: Option<&str>, category: &str) -> u32 {
        policy
            .and_then(|id| rules.acts[act].policies.iter().find(|p| p.id == id))
            .filter(|p| p.category == category)
            .map_or(100, |p| p.factor_percent)
    }

    /// Live status of the act in progress, or of the last one once all are over.
    pub fn act_status(&self, data: &GameData) -> Option<ActStatus> {
        let rules = &data.acts_rules;
        if !rules.enabled() || !self.acts.started {
            return None;
        }
        let index = self.acts.current.min(rules.acts.len() - 1);
        let player = &self.player_faction;
        let policy = self.acts.policy.as_deref();
        let objectives: Vec<ObjectiveLine> = rules
            .objectives(player, government_key(data, player), &rules.acts[index].id)
            .iter()
            .map(|objective| {
                let (ratio, progress) = self.act_condition(player, &objective.condition);
                ObjectiveLine {
                    met: ratio >= 0.999,
                    percent: (ratio * 100.0).round() as u32,
                    progress,
                    factor_percent: Self::policy_factor(rules, index, policy, &objective.category),
                    objective: objective.clone(),
                }
            })
            .collect();
        let pressures: Vec<PressureLine> = self
            .act_pressures(data, rules, index)
            .into_iter()
            .map(|pressure| PressureLine {
                active: self.act_condition(player, &pressure.condition).0 >= 0.999,
                pressure: pressure.clone(),
            })
            .collect();
        let score = score_of(&objectives, &pressures);
        Some(ActStatus {
            index,
            objectives,
            pressures,
            score,
        })
    }

    /// Chooses the policy of the household for the act in progress.
    pub fn choose_act_policy(&mut self, data: &GameData, policy: &str) -> Result<(), ActError> {
        let rules = &data.acts_rules;
        if !rules.enabled() || !self.acts.started || self.acts.finished {
            return Err(ActError::NoAct);
        }
        let act = &rules.acts[self.acts.current.min(rules.acts.len() - 1)];
        if !act.policies.iter().any(|p| p.id == policy) {
            return Err(ActError::UnknownPolicy(policy.to_owned()));
        }
        self.acts.policy = Some(policy.to_owned());
        Ok(())
    }

    /// Reign score (0-100): the acts over, weighted, plus the act in progress when
    /// `with_current` (campaign cut short or defeat).
    pub fn reign_score(&self, data: &GameData, with_current: bool) -> u32 {
        let rules = &data.acts_rules;
        if !rules.enabled() {
            return 0;
        }
        let mut weighted = 0u64;
        let mut total = 0u64;
        for report in &self.acts.reports {
            let weight = rules
                .acts
                .iter()
                .find(|a| a.id == report.act_id)
                .map_or(0, |a| u64::from(a.weight));
            weighted += weight * u64::from(report.score);
            total += weight;
        }
        if with_current && !self.acts.finished {
            if let Some(status) = self.act_status(data) {
                let weight = u64::from(rules.acts[status.index].weight);
                weighted += weight * u64::from(status.score);
                total += weight;
            }
        }
        (weighted / total.max(1)).min(100) as u32
    }

    /// Defeat conditions of the house (not losing every land), if one is met.
    pub fn acts_ruin(&self, data: &GameData) -> Option<Ruin> {
        let rules = &data.acts_rules;
        if !rules.enabled() || !self.acts.started {
            return None;
        }
        let ruin = &rules.ruin;
        let player = &self.player_faction;
        let faction = self.factions.get(player)?;
        if ruin.defaults > 0 && faction.finance.defaults >= ruin.defaults && faction.treasury < 0 {
            return Some(Ruin::Bankruptcy);
        }
        if ruin.capital_lost_seasons > 0
            && self.acts.capital_lost_seasons >= ruin.capital_lost_seasons
        {
            return Some(Ruin::Capital);
        }
        let dynastic = ruin
            .dynastic_governments
            .iter()
            .any(|g| g == government_key(data, player));
        if dynastic && !self.acts.house.is_empty() {
            let ruler_house = faction
                .ruler
                .as_ref()
                .and_then(|r| self.characters.get(r))
                .map(|c| c.house.as_str());
            let line_alive = self
                .characters
                .values()
                .any(|c| c.alive && &c.faction == player && c.house == self.acts.house);
            if ruler_house.is_some_and(|h| h != self.acts.house) && !line_alive {
                return Some(Ruin::Dynasty);
            }
        }
        None
    }

    /// Year the campaign ends for the arc: the closing year of the last act.
    pub fn acts_end_year(&self, data: &GameData) -> Option<i32> {
        data.acts_rules.acts.last().map(|a| a.end_year)
    }

    /// The transition screen of the last act change has not been seen yet.
    pub fn act_transition_pending(&self) -> bool {
        self.acts.last_transition_turn.is_some() && !self.acts.transition_seen
    }

    /// The player has seen the transition screen.
    pub fn mark_act_transition_seen(&mut self) {
        self.acts.transition_seen = true;
    }
}

fn score_of(objectives: &[ObjectiveLine], pressures: &[PressureLine]) -> u32 {
    let mut total = 0.0;
    let mut earned = 0.0;
    for line in objectives {
        let weight = f64::from(line.objective.weight) * f64::from(line.factor_percent) / 100.0;
        let credit = if line.met {
            1.0
        } else {
            PARTIAL_CREDIT * f64::from(line.percent) / 100.0
        };
        total += weight;
        earned += weight * credit;
    }
    let base = if total > 0.0 {
        earned / total * 100.0
    } else {
        0.0
    };
    let adjust: i32 = pressures
        .iter()
        .filter(|p| p.active)
        .map(|p| p.pressure.points)
        .sum();
    (base.round() as i32 + adjust).clamp(0, 100) as u32
}

/// The heaviest causes touching the player's house since `since_turn` (at most two of a kind
/// while other kinds can fill the list).
fn act_causes(state: &CampaignState, since_turn: u32) -> Vec<CauseRecord> {
    let player = &state.player_faction;
    let mut records: Vec<&CauseRecord> = state
        .cause_journal()
        .records()
        .filter(|r| r.turn >= since_turn && r.cause.concerns(player))
        .filter(|r| {
            !matches!(
                r.cause.kind,
                CauseKind::GameWon
                    | CauseKind::GameLost
                    | CauseKind::UnrestWarning
                    | CauseKind::ExtraordinaryTax
                    | CauseKind::Debasement
            )
        })
        .collect();
    records.sort_by(|a, b| {
        b.cause
            .severity
            .cmp(&a.cause.severity)
            .then_with(|| b.id.cmp(&a.id))
    });
    let mut chosen: Vec<&CauseRecord> = Vec::new();
    let mut rest: Vec<&CauseRecord> = Vec::new();
    for record in records {
        let same = chosen
            .iter()
            .filter(|c| c.cause.kind == record.cause.kind)
            .count();
        if same >= 2 {
            rest.push(record);
        } else {
            chosen.push(record);
        }
    }
    chosen.extend(rest);
    chosen.truncate(BILAN_CAUSES);
    chosen.into_iter().cloned().collect()
}

/// Enters the arc on the first season: the act that covers the current year begins.
fn enter(state: &mut CampaignState, data: &GameData) {
    let rules = &data.acts_rules;
    let player = state.player_faction.clone();
    let year = state.year;
    let current = rules
        .acts
        .iter()
        .position(|a| year <= a.end_year)
        .unwrap_or(rules.acts.len() - 1);
    state.acts.started = true;
    state.acts.current = current;
    state.acts.act_start_turn = state.turn;
    state.acts.act_start_provinces = state.controlled_provinces(&player).count() as u32;
    state.acts.act_start_defaults = state
        .factions
        .get(&player)
        .map_or(0, |f| f.finance.defaults);
    state.acts.house = state
        .factions
        .get(&player)
        .and_then(|f| f.ruler.as_ref())
        .and_then(|r| state.characters.get(r))
        .map(|c| c.house.clone())
        .unwrap_or_default();
}

/// Why the act in progress closes this season, if it does.
fn end_reason(state: &CampaignState, data: &GameData) -> Option<&'static str> {
    let rules = &data.acts_rules;
    let act = &rules.acts[state.acts.current];
    if state.year > act.end_year {
        return Some("year");
    }
    let early = &act.early_end;
    let seasons = state.turn.saturating_sub(state.acts.act_start_turn);
    let player = &state.player_faction;
    if early.collapse_domain_percent > 0 && state.acts.act_start_provinces >= 3 {
        let held = state.controlled_provinces(player).count() as u32;
        if held * 100 < state.acts.act_start_provinces * early.collapse_domain_percent {
            return Some("collapse");
        }
    }
    if early.collapse_defaults > 0 {
        let defaults = state
            .factions
            .get(player)
            .map_or(0, |f| f.finance.defaults)
            .saturating_sub(state.acts.act_start_defaults);
        if defaults >= early.collapse_defaults {
            return Some("collapse");
        }
    }
    if early.peace_seasons > 0
        && state.acts.peace_streak >= early.peace_seasons
        && seasons >= early.min_seasons
    {
        return Some("peace");
    }
    None
}

/// Season hook (called by the victory phase): enters the arc, follows the house, closes the acts
/// whose time has come. At most one transition per act: a long jump of the calendar closes
/// several acts in one call.
pub(crate) fn advance(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let rules = &data.acts_rules;
    if !rules.enabled() || state.acts.finished {
        return;
    }
    if !state.acts.started {
        enter(state, data);
    }
    let player = state.player_faction.clone();
    // Follow the house.
    let at_war = state.factions.get(&player).is_some_and(|f| {
        f.at_war_with
            .iter()
            .any(|o| o.as_str() != crate::diplomacy::REBELS_FACTION)
    });
    state.acts.peace_streak = if at_war {
        0
    } else {
        state.acts.peace_streak + 1
    };
    if let Some(faction) = state.factions.get(&player) {
        let capital_lost = !state.controls_province(&player, &faction.capital);
        state.acts.capital_lost_seasons = if capital_lost && faction.heir.is_none() {
            state.acts.capital_lost_seasons + 1
        } else {
            0
        };
        // A change of line that leaves kin of the old house keeps following the new ruler.
        let ruler_house = faction
            .ruler
            .as_ref()
            .and_then(|r| state.characters.get(r))
            .map(|c| c.house.clone());
        if let Some(house) = ruler_house {
            let line_alive = state
                .characters
                .values()
                .any(|c| c.alive && c.faction == player && c.house == state.acts.house);
            if house != state.acts.house && (line_alive || state.acts.house.is_empty()) {
                state.acts.house = house;
            }
        }
    }
    while !state.acts.finished {
        let Some(reason) = end_reason(state, data) else {
            break;
        };
        close_act(state, data, reason, events);
    }
}

/// Closes the act in progress and opens the next one (or ends the arc).
fn close_act(
    state: &mut CampaignState,
    data: &GameData,
    reason: &'static str,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.acts_rules;
    let status = state.act_status(data).expect("arc entered");
    let act = &rules.acts[status.index];
    let report = ActReport {
        act_id: act.id.clone(),
        ended_by: reason.to_owned(),
        start_turn: state.acts.act_start_turn,
        end_turn: state.turn,
        end_year: state.year,
        score: status.score,
        policy: state.acts.policy.clone(),
        objectives: status
            .objectives
            .iter()
            .map(|o| ObjectiveResult {
                id: o.objective.id.clone(),
                met: o.met,
                percent: o.percent,
            })
            .collect(),
        pressures: status
            .pressures
            .iter()
            .map(|p| PressureResult {
                id: p.pressure.id.clone(),
                active: p.active,
                points: if p.active { p.pressure.points } else { 0 },
            })
            .collect(),
        causes: act_causes(state, state.acts.act_start_turn),
    };
    let player = state.player_faction.clone();
    let name = data.faction_name(&player);
    state.acts.reports.push(report);
    state.acts.last_transition_turn = Some(state.turn);
    state.acts.transition_seen = false;
    let last = status.index + 1 >= rules.acts.len();
    let text = if last {
        format!(
            "Fin de l'acte {} « {} » : {name} entre dans le bilan de son règne.",
            status.index + 1,
            act.name
        )
    } else {
        format!(
            "Fin de l'acte {} « {} » ; commence l'acte {} « {} ».",
            status.index + 1,
            act.name,
            status.index + 2,
            rules.acts[status.index + 1].name
        )
    };
    events.push(GameEvent::new(EventKind::Chronicle, text).faction(&player));
    if last {
        state.acts.finished = true;
        return;
    }
    state.acts.current = status.index + 1;
    state.acts.act_start_turn = state.turn;
    state.acts.act_start_provinces = state.controlled_provinces(&player).count() as u32;
    state.acts.act_start_defaults = state
        .factions
        .get(&player)
        .map_or(0, |f| f.finance.defaults);
    state.acts.policy = None;
}
