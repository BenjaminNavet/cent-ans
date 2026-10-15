//! Campaign progress views (lot "progress"): objectives panel, victory tracker and end screen,
//! army traditions, first-turns tutorial.
//!
//! Replaces `get_objectives`, `get_outcome`, `set_victory_length`, `get_campaign_report`,
//! `get_army_traditions`, `get_armies_with_pending_traditions` of `godot-bridge` and the logic
//! of `tutorial_steps.gd` / `tutorial_controller.gd` that reads the simulation. Pure views:
//! orders go through `diplomacy::submit_order` (`Order::ChooseArmyTradition`).

use std::collections::BTreeMap;

use data_model::{FactionId, GameData};
use serde::Serialize;
use serde_json::Value;
use sim_campaign::victory::{OutcomeKind, VictoryLength};
use sim_campaign::{ArmyId, CampaignState};

// ------------------------------------------------------------------ objectives

/// One objective with its progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectiveRow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub done: bool,
    pub progress: String,
}

/// Everything the objectives panel and the victory tracker show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectivesView {
    /// Historical summary of the faction (`victory.summary`), may be empty.
    pub summary: String,
    pub end_year: i32,
    /// `short` or `long` (ADR 0332).
    pub length: String,
    pub score: i64,
    /// Seasons all objectives must hold in a row.
    pub hold_turns: u32,
    /// Seasons they have held so far.
    pub streak: u32,
    /// `ongoing`, `victory`, `defeat` or `ended`.
    pub state: String,
    pub objectives: Vec<ObjectiveRow>,
    pub done_count: usize,
}

impl ObjectivesView {
    /// Line under the panel title: summary, deadline and score.
    pub fn header_line(&self) -> String {
        let mut line = format!(
            "{} — échéance {} — score actuel : {}",
            self.summary, self.end_year, self.score
        );
        if self.hold_turns > 1 {
            if self.streak > 0 {
                let left = self.hold_turns.saturating_sub(self.streak);
                line.push_str(&format!(
                    "\nTous les objectifs sont remplis : encore {left} {} à les tenir pour l’emporter.",
                    if left <= 1 { "saison" } else { "saisons" }
                ));
            } else {
                line.push_str(&format!(
                    "\nLa victoire exige de tenir tous les objectifs {} saisons d’affilée.",
                    self.hold_turns
                ));
            }
        }
        line
    }
}

fn length_key(length: VictoryLength) -> &'static str {
    match length {
        VictoryLength::Short => "short",
        VictoryLength::Long => "long",
    }
}

fn kind_key(kind: OutcomeKind) -> &'static str {
    match kind {
        OutcomeKind::Victory => "victory",
        OutcomeKind::Defeat => "defeat",
        OutcomeKind::Ended => "ended",
    }
}

/// End year that applies to `faction` for the chosen length (same rule as `victory_set` of
/// `sim-campaign`, which keeps it private).
pub fn end_year(data: &GameData, state: &CampaignState, faction: &FactionId) -> i32 {
    let rules = &data.feudal_rules.victory;
    let short = state.victory_length == VictoryLength::Short;
    match data.factions.get(faction).and_then(|f| f.victory.as_ref()) {
        Some(v) if short => v
            .short_end_year
            .unwrap_or(rules.short_end_year)
            .min(v.end_year),
        Some(v) => v.end_year,
        None if short => rules.short_end_year,
        None => data.feudal_rules.default_end_year,
    }
}

/// Objectives panel of the player's faction.
pub fn objectives_view(data: &GameData, state: &CampaignState) -> ObjectivesView {
    let faction = state.player_faction();
    let objectives: Vec<ObjectiveRow> = state
        .objectives(data, faction)
        .into_iter()
        .map(|o| ObjectiveRow {
            id: o.id,
            title: o.title,
            description: o.description,
            done: o.done,
            progress: o.progress,
        })
        .collect();
    let done_count = objectives.iter().filter(|o| o.done).count();
    let (score, state_key) = match &state.outcome {
        Some(outcome) => (outcome.score, kind_key(outcome.kind)),
        None => (state.campaign_score(data, faction), "ongoing"),
    };
    ObjectivesView {
        summary: data
            .factions
            .get(faction)
            .and_then(|f| f.victory.as_ref())
            .and_then(|v| v.summary.clone())
            .unwrap_or_default(),
        end_year: end_year(data, state, faction),
        length: length_key(state.victory_length).to_owned(),
        score,
        hold_turns: state.victory_hold_turns(data, faction),
        streak: state.victory_streak,
        state: state_key.to_owned(),
        objectives,
        done_count,
    }
}

/// Chooses the campaign length (`short` or `long`); false for another word.
pub fn set_victory_length(state: &mut CampaignState, length: &str) -> bool {
    let length = match length {
        "short" => VictoryLength::Short,
        "long" => VictoryLength::Long,
        _ => return false,
    };
    state.set_victory_length(length);
    true
}

// ------------------------------------------------------------------ end screen

/// Victory / defeat / end of campaign screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EndView {
    /// `victory`, `defeat` or `ended`.
    pub kind: String,
    pub title: String,
    pub text: String,
    pub score: i64,
    pub turn: u32,
    /// Illustration under `game/assets` (`art/endings/...`).
    pub art: String,
    /// Figures of the campaign, one line each.
    pub lines: Vec<String>,
    /// Tooltip of the score: one line per component.
    pub breakdown: String,
}

fn count_text(value: i64, singular: &str) -> String {
    if value.abs() > 1 {
        format!("{value} {singular}s")
    } else {
        format!("{value} {singular}")
    }
}

/// End screen once the campaign has an outcome (`None` while it goes on).
pub fn end_view(data: &GameData, state: &CampaignState) -> Option<EndView> {
    let outcome = state.outcome.as_ref()?;
    let report = state.campaign_report(data);
    let breakdown = {
        let mut parts = vec![
            format!("{} terres", report.score_provinces),
            format!("{} objectifs", report.score_objectives),
            format!("{} prestige", report.score_prestige),
            format!("{} trésor", report.score_treasury),
        ];
        if report.score_other != 0 {
            parts.push(format!("{} autres", report.score_other));
        }
        format!("{}\n= {}", parts.join("\n"), report.score)
    };
    let lines = vec![
        format!(
            "{} – {} : {}",
            report.start_year,
            report.year,
            count_text(i64::from(report.turns), "saison")
        ),
        format!(
            "Provinces : {} au départ, {} au plus haut, {} à la fin",
            report.provinces_start, report.provinces_peak, report.provinces_end
        ),
        format!(
            "Batailles : {} gagnée(s), {} perdue(s)",
            report.battles_won, report.battles_lost
        ),
        format!(
            "Objectifs : {} sur {} remplis ; prestige {}",
            report.objectives_done, report.objectives_total, report.prestige
        ),
        format!("Score : {}", breakdown.replace('\n', " · ")),
    ];
    let (title, art) = match outcome.kind {
        OutcomeKind::Victory => (
            "Le royaume est à vous",
            "art/endings/end_campaign_victory.jpg",
        ),
        OutcomeKind::Defeat => (
            "La maison est tombée",
            "art/endings/end_campaign_defeat.jpg",
        ),
        OutcomeKind::Ended => ("Fin de la campagne", "art/endings/end_campaign_victory.jpg"),
    };
    Some(EndView {
        kind: kind_key(outcome.kind).to_owned(),
        title: title.to_owned(),
        text: outcome.text_fr.clone(),
        score: outcome.score,
        turn: outcome.turn,
        art: art.to_owned(),
        lines,
        breakdown,
    })
}

// ------------------------------------------------------------------ traditions

/// One tradition of the panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraditionRow {
    pub id: String,
    pub branch: String,
    pub branch_name: String,
    pub tier: u8,
    pub name: String,
    pub description: String,
    pub effects_text: String,
    pub chosen: bool,
    pub allowed: bool,
    pub reason: String,
}

/// Traditions panel of an army.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraditionsView {
    pub army: String,
    pub name: String,
    pub xp: u32,
    pub rank: u8,
    pub max_rank: u8,
    /// Experience of the next rank, `None` at the last one.
    pub next_threshold: Option<u32>,
    /// Ranks still to spend.
    pub pending: u8,
    pub banner_house: String,
    pub effects_text: String,
    /// Names of the traditions chosen.
    pub chosen: Vec<String>,
    /// Every tradition, in data order.
    pub options: Vec<TraditionRow>,
}

impl TraditionsView {
    /// « Rang 1 sur 4 — 25 / 60 d’expérience ».
    pub fn rank_line(&self) -> String {
        let head = format!("Rang {} sur {} — {}", self.rank, self.max_rank, self.xp);
        match self.next_threshold {
            Some(next) => format!("{head} / {next} d’expérience"),
            None => format!("{head} d’expérience (rang maximal)"),
        }
    }

    /// Next tradition of each branch (the lowest tier not chosen), in data order.
    pub fn next_by_branch(&self) -> Vec<&TraditionRow> {
        let mut order: Vec<&str> = Vec::new();
        let mut best: BTreeMap<&str, &TraditionRow> = BTreeMap::new();
        for option in self.options.iter().filter(|o| !o.chosen) {
            match best.get(option.branch.as_str()) {
                None => {
                    order.push(option.branch.as_str());
                    best.insert(option.branch.as_str(), option);
                }
                Some(current) if option.tier < current.tier => {
                    best.insert(option.branch.as_str(), option);
                }
                _ => {}
            }
        }
        order
            .into_iter()
            .filter_map(|b| best.get(b).copied())
            .collect()
    }
}

/// Traditions panel of `army`; `None` for an unknown army.
pub fn traditions_view(
    data: &GameData,
    state: &CampaignState,
    army: &ArmyId,
) -> Option<TraditionsView> {
    let view = state.army_tradition_view(data, army)?;
    Some(TraditionsView {
        army: army.as_str().to_owned(),
        name: sim_campaign::events::capitalize(&view.name),
        xp: view.xp,
        rank: view.rank,
        max_rank: view.max_rank,
        next_threshold: view.next_threshold,
        pending: view.pending,
        banner_house: view.banner_house.unwrap_or_default(),
        effects_text: view.effects.text_fr(),
        chosen: view
            .options
            .iter()
            .filter(|o| o.chosen)
            .map(|o| o.name.clone())
            .collect(),
        options: view
            .options
            .into_iter()
            .map(|o| TraditionRow {
                id: o.id,
                branch: o.branch.key().to_owned(),
                branch_name: o.branch_name,
                tier: o.tier,
                name: o.name,
                description: o.description,
                effects_text: o.effects_text,
                chosen: o.chosen,
                allowed: o.allowed,
                reason: o.reason,
            })
            .collect(),
    })
}

/// An army of the player with a rank to spend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingTradition {
    pub army: String,
    pub name: String,
    pub pending: u8,
}

/// Armies of the player with a tradition to choose.
pub fn pending_traditions(data: &GameData, state: &CampaignState) -> Vec<PendingTradition> {
    let player = state.player_faction();
    state
        .armies()
        .iter()
        .filter(|(_, a)| &a.faction == player)
        .filter_map(|(id, a)| {
            let pending = sim_campaign::traditions::pending_choices(data, a);
            (pending > 0).then(|| PendingTradition {
                army: id.as_str().to_owned(),
                name: sim_campaign::events::capitalize(&state.army_name(data, id)),
                pending,
            })
        })
        .collect()
}

// ------------------------------------------------------------------ tutorial

/// One step of the tutorial, texts resolved for the player's faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TutorialStep {
    pub id: String,
    pub title: String,
    pub text: String,
    pub objective: String,
    /// Historical advice of the faction for this step (may be empty).
    pub advice: String,
    /// Name of the thing to point at (`royal_army`, `capital`, `end_turn`...).
    pub target: String,
    /// Validated by the « Continuer » button.
    pub manual: bool,
    /// The step may be done while a modal window is open.
    pub modal_ok: bool,
}

fn fill(template: &str, values: &[(&str, String)]) -> String {
    let mut text = template.to_owned();
    for (key, value) in values {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text
}

/// Steps of the tutorial of the player's faction (`data/tutorial/steps.json` as `raw`).
pub fn tutorial_steps(raw: &Value, data: &GameData, state: &CampaignState) -> Vec<TutorialStep> {
    let faction_id = state.player_faction();
    let faction = data.factions.get(faction_id);
    let sheet = crate::feudal::feudal_sheet(data, state, faction_id);
    let ruler = sheet.as_ref().map(|s| s.ruler.clone()).unwrap_or_default();
    let liege = sheet
        .as_ref()
        .and_then(|s| s.liege.as_ref())
        .map(|l| l.name.clone())
        .unwrap_or_default();
    let mut lines: Vec<String> = faction
        .and_then(|f| f.victory.as_ref())
        .map(|v| {
            v.objectives
                .iter()
                .map(|o| format!("• {}", o.title))
                .collect()
        })
        .unwrap_or_default();
    let mut objectives_title = format!(
        "Vos objectifs historiques (avant {})",
        faction
            .and_then(|f| f.victory.as_ref())
            .map_or(1453, |v| v.end_year)
    );
    if lines.is_empty() {
        if let Some(sheet) = &sheet {
            lines = sheet
                .objectives
                .iter()
                .map(|o| format!("• {}", o.title))
                .collect();
            if !lines.is_empty() {
                objectives_title = "Les objectifs de votre titre".to_owned();
            }
        }
    }
    let army_name = raw["army_names"][faction_id.as_str()]
        .as_str()
        .unwrap_or("votre armée principale");
    let values = [
        ("army", army_name.to_owned()),
        (
            "faction",
            faction.map_or_else(|| "votre royaume".to_owned(), |f| f.name.display.clone()),
        ),
        (
            "welcome",
            if ruler.is_empty() {
                "Bienvenue".to_owned()
            } else {
                format!("Bienvenue, {ruler}")
            },
        ),
        (
            "capital",
            faction
                .and_then(|f| f.capital_city.clone())
                .unwrap_or_else(|| "votre capitale".to_owned()),
        ),
        (
            "objectives",
            if lines.is_empty() {
                "• Survivre et prospérer.".to_owned()
            } else {
                lines.join("\n")
            },
        ),
        ("objectives_title", objectives_title),
        ("liege", liege.clone()),
    ];
    let advice_of = |step: &str| -> String {
        let own = &raw["advice"][faction_id.as_str()];
        let text = if own.is_object() {
            own[step].as_str()
        } else if step == "intro" {
            faction.and_then(|f| f.description.as_deref())
        } else if step == "diplomacy" && !liege.is_empty() {
            raw["generic_advice"]["diplomacy_vassal"].as_str()
        } else {
            raw["generic_advice"][step].as_str()
        };
        fill(text.unwrap_or(""), &values)
    };
    raw["step_ids"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|id| {
            let base = &raw["steps"][id];
            let text = |key: &str| fill(base[key].as_str().unwrap_or(""), &values);
            TutorialStep {
                id: id.to_owned(),
                title: text("title"),
                text: text("text"),
                objective: text("objective"),
                advice: advice_of(id),
                target: base["target"].as_str().unwrap_or("").to_owned(),
                manual: base["manual"].as_bool().unwrap_or(false),
                modal_ok: base["modal_ok"].as_bool().unwrap_or(false),
            }
        })
        .collect()
}

/// What the tutorial remembers of the simulation when a step opens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TutorialSnapshot {
    pub turn: u32,
    /// Position and march of each army of the player.
    pub places: BTreeMap<String, String>,
    pub constructions: u32,
    pub research: String,
    pub tax: String,
    /// Governed province of each character of the player's faction.
    pub governors: BTreeMap<String, String>,
}

fn army_place(army: &sim_campaign::Army) -> String {
    format!("{:?}", army.position)
}

/// Ids of the armies of the player.
pub fn player_armies(state: &CampaignState) -> Vec<String> {
    let player = state.player_faction();
    state
        .armies()
        .iter()
        .filter(|(_, a)| &a.faction == player)
        .map(|(id, _)| id.as_str().to_owned())
        .collect()
}

/// The royal army: the one led by the ruler, else the largest.
pub fn royal_army(data: &GameData, state: &CampaignState) -> Option<String> {
    let player = state.player_faction();
    let ruler = data.factions.get(player).and_then(|f| f.ruler.clone());
    let mut best: Option<(&ArmyId, usize)> = None;
    for (id, army) in state.armies().iter().filter(|(_, a)| &a.faction == player) {
        if ruler.is_some() && army.general == ruler {
            return Some(id.as_str().to_owned());
        }
        if best.is_none_or(|(_, n)| army.units.len() > n) {
            best = Some((id, army.units.len()));
        }
    }
    best.map(|(id, _)| id.as_str().to_owned())
}

fn construction_count(state: &CampaignState) -> u32 {
    let player = state.player_faction();
    state
        .settlements
        .values()
        .filter(|s| &s.owner == player && s.construction.is_some())
        .count() as u32
}

/// Snapshot taken when a step opens.
pub fn tutorial_snapshot(state: &CampaignState) -> TutorialSnapshot {
    let player = state.player_faction();
    let faction = state.faction_state(player);
    TutorialSnapshot {
        turn: state.turn(),
        places: state
            .armies()
            .iter()
            .filter(|(_, a)| &a.faction == player)
            .map(|(id, a)| (id.as_str().to_owned(), army_place(a)))
            .collect(),
        constructions: construction_count(state),
        research: faction
            .and_then(|f| f.research.as_ref())
            .map_or_else(String::new, ToString::to_string),
        tax: faction.map_or_else(String::new, |f| format!("{:?}", f.tax_rate)),
        governors: state
            .characters
            .iter()
            .filter(|(_, c)| &c.faction == player)
            .map(|(id, c)| {
                (
                    id.to_string(),
                    c.governor_of
                        .as_ref()
                        .map_or_else(String::new, ToString::to_string),
                )
            })
            .collect(),
    }
}

/// Whether the objective of `step_id` is met by the simulation, given the snapshot taken when
/// the step opened. `None` for steps that read the interface (selection, open panels...) and
/// manual steps: the client decides those.
pub fn sim_objective_met(
    step_id: &str,
    before: &TutorialSnapshot,
    state: &CampaignState,
) -> Option<bool> {
    let now = tutorial_snapshot(state);
    Some(match step_id {
        "move_army" => state.armies().iter().any(|(id, army)| {
            if &army.faction != state.player_faction() {
                return false;
            }
            !army.planned_path.is_empty()
                || before
                    .places
                    .get(id.as_str())
                    .is_some_and(|p| *p != army_place(army))
        }),
        "build" => now.constructions > before.constructions,
        "research" => !now.research.is_empty() && now.research != before.research,
        "end_turn" => now.turn > before.turn,
        "tax" => now.tax != before.tax,
        "governor" => now.governors.iter().any(|(id, province)| {
            !province.is_empty() && before.governors.get(id) != Some(province)
        }),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_campaign::Order;

    fn setup() -> (&'static GameData, CampaignState) {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        (data, state)
    }

    fn steps_json() -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../data/tutorial/steps.json");
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn objectives_at_start() {
        let (data, state) = setup();
        let view = objectives_view(data, &state);
        assert_eq!(view.state, "ongoing");
        assert_eq!(view.length, "long");
        assert!(!view.objectives.is_empty());
        assert!(view.end_year > 1337);
        assert!(view.header_line().contains("échéance"));
        assert!(serde_json::to_string(&view).unwrap().contains("objectives"));
    }

    #[test]
    fn short_campaign_ends_sooner() {
        let (data, mut state) = setup();
        let long = objectives_view(data, &state).end_year;
        assert!(set_victory_length(&mut state, "short"));
        let short = objectives_view(data, &state);
        assert_eq!(short.length, "short");
        assert!(short.end_year <= long);
        assert!(!set_victory_length(&mut state, "medium"));
    }

    #[test]
    fn no_end_view_while_ongoing() {
        let (data, state) = setup();
        assert!(end_view(data, &state).is_none());
    }

    #[test]
    fn end_view_after_outcome() {
        let (data, mut state) = setup();
        state.outcome = Some(sim_campaign::victory::Outcome {
            kind: OutcomeKind::Victory,
            turn: 12,
            text_fr: "Gloire".to_owned(),
            score: 321,
        });
        let view = end_view(data, &state).unwrap();
        assert_eq!(view.kind, "victory");
        assert_eq!(view.lines.len(), 5);
        assert!(view.breakdown.contains("= "));
        assert_eq!(objectives_view(data, &state).state, "victory");
    }

    #[test]
    fn traditions_of_an_army() {
        let (data, state) = setup();
        let ids = player_armies(&state);
        let id = ArmyId::parse(&ids[0]).unwrap();
        let view = traditions_view(data, &state, &id).unwrap();
        assert_eq!(view.pending, 0);
        assert!(view.rank_line().starts_with("Rang"));
        let next = view.next_by_branch();
        assert!(!next.is_empty());
        assert!(next.iter().all(|o| o.tier == 1));
        assert!(pending_traditions(data, &state).is_empty());
        assert!(traditions_view(data, &state, &ArmyId::from_index(9999)).is_none());
    }

    #[test]
    fn tutorial_steps_are_resolved() {
        let (data, state) = setup();
        let steps = tutorial_steps(&steps_json(), data, &state);
        assert_eq!(steps.len(), 14);
        assert_eq!(steps[0].id, "intro");
        assert!(steps[0].manual && steps[13].manual);
        assert!(steps
            .iter()
            .all(|s| !s.title.is_empty() && !s.text.is_empty()));
        assert!(steps
            .iter()
            .all(|s| !s.text.contains('{') && !s.title.contains('{')));
        assert!(steps[0].title.starts_with("Bienvenue"));
        assert!(!steps[1].advice.is_empty());
    }

    #[test]
    fn end_turn_and_march_are_detected() {
        let (data, mut state) = setup();
        let before = tutorial_snapshot(&state);
        assert_eq!(sim_objective_met("end_turn", &before, &state), Some(false));
        assert_eq!(sim_objective_met("move_army", &before, &state), Some(false));
        assert_eq!(sim_objective_met("select_army", &before, &state), None);
        let _ = royal_army(data, &state).expect("a royal army");
        state.end_turn(data);
        assert_eq!(sim_objective_met("end_turn", &before, &state), Some(true));
        let _ = Order::ChooseMission { choice: None };
    }
}
