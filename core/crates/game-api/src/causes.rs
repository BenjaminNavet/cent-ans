//! « Chroniques et causes » (ADR 0748): views of the core's journal of causes.
//!
//! The core records keys and numbers; the French sentences come from `data/ui/causes.fr.json`
//! (schema `causes.schema.json`). This module formats them from the player's point of view
//! (loss, gain, other), lists the journal (the player's realm or all of Europe), explains one
//! notification by its `cause_id`, and ranks the « three main causes » of an ended campaign or
//! of a battle. The client only draws the result.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use data_model::GameData;
use serde::{Deserialize, Serialize};
use sim_campaign::causes::{Cause, CauseKind, CauseRecord};
use sim_campaign::CampaignState;

const TEXTS_JSON: &str = include_str!("../../../../data/ui/causes.fr.json");

/// Entries of the « Causes » list, most recent first.
pub const LIST_LIMIT: usize = 60;
/// Factors shown by line of a summary (tooltip, end screen).
pub const SUMMARY_FACTORS: usize = 3;

#[derive(Debug, Deserialize)]
struct KindTexts {
    loss: Option<String>,
    gain: Option<String>,
    other: String,
}

#[derive(Debug, Deserialize)]
struct Texts {
    ui: BTreeMap<String, String>,
    kinds: BTreeMap<String, KindTexts>,
    factors: BTreeMap<String, String>,
    reasons: BTreeMap<String, String>,
    /// Labels of the terms under the figures of the top bar.
    #[serde(default)]
    terms: BTreeMap<String, String>,
}

/// French label of a term under a figure of the top bar: the core's own when it has one
/// (building names), else the text of `causes.fr.json`, else the key.
pub fn term_label(key: &str, label: &str) -> String {
    if !label.is_empty() {
        return label.to_owned();
    }
    texts()
        .terms
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

/// Minus sign of the French typography (U+2212).
const MINUS: char = '\u{2212}';

/// « +1 200 » / « −2 140 » (narrow no-break space between thousands); « 0 » for zero.
pub fn signed_amount(value: i64) -> String {
    let digits = sim_campaign::advisor::fr_number(value.abs());
    match value {
        0 => "0".to_owned(),
        v if v > 0 => format!("+{digits}"),
        _ => format!("{MINUS}{digits}"),
    }
}

/// « ▲ +1 200 » / « ▼ −2 140 » / « = » (the glyphs are the ones the top bar already uses).
pub fn arrow_amount(value: i64) -> String {
    match value {
        0 => "=".to_owned(),
        v if v > 0 => format!("▲ {}", signed_amount(v)),
        v => format!("▼ {}", signed_amount(v)),
    }
}

fn texts() -> &'static Texts {
    static TEXTS: OnceLock<Texts> = OnceLock::new();
    TEXTS.get_or_init(|| serde_json::from_str(TEXTS_JSON).expect("data/ui/causes.fr.json parses"))
}

/// A UI label of `causes.fr.json` (`ui` block); the key itself when missing.
pub fn ui_text(key: &str) -> String {
    texts()
        .ui
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

/// Whose side a cause is on, from the player's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Perspective {
    /// The player is the faction hit.
    Loss,
    /// The player is the faction acting (and not the one hit).
    Gain,
    /// Someone else's affair.
    Other,
}

/// Which records the list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CauseFilter {
    /// Those that concern the player's realm.
    Mine,
    /// All of Europe.
    All,
}

/// One factor, with its sentence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FactorView {
    pub key: String,
    pub text: String,
    pub weight: u8,
}

/// One recorded cause, ready to draw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CauseView {
    pub id: u32,
    pub kind: String,
    pub perspective: Perspective,
    /// « 12 Printemps 1340 »-style date (season and year).
    pub date: String,
    pub turn: u32,
    pub severity: u8,
    /// Province of the event, if any (identifier, for the « go to » button).
    pub province: Option<String>,
    /// The headline sentence.
    pub summary: String,
    /// Factors, heaviest first.
    pub factors: Vec<FactorView>,
    /// The AI's named reason, as a sentence.
    pub reason: Option<String>,
}

impl CauseView {
    /// The summary and its heaviest factors on one line (tooltips).
    pub fn one_line(&self) -> String {
        let top: Vec<&str> = self
            .factors
            .iter()
            .take(SUMMARY_FACTORS)
            .map(|f| f.text.as_str())
            .collect();
        if top.is_empty() {
            self.summary.clone()
        } else {
            format!("{} ({})", self.summary, top.join(" ; "))
        }
    }
}

fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .fold(template.to_owned(), |text, (key, value)| {
            text.replace(&format!("{{{key}}}"), value)
        })
        .replace("{money}", &data_model::currency::plural())
}

/// « de X » with elision (« d’Angleterre »); empty when there is no name.
fn optional_de(name: &str) -> String {
    if name.is_empty() {
        String::new()
    } else {
        sim_campaign::events::de(name)
    }
}

fn perspective(cause: &Cause, player: &data_model::FactionId) -> Perspective {
    if cause.subject.as_ref() == Some(player) {
        Perspective::Loss
    } else if cause.actor.as_ref() == Some(player) {
        Perspective::Gain
    } else {
        Perspective::Other
    }
}

/// The sentence of one factor: `{value}` is the raw number, `{amount}` the signed amount in
/// French typography. A key `attitude:<label>` carries its own label.
fn factor_text(key: &str, value: i64) -> String {
    if let Some(label) = key.strip_prefix(sim_campaign::cause_builders::LABELLED_PREFIX) {
        return format!("{label} ({})", signed_amount(value));
    }
    fill(
        texts().factors.get(key).map_or(key, String::as_str),
        &[
            ("value", &value.to_string()),
            ("amount", &signed_amount(value)),
        ],
    )
}

/// View of one record.
pub fn cause_view(state: &CampaignState, data: &GameData, record: &CauseRecord) -> CauseView {
    let t = texts();
    let cause = &record.cause;
    let side = perspective(cause, state.player_faction());
    let kind = cause.kind.as_str();
    let summary_template = t.kinds.get(kind).map(|k| match side {
        Perspective::Loss => k.loss.as_deref().unwrap_or(&k.other),
        Perspective::Gain => k.gain.as_deref().unwrap_or(&k.other),
        Perspective::Other => k.other.as_str(),
    });
    let place = cause
        .province
        .as_ref()
        .map(|p| data.province_name(p))
        .unwrap_or_default();
    let subject = cause
        .subject
        .as_ref()
        .map(|f| data.faction_name(f))
        .unwrap_or_default();
    let actor = cause
        .actor
        .as_ref()
        .map(|f| data.faction_name(f))
        .unwrap_or_default();
    let summary = fill(
        summary_template.unwrap_or(kind),
        &[
            ("lieu", &place),
            ("sujet", &subject),
            ("acteur", &actor),
            ("de_lieu", &optional_de(&place)),
            ("de_sujet", &optional_de(&subject)),
            ("de_acteur", &optional_de(&actor)),
        ],
    );
    let factors = cause
        .factors
        .iter()
        .map(|f| FactorView {
            key: f.key.clone(),
            text: factor_text(&f.key, f.value),
            weight: f.weight,
        })
        .collect();
    let reason = cause.reason.as_ref().map(|r| {
        let sentence = fill(
            t.reasons.get(&r.key).map_or(r.key.as_str(), String::as_str),
            &[
                ("lieu", &place),
                ("de_lieu", &optional_de(&place)),
                ("acteur", &actor),
                ("sujet", &subject),
            ],
        );
        let line = fill(&ui_text("reason_line"), &[("reason", &sentence)]);
        // ADR 0782: the doctrine the AI followed, named by the catalogue.
        match r
            .doctrine
            .as_ref()
            .and_then(|id| data_model::StrategicDoctrines::bundled().doctrines.get(id))
        {
            Some(doctrine) => format!(
                "{line} {}",
                fill(&ui_text("doctrine_line"), &[("doctrine", &doctrine.label)])
            ),
            None => line,
        }
    });
    CauseView {
        id: record.id,
        kind: kind.to_owned(),
        perspective: side,
        date: fill(
            &ui_text("date_line"),
            &[
                ("season", record.season.label_fr()),
                ("year", &record.year.to_string()),
            ],
        ),
        turn: record.turn,
        severity: cause.severity,
        province: cause.province.as_ref().map(ToString::to_string),
        summary,
        factors,
        reason,
    }
}

/// The journal, most recent first, filtered, at most `limit` entries.
pub fn causes_view(
    state: &CampaignState,
    data: &GameData,
    filter: CauseFilter,
    limit: usize,
) -> Vec<CauseView> {
    let player = state.player_faction();
    state
        .cause_journal()
        .records()
        .rev()
        .filter(|r| filter == CauseFilter::All || r.cause.concerns(player))
        .take(limit)
        .map(|r| cause_view(state, data, r))
        .collect()
}

/// The cause a notification points to (`GameEvent::cause_id`).
pub fn cause_of(state: &CampaignState, data: &GameData, id: u32) -> Option<CauseView> {
    state
        .cause_journal()
        .get(id)
        .map(|r| cause_view(state, data, r))
}

/// Tooltip of a notification carrying `cause_id`: the cause in one line.
pub fn notification_tooltip(state: &CampaignState, data: &GameData, id: u32) -> Option<String> {
    let view = cause_of(state, data, id)?;
    Some(fill(
        &ui_text("notification_tooltip"),
        &[("summary", &view.one_line())],
    ))
}

/// Whether a record explains a setback (`false`) or a success (`true`) of the player, and
/// counts among the main causes of the end of the campaign.
fn explains(cause: &Cause, player: &data_model::FactionId, favourable: bool) -> bool {
    match cause.kind {
        CauseKind::GameWon
        | CauseKind::GameLost
        | CauseKind::UnrestWarning
        // XP-POURQUOI: swings explain the season's story, not the end of the campaign.
        | CauseKind::TreasurySwing
        | CauseKind::UnrestSwing
        | CauseKind::AttitudeShift
        | CauseKind::ExtraordinaryTax
        | CauseKind::Debasement => false,
        CauseKind::Battle | CauseKind::PlaceTaken => {
            if favourable {
                cause.actor.as_ref() == Some(player)
            } else {
                cause.subject.as_ref() == Some(player)
            }
        }
        // Wars and betrayals suffered are setbacks; those the player wages are not causes of
        // his victory.
        CauseKind::WarDeclared | CauseKind::AllianceBroken => {
            !favourable && cause.subject.as_ref() == Some(player)
        }
        CauseKind::Bankruptcy
        | CauseKind::RoyalDefault
        | CauseKind::Revolt
        | CauseKind::Succession => !favourable && cause.subject.as_ref() == Some(player),
    }
}

/// The `n` main causes of a setback (`favourable == false`: defeat) or of a success: the
/// heaviest records of the player, at most two of a kind while other kinds can fill the list.
pub fn top_causes(
    state: &CampaignState,
    data: &GameData,
    favourable: bool,
    n: usize,
) -> Vec<CauseView> {
    let player = state.player_faction();
    let mut records: Vec<&CauseRecord> = state
        .cause_journal()
        .records()
        .filter(|r| explains(&r.cause, player, favourable))
        .collect();
    records.sort_by(|a, b| {
        b.cause
            .severity
            .cmp(&a.cause.severity)
            .then_with(|| b.id.cmp(&a.id))
    });
    let mut per_kind: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut chosen = Vec::new();
    let mut skipped = Vec::new();
    for record in records {
        let count = per_kind.entry(record.cause.kind.as_str()).or_default();
        if *count >= 2 {
            skipped.push(record);
            continue;
        }
        *count += 1;
        chosen.push(record);
    }
    // Too few kinds to fill the list: the heaviest of the rest complete it.
    chosen.extend(skipped);
    chosen.truncate(n);
    chosen.sort_by(|a, b| {
        b.cause
            .severity
            .cmp(&a.cause.severity)
            .then_with(|| b.id.cmp(&a.id))
    });
    chosen
        .into_iter()
        .map(|record| cause_view(state, data, record))
        .collect()
}

/// Seasons back, from the one just resolved, that may complete the « Pourquoi » block when
/// the last season alone explains too little.
pub const STORY_LOOKBACK: u32 = 4;

/// One line of « Ce qui a changé » or « Ce qui vient »: a figure and its change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoryLine {
    /// `treasury`, `net_income`, `men`, `prestige`, `unrest`.
    pub figure: String,
    pub label: String,
    pub delta: i64,
    /// « Trésor : ▼ −2 140 ».
    pub text: String,
}

/// The renown the house earned in the season just resolved, with its terms (ADR 0805).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActDelta {
    /// 1-based number of the act in progress.
    pub act_number: usize,
    /// Renown gained over the season (sum of `terms`).
    pub renown_gained: i32,
    /// Facts of renown, newest first, one line each (« Bataille gagnée +3 (Angleterre) »).
    pub terms: Vec<String>,
    /// « Acte 1 : +4 points de renom cette saison. » (empty without any fact).
    pub text: String,
}

/// The season in three blocks (XP-POURQUOI, R4): what changed, why, what comes. API shared by
/// the « Récit de saison » panel and the Audience (XP-AUDIENCE reads `why` for its causes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SeasonStory {
    /// Turn the story is told at (the new season).
    pub turn: u32,
    pub date_label: String,
    /// The figures that moved since last season, heaviest relative change first.
    pub changed: Vec<StoryLine>,
    /// The causes of plus fort effet, weight decreasing (`story_causes` at most).
    pub why: Vec<CauseView>,
    /// The forecast of the coming season.
    pub next: Vec<StoryLine>,
    /// Renown earned over the season just resolved.
    pub act_delta: Option<ActDelta>,
}

fn figure_label(figure: &str) -> String {
    ui_text(&format!("figure_{figure}"))
}

fn story_line(figure: &str, delta: i64) -> StoryLine {
    let label = figure_label(figure);
    StoryLine {
        figure: figure.to_owned(),
        text: format!("{label} : {}", arrow_amount(delta)),
        label,
        delta,
    }
}

/// The records that concern the player and were noted in the last seasons, heaviest first.
/// At most `n`, at most two of a kind while other kinds can fill the list, those of the last
/// resolved season first and older ones (up to [`STORY_LOOKBACK`] seasons back) only to
/// complete the list.
pub fn season_causes(state: &CampaignState, data: &GameData, n: usize) -> Vec<CauseView> {
    let player = state.player_faction();
    let turn = state.turn();
    let mut recent: Vec<&CauseRecord> = Vec::new();
    let mut older: Vec<&CauseRecord> = Vec::new();
    for record in state.cause_journal().records() {
        if !record.cause.concerns(player) || record.cause.kind == CauseKind::UnrestWarning {
            continue;
        }
        if record.turn >= turn {
            recent.push(record);
        } else if record.turn + STORY_LOOKBACK >= turn {
            older.push(record);
        }
    }
    let rank = |records: &mut Vec<&CauseRecord>| {
        records.sort_by(|a, b| {
            b.cause
                .severity
                .cmp(&a.cause.severity)
                .then_with(|| b.id.cmp(&a.id))
        });
    };
    rank(&mut recent);
    rank(&mut older);
    let mut per_kind: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut chosen: Vec<&CauseRecord> = Vec::new();
    let mut skipped: Vec<&CauseRecord> = Vec::new();
    for record in recent.into_iter().chain(older) {
        let count = per_kind.entry(record.cause.kind.as_str()).or_default();
        if *count >= 2 {
            skipped.push(record);
        } else {
            *count += 1;
            chosen.push(record);
        }
    }
    chosen.extend(skipped);
    chosen.truncate(n);
    // The last season first, then the weight (the order of `chosen` already is: stable sort).
    chosen.sort_by(|a, b| {
        (b.turn >= turn)
            .cmp(&(a.turn >= turn))
            .then_with(|| b.cause.severity.cmp(&a.cause.severity))
            .then_with(|| b.id.cmp(&a.id))
    });
    chosen
        .into_iter()
        .map(|record| cause_view(state, data, record))
        .collect()
}

/// The renown of the season just resolved.
fn act_delta(data: &GameData, state: &CampaignState) -> Option<ActDelta> {
    let number = crate::acts::act_view(data, state)?.number;
    let last = state.turn().checked_sub(1)?;
    let facts: Vec<&sim_campaign::acts::RenownFact> = state
        .acts
        .renown
        .journal
        .iter()
        .rev()
        .filter(|fact| fact.turn == last)
        .collect();
    let gained: i32 = facts.iter().map(|fact| fact.points).sum();
    let text = if facts.is_empty() {
        String::new()
    } else {
        fill(
            &ui_text("story_act_line"),
            &[
                ("act", &number.to_string()),
                ("delta", &signed_amount(i64::from(gained))),
            ],
        )
    };
    Some(ActDelta {
        act_number: number,
        renown_gained: gained,
        terms: facts
            .iter()
            .map(|fact| crate::acts::renown_line(data, fact))
            .collect(),
        text,
    })
}

/// The story of the season just resolved, for `faction` (the player). Computes the forecast
/// itself (it simulates the next turn); use [`season_story_with`] to pass a cached one.
pub fn season_story(
    data: &GameData,
    state: &CampaignState,
    faction: &data_model::FactionId,
) -> SeasonStory {
    let forecast = crate::hud::forecast(data, state, faction);
    season_story_with(data, state, faction, forecast.as_ref())
}

/// [`season_story`] with the forecast the caller already has (or none: `next` is then empty).
pub fn season_story_with(
    data: &GameData,
    state: &CampaignState,
    faction: &data_model::FactionId,
    forecast: Option<&crate::hud::ForecastView>,
) -> SeasonStory {
    let mut changed: Vec<StoryLine> = crate::hud::figure_trends(data, state, faction)
        .into_iter()
        .filter_map(|trend| {
            trend
                .delta_since_last_season
                .filter(|delta| *delta != 0)
                .map(|delta| story_line(&trend.figure, delta))
        })
        .collect();
    changed.sort_by(|a, b| {
        b.delta
            .abs()
            .cmp(&a.delta.abs())
            .then_with(|| a.figure.cmp(&b.figure))
    });
    let mut next = Vec::new();
    if let Some(economy) = state.faction_economy(data, faction) {
        next.push(story_line("net_income", economy.net_income()));
    }
    if let Some(forecast) = forecast {
        for (figure, gauge) in [
            ("men", &forecast.men),
            ("prestige", &forecast.prestige),
            ("unrest", &forecast.unrest),
        ] {
            if gauge.delta != 0 {
                next.push(story_line(figure, gauge.delta));
            }
        }
    }
    next.retain(|line| line.delta != 0);
    SeasonStory {
        turn: state.turn(),
        date_label: state.date_label(),
        changed,
        why: season_causes(state, data, data.cause_thresholds.story_causes as usize),
        next,
        act_delta: act_delta(data, state),
    }
}

/// The causes of the latest battle the player fought (`Battle` records of the newest turn
/// that has one concerning him): the end-of-battle panel. Empty if none is recorded.
pub fn last_battle_causes(state: &CampaignState, data: &GameData) -> Vec<CauseView> {
    let player = state.player_faction();
    let newest = state
        .cause_journal()
        .records()
        .rev()
        .find(|r| r.cause.kind == CauseKind::Battle && r.cause.concerns(player));
    newest
        .map(|r| vec![cause_view(state, data, r)])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_model::test_support::{fac, game_data};
    use sim_campaign::cause_builders as build;
    use sim_campaign::test_support::start_quiet;

    fn every_key_resolves(cause: &Cause) {
        let t = texts();
        for factor in &cause.factors {
            if factor.key.starts_with(build::LABELLED_PREFIX) {
                continue;
            }
            assert!(
                t.factors.contains_key(&factor.key),
                "missing factor text: {}",
                factor.key
            );
        }
        if let Some(reason) = &cause.reason {
            assert!(
                t.reasons.contains_key(&reason.key),
                "missing reason text: {}",
                reason.key
            );
        }
        assert!(
            t.kinds.contains_key(cause.kind.as_str()),
            "missing kind text: {}",
            cause.kind.as_str()
        );
    }

    #[test]
    fn every_key_the_core_can_emit_has_a_text() {
        let (a, b) = (fac("fac_england"), fac("fac_france"));
        let province = data_model::ProvinceId::new("prov_picardie").unwrap();
        every_key_resolves(&build::battle_cause(&build::BattleFacts {
            winner: &a,
            loser: &b,
            province: &province,
            winner_strength: 1000,
            loser_strength: 500,
            loser_losses: 400,
            winner_power: 200.0,
            loser_power: 100.0,
            loser_general_lost: true,
            loser_routed: true,
            loser_morale_delta: -20,
            winner_defended: true,
            rough_terrain: Some("hills"),
        }));
        for terrain in ["hills", "forest", "mountains", "marsh", "heath", "bocage"] {
            let cause = build::battle_cause(&build::BattleFacts {
                winner: &a,
                loser: &b,
                province: &province,
                winner_strength: 1,
                loser_strength: 1,
                loser_losses: 1,
                winner_power: 0.0,
                loser_power: 0.0,
                loser_general_lost: false,
                loser_routed: false,
                loser_morale_delta: -3,
                winner_defended: true,
                rough_terrain: Some(terrain),
            });
            every_key_resolves(&cause);
        }
        every_key_resolves(&build::place_cause(&build::PlaceFacts {
            previous: &b,
            new_holder: &a,
            province: &province,
            is_city: true,
            siege: Some((3, 0, 80)),
            garrison: 100,
            besiegers: 900,
        }));
        every_key_resolves(&build::place_cause(&build::PlaceFacts {
            previous: &b,
            new_holder: &a,
            province: &province,
            is_city: false,
            siege: None,
            garrison: 0,
            besiegers: 0,
        }));
        every_key_resolves(&build::bankruptcy_cause(&build::BankruptcyFacts {
            faction: &b,
            treasury: -5,
            income: 1,
            army_upkeep: 10,
            army_surcharge: 2,
            building_upkeep: 3,
            administration: 4,
            table: 5,
            household: 0,
            wages: 0,
            siege: 0,
            obligations: 0,
            interest: 0,
            deficit_seasons: 3,
        }));
        let terms: Vec<(&str, f64)> = [
            "tax",
            "devastation",
            "goods",
            "health",
            "garrison",
            "occupation",
            "foreign_religion",
            "disorder",
            "buildings",
            "loyalty",
        ]
        .iter()
        .map(|k| (*k, 9.0))
        .collect();
        every_key_resolves(&build::unrest_cause(
            CauseKind::Revolt,
            &b,
            &province,
            100,
            &terms,
        ));
        for reason in [
            "ultimatum_refused",
            "independence",
            "ally_war",
            "crown_claim",
            "opportunism",
            "ambition",
            "revenge",
            "revenge_at",
        ] {
            every_key_resolves(&build::war_cause(&build::WarFacts {
                attacker: &a,
                target: &b,
                casus_belli: false,
                truce_broken: true,
                pact_broken: true,
                sworn_bond: true,
                attacker_power: 300.0,
                target_power: 100.0,
                reason: Some(reason),
            }));
        }
        every_key_resolves(&build::war_cause(&build::WarFacts {
            attacker: &a,
            target: &b,
            casus_belli: true,
            truce_broken: false,
            pact_broken: false,
            sworn_bond: false,
            attacker_power: 100.0,
            target_power: 300.0,
            reason: None,
        }));
        every_key_resolves(
            &build::alliance_cause(&a, &b, 30, Some("realignment")).factor("call_refused", 1, 80),
        );
        every_key_resolves(&build::succession_cause(&build::SuccessionFacts {
            faction: &b,
            dead_ruler_age: Some(60),
            designated_heir: false,
            contested: true,
            successor_age: Some(7),
            successor_minor: true,
            line_extinct: true,
            elective: true,
        }));
        for kind in [CauseKind::GameWon, CauseKind::GameLost] {
            every_key_resolves(&build::game_end_cause(kind, &b, 0, (1, 4), 50));
            every_key_resolves(&build::game_end_cause(kind, &b, 5, (1, 4), 50));
        }
    }

    #[test]
    fn summaries_elide_and_never_agree_with_a_place_name() {
        let t = texts();
        for (kind, k) in &t.kinds {
            for text in [k.loss.as_deref(), k.gain.as_deref(), Some(k.other.as_str())]
                .into_iter()
                .flatten()
            {
                assert!(!text.contains("de {"), "{kind}: unelided « de » in {text}");
                assert!(
                    !text.contains("perdue"),
                    "{kind}: feminine agreement: {text}"
                );
                assert!(!text.contains('\''), "{kind}: straight apostrophe: {text}");
            }
        }
        let filled = fill(
            "Perte {de_lieu}, au profit {de_acteur}",
            &[
                ("de_lieu", &optional_de("Agenais")),
                ("de_acteur", &optional_de("Angleterre")),
            ],
        );
        assert_eq!(filled, "Perte d’Agenais, au profit d’Angleterre");
        assert_eq!(optional_de("Écosse"), "d’Écosse");
        assert_eq!(optional_de("Aragon"), "d’Aragon");
        assert_eq!(optional_de("Albret"), "d’Albret");
        assert_eq!(optional_de("Bourgogne"), "de Bourgogne");
    }

    #[test]
    fn a_bankruptcy_reads_in_french_with_its_heaviest_line_first() {
        let data = game_data();
        let france = fac("fac_france");
        let mut state = start_quiet(data, "fac_france", 5);
        state.factions.get_mut(&france).unwrap().treasury = -10_000_000;
        state.end_turn_with(data, sim_campaign::test_support::idle);
        let list = causes_view(&state, data, CauseFilter::Mine, LIST_LIMIT);
        let view = list
            .iter()
            .find(|v| v.kind == "bankruptcy")
            .expect("bankruptcy listed");
        assert_eq!(view.summary, "Banqueroute : le trésor est vide");
        assert_eq!(view.perspective, Perspective::Loss);
        assert!(
            view.factors[0].text.contains("livres"),
            "{:?}",
            view.factors
        );
        assert!(view.date.contains("1337") || view.date.contains("1338"));
        let tooltip = notification_tooltip(&state, data, view.id).unwrap();
        assert!(tooltip.starts_with("Cause : Banqueroute"));
    }

    #[test]
    fn top_causes_are_varied_and_capped() {
        let data = game_data();
        let mut state = start_quiet(data, "fac_france", 5);
        let (france, england) = (fac("fac_france"), fac("fac_england"));
        let province = data_model::ProvinceId::new("prov_picardie").unwrap();
        for i in 0..5 {
            let cause = build::battle_cause(&build::BattleFacts {
                winner: &england,
                loser: &france,
                province: &province,
                winner_strength: 1000,
                loser_strength: 1000,
                loser_losses: 100 + i * 100,
                winner_power: 0.0,
                loser_power: 0.0,
                loser_general_lost: false,
                loser_routed: false,
                loser_morale_delta: 0,
                winner_defended: false,
                rough_terrain: None,
            });
            state.note_cause(cause);
        }
        state.note_cause(build::game_end_cause(
            CauseKind::GameLost,
            &france,
            0,
            (0, 3),
            10,
        ));
        let top = top_causes(&state, data, false, 3);
        assert_eq!(top.len(), 3, "the list is completed when kinds run out");
        assert!(top[0].severity >= top[1].severity);
        assert!(top.iter().all(|v| v.kind == "battle"));
        assert!(top_causes(&state, data, true, 3).is_empty());
    }

    #[test]
    fn swing_causes_have_a_text_for_every_key() {
        let (a, b) = (fac("fac_england"), fac("fac_france"));
        let province = data_model::ProvinceId::new("prov_picardie").unwrap();
        let terms: Vec<(&str, i64)> = [
            "receipts",
            "armies",
            "buildings",
            "administration",
            "table",
            "household",
            "wages",
            "siege",
            "obligations",
            "interest",
            "levies",
            "loans",
            "other",
        ]
        .iter()
        .map(|k| (*k, -100))
        .collect();
        every_key_resolves(&build::treasury_swing_cause(&a, 1000, -600, &terms));
        every_key_resolves(&build::unrest_swing_cause(
            &a,
            &province,
            10.0,
            40.0,
            &[("tax", 9.0), ("goods", 4.0)],
        ));
        every_key_resolves(&build::unrest_swing_cause(&a, &province, 40.0, 10.0, &[]));
        for tier in 0..5 {
            let cause =
                build::attitude_shift_cause(&a, &b, (-10, 30), tier, &[("Alliés".to_owned(), 30)]);
            every_key_resolves(&cause);
        }
        // Every term key of the top bar has a label.
        for key in [
            "receipts",
            "armies",
            "buildings",
            "table",
            "administration",
            "household",
            "wages",
            "siege",
            "obligations",
            "interest",
            "levies",
            "loans",
            "other",
            "others",
            "tech",
            "ruler",
            "retinue",
            "order",
            "building",
            "tax",
            "devastation",
            "goods",
            "health",
            "garrison",
            "occupation",
            "foreign_religion",
            "disorder",
            "loyalty",
            "inertia",
        ] {
            assert_ne!(term_label(key, ""), key, "missing term label: {key}");
        }
    }

    #[test]
    fn signed_amounts_use_french_typography() {
        assert_eq!(signed_amount(1200), "+1\u{202f}200");
        assert_eq!(signed_amount(-2140), "\u{2212}2\u{202f}140");
        assert_eq!(signed_amount(0), "0");
        assert_eq!(arrow_amount(-5), "▼ \u{2212}5");
        assert_eq!(arrow_amount(0), "=");
    }

    #[test]
    fn labelled_attitude_factor_shows_its_label_and_signed_value() {
        let cause = build::attitude_shift_cause(
            &fac("fac_england"),
            &fac("fac_france"),
            (-30, 25),
            3,
            &[
                ("Alliés".to_owned(), 30),
                ("Voisin menaçant".to_owned(), -10),
            ],
        );
        let texts: Vec<String> = cause
            .factors
            .iter()
            .map(|f| factor_text(&f.key, f.value))
            .collect();
        assert!(texts.iter().any(|t| t == "Alliés (+30)"));
        assert!(texts.iter().any(|t| t.contains("bienveillant")));
    }

    #[test]
    fn season_story_is_deterministic_and_never_empty_on_the_third_turn() {
        let data = game_data();
        let run = || {
            let mut state = start_quiet(data, "fac_england", 1337);
            for _ in 0..3 {
                state.end_turn_with(data, sim_campaign::test_support::idle);
            }
            let player = state.player_faction().clone();
            season_story_with(data, &state, &player, None)
        };
        let story = run();
        assert_eq!(story, run());
        assert!(story.why.len() <= data.cause_thresholds.story_causes as usize);
        assert!(
            !story.why.is_empty(),
            "turn 3 of the English must already have a cause: {story:#?}"
        );
        assert!(story
            .why
            .windows(2)
            .all(|w| (w[0].turn >= story.turn) >= (w[1].turn >= story.turn)));
        assert!(!story.changed.is_empty());
    }
}
