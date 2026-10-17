//! History kept by the client between turns (the core only keeps the last turn's events):
//! journal lines by date, « Monde » lines, sealed letters. Pure data and BBCode building, tested
//! without Bevy. Ports the state of `journal_view.gd` (`lines`, `world_lines`, `MAX_LOG_LINES`)
//! and `news_letters.gd` (`MAX_KEPT`).

use game_api::chronicle::{self, JournalEntry, LineStyle, NewsLetter, TurnJournal};

use crate::codex::{store::CodexStore, text as codex_text};

/// Lines kept in the journal (`JournalView.MAX_LOG_LINES`), date headers included.
pub const MAX_LOG_LINES: usize = 200;
/// Letters kept (`NewsLetters.MAX_KEPT`).
pub const MAX_LETTERS: usize = 40;

/// One journal line: `genre` is `""` for a date header.
pub type Line = (String, String);

#[derive(
    bevy::prelude::Resource, Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize,
)]
pub struct ChronicleLog {
    /// Journal, most recent first.
    pub lines: Vec<Line>,
    /// « Monde » tab, most recent first.
    pub world: Vec<Line>,
    /// Letters, most recent first.
    pub letters: Vec<NewsLetter>,
    /// Turns recorded since the campaign began in this client.
    pub turns: u32,
    /// Expired-decision notices of the last turn (shown as toasts).
    pub last_expired: Vec<String>,
    /// Event kinds of the newest turn's journal lines (picks the journal vignette).
    pub newest_kinds: Vec<String>,
}

/// BBCode of a journal entry (`JournalView.format_line`); brackets of the text are escaped.
pub fn format_line(entry: &JournalEntry) -> String {
    format_line_with(entry, None)
}

/// [`format_line`] with the Codex links of the text (`CodexText.format(text, true)` in Godot):
/// `[[id|label]]` markup and the first occurrence of each alias become links. Without a store the
/// text is escaped instead.
pub fn format_line_with(entry: &JournalEntry, store: Option<&CodexStore>) -> String {
    let text = match store {
        Some(store) if !store.is_empty() => codex_text::format_auto(store, &entry.text),
        _ => escape(&entry.text),
    };
    let Some(LineStyle { color, bold, glyph }) = entry.style else {
        return text;
    };
    let mut body = if glyph.is_empty() {
        text
    } else {
        format!("{glyph} {text}")
    };
    if bold {
        body = format!("[b]{body}[/b]");
    }
    format!("[color={color}]{body}[/color]")
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '[' => out.push_str("[lb]"),
            ']' => out.push_str("[rb]"),
            _ => out.push(c),
        }
    }
    out
}

fn with_header(new: Vec<Line>, date: &str, previous: &[Line]) -> Vec<Line> {
    let mut block = vec![(String::new(), format!("[b]— {} —[/b]", escape(date)))];
    block.extend(new);
    block.extend(previous.iter().cloned());
    block.truncate(MAX_LOG_LINES);
    block
}

impl ChronicleLog {
    /// Adds a resolved turn on top of the history.
    pub fn record(&mut self, turn: &TurnJournal) {
        self.record_with(turn, None);
    }

    /// [`Self::record`] with the Codex links of the lines.
    pub fn record_with(&mut self, turn: &TurnJournal, store: Option<&CodexStore>) {
        self.turns += 1;
        let mut new_lines: Vec<Line> = turn
            .entries
            .iter()
            .map(|e| (e.genre.clone(), format_line_with(e, store)))
            .collect();
        if new_lines.is_empty() {
            new_lines.push((String::new(), "[i]Rien à signaler.[/i]".to_owned()));
        }
        self.lines = with_header(new_lines, &turn.date_label, &self.lines);
        if !turn.world.is_empty() {
            let new_world = turn
                .world
                .iter()
                .map(|e| (e.genre.clone(), format_line_with(e, store)))
                .collect();
            self.world = with_header(new_world, &turn.date_label, &self.world);
        }
        for letter in &turn.letters {
            self.letters.insert(0, letter.clone());
        }
        self.letters.truncate(MAX_LETTERS);
        self.last_expired = turn.expired.clone();
        self.newest_kinds = turn.entries.iter().map(|e| e.kind.clone()).collect();
    }

    /// Journal lines of `genre` (date headers only before a matching line).
    pub fn shown_lines(&self, genre: &str) -> Vec<Line> {
        chronicle::filter_lines(&self.lines, genre)
            .into_iter()
            .cloned()
            .collect()
    }

    /// « Monde » lines of `genre`.
    pub fn shown_world(&self, genre: &str) -> Vec<Line> {
        chronicle::filter_lines(&self.world, genre)
            .into_iter()
            .cloned()
            .collect()
    }

    /// News lines of « Monde » without the date headers.
    pub fn world_count(&self) -> usize {
        self.world.iter().filter(|(g, _)| !g.is_empty()).count()
    }

    /// Journal lines without the date headers or the « Rien à signaler » filler.
    pub fn line_count(&self) -> usize {
        self.lines.iter().filter(|(g, _)| !g.is_empty()).count()
    }

    /// Drops the letter at `index` (0 = most recent).
    pub fn dismiss(&mut self, index: usize) {
        if index < self.letters.len() {
            self.letters.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: &str, genre: &str, text: &str) -> JournalEntry {
        JournalEntry {
            kind: kind.into(),
            genre: genre.into(),
            text: text.into(),
            faction: None,
            province: None,
            relevance: "player".into(),
            relevance_label: "Votre royaume".into(),
            public: false,
            loss: false,
            style: chronicle::line_style(kind),
        }
    }

    fn turn(date: &str, entries: Vec<JournalEntry>) -> TurnJournal {
        TurnJournal {
            turn: 1,
            date_label: date.into(),
            entries,
            world: Vec::new(),
            letters: Vec::new(),
            expired: Vec::new(),
        }
    }

    #[test]
    fn lines_are_grouped_by_date_newest_first() {
        let mut log = ChronicleLog::default();
        log.record(&turn(
            "Printemps 1337",
            vec![entry("battle", "war", "Bataille")],
        ));
        log.record(&turn(
            "Été 1337",
            vec![entry("marriage", "diplomacy", "Noces")],
        ));
        assert_eq!(log.lines[0].1, "[b]— Été 1337 —[/b]");
        assert_eq!(log.lines.len(), 4);
        assert_eq!(log.line_count(), 2);
        let war = log.shown_lines("war");
        assert_eq!(war.len(), 2);
        assert!(war[1].1.contains("Bataille"));
    }

    #[test]
    fn lines_get_codex_links_with_a_store() {
        let store = CodexStore::parse(
            r#"{"entries":[{"id":"cdx_crecy","title":"Crécy","category":"bataille","aliases":["Crécy"],"summary":"s"}]}"#
                .as_bytes(),
        )
        .unwrap();
        let mut log = ChronicleLog::default();
        log.record_with(
            &turn(
                "A",
                vec![entry(
                    "battle",
                    "war",
                    "Victoire à Crécy, voir [[cdx_crecy|la bataille]]",
                )],
            ),
            Some(&store),
        );
        let line = &log.lines[1].1;
        assert!(line.contains("[url=cdx:cdx_crecy]"), "{line}");
        assert!(line.starts_with("[color="));
    }

    #[test]
    fn empty_turn_says_nothing_to_report_and_history_is_bounded() {
        let mut log = ChronicleLog::default();
        log.record(&turn("A", vec![]));
        assert!(log.lines[1].1.contains("Rien à signaler"));
        for i in 0..300 {
            log.record(&turn(&format!("T{i}"), vec![entry("battle", "war", "x")]));
        }
        assert_eq!(log.lines.len(), MAX_LOG_LINES);
    }

    #[test]
    fn format_line_styles_and_escapes() {
        let line = format_line(&entry("battle", "war", "Siège de [Rouen]"));
        assert_eq!(
            line,
            "[color=#8b1a1a][b]⚔ Siège de [lb]Rouen[rb][/b][/color]"
        );
        assert_eq!(format_line(&entry("zzz", "other", "brut")), "brut");
    }

    #[test]
    fn letters_are_capped_and_dismissable() {
        let mut log = ChronicleLog::default();
        let letter = |n: usize| NewsLetter {
            kind: "death".into(),
            rubric: "Décès".into(),
            title: format!("t{n}"),
            text: format!("t{n}"),
            faction: None,
            province: None,
            interest: "Votre royaume".into(),
            wax: "dark".into(),
        };
        let mut t = turn("A", vec![]);
        t.letters = (0..50).map(letter).collect();
        log.record(&t);
        assert_eq!(log.letters.len(), MAX_LETTERS);
        assert_eq!(log.letters[0].title, "t49");
        log.dismiss(0);
        assert_eq!(log.letters[0].title, "t48");
        log.dismiss(99);
        assert_eq!(log.letters.len(), MAX_LETTERS - 1);
    }
}
