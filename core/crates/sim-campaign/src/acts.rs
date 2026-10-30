//! The campaign arc in three acts (ADR 0784, refit R1): « La querelle » (1337-1360), « La reprise
//! et la ruine » (1360-1415), « Le sursaut » (1415-1453). Each act gives the house of the player
//! objectives, pressures and ambitions (`data/rules/acts.json`); an act closes with its year and
//! only then (R1-1: no more early end by peace or collapse).
//!
//! **The reign is told by renown (R1-1).** Each season, facts add or remove renown to the act in
//! progress: battles won or lost, places taken or lost, a season of chevauchée, ransoms received
//! or paid, a truce imposed (each capped per season, `renown.sources`). An objective earns its
//! renown (to its own act) the season it is met and gives it back the season it is lost; an
//! objective already met at the start of the campaign is an acquis: it earns nothing, its loss
//! costs as much. Pressures add or remove renown while they hold during their act. The renown of
//! facts of the act in progress wears slowly. The score of an act is its renown over the act's
//! `renown_target`; the reign score is the weighted mean of the three acts, so it moves until 1453
//! and the victory is judged then only.
//!
//! **Ambitions (R1-2).** At each act the player chooses `ambitions_to_choose` ambitions among four;
//! each changes the simulation (effects merged into the realm's, like a technology, see
//! [`ambition_effects`]) and earns more renown from the facts and objectives it names.
//!
//! The extinction of the dynasty, an irreversible royal bankruptcy and the loss of the capital
//! without an heir end the campaign at once (ruin). Only the player's house is followed.

use data_model::currency::MONEY;
use std::collections::{BTreeMap, BTreeSet};

use data_model::{
    ActAmbition, ActCondition, ActObjective, ActPressure, ActsRules, FactionId, GameData,
};
use serde::{Deserialize, Serialize};

use crate::buildings::EffectTotals;
use crate::causes::{CauseKind, CauseRecord};
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// Causes kept in the bilan of an act.
const BILAN_CAUSES: usize = 3;
/// Renown is kept in thousandths, so that the slow wear does not stall on rounding.
const MILLI: i64 = 1000;
/// Prefix of the credit key of a pressure (objective keys are their ids).
const PRESSURE_KEY: &str = "pressure:";

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
    /// `year` (older saves may hold `peace` or `collapse`).
    pub ended_by: String,
    pub start_turn: u32,
    pub end_turn: u32,
    pub end_year: i32,
    /// Act score, 0-100.
    pub score: u32,
    /// R1-1: renown earned in the act.
    #[serde(default)]
    pub renown: i32,
    /// R1-2: ambitions held at the end of the act.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ambitions: Vec<String>,
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

/// R1-1: one fact of renown, as the journal keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenownFact {
    pub turn: u32,
    /// Source name (`battle_won`, `objective_met`...).
    pub source: String,
    /// Renown added (negative: removed).
    pub points: i32,
    /// Act credited.
    pub act: String,
    /// Short French cause (the objective's title, the other party...).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cause: String,
}

/// R1-1: the renown of the house (saved with the campaign, version 12).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RenownState {
    /// The starting point of the house has been recorded (acquis, places, battle counters).
    pub baseline_taken: bool,
    /// Renown of facts per act id, in thousandths (the act in progress wears).
    pub facts_milli: BTreeMap<String, i64>,
    /// Renown of objectives and pressures per act id.
    pub goals: BTreeMap<String, i64>,
    /// Renown each objective (its id) or pressure (`pressure:<id>`) grants now.
    pub credits: BTreeMap<String, i64>,
    /// Objectives already met at the start: acquis to keep, they earn nothing.
    pub acquis: BTreeSet<String>,
    /// Settlements the house controlled last season.
    pub settlements: BTreeSet<String>,
    /// Battle counters of the house last season (`CampaignStats`).
    pub battles_won: u32,
    pub battles_lost: u32,
    /// Renown counted per source in the season `season_turn` (caps).
    pub season_turn: u32,
    pub season_points: BTreeMap<String, i32>,
    /// The last facts, oldest first.
    pub journal: Vec<RenownFact>,
}

/// Bookkeeping of the arc, saved with the campaign (version 12).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ActsState {
    /// The arc has been entered.
    pub started: bool,
    /// Index in `acts_rules.acts` of the act in progress.
    pub current: usize,
    /// Every act is over (the campaign now awaits its final bilan).
    pub finished: bool,
    pub act_start_turn: u32,
    /// R1-2: ambitions held for the act in progress, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ambitions: Vec<String>,
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
    /// R1-1: the renown of the house.
    #[serde(default)]
    pub renown: RenownState,
}

/// Errors of the player's act choices.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActError {
    #[error("the campaign has no act in progress")]
    NoAct,
    #[error("unknown ambition {0}")]
    UnknownAmbition(String),
    #[error("the ambitions of the act can no longer change")]
    AmbitionsLocked,
}

/// Live status of the act in progress (read-only view for the interface).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActStatus {
    pub index: usize,
    pub objectives: Vec<ObjectiveLine>,
    pub pressures: Vec<PressureLine>,
    /// Act score, 0-100.
    pub score: u32,
    /// Renown earned in the act so far.
    pub renown: i32,
}

/// An objective with its live progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveLine {
    pub objective: ActObjective,
    pub met: bool,
    pub percent: u32,
    pub progress: String,
    /// Renown of the objective in percent of its weight, the ambitions' bonus included.
    pub factor_percent: u32,
    /// Already met at the start of the campaign: an acquis to keep (earns nothing, its loss costs).
    pub hold: bool,
    /// Progress toward the target, 0-100 (the advisor's « most worth » objective).
    pub gain_percent: u32,
    /// Renown the objective grants now (negative: a lost acquis).
    pub renown: i32,
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
        map.entry("acts").or_insert_with(|| {
            serde_json::json!({
                "started": false, "current": 0, "finished": false, "act_start_turn": 0,
                "capital_lost_seasons": 0, "transition_seen": false, "reports": []
            })
        });
    }
}

/// Migration v11 -> v12 (R1): the policy becomes the first ambition, the early-end and acquis
/// bookkeeping goes, the renown starts empty (its baseline is taken at the next season).
pub(crate) fn migrate_v11_to_v12(value: &mut serde_json::Value) {
    let Some(acts) = value.get_mut("acts").and_then(|a| a.as_object_mut()) else {
        return;
    };
    let policy_to_ambitions = |map: &mut serde_json::Map<String, serde_json::Value>| {
        if let Some(serde_json::Value::String(policy)) = map.remove("policy") {
            map.insert("ambitions".to_owned(), serde_json::json!([policy]));
        }
    };
    policy_to_ambitions(acts);
    for key in [
        "peace_streak",
        "start_permille",
        "act_start_provinces",
        "act_start_defaults",
    ] {
        acts.remove(key);
    }
    if let Some(reports) = acts.get_mut("reports").and_then(|r| r.as_array_mut()) {
        for report in reports.iter_mut().filter_map(|r| r.as_object_mut()) {
            policy_to_ambitions(report);
        }
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

/// Renown rounded from thousandths.
fn whole(milli: i64) -> i32 {
    ((milli + milli.signum() * MILLI / 2) / MILLI) as i32
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
                // The start count is recorded at the first resolution: until then (turn 0) the
                // domain is the one held now, not 1 province (which read « 1000 % »).
                let start = if self.victory_start_provinces == 0 {
                    held_now.max(1)
                } else {
                    self.victory_start_provinces as usize
                };
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
                    format!("{treasury}/{min} {MONEY}"),
                )
            }
            ActCondition::DebtBelow { max } => {
                let debt = self.factions.get(faction).map_or(0, |f| f.finance.debt());
                let r = if debt <= *max {
                    1.0
                } else {
                    (*max as f64 / debt as f64).max(0.0)
                };
                (r, format!("dette {debt} {MONEY} (au plus {max})"))
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

    /// The objectives of act `index` for the player's house.
    fn act_objectives<'a>(&self, data: &'a GameData, index: usize) -> &'a [ActObjective] {
        let rules = &data.acts_rules;
        let player = &self.player_faction;
        rules.objectives(player, government_key(data, player), &rules.acts[index].id)
    }

    /// R1-2: the ambitions the player's house may choose in the act in progress.
    pub fn act_ambitions<'a>(&self, data: &'a GameData) -> &'a [ActAmbition] {
        let rules = &data.acts_rules;
        if !rules.enabled() {
            return &[];
        }
        let index = self.acts.current.min(rules.acts.len() - 1);
        let player = &self.player_faction;
        rules.ambitions(player, government_key(data, player), &rules.acts[index].id)
    }

    /// R1-2: the first seasons of an act, when a third ambition may still replace the oldest.
    pub fn ambition_choice_open(&self, data: &GameData) -> bool {
        self.turn.saturating_sub(self.acts.act_start_turn) < data.acts_rules.ambition_choice_seasons
    }

    /// R1-2: the ambitions the player's house holds now (none once the arc is over).
    pub fn chosen_ambitions<'a>(&self, data: &'a GameData) -> Vec<&'a ActAmbition> {
        if !self.acts.started || self.acts.finished {
            return Vec::new();
        }
        let list = self.act_ambitions(data);
        self.acts
            .ambitions
            .iter()
            .filter_map(|id| list.iter().find(|a| &a.id == id))
            .collect()
    }

    /// Bonus (percent) the held ambitions give to a renown source or an objective category.
    fn ambition_bonus(&self, data: &GameData, source: Option<&str>, category: Option<&str>) -> u32 {
        self.chosen_ambitions(data)
            .into_iter()
            .filter(|a| {
                source.is_some_and(|s| a.renown_sources.iter().any(|x| x == s))
                    || category.is_some_and(|c| a.objective_categories.iter().any(|x| x == c))
            })
            .map(|a| a.renown_bonus_percent)
            .sum()
    }

    /// Renown earned so far in act `act_id`, in thousandths.
    fn act_renown_milli(&self, act_id: &str) -> i64 {
        let renown = &self.acts.renown;
        renown.facts_milli.get(act_id).copied().unwrap_or(0)
            + renown.goals.get(act_id).copied().unwrap_or(0) * MILLI
    }

    /// Score (0-100) of act `index`: its bilan once over, else its renown over its target.
    fn act_score(&self, rules: &ActsRules, index: usize) -> u32 {
        self.act_score_signed(rules, index).max(0) as u32
    }

    /// Signed act score (-100 to 100): an act whose renown is negative weighs on the reign score,
    /// so that losses keep moving it (R1-1: the reign score never freezes at an act's floor).
    fn act_score_signed(&self, rules: &ActsRules, index: usize) -> i64 {
        let act = &rules.acts[index];
        let target = i64::from(act.renown_target.max(1));
        if let Some(report) = self.acts.reports.iter().find(|r| r.act_id == act.id) {
            // Reports of older saves carry no renown: their score stands.
            return if report.renown < 0 {
                (i64::from(report.renown) * 100 / target).max(-100)
            } else {
                i64::from(report.score)
            };
        }
        (self.act_renown_milli(&act.id) * 100 / (target * MILLI)).clamp(-100, 100)
    }

    /// Live status of the act in progress, or of the last one once all are over.
    pub fn act_status(&self, data: &GameData) -> Option<ActStatus> {
        let rules = &data.acts_rules;
        if !rules.enabled() || !self.acts.started {
            return None;
        }
        let index = self.acts.current.min(rules.acts.len() - 1);
        let player = &self.player_faction;
        let gain = rules.renown.source("objective_met").map_or(0, |s| s.points);
        let objectives: Vec<ObjectiveLine> = self
            .act_objectives(data, index)
            .iter()
            .map(|objective| {
                let (ratio, progress) = self.act_condition(player, &objective.condition);
                let bonus = self.ambition_bonus(data, None, Some(&objective.category));
                ObjectiveLine {
                    met: ratio >= 0.999,
                    hold: self.acts.renown.acquis.contains(&objective.id),
                    gain_percent: (ratio * 100.0).round() as u32,
                    percent: (ratio * 100.0).round() as u32,
                    progress,
                    factor_percent: (gain.max(0) as u32) * (100 + bonus) / 100,
                    renown: self
                        .acts
                        .renown
                        .credits
                        .get(&objective.id)
                        .map_or(0, |c| *c as i32),
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
        Some(ActStatus {
            index,
            objectives,
            pressures,
            score: self.act_score(rules, index),
            renown: whole(self.act_renown_milli(&rules.acts[index].id)),
        })
    }

    /// R1-2: takes an ambition for the act in progress. While fewer than `ambitions_to_choose` are
    /// held it is added; beyond, it replaces the oldest, during the first `ambition_choice_seasons`
    /// of the act only.
    pub fn choose_act_ambition(&mut self, data: &GameData, ambition: &str) -> Result<(), ActError> {
        let rules = &data.acts_rules;
        if !rules.enabled() || !self.acts.started || self.acts.finished {
            return Err(ActError::NoAct);
        }
        if !self.act_ambitions(data).iter().any(|a| a.id == ambition) {
            return Err(ActError::UnknownAmbition(ambition.to_owned()));
        }
        if self.acts.ambitions.iter().any(|a| a == ambition) {
            return Ok(());
        }
        let wanted = rules.ambitions_to_choose.max(1) as usize;
        if self.acts.ambitions.len() >= wanted {
            if !self.ambition_choice_open(data) {
                return Err(ActError::AmbitionsLocked);
            }
            self.acts.ambitions.remove(0);
        }
        self.acts.ambitions.push(ambition.to_owned());
        Ok(())
    }

    /// R1-2: ambitions still to choose in the act in progress.
    pub fn ambitions_pending(&self, data: &GameData) -> usize {
        if !self.acts.started || self.acts.finished {
            return 0;
        }
        (data.acts_rules.ambitions_to_choose as usize).saturating_sub(self.acts.ambitions.len())
    }

    /// Reign score (0-100): the signed scores of the three acts weighted by their weight. It
    /// accumulates with the renown and moves until the end of the last act (R1-1).
    pub fn reign_score(&self, data: &GameData) -> u32 {
        let rules = &data.acts_rules;
        if !rules.enabled() {
            return 0;
        }
        let (mut weighted, mut total) = (0i64, 0i64);
        for (index, act) in rules.acts.iter().enumerate() {
            weighted += i64::from(act.weight) * self.act_score_signed(rules, index);
            total += i64::from(act.weight);
        }
        (weighted / total.max(1)).clamp(0, 100) as u32
    }

    /// R1-1: the renown of the house, every act together (the gauge the interface shows).
    pub fn house_renown(&self, data: &GameData) -> i32 {
        whole(
            data.acts_rules
                .acts
                .iter()
                .map(|a| self.act_renown_milli(&a.id))
                .sum(),
        )
    }

    /// R1-1: the last facts of renown, newest first, at most `count`.
    pub fn renown_recent(&self, count: usize) -> Vec<&RenownFact> {
        self.acts.renown.journal.iter().rev().take(count).collect()
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

/// R1-2: the effects of the ambitions `faction` holds (the player's house only), merged into the
/// realm's effects by `research::faction_tech_effects`.
pub(crate) fn ambition_effects(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> EffectTotals {
    let mut totals = EffectTotals::default();
    if faction != &state.player_faction || state.acts.ambitions.is_empty() {
        return totals;
    }
    for ambition in state.chosen_ambitions(data) {
        for effect in &ambition.effects {
            totals.add_effect(effect);
        }
    }
    totals
}

/// Records a fact of renown for the act in progress: `count` times the points of `source`, the
/// ambitions' bonus on gains, within the cap of the season. No-op outside the arc.
fn add_fact(state: &mut CampaignState, data: &GameData, source: &str, count: i32, cause: String) {
    let rules = &data.acts_rules;
    if !rules.enabled() || !state.acts.started || state.acts.finished || count == 0 {
        return;
    }
    let Some(rule) = rules.renown.source(source) else {
        return;
    };
    let mut points = rule.points.saturating_mul(count);
    if points > 0 {
        let bonus = state.ambition_bonus(data, Some(source), None) as i32;
        points = points * (100 + bonus) / 100;
    }
    let turn = state.turn;
    let renown = &mut state.acts.renown;
    if renown.season_turn != turn {
        renown.season_turn = turn;
        renown.season_points.clear();
    }
    let used = renown.season_points.get(source).copied().unwrap_or(0);
    if rule.season_cap > 0 {
        let cap = rule.season_cap as i32;
        let room = (cap - used.abs()).max(0);
        points = points.clamp(-room, room);
    }
    if points == 0 {
        return;
    }
    *renown.season_points.entry(source.to_owned()).or_default() += points;
    let act = rules.acts[state.acts.current.min(rules.acts.len() - 1)]
        .id
        .clone();
    *renown.facts_milli.entry(act.clone()).or_default() += i64::from(points) * MILLI;
    push_journal(state, data, source, points, act, cause);
}

fn push_journal(
    state: &mut CampaignState,
    data: &GameData,
    source: &str,
    points: i32,
    act: String,
    cause: String,
) {
    let turn = state.turn;
    let journal = &mut state.acts.renown.journal;
    journal.push(RenownFact {
        turn,
        source: source.to_owned(),
        points,
        act,
        cause,
    });
    let keep = data.acts_rules.renown.journal_len as usize;
    if journal.len() > keep {
        let excess = journal.len() - keep;
        journal.drain(..excess);
    }
}

/// Sets the renown an objective or pressure grants (`key`), crediting the difference to `act`.
fn set_credit(
    state: &mut CampaignState,
    data: &GameData,
    key: &str,
    act: &str,
    credit: i64,
    source: &str,
    cause: &str,
) {
    let renown = &mut state.acts.renown;
    let old = renown.credits.get(key).copied().unwrap_or(0);
    if old == credit {
        return;
    }
    if credit == 0 {
        renown.credits.remove(key);
    } else {
        renown.credits.insert(key.to_owned(), credit);
    }
    *renown.goals.entry(act.to_owned()).or_default() += credit - old;
    push_journal(
        state,
        data,
        source,
        (credit - old) as i32,
        act.to_owned(),
        cause.to_owned(),
    );
}

/// R1-1: a ransom changes hands (`payer`'s captive freed for money to `captor`).
pub(crate) fn note_ransom(
    state: &mut CampaignState,
    data: &GameData,
    payer: &FactionId,
    captor: &FactionId,
) {
    let player = state.player_faction.clone();
    if captor == &player {
        add_fact(state, data, "ransom_received", 1, data.faction_name(payer));
    } else if payer == &player {
        add_fact(state, data, "ransom_paid", 1, data.faction_name(captor));
    }
}

/// R1-1: a treaty ends the war between `a` and `b` (called before the peace is made, while the
/// war score still stands): a truce imposed when the player's house had the upper hand.
pub(crate) fn note_war_ended_by_treaty(
    state: &mut CampaignState,
    data: &GameData,
    a: &FactionId,
    b: &FactionId,
) {
    let player = state.player_faction.clone();
    let other = if a == &player {
        b
    } else if b == &player {
        a
    } else {
        return;
    };
    if !state.is_at_war(&player, other) {
        return;
    }
    if state.war_score(data, &player, other) >= data.acts_rules.renown.truce_min_war_score {
        add_fact(state, data, "truce_imposed", 1, data.faction_name(other));
    }
}

/// Records the starting point of the renown: the objectives already met (acquis), the places,
/// the battle counters. Called when the arc opens (and once for older saves).
fn take_baseline(state: &mut CampaignState, data: &GameData) {
    let rules = &data.acts_rules;
    let player = state.player_faction.clone();
    let mut acquis = BTreeSet::new();
    for index in state.acts.current.min(rules.acts.len())..rules.acts.len() {
        for objective in state.act_objectives(data, index) {
            if state.act_condition(&player, &objective.condition).0 >= 0.999 {
                acquis.insert(objective.id.clone());
            }
        }
    }
    let settlements = player_settlements(state, &player);
    let renown = &mut state.acts.renown;
    renown.acquis = acquis;
    renown.settlements = settlements;
    renown.battles_won = state.stats.battles_won;
    renown.battles_lost = state.stats.battles_lost;
    renown.baseline_taken = true;
}

fn player_settlements(state: &CampaignState, player: &FactionId) -> BTreeSet<String> {
    state
        .settlements
        .iter()
        .filter(|(_, s)| &s.controller == player)
        .map(|(id, _)| id.to_string())
        .collect()
}

/// R1-1: the renown of the season — battles, places, chevauchées, objectives, pressures, wear.
fn season_renown(state: &mut CampaignState, data: &GameData, events: &[GameEvent]) {
    let rules = &data.acts_rules;
    let player = state.player_faction.clone();
    // Battles (every battle of the house, wherever it was resolved).
    let won = state
        .stats
        .battles_won
        .saturating_sub(state.acts.renown.battles_won);
    let lost = state
        .stats
        .battles_lost
        .saturating_sub(state.acts.renown.battles_lost);
    state.acts.renown.battles_won = state.stats.battles_won;
    state.acts.renown.battles_lost = state.stats.battles_lost;
    add_fact(state, data, "battle_won", won as i32, String::new());
    add_fact(state, data, "battle_lost", lost as i32, String::new());
    // Places taken and lost.
    let now = player_settlements(state, &player);
    let taken = now.difference(&state.acts.renown.settlements).count() as i32;
    let lost_places = state.acts.renown.settlements.difference(&now).count() as i32;
    state.acts.renown.settlements = now;
    add_fact(state, data, "place_taken", taken, String::new());
    add_fact(state, data, "place_lost", lost_places, String::new());
    // A season of chevauchée.
    let raids = events
        .iter()
        .filter(|e| e.kind == EventKind::Raid && e.faction.as_ref() == Some(&player))
        .count() as i32;
    add_fact(state, data, "raid_season", raids, String::new());
    // Objectives of the act in progress and of the acts to come.
    let gain = rules.renown.source("objective_met").map_or(0, |s| s.points);
    let loss = rules
        .renown
        .source("objective_lost")
        .map_or(0, |s| s.points.abs());
    for index in state.acts.current..rules.acts.len() {
        let act_id = rules.acts[index].id.as_str();
        for objective in state.act_objectives(data, index) {
            let acquis = state.acts.renown.acquis.contains(&objective.id);
            // An acquis of an act to come is kept from the opening of its act only (the same
            // dynasty or debt objective of three acts is not lost three times at once).
            if acquis && index > state.acts.current {
                continue;
            }
            let met = state.act_condition(&player, &objective.condition).0 >= 0.999;
            let current = state
                .acts
                .renown
                .credits
                .get(&objective.id)
                .copied()
                .unwrap_or(0);
            let weight = i64::from(objective.weight);
            let target = if acquis {
                // An acquis earns nothing; its loss costs its weight.
                match (met, current) {
                    (true, _) => 0,
                    (false, 0) => -weight * i64::from(loss) / 100,
                    (false, c) => c,
                }
            } else {
                match (met, current) {
                    (true, 0) => {
                        let bonus = state.ambition_bonus(data, None, Some(&objective.category));
                        weight * i64::from(gain) / 100 * i64::from(100 + bonus) / 100
                    }
                    (true, c) => c,
                    (false, _) => 0,
                }
            };
            if target != current {
                let source = if target > current {
                    "objective_met"
                } else {
                    "objective_lost"
                };
                set_credit(
                    state,
                    data,
                    &objective.id,
                    act_id,
                    target,
                    source,
                    &objective.title,
                );
            }
        }
    }
    // Pressures of the act in progress.
    let index = state.acts.current;
    let act_id = rules.acts[index].id.as_str();
    let percent = rules.renown.source("pressure").map_or(0, |s| s.points);
    for pressure in state.act_pressures(data, rules, index) {
        let active = state.act_condition(&player, &pressure.condition).0 >= 0.999;
        let target = if active {
            i64::from(pressure.points) * i64::from(percent) / 100
        } else {
            0
        };
        let key = format!("{PRESSURE_KEY}{}", pressure.id);
        set_credit(
            state,
            data,
            &key,
            act_id,
            target,
            "pressure",
            &pressure.title,
        );
    }
    // The slow wear of the renown of facts of the act in progress.
    let decay = i64::from(rules.renown.decay_permille_per_season);
    if let Some(facts) = state.acts.renown.facts_milli.get_mut(act_id) {
        *facts -= *facts * decay / 1000;
    }
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

/// Opens the first act at the creation of a campaign (no-op without the arc).
pub(crate) fn start(state: &mut CampaignState, data: &GameData) {
    if data.acts_rules.enabled() && !state.acts.started {
        enter(state, data);
    }
}

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
    state.acts.house = state
        .factions
        .get(&player)
        .and_then(|f| f.ruler.as_ref())
        .and_then(|r| state.characters.get(r))
        .map(|c| c.house.clone())
        .unwrap_or_default();
    take_baseline(state, data);
}

/// Season hook (called by the victory phase): enters the arc, follows the house, counts the
/// renown of the season, closes the acts whose year is past. A long jump of the calendar closes
/// several acts in one call.
pub(crate) fn advance(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let rules = &data.acts_rules;
    if !rules.enabled() || state.acts.finished {
        return;
    }
    if !state.acts.started {
        enter(state, data);
    }
    if !state.acts.renown.baseline_taken {
        take_baseline(state, data);
    }
    let player = state.player_faction.clone();
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
    season_renown(state, data, events);
    while !state.acts.finished && state.year > rules.acts[state.acts.current].end_year {
        close_act(state, data, events);
    }
}

/// Closes the act in progress and opens the next one (or ends the arc).
fn close_act(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let rules = &data.acts_rules;
    let status = state.act_status(data).expect("arc entered");
    let act = &rules.acts[status.index];
    let report = ActReport {
        act_id: act.id.clone(),
        ended_by: "year".to_owned(),
        start_turn: state.acts.act_start_turn,
        end_turn: state.turn,
        end_year: state.year,
        score: status.score,
        renown: status.renown,
        ambitions: state.acts.ambitions.clone(),
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
        state.acts.ambitions.clear();
        return;
    }
    state.acts.current = status.index + 1;
    state.acts.act_start_turn = state.turn;
    state.acts.ambitions.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_v11_to_v12_turns_the_policy_into_an_ambition() {
        let mut value = serde_json::json!({"acts": {
            "started": true, "current": 1, "finished": false, "act_start_turn": 92,
            "policy": "pol_2_rigor", "peace_streak": 3, "start_permille": 400,
            "capital_lost_seasons": 0, "transition_seen": true,
            "reports": [{"act_id": "act_1", "ended_by": "peace", "start_turn": 0, "end_turn": 92,
                "end_year": 1360, "score": 40, "policy": "pol_1_war", "objectives": [],
                "pressures": []}]
        }});
        migrate_v11_to_v12(&mut value);
        let acts: ActsState = serde_json::from_value(value["acts"].clone()).unwrap();
        assert_eq!(acts.ambitions, vec!["pol_2_rigor"]);
        assert_eq!(acts.reports[0].ambitions, vec!["pol_1_war"]);
        assert!(!acts.renown.baseline_taken);
    }
}
