//! View of the assemblies of a house (ECO-R1d, ADR 0813): the « États et Parlement » tab.
//!
//! Everything is read from `sim_campaign::assembly` (`overview`, `forecast`); the client only
//! chooses a necessity, a tier of demand and a form, shows the forecast and sends the
//! `ConveneAssembly` / `AnswerAssembly` orders. No rule here.

use data_model::{FactionId, GameData, NecessityKind};
use serde::Serialize;
use sim_campaign::assembly::{self, SessionStatus, VoteOutcome};
use sim_campaign::CampaignState;

/// A concession the estates ask (or the crown promised).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConcessionView {
    pub id: String,
    pub name: String,
    pub text: String,
    /// Points of agreement the concession is worth.
    pub cost: u32,
    pub duration_seasons: u32,
}

/// A necessity the realm can invoke now.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NecessityView {
    /// `NecessityKind::key`.
    pub key: String,
    pub name: String,
}

/// A tier of demand.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AskView {
    pub id: String,
    pub name: String,
    pub term_seasons: u32,
}

/// A form of subsidy the assembly can vote this year.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FormView {
    pub id: String,
    pub name: String,
    /// The estates could be asked to make it an ordinary tax now.
    pub permanent_possible: bool,
}

/// A session of the assembly that still waits for somebody.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionView {
    pub id: u32,
    pub necessity: String,
    pub necessity_name: String,
    pub ask_name: String,
    pub form_name: String,
    pub permanent: bool,
    pub convened_turn: u32,
    /// `pending` (the estates have not answered) or `proposed` (terms to accept or decline).
    pub status: String,
    pub accord: i32,
    pub per_season: i64,
    pub term: u32,
    pub concessions: Vec<ConcessionView>,
    /// `proposed`: last season in which the crown can still answer.
    pub answer_last_turn: Option<u32>,
}

/// A promise made to the estates and still running.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PromiseView {
    pub concession: ConcessionView,
    pub since: u32,
    /// The promise holds until this turn.
    pub until: u32,
    /// `end_war` promises: the truce is due by this turn.
    pub deadline: Option<u32>,
}

/// One assembly seen by its house.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssemblyRow {
    pub id: String,
    pub name: String,
    pub orders: Vec<String>,
    pub mood: i32,
    pub covered_provinces: u32,
    /// Why it cannot be convened now (French, from the core); `None` when it can.
    pub blocked: Option<String>,
    pub running_per_season: i64,
    pub forms: Vec<FormView>,
    pub session: Option<SessionView>,
    pub promises: Vec<PromiseView>,
}

/// The « États et Parlement » tab.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssemblyPanelView {
    pub assemblies: Vec<AssemblyRow>,
    /// Necessities of the realm now, strongest first (the same for every assembly of the house).
    pub necessities: Vec<NecessityView>,
    pub asks: Vec<AskView>,
    /// Agreement from which the estates grant, and from which they set terms.
    pub grant_threshold: i32,
    pub conditional_threshold: i32,
}

/// How the estates would answer a request.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastView {
    /// `grant`, `conditions` or `refuse`.
    pub outcome: String,
    pub accord: i32,
    pub per_season: i64,
    pub term: u32,
    /// Concessions that would be demanded.
    pub terms: Vec<ConcessionView>,
    /// Parts of the agreement: `assembly_mood`, `assembly_prestige`, `assembly_necessity`,
    /// `assembly_ask`, `assembly_subsidies`, `assembly_unrest`, `assembly_broken`,
    /// `assembly_permanent`.
    pub parts: Vec<PartView>,
}

/// One named part of the agreement.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PartView {
    pub key: String,
    pub points: i32,
}

/// The house has at least one assembly: its extraordinary revenue is consented, not levied.
pub fn has_assembly(data: &GameData, faction: &FactionId) -> bool {
    assembly::has_assembly(data, faction)
}

/// Necessity of the data from its key (`NecessityKind::key`).
pub fn necessity_of(key: &str) -> Option<NecessityKind> {
    NecessityKind::ALL.into_iter().find(|k| k.key() == key)
}

fn concession_view(data: &GameData, id: &str) -> Option<ConcessionView> {
    data.assembly_rules.concession(id).map(|c| ConcessionView {
        id: c.id.clone(),
        name: c.name.clone(),
        text: c.text.clone(),
        cost: c.cost,
        duration_seasons: c.duration_seasons,
    })
}

fn session_view(data: &GameData, session: &assembly::Session) -> SessionView {
    let rules = &data.assembly_rules;
    SessionView {
        id: session.id,
        necessity: session.necessity.key().to_owned(),
        necessity_name: rules
            .necessity(session.necessity)
            .map_or_else(|| session.necessity.key().to_owned(), |n| n.name.clone()),
        ask_name: rules
            .ask(&session.ask)
            .map_or_else(|| session.ask.clone(), |a| a.name.clone()),
        form_name: rules
            .form(&session.form)
            .map_or_else(|| session.form.clone(), |f| f.name.clone()),
        permanent: session.permanent,
        convened_turn: session.convened_turn,
        status: session.status.key().to_owned(),
        accord: session.accord,
        per_season: session.per_season,
        term: session.term,
        concessions: session
            .concessions
            .iter()
            .filter_map(|id| concession_view(data, id))
            .collect(),
        answer_last_turn: if session.status == SessionStatus::Proposed {
            session
                .answered_turn
                .map(|t| (t + rules.vote.answer_window_seasons).saturating_sub(1))
        } else {
            None
        },
    }
}

/// The assemblies of `faction`; `None` when its house has none.
pub fn assembly_panel(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> Option<AssemblyPanelView> {
    let overview = assembly::overview(state, data, faction);
    if overview.is_empty() {
        return None;
    }
    let rules = &data.assembly_rules;
    let necessities = assembly::available_necessities(state, data, faction)
        .into_iter()
        .map(|kind| NecessityView {
            key: kind.key().to_owned(),
            name: rules
                .necessity(kind)
                .map_or_else(|| kind.key().to_owned(), |n| n.name.clone()),
        })
        .collect();
    let assemblies = overview
        .into_iter()
        .map(|view| {
            let rule = rules.assembly(&view.id);
            let forms = rule.map_or_else(Vec::new, |a| {
                assembly::available_forms(state, data, a)
                    .into_iter()
                    .map(|form| FormView {
                        id: form.id.clone(),
                        name: form.name.clone(),
                        // Probed through the forecast: the core owns the rule.
                        permanent_possible: view.necessities.first().is_some_and(|kind| {
                            assembly::forecast(
                                state,
                                data,
                                faction,
                                &view.id,
                                *kind,
                                rules.asks.first().map_or("", |ask| ask.id.as_str()),
                                Some(&form.id),
                                true,
                            )
                            .is_ok()
                        }),
                    })
                    .collect()
            });
            AssemblyRow {
                id: view.id.clone(),
                name: view.name.clone(),
                orders: rule.map_or_else(Vec::new, |a| a.orders.clone()),
                mood: view.mood,
                covered_provinces: view.covered_provinces,
                blocked: view.convenable.err().map(|e| e.to_string()),
                running_per_season: view.running_per_season,
                forms,
                session: view
                    .open_session
                    .as_ref()
                    .map(|session| session_view(data, session)),
                promises: view
                    .promises
                    .iter()
                    .filter_map(|p| {
                        Some(PromiseView {
                            concession: concession_view(data, &p.concession)?,
                            since: p.since,
                            until: p.until,
                            deadline: (p.deadline > 0).then_some(p.deadline),
                        })
                    })
                    .collect(),
            }
        })
        .collect();
    Some(AssemblyPanelView {
        assemblies,
        necessities,
        asks: rules
            .asks
            .iter()
            .map(|a| AskView {
                id: a.id.clone(),
                name: a.name.clone(),
                term_seasons: a.term_seasons,
            })
            .collect(),
        grant_threshold: rules.vote.grant_threshold.round() as i32,
        conditional_threshold: rules.vote.conditional_threshold.round() as i32,
    })
}

/// What the estates would answer now (no change to the state); `Err`: the French reason the
/// request is refused (necessity absent, form unavailable...).
#[allow(clippy::too_many_arguments)]
pub fn forecast_view(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
    assembly_id: &str,
    necessity: NecessityKind,
    ask: &str,
    form: Option<&str>,
    permanent: bool,
) -> Result<ForecastView, String> {
    let vote = assembly::forecast(
        state,
        data,
        faction,
        assembly_id,
        necessity,
        ask,
        form,
        permanent,
    )
    .map_err(|e| e.to_string())?;
    Ok(ForecastView {
        outcome: match vote.outcome {
            VoteOutcome::Grant => "grant",
            VoteOutcome::Conditions => "conditions",
            VoteOutcome::Refuse => "refuse",
        }
        .to_owned(),
        accord: vote.accord,
        per_season: vote.per_season,
        term: vote.term,
        terms: vote
            .terms
            .iter()
            .filter_map(|id| concession_view(data, id))
            .collect(),
        parts: vote
            .parts
            .iter()
            .map(|(key, points)| PartView {
                key: (*key).to_owned(),
                points: *points,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn france_has_two_assemblies_and_a_forecast_for_a_war() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 5);
        let france = FactionId::new("fac_france").unwrap();
        assert!(has_assembly(data, &france));
        let view = assembly_panel(data, &state, &france).expect("france has estates");
        assert_eq!(view.assemblies.len(), 2);
        assert!(view.assemblies.iter().all(|a| a.mood > 0));
        assert!(!view.asks.is_empty());
        let burgundy = FactionId::new("fac_burgundy").unwrap();
        assert!(assembly_panel(data, &state, &burgundy).is_none());
        if let Some(first) = view.necessities.first() {
            let kind = necessity_of(&first.key).expect("known key");
            let forecast = forecast_view(
                data,
                &state,
                &france,
                &view.assemblies[0].id,
                kind,
                &view.asks[0].id,
                None,
                false,
            )
            .expect("forecast");
            assert!(!forecast.parts.is_empty());
            assert!(["grant", "conditions", "refuse"].contains(&forecast.outcome.as_str()));
        }
    }
}
