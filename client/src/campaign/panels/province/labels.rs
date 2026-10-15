//! French labels of the province panel, read from `data/ui/tooltips.json` (the same table the
//! Godot `RichTooltip` used): effect kinds, social classes, building categories.

use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::Value;

use crate::paths;

#[derive(Resource, Default, Debug, Clone)]
pub struct UiLabels {
    pub effects: HashMap<String, String>,
    pub classes: HashMap<String, String>,
    pub categories: HashMap<String, String>,
}

fn block(root: &Value, name: &str, field: &str) -> HashMap<String, String> {
    root.get(name)
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(key, entry)| {
                    let text = entry.get(field).or_else(|| entry.get("title"))?.as_str()?;
                    Some((key.clone(), text.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

impl UiLabels {
    pub fn parse(json: &str) -> Self {
        let root: Value = serde_json::from_str(json).unwrap_or(Value::Null);
        Self {
            effects: block(&root, "effects", "label"),
            classes: block(&root, "classes", "title"),
            categories: block(&root, "categories", "title"),
        }
    }

    pub fn load() -> Self {
        paths::find_root()
            .ok()
            .and_then(|root| {
                std::fs::read_to_string(paths::data_dir(&root).join("ui/tooltips.json")).ok()
            })
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    pub fn effect(&self, key: &str) -> String {
        self.effects
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.replace('_', " "))
    }

    pub fn class(&self, key: &str) -> String {
        self.classes
            .get(key)
            .cloned()
            .unwrap_or_else(|| class_fallback(key).to_owned())
    }

    pub fn category(&self, key: &str) -> String {
        let label = self
            .categories
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.replace('_', " "));
        capitalize(&label)
    }
}

pub fn class_fallback(key: &str) -> &str {
    match key {
        "peasants" => "Paysans",
        "burghers" => "Bourgeois",
        "clergy" => "Clergé",
        "nobility" => "Noblesse",
        other => other,
    }
}

pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

/// Terms of the unrest target (`unrest_terms` keys of the core).
pub fn unrest_term(key: &str) -> &str {
    match key {
        "tax" => "Impôt",
        "devastation" => "Dévastation",
        "goods" => "Biens de consommation",
        "health" => "Santé",
        "garrison" => "Garnison",
        "occupation" => "Occupation",
        "foreign_religion" => "Religion étrangère",
        "disorder" => "Désordre récent",
        "buildings" => "Bâtiments, gouverneur, techniques",
        "loyalty" => "Loyauté de la noblesse",
        other => other,
    }
}

pub fn settlement_kind(key: &str) -> &str {
    match key {
        "city" => "Cité",
        "town" => "Ville",
        "castle" => "Château",
        "abbey" => "Abbaye",
        "village" => "Village",
        other => other,
    }
}

pub fn tax_rate(key: &str) -> &str {
    match key {
        "low" => "Faible",
        "normal" => "Normal",
        "high" => "Lourd",
        other => other,
    }
}

pub fn node_state(key: &str) -> &str {
    match key {
        "built" => "Bâti",
        "building" => "En chantier",
        "queued" => "En file",
        "available" => "Disponible",
        "locked" => "Verrouillé",
        other => other,
    }
}

/// Signed number as the Godot tooltips show it (`+5`, `−3`, `+12 %`).
pub fn signed_value(value: f64, percent: bool) -> String {
    let body = if (value - value.round()).abs() < 1e-9 {
        format!("{}", value.abs().round() as i64)
    } else {
        format!("{:.2}", value.abs()).replace('.', ",")
    };
    let sign = if value >= 0.0 {
        '+'
    } else {
        crate::ui::kit::fr::MINUS
    };
    format!("{sign}{body}{}", if percent { " %" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_labels() {
        let labels = UiLabels::parse(
            r#"{"effects":{"unrest":{"label":"Mécontentement"}},"classes":{"clergy":{"title":"Clergé"}}}"#,
        );
        assert_eq!(labels.effect("unrest"), "Mécontentement");
        assert_eq!(labels.effect("tax_income"), "tax income");
        assert_eq!(labels.class("clergy"), "Clergé");
        assert_eq!(signed_value(5.0, true), "+5 %");
        assert_eq!(signed_value(-2.5, false), "\u{2212}2,50");
    }
}
