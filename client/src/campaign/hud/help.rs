//! Help: the shortcut sheet (ports game/scripts/ui/shortcut_sheet.gd, opened by F1) and the map
//! legend (ports game/scripts/map/map_legend.gd, opened by the « Légende » button of the minimap).
//! Both are modal notes of the kit; the legend follows the current map mode.
//!
//! Keys come from the Godot `InputMap` defaults, labelled for the keyboard layout of the
//! settings (`input/layout`: AZERTY swaps A/Q, Z/W, M/;); a custom binding saved in
//! `Settings::bindings` replaces the default of its action. Keys of features not ported yet are
//! listed all the same (the keymap lot of the client owns the real bindings).

use bevy::prelude::*;
use serde_json::{Value, json};

use super::HudSet;
use crate::agent::dump::DumpAppExt;
use crate::campaign::provinces::modes::{MapMode, MapModeState};
use crate::menu::KeyEdges;
use crate::settings::Settings;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::modal::{ModalQueue, ModalSpec, PRIORITY_REPORT};

const SHEET_KEY: &str = "hud:shortcuts";
const LEGEND_KEY: &str = "hud:legend_modal";

/// `(action, label, default keys in QWERTY positions)`.
type Line = (&'static str, &'static str, &'static str);

/// Lines of the sheet per section.
pub const SECTIONS: &[(&str, &[Line])] = &[
    (
        "Caméra",
        &[
            ("map_pan_up", "Avancer", "W / ↑"),
            ("map_pan_left", "Aller à gauche", "A / ←"),
            ("map_pan_down", "Reculer", "S / ↓"),
            ("map_pan_right", "Aller à droite", "D / →"),
            ("map_rotate_left", "Tourner à gauche", "Q"),
            ("map_rotate_right", "Tourner à droite", "E"),
            (
                "map_toggle_edge_pan",
                "Défilement par les bords (oui / non)",
                "ScrollLock",
            ),
        ],
    ),
    (
        "Fenêtres",
        &[
            ("map_toggle_court", "Cour et personnages", "C"),
            ("map_toggle_tech", "Technologies", "F4"),
            ("map_toggle_diplomacy", "Diplomatie et religion", "F3"),
            ("map_toggle_objectives", "Objectifs", "O"),
            ("map_toggle_agents", "Agents", "G"),
            ("map_toggle_units", "Mes unités (armées et agents)", "U"),
            (
                "map_toggle_holdings",
                "Colonies (revenus, chantiers, menaces)",
                "B",
            ),
            (
                "map_toggle_council",
                "Conseil du royaume (charges et conseils)",
                "J",
            ),
            ("codex_open", "Codex (histoire et règles)", "K"),
            ("help_open", "Aide et raccourcis", "F1"),
        ],
    ),
    (
        "Filtres de carte",
        &[
            (
                "map_filters_menu",
                "Menu des filtres (richesse, population, loyauté…)",
                "F",
            ),
            ("map_toggle_unrest", "Mécontentement", "M"),
            ("map_mode_diplomacy", "Carte diplomatique", "N"),
            ("map_mode_religion", "Carte religieuse", "R"),
            ("map_toggle_trade", "Routes commerciales", "V"),
        ],
    ),
    (
        "Armées et colonies",
        &[
            ("campaign_next_idle", "Armée inactive suivante", "Tab"),
            ("campaign_prev_idle", "Armée inactive précédente", "Maj+Tab"),
            ("campaign_next_settlement", "Colonie suivante", "Ctrl+Tab"),
            ("army_center", "Centrer sur l’armée sélectionnée", "."),
            ("army_follow", "Suivre l’armée sélectionnée", "Y"),
            ("army_split", "Séparer les régiments choisis", "X"),
            (
                "army_garrison",
                "Laisser les régiments choisis en garnison",
                "H",
            ),
            ("army_stance_normal", "Posture : normale", "1"),
            ("army_stance_raid", "Posture : chevauchée", "2"),
            ("army_stance_siege", "Posture : siège", "3"),
            ("army_stance_ambush", "Posture : embuscade", "4"),
            ("army_stance_forced_march", "Posture : marche forcée", "5"),
            ("army_stance_entrenched", "Posture : camp retranché", "6"),
        ],
    ),
    (
        "Partie",
        &[
            ("campaign_end_turn", "Finir la saison", "Entrée"),
            (
                "campaign_end_turn_fast",
                "Finir la saison sans confirmation",
                "Maj+Entrée",
            ),
            (
                "campaign_pause",
                "Fermer la fenêtre du dessus, puis menu pause",
                "Échap",
            ),
            ("quick_save", "Sauvegarde rapide", "F6"),
            ("settings", "Réglages", "F10"),
            (
                "tooltip_explore",
                "Maintenir : explorer les infobulles en chaîne",
                "Alt",
            ),
        ],
    ),
];

/// Mouse commands (outside the key map).
const MOUSE_LINES: [(&str, &str); 4] = [
    (
        "Clic gauche",
        "Sélectionner une armée, une province ou une colonie",
    ),
    ("Clic droit", "Armée sélectionnée : marcher vers ce lieu"),
    ("Molette", "Zoom"),
    ("Espace", "Passer le rejeu des mouvements de l’IA"),
];

/// Label of a QWERTY-position key on an AZERTY keyboard.
fn azerty(key: &str) -> String {
    match key {
        "Q" => "A",
        "A" => "Q",
        "W" => "Z",
        "Z" => "W",
        "M" => ",",
        other => other,
    }
    .to_owned()
}

/// Keys of a default « W / ↑ » for the layout (`azerty` or anything else = qwerty).
pub fn keys_for(default: &str, layout: &str) -> String {
    if layout != "azerty" {
        return default.to_owned();
    }
    default
        .split(" / ")
        .map(|chord| {
            let mut parts: Vec<String> = chord.split('+').map(str::to_owned).collect();
            if let Some(last) = parts.last_mut()
                && last.chars().count() == 1
            {
                *last = azerty(last);
            }
            parts.join("+")
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

/// Effective layout: the setting, `auto` meaning the French default of the game.
fn layout_of(settings: Option<&Settings>) -> String {
    match settings.map(|s| s.keyboard_layout.as_str()) {
        Some("qwerty") => "qwerty".to_owned(),
        _ => "azerty".to_owned(),
    }
}

/// The sheet in BBCode.
pub fn sheet_bbcode(settings: Option<&Settings>) -> String {
    sheet_bbcode_with(settings, None)
}

/// The sheet with the live key map: an action known to `settings::ACTIONS` shows its current
/// chords (rebinds included); the others keep the Godot default.
pub fn sheet_bbcode_with(
    settings: Option<&Settings>,
    keymap: Option<&crate::settings::KeyMap>,
) -> String {
    let layout = layout_of(settings);
    let mut parts: Vec<String> = Vec::new();
    for (title, lines) in SECTIONS {
        parts.push(format!("[b]{title}[/b]"));
        for (action, label, default) in *lines {
            let custom = settings
                .and_then(|s| s.bindings.get(*action))
                .filter(|keys| !keys.is_empty())
                .map(|keys| keys.join(" / "));
            let live = keymap
                .filter(|_| crate::settings::ACTIONS.iter().any(|a| a.id == *action))
                .map(|map| map.label(action, &layout))
                .filter(|label| !label.is_empty());
            let keys = live
                .or(custom)
                .unwrap_or_else(|| keys_for(default, &layout));
            parts.push(format!("• [b]{keys}[/b] : {label}"));
        }
        parts.push(String::new());
    }
    parts.push("[b]Souris[/b]".to_owned());
    for (keys, label) in MOUSE_LINES {
        parts.push(format!("• [b]{keys}[/b] : {label}"));
    }
    parts.join("\n")
}

/// Legend BBCode for a map mode key (`data/ui/map_legend.json`).
pub fn legend_bbcode(tree: &Value, mode: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(intro) = tree.get("intro").and_then(Value::as_str) {
        parts.push(format!("[i]{intro}[/i]"));
    }
    let mode_title = tree
        .get("mode_titles")
        .and_then(|t| t.get(mode))
        .and_then(Value::as_str);
    if let Some(title) = mode_title {
        parts.push(format!("[b]{title}[/b]"));
    }
    let sections = tree.get("sections").and_then(Value::as_array);
    for section in sections.into_iter().flatten() {
        let modes = section.get("modes").and_then(Value::as_array);
        if modes.is_some_and(|m| !m.is_empty() && !m.iter().any(|x| x.as_str() == Some(mode))) {
            continue;
        }
        parts.push(String::new());
        parts.push(format!(
            "[b]{}[/b]",
            section.get("title").and_then(Value::as_str).unwrap_or("")
        ));
        for entry in section
            .get("entries")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let sample = entry.get("sample");
            let swatch = |key: &str| {
                sample
                    .and_then(|s| s.get(key))
                    .and_then(Value::as_str)
                    .map(|hex| format!("[color={hex}]■[/color]"))
            };
            let mark = match (swatch("color"), swatch("from"), swatch("to")) {
                (Some(c), _, _) => format!("{c} "),
                (None, Some(a), Some(b)) => format!("{a}{b} "),
                _ => "• ".to_owned(),
            };
            parts.push(format!(
                "{mark}[b]{}[/b] : {}",
                entry.get("label").and_then(Value::as_str).unwrap_or(""),
                entry.get("text").and_then(Value::as_str).unwrap_or("")
            ));
        }
    }
    parts.join("\n")
}

#[derive(Resource, Default)]
pub struct Help {
    legend: Option<Value>,
    pub sheet_opened: u32,
    pub legend_opened: u32,
    pub last_legend_mode: String,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Help>()
        .register_dump(dump_help)
        .add_systems(Update, open_help.in_set(HudSet::Ui));
}

#[allow(clippy::too_many_arguments)]
fn open_help(
    mut pressed: MessageReader<ButtonPressed>,
    edges: Option<Res<KeyEdges>>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Option<Res<Settings>>,
    keymap: Option<Res<crate::settings::KeyMap>>,
    modes: Option<Res<MapModeState>>,
    mut help: ResMut<Help>,
    mut queue: ResMut<ModalQueue>,
) {
    let _ = edges;
    let sheet = keymap.as_deref().map_or_else(
        || keys.just_pressed(KeyCode::F1),
        |map| map.just_pressed("help_open", &keys),
    );
    let legend = pressed.read().any(|p| p.name == "hud:legend");
    if sheet && !queue.is_busy() {
        let mut spec = ModalSpec::info(
            "Aide et raccourcis",
            &sheet_bbcode_with(settings.as_deref(), keymap.as_deref()),
        );
        spec.width = 560.0;
        queue.request(SHEET_KEY, PRIORITY_REPORT, spec);
        help.sheet_opened += 1;
    }
    if legend && !queue.is_busy() {
        if help.legend.is_none() {
            help.legend = crate::paths::find_root()
                .ok()
                .and_then(|root| std::fs::read_to_string(root.join("data/ui/map_legend.json")).ok())
                .and_then(|text| serde_json::from_str(&text).ok())
                .or(Some(Value::Null));
        }
        let mode = modes.map_or(MapMode::Political, |m| m.mode);
        let tree = help.legend.clone().unwrap_or(Value::Null);
        let title = tree
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Légende de la carte")
            .to_owned();
        let mut spec = ModalSpec::info(&title, &legend_bbcode(&tree, mode.key()));
        spec.width = 520.0;
        queue.request(LEGEND_KEY, PRIORITY_REPORT, spec);
        help.legend_opened += 1;
        help.last_legend_mode = mode.key().to_owned();
    }
}

fn dump_help(world: &World) -> (String, Value) {
    let help = world.resource::<Help>();
    (
        "help".to_owned(),
        json!({
            "sheet_opened": help.sheet_opened, "legend_opened": help.legend_opened,
            "legend_mode": help.last_legend_mode,
            "sheet_lines": SECTIONS.iter().map(|(_, l)| l.len()).sum::<usize>(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn azerty_swaps_letters_only() {
        assert_eq!(keys_for("W / ↑", "azerty"), "Z / ↑");
        assert_eq!(keys_for("A / ←", "azerty"), "Q / ←");
        assert_eq!(keys_for("Maj+Tab", "azerty"), "Maj+Tab");
        assert_eq!(keys_for("Q", "azerty"), "A");
        assert_eq!(keys_for("W / ↑", "qwerty"), "W / ↑");
        assert_eq!(keys_for("F4", "azerty"), "F4");
    }

    #[test]
    fn sheet_lists_every_section_and_the_mouse() {
        let text = sheet_bbcode(None);
        for (title, _) in SECTIONS {
            assert!(text.contains(title));
        }
        assert!(text.contains("[b]Souris[/b]"));
        assert!(text.contains("[b]Z / ↑[/b] : Avancer"));
    }

    #[test]
    fn custom_binding_wins() {
        let mut settings = Settings::default();
        settings
            .bindings
            .insert("map_rotate_left".into(), vec!["J".into()]);
        assert!(sheet_bbcode(Some(&settings)).contains("[b]J[/b] : Tourner à gauche"));
    }

    #[test]
    fn legend_filters_sections_by_mode() {
        let tree = json!({"intro": "i", "mode_titles": {"unrest": "Carte M"}, "sections": [
            {"id": "a", "title": "Politique", "modes": ["political"], "entries": [
                {"sample": {"type": "faction_colors"}, "label": "L1", "text": "t1"}]},
            {"id": "b", "title": "Troubles", "modes": ["unrest"], "entries": [
                {"sample": {"type": "gradient", "from": "#00ff00", "to": "#ff0000"}, "label": "L2", "text": "t2"}]},
            {"id": "c", "title": "Toujours", "entries": [
                {"sample": {"type": "color", "color": "#112233"}, "label": "L3", "text": "t3"}]}]});
        let text = legend_bbcode(&tree, "unrest");
        assert!(text.contains("Carte M") && text.contains("Troubles") && text.contains("Toujours"));
        assert!(!text.contains("Politique"));
        assert!(text.contains("[color=#00ff00]■[/color][color=#ff0000]■[/color]"));
    }
}
