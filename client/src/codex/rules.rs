//! The « Règles » half of the hub: mechanic sheets (`data/ui/encyclopedia.json`, port of
//! game/scripts/ui/mechanic_sheet.gd) and short sheets of units, buildings, technologies and
//! resources read from `data/` (a subset of game/scripts/ui/encyclopedia.gd).
//!
//! Interface text only: the numbers quoted by the sheets (`{rule.name}`) come from the core
//! (`sim_campaign::rule_constants`), nothing is computed here. Not ported: the tabs Traits,
//! Compétences, Suite, Factions, Religion, Agents, icons and the stat tables of the Godot
//! encyclopedia.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use super::store::CodexStore;
use super::text;
use crate::ui::kit::fr;

/// One rules sheet. `body` keeps its `[[cdx_id|label]]` links (resolved when shown).
#[derive(Debug, Clone, Default)]
pub struct RulesEntry {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub body: String,
    /// Illustration under `game/assets` (`illustrations/<id>.jpg`).
    pub art: Option<String>,
    /// Folded name + id + body, for the search.
    pub search: String,
}

#[derive(Debug, Clone, Default)]
pub struct RulesCategory {
    pub id: &'static str,
    pub label: &'static str,
    pub entries: Vec<RulesEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct RulesBook {
    pub categories: Vec<RulesCategory>,
}

/// `{rule.name}` / `{rule.name:1}` replaced by the value in French format (`?` when unknown).
pub fn fill_rule_values(template: &str, constants: &BTreeMap<String, f64>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{rule.") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 6..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let (key, decimals) = match after[..end].split_once(':') {
            Some((key, decimals)) => (key, decimals.parse::<usize>().ok()),
            None => (&after[..end], None),
        };
        match constants.get(key) {
            Some(value) => out.push_str(&number(*value, decimals)),
            None => out.push('?'),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// `8.0` -> « 8 », `1.5` -> « 1,5 », `1000.0` -> « 1 000 »; `Some(1)` imposes the decimals.
pub fn number(value: f64, decimals: Option<usize>) -> String {
    if let Some(places) = decimals {
        return format!("{value:.places$}").replace('.', ",");
    }
    if (value - value.round()).abs() < 1e-9 {
        return fr::digits(value.round() as i64);
    }
    let text = format!("{value:.2}");
    text.trim_end_matches('0').replace('.', ",")
}

pub(super) fn read_json_dir(dir: &Path) -> Vec<Value> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = read
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
        .iter()
        .filter_map(|path| std::fs::read(path).ok())
        .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
        .collect()
}

pub(super) fn str_of<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

pub(super) fn display_name(value: &Value) -> String {
    match value.get("name") {
        Some(Value::String(name)) => name.clone(),
        Some(name) => name
            .get("display")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        None => String::new(),
    }
}

fn money(value: &Value) -> Option<i64> {
    match value.get("cost")? {
        Value::Number(number) => number.as_i64(),
        cost => cost.get("money").and_then(Value::as_i64),
    }
}

pub(super) fn label_of(kind: &str) -> String {
    match kind {
        "infantry" => "infanterie",
        "cavalry" => "cavalerie",
        "ranged" => "tireurs",
        "siege" => "siège",
        "naval" => "marine",
        "religious" => "religieux",
        "military" => "militaire",
        "economic" => "économie",
        "commerce" => "commerce",
        "civil" => "civil",
        "raw" => "matière première",
        "manufactured" => "produit manufacturé",
        "food" => "denrée",
        "luxury" => "luxe",
        other => return other.replace('_', " "),
    }
    .to_owned()
}

impl RulesEntry {
    pub(super) fn new(
        id: &str,
        name: String,
        subtitle: String,
        body: String,
        art_root: &Path,
    ) -> Self {
        let art = format!("illustrations/{id}.jpg");
        let art = art_root.join(&art).is_file().then_some(art);
        let search = text::fold(&format!("{name} {id} {}", text::plain_links(&body)));
        Self {
            id: id.to_owned(),
            name,
            subtitle,
            body,
            art,
            search,
        }
    }
}

/// Playable factions `(id, name, json)` sorted by name.
fn playable_factions(data_dir: &Path) -> Vec<(String, String, Value)> {
    let mut factions: Vec<_> = read_json_dir(&data_dir.join("factions"))
        .into_iter()
        .filter(|faction| faction.get("playable").and_then(Value::as_bool) == Some(true))
        .map(|faction| {
            (
                str_of(&faction, "id").to_owned(),
                display_name(&faction),
                faction,
            )
        })
        .collect();
    factions.sort_by_key(|(_, name, _)| text::fold(name));
    factions
}

pub(super) fn succession_label(law: &str) -> String {
    match law {
        "salic" => "loi salique (héritiers mâles par les mâles)",
        "male_preference_primogeniture" => "primogéniture à préférence masculine",
        "cognatic_primogeniture" => "primogéniture cognatique",
        "elective" => "élective",
        other => other,
    }
    .to_owned()
}

/// Sections some mechanics add from the data (objectives, succession laws, wars, class goods).
fn mechanic_extra(kind: &str, data_dir: &Path, store: &CodexStore) -> String {
    let factions = playable_factions(data_dir);
    let name_of = |id: &str, name: &str| match store.entry_for_entity(id) {
        Some(entry) => format!("[[{entry}|{name}]]"),
        None => name.to_owned(),
    };
    let lines: Vec<String> = match kind {
        "victory" => factions
            .iter()
            .map(|(id, name, faction)| {
                let victory = faction.get("victory").cloned().unwrap_or(Value::Null);
                let titles: Vec<&str> = victory
                    .get("objectives")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|objective| str_of(objective, "title"))
                    .collect();
                format!(
                    "• {} (échéance {}) : {}",
                    name_of(id, name),
                    victory.get("end_year").and_then(Value::as_i64).unwrap_or(0),
                    titles.join(", ")
                )
            })
            .collect(),
        "succession" => factions
            .iter()
            .map(|(id, name, faction)| {
                format!(
                    "• {} : {}",
                    name_of(id, name),
                    succession_label(str_of(faction, "succession_law"))
                )
            })
            .collect(),
        "relations" => {
            let names: BTreeMap<String, String> = read_json_dir(&data_dir.join("factions"))
                .iter()
                .map(|faction| (str_of(faction, "id").to_owned(), display_name(faction)))
                .collect();
            factions
                .iter()
                .filter_map(|(id, name, faction)| {
                    let wars: Vec<String> = faction
                        .get("relations")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter(|relation| str_of(relation, "status") == "war")
                        .map(|relation| {
                            let other = str_of(relation, "faction");
                            names
                                .get(other)
                                .cloned()
                                .unwrap_or_else(|| other.to_owned())
                        })
                        .collect();
                    (!wars.is_empty()).then(|| {
                        format!(
                            "• {} en guerre contre {}",
                            name_of(id, name),
                            wars.join(", ")
                        )
                    })
                })
                .collect()
        }
        "classes" => {
            let resources = read_json_dir(&data_dir.join("resources"));
            [
                ("peasants", "Paysans"),
                ("burghers", "Bourgeois"),
                ("clergy", "Clergé"),
                ("nobility", "Noblesse"),
            ]
            .iter()
            .map(|(class, label)| {
                let goods: Vec<String> = resources
                    .iter()
                    .filter(|resource| {
                        resource
                            .get("satisfies_classes")
                            .and_then(Value::as_array)
                            .is_some_and(|list| list.iter().any(|item| item == class))
                    })
                    .map(display_name)
                    .collect();
                format!("• {label} : {}", goods.join(", "))
            })
            .collect()
        }
        _ => return String::new(),
    };
    let title = match kind {
        "victory" => "Objectifs des factions jouables",
        "succession" => "Lois de succession",
        "relations" => "Guerres en 1337",
        _ => "Biens recherchés par classe",
    };
    format!("\n\n[b]{title}[/b]\n{}", lines.join("\n"))
}

impl RulesBook {
    /// Builds the book from `data/` (`ui/encyclopedia.json`, `unit_types`, `buildings`,
    /// `technologies`, `resources`, `factions`) and the core's rule constants.
    pub fn build(
        data_dir: &Path,
        asset_root: &Path,
        constants: &BTreeMap<String, f64>,
        store: &CodexStore,
    ) -> Self {
        let mut categories = Vec::new();

        let mut mechanics = RulesCategory {
            id: "mechanics",
            label: "Mécaniques",
            entries: Vec::new(),
        };
        if let Ok(bytes) = std::fs::read(data_dir.join("ui/encyclopedia.json"))
            && let Ok(value) = serde_json::from_slice::<Value>(&bytes)
        {
            for mechanic in value
                .get("mechanics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let body = fill_rule_values(str_of(mechanic, "text"), constants)
                    + &mechanic_extra(str_of(mechanic, "extra"), data_dir, store);
                mechanics.entries.push(RulesEntry::new(
                    str_of(mechanic, "id"),
                    str_of(mechanic, "name").to_owned(),
                    "Mécanique de jeu".to_owned(),
                    body,
                    asset_root,
                ));
            }
        }
        categories.push(mechanics);

        categories.push(sheet_category(
            "units",
            "Unités",
            &data_dir.join("unit_types"),
            asset_root,
            |unit| {
                let mut lines = Vec::new();
                if let Some(cost) = money(unit) {
                    lines.push(format!(
                        "[b]Coût[/b] {}, entretien {} par saison",
                        fr::amount(cost),
                        fr::amount(unit.get("upkeep").and_then(Value::as_i64).unwrap_or(0))
                    ));
                }
                if let Some(soldiers) = unit.get("soldiers").and_then(Value::as_i64) {
                    lines.push(format!("[b]Effectif[/b] {soldiers} soldats"));
                }
                if let Some(stats) = unit.get("stats").and_then(Value::as_object) {
                    let get = |key: &str| stats.get(key).and_then(Value::as_i64).unwrap_or(0);
                    lines.push(format!(
                        "[b]Mêlée[/b] {} · [b]Tir[/b] {} · [b]Armure[/b] {} · [b]Moral[/b] {} · [b]Vitesse[/b] {}",
                        get("melee"),
                        get("ranged"),
                        get("armor"),
                        get("morale"),
                        get("speed")
                    ));
                }
                let equipment = str_of(unit, "equipment");
                if !equipment.is_empty() {
                    lines.push(format!("[b]Équipement[/b] {equipment}"));
                }
                (
                    format!("Unité · {}", label_of(str_of(unit, "category"))),
                    lines,
                )
            },
        ));
        categories.push(sheet_category(
            "buildings",
            "Bâtiments",
            &data_dir.join("buildings"),
            asset_root,
            |building| {
                let mut lines = Vec::new();
                if let Some(cost) = money(building) {
                    lines.push(format!(
                        "[b]Coût[/b] {}, {} tours de construction, entretien {} par saison",
                        fr::amount(cost),
                        building
                            .get("build_time_turns")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        fr::amount(building.get("upkeep").and_then(Value::as_i64).unwrap_or(0))
                    ));
                }
                (
                    format!(
                        "Bâtiment · {} · niveau {}",
                        label_of(str_of(building, "category")),
                        building.get("tier").and_then(Value::as_i64).unwrap_or(1)
                    ),
                    lines,
                )
            },
        ));
        categories.push(sheet_category(
            "technologies",
            "Technologies",
            &data_dir.join("technologies"),
            asset_root,
            |tech| {
                let lines = tech
                    .get("cost")
                    .and_then(Value::as_i64)
                    .map(|cost| format!("[b]Coût[/b] {} points de recherche", fr::digits(cost)))
                    .into_iter()
                    .collect();
                (
                    format!(
                        "Technologie · {} · niveau {}",
                        str_of(tech, "branch").replace('_', " "),
                        tech.get("tier").and_then(Value::as_i64).unwrap_or(1)
                    ),
                    lines,
                )
            },
        ));
        categories.push(sheet_category(
            "resources",
            "Ressources",
            &data_dir.join("resources"),
            asset_root,
            |resource| {
                let lines = resource
                    .get("base_price")
                    .and_then(Value::as_i64)
                    .map(|price| format!("[b]Prix de base[/b] {}", fr::amount(price)))
                    .into_iter()
                    .collect();
                (
                    format!("Ressource · {}", label_of(str_of(resource, "category"))),
                    lines,
                )
            },
        ));
        categories.extend(super::encyclopedia::extra_categories(
            data_dir, asset_root, store,
        ));
        Self { categories }
    }

    pub fn category(&self, index: Option<usize>) -> Vec<(usize, &RulesEntry)> {
        self.categories
            .iter()
            .enumerate()
            .filter(|(i, _)| index.is_none_or(|wanted| wanted == *i))
            .flat_map(|(i, category)| category.entries.iter().map(move |entry| (i, entry)))
            .collect()
    }

    pub fn entry(&self, id: &str) -> Option<&RulesEntry> {
        self.categories
            .iter()
            .flat_map(|category| category.entries.iter())
            .find(|entry| entry.id == id)
    }

    pub fn len(&self) -> usize {
        self.categories.iter().map(|c| c.entries.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn match_count(&self, query: &str) -> usize {
        let needle = text::fold(query.trim());
        self.categories
            .iter()
            .flat_map(|category| category.entries.iter())
            .filter(|entry| entry.search.contains(&needle))
            .count()
    }
}

fn sheet_category(
    id: &'static str,
    label: &'static str,
    dir: &Path,
    asset_root: &Path,
    describe: impl Fn(&Value) -> (String, Vec<String>),
) -> RulesCategory {
    let mut entries: Vec<RulesEntry> = read_json_dir(dir)
        .iter()
        .map(|definition| {
            let (subtitle, lines) = describe(definition);
            let mut body = str_of(definition, "description").to_owned();
            if !lines.is_empty() {
                body.push_str("\n\n");
                body.push_str(&lines.join("\n"));
            }
            RulesEntry::new(
                str_of(definition, "id"),
                display_name(definition),
                subtitle,
                body,
                asset_root,
            )
        })
        .collect();
    entries.sort_by_cached_key(|entry| text::fold(&entry.name));
    RulesCategory { id, label, entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_values_use_french_format() {
        let constants: BTreeMap<String, f64> = [("a".to_owned(), 1000.0), ("b".to_owned(), 1.5)]
            .into_iter()
            .collect();
        assert_eq!(
            fill_rule_values(
                "{rule.a} / {rule.b} / {rule.b:2} / {rule.zz} fin",
                &constants
            ),
            format!("{} / 1,5 / 1,50 / ? fin", fr::digits(1000))
        );
    }

    #[test]
    fn real_book_has_mechanics_and_sheets() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let data = crate::paths::data_dir(&root);
        let assets = crate::paths::asset_root(&root);
        let store = CodexStore::load(&data, &assets).unwrap();
        let book = RulesBook::build(&data, &assets, &BTreeMap::new(), &store);
        assert_eq!(book.categories[0].entries.len(), 13);
        assert!(book.categories[1].entries.len() >= 30);
        let victory = book.entry("mech_victory").unwrap();
        assert!(victory.body.contains("Objectifs des factions jouables"));
        assert!(book.match_count("moral") >= 1);
    }
}
