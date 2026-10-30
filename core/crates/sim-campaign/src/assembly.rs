//! Consented taxation (ECO-R1, « L'impôt consenti »).
//!
//! The ordinary revenue (domain, rights, the standing tax) pays the court and the peace; war is
//! paid by **subsidies** that the crown asks of an **assembly** (États de langue d'oïl and d'oc,
//! Parlement, états de Bretagne, membres de Flandre). The crown *convenes* the assembly
//! ([`convene`]) for a necessity ([`NecessityKind`]) and a tier of demand; the next season the
//! estates answer by an agreement, an agreement under terms (1 to 3 concessions) or a refusal
//! ([`resolve_assemblies`]). A granted subsidy is collected **each season** for a term, spread
//! over the covered provinces (the local unrest follows the share taken, [`province_unrest`]).
//! The concessions become promises: kept, they warm the mood of the estates, broken, they chill it,
//! stir the towns and are remembered by the vote.
//!
//! Aide, gabelle and fouage are forms of subsidy; the estates can make them **permanent**
//! (ordinary taxes, [`permanent_income`]) from 1360-1363, at the price of a standing unrest.
//!
//! Everything numeric is in `data/rules/assemblies.json` ([`data_model::AssemblyRules`]). A house
//! without an assembly keeps the extraordinary levies of `finance.json`; without the file nothing
//! changes. The module exposes read-only views (`forecast`, `overview`) for the interface and the
//! AI (R1-c, R1-d).

use std::collections::BTreeMap;

use data_model::currency::MONEY;
use data_model::{
    AskTier, Assembly, Concession, ConcessionKind, FactionId, GameData, NecessityKind, ProvinceId,
    SubsidyForm,
};
use serde::{Deserialize, Serialize};

use crate::cause_builders;
use crate::causes::Cause;
use crate::coinage::CoinageLevel;
use crate::events::{EventKind, GameEvent};
use crate::finance::{Obligation, ObligationKind};
use crate::plan_cache::PlanCache;
use crate::state::CampaignState;
use crate::Order;

// ----- State ------------------------------------------------------------------------------

/// A province's part of a subsidy: what it pays each season and its ordinary tax at the time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Share {
    pub province: ProvinceId,
    /// Livres collected per season.
    pub amount: i64,
    /// Ordinary tax of the province when the subsidy was voted (the burden is `amount / base`).
    pub base: i64,
}

/// Where a session stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Convened, the estates have not answered yet.
    Pending,
    /// The estates grant the subsidy under terms; the crown has to accept or decline.
    Proposed,
    /// Voted (with or without terms): the subsidy runs.
    Granted,
    /// The estates refused.
    Refused,
    /// The crown declined the terms.
    Declined,
    /// The crown did not answer in time.
    Lapsed,
}

impl SessionStatus {
    /// The session still waits for somebody.
    pub fn is_open(self) -> bool {
        matches!(self, SessionStatus::Pending | SessionStatus::Proposed)
    }

    pub fn key(self) -> &'static str {
        match self {
            SessionStatus::Pending => "pending",
            SessionStatus::Proposed => "proposed",
            SessionStatus::Granted => "granted",
            SessionStatus::Refused => "refused",
            SessionStatus::Declined => "declined",
            SessionStatus::Lapsed => "lapsed",
        }
    }
}

/// One convocation of an assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: u32,
    pub faction: FactionId,
    pub necessity: NecessityKind,
    /// Id of the tier of demand.
    pub ask: String,
    /// Id of the form of subsidy.
    pub form: String,
    /// The crown asks the estates to make the form permanent.
    #[serde(default)]
    pub permanent: bool,
    pub convened_turn: u32,
    pub status: SessionStatus,
    /// Agreement of the estates at the vote (0 until the vote).
    #[serde(default)]
    pub accord: i32,
    /// Livres per season voted.
    #[serde(default)]
    pub per_season: i64,
    #[serde(default)]
    pub term: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shares: Vec<Share>,
    /// Concessions demanded (ids of the catalogue).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub concessions: Vec<String>,
    /// Turn of the vote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_turn: Option<u32>,
}

/// A subsidy being collected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subsidy {
    pub session: u32,
    pub faction: FactionId,
    pub form: String,
    pub seasons_left: u32,
    pub shares: Vec<Share>,
}

/// Where a promise stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromiseStatus {
    Active,
    Kept,
    Broken,
}

/// A concession the crown promised the estates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Promise {
    pub id: u32,
    pub faction: FactionId,
    /// Id of the concession.
    pub concession: String,
    pub since: u32,
    /// The promise holds until this turn.
    pub until: u32,
    /// `EndWar`: the truce is due by this turn.
    pub deadline: u32,
    pub status: PromiseStatus,
    /// Turn the promise was kept or broken.
    #[serde(default)]
    pub resolved: u32,
    /// `EndWar`: the wars to end.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wars: Vec<FactionId>,
}

/// Unrest spread on the provinces covered by an assembly until `until_turn`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssemblyGrievance {
    pub faction: FactionId,
    /// Tenths of a point (negative: the provinces are calmed).
    pub tenths: i32,
    pub until_turn: u32,
    /// The weight of the burghers' share applies (broken promises anger the towns most).
    #[serde(default)]
    pub urban: bool,
}

/// What an assembly remembers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssemblyRuntime {
    /// Mood of the estates (0-100): their trust in the crown.
    pub mood: i32,
    /// Turn of the last convocation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_convened: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<Session>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub promises: Vec<Promise>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subsidies: Vec<Subsidy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grievances: Vec<AssemblyGrievance>,
    /// Sessions convened so far.
    #[serde(default)]
    pub convened: u32,
    /// Sessions that ended in a subsidy.
    #[serde(default)]
    pub granted: u32,
    /// Sessions refused (by the estates, or whose terms the crown declined).
    #[serde(default)]
    pub refused: u32,
}

impl AssemblyRuntime {
    fn new(mood: i32) -> Self {
        AssemblyRuntime {
            mood,
            last_convened: None,
            sessions: Vec::new(),
            promises: Vec::new(),
            subsidies: Vec::new(),
            grievances: Vec::new(),
            convened: 0,
            granted: 0,
            refused: 0,
        }
    }
}

/// The assemblies of the campaign (`CampaignState::assemblies`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssemblyStates {
    /// Runtime of each assembly, by id (created at the first convocation).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub houses: BTreeMap<String, AssemblyRuntime>,
    /// Forms of subsidy made permanent, by faction.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub permanent: BTreeMap<FactionId, Vec<String>>,
    /// Identifier of the next session or promise.
    #[serde(default)]
    pub next_id: u32,
}

impl AssemblyStates {
    pub fn is_default(&self) -> bool {
        *self == AssemblyStates::default()
    }

    fn take_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }
}

/// Why an assembly order is refused (French messages for the interface).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssemblyError {
    #[error("aucune assemblée n'existe dans ces règles")]
    NoAssemblies,
    #[error("assemblée inconnue : {0}")]
    UnknownAssembly(String),
    #[error("cette assemblée n'est pas convoquée par votre maison")]
    NotYourAssembly,
    #[error("il faut tenir au moins {0} provinces de cette assemblée pour la convoquer")]
    TooFewProvinces(u32),
    #[error("l'assemblée ne peut pas être convoquée avant le tour {0}")]
    TooSoon(u32),
    #[error("une session de cette assemblée est déjà ouverte")]
    SessionOpen,
    #[error("palier de demande inconnu : {0}")]
    UnknownAsk(String),
    #[error("forme de subside inconnue : {0}")]
    UnknownForm(String),
    #[error("{0} n'existe pas encore ou n'est pas votée par cette assemblée")]
    FormNotAvailable(String),
    #[error("la nécessité invoquée ({0}) n'existe pas dans la situation du royaume")]
    NecessityNotMet(String),
    #[error("{0} ne peut pas être rendu permanent maintenant")]
    PermanentUnavailable(String),
    #[error("session inconnue")]
    UnknownSession,
    #[error("cette session n'attend pas votre réponse")]
    NotAwaitingAnswer,
}

// ----- Reading helpers ---------------------------------------------------------------------

fn rules(data: &GameData) -> &data_model::AssemblyRules {
    &data.assembly_rules
}

/// The faction has at least one assembly in the data: its extraordinary revenue is consented.
pub fn has_assembly(data: &GameData, faction: &FactionId) -> bool {
    rules(data).of_house(faction.as_str()).next().is_some()
}

/// Assemblies convened by `faction`.
pub fn assemblies_of<'a>(data: &'a GameData, faction: &FactionId) -> Vec<&'a Assembly> {
    rules(data).of_house(faction.as_str()).collect()
}

fn covers(data: &GameData, assembly: &Assembly, province: &ProvinceId) -> bool {
    assembly.all_provinces
        || assembly.provinces.iter().any(|p| p == province.as_str())
        || data
            .provinces
            .get(province)
            .is_some_and(|p| assembly.regions.contains(&p.region))
}

/// Provinces `faction` controls that the assembly speaks for.
pub fn covered_provinces(
    state: &CampaignState,
    data: &GameData,
    assembly: &Assembly,
    faction: &FactionId,
) -> Vec<ProvinceId> {
    state
        .controlled_provinces(faction)
        .filter(|p| covers(data, assembly, p))
        .cloned()
        .collect()
}

/// Mood of an assembly (its starting mood before the first convocation).
pub fn mood_of(state: &CampaignState, assembly: &Assembly) -> i32 {
    state
        .assemblies
        .houses
        .get(&assembly.id)
        .map_or(assembly.start_mood, |r| r.mood)
}

fn runtime<'a>(state: &'a CampaignState, id: &str) -> Option<&'a AssemblyRuntime> {
    state.assemblies.houses.get(id)
}

fn runtime_mut<'a>(state: &'a mut CampaignState, assembly: &Assembly) -> &'a mut AssemblyRuntime {
    state
        .assemblies
        .houses
        .entry(assembly.id.clone())
        .or_insert_with(|| AssemblyRuntime::new(assembly.start_mood))
}

fn ruler_prestige(state: &CampaignState, faction: &FactionId) -> i32 {
    state
        .factions
        .get(faction)
        .and_then(|f| f.ruler.as_ref())
        .and_then(|r| state.characters.get(r))
        .map_or(0, |c| c.prestige)
}

fn is_aggressor(state: &CampaignState, faction: &FactionId, enemy: &FactionId) -> bool {
    state
        .factions
        .get(faction)
        .and_then(|f| f.ledger.war_goals.get(enemy))
        .is_some_and(|goals| !goals.is_empty())
}

fn enemies(state: &CampaignState, faction: &FactionId) -> Vec<FactionId> {
    state.factions.get(faction).map_or(Vec::new(), |f| {
        f.at_war_with
            .iter()
            .filter(|e| !e.is_rebels())
            .cloned()
            .collect()
    })
}

fn invaded(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    let enemies = enemies(state, faction);
    if enemies.is_empty() {
        return false;
    }
    let lost = state
        .owned_provinces(faction)
        .iter()
        .any(|p| state.province_controller(p).is_some_and(|c| c != faction));
    lost || state.armies.values().any(|a| {
        enemies.contains(&a.faction)
            && state
                .army_province(data, a)
                .is_some_and(|p| state.province_owner(&p) == Some(faction))
    })
}

/// Does the situation of the realm justify the necessity?
pub fn necessity_met(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    kind: NecessityKind,
) -> bool {
    let Some(f) = state.factions.get(faction) else {
        return false;
    };
    let wars = enemies(state, faction);
    match kind {
        NecessityKind::Invasion => invaded(state, data, faction),
        NecessityKind::DefensiveWar => wars.iter().any(|e| !is_aggressor(state, faction, e)),
        NecessityKind::OffensiveWar => wars.iter().any(|e| is_aggressor(state, faction, e)),
        NecessityKind::Ransom => {
            !f.ransom_debts.is_empty()
                || f.ruler
                    .as_ref()
                    .and_then(|r| state.characters.get(r))
                    .is_some_and(|c| c.captive)
        }
        NecessityKind::Crusade => crate::crusade::is_crusader(state, data, faction),
    }
}

/// Necessities the realm can invoke now, strongest first (subsidy multiplier, then id).
pub fn available_necessities(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<NecessityKind> {
    let mut kinds: Vec<(&data_model::Necessity, NecessityKind)> = rules(data)
        .necessities
        .iter()
        .filter(|n| necessity_met(state, data, faction, n.id))
        .map(|n| (n, n.id))
        .collect();
    kinds.sort_by(|a, b| {
        b.0.multiplier
            .partial_cmp(&a.0.multiplier)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
    });
    kinds.into_iter().map(|(_, k)| k).collect()
}

fn form_available(form: &SubsidyForm, assembly: &Assembly, year: i32) -> bool {
    year >= form.from_year
        && (form.assemblies.is_empty() || form.assemblies.iter().any(|a| a == &assembly.id))
}

/// Forms of subsidy the assembly can vote this year, in file order.
pub fn available_forms<'a>(
    state: &CampaignState,
    data: &'a GameData,
    assembly: &Assembly,
) -> Vec<&'a SubsidyForm> {
    rules(data)
        .forms
        .iter()
        .filter(|f| form_available(f, assembly, state.year))
        .collect()
}

fn can_be_permanent(state: &CampaignState, form: &SubsidyForm, faction: &FactionId) -> bool {
    form.permanent_from_year.is_some_and(|y| state.year >= y)
        && !state
            .assemblies
            .permanent
            .get(faction)
            .is_some_and(|forms| forms.contains(&form.id))
}

// ----- The vote ----------------------------------------------------------------------------

/// What is asked of an assembly.
struct Request<'a> {
    necessity: NecessityKind,
    ask: &'a AskTier,
    form: &'a SubsidyForm,
    permanent: bool,
}

/// How the estates will (or did) answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteOutcome {
    Grant,
    Conditions,
    Refuse,
}

/// The result of the vote of an assembly (also the forecast shown before convening).
#[derive(Debug, Clone, PartialEq)]
pub struct Vote {
    /// Agreement of the estates (rounded).
    pub accord: i32,
    pub outcome: VoteOutcome,
    /// Livres per season voted if the estates agree.
    pub per_season: i64,
    pub term: u32,
    pub shares: Vec<Share>,
    /// Concessions demanded when `outcome` is `Conditions`.
    pub terms: Vec<String>,
    /// Parts of the agreement (key, points): mood, prestige, necessity, ask, subsidies, unrest,
    /// broken, permanent. Keys are those of the texts of the causes.
    pub parts: Vec<(&'static str, i32)>,
}

fn build_shares(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    provinces: &[ProvinceId],
    multiple: f64,
) -> Vec<Share> {
    let tax_rate = state
        .factions
        .get(faction)
        .map(|f| f.tax_rate)
        .unwrap_or_default();
    let tech = crate::research::faction_province_tech_effects(state, data, faction);
    provinces
        .iter()
        .map(|p| {
            let base = state.province_gross_income(data, p, faction, tax_rate, &tech);
            Share {
                province: p.clone(),
                amount: (base as f64 * multiple).round() as i64,
                base,
            }
        })
        .filter(|s| s.base > 0)
        .collect()
}

/// Livres per season of the running subsidies of `faction` from this assembly.
fn running_amount(state: &CampaignState, assembly: &Assembly, faction: &FactionId) -> i64 {
    runtime(state, &assembly.id).map_or(0, |r| {
        r.subsidies
            .iter()
            .filter(|s| &s.faction == faction)
            .map(|s| paid_amount(state, faction, &s.shares))
            .sum()
    })
}

fn paid_amount(state: &CampaignState, faction: &FactionId, shares: &[Share]) -> i64 {
    shares
        .iter()
        .filter(|s| state.controls_province(faction, &s.province))
        .map(|s| s.amount)
        .sum()
}

fn mean_unrest(state: &CampaignState, provinces: &[ProvinceId]) -> f64 {
    let (mut weighted, mut total) = (0.0_f64, 0.0_f64);
    for p in provinces {
        if let Some(province) = state.provinces.get(p) {
            let people = province.population.total() as f64;
            weighted += crate::population::weighted_unrest(&province.population) * people;
            total += people;
        }
    }
    if total > 0.0 {
        weighted / total
    } else {
        0.0
    }
}

fn broken_remembered(state: &CampaignState, data: &GameData, assembly: &Assembly) -> usize {
    let memory = rules(data).vote.broken_memory_seasons;
    runtime(state, &assembly.id).map_or(0, |r| {
        r.promises
            .iter()
            .filter(|p| p.status == PromiseStatus::Broken && p.resolved + memory > state.turn)
            .count()
    })
}

fn compute_vote(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    assembly: &Assembly,
    request: &Request<'_>,
) -> Vote {
    let rules = rules(data);
    let v = &rules.vote;
    let necessity = rules.necessity(request.necessity);
    let covered = covered_provinces(state, data, assembly, faction);
    let multiplier = necessity.map_or(1.0, |n| n.multiplier);
    let shares = build_shares(
        state,
        data,
        faction,
        &covered,
        request.ask.ordinary_multiple * multiplier,
    );
    let ordinary: i64 = shares.iter().map(|s| s.base).sum();
    let per_season: i64 = shares.iter().map(|s| s.amount).sum();
    let running = running_amount(state, assembly, faction);
    let running_ratio = if ordinary > 0 {
        running as f64 / ordinary as f64
    } else {
        0.0
    };
    let mut parts: Vec<(&'static str, i32)> = Vec::new();
    let mut part = |key: &'static str, value: f64| parts.push((key, value.round() as i32));
    part("assembly_mood", f64::from(mood_of(state, assembly)));
    part(
        "assembly_prestige",
        v.prestige_weight * f64::from(ruler_prestige(state, faction)),
    );
    part(
        "assembly_necessity",
        necessity.map_or(0.0, |n| n.accord_bonus),
    );
    part("assembly_ask", request.ask.accord_modifier);
    part("assembly_subsidies", -v.subsidy_weight * running_ratio);
    part(
        "assembly_unrest",
        -v.unrest_weight * mean_unrest(state, &covered),
    );
    part(
        "assembly_broken",
        -v.broken_promise_weight * broken_remembered(state, data, assembly) as f64,
    );
    if request.permanent {
        part("assembly_permanent", -v.permanent_penalty);
    }
    let accord: i32 = parts.iter().map(|(_, p)| *p).sum();
    let accord_f = f64::from(accord);
    let (outcome, terms) = if per_season <= 0 || accord_f < v.conditional_threshold {
        (VoteOutcome::Refuse, Vec::new())
    } else if accord_f >= v.grant_threshold {
        (VoteOutcome::Grant, Vec::new())
    } else {
        let wanted = (((v.grant_threshold - accord_f) / v.points_per_concession.max(0.1)).ceil()
            as u32)
            .clamp(1, v.max_concessions);
        let terms = pick_terms(state, data, faction, assembly, wanted as usize);
        if terms.is_empty() {
            (VoteOutcome::Grant, terms)
        } else {
            (VoteOutcome::Conditions, terms)
        }
    };
    Vote {
        accord,
        outcome,
        per_season,
        term: request.ask.term_seasons,
        shares,
        terms,
        parts,
    }
}

/// Is the concession both meaningful now and not already promised?
fn feasible(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    assembly: &Assembly,
    concession: &Concession,
) -> bool {
    let already = runtime(state, &assembly.id).is_some_and(|r| {
        r.promises.iter().any(|p| {
            &p.faction == faction
                && p.concession == concession.id
                && p.status == PromiseStatus::Active
        })
    });
    if already {
        return false;
    }
    match concession.kind {
        ConcessionKind::EndWar => !enemies(state, faction).is_empty(),
        ConcessionKind::DismissCounsellor => concession.office.as_ref().is_some_and(|office| {
            state
                .factions
                .get(faction)
                .is_some_and(|f| f.council.holders.contains_key(office))
        }),
        ConcessionKind::ElectedOfficers => {
            crate::finance::household_percent_uncapped(state, data, faction)
                > concession.household_cap_percent.unwrap_or(f64::MAX)
        }
        ConcessionKind::StrongMoney | ConcessionKind::Grant => true,
    }
}

/// The `count` cheapest feasible concessions among the assembly's demands (cost, then id).
fn pick_terms(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    assembly: &Assembly,
    count: usize,
) -> Vec<String> {
    let rules = rules(data);
    let mut candidates: Vec<&Concession> = assembly
        .demands
        .iter()
        .filter_map(|id| rules.concession(id))
        .filter(|c| feasible(state, data, faction, assembly, c))
        .collect();
    candidates.sort_by(|a, b| a.cost.cmp(&b.cost).then(a.id.cmp(&b.id)));
    candidates
        .into_iter()
        .take(count)
        .map(|c| c.id.clone())
        .collect()
}

// ----- Orders ------------------------------------------------------------------------------

fn find_assembly<'a>(data: &'a GameData, id: &str) -> Result<&'a Assembly, AssemblyError> {
    if !rules(data).is_active() {
        return Err(AssemblyError::NoAssemblies);
    }
    rules(data)
        .assembly(id)
        .ok_or_else(|| AssemblyError::UnknownAssembly(id.to_owned()))
}

/// Can `faction` convene the assembly now (house, provinces, delay, no open session)?
pub fn can_convene(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    assembly_id: &str,
) -> Result<(), AssemblyError> {
    let assembly = find_assembly(data, assembly_id)?;
    if assembly.house != faction.as_str() {
        return Err(AssemblyError::NotYourAssembly);
    }
    let covered = covered_provinces(state, data, assembly, faction).len() as u32;
    if covered < assembly.min_covered_provinces {
        return Err(AssemblyError::TooFewProvinces(
            assembly.min_covered_provinces,
        ));
    }
    if let Some(r) = runtime(state, assembly_id) {
        if r.sessions.iter().any(|s| s.status.is_open()) {
            return Err(AssemblyError::SessionOpen);
        }
        if let Some(last) = r.last_convened {
            let ready = last + assembly.min_gap_seasons;
            if state.turn < ready {
                return Err(AssemblyError::TooSoon(ready));
            }
        }
    }
    Ok(())
}

/// Validates the request (everything but the delay between convocations).
fn prepare<'a>(
    state: &CampaignState,
    data: &'a GameData,
    faction: &FactionId,
    assembly_id: &str,
    necessity: NecessityKind,
    ask_id: &str,
    form_id: Option<&str>,
    permanent: bool,
) -> Result<(&'a Assembly, Request<'a>), AssemblyError> {
    let assembly = find_assembly(data, assembly_id)?;
    if assembly.house != faction.as_str() {
        return Err(AssemblyError::NotYourAssembly);
    }
    let rules = rules(data);
    let ask = rules
        .ask(ask_id)
        .ok_or_else(|| AssemblyError::UnknownAsk(ask_id.to_owned()))?;
    let form = match form_id {
        Some(id) => {
            let form = rules
                .form(id)
                .ok_or_else(|| AssemblyError::UnknownForm(id.to_owned()))?;
            if !form_available(form, assembly, state.year) {
                return Err(AssemblyError::FormNotAvailable(form.name.clone()));
            }
            form
        }
        None => available_forms(state, data, assembly)
            .into_iter()
            .next()
            .ok_or_else(|| AssemblyError::FormNotAvailable("aucun subside".to_owned()))?,
    };
    if permanent && !can_be_permanent(state, form, faction) {
        return Err(AssemblyError::PermanentUnavailable(form.name.clone()));
    }
    if !necessity_met(state, data, faction, necessity) {
        let name = rules
            .necessity(necessity)
            .map_or_else(|| necessity.key().to_owned(), |n| n.name.clone());
        return Err(AssemblyError::NecessityNotMet(name));
    }
    Ok((
        assembly,
        Request {
            necessity,
            ask,
            form,
            permanent,
        },
    ))
}

/// What the estates would answer now to this request (no change to the state).
#[allow(clippy::too_many_arguments)]
pub fn forecast(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    assembly_id: &str,
    necessity: NecessityKind,
    ask_id: &str,
    form_id: Option<&str>,
    permanent: bool,
) -> Result<Vote, AssemblyError> {
    let (assembly, request) = prepare(
        state,
        data,
        faction,
        assembly_id,
        necessity,
        ask_id,
        form_id,
        permanent,
    )?;
    Ok(compute_vote(state, data, faction, assembly, &request))
}

fn player_note(state: &mut CampaignState, faction: &FactionId, event: GameEvent) {
    if faction == &state.player_faction {
        state.pending_events.push(event);
    }
}

/// `ConveneAssembly` order: the crown summons the estates. They answer the next season.
#[allow(clippy::too_many_arguments)]
pub fn convene(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    assembly_id: &str,
    necessity: NecessityKind,
    ask_id: &str,
    form_id: Option<&str>,
    permanent: bool,
) -> Result<u32, AssemblyError> {
    let (assembly, request) = prepare(
        state,
        data,
        faction,
        assembly_id,
        necessity,
        ask_id,
        form_id,
        permanent,
    )?;
    can_convene(state, data, faction, assembly_id)?;
    let (ask, form) = (request.ask.id.clone(), request.form.id.clone());
    let (necessity_name, ask_name) = (
        rules(data)
            .necessity(necessity)
            .map_or_else(String::new, |n| n.name.to_lowercase()),
        request.ask.name.to_lowercase(),
    );
    let turn = state.turn;
    let id = state.assemblies.take_id();
    let rt = runtime_mut(state, assembly);
    rt.last_convened = Some(turn);
    rt.convened += 1;
    rt.sessions.push(Session {
        id,
        faction: faction.clone(),
        necessity,
        ask,
        form,
        permanent,
        convened_turn: turn,
        status: SessionStatus::Pending,
        accord: 0,
        per_season: 0,
        term: 0,
        shares: Vec::new(),
        concessions: Vec::new(),
        answered_turn: None,
    });
    let label = data.faction_label(faction);
    player_note(
        state,
        faction,
        GameEvent::new(
            EventKind::Assembly,
            format!(
                "{label} convoque {} : {necessity_name}, {ask_name}. La réponse viendra la saison prochaine.",
                assembly.name
            ),
        )
        .faction(faction),
    );
    Ok(id)
}

/// `AnswerAssembly` order: the crown accepts or declines the terms the estates set.
pub fn answer(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    session_id: u32,
    accept_terms: bool,
) -> Result<(), AssemblyError> {
    let mut found: Option<(&Assembly, usize)> = None;
    for assembly in rules(data).of_house(faction.as_str()) {
        if let Some(index) = runtime(state, &assembly.id).and_then(|r| {
            r.sessions
                .iter()
                .position(|s| s.id == session_id && &s.faction == faction)
        }) {
            found = Some((assembly, index));
        }
    }
    let (assembly, index) = found.ok_or(AssemblyError::UnknownSession)?;
    if runtime(state, &assembly.id).map(|r| r.sessions[index].status)
        != Some(SessionStatus::Proposed)
    {
        return Err(AssemblyError::NotAwaitingAnswer);
    }
    let mut events = Vec::new();
    if accept_terms {
        activate(state, data, assembly, index, &mut events);
    } else {
        close_unaccepted(
            state,
            data,
            assembly,
            index,
            SessionStatus::Declined,
            &mut events,
        );
    }
    let player = state.player_faction.clone();
    state.pending_events.extend(
        events
            .into_iter()
            .filter(|e| e.faction.as_ref() == Some(&player)),
    );
    Ok(())
}

// ----- Resolution --------------------------------------------------------------------------

fn event(
    kind_text: String,
    faction: &FactionId,
    loss: bool,
    cause: Option<Cause>,
    player: bool,
) -> GameEvent {
    let mut e = GameEvent::new(EventKind::Assembly, kind_text).faction(faction);
    e.loss = loss;
    match cause {
        Some(c) if player => e.cause(c),
        _ => e,
    }
}

fn parts_factors(parts: &[(&'static str, i32)]) -> Vec<(&'static str, i64)> {
    parts.iter().map(|(k, v)| (*k, i64::from(*v))).collect()
}

/// The estates grant the subsidy (with the terms the crown accepted).
fn activate(
    state: &mut CampaignState,
    data: &GameData,
    assembly: &Assembly,
    index: usize,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data);
    let session = runtime(state, &assembly.id)
        .expect("session exists")
        .sessions[index]
        .clone();
    let faction = session.faction.clone();
    let Some(form) = rules.form(&session.form) else {
        return;
    };
    let player = faction == state.player_faction;
    {
        let rt = runtime_mut(state, assembly);
        rt.sessions[index].status = SessionStatus::Granted;
        rt.granted += 1;
        rt.mood = (rt.mood - rules.mood.grant_fatigue).max(0);
        if session.per_season > 0 {
            rt.subsidies.push(Subsidy {
                session: session.id,
                faction: faction.clone(),
                form: session.form.clone(),
                seasons_left: session.term,
                shares: session.shares.clone(),
            });
        }
    }
    if session.permanent {
        let forms = state
            .assemblies
            .permanent
            .entry(faction.clone())
            .or_default();
        if !forms.contains(&session.form) {
            forms.push(session.form.clone());
        }
    }
    let mut terms = Vec::new();
    for id in &session.concessions {
        if let Some(c) = rules.concession(id) {
            apply_concession(state, data, assembly, &faction, c);
            terms.push(c.name.to_lowercase());
        }
    }
    let label = data.faction_label(&faction);
    let mut text = format!(
        "{} accordent à {label} {}{} {MONEY} par saison pendant {} saisons ({}).",
        assembly.name,
        if session.permanent {
            "un subside et la permanence de la forme : "
        } else {
            ""
        },
        session.per_season,
        session.term,
        form.name.to_lowercase(),
    );
    if !terms.is_empty() {
        text.push_str(&format!(" Conditions : {}.", terms.join(", ")));
    }
    let cause = cause_builders::subsidy_granted_cause(&cause_builders::SubsidyFacts {
        faction: &faction,
        per_season: session.per_season,
        term: i64::from(session.term),
        accord: i64::from(session.accord),
        concessions: terms.len() as i64,
    });
    events.push(event(text, &faction, false, Some(cause), player));
}

/// Applies one concession the crown accepted.
fn apply_concession(
    state: &mut CampaignState,
    data: &GameData,
    assembly: &Assembly,
    faction: &FactionId,
    concession: &Concession,
) {
    let turn = state.turn;
    let until = turn + concession.duration_seasons;
    if concession.kind == ConcessionKind::DismissCounsellor {
        if let Some(office) = &concession.office {
            let _ = crate::council::dismiss(state, data, faction, office);
        }
    }
    if concession.prestige_cost > 0 {
        if let Some(ruler) = state.factions.get(faction).and_then(|f| f.ruler.clone()) {
            if let Some(c) = state.characters.get_mut(&ruler) {
                c.prestige -= concession.prestige_cost;
            }
        }
    }
    if concession.duration_seasons == 0 {
        return;
    }
    let covered = covered_provinces(state, data, assembly, faction);
    if concession.ordinary_percent > 0.0 {
        let tax_rate = state
            .factions
            .get(faction)
            .map(|f| f.tax_rate)
            .unwrap_or_default();
        let tech = crate::research::faction_province_tech_effects(state, data, faction);
        let ordinary: i64 = covered
            .iter()
            .map(|p| state.province_gross_income(data, p, faction, tax_rate, &tech))
            .sum();
        let per_season = (ordinary as f64 * concession.ordinary_percent / 100.0).round() as i64;
        if per_season > 0 {
            if let Some(f) = state.factions.get_mut(faction) {
                f.finance.obligations.push(Obligation {
                    kind: ObligationKind::Remission,
                    per_season,
                    seasons_left: concession.duration_seasons,
                    creditor: None,
                });
            }
        }
    }
    let id = state.assemblies.take_id();
    let wars = if concession.kind == ConcessionKind::EndWar {
        enemies(state, faction)
    } else {
        Vec::new()
    };
    let rt = runtime_mut(state, assembly);
    if concession.grievance_tenths != 0 {
        rt.grievances.push(AssemblyGrievance {
            faction: faction.clone(),
            tenths: concession.grievance_tenths,
            until_turn: until,
            urban: concession.grievance_tenths < 0,
        });
    }
    if matches!(
        concession.kind,
        ConcessionKind::StrongMoney | ConcessionKind::ElectedOfficers | ConcessionKind::EndWar
    ) {
        rt.promises.push(Promise {
            id,
            faction: faction.clone(),
            concession: concession.id.clone(),
            since: turn,
            until,
            deadline: turn
                + concession
                    .truce_within_seasons
                    .unwrap_or(concession.duration_seasons),
            status: PromiseStatus::Active,
            resolved: 0,
            wars,
        });
    }
}

/// The session ends without a subsidy: the estates refused, or the crown declined the terms.
fn close_unaccepted(
    state: &mut CampaignState,
    data: &GameData,
    assembly: &Assembly,
    index: usize,
    status: SessionStatus,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data);
    let (faction, accord) = {
        let s = &runtime(state, &assembly.id)
            .expect("session exists")
            .sessions[index];
        (s.faction.clone(), s.accord)
    };
    let parts = parts_of_session(state, data, assembly, index);
    {
        let rt = runtime_mut(state, assembly);
        rt.sessions[index].status = status;
        rt.refused += 1;
        rt.mood = (rt.mood - rules.mood.refusal_penalty).max(0);
    }
    let label = data.faction_label(&faction);
    let text = match status {
        SessionStatus::Refused => {
            format!("{} refusent le subside demandé par {label}.", assembly.name)
        }
        SessionStatus::Declined => format!(
            "{label} décline les conditions des {} : pas de subside.",
            assembly.name
        ),
        _ => format!(
            "{label} laisse passer le délai : {} ne votent pas de subside.",
            assembly.name
        ),
    };
    let player = faction == state.player_faction;
    let cause = cause_builders::assembly_refused_cause(&cause_builders::RefusalFacts {
        faction: &faction,
        accord: i64::from(accord),
        threshold: rules.vote.conditional_threshold.round() as i64,
        parts: parts_factors(&parts),
    });
    events.push(event(text, &faction, true, Some(cause), player));
}

/// The parts of the agreement of a session, recomputed from the stored request (best effort).
fn parts_of_session(
    state: &CampaignState,
    data: &GameData,
    assembly: &Assembly,
    index: usize,
) -> Vec<(&'static str, i32)> {
    let rules = rules(data);
    let Some(session) = runtime(state, &assembly.id).map(|r| &r.sessions[index]) else {
        return Vec::new();
    };
    let (Some(ask), Some(form)) = (rules.ask(&session.ask), rules.form(&session.form)) else {
        return Vec::new();
    };
    let request = Request {
        necessity: session.necessity,
        ask,
        form,
        permanent: session.permanent,
    };
    compute_vote(state, data, &session.faction, assembly, &request).parts
}

/// The estates vote on a session whose delay has run out.
fn vote_session(
    state: &mut CampaignState,
    data: &GameData,
    assembly: &Assembly,
    index: usize,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data);
    let session = runtime(state, &assembly.id)
        .expect("session exists")
        .sessions[index]
        .clone();
    let alive = state
        .factions
        .get(&session.faction)
        .is_some_and(|f| f.alive);
    let (Some(ask), Some(form)) = (rules.ask(&session.ask), rules.form(&session.form)) else {
        runtime_mut(state, assembly).sessions[index].status = SessionStatus::Lapsed;
        return;
    };
    let covered = covered_provinces(state, data, assembly, &session.faction).len() as u32;
    if !alive || covered < assembly.min_covered_provinces {
        runtime_mut(state, assembly).sessions[index].status = SessionStatus::Lapsed;
        return;
    }
    let request = Request {
        necessity: session.necessity,
        ask,
        form,
        permanent: session.permanent,
    };
    let vote = compute_vote(state, data, &session.faction, assembly, &request);
    let turn = state.turn;
    {
        let s = &mut runtime_mut(state, assembly).sessions[index];
        s.accord = vote.accord;
        s.per_season = vote.per_season;
        s.term = vote.term;
        s.shares = vote.shares.clone();
        s.concessions = vote.terms.clone();
        s.answered_turn = Some(turn);
    }
    match vote.outcome {
        VoteOutcome::Grant => activate(state, data, assembly, index, events),
        VoteOutcome::Refuse => {
            close_unaccepted(state, data, assembly, index, SessionStatus::Refused, events);
        }
        VoteOutcome::Conditions => {
            runtime_mut(state, assembly).sessions[index].status = SessionStatus::Proposed;
            let names: Vec<String> = vote
                .terms
                .iter()
                .filter_map(|id| rules.concession(id))
                .map(|c| c.name.to_lowercase())
                .collect();
            let label = data.faction_label(&session.faction);
            let player = session.faction == state.player_faction;
            let text = format!(
                "{} n'accordent le subside à {label} que sous conditions : {}. À vous de répondre.",
                assembly.name,
                names.join(", ")
            );
            events.push(event(text, &session.faction, false, None, player));
        }
    }
}

/// A promise is broken: the mood falls, the towns grumble, the vote will remember.
fn break_promise(
    state: &mut CampaignState,
    data: &GameData,
    assembly: &Assembly,
    index: usize,
) -> GameEvent {
    let rules = rules(data);
    let turn = state.turn;
    let (faction, concession) = {
        let p = &runtime(state, &assembly.id)
            .expect("promise exists")
            .promises[index];
        (p.faction.clone(), p.concession.clone())
    };
    let mood_before = mood_of(state, assembly);
    {
        let rt = runtime_mut(state, assembly);
        rt.promises[index].status = PromiseStatus::Broken;
        rt.promises[index].resolved = turn;
        rt.mood = (rt.mood - rules.mood.promise_broken_penalty).max(0);
        if rules.grogne.broken_promise_unrest_turns > 0 {
            rt.grievances.push(AssemblyGrievance {
                faction: faction.clone(),
                tenths: (rules.grogne.broken_promise_unrest * 10.0).round() as i32,
                until_turn: turn + rules.grogne.broken_promise_unrest_turns,
                urban: true,
            });
        }
    }
    let name = rules
        .concession(&concession)
        .map_or(concession.as_str(), |c| c.name.as_str())
        .to_lowercase();
    let label = data.faction_label(&faction);
    let cause = cause_builders::promise_broken_cause(&cause_builders::PromiseFacts {
        faction: &faction,
        mood_before: i64::from(mood_before),
        mood_loss: i64::from(rules.mood.promise_broken_penalty),
        unrest: rules.grogne.broken_promise_unrest.round() as i64,
    });
    event(
        format!(
            "{label} a trahi sa promesse aux {} ({name}) : les États s'en souviendront, les villes grondent.",
            assembly.name
        ),
        &faction,
        true,
        Some(cause),
        faction == state.player_faction,
    )
}

fn keep_promise(
    state: &mut CampaignState,
    data: &GameData,
    assembly: &Assembly,
    index: usize,
) -> GameEvent {
    let rules = rules(data);
    let turn = state.turn;
    let (faction, concession) = {
        let p = &runtime(state, &assembly.id)
            .expect("promise exists")
            .promises[index];
        (p.faction.clone(), p.concession.clone())
    };
    {
        let rt = runtime_mut(state, assembly);
        rt.promises[index].status = PromiseStatus::Kept;
        rt.promises[index].resolved = turn;
        rt.mood = (rt.mood + rules.mood.promise_kept_bonus).min(rules.mood.max);
    }
    let name = rules
        .concession(&concession)
        .map_or(concession.as_str(), |c| c.name.as_str())
        .to_lowercase();
    let label = data.faction_label(&faction);
    event(
        format!(
            "{label} a tenu sa promesse aux {} ({name}) : leur confiance grandit.",
            assembly.name
        ),
        &faction,
        false,
        None,
        faction == state.player_faction,
    )
}

fn weakening(old: CoinageLevel, new: CoinageLevel) -> bool {
    let rank = |l: CoinageLevel| match l {
        CoinageLevel::Strong => 0,
        CoinageLevel::Sound => 1,
        CoinageLevel::Debased => 2,
        CoinageLevel::HeavilyDebased => 3,
    };
    rank(new) > rank(old) && rank(new) >= 2
}

/// Hook of `coinage::set_coinage`: debasing the money breaks a promise of strong money.
pub(crate) fn on_coinage_set(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    old: CoinageLevel,
    new: CoinageLevel,
) {
    if !weakening(old, new) || state.assemblies.houses.is_empty() {
        return;
    }
    let rules = rules(data);
    let mut broken = Vec::new();
    for assembly in &rules.assemblies {
        let indices: Vec<usize> = runtime(state, &assembly.id).map_or(Vec::new(), |r| {
            r.promises
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    &p.faction == faction
                        && p.status == PromiseStatus::Active
                        && rules
                            .concession(&p.concession)
                            .is_some_and(|c| c.kind == ConcessionKind::StrongMoney)
                })
                .map(|(i, _)| i)
                .collect()
        });
        for index in indices {
            broken.push(break_promise(state, data, assembly, index));
        }
    }
    for e in broken {
        player_note(state, faction, e);
    }
}

/// Cap of the household set by promised elected officers (per cent of the gross income).
pub(crate) fn household_cap(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<f64> {
    if state.assemblies.houses.is_empty() {
        return None;
    }
    let rules = rules(data);
    state
        .assemblies
        .houses
        .values()
        .flat_map(|r| r.promises.iter())
        .filter(|p| &p.faction == faction && p.status == PromiseStatus::Active)
        .filter_map(|p| rules.concession(&p.concession))
        .filter(|c| c.kind == ConcessionKind::ElectedOfficers)
        .filter_map(|c| c.household_cap_percent)
        .reduce(f64::min)
}

/// Yield of the forms made permanent, added to the gross tax income `net`.
pub(crate) fn permanent_income(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    net: i64,
) -> i64 {
    let Some(forms) = state.assemblies.permanent.get(faction) else {
        return 0;
    };
    let percent: f64 = forms
        .iter()
        .filter_map(|id| rules(data).form(id))
        .map(|f| f.permanent_percent)
        .sum();
    (net.max(0) as f64 * percent / 100.0).round() as i64
}

fn urban_factor(state: &CampaignState, data: &GameData, province: &ProvinceId) -> f64 {
    let Some(p) = state.provinces.get(province) else {
        return 0.0;
    };
    let total = p.population.total();
    if total == 0 {
        return 0.0;
    }
    let share = p.population.burghers.count as f64 / total as f64;
    (share / rules(data).grogne.urban_full_share.max(0.001)).min(1.0)
}

fn form_weight(form: &SubsidyForm, urban: f64) -> f64 {
    form.rural_weight + (form.urban_weight - form.rural_weight) * urban
}

/// Unrest (points) the subsidies, the permanent taxes and the assemblies' grievances add to
/// `province`, held by `controller`.
pub fn province_unrest(
    state: &CampaignState,
    data: &GameData,
    province: &ProvinceId,
    controller: &FactionId,
) -> f64 {
    if state.assemblies.is_default() {
        return 0.0;
    }
    let rules = rules(data);
    let g = &rules.grogne;
    let urban = urban_factor(state, data, province);
    let mut levy = 0.0;
    for rt in state.assemblies.houses.values() {
        for subsidy in rt.subsidies.iter().filter(|s| &s.faction == controller) {
            let Some(form) = rules.form(&subsidy.form) else {
                continue;
            };
            if let Some(share) = subsidy.shares.iter().find(|s| &s.province == province) {
                let burden = share.amount as f64 / share.base.max(1) as f64;
                levy += burden * g.unrest_per_burden * form_weight(form, urban);
            }
        }
    }
    if let Some(forms) = state.assemblies.permanent.get(controller) {
        for form in forms.iter().filter_map(|id| rules.form(id)) {
            levy += form.permanent_unrest * form_weight(form, urban);
        }
    }
    let mut total = levy.min(g.max_unrest);
    for assembly in &rules.assemblies {
        let Some(rt) = runtime(state, &assembly.id) else {
            continue;
        };
        if !covers(data, assembly, province) {
            continue;
        }
        for grievance in rt.grievances.iter().filter(|x| &x.faction == controller) {
            let weight = if grievance.urban {
                0.5 + 0.5 * urban
            } else {
                1.0
            };
            total += f64::from(grievance.tenths) / 10.0 * weight;
        }
    }
    total
}

/// End of a season, before the economy books the budget: the estates vote, the subsidies are
/// collected, the promises are kept or broken, the mood drifts.
pub(crate) fn resolve_assemblies(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data);
    if !rules.is_active() {
        return;
    }
    let turn = state.turn;
    for assembly in &rules.assemblies {
        if runtime(state, &assembly.id).is_none() {
            continue;
        }
        // 1. Sessions: the estates vote; unanswered terms lapse.
        let ready: Vec<usize> = runtime(state, &assembly.id).map_or(Vec::new(), |r| {
            r.sessions
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.status == SessionStatus::Pending
                        && turn + 1 >= s.convened_turn + rules.vote.answer_delay_seasons
                })
                .map(|(i, _)| i)
                .collect()
        });
        for index in ready {
            vote_session(state, data, assembly, index, events);
        }
        let expired: Vec<usize> = runtime(state, &assembly.id).map_or(Vec::new(), |r| {
            r.sessions
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.status == SessionStatus::Proposed
                        && s.answered_turn
                            .is_some_and(|t| turn + 1 >= t + rules.vote.answer_window_seasons)
                })
                .map(|(i, _)| i)
                .collect()
        });
        for index in expired {
            close_unaccepted(state, data, assembly, index, SessionStatus::Lapsed, events);
        }
        // 2. Promises.
        let due: Vec<(usize, bool)> = runtime(state, &assembly.id).map_or(Vec::new(), |r| {
            r.promises
                .iter()
                .enumerate()
                .filter(|(_, p)| p.status == PromiseStatus::Active)
                .filter_map(|(i, p)| promise_verdict(state, data, p).map(|kept| (i, kept)))
                .collect()
        });
        for (index, kept) in due {
            let e = if kept {
                keep_promise(state, data, assembly, index)
            } else {
                break_promise(state, data, assembly, index)
            };
            events.push(e);
        }
    }
    // 3. Subsidies are collected, season after season.
    collect_subsidies(state, data);
    // 4. Mood, memory.
    for assembly in &rules.assemblies {
        let Some(rt) = state.assemblies.houses.get_mut(&assembly.id) else {
            continue;
        };
        let step = rules.mood.recovery_per_season;
        rt.mood += (rules.mood.rest - rt.mood).clamp(-step, step);
        rt.mood = rt.mood.clamp(0, rules.mood.max);
        rt.grievances.retain(|g| g.until_turn > turn);
        let memory = rules.vote.broken_memory_seasons;
        rt.promises
            .retain(|p| p.status == PromiseStatus::Active || p.resolved + memory > turn);
        let keep = rules.vote.history_sessions as usize;
        while rt.sessions.iter().filter(|s| !s.status.is_open()).count() > keep {
            if let Some(i) = rt.sessions.iter().position(|s| !s.status.is_open()) {
                rt.sessions.remove(i);
            } else {
                break;
            }
        }
    }
}

/// `Some(true)`: kept, `Some(false)`: broken, `None`: still running.
fn promise_verdict(state: &CampaignState, data: &GameData, promise: &Promise) -> Option<bool> {
    let concession = rules(data).concession(&promise.concession);
    let turn = state.turn;
    match concession.map(|c| c.kind) {
        Some(ConcessionKind::EndWar) => {
            let at_war = state.factions.get(&promise.faction).map_or(false, |f| {
                promise.wars.iter().any(|w| f.at_war_with.contains(w))
            });
            if !at_war {
                Some(true)
            } else if turn >= promise.deadline {
                Some(false)
            } else {
                None
            }
        }
        _ => (turn >= promise.until).then_some(true),
    }
}

fn collect_subsidies(state: &mut CampaignState, data: &GameData) {
    let rules = rules(data);
    for assembly in &rules.assemblies {
        let Some(rt) = state.assemblies.houses.get(&assembly.id) else {
            continue;
        };
        if rt.subsidies.is_empty() {
            continue;
        }
        let payments: Vec<(FactionId, i64)> = rt
            .subsidies
            .iter()
            .map(|s| (s.faction.clone(), paid_amount(state, &s.faction, &s.shares)))
            .collect();
        for (faction, amount) in payments {
            if let Some(f) = state.factions.get_mut(&faction) {
                f.treasury += amount;
                f.finance.ledger.levied += amount;
            }
        }
        let rt = state
            .assemblies
            .houses
            .get_mut(&assembly.id)
            .expect("checked above");
        for s in &mut rt.subsidies {
            s.seasons_left = s.seasons_left.saturating_sub(1);
        }
        rt.subsidies.retain(|s| s.seasons_left > 0);
    }
}

// ----- Views (interface, AI) ---------------------------------------------------------------

/// One assembly seen by its house.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblyView {
    pub id: String,
    pub name: String,
    pub mood: i32,
    pub covered_provinces: u32,
    /// `Ok`: can be convened now; `Err`: the reason (French).
    pub convenable: Result<(), AssemblyError>,
    pub necessities: Vec<NecessityKind>,
    pub forms: Vec<String>,
    /// Livres per season of the running subsidies.
    pub running_per_season: i64,
    pub open_session: Option<Session>,
    pub promises: Vec<Promise>,
}

/// The assemblies of `faction` with their state (the future « États et Parlement » tab).
pub fn overview(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<AssemblyView> {
    assemblies_of(data, faction)
        .into_iter()
        .map(|a| {
            let rt = runtime(state, &a.id);
            AssemblyView {
                id: a.id.clone(),
                name: a.name.clone(),
                mood: mood_of(state, a),
                covered_provinces: covered_provinces(state, data, a, faction).len() as u32,
                convenable: can_convene(state, data, faction, &a.id),
                necessities: available_necessities(state, data, faction),
                forms: available_forms(state, data, a)
                    .into_iter()
                    .map(|f| f.id.clone())
                    .collect(),
                running_per_season: running_amount(state, a, faction),
                open_session: rt
                    .and_then(|r| r.sessions.iter().find(|s| s.status.is_open()).cloned()),
                promises: rt.map_or(Vec::new(), |r| {
                    r.promises
                        .iter()
                        .filter(|p| p.status == PromiseStatus::Active)
                        .cloned()
                        .collect()
                }),
            }
        })
        .collect()
}

// ----- AI (minimal; R1-c refines it) -------------------------------------------------------

/// The assembly orders of an AI faction: answer the terms the estates set, then convene when the
/// treasury runs low under a war.
pub fn ai_assembly_orders(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let state = cache.state();
    let rules = rules(data);
    if !rules.is_active() || faction.is_rebels() {
        return Vec::new();
    }
    let Some(f) = state.factions.get(faction).filter(|f| f.alive) else {
        return Vec::new();
    };
    let mut orders = Vec::new();
    let mut convene_ready = true;
    for assembly in rules.of_house(faction.as_str()) {
        let Some(rt) = runtime(state, &assembly.id) else {
            continue;
        };
        for session in rt
            .sessions
            .iter()
            .filter(|s| &s.faction == faction && s.status == SessionStatus::Proposed)
        {
            let cost: u32 = session
                .concessions
                .iter()
                .filter_map(|id| rules.concession(id))
                .map(|c| c.cost)
                .sum();
            orders.push(Order::AnswerAssembly {
                session: session.id,
                accept_terms: cost <= rules.ai.accept_cost_up_to,
            });
        }
        if rt.sessions.iter().any(|s| s.status.is_open()) {
            convene_ready = false;
        }
    }
    if !convene_ready {
        return orders;
    }
    let gross = cache.faction_income(data, faction).max(1);
    let expected = f.treasury + (f.last_budget.receipts() - f.last_budget.upkeep()).min(0);
    if (expected as f64) >= rules.ai.convene_below_seasons * gross as f64 {
        return orders;
    }
    let Some(necessity) = available_necessities(state, data, faction)
        .into_iter()
        .next()
    else {
        return orders;
    };
    // The assembly in the best mood among those that can be convened now.
    let best = rules
        .of_house(faction.as_str())
        .filter(|a| can_convene(state, data, faction, &a.id).is_ok())
        .max_by(|a, b| {
            mood_of(state, a)
                .cmp(&mood_of(state, b))
                .then(b.id.cmp(&a.id))
        });
    if let Some(assembly) = best {
        if let Some(form) = available_forms(state, data, assembly).first() {
            orders.push(Order::ConveneAssembly {
                assembly: assembly.id.clone(),
                necessity,
                ask: rules.ai.ask.clone(),
                form: Some(form.id.clone()),
                permanent: false,
            });
        }
    }
    orders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty() {
        let s = AssemblyStates::default();
        assert!(s.is_default());
    }

    #[test]
    fn debasing_is_a_weakening_only_from_a_better_level() {
        assert!(weakening(CoinageLevel::Sound, CoinageLevel::Debased));
        assert!(weakening(
            CoinageLevel::Strong,
            CoinageLevel::HeavilyDebased
        ));
        assert!(!weakening(CoinageLevel::Debased, CoinageLevel::Sound));
        assert!(!weakening(CoinageLevel::Strong, CoinageLevel::Sound));
        assert!(!weakening(
            CoinageLevel::HeavilyDebased,
            CoinageLevel::Debased
        ));
    }
}
