//! Codex text: `[[cdx_id]]` / `[[cdx_id|label]]` links of `data/` turned into BBCode
//! `[url=cdx:id]` for the kit's `rich_text` (port of game/scripts/codex/codex_text.gd).
//!
//! The link is rubric red and underlined while the entry is unread, brown afterwards;
//! `[[!text]]` renders `text` as plain text. Unknown ids render their label (or title) only.
//! [`auto_link`] links the first occurrence (whole word, case ignored) of each alias of the Codex
//! in free texts produced by the simulation, outside tags and existing links; the longest alias
//! wins at a position and an entry may list `exclude_contexts` (B8 homonyms).

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
    format_with(store, text, false)
}

/// [`format`] then [`auto_link`]: for free texts of the simulation (chronicle, messages).
/// `[[!text]]` stays plain and is never auto-linked.
pub fn format_auto(store: &CodexStore, text: &str) -> String {
    auto_link(store, &format_with(store, text, true))
        .replace("[lang=fr]", "")
        .replace("[/lang]", "")
}

fn format_with(store: &CodexStore, text: &str, protect_escapes: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, id, label) in links(text) {
        out.push_str(&text[cursor..start]);
        if let Some(escaped) = id.strip_prefix('!') {
            if protect_escapes {
                out.push_str("[lang=fr]");
            }
            out.push_str(escaped);
            if !label.is_empty() {
                out.push('|');
                out.push_str(&label);
            }
            if protect_escapes {
                out.push_str("[/lang]");
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

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn lower_char(c: char) -> char {
    if c == '’' {
        '\''
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

/// Links the aliases found in one text segment (no tags in it).
fn link_segment(
    store: &CodexStore,
    segment: &str,
    linked: &mut std::collections::BTreeSet<String>,
) -> String {
    let chars: Vec<char> = segment.chars().collect();
    let lowered: Vec<char> = chars.iter().copied().map(lower_char).collect();
    let index = store.alias_index();
    let mut out = String::with_capacity(segment.len() + 64);
    let mut i = 0;
    while i < chars.len() {
        let mut matched = None;
        let boundary_before = i == 0
            || !(is_word(chars[i - 1])
                || (chars[i - 1] == '-' && i >= 2 && chars[i - 2].is_alphanumeric()));
        if boundary_before && let Some(candidates) = index.by_first.get(&lowered[i]) {
            for &candidate in candidates {
                let (alias, id) = &index.aliases[candidate];
                let end = i + alias.len();
                if end > chars.len() || lowered[i..end] != alias[..] {
                    continue;
                }
                let boundary_after = end == chars.len()
                    || !(is_word(chars[end])
                        || (chars[end] == '-'
                            && chars.get(end + 1).is_some_and(|c| c.is_alphanumeric())));
                if boundary_after {
                    matched = Some((end, id));
                    break;
                }
            }
        }
        let Some((end, id)) = matched else {
            out.push(chars[i]);
            i += 1;
            continue;
        };
        let original: String = chars[i..end].iter().collect();
        if linked.contains(id.as_str()) || excluded(store, id, &lowered, i, end) {
            out.push_str(&original);
        } else {
            linked.insert(id.clone());
            out.push_str(&link(store, id, &original));
        }
        i = end;
    }
    out
}

/// True when the occurrence `[start, end)` belongs to one of the excluded expressions of `id`.
fn excluded(store: &CodexStore, id: &str, lowered: &[char], start: usize, end: usize) -> bool {
    let Some(entry) = store.entry(id) else {
        return false;
    };
    let alias = &lowered[start..end];
    entry.exclude_contexts.iter().any(|context| {
        let context: Vec<char> = context.chars().collect();
        (0..context.len().saturating_sub(alias.len()) + 1).any(|offset| {
            context[offset..].starts_with(alias)
                && start >= offset
                && lowered.get(start - offset..start - offset + context.len()) == Some(&context[..])
        })
    })
}

/// `bbcode` with the first occurrence of each Codex alias linked, outside tags, `[url]`, `[img]`
/// and `[lang]` passages.
pub fn auto_link(store: &CodexStore, bbcode: &str) -> String {
    let mut out = String::with_capacity(bbcode.len() + 128);
    let mut linked = std::collections::BTreeSet::new();
    let mut skip_depth = 0usize;
    let mut rest = bbcode;
    while !rest.is_empty() {
        // Next tag: `[` + optional `/` + letter, closed by `]`.
        let tag = rest.match_indices('[').find_map(|(at, _)| {
            let after = rest[at + 1..].trim_start_matches('/');
            let starts_tag = after
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic());
            let close = rest[at..].find(']')?;
            starts_tag.then_some((at, at + close + 1))
        });
        let (text_end, tag_end) = tag.unwrap_or((rest.len(), rest.len()));
        let segment = &rest[..text_end];
        if skip_depth > 0 || segment.trim().is_empty() {
            out.push_str(segment);
        } else {
            out.push_str(&link_segment(store, segment, &mut linked));
        }
        let tag_text = &rest[text_end..tag_end];
        if tag_text.starts_with("[url")
            || tag_text.starts_with("[img")
            || tag_text.starts_with("[lang")
        {
            skip_depth += 1;
        } else if matches!(tag_text, "[/url]" | "[/img]" | "[/lang]") {
            skip_depth = skip_depth.saturating_sub(1);
        }
        out.push_str(tag_text);
        rest = &rest[tag_end..];
    }
    out
}

/// Entry id of a link target (`cdx:cdx_abbaye`), `None` for other targets.
pub fn target_id(target: &str) -> Option<&str> {
    target.strip_prefix(META_PREFIX)
}

/// Prefix of the link target of an interface bubble (`ib:<kind>:<id>`: entity or rule).
pub const IB_PREFIX: &str = "ib:";

/// Entity kind of an id by its prefix (`RichTooltip.ENTITY_KINDS`), `None` when unknown.
pub fn entity_kind(id: &str) -> Option<&'static str> {
    [
        ("unit_", "unit"),
        ("bld_", "building"),
        ("tech_", "technology"),
        ("res_", "resource"),
        ("trait_", "trait"),
        ("skill_", "skill"),
    ]
    .into_iter()
    .find_map(|(prefix, kind)| id.starts_with(prefix).then_some(kind))
}

/// BBCode of a link to the entity bubble `ib:<kind>:<id>` (`CodexText.ib_link`): rubric red and
/// underlined until `read`.
pub fn ib_link(kind: &str, id: &str, label: &str, read: bool) -> String {
    if label.is_empty() || id.is_empty() {
        return label.to_owned();
    }
    if read {
        format!("[url={IB_PREFIX}{kind}:{id}][color={READ_COLOR}]{label}[/color][/url]")
    } else {
        format!("[url={IB_PREFIX}{kind}:{id}][color={UNREAD_COLOR}][u]{label}[/u][/color][/url]")
    }
}

/// [`ib_link`] of a game entity whose kind follows from its id; the plain label otherwise.
pub fn entity_link(id: &str, label: &str, read: bool) -> String {
    entity_kind(id).map_or_else(|| label.to_owned(), |kind| ib_link(kind, id, label, read))
}

/// The `ib:<kind>:<id>` key of a link target, `None` for other targets.
pub fn ib_key(target: &str) -> Option<&str> {
    (target.starts_with(IB_PREFIX) && target.matches(':').count() >= 2).then_some(target)
}

/// Id of the entity behind an `ib:<kind>:<id>` key (`rule` keys have none).
pub fn ib_entity(key: &str) -> Option<&str> {
    let mut parts = key.splitn(3, ':');
    let (_, kind, id) = (parts.next()?, parts.next()?, parts.next()?);
    (kind != "rule").then_some(id)
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

    fn alias_store() -> CodexStore {
        CodexStore::parse(
            r#"{"entries":[
              {"id":"cdx_poitiers","title":"Bataille de Poitiers","aliases":["Poitiers"],"exclude_contexts":["Louis de Poitiers"]},
              {"id":"cdx_louis","title":"Louis de Poitiers"},
              {"id":"cdx_roi","title":"Roi","aliases":["roi d'Angleterre"]},
              {"id":"cdx_edouard","title":"Édouard III"}]}"#
                .as_bytes(),
        )
        .unwrap()
    }

    #[test]
    fn auto_link_first_occurrence_longest_alias_whole_words() {
        let store = alias_store();
        let out = auto_link(
            &store,
            "[b]Poitiers[/b] : le roi d’Angleterre à Poitiers, rois.",
        );
        assert_eq!(out.matches("[url=cdx:cdx_poitiers]").count(), 1);
        assert!(out.contains("[url=cdx:cdx_roi][color=#8b1a1a][u]roi d’Angleterre"));
        assert!(!out.contains("cdx:cdx_roi][color=#8b1a1a][u]roi[/u]"));
        assert!(
            out.ends_with("à Poitiers, rois."),
            "second occurrence stays plain: {out}"
        );
    }

    #[test]
    fn auto_link_skips_links_tags_and_excluded_contexts() {
        let store = alias_store();
        let out = auto_link(&store, "[url=x]Édouard III[/url] puis Édouard III.");
        assert_eq!(out.matches("cdx:cdx_edouard").count(), 1, "{out}");
        assert!(out.starts_with("[url=x]Édouard III[/url] puis [url=cdx:cdx_edouard]"));
        let out = auto_link(&store, "Louis de Poitiers, puis Poitiers");
        assert!(out.starts_with("[url=cdx:cdx_louis]"), "{out}");
        assert_eq!(out.matches("cdx:cdx_poitiers").count(), 1, "{out}");
        assert!(out.contains("puis [url=cdx:cdx_poitiers]"));
        assert_eq!(
            auto_link(&store, "[color=#fff]rien[/color]"),
            "[color=#fff]rien[/color]"
        );
    }

    #[test]
    fn ib_links_and_keys() {
        assert_eq!(entity_kind("unit_archer"), Some("unit"));
        assert_eq!(entity_kind("chr_x"), None);
        assert_eq!(
            entity_link("tech_x", "Étrier", false),
            "[url=ib:technology:tech_x][color=#8b1a1a][u]Étrier[/u][/color][/url]"
        );
        assert_eq!(entity_link("chr_x", "Jean", false), "Jean");
        assert!(ib_link("unit", "unit_a", "A", true).contains("5a3a1a"));
        assert_eq!(ib_key("ib:rule:morale"), Some("ib:rule:morale"));
        assert_eq!(ib_key("ib:x"), None);
        assert_eq!(ib_key("cdx:a"), None);
        assert_eq!(ib_entity("ib:unit:unit_archer"), Some("unit_archer"));
        assert_eq!(ib_entity("ib:rule:morale"), None);
    }

    #[test]
    fn format_auto_keeps_escapes_plain() {
        let store = alias_store();
        assert_eq!(format_auto(&store, "[[!Poitiers]] et x"), "Poitiers et x");
    }
}
