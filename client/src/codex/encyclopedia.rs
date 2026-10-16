//! Encyclopedia tabs added to the rules book: Traits, Compétences, Suite, Factions, Religion,
//! Agents (port of the matching tabs of game/scripts/ui/encyclopedia.gd, `_trait_fiche`,
//! `_skill_fiche`, `_retinue_fiche`, `_faction_fiche`, `_religion_fiche`, `_agent_fiche`).
//!
//! Data: `data/{traits,skills,factions,religions}/*.json`, `data/retinue.json`,
//! `data/rules/agents.json`. Everything is read; nothing is a rule. Ids a sheet refers to are
//! shown by name, as a Codex link when the entity has a historical sheet.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use super::rules::{RulesCategory, RulesEntry, display_name, read_json_dir, str_of};
use super::store::CodexStore;
use crate::ui::kit::fr;

/// Tab order after the first rules tabs: id, label.
pub const TABS: [(&str, &str); 6] = [
    ("traits", "Traits"),
    ("skills", "Compétences"),
    ("retinue", "Suite"),
    ("factions", "Factions"),
    ("religions", "Religion"),
    ("agents", "Agents"),
];

/// Icon of the tab of a rules category (`TABS` of game/scripts/ui/encyclopedia.gd).
pub fn tab_icon(category: &str) -> Option<&'static str> {
    Some(match category {
        "units" => "cat_unit",
        "buildings" => "cat_building",
        "technologies" => "cat_technology",
        "resources" => "cat_resource",
        "traits" => "cat_trait",
        "skills" => "cat_skill",
        "retinue" => "hud_army",
        "factions" | "agents" => "hud_diplomacy",
        "religions" => "bld_parish_church",
        "mechanics" => "hud_menu",
        _ => return None,
    })
}

/// Side of a tab icon (`icon_max_width` 18 in Godot).
pub const TAB_ICON_PX: f32 = 18.0;

/// `+5 %`, `+2`, `= 3` for an effect value (`mode`: add, percent, set…).
pub fn effect_line(effect: &Value) -> String {
    let name = str_of(effect, "effect").replace('_', " ");
    let value = effect.get("value").and_then(Value::as_f64).unwrap_or(0.0);
    let shown = if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        super::rules::number(value, Some(1))
    };
    match str_of(effect, "mode") {
        "percent" => format!("• {name} : {value:+} %", value = signed(value, &shown)),
        "set" => format!("• {name} : = {shown}"),
        _ => format!("• {name} : {value:+}", value = signed(value, &shown)),
    }
}

fn signed(value: f64, shown: &str) -> String {
    if value >= 0.0 {
        format!("+{shown}")
    } else {
        shown.to_owned()
    }
}

fn section(title: &str, lines: &[String]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("\n\n[b]{title}[/b]\n{}", lines.join("\n"))
    }
}

fn paragraph(text: &str) -> String {
    text.to_owned()
}

fn local_name(value: &Value) -> Option<String> {
    let name = value.get("name")?;
    let local = name.get("local").and_then(Value::as_str)?;
    (!local.is_empty()).then(|| local.to_owned())
}

struct Names<'a> {
    names: BTreeMap<String, String>,
    store: &'a CodexStore,
}

impl Names<'_> {
    fn link(&self, id: &str) -> String {
        let name = self.names.get(id).cloned().unwrap_or_else(|| id.to_owned());
        match self.store.entry_for_entity(id) {
            Some(entry) => format!("[[{entry}|{name}]]"),
            None => name,
        }
    }

    fn links(&self, ids: &[&str]) -> String {
        ids.iter()
            .map(|id| self.link(id))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn strs<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn sources(value: &Value) -> String {
    let list = strs(value, "sources");
    if list.is_empty() {
        String::new()
    } else {
        format!("\n\n[i]Sources : {}[/i]", list.join(" ; "))
    }
}

fn trait_category(kind: &str) -> String {
    match kind {
        "acquired" => "acquis",
        "innate" => "inné",
        "personality" => "personnalité",
        "education" => "éducation",
        "lifestyle" => "mode de vie",
        other => return other.replace('_', " "),
    }
    .to_owned()
}

fn branch_label(branch: &str) -> String {
    match branch {
        "governance" => "Gouvernement",
        "war" | "military" => "Guerre",
        "diplomacy" => "Diplomatie",
        "learning" => "Savoir",
        "intrigue" => "Intrigue",
        "faith" | "religion" => "Foi",
        "commerce" | "economy" => "Commerce",
        other => return other.replace('_', " "),
    }
    .to_owned()
}

fn trigger_label(trigger: &str) -> &str {
    match trigger {
        "battle_won" => "après une victoire",
        "battle_fought" => "après une bataille",
        "siege_won" => "après un siège victorieux",
        "raid_led" => "après une chevauchée",
        "season_in_settlement" => "saison passée dans une ville amie dotée de : {}",
        "ransom_received" => "quand le souverain touche une rançon",
        other => other,
    }
}

fn religion_kind(kind: &str) -> &str {
    match kind {
        "church" => "Église",
        "heresy" => "hérésie",
        "obedience" => "obédience",
        "other_faith" => "autre foi",
        other => other,
    }
}

fn government_label(kind: &str) -> String {
    match kind {
        "kingdom" => "royaume",
        "duchy" => "duché",
        "county" => "comté",
        "republic" => "république",
        "empire" => "empire",
        "theocracy" => "théocratie",
        "confederation" => "confédération",
        "lordship" => "seigneurie",
        other => return other.to_owned(),
    }
    .to_owned()
}

fn make(id: &str, name: String, subtitle: String, body: String, asset_root: &Path) -> RulesEntry {
    RulesEntry::new(id, name, subtitle, body, asset_root)
}

fn collect(
    dir: &Path,
    id: &'static str,
    label: &'static str,
    mut build: impl FnMut(&Value) -> Option<RulesEntry>,
) -> RulesCategory {
    let mut entries: Vec<RulesEntry> = read_json_dir(dir).iter().filter_map(&mut build).collect();
    entries.sort_by_key(|entry| super::text::fold(&entry.name));
    RulesCategory { id, label, entries }
}

/// The six categories, in tab order.
pub fn extra_categories(
    data_dir: &Path,
    asset_root: &Path,
    store: &CodexStore,
) -> Vec<RulesCategory> {
    let mut names = BTreeMap::new();
    for dir in ["traits", "skills", "religions", "buildings", "factions"] {
        for value in read_json_dir(&data_dir.join(dir)) {
            names.insert(str_of(&value, "id").to_owned(), display_name(&value));
        }
    }
    let skill_defs = read_json_dir(&data_dir.join("skills"));
    let names = Names { names, store };

    let traits = collect(&data_dir.join("traits"), "traits", "Traits", |value| {
        let id = str_of(value, "id");
        let mut body = paragraph(str_of(value, "description"));
        let effects: Vec<String> = value
            .get("effects")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(effect_line)
            .collect();
        body += &section("Effets", &effects);
        let opposites = strs(value, "opposites");
        if !opposites.is_empty() {
            body += &format!("\n\n[b]Incompatible avec[/b]\n{}", names.links(&opposites));
        }
        Some(make(
            id,
            display_name(value),
            format!("Trait {}", trait_category(str_of(value, "category"))),
            body,
            asset_root,
        ))
    });

    let skills = collect(&data_dir.join("skills"), "skills", "Compétences", |value| {
        let id = str_of(value, "id");
        let cost = value.get("cost").and_then(Value::as_i64).unwrap_or(1);
        let mut body = paragraph(str_of(value, "description"));
        let effects: Vec<String> = value
            .get("effects")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(effect_line)
            .collect();
        body += &section("Effets", &effects);
        let prerequisites = strs(value, "prerequisites");
        body += &format!(
            "\n\n[b]Prérequis[/b]\n{}",
            if prerequisites.is_empty() {
                "Aucun".to_owned()
            } else {
                names.links(&prerequisites)
            }
        );
        let mut opens: Vec<&str> = skill_defs
            .iter()
            .filter(|other| strs(other, "prerequisites").contains(&id))
            .map(|other| str_of(other, "id"))
            .collect();
        opens.sort_unstable();
        if !opens.is_empty() {
            body += &format!("\n\n[b]Ouvre[/b]\n{}", names.links(&opens));
        }
        Some(make(
            id,
            display_name(value),
            format!(
                "{}, rang {} — {} point{} de compétence",
                branch_label(str_of(value, "branch")),
                value.get("tier").and_then(Value::as_i64).unwrap_or(1),
                cost,
                if cost > 1 { "s" } else { "" }
            ),
            body,
            asset_root,
        ))
    });

    let retinue = {
        let mut category = RulesCategory {
            id: "retinue",
            label: "Suite",
            entries: Vec::new(),
        };
        if let Ok(bytes) = std::fs::read(data_dir.join("retinue.json"))
            && let Ok(root) = serde_json::from_slice::<Value>(&bytes)
        {
            for value in root
                .get("companions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                category
                    .entries
                    .push(retinue_entry(value, &names, asset_root));
            }
        }
        category
            .entries
            .sort_by_key(|entry| super::text::fold(&entry.name));
        category
    };

    let factions = collect(
        &data_dir.join("factions"),
        "factions",
        "Factions",
        |value| {
            let id = str_of(value, "id");
            let mut lines = vec![format!(
                "[b]Régime[/b] {}",
                government_label(str_of(value, "government"))
            )];
            for (label, key) in [
                ("Religion", "religion"),
                ("Suzerain", "suzerain"),
                ("Souverain", "ruler"),
            ] {
                let target = str_of(value, key);
                if !target.is_empty() {
                    lines.push(format!("[b]{label}[/b] {}", names.link(target)));
                }
            }
            let capital = str_of(value, "capital_city");
            if !capital.is_empty() {
                lines.push(format!("[b]Capitale[/b] {capital}"));
            }
            let law = str_of(value, "succession_law");
            if !law.is_empty() {
                lines.push(format!(
                    "[b]Succession[/b] {}",
                    super::rules::succession_label(law)
                ));
            }
            if let Some(treasury) = value.get("treasury").and_then(Value::as_i64) {
                lines.push(format!("[b]Trésor en 1337[/b] {}", fr::amount(treasury)));
            }
            let blazon = value
                .get("heraldry")
                .map(|heraldry| str_of(heraldry, "blazon"))
                .unwrap_or("");
            if !blazon.is_empty() {
                lines.push(format!("[b]Armes[/b] {blazon}"));
            }
            let mut body = lines.join("\n");
            let description = str_of(value, "description");
            if !description.is_empty() {
                body += &format!("\n\n{description}");
            }
            body += &sources(value);
            let playable = value.get("playable").and_then(Value::as_bool) == Some(true);
            Some(make(
                id,
                display_name(value),
                format!(
                    "Faction{} — {}",
                    if playable { " jouable" } else { "" },
                    local_name(value).unwrap_or_default()
                )
                .trim_end_matches(" — ")
                .to_owned(),
                body,
                asset_root,
            ))
        },
    );

    let religions = collect(
        &data_dir.join("religions"),
        "religions",
        "Religion",
        |value| {
            let id = str_of(value, "id");
            let mut body = paragraph(str_of(value, "description"));
            let seat = str_of(value, "seat");
            if !seat.is_empty() {
                body += &format!("\n\n[b]Siège[/b] {seat}");
            }
            body += &sources(value);
            Some(make(
                id,
                display_name(value),
                format!("Religion — {}", religion_kind(str_of(value, "kind"))),
                body,
                asset_root,
            ))
        },
    );

    let agents = agents_category(data_dir, asset_root);
    vec![traits, skills, retinue, factions, religions, agents]
}

fn retinue_entry(value: &Value, names: &Names<'_>, asset_root: &Path) -> RulesEntry {
    let id = str_of(value, "id");
    let category = match str_of(value, "category") {
        "military" => "Guerre",
        "court" => "Cour",
        "faith" => "Foi",
        "learning" => "Savoir",
        "commerce" => "Commerce",
        "intrigue" => "Intrigue",
        other => other,
    };
    let mut subtitle = format!("Compagnon de suite — {category}");
    if let Some(local) = local_name(value) {
        subtitle += &format!(" — « {local} »");
    }
    let mut body = paragraph(str_of(value, "description"));
    let effects: Vec<String> = value
        .get("effects")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(effect_line)
        .collect();
    body += &section("Effets", &effects);
    let ways: Vec<String> = value
        .get("acquisition")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|rule| {
            let label = trigger_label(str_of(rule, "trigger"))
                .replace("{}", &names.link(str_of(rule, "building")));
            format!(
                "• {label} ({} ‰ par occasion)",
                rule.get("chance_permille")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
            )
        })
        .collect();
    body += &section("Obtention", &ways);
    let mut limits = Vec::new();
    if let Some(conditions) = value.get("conditions") {
        let factions = strs(conditions, "factions");
        if !factions.is_empty() {
            limits.push(format!("Réservé à : {}", names.links(&factions)));
        }
        let battles = conditions
            .get("min_battles")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if battles > 0 {
            limits.push(format!("Au moins {battles} batailles livrées"));
        }
        let command = conditions
            .get("min_command")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if command > 0 {
            limits.push(format!("Commandement {command} au moins"));
        }
        let excluded = strs(conditions, "excludes_traits");
        if !excluded.is_empty() {
            limits.push(format!("Jamais avec : {}", names.links(&excluded)));
        }
    }
    body += &section("Conditions", &limits);
    body += if value.get("inheritable").and_then(Value::as_bool) == Some(true) {
        "\n\n[b]Transmission[/b]\nPasse à l’héritier (fils aîné majeur, sinon le nouveau souverain) à la mort de son maître."
    } else {
        "\n\n[b]Transmission[/b]\nQuitte la maison à la mort de son maître."
    };
    make(id, str_of_name(value), subtitle, body, asset_root)
}

fn str_of_name(value: &Value) -> String {
    display_name(value)
}

fn agents_category(data_dir: &Path, asset_root: &Path) -> RulesCategory {
    let mut category = RulesCategory {
        id: "agents",
        label: "Agents",
        entries: Vec::new(),
    };
    let Ok(bytes) = std::fs::read(data_dir.join("rules/agents.json")) else {
        return category;
    };
    let Ok(root) = serde_json::from_slice::<Value>(&bytes) else {
        return category;
    };
    let actions: Vec<(&str, &Value)> = root
        .get("actions")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(key, value)| (key.as_str(), value))
                .collect()
        })
        .unwrap_or_default();
    for (kind, agent) in root
        .get("types")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let mut lines = Vec::new();
        if let Some(cost) = agent.get("cost").and_then(Value::as_i64) {
            lines.push(format!(
                "[b]Coût[/b] {}, entretien {} par saison",
                fr::amount(cost),
                fr::amount(agent.get("upkeep").and_then(Value::as_i64).unwrap_or(0))
            ));
        }
        if let Some(max) = agent.get("max_per_faction").and_then(Value::as_i64) {
            lines.push(format!("[b]Maximum par faction[/b] {max}"));
        }
        if let Some(steps) = agent.get("movement_steps").and_then(Value::as_i64) {
            lines.push(format!("[b]Déplacement[/b] {steps} pas par saison"));
        }
        let mut body = paragraph(str_of(agent, "description"));
        if !lines.is_empty() {
            body += &format!("\n\n{}", lines.join("\n"));
        }
        category.entries.push(make(
            &format!("agent_{kind}"),
            str_of(agent, "name").to_owned(),
            "Agent de campagne".to_owned(),
            body,
            asset_root,
        ));
    }
    let lines: Vec<String> = actions
        .iter()
        .map(|(_, action)| {
            let risk = action
                .get("death_risk")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            format!(
                "• [b]{}[/b] — {} % (+{} par sceau){} : {}",
                str_of(action, "name"),
                action
                    .get("base_chance")
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                action.get("per_level").and_then(Value::as_i64).unwrap_or(0),
                if risk > 0 {
                    format!(", mort {risk} % en cas d’échec")
                } else {
                    String::new()
                },
                str_of(action, "description")
            )
        })
        .collect();
    category.entries.push(make(
        "agent_actions",
        "Actions des agents".to_owned(),
        "Agents de campagne".to_owned(),
        format!(
            "Une action par saison ; chance de réussite de base, bonus par niveau de sceau.{}",
            section("Actions", &lines)
        ),
        asset_root,
    ));
    category
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_data_fills_the_six_tabs() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let store = CodexStore::default();
        let categories = extra_categories(
            &crate::paths::data_dir(&root),
            &crate::paths::asset_root(&root),
            &store,
        );
        let ids: Vec<&str> = categories.iter().map(|c| c.id).collect();
        assert_eq!(ids, TABS.map(|(id, _)| id));
        for category in &categories {
            assert!(!category.entries.is_empty(), "{} is empty", category.id);
        }
        assert_eq!(
            effect_line(&serde_json::json!({"effect":"army_morale","value":2,"mode":"add"})),
            "• army morale : +2"
        );
    }
}
