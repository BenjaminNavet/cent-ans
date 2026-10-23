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
        &[("lieu", &place), ("sujet", &subject), ("acteur", &actor)],
    );
    let factors = cause
        .factors
        .iter()
        .map(|f| FactorView {
            key: f.key.clone(),
            text: fill(
                t.factors.get(&f.key).map_or(f.key.as_str(), String::as_str),
                &[("value", &f.value.to_string())],
            ),
            weight: f.weight,
        })
        .collect();
    let reason = cause.reason.as_ref().map(|r| {
        let sentence = t.reasons.get(&r.key).map_or(r.key.as_str(), String::as_str);
        fill(&ui_text("reason_line"), &[("reason", sentence)])
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
        CauseKind::GameWon | CauseKind::GameLost | CauseKind::UnrestWarning => false,
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
        CauseKind::Bankruptcy | CauseKind::Revolt | CauseKind::Succession => {
            !favourable && cause.subject.as_ref() == Some(player)
        }
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
}
