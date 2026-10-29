//! The arc in three acts (ADR 0784, 0785): the act in progress, the transition screen with the
//! bilan of the act that has just closed, and the policy choice of the household. Pure views over
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

/// A policy the player may choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActPolicyRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub chosen: bool,
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
    /// Reign score as it stands (acts over + this one).
    pub reign_score: u32,
    pub objectives: Vec<ActObjectiveRow>,
    pub pressures: Vec<ActPressureRow>,
    pub policies: Vec<ActPolicyRow>,
    /// A policy is still to be chosen.
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

fn policy_rows(data: &GameData, act: usize, chosen: Option<&str>) -> Vec<ActPolicyRow> {
    data.acts_rules.acts[act]
        .policies
        .iter()
        .map(|p| ActPolicyRow {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            chosen: chosen == Some(p.id.as_str()),
        })
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
    let chosen = state.acts.policy.as_deref();
    Some(ActView {
        number: status.index + 1,
        total: rules.acts.len(),
        name: act.name.clone(),
        subtitle: act.subtitle.clone(),
        intro: act.intro.clone(),
        end_year: act.end_year,
        score: status.score,
        reign_score: state.reign_score(data, true),
        objectives,
        pressures,
        policies: policy_rows(data, status.index, chosen),
        policy_pending: !state.acts.finished && chosen.is_none(),
    })
}

/// Chooses the policy of the act in progress; `false` for an unknown policy or no act.
pub fn choose_policy(data: &GameData, state: &mut CampaignState, policy: &str) -> bool {
    state.choose_act_policy(data, policy).is_ok()
}

/// The transition screen: the bilan of the act that has just closed, and the act that begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransitionView {
    pub title: String,
    /// Name of the act that closed.
    pub act_name: String,
    /// How it closed: `year`, `peace` or `collapse`.
    pub ended_by: String,
    pub closing: String,
    pub ended_line: String,
    pub score: u32,
    pub score_line: String,
    pub grade_line: String,
    pub reign_score: u32,
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
    let reign_score = state.reign_score(data, false);
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
        score_line: format!("{} : {} / 100", act.name, report.score),
        grade_line: grade.map_or_else(String::new, |g| {
            format!(
                "{} : {} — {}",
                text(data, "reign_score"),
                reign_score,
                g.title
            )
        }),
        reign_score,
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
            policy_rows(data, index + 1, state.acts.policy.as_deref())
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
