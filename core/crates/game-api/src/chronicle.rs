//! Chronicle views: pending decisions, journal of a resolved turn, sealed news letters.
//!
//! Replaces the state-reading half of `game/scripts/ui/journal_view.gd` (filter, relevance,
//! genre, line style), `news_letters.gd` (which events deserve a letter, rubric, wax colour),
//! `news_interest.gd` (the "interest" mode, now the core's [`Relevance`], audit A6) and
//! `chronicle_controller.gd` (expired decision notices). The client only keeps the history of
//! turns it has seen and draws it.

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_campaign::chronicle::DecisionView;
use sim_campaign::news_relevance::{NewsRelevance, Relevance};
use sim_campaign::{CampaignState, GameEvent};

/// Mark of an expired decision in the core's chronicle entry (`resolve_decision`).
pub const EXPIRED_MARK: &str = "(délai écoulé)";

/// Genres of the journal filter (`data/ui/journal_genres.json`).
const GENRES_JSON: &str = include_str!("../../../../data/ui/journal_genres.json");

/// Which news reach the letters and the top banner (setting `interface/news_filter`); the
/// journal keeps everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NewsMode {
    /// Neighbours, allies, enemies and public news.
    Interest,
    /// All of Europe.
    All,
    /// Only the player's own realm.
    Own,
}

impl NewsMode {
    pub const MODES: [NewsMode; 3] = [NewsMode::Interest, NewsMode::All, NewsMode::Own];

    /// Mode of a setting value; unknown names give [`NewsMode::Interest`].
    pub fn parse(name: &str) -> NewsMode {
        match name {
            "all" => NewsMode::All,
            "own" => NewsMode::Own,
            _ => NewsMode::Interest,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            NewsMode::Interest => "interest",
            NewsMode::All => "all",
            NewsMode::Own => "own",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            NewsMode::Interest => "Voisins, alliés, ennemis et grandes puissances",
            NewsMode::All => "Toute l’Europe",
            NewsMode::Own => "Seulement mon royaume",
        }
    }
}

/// One choice of the genre filter bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GenreChoice {
    pub id: String,
    pub label: String,
}

fn genres_config() -> serde_json::Value {
    serde_json::from_str(GENRES_JSON).unwrap_or_default()
}

/// Filter choices: « Tout » first, then the genres of `journal_genres.json`.
pub fn genre_choices() -> Vec<GenreChoice> {
    let config = genres_config();
    let mut choices = vec![GenreChoice {
        id: "all".into(),
        label: config["all_label"].as_str().unwrap_or("Tout").to_owned(),
    }];
    for genre in config["genres"].as_array().into_iter().flatten() {
        choices.push(GenreChoice {
            id: genre["id"].as_str().unwrap_or_default().to_owned(),
            label: genre["label"].as_str().unwrap_or_default().to_owned(),
        });
    }
    choices
}

/// Genre id of an event `kind` (`other` when no genre lists it).
pub fn genre_of(kind: &str) -> String {
    let config = genres_config();
    for genre in config["genres"].as_array().into_iter().flatten() {
        let listed = genre["kinds"]
            .as_array()
            .is_some_and(|kinds| kinds.iter().any(|k| k.as_str() == Some(kind)));
        if listed {
            return genre["id"].as_str().unwrap_or_default().to_owned();
        }
    }
    config["other"].as_str().unwrap_or("other").to_owned()
}

/// Colour, weight and glyph of a journal line (`JournalView.JOURNAL_STYLES`, then
/// `SeasonReport.KIND_STYLES`); `None` = plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LineStyle {
    pub color: &'static str,
    pub bold: bool,
    pub glyph: &'static str,
}

const fn style(color: &'static str, bold: bool, glyph: &'static str) -> LineStyle {
    LineStyle { color, bold, glyph }
}

/// Style of an event kind.
pub fn line_style(kind: &str) -> Option<LineStyle> {
    const WAR: LineStyle = style("#8b1a1a", true, "⚔");
    const OFFICE: LineStyle = style("#4a3a10", false, "⚑");
    const ALLIANCE: LineStyle = style("#1a3a8b", true, "⚜");
    const MAIL: LineStyle = style("#4a3a10", false, "✉");
    const FAITH: LineStyle = style("#5a2a6a", true, "✠");
    const SKILL: LineStyle = style("#2a5a7a", false, "★");
    Some(match kind {
        "battle" | "siege_started" | "province_captured" | "war_declared" => WAR,
        "revolt" => style("#a1121a", true, "⚑"),
        "plague" => style("#4a6b2a", true, "☠"),
        "famine" => style("#8a5a10", true, "⚠"),
        "building_completed" => style("#1a5c8b", false, "⚒"),
        "birth" => style("#2a7a4a", false, "✚"),
        "marriage" => style("#8a3a8a", true, "♥"),
        "death" => style("#3a3a3a", true, "✝"),
        "succession" => style("#7a5a10", true, "♔"),
        "regency" => style("#7a5a10", false, "⚖"),
        "trait_acquired" => style("#2a5a7a", false, "✦"),
        "skill_learned" => SKILL,
        "appointment" => OFFICE,
        "peace_signed" => style("#2a6a2a", true, "☮"),
        "alliance_formed" | "vassalage" => ALLIANCE,
        "alliance_broken" | "vassal_rebellion" => style("#a1121a", true, "⚡"),
        "embargo" | "diplomatic_offer" | "diplomacy" => MAIL,
        "trade" => style("#1a5a6a", false, "⚓"),
        "excommunication" | "schism" | "heresy" => FAITH,
        "chronicle" => style("#7a3b0c", true, "§"),
        "technology_researched" => style("#5a2a8a", true, "⚙"),
        "income" => style("#4a3a10", false, ""),
        // SeasonReport.KIND_STYLES
        "table" => style("#7a4a10", false, "♨"),
        "medicine" => style("#2a6a4a", false, "✚"),
        "coinage" => style("#8a6a10", false, "¤"),
        "ransom" => style("#7a2a1a", false, "⚖"),
        "chivalry" => style("#2a3a7a", false, "⚜"),
        "agent" => style("#4a2a6a", false, "✦"),
        "mission" => style("#5a3a10", false, "✠"),
        "royal_act" => style("#7a5a10", false, "♛"),
        "level_up" => style("#2a5a2a", false, "★"),
        "loyalty" => style("#7a2a2a", false, "⚖"),
        "captain" => style("#5a3a10", false, "⚔"),
        "crusade" => style("#8b1a1a", false, "✠"),
        _ => return None,
    })
}

/// Rubric of a letter (`NewsLetters.KIND_LABELS`); « Nouvelle » for other kinds.
pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "faction_met" => "Rencontre",
        "war_declared" => "Déclaration de guerre",
        "peace" | "peace_signed" => "Traité de paix",
        "birth" => "Naissance",
        "death" => "Décès",
        "succession" => "Succession",
        "city_captured" | "province_captured" => "Prise de ville",
        "alliance_formed" => "Alliance",
        "alliance_broken" => "Rupture d’alliance",
        "marriage" => "Mariage",
        "faction_destroyed" => "Chute d’une maison",
        "vassalage" => "Hommage",
        "vassal_rebellion" => "Révolte d’un vassal",
        "excommunication" => "Excommunication",
        "regency" => "Régence",
        "general_captured" => "Chef capturé",
        "revolt" => "Révolte",
        "plague" => "Peste",
        "table" => "La Table",
        "edict" => "Édit régional",
        "medicine" => "Médecine",
        "coinage" => "Monnaie",
        "ransom" => "Rançon",
        "chivalry" => "Chevalerie",
        "crusade" => "Croisade",
        "royal_act" => "Acte royal",
        "level_up" => "Montée de niveau",
        "loyalty" => "Loyauté",
        "captain" => "Capitaine engagé",
        _ => "Nouvelle",
    }
}

/// Event kinds that deserve a letter (`NewsLetters.NEWS_EVENT_KINDS`); the others stay in the
/// journal.
pub const NEWS_EVENT_KINDS: [&str; 25] = [
    "war_declared",
    "peace_signed",
    "alliance_formed",
    "alliance_broken",
    "marriage",
    "birth",
    "death",
    "succession",
    "province_captured",
    "faction_destroyed",
    "vassalage",
    "vassal_rebellion",
    "excommunication",
    "regency",
    "general_captured",
    "revolt",
    "plague",
    "coinage",
    "ransom",
    "chivalry",
    "royal_act",
    "level_up",
    "captain",
    "loyalty",
    "crusade",
];

/// Wax colour family of a letter: `green` (peace, alliance, marriage, hommage), `dark` (death),
/// `light` (birth), `red` otherwise.
pub fn wax_of(kind: &str) -> &'static str {
    match kind {
        "peace" | "peace_signed" | "faction_met" | "alliance_formed" | "marriage" | "vassalage" => {
            "green"
        }
        "death" => "dark",
        "birth" => "light",
        _ => "red",
    }
}

/// Foreign events of no interest to the player (internal management of another faction).
const FOREIGN_MINOR_KINDS: [&str; 12] = [
    "income",
    "bankruptcy",
    "attrition",
    "recruited",
    "building_completed",
    "trait_acquired",
    "skill_learned",
    "appointment",
    "technology_researched",
    "regency",
    "birth",
    "raid",
];

/// French label of a relevance (`JournalView.relevance_label`).
pub fn relevance_label(relevance: Relevance) -> &'static str {
    match relevance {
        Relevance::Player => "Votre royaume",
        Relevance::Related => "Allié, ennemi ou vassal",
        Relevance::Neighbor => "Voisin",
        Relevance::Far => "Nouvelle lointaine",
    }
}

/// A journal line of a resolved turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JournalEntry {
    pub kind: String,
    pub genre: String,
    /// French text, prefixed with the faction name for other factions' news.
    pub text: String,
    pub faction: Option<String>,
    pub province: Option<String>,
    /// `player`, `related`, `neighbor` or `far`.
    pub relevance: String,
    pub relevance_label: String,
    pub public: bool,
    pub loss: bool,
    pub style: Option<LineStyle>,
}

/// A sealed letter (`NewsLetters.news_from_event`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewsLetter {
    pub kind: String,
    pub rubric: String,
    pub title: String,
    pub text: String,
    pub faction: Option<String>,
    pub province: Option<String>,
    /// Why the news is kept (the relevance label).
    pub interest: String,
    pub wax: String,
}

/// What a resolved turn adds to the chronicle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TurnJournal {
    pub turn: u32,
    pub date_label: String,
    /// The player's journal lines (not « Monde »).
    pub entries: Vec<JournalEntry>,
    /// « Monde »: distant news, kept folded.
    pub world: Vec<JournalEntry>,
    /// Letters of the turn under the news filter, oldest first.
    pub letters: Vec<NewsLetter>,
    /// Notices for decisions that expired this turn.
    pub expired: Vec<String>,
}

fn kind_name(event: &GameEvent) -> String {
    serde_json::to_value(event.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Whether the journal shows `event` to `player` (`JournalView.keeps`).
fn journal_keeps(event: &GameEvent, kind: &str, player: &FactionId) -> bool {
    match &event.faction {
        None => true,
        Some(f) if f == player => true,
        Some(_) => {
            let private_crusade = kind == "crusade" && !event.public;
            !private_crusade && !FOREIGN_MINOR_KINDS.contains(&kind)
        }
    }
}

/// Whether the news deserves a letter under `mode` (`JournalView.keeps_news`).
pub fn keeps_news(mode: NewsMode, relevance: Relevance, public: bool) -> bool {
    if public || mode == NewsMode::All {
        return true;
    }
    match mode {
        NewsMode::Own => relevance == Relevance::Player,
        _ => relevance != Relevance::Far,
    }
}

/// Notices for the player's decisions that expired in `events` (`notify_expired`).
pub fn expired_notices(events: &[GameEvent], player: &FactionId) -> Vec<String> {
    events
        .iter()
        .filter(|e| kind_name(e) == "chronicle" && e.faction.as_ref() == Some(player))
        .filter(|e| e.text_fr.contains(EXPIRED_MARK))
        .map(|e| {
            let decided = e
                .text_fr
                .replace(&format!(" {EXPIRED_MARK}"), "")
                .trim_end_matches('.')
                .to_owned();
            format!("Délai écoulé, le conseil a tranché — {decided}.")
        })
        .collect()
}

/// Sorts the events of the turn that just resolved into the player's journal, « Monde » and
/// letters. `state` is the state after the turn (relevance reads current owners and borders).
pub fn turn_journal(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    events: &[GameEvent],
    mode: NewsMode,
) -> TurnJournal {
    let relevance = NewsRelevance::new(state, data, player);
    let mut entries = Vec::new();
    let mut world = Vec::new();
    let mut letters = Vec::new();
    for event in events {
        let kind = kind_name(event);
        if !journal_keeps(event, &kind, player) {
            continue;
        }
        let level = relevance.classify(
            event.faction.as_ref(),
            event.province.as_ref(),
            event.public,
        );
        let label = relevance_label(level);
        if NEWS_EVENT_KINDS.contains(&kind.as_str()) && keeps_news(mode, level, event.public) {
            letters.push(NewsLetter {
                kind: kind.clone(),
                rubric: kind_label(&kind).to_owned(),
                title: event.text_fr.clone(),
                text: event.text_fr.clone(),
                faction: event.faction.as_ref().map(|f| f.as_str().to_owned()),
                province: event.province.as_ref().map(|p| p.as_str().to_owned()),
                interest: label.to_owned(),
                wax: wax_of(&kind).to_owned(),
            });
        }
        if event.text_fr.is_empty() {
            continue;
        }
        let text = match &event.faction {
            Some(f) if f != player => {
                let name = data.faction_name(f);
                if name.is_empty() || event.text_fr.contains(&name) {
                    event.text_fr.clone()
                } else {
                    format!("{name} — {}", event.text_fr)
                }
            }
            _ => event.text_fr.clone(),
        };
        let entry = JournalEntry {
            genre: genre_of(&kind),
            style: line_style(&kind),
            text,
            faction: event.faction.as_ref().map(|f| f.as_str().to_owned()),
            province: event.province.as_ref().map(|p| p.as_str().to_owned()),
            relevance: level.key().to_owned(),
            relevance_label: label.to_owned(),
            public: event.public,
            loss: event.loss,
            kind,
        };
        if level == Relevance::Far && !event.public {
            world.push(entry);
        } else {
            entries.push(entry);
        }
    }
    TurnJournal {
        turn: state.turn,
        date_label: state.date_label(),
        entries,
        world,
        letters,
        expired: expired_notices(events, player),
    }
}

/// Pending decisions of `player` and counts for the « Chronique (n) » button.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChronicleView {
    pub decisions: Vec<DecisionView>,
    pub count: usize,
    /// Decisions that open a dialog (the others sit on the map as incidents).
    pub dialog_count: usize,
    /// Tooltip of the button.
    pub tooltip: String,
}

/// Pending decisions of `player` with the button tooltip (`ChronicleController.refresh`).
pub fn chronicle_view(data: &GameData, state: &CampaignState, player: &FactionId) -> ChronicleView {
    let decisions = state.decision_views(data, player);
    let count = decisions.len();
    let dialog_count = decisions
        .iter()
        .filter(|d| !matches!(d.presentation, data_model::EventPresentation::Map))
        .count();
    let tooltip = if count > 0 {
        let noun = if count == 1 {
            "événement"
        } else {
            "événements"
        };
        let verb = if count == 1 { "attend" } else { "attendent" };
        format!("{count} {noun} {verb} votre décision : deux saisons pour choisir, sinon le conseil tranche.")
    } else {
        "Aucun événement n’attend votre décision pour l’instant. Les grands événements, historiques (Crécy, la Peste noire…) ou aléatoires, arrivent en fin de saison.".to_owned()
    };
    ChronicleView {
        decisions,
        count,
        dialog_count,
        tooltip,
    }
}

/// Lines of `source` of genre `genre` (`all` keeps everything): a date header is kept only when
/// a line of the genre follows it (`JournalView.filter_lines`). `source` items are
/// `(genre, text)`; a header has genre `""`.
pub fn filter_lines<'a>(source: &'a [(String, String)], genre: &str) -> Vec<&'a (String, String)> {
    if genre == "all" {
        return source.iter().collect();
    }
    let mut out = Vec::new();
    let mut header: Option<&(String, String)> = None;
    for line in source {
        if line.0.is_empty() {
            header = Some(line);
        } else if line.0 == genre {
            if let Some(h) = header.take() {
                out.push(h);
            }
            out.push(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_campaign::EventKind;

    fn fid(s: &str) -> FactionId {
        FactionId::new(s.to_owned()).unwrap()
    }

    #[test]
    fn genres_load_and_classify() {
        let choices = genre_choices();
        assert_eq!(choices[0].id, "all");
        assert!(choices.len() >= 7);
        assert_eq!(genre_of("battle"), "war");
        assert_eq!(genre_of("marriage"), "diplomacy");
        assert_eq!(genre_of("nothing_like_this"), "other");
    }

    #[test]
    fn mode_parsing_and_keeps_news() {
        assert_eq!(NewsMode::parse("own"), NewsMode::Own);
        assert_eq!(NewsMode::parse("weird"), NewsMode::Interest);
        assert!(keeps_news(NewsMode::All, Relevance::Far, false));
        assert!(!keeps_news(NewsMode::Interest, Relevance::Far, false));
        assert!(keeps_news(NewsMode::Interest, Relevance::Neighbor, false));
        assert!(keeps_news(NewsMode::Interest, Relevance::Far, true));
        assert!(!keeps_news(NewsMode::Own, Relevance::Related, false));
        assert!(keeps_news(NewsMode::Own, Relevance::Player, false));
    }

    #[test]
    fn letter_kinds_and_labels() {
        assert!(NEWS_EVENT_KINDS.contains(&"war_declared"));
        assert!(!NEWS_EVENT_KINDS.contains(&"income"));
        assert_eq!(kind_label("peace_signed"), "Traité de paix");
        assert_eq!(kind_label("zzz"), "Nouvelle");
        assert_eq!(wax_of("death"), "dark");
        assert_eq!(wax_of("marriage"), "green");
        assert!(line_style("battle").is_some_and(|s| s.bold));
        assert!(line_style("zzz").is_none());
    }

    #[test]
    fn filter_keeps_headers_only_before_matching_lines() {
        let src = vec![
            (String::new(), "d1".to_owned()),
            ("war".to_owned(), "a".to_owned()),
            (String::new(), "d2".to_owned()),
            ("faith".to_owned(), "b".to_owned()),
            (String::new(), "d3".to_owned()),
            ("war".to_owned(), "c".to_owned()),
        ];
        let war: Vec<_> = filter_lines(&src, "war")
            .iter()
            .map(|l| l.1.as_str())
            .collect();
        assert_eq!(war, ["d1", "a", "d3", "c"]);
        assert_eq!(filter_lines(&src, "all").len(), 6);
    }

    #[test]
    fn expired_decision_notice() {
        let player = fid("fac_france");
        let events = vec![
            GameEvent::new(
                EventKind::Chronicle,
                "La Jacquerie : Réprimer (délai écoulé).",
            )
            .faction(&player),
            GameEvent::new(EventKind::Chronicle, "Crécy : Combattre.").faction(&player),
            GameEvent::new(EventKind::Chronicle, "Autre : X (délai écoulé).")
                .faction(&fid("fac_england")),
        ];
        let notices = expired_notices(&events, &player);
        assert_eq!(
            notices,
            ["Délai écoulé, le conseil a tranché — La Jacquerie : Réprimer."]
        );
    }

    #[test]
    fn turn_journal_sorts_world_and_letters() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(&data, "fac_france", 1337);
        let player = state.player_faction().clone();
        let mut events = vec![
            GameEvent::new(EventKind::WarDeclared, "Guerre déclarée.").faction(&player),
            GameEvent::new(EventKind::Income, "Revenus.").faction(&player),
        ];
        let other = state
            .factions
            .keys()
            .find(|f| **f != player)
            .cloned()
            .expect("another faction");
        events.push(GameEvent::new(EventKind::Recruited, "Levée.").faction(&other));
        let journal = turn_journal(&data, &state, &player, &events, NewsMode::Own);
        // The foreign minor event is dropped from the journal.
        assert!(journal.entries.iter().all(|e| e.kind != "recruited"));
        assert!(journal.entries.iter().any(|e| e.kind == "war_declared"));
        assert_eq!(journal.letters.len(), 1);
        assert_eq!(journal.letters[0].rubric, "Déclaration de guerre");
        assert_eq!(journal.letters[0].interest, "Votre royaume");
    }

    #[test]
    fn chronicle_view_counts_decisions() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(&data, "fac_france", 1337);
        let view = chronicle_view(&data, &state, state.player_faction());
        assert_eq!(view.count, view.decisions.len());
        if view.count == 0 {
            assert!(view.tooltip.starts_with("Aucun événement"));
        }
    }
}
