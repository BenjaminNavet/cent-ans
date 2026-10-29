//! The arc in three acts (ADR 0784, refit R1): the act in progress, the renown of the house, the
//! transition screen with the bilan of the act that has just closed, and the ambitions of the
//! household (the `policies` rows keep their name for the client). Pure views over
//! `sim_campaign::acts`; the texts come from `data/rules/acts.json`.

use data_model::GameData;
use serde::Serialize;
use sim_campaign::CampaignState;

/// One objective row of the act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActObjectiveRow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    /// Weight in the act score, policy included.
    pub weight: u32,
    pub done: bool,
    /// Already met when the act opened: to keep, earns nothing (not an accomplishment).
    pub held: bool,
    pub percent: u32,
    pub progress: String,
}

/// One pressure of the act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActPressureRow {
    pub id: String,
    pub title: String,
    pub text: String,
    pub active: bool,
    pub points: i32,
}

/// An ambition the player may choose (R1-2; the row keeps its old name for the client).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActPolicyRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub chosen: bool,
    /// Extra renown, in percent, from the facts and objectives the ambition names.
    pub renown_bonus_percent: u32,
}

/// The act in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActView {
    /// 1-based number of the act.
    pub number: usize,
    pub total: usize,
    pub name: String,
    pub subtitle: String,
    pub intro: String,
    pub end_year: i32,
    /// Score of the act as it stands (0-100).
    pub score: u32,
    /// Reign score as it stands (every act, R1-1: it moves until 1453).
    pub reign_score: u32,
    /// R1-1: renown earned in this act.
    pub renown: i32,
    /// R1-1: renown of the house, every act together.
    pub house_renown: i32,
    /// R1-1: the last three facts of renown, newest first, one line each.
    pub renown_recent: Vec<String>,
    pub objectives: Vec<ActObjectiveRow>,
    pub pressures: Vec<ActPressureRow>,
    pub policies: Vec<ActPolicyRow>,
    /// An ambition is still to be chosen.
    pub policy_pending: bool,
}

fn text(data: &GameData, key: &str) -> String {
    data.acts_rules.texts.get(key).cloned().unwrap_or_default()
}

fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    pairs.iter().fold(template.to_owned(), |acc, (key, value)| {
        acc.replace(&format!("{{{key}}}"), value)
    })
}

/// The ambitions of the act in progress, the held ones marked.
fn policy_rows(data: &GameData, state: &CampaignState) -> Vec<ActPolicyRow> {
    state
        .act_ambitions(data)
        .iter()
        .map(|a| ActPolicyRow {
            id: a.id.clone(),
            name: a.name.clone(),
            description: a.description.clone(),
            chosen: state.acts.ambitions.contains(&a.id),
            renown_bonus_percent: a.renown_bonus_percent,
        })
        .collect()
}

/// R1-1: one fact of renown as a line (« Bataille gagnée +3 (Angleterre) »).
pub fn renown_line(data: &GameData, fact: &sim_campaign::acts::RenownFact) -> String {
    let label = data
        .acts_rules
        .renown
        .labels
        .get(&fact.source)
        .cloned()
        .unwrap_or_else(|| fact.source.clone());
    if fact.cause.is_empty() {
        format!("{label} {:+}", fact.points)
    } else {
        format!("{label} {:+} ({})", fact.points, fact.cause)
    }
}

/// R1-1: the last `count` facts of renown, newest first.
pub fn renown_recent(data: &GameData, state: &CampaignState, count: usize) -> Vec<String> {
    state
        .renown_recent(count)
        .into_iter()
        .map(|fact| renown_line(data, fact))
        .collect()
}

/// The act in progress (`None` without the arc, or before the first season).
pub fn act_view(data: &GameData, state: &CampaignState) -> Option<ActView> {
    let status = state.act_status(data)?;
    let rules = &data.acts_rules;
    let act = &rules.acts[status.index];
    let objectives = status
        .objectives
        .iter()
        .map(|o| ActObjectiveRow {
            id: o.objective.id.clone(),
            title: o.objective.title.clone(),
            description: data_model::currency::fill(&o.objective.description),
            category: rules
                .categories
                .get(&o.objective.category)
                .cloned()
                .unwrap_or_else(|| o.objective.category.clone()),
            weight: o.objective.weight * o.factor_percent / 100,
            done: o.met,
            held: o.hold,
            percent: o.percent,
            progress: o.progress.clone(),
        })
        .collect();
    let pressures = status
        .pressures
        .iter()
        .map(|p| ActPressureRow {
            id: p.pressure.id.clone(),
            title: p.pressure.title.clone(),
            text: p.pressure.text.clone(),
            active: p.active,
            points: p.pressure.points,
        })
        .collect();
    Some(ActView {
        number: status.index + 1,
        total: rules.acts.len(),
        name: act.name.clone(),
        subtitle: act.subtitle.clone(),
        intro: act.intro.clone(),
        end_year: act.end_year,
        score: status.score,
        reign_score: state.reign_score(data),
        renown: status.renown,
        house_renown: state.house_renown(data),
        renown_recent: renown_recent(data, state, 3),
        objectives,
        pressures,
        policies: if state.acts.finished {
            Vec::new()
        } else {
            policy_rows(data, state)
        },
        policy_pending: state.ambitions_pending(data) > 0,
    })
}

/// Takes an ambition for the act in progress (R1-2: two among four; beyond, it replaces the
/// oldest while the choice is open); `false` for an unknown ambition, no act, or a closed choice.
pub fn choose_policy(data: &GameData, state: &mut CampaignState, policy: &str) -> bool {
    state.choose_act_ambition(data, policy).is_ok()
}

/// The transition screen: the bilan of the act that has just closed, and the act that begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransitionView {
    pub title: String,
    /// Name of the act that closed.
    pub act_name: String,
    /// How it closed: `year` (R1-1; older saves may hold `peace` or `collapse`).
    pub ended_by: String,
    pub closing: String,
    pub ended_line: String,
    pub score: u32,
    pub score_line: String,
    pub grade_line: String,
    pub reign_score: u32,
    /// R1-1: renown earned in the act that closed, and of the house.
    pub renown: i32,
    pub house_renown: i32,
    /// R1-1: the last three facts of renown, newest first.
    pub renown_recent: Vec<String>,
    /// Objectives of the act: `(title, met)`.
    pub objectives: Vec<(String, bool)>,
    /// Pressures that held at the closing, with their points.
    pub pressures: Vec<String>,
    /// The three main causes of the act, one line each.
    pub causes_title: String,
    pub causes: Vec<String>,
    /// The next act (`None` after the last one: the final bilan follows).
    pub next_name: Option<String>,
    pub next_subtitle: String,
    pub next_intro: String,
    pub policy_prompt: String,
    pub policies: Vec<ActPolicyRow>,
}

/// The transition screen if the player has not seen the last one yet.
pub fn transition_view(data: &GameData, state: &CampaignState) -> Option<TransitionView> {
    if !state.act_transition_pending() {
        return None;
    }
    let report = state.acts.reports.last()?;
    let rules = &data.acts_rules;
    let index = rules.acts.iter().position(|a| a.id == report.act_id)?;
    let act = &rules.acts[index];
    let set = rules.objectives(
        state.player_faction(),
        &government_key(data, state),
        &act.id,
    );
    let objectives = report
        .objectives
        .iter()
        .map(|result| {
            let title = set
                .iter()
                .find(|o| o.id == result.id)
                .map_or_else(|| result.id.clone(), |o| o.title.clone());
            (title, result.met)
        })
        .collect();
    let pressures = report
        .pressures
        .iter()
        .filter(|p| p.active)
        .filter_map(|p| {
            act.pressures
                .iter()
                .find(|d| d.id == p.id)
                .map(|d| format!("{} ({:+})", d.title, p.points))
        })
        .collect();
    let causes = report
        .causes
        .iter()
        .enumerate()
        .map(|(i, record)| {
            format!(
                "{}. {}",
                i + 1,
                crate::causes::cause_view(state, data, record).one_line()
            )
        })
        .collect::<Vec<_>>();
    let reign_score = state.reign_score(data);
    let next = rules.acts.get(index + 1);
    let grade = rules.grade(reign_score);
    let number = (index + 1).to_string();
    Some(TransitionView {
        title: fill(&text(data, "transition_title"), &[("n", &number)]),
        act_name: act.name.clone(),
        ended_by: report.ended_by.clone(),
        closing: act
            .closing
            .get(&report.ended_by)
            .cloned()
            .unwrap_or_default(),
        ended_line: text(data, &format!("ended_by_{}", report.ended_by)),
        score: report.score,
        score_line: format!(
            "{} : {} / 100 — {} {}",
            act.name,
            report.score,
            text(data, "renown"),
            report.renown
        ),
        grade_line: grade.map_or_else(String::new, |g| {
            format!(
                "{} : {} — {}",
                text(data, "reign_score"),
                reign_score,
                g.title
            )
        }),
        reign_score,
        renown: report.renown,
        house_renown: state.house_renown(data),
        renown_recent: renown_recent(data, state, 3),
        objectives,
        pressures,
        causes_title: if causes.is_empty() {
            String::new()
        } else {
            text(data, "causes_title")
        },
        causes,
        next_name: next.map(|a| a.name.clone()),
        next_subtitle: next.map_or_else(String::new, |a| a.subtitle.clone()),
        next_intro: next.map_or_else(String::new, |a| a.intro.clone()),
        policy_prompt: if next.is_some() {
            text(data, "policy_prompt")
        } else {
            String::new()
        },
        policies: if next.is_some() {
            policy_rows(data, state)
        } else {
            Vec::new()
        },
    })
}

/// The player has seen the transition screen.
pub fn mark_transition_seen(state: &mut CampaignState) {
    state.mark_act_transition_seen();
}

fn government_key(data: &GameData, state: &CampaignState) -> String {
    data.factions
        .get(state.player_faction())
        .map(|f| format!("{:?}", f.government).to_lowercase())
        .unwrap_or_default()
}

/// Lines of the bilan of each act and the reign score, for the end screen.
pub fn reign_lines(data: &GameData, state: &CampaignState) -> Vec<String> {
    let rules = &data.acts_rules;
    let mut lines: Vec<String> = state
        .acts
        .reports
        .iter()
        .filter_map(|report| {
            let act = rules.acts.iter().find(|a| a.id == report.act_id)?;
            Some(format!(
                "Acte {} « {} » : {} / 100 (poids {})",
                rules.acts.iter().position(|a| a.id == act.id)? + 1,
                act.name,
                report.score,
                act.weight
            ))
        })
        .collect();
    if !state.acts.finished {
        if let Some(view) = act_view(data, state) {
            lines.push(format!(
                "Acte {} « {} » (en cours) : {} / 100",
                view.number, view.name, view.score
            ));
        }
    }
    lines
}
