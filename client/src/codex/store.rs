//! Codex entries and the player's discoveries (port of game/scripts/codex/codex_store.gd and the
//! list / search logic of codex_window.gd).
//!
//! Entries come from `data/codex_bundle.json` (generated from `data/codex/cdx_*.json`, schema
//! `data/schemas/codex_bundle.schema.json`). Discoveries are a meta-progression, not a game
//! state: a JSON file `{"version": 1, "discovered": [ids]}` next to the settings file
//! (`codex.json`, overridable with `CENT_ANS_CODEX_PATH`). No game rule lives here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bevy::prelude::Resource;
use serde_json::Value;

use super::text;

/// Environment variable overriding the discoveries file (tests, background runs).
pub const PATH_ENV: &str = "CENT_ANS_CODEX_PATH";
/// Bundle file, relative to `data/`.
pub const BUNDLE_FILE: &str = "codex_bundle.json";

/// Big families of the window: label, categories, short tab label.
pub const FAMILIES: [(&str, &[&str], &str); 11] = [
    (
        "Personnages et dynasties",
        &["personnage", "dynastie"],
        "Personnages",
    ),
    ("Lieux", &["lieu"], "Lieux"),
    ("Guerre et batailles", &["guerre", "bataille"], "Guerre"),
    (
        "Société et institutions",
        &[
            "institution",
            "societe",
            "economie",
            "evenement",
            "vie_quotidienne",
        ],
        "Société",
    ),
    ("Religion", &["religion"], "Religion"),
    ("Table", &["cuisine", "ingredient", "recette"], "Table"),
    ("Médecine et herbier", &["medecine", "plante"], "Médecine"),
    ("Nature", &["arbre", "animal", "oiseau", "roche"], "Nature"),
    ("Savoirs", &["savoir"], "Savoirs"),
    (
        "Armées, navires et bâtiments",
        &["unite", "batiment", "technique"],
        "Armées",
    ),
    ("Mécaniques du jeu", &["mecanique"], "Le jeu"),
];

/// Display label of a category.
pub fn category_label(category: &str) -> String {
    match category {
        "personnage" => "Personnage",
        "dynastie" => "Dynastie",
        "lieu" => "Lieu",
        "bataille" => "Bataille",
        "evenement" => "Événement",
        "institution" => "Institution",
        "societe" => "Société",
        "guerre" => "Guerre",
        "religion" => "Religion",
        "economie" => "Économie",
        "cuisine" => "Cuisine",
        "ingredient" => "Ingrédient",
        "recette" => "Recette",
        "medecine" => "Médecine",
        "plante" => "Plante",
        "savoir" => "Savoir",
        "vie_quotidienne" => "Vie quotidienne",
        "mecanique" => "Mécanique du jeu",
        "batiment" => "Bâtiment",
        "unite" => "Unité",
        "technique" => "Technique",
        "arbre" => "Arbre",
        "animal" => "Animal",
        "oiseau" => "Oiseau",
        "roche" => "Roche",
        other => return capitalize(other),
    }
    .to_owned()
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + &chars.as_str().replace('_', " ")
    })
}

/// One Codex entry (`codex_bundle.json`).
#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub category: String,
    pub aliases: Vec<String>,
    pub era_from: String,
    pub era_to: String,
    pub summary: String,
    pub body: String,
    pub gameplay: String,
    pub anachronism: String,
    pub see_also: Vec<String>,
    pub sources: Vec<String>,
    /// Game entity this entry describes (`chr_…`, `tech_…`, `unit_…`), possibly empty.
    pub entity: String,
    /// Latin name of a species (Nature entries).
    pub latin: String,
    /// Decor keys (`tree:oak`, `fauna:animal_wolf_grey`) the entry documents (ADR 0219).
    pub decor: Vec<String>,
    /// Expressions (lowercase) in which an alias is not auto-linked (homonyms, B8).
    pub exclude_contexts: Vec<String>,
}

fn string_of(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn strings_of(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().map(|item| string_of(Some(item))).collect())
        .unwrap_or_default()
}

impl Entry {
    fn from_json(value: &Value) -> Option<Self> {
        let id = value.get("id")?.as_str()?.to_owned();
        let era = value.get("era");
        Some(Self {
            title: string_of(value.get("title")),
            category: string_of(value.get("category")),
            aliases: strings_of(value.get("aliases")),
            era_from: string_of(era.and_then(|era| era.get("from"))),
            era_to: string_of(era.and_then(|era| era.get("to"))),
            summary: string_of(value.get("summary")),
            body: string_of(value.get("body")),
            gameplay: string_of(value.get("gameplay")),
            anachronism: string_of(value.get("anachronism")),
            see_also: strings_of(value.get("see_also")),
            sources: strings_of(value.get("sources")),
            entity: string_of(value.get("entity")),
            latin: string_of(value.get("latin")),
            decor: strings_of(value.get("decor")),
            exclude_contexts: strings_of(value.get("exclude_contexts"))
                .iter()
                .map(|context| plain_key(context))
                .filter(|context| !context.is_empty())
                .collect(),
            id,
        })
    }

    /// « 1338 – 1380 », « 1346 » or empty.
    pub fn era_label(&self) -> String {
        let (from, to) = (&self.era_from, &self.era_to);
        if !from.is_empty() && !to.is_empty() && from != to {
            format!("{from} – {to}")
        } else if from.is_empty() {
            to.clone()
        } else {
            from.clone()
        }
    }

    pub fn category_label(&self) -> String {
        category_label(&self.category)
    }

    /// True when `needle` (already folded, see [`text::fold`]) is in the title, aliases or summary.
    pub fn matches(&self, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        let haystack = format!(
            "{} {} {}",
            self.title,
            self.aliases.join(" "),
            text::plain_links(&self.summary)
        );
        text::fold(&haystack).contains(needle)
    }
}

/// Alias key: trimmed lowercase, typographic apostrophe straightened (`CodexStore._plain`).
pub fn plain_key(text: &str) -> String {
    text.trim().to_lowercase().replace('’', "'")
}

/// Auto-link index: aliases (lowercase chars) longest first, with the entry they designate.
#[derive(Debug, Default, Clone)]
pub struct AliasIndex {
    pub aliases: Vec<(Vec<char>, String)>,
    /// First char -> indexes into `aliases` (keeps the longest-first order).
    pub by_first: BTreeMap<char, Vec<usize>>,
}

/// All Codex entries plus the discoveries.
#[derive(Resource, Debug, Default)]
pub struct CodexStore {
    entries: BTreeMap<String, Entry>,
    by_entity: BTreeMap<String, String>,
    by_decor: BTreeMap<String, String>,
    discovered: BTreeSet<String>,
    alias_index: AliasIndex,
    save_path: Option<PathBuf>,
    asset_root: Option<PathBuf>,
}

impl CodexStore {
    /// Reads `<data_dir>/codex_bundle.json`.
    pub fn load(data_dir: &Path, asset_root: &Path) -> Result<Self, String> {
        let path = data_dir.join(BUNDLE_FILE);
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut store = Self::parse(&bytes)?;
        store.asset_root = Some(asset_root.to_path_buf());
        Ok(store)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let mut store = Self::default();
        let mut file_order: Vec<String> = Vec::new();
        for entry in value
            .get("entries")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Entry::from_json)
        {
            file_order.push(entry.id.clone());
            if !entry.entity.is_empty() {
                store
                    .by_entity
                    .insert(entry.entity.clone(), entry.id.clone());
            }
            for key in &entry.decor {
                store.by_decor.insert(key.clone(), entry.id.clone());
            }
            store.entries.insert(entry.id.clone(), entry);
        }
        store.build_aliases(&file_order);
        Ok(store)
    }

    /// Titles and aliases (first entry of the bundle wins a shared form, minimum 2 chars).
    fn build_aliases(&mut self, file_order: &[String]) {
        let mut taken: BTreeMap<String, String> = BTreeMap::new();
        for id in file_order {
            let Some(entry) = self.entries.get(id) else {
                continue;
            };
            for form in std::iter::once(&entry.title).chain(&entry.aliases) {
                let key = plain_key(form);
                if key.chars().count() >= 2 {
                    taken.entry(key).or_insert_with(|| id.clone());
                }
            }
        }
        let mut aliases: Vec<(Vec<char>, String)> = taken
            .into_iter()
            .map(|(key, id)| (key.chars().collect(), id))
            .collect();
        aliases.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
        let mut by_first: BTreeMap<char, Vec<usize>> = BTreeMap::new();
        for (index, (chars, _)) in aliases.iter().enumerate() {
            by_first.entry(chars[0]).or_default().push(index);
        }
        self.alias_index = AliasIndex { aliases, by_first };
    }

    pub fn alias_index(&self) -> &AliasIndex {
        &self.alias_index
    }

    /// Default discoveries file: `CENT_ANS_CODEX_PATH`, else `codex.json` beside the settings file.
    pub fn default_save_path() -> PathBuf {
        std::env::var_os(PATH_ENV).map_or_else(
            || crate::settings::Settings::file_path().with_file_name("codex.json"),
            PathBuf::from,
        )
    }

    /// Loads the discoveries of `path` (missing file: none) and saves there from now on.
    pub fn use_save_file(&mut self, path: PathBuf) {
        self.discovered.clear();
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(value) = serde_json::from_str::<Value>(&text)
        {
            for id in strings_of(value.get("discovered")) {
                self.discovered.insert(id);
            }
        }
        self.save_path = Some(path);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn has(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.get(id)
    }

    /// Title of `id`, the id itself when unknown.
    pub fn title(&self, id: &str) -> String {
        self.entry(id)
            .map_or_else(|| id.to_owned(), |entry| entry.title.clone())
    }

    /// Entry about a game entity (`chr_…`, `evt_…`, `tech_…`).
    pub fn entry_for_entity(&self, entity: &str) -> Option<&str> {
        self.by_entity.get(entity).map(String::as_str)
    }

    /// Entry of a natural decor element (`tree:oak`, `fauna:animal_wolf_grey`, …) (ADR 0219).
    pub fn entry_for_decor(&self, kind: &str, species: &str) -> Option<&str> {
        self.by_decor
            .get(&format!("{kind}:{species}"))
            .map(String::as_str)
    }

    /// Ids of a family (index of [`FAMILIES`], `None` = all) sorted by folded title.
    pub fn ids_in_family(&self, family: Option<usize>) -> Vec<&str> {
        let categories = family.and_then(|index| FAMILIES.get(index)).map(|f| f.1);
        let mut ids: Vec<&Entry> = self
            .entries
            .values()
            .filter(|entry| categories.is_none_or(|list| list.contains(&entry.category.as_str())))
            .collect();
        ids.sort_by_cached_key(|entry| text::fold(&entry.title));
        ids.into_iter().map(|entry| entry.id.as_str()).collect()
    }

    /// Number of entries (all families) answering `query`.
    pub fn match_count(&self, query: &str) -> usize {
        let needle = text::fold(query.trim());
        self.entries
            .values()
            .filter(|entry| entry.matches(&needle))
            .count()
    }

    pub fn is_discovered(&self, id: &str) -> bool {
        self.discovered.contains(id)
    }

    /// Marks the entry discovered (saved at once); true when it is new.
    pub fn discover(&mut self, id: &str) -> bool {
        if !self.entries.contains_key(id) || !self.discovered.insert(id.to_owned()) {
            return false;
        }
        self.save();
        true
    }

    pub fn discovered_count(&self) -> usize {
        self.discovered
            .iter()
            .filter(|id| self.entries.contains_key(*id))
            .count()
    }

    fn save(&self) {
        let Some(path) = &self.save_path else {
            return;
        };
        let value = serde_json::json!({"version": 1, "discovered": self.discovered});
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = std::fs::write(path, value.to_string()) {
            bevy::log::warn!("codex discoveries not saved ({}): {error}", path.display());
        }
    }

    /// Illustration of an entry (path under `game/assets`, `None` when none): its own
    /// `illustrations/<id>.jpg`, the portrait `portraits/chr_<slug>.png`, then those of its entity.
    pub fn art_of(&self, id: &str) -> Option<String> {
        let root = self.asset_root.as_ref()?;
        let entity = self.entry(id).map_or("", |entry| entry.entity.as_str());
        let mut candidates = vec![
            format!("illustrations/{id}.jpg"),
            format!("portraits/chr_{}.png", id.trim_start_matches("cdx_")),
        ];
        if !entity.is_empty() {
            candidates.extend([
                format!("events/{entity}.jpg"),
                format!("illustrations/{entity}.jpg"),
                format!("portraits/{entity}.png"),
                format!("icons/entity/{entity}.png"),
            ]);
        }
        candidates
            .into_iter()
            .find(|path| root.join(path).is_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUNDLE: &str = r#"{"version":1,"entries":[
      {"id":"cdx_b","title":"Étienne Marcel","category":"personnage","aliases":["prévôt"],"era":{"from":"1315","to":"1358"},"summary":"Prévôt des [[cdx_a|marchands]].","body":"x","entity":"chr_marcel"},
      {"id":"cdx_a","title":"Abbaye","category":"batiment","era":{"from":"529","to":"529"},"summary":"s","body":"b","see_also":["cdx_b"]},
      {"id":"cdx_c","title":"Chêne","category":"arbre","latin":"Quercus","decor":["tree:oak"],"summary":"s","body":"b"}
    ]}"#;

    fn store() -> CodexStore {
        CodexStore::parse(BUNDLE.as_bytes()).unwrap()
    }

    #[test]
    fn parses_entries_and_indexes() {
        let store = store();
        assert_eq!(store.len(), 3);
        assert_eq!(store.entry_for_entity("chr_marcel"), Some("cdx_b"));
        assert_eq!(store.entry_for_decor("tree", "oak"), Some("cdx_c"));
        assert_eq!(store.entry("cdx_b").unwrap().era_label(), "1315 – 1358");
        assert_eq!(store.entry("cdx_a").unwrap().era_label(), "529");
        assert_eq!(store.entry("cdx_c").unwrap().latin, "Quercus");
    }

    #[test]
    fn families_filter_and_sort_by_folded_title() {
        let store = store();
        assert_eq!(store.ids_in_family(None), ["cdx_a", "cdx_c", "cdx_b"]);
        assert_eq!(store.ids_in_family(Some(0)), ["cdx_b"]);
        assert_eq!(store.ids_in_family(Some(7)), ["cdx_c"]);
    }

    #[test]
    fn search_ignores_accents_case_and_links() {
        let store = store();
        assert_eq!(store.match_count("etienne"), 1);
        assert_eq!(store.match_count("PRÉVÔT"), 1);
        assert_eq!(store.match_count("marchands"), 1);
        assert_eq!(store.match_count("cdx_a"), 0);
        assert_eq!(store.match_count(""), 3);
    }

    #[test]
    fn discoveries_round_trip_through_the_save_file() {
        let dir = std::env::temp_dir().join(format!("codex-test-{}", std::process::id()));
        let path = dir.join("codex.json");
        let mut store = store();
        store.use_save_file(path.clone());
        assert!(store.discover("cdx_a"));
        assert!(!store.discover("cdx_a"));
        assert!(!store.discover("cdx_missing"));
        let mut again = self::store();
        again.use_save_file(path);
        assert!(again.is_discovered("cdx_a"));
        assert_eq!(again.discovered_count(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn real_bundle_loads_when_present() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let store = CodexStore::load(
            &crate::paths::data_dir(&root),
            &crate::paths::asset_root(&root),
        )
        .unwrap();
        assert!(store.len() > 500);
        assert!(store.ids_in_family(Some(7)).len() >= 60, "Nature entries");
        assert!(store.art_of("cdx_chamois").is_some());
    }
}
