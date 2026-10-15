//! Icon helper: `icon(theme, assets, "name", size)` (port of `IconLibrary.make_rect`).
//!
//! Godot's `IconLibrary` (game/scripts/ui/icon_library.gd) resolves an id through, in priority
//! order: painted entity miniatures (`assets/icons/entity/index.json`, units, buildings, techs…),
//! the ink action icons (`assets/icons/ink/index.json`, HUD and actions, PNG 128 px in INK colour,
//! tintable) and the game-icons.net SVGs (`assets/icons/icons.json`). Medallions
//! (`assets/ui/medallions/index.json`) are a fourth, separate family.
//!
//! Bevy cannot load SVG: SVG-only ids resolve to nothing here ([`IconLibrary::svg_only`] lists them
//! so the gap is visible in the dump); `icon` then returns a transparent box of the right size, so
//! layouts do not move. Each missing id needs a PNG in the entity or ink family (asset pipeline,
//! not a kit job).
//!
//! The library is built at `Startup` from the index files and stored in [`UiTheme::icons`].

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde_json::{Value, json};

use super::UiTheme;
use super::style;
use super::widgets::KitWidget;

/// Icon family, in resolution priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IconFamily {
    /// Painted, framed 128 px miniature (never tinted).
    Entity,
    /// Ink icon (RGB = `INK`, alpha = ink): tintable through [`ink_tint`].
    Ink,
    /// Illuminated medallion (`ui/medallions`).
    Medallion,
}

impl IconFamily {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Ink => "ink",
            Self::Medallion => "medallion",
        }
    }
}

/// Resolved icon: family and path relative to `game/assets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconEntry {
    pub family: IconFamily,
    pub path: String,
}

/// Name → PNG table of the three PNG families (entity > ink > medallion), plus the SVG-only ids.
#[derive(Debug, Clone, Default)]
pub struct IconLibrary {
    entries: BTreeMap<String, IconEntry>,
    /// Ids that only exist as SVG in `icons.json` (not loadable in Bevy).
    svg_only: BTreeSet<String>,
}

/// Category prefixes of ids (`IconLibrary.PREFIX_CATEGORIES`), for the generic fallbacks.
pub const PREFIX_CATEGORIES: [(&str, &str); 11] = [
    ("unit_", "unit"),
    ("bld_", "building"),
    ("building_", "building"),
    ("res_", "resource"),
    ("tech_", "technology"),
    ("trait_", "trait"),
    ("skill_", "skill"),
    ("class_", "class"),
    ("gauge_", "gauge"),
    ("hud_", "hud"),
    ("branch_", "branch"),
];

/// Category of an id by prefix (`default` when none matches).
pub fn category_of(id: &str) -> &'static str {
    PREFIX_CATEGORIES
        .iter()
        .find(|(prefix, _)| id.starts_with(prefix))
        .map_or("default", |(_, category)| category)
}

/// Modulation that turns the baked ink (`INK`) into `color` (`IconLibrary.tint`), for
/// `ImageNode::color`: components above 1 are clamped by Bevy, so use colours at most as bright
/// as the ink's channel ratio allows (gold works on the red/green channels only partly; prefer
/// dark or mid tones).
pub fn ink_tint(color: Color) -> Color {
    let ink = style::INK.to_srgba();
    let target = color.to_srgba();
    Color::srgba(
        (target.red / ink.red).min(1.0),
        (target.green / ink.green).min(1.0),
        (target.blue / ink.blue).min(1.0),
        target.alpha,
    )
}

impl IconLibrary {
    /// Builds the table from the three index files (`{"icons": {id: file}}`) and `icons.json`.
    /// Only PNG files listed by an index count; priority entity > ink > medallion.
    pub fn from_assets(read: impl Fn(&str) -> Option<Vec<u8>>) -> Self {
        let mut library = Self::default();
        let families = [
            (IconFamily::Medallion, "ui/medallions"),
            (IconFamily::Ink, "icons/ink"),
            (IconFamily::Entity, "icons/entity"),
        ];
        // Lowest priority first so higher families overwrite.
        for (family, dir) in families {
            let Some(bytes) = read(&format!("{dir}/index.json")) else {
                continue;
            };
            let Ok(Value::Object(root)) = serde_json::from_slice::<Value>(&bytes) else {
                continue;
            };
            let Some(Value::Object(table)) = root.get("icons") else {
                continue;
            };
            for (id, file) in table {
                if let Some(file) = file.as_str() {
                    library.entries.insert(
                        id.clone(),
                        IconEntry {
                            family,
                            path: format!("{dir}/{file}"),
                        },
                    );
                }
            }
        }
        if let Some(bytes) = read("icons/icons.json")
            && let Ok(Value::Object(root)) = serde_json::from_slice::<Value>(&bytes)
            && let Some(Value::Object(table)) = root.get("icons")
        {
            for id in table.keys() {
                if !library.entries.contains_key(id) {
                    library.svg_only.insert(id.clone());
                }
            }
        }
        library
    }

    pub fn has(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    pub fn get(&self, id: &str) -> Option<&IconEntry> {
        self.entries.get(id)
    }

    /// Ids of the SVG-only icons (no PNG equivalent).
    pub fn svg_only(&self) -> &BTreeSet<String> {
        &self.svg_only
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Names of one family, sorted.
    pub fn names(&self, family: IconFamily) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.family == family)
            .map(|(id, _)| id.as_str())
            .collect()
    }
}

/// Marker on every icon box: the requested id and what it resolved to.
#[derive(Component, Debug, Clone)]
pub struct Icon {
    pub id: String,
    pub family: Option<IconFamily>,
}

/// Square icon of `size` px (`IconLibrary.make_rect`): an `ImageNode` of the resolved PNG, or an
/// empty transparent box of the same size when the id has no PNG equivalent.
pub fn icon(theme: &UiTheme, assets: &AssetServer, name: &str, size: f32) -> impl Bundle {
    build(theme, assets, name, size, Color::WHITE)
}

/// [`icon`] of an ink icon tinted to `color` (ignored for painted miniatures and medallions,
/// which keep their colours).
pub fn icon_tinted(
    theme: &UiTheme,
    assets: &AssetServer,
    name: &str,
    size: f32,
    color: Color,
) -> impl Bundle {
    let tint = match theme.icons.get(name) {
        Some(entry) if entry.family == IconFamily::Ink => ink_tint(color),
        _ => Color::WHITE,
    };
    build(theme, assets, name, size, tint)
}

fn build(theme: &UiTheme, assets: &AssetServer, name: &str, size: f32, tint: Color) -> impl Bundle {
    let entry = theme.icons.get(name);
    let image = match entry {
        Some(entry) => ImageNode::new(assets.load(entry.path.clone())).with_color(tint),
        None => ImageNode::default().with_color(Color::NONE),
    };
    (
        Name::new(format!("Icon_{name}")),
        KitWidget("icon"),
        Icon {
            id: name.to_owned(),
            family: entry.map(|entry| entry.family),
        },
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            ..default()
        },
        image,
    )
}

/// Dump of the library: counts per family, every name, and the SVG-only ids.
pub fn dump(world: &World) -> Value {
    let theme = world.resource::<UiTheme>();
    let library = &theme.icons;
    let mut used = 0;
    let mut unresolved = Vec::new();
    for entity in world.iter_entities() {
        if let Some(icon) = entity.get::<Icon>() {
            used += 1;
            if icon.family.is_none() {
                unresolved.push(icon.id.clone());
            }
        }
    }
    json!({
        "entity": library.names(IconFamily::Entity).len(),
        "ink": library.names(IconFamily::Ink).len(),
        "medallion": library.names(IconFamily::Medallion).len(),
        "svg_only": library.svg_only.len(),
        "names": {
            "entity": library.names(IconFamily::Entity),
            "ink": library.names(IconFamily::Ink),
            "medallion": library.names(IconFamily::Medallion),
        },
        "svg_only_names": library.svg_only.iter().collect::<Vec<_>>(),
        "icons_on_screen": used,
        "unresolved_on_screen": unresolved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(path: &str) -> Option<Vec<u8>> {
        let text = match path {
            "icons/ink/index.json" => r#"{"icons": {"treasury": "treasury.png", "both": "b.png"}}"#,
            "icons/entity/index.json" => r#"{"icons": {"unit_knights": "k.png", "both": "e.png"}}"#,
            "ui/medallions/index.json" => r#"{"icons": {"agents": "agents.png"}}"#,
            "icons/icons.json" => r#"{"icons": {"treasury": {}, "cat_unit": {}, "res_wheat": {}}}"#,
            _ => return None,
        };
        Some(text.as_bytes().to_vec())
    }

    #[test]
    fn priority_entity_over_ink() {
        let library = IconLibrary::from_assets(fake);
        assert_eq!(library.get("both").unwrap().family, IconFamily::Entity);
        assert_eq!(library.get("both").unwrap().path, "icons/entity/e.png");
        assert_eq!(library.get("treasury").unwrap().family, IconFamily::Ink);
        assert_eq!(
            library.get("agents").unwrap().path,
            "ui/medallions/agents.png"
        );
        assert_eq!(library.len(), 4);
    }

    #[test]
    fn svg_only_listed() {
        let library = IconLibrary::from_assets(fake);
        let names: Vec<_> = library.svg_only().iter().cloned().collect();
        assert_eq!(names, ["cat_unit", "res_wheat"]);
    }

    #[test]
    fn categories_by_prefix() {
        assert_eq!(category_of("unit_knights"), "unit");
        assert_eq!(category_of("hud_treasury"), "hud");
        assert_eq!(category_of("zzz"), "default");
    }

    #[test]
    fn ink_tint_identity_on_ink() {
        let tint = ink_tint(style::INK).to_srgba();
        assert!((tint.red - 1.0).abs() < 1e-4 && (tint.green - 1.0).abs() < 1e-4);
    }
}
