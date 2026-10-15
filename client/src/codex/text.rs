//! Codex text: `[[cdx_id]]` / `[[cdx_id|label]]` links of `data/` turned into BBCode
//! `[url=cdx:id]` for the kit's `rich_text` (port of game/scripts/codex/codex_text.gd).
//!
//! The link is rubric red and underlined while the entry is unread, brown afterwards;
//! `[[!text]]` renders `text` as plain text. Unknown ids render their label (or title) only.
//! The auto-linking of aliases in simulation texts (`auto_link`) is not ported: no Bevy screen
//! shows such text yet.

use super::store::CodexStore;

/// Prefix of the link target of a Codex entry.
pub const META_PREFIX: &str = "cdx:";
pub const UNREAD_COLOR: &str = "#8b1a1a";
pub const READ_COLOR: &str = "#5a3a1a";

/// One `[[…]]` occurrence: `(start, end, id, label)`; an escape has an id starting with `!`.
fn links(text: &str) -> Vec<(usize, usize, String, String)> {
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(open) = text[cursor..].find("[[") {
        let start = cursor + open;
        let Some(close) = text[start + 2..].find("]]") else {
            break;
        };
        let end = start + 2 + close + 2;
        let inner = &text[start + 2..end - 2];
        if inner.is_empty() || inner.contains('[') {
            cursor = start + 2;
            continue;
        }
        let (id, label) = inner.split_once('|').unwrap_or((inner, ""));
        found.push((start, end, id.to_owned(), label.to_owned()));
        cursor = end;
    }
    found
}

/// BBCode of a link to `id` displayed `label` (the entry title when empty).
pub fn link(store: &CodexStore, id: &str, label: &str) -> String {
    if !store.has(id) {
        return if label.is_empty() {
            id.to_owned()
        } else {
            label.to_owned()
        };
    }
    let label = if label.is_empty() {
        store.title(id)
    } else {
        label.to_owned()
    };
    if store.is_discovered(id) {
        format!("[url={META_PREFIX}{id}][color={READ_COLOR}]{label}[/color][/url]")
    } else {
        format!("[url={META_PREFIX}{id}][color={UNREAD_COLOR}][u]{label}[/u][/color][/url]")
    }
}

/// `text` with its `[[…]]` replaced by BBCode links.
pub fn format(store: &CodexStore, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, id, label) in links(text) {
        out.push_str(&text[cursor..start]);
        if let Some(escaped) = id.strip_prefix('!') {
            out.push_str(escaped);
            if !label.is_empty() {
                out.push('|');
                out.push_str(&label);
            }
        } else {
            out.push_str(&link(store, id.trim(), label.trim()));
        }
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

/// `text` without the `[[…]]` markup (label only; the id is kept when there is none).
pub fn plain_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, id, label) in links(text) {
        out.push_str(&text[cursor..start]);
        if let Some(escaped) = id.strip_prefix('!') {
            out.push_str(escaped);
        } else if label.is_empty() {
            out.push_str(id.trim());
        } else {
            out.push_str(label.trim());
        }
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

/// Entry id of a link target (`cdx:cdx_abbaye`), `None` for other targets.
pub fn target_id(target: &str) -> Option<&str> {
    target.strip_prefix(META_PREFIX)
}

/// Lowercase without accents, typographic apostrophe as a straight one (search « epee » finds
/// « Épée »).
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .flat_map(|character| {
            match character {
                'à' | 'â' | 'ä' | 'á' => "a",
                'ç' => "c",
                'é' | 'è' | 'ê' | 'ë' => "e",
                'î' | 'ï' | 'í' => "i",
                'ô' | 'ö' | 'ó' => "o",
                'ù' | 'û' | 'ü' | 'ú' => "u",
                'ÿ' => "y",
                'œ' => "oe",
                'æ' => "ae",
                '’' => "'",
                _ => return character.to_string().chars().collect::<Vec<_>>(),
            }
            .chars()
            .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> CodexStore {
        CodexStore::parse(
            br#"{"entries":[{"id":"cdx_a","title":"Abbaye"},{"id":"cdx_b","title":"Bataille"}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn links_become_url_spans_underlined_until_read() {
        let mut store = store();
        let unread = format(&store, "Voir [[cdx_a|l'abbaye]] et [[cdx_b]].");
        assert_eq!(
            unread,
            "Voir [url=cdx:cdx_a][color=#8b1a1a][u]l'abbaye[/u][/color][/url] et [url=cdx:cdx_b][color=#8b1a1a][u]Bataille[/u][/color][/url]."
        );
        store.use_save_file(
            std::env::temp_dir().join(format!("codex-text-{}.json", std::process::id())),
        );
        store.discover("cdx_a");
        let read = format(&store, "[[cdx_a|x]]");
        assert_eq!(read, "[url=cdx:cdx_a][color=#5a3a1a]x[/color][/url]");
        let _ = std::fs::remove_file(
            std::env::temp_dir().join(format!("codex-text-{}.json", std::process::id())),
        );
    }

    #[test]
    fn unknown_ids_and_escapes_are_plain() {
        let store = store();
        assert_eq!(
            format(&store, "[[cdx_zz|texte]] [[cdx_zz]]"),
            "texte cdx_zz"
        );
        assert_eq!(
            format(&store, "[[!Louis de Poitiers]]"),
            "Louis de Poitiers"
        );
        assert_eq!(
            plain_links("[[cdx_a|l'abbaye]] [[cdx_b]] [[!x]]"),
            "l'abbaye cdx_b x"
        );
        assert_eq!(plain_links("a [[ouvert"), "a [[ouvert");
    }

    #[test]
    fn fold_removes_accents() {
        assert_eq!(fold("Épée d’Œdipe"), "epee d'oedipe");
        assert_eq!(target_id("cdx:cdx_a"), Some("cdx_a"));
        assert_eq!(target_id("ib:rule:x"), None);
    }
}
