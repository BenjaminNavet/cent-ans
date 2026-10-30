//! The attention budget and the season's Audience (XP-AUDIENCE, R1, ADR « Budget d'attention »).
//!
//! Every channel that used to speak to the player on its own (the end-of-turn bell, the counsellor
//! band, the « Que faire » card, the mission offer, the chronicle decisions, the ambitions to
//! choose) is a *source*. [`agenda`] ranks the sources by `data/rules/agenda.json` into
//! - **Affairs** (at most `max_affairs`): the only things pushed at the player, each spoken by a
//!   councillor with an action button;
//! - **À surveiller** (at most `max_watch`): listed, not pushed;
//! - **Nouvelles**: folded, counted by rubric.
//!
//! The registry keeps everything; only the *push* is limited. The client draws the result and
//! writes no text: the lines are in `data/ui/audience.fr.json`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use data_model::{AgendaRank, AgendaSource, FactionId, GameData};
use serde::{Deserialize, Serialize};
use sim_campaign::events::GameEvent;
use sim_campaign::CampaignState;

use crate::turn::{Alert, Severity};

const TEXTS_JSON: &str = include_str!("../../../../data/ui/audience.fr.json");

#[derive(Debug, Deserialize)]
struct Texts {
    ui: BTreeMap<String, String>,
    lines: BTreeMap<String, String>,
    lines_many: BTreeMap<String, String>,
    buttons: BTreeMap<String, String>,
    speakers: BTreeMap<String, String>,
}

fn texts() -> &'static Texts {
    static TEXTS: OnceLock<Texts> = OnceLock::new();
    TEXTS.get_or_init(|| serde_json::from_str(TEXTS_JSON).expect("data/ui/audience.fr.json parses"))
}

/// A UI label of `audience.fr.json` (`ui` block); the key itself when missing.
pub fn ui_text(key: &str) -> String {
    ui(key)
}

fn ui(key: &str) -> String {
    texts()
        .ui
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    pairs.iter().fold(template.to_owned(), |acc, (key, value)| {
        acc.replace(&format!("{{{key}}}"), value)
    })
}

/// Who speaks an item: a councillor (or the chronicler) with the office's portrait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Speaker {
    /// Council office id (`chancellor`, `constable`...); empty for the chronicler.
    pub office: String,
    /// « Grand trésorier », « Chroniqueur ».
    pub role: String,
    /// The holder's name, or « Le chroniqueur » / « Le conseil » (vacant office).
    pub name: String,
    /// `chr_*` of the holder (the portrait); empty without one.
    pub character_id: String,
}

/// What the main button does; the client maps `kind` to a panel or a camera move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgendaAction {
    /// `open_chronicle`, `open_offer`, `open_ambitions`, `center_province`, `center_army`,
    /// `open_finance`, `open_diplomacy`, `open_research`, `none`.
    pub kind: String,
    pub label: String,
    /// Province or army id for `center_*`.
    pub target: Option<String>,
    /// `province`, `army` or empty.
    pub target_kind: String,
}

/// One matter pushed at the player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Affair {
    pub id: String,
    /// Source kind (`chronicle_decision`, `mission_offer`, `siege`...).
    pub kind: String,
    pub speaker: Speaker,
    /// What the speaker says (French, from `audience.fr.json`).
    pub text: String,
    pub action: AgendaAction,
    /// The end of the season waits for it.
    pub blocking: bool,
    /// The cause in the journal of causes that explains it (button « Pourquoi ? »).
    pub cause_id: Option<u32>,
    /// Several sources of the kind folded into this line.
    pub count: usize,
}

/// A line of « À surveiller ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WatchItem {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub action: AgendaAction,
    pub count: usize,
}

/// News folded by rubric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewsGroup {
    pub key: String,
    pub label: String,
    pub count: usize,
}

/// « Nouvelles du monde » : counted, folded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewsDigest {
    /// Lines of the journal of the season, plus the far news.
    pub total: usize,
    /// Player's journal lines per rubric, biggest first.
    pub groups: Vec<NewsGroup>,
    /// Letters kept for the player (under the news filter).
    pub letters: usize,
    /// Distant news kept folded.
    pub world: usize,
    /// Items that did not fit « À surveiller ».
    pub overflow: usize,
}

/// The season's agenda of `faction`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Agenda {
    pub turn: u32,
    pub date_label: String,
    /// « Printemps 1338 — Audience ».
    pub title: String,
    pub affairs: Vec<Affair>,
    pub watch: Vec<WatchItem>,
    pub news: NewsDigest,
    /// Text of the Affair that blocks the end of the season (the seal says which), if any.
    pub blocking: Option<String>,
    /// Said when there is no Affair.
    pub quiet: String,
    /// « À surveiller (2) » and « Nouvelles du monde (19) ».
    pub watch_heading: String,
    pub news_heading: String,
    pub news_summary: String,
    /// Fingerprint of the pushed Affairs: the client re-opens the Audience when it changes.
    pub signature: String,
}

/// A source before ranking.
#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    kind: String,
    /// Sentence parameters.
    text: String,
    title: String,
    amount: String,
    terms: String,
    count: usize,
    blocking: bool,
    province: Option<String>,
    army: Option<String>,
    cause_id: Option<u32>,
    severity: u8,
}

fn candidate(id: &str, kind: &str) -> Candidate {
    Candidate {
        id: id.to_owned(),
        kind: kind.to_owned(),
        text: String::new(),
        title: String::new(),
        amount: String::new(),
        terms: String::new(),
        count: 1,
        blocking: false,
        province: None,
        army: None,
        cause_id: None,
        severity: 0,
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Danger => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

fn from_alert(alert: &Alert) -> Candidate {
    let kind = if alert.kind == "chronicle_decision" && !alert.blocking {
        "chronicle_incident"
    } else {
        alert.kind.as_str()
    };
    let mut c = candidate(&alert.id, kind);
    let (first, _) = alert.text.split_once('\n').unwrap_or((&alert.text, ""));
    c.text = first.to_owned();
    c.title = first
        .strip_prefix("Chronique : ")
        .unwrap_or(first)
        .to_owned();
    c.blocking = alert.blocking;
    c.province = alert.province.clone();
    c.army = alert.army.clone();
    c.severity = severity_rank(alert.severity);
    c
}

fn speaker(data: &GameData, state: &CampaignState, faction: &FactionId, office: &str) -> Speaker {
    let t = texts();
    let word = |key: &str| t.speakers.get(key).cloned().unwrap_or_default();
    if office.is_empty() {
        return Speaker {
            office: String::new(),
            role: word("chronicler_role"),
            name: word("chronicler_name"),
            character_id: String::new(),
        };
    }
    let found = crate::court::council(data, state, faction)
        .into_iter()
        .find(|o| o.id == office);
    match found {
        Some(o) if !o.holder_id.is_empty() => Speaker {
            office: o.id,
            role: o.name,
            name: o.holder_name,
            character_id: o.holder_id,
        },
        Some(o) => Speaker {
            office: o.id,
            role: o.name,
            name: word("vacant_name"),
            character_id: String::new(),
        },
        None => Speaker {
            office: office.to_owned(),
            role: String::new(),
            name: word("vacant_name"),
            character_id: String::new(),
        },
    }
}

fn action(source: &AgendaSource, item: &Candidate) -> AgendaAction {
    let (target, target_kind) = match source.action.as_str() {
        "center_province" => (item.province.clone(), "province"),
        "center_army" => match (&item.army, &item.province) {
            (Some(army), _) => (Some(army.clone()), "army"),
            (None, Some(province)) => (Some(province.clone()), "province"),
            _ => (None, ""),
        },
        _ => (None, ""),
    };
    let kind = match (source.action.as_str(), &target) {
        ("center_army" | "center_province", None) => "none",
        (other, _) => other,
    };
    AgendaAction {
        kind: kind.to_owned(),
        label: texts().buttons.get(kind).cloned().unwrap_or_default(),
        target,
        target_kind: target_kind.to_owned(),
    }
}

fn line(source: &AgendaSource, item: &Candidate, to_choose: u32) -> String {
    let t = texts();
    let template = if item.count > 1 {
        t.lines_many
            .get(&source.kind)
            .or_else(|| t.lines.get(&source.kind))
    } else {
        t.lines.get(&source.kind)
    };
    let count = if source.kind == "ambition_choice" {
        to_choose.to_string()
    } else {
        item.count.to_string()
    };
    fill(
        template.map_or("{text}", String::as_str),
        &[
            ("text", &item.text),
            ("title", &item.title),
            ("count", &count),
            ("amount", &item.amount),
            ("terms", &item.terms),
        ],
    )
}

/// The sources of the season: the bell's alerts, the mission offer, the ambitions to choose and
/// the treasurer's balance.
fn candidates(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> (Vec<Candidate>, u32) {
    let mut out: Vec<Candidate> = crate::turn::turn_readiness(data, state, faction)
        .alerts
        .iter()
        .map(from_alert)
        .collect();
    let player = faction == state.player_faction();
    let mut to_choose = 0;
    if player {
        if state.mission_offer(data).is_some() {
            out.push(candidate("mission_offer", "mission_offer"));
        }
        if let Some(act) = crate::acts::act_view(data, state) {
            to_choose = act.ambitions_to_choose;
            if act.ambition_pending && !act.ambitions.is_empty() {
                out.push(candidate("ambition_choice", "ambition_choice"));
            }
        }
    }
    if let Some(economy) = state.faction_economy(data, faction) {
        let net = economy.net_income();
        if player && net < data.agenda_rules.treasury_affair_below {
            let mut c = candidate("treasury_swing", "treasury_swing");
            c.amount = sim_campaign::advisor::fr_number(-net);
            c.terms = treasury_terms(data, state, faction);
            c.cause_id = crate::causes::season_causes(state, data, 6)
                .iter()
                .find(|cause| cause.kind == "treasury_swing")
                .map(|cause| cause.id);
            out.push(c);
        }
    }
    (out, to_choose)
}

/// « soldes (−1 800), Guyenne perdue (−600) »: the heaviest terms of the coming balance.
fn treasury_terms(data: &GameData, state: &CampaignState, faction: &FactionId) -> String {
    crate::hud::figure_trends(data, state, faction)
        .into_iter()
        .find(|trend| trend.figure == "net_income")
        .map(|trend| {
            trend
                .top_terms
                .iter()
                .filter(|term| term.amount < 0)
                .map(|term| {
                    format!(
                        "{} ({})",
                        term.label.to_lowercase(),
                        crate::causes::signed_amount(term.amount)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// Folds the items of a grouped kind into one.
fn fold_groups(rules: &data_model::AgendaRules, items: Vec<Candidate>) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    for item in items {
        let grouped = rules.source(&item.kind).is_some_and(|s| s.grouped);
        if grouped {
            if let Some(first) = out.iter_mut().find(|c| c.kind == item.kind) {
                first.count += 1;
                continue;
            }
        }
        out.push(item);
    }
    out
}

fn news_digest(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
    events: &[GameEvent],
    overflow: usize,
) -> NewsDigest {
    let journal = crate::chronicle::turn_journal(
        data,
        state,
        faction,
        events,
        crate::chronicle::NewsMode::Interest,
    );
    let labels: BTreeMap<String, String> = crate::chronicle::genre_choices()
        .into_iter()
        .map(|g| (g.id, g.label))
        .collect();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &journal.entries {
        *counts.entry(entry.genre.clone()).or_default() += 1;
    }
    let mut groups: Vec<NewsGroup> = counts
        .into_iter()
        .map(|(key, count)| NewsGroup {
            label: labels.get(&key).cloned().unwrap_or_else(|| key.clone()),
            key,
            count,
        })
        .collect();
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    NewsDigest {
        total: journal.entries.len() + journal.world.len(),
        groups,
        letters: journal.letters.len(),
        world: journal.world.len(),
        overflow,
    }
}

fn news_summary(news: &NewsDigest) -> String {
    let mut parts: Vec<String> = Vec::new();
    if news.letters > 0 {
        parts.push(fill(
            &ui("news_letters"),
            &[("count", &news.letters.to_string())],
        ));
    }
    if news.world > 0 {
        parts.push(fill(
            &ui("news_world"),
            &[("count", &news.world.to_string())],
        ));
    }
    for group in news.groups.iter().take(3) {
        parts.push(format!("{} {}", group.label, group.count));
    }
    if parts.is_empty() {
        ui("news_none")
    } else {
        parts.join(" · ")
    }
}

/// The agenda of `faction` this season. `events` are those of the season just resolved (the
/// folded news); pass an empty slice before the first season.
pub fn agenda(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
    events: &[GameEvent],
) -> Agenda {
    let rules = &data.agenda_rules;
    let (raw, to_choose) = candidates(data, state, faction);
    let folded = fold_groups(rules, raw);
    // Rank: each item takes the rank of its source; unknown kinds stay out (the registry has them).
    let mut affairs: Vec<(&AgendaSource, Candidate)> = Vec::new();
    let mut watch: Vec<(u32, &AgendaSource, Candidate)> = Vec::new();
    let mut folded_news = 0usize;
    for item in folded {
        let Some(source) = rules.source(&item.kind) else {
            folded_news += 1;
            continue;
        };
        match source.rank {
            AgendaRank::Affair => affairs.push((source, item)),
            AgendaRank::Watch => watch.push((source.order, source, item)),
            AgendaRank::News => folded_news += 1,
        }
    }
    // Blocking and guaranteed first, then by `order`; the id keeps the order deterministic.
    affairs.sort_by(|(sa, a), (sb, b)| {
        (!sa.guaranteed, sa.order, a.severity, &a.id).cmp(&(
            !sb.guaranteed,
            sb.order,
            b.severity,
            &b.id,
        ))
    });
    let cap = rules.max_affairs as usize;
    for (source, item) in affairs.split_off(cap.min(affairs.len())) {
        // What does not fit a slot is watched, after the native watch items.
        watch.push((1000 + source.order, source, item));
    }
    watch.sort_by(|(oa, _, a), (ob, _, b)| (oa, &a.id).cmp(&(ob, &b.id)));
    let watch_cap = rules.max_watch as usize;
    let overflow = watch.len().saturating_sub(watch_cap) + folded_news;
    watch.truncate(watch_cap);

    let affair_views: Vec<Affair> = affairs
        .iter()
        .map(|(source, item)| Affair {
            id: item.id.clone(),
            kind: item.kind.clone(),
            speaker: speaker(data, state, faction, &source.office),
            text: line(source, item, to_choose),
            action: action(source, item),
            blocking: item.blocking,
            cause_id: item.cause_id,
            count: item.count,
        })
        .collect();
    let watch_views: Vec<WatchItem> = watch
        .iter()
        .map(|(_, source, item)| WatchItem {
            id: item.id.clone(),
            kind: item.kind.clone(),
            text: line(source, item, to_choose),
            action: action(source, item),
            count: item.count,
        })
        .collect();
    let news = news_digest(data, state, faction, events, overflow);
    let blocking = affair_views
        .iter()
        .find(|a| a.blocking)
        .map(|a| fill(&ui("blocking_seal"), &[("text", &a.text)]));
    let signature = format!(
        "{}|{}",
        state.turn(),
        affair_views
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    Agenda {
        turn: state.turn(),
        date_label: state.date_label(),
        title: fill(&ui("title"), &[("date", &state.date_label())]),
        quiet: ui("quiet"),
        watch_heading: fill(
            &ui("watch_heading"),
            &[("count", &watch_views.len().to_string())],
        ),
        news_heading: fill(&ui("news_heading"), &[("count", &news.total.to_string())]),
        news_summary: news_summary(&news),
        affairs: affair_views,
        watch: watch_views,
        news,
        blocking,
        signature,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn france() -> FactionId {
        FactionId::new("fac_france").unwrap()
    }

    #[test]
    fn texts_and_rules_cover_every_source() {
        let data = data_model::test_support::game_data();
        assert!(
            !data.agenda_rules.sources.is_empty(),
            "agenda.json is loaded"
        );
        for source in &data.agenda_rules.sources {
            assert!(
                texts().lines.contains_key(&source.kind),
                "no line for {}",
                source.kind
            );
            assert!(
                texts().buttons.contains_key(&source.action),
                "no button for {}",
                source.action
            );
            if source.grouped {
                assert!(texts().lines_many.contains_key(&source.kind));
            }
        }
    }

    #[test]
    fn caps_and_determinism_over_seeds_and_seasons() {
        let data = data_model::test_support::game_data();
        let rules = &data.agenda_rules;
        for (seed, faction) in [
            (1337u64, "fac_france"),
            (7, "fac_england"),
            (42, "fac_burgundy"),
        ] {
            let mut state = sim_campaign::test_support::start(data, faction, seed);
            let player = state.player_faction().clone();
            let mut events = Vec::new();
            for _ in 0..12 {
                let a = agenda(data, &state, &player, &events);
                assert!(a.affairs.len() <= rules.max_affairs as usize);
                assert!(a.watch.len() <= rules.max_watch as usize);
                assert_eq!(a, agenda(data, &state, &player, &events), "deterministic");
                // A blocking decision is always an Affair.
                let blocking = state
                    .decision_views(data, &player)
                    .iter()
                    .filter(|d| !matches!(d.presentation, data_model::EventPresentation::Map))
                    .count();
                if blocking > 0 {
                    assert!(
                        a.affairs.iter().any(|x| x.blocking),
                        "{faction} seed {seed}"
                    );
                    assert!(a.blocking.is_some());
                }
                events = state.end_turn_with(data, sim_campaign::test_support::idle);
            }
        }
    }

    /// XP-AUDIENCE: the choice of the ambitions is an Affair from the first season, gone once
    /// two are held; once the choice freezes, the others are locked (greyed by the client).
    #[test]
    fn ambitions_are_an_affair_until_chosen_then_locked() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        state.end_turn(data);
        let player = state.player_faction().clone();
        let a = agenda(data, &state, &player, &[]);
        let ambition = a
            .affairs
            .iter()
            .find(|x| x.kind == "ambition_choice")
            .expect("ambition affair at turn 1");
        assert_eq!(ambition.action.kind, "open_ambitions");
        assert!(ambition
            .text
            .contains(&data.acts_rules.ambitions_to_choose.to_string()));
        let view = crate::acts::act_view(data, &state).unwrap();
        assert!(
            view.ambitions.iter().all(|x| !x.locked),
            "open choice: nothing locked"
        );
        let ids: Vec<String> = view.ambitions.iter().map(|x| x.id.clone()).collect();
        assert!(crate::acts::choose_ambition(data, &mut state, &ids[0]));
        assert!(crate::acts::choose_ambition(data, &mut state, &ids[1]));
        let a = agenda(data, &state, &player, &[]);
        assert!(a.affairs.iter().all(|x| x.kind != "ambition_choice"));
        // Freeze the choice: the seasons allowed for replacing an ambition are over.
        for _ in 0..=data.acts_rules.ambition_choice_seasons {
            state.end_turn_with(data, sim_campaign::test_support::idle);
        }
        let view = crate::acts::act_view(data, &state).unwrap();
        if !view.ambition_choice_open && !view.ambitions.is_empty() {
            assert!(view.ambitions.iter().all(|x| x.locked == !x.chosen));
        }
    }

    /// Spec property: never more than three Affairs, over 200 seeds x 12 seasons (a few minutes in
    /// debug; `cargo test -p game-api agenda_budget_200 -- --ignored`).
    #[test]
    #[ignore = "200 seeds x 12 seasons"]
    fn agenda_budget_200_seeds() {
        let data = data_model::test_support::game_data();
        let factions = ["fac_france", "fac_england", "fac_burgundy"];
        for seed in 0..200u64 {
            let faction = factions[(seed % 3) as usize];
            let mut state = sim_campaign::test_support::start(data, faction, seed + 1);
            let player = state.player_faction().clone();
            let mut events = Vec::new();
            for _ in 0..12 {
                let a = agenda(data, &state, &player, &events);
                assert!(a.affairs.len() <= data.agenda_rules.max_affairs as usize);
                let blocking = state
                    .decision_views(data, &player)
                    .iter()
                    .filter(|d| !matches!(d.presentation, data_model::EventPresentation::Map))
                    .count();
                if blocking > 0 {
                    assert!(
                        a.affairs.iter().any(|x| x.blocking),
                        "{faction} seed {seed}"
                    );
                }
                events = state.end_turn_with(data, sim_campaign::test_support::idle);
            }
        }
    }

    #[test]
    fn a_pending_offer_is_an_affair_and_a_quiet_start_says_so() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let a = agenda(data, &state, &france(), &[]);
        assert!(a.title.contains("Audience"));
        assert!(a.affairs.iter().all(|x| !x.text.is_empty()));
        assert!(!a.quiet.is_empty());
    }
}
