//! The key map: named actions, their default keys and the player's re-bindings.
//!
//! Ports `game/scripts/ui/key_bindings.gd` (actions of `ShortcutSheet.CAMPAIGN_SECTIONS`, defaults
//! of `project.godot`, swap-on-conflict re-binding). The [`KeyMap`] resource is derived from
//! `Settings::bindings` (action -> chord labels, only the actions that differ from their defaults
//! are stored), so it is saved with the settings file and follows « Annuler » / « Valider ».
//!
//! **Other lots read it**: `keymap.just_pressed("map_rotate_left", &keys)`,
//! `keymap.pressed(..)`, `keymap.label("campaign_end_turn", "azerty")` for hints.
//!
//! Chord labels use the vocabulary of the shortcut sheet (`help.rs`): the key in its QWERTY
//! position (`W`, `Tab`, `Entrée`, `Échap`, `F4`, `↑`, `.`), optionally prefixed by `Maj+`,
//! `Ctrl+`, `Alt+`. Modifiers must match exactly (`Tab` does not fire on `Maj+Tab`), except the
//! modifier that is the key itself (`Alt` alone).

use std::collections::BTreeMap;

use bevy::prelude::*;

/// A key with its modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub key: KeyCode,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// `(key, label)`; the first entry of a key is its canonical label.
const KEY_NAMES: &[(KeyCode, &str)] = &[
    (KeyCode::ArrowUp, "↑"),
    (KeyCode::ArrowLeft, "←"),
    (KeyCode::ArrowDown, "↓"),
    (KeyCode::ArrowRight, "→"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::Enter, "Entrée"),
    (KeyCode::NumpadEnter, "Entrée (pavé)"),
    (KeyCode::Escape, "Échap"),
    (KeyCode::Space, "Espace"),
    (KeyCode::Backspace, "Retour"),
    (KeyCode::Delete, "Suppr"),
    (KeyCode::Home, "Début"),
    (KeyCode::End, "Fin"),
    (KeyCode::PageUp, "Pg. préc."),
    (KeyCode::PageDown, "Pg. suiv."),
    (KeyCode::Insert, "Inser"),
    (KeyCode::ScrollLock, "ScrollLock"),
    (KeyCode::Period, "."),
    (KeyCode::Comma, ","),
    (KeyCode::Semicolon, ";"),
    (KeyCode::Quote, "'"),
    (KeyCode::Minus, "-"),
    (KeyCode::Equal, "="),
    (KeyCode::Slash, "/"),
    (KeyCode::Backslash, "\\"),
    (KeyCode::BracketLeft, "["),
    (KeyCode::BracketRight, "]"),
    (KeyCode::Backquote, "`"),
    (KeyCode::NumpadAdd, "Pavé +"),
    (KeyCode::NumpadSubtract, "Pavé -"),
    (KeyCode::AltLeft, "Alt"),
    (KeyCode::AltRight, "Alt"),
];

const LETTERS: [KeyCode; 26] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
];
const DIGITS: [KeyCode; 10] = [
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];
const FUNCTION: [KeyCode; 12] = [
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
];

/// Label of a key without modifiers (`None` for a key that cannot be bound).
pub fn key_label(key: KeyCode) -> Option<String> {
    if let Some(index) = LETTERS.iter().position(|k| *k == key) {
        return Some(((b'A' + index as u8) as char).to_string());
    }
    if let Some(index) = DIGITS.iter().position(|k| *k == key) {
        return Some(index.to_string());
    }
    if let Some(index) = FUNCTION.iter().position(|k| *k == key) {
        return Some(format!("F{}", index + 1));
    }
    KEY_NAMES
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, name)| (*name).to_owned())
}

fn key_from_label(label: &str) -> Option<KeyCode> {
    let mut chars = label.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_uppercase() {
            return Some(LETTERS[(c as u8 - b'A') as usize]);
        }
        if c.is_ascii_digit() {
            return Some(DIGITS[(c as u8 - b'0') as usize]);
        }
    }
    if let Some(number) = label
        .strip_prefix('F')
        .and_then(|n| n.parse::<usize>().ok())
        && (1..=12).contains(&number)
    {
        return Some(FUNCTION[number - 1]);
    }
    KEY_NAMES
        .iter()
        .find(|(_, name)| *name == label)
        .map(|(k, _)| *k)
}

/// True for the keys that are modifiers (never captured alone, except `Alt` through the table).
pub fn is_modifier(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    )
}

impl Chord {
    pub const fn key(key: KeyCode) -> Self {
        Self {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    /// Parses `Maj+Tab`, `Ctrl+Tab`, `W`, `Entrée`…; `None` for an unknown key.
    pub fn parse(text: &str) -> Option<Self> {
        // The label of a key may itself hold a `+` (`Pavé +`).
        let (modifiers, key_text) = KEY_NAMES
            .iter()
            .map(|(_, name)| *name)
            .find(|name| name.contains('+') && text.ends_with(name))
            .map_or_else(
                || match text.rsplit_once('+') {
                    Some((head, tail)) => (head, tail.trim()),
                    None => ("", text.trim()),
                },
                |name| (text[..text.len() - name.len()].trim_end_matches('+'), name),
            );
        let key = key_from_label(key_text)?;
        let parts: Vec<&str> = modifiers
            .split('+')
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .collect();
        let mut chord = Chord::key(key);
        for modifier in parts {
            match modifier {
                "Maj" => chord.shift = true,
                "Ctrl" => chord.ctrl = true,
                "Alt" => chord.alt = true,
                _ => return None,
            }
        }
        Some(chord)
    }

    /// Label in the shortcut-sheet vocabulary (QWERTY positions).
    pub fn label(&self) -> String {
        let mut text = String::new();
        if self.ctrl {
            text.push_str("Ctrl+");
        }
        if self.alt {
            text.push_str("Alt+");
        }
        if self.shift {
            text.push_str("Maj+");
        }
        text.push_str(&key_label(self.key).unwrap_or_else(|| format!("{:?}", self.key)));
        text
    }

    /// Chord of the key being pressed now, from the modifier state of `keys`.
    pub fn from_pressed(key: KeyCode, keys: &ButtonInput<KeyCode>) -> Self {
        Self {
            key,
            shift: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            ctrl: keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
            alt: keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
        }
    }

    fn modifiers_match(&self, keys: &ButtonInput<KeyCode>) -> bool {
        let own = Chord::from_pressed(self.key, keys);
        let key_is = |a: KeyCode, b: KeyCode| self.key == a || self.key == b;
        (self.shift == own.shift || key_is(KeyCode::ShiftLeft, KeyCode::ShiftRight))
            && (self.ctrl == own.ctrl || key_is(KeyCode::ControlLeft, KeyCode::ControlRight))
            && (self.alt == own.alt || key_is(KeyCode::AltLeft, KeyCode::AltRight))
    }

    /// The alternative physical key of a modifier chord (`Alt` is on both sides).
    fn twin(&self) -> Option<KeyCode> {
        match self.key {
            KeyCode::AltLeft => Some(KeyCode::AltRight),
            KeyCode::AltRight => Some(KeyCode::AltLeft),
            _ => None,
        }
    }

    pub fn pressed(&self, keys: &ButtonInput<KeyCode>) -> bool {
        (keys.pressed(self.key) || self.twin().is_some_and(|twin| keys.pressed(twin)))
            && self.modifiers_match(keys)
    }

    pub fn just_pressed(&self, keys: &ButtonInput<KeyCode>) -> bool {
        (keys.just_pressed(self.key) || self.twin().is_some_and(|twin| keys.just_pressed(twin)))
            && self.modifiers_match(keys)
    }
}

/// An action of the key map.
#[derive(Debug, Clone, Copy)]
pub struct ActionDef {
    pub id: &'static str,
    pub label: &'static str,
    pub section: &'static str,
    /// Default chords (labels).
    pub defaults: &'static [&'static str],
    /// Only acts while a tooltip is open: may share a key with a map action.
    pub contextual: bool,
    /// Battle action: conflicts (swap on re-binding) are only looked for among battle actions,
    /// since the campaign map and the battlefield never listen at the same time.
    pub battle: bool,
}

const fn action(
    section: &'static str,
    id: &'static str,
    label: &'static str,
    defaults: &'static [&'static str],
) -> ActionDef {
    ActionDef {
        id,
        label,
        section,
        defaults,
        contextual: false,
        battle: false,
    }
}

const fn battle(
    id: &'static str,
    label: &'static str,
    defaults: &'static [&'static str],
) -> ActionDef {
    ActionDef {
        id,
        label,
        section: SECTION_BATTLE,
        defaults,
        contextual: false,
        battle: true,
    }
}

/// Title of the section of the battle actions.
pub const SECTION_BATTLE: &str = "Bataille";
/// Section of the developer hooks (`army_test_*`): kept out of the Commandes screen.
pub const SECTION_DEV: &str = "Essais (développement)";

const fn contextual(
    section: &'static str,
    id: &'static str,
    label: &'static str,
    defaults: &'static [&'static str],
) -> ActionDef {
    ActionDef {
        id,
        label,
        section,
        defaults,
        contextual: true,
        battle: false,
    }
}

/// Re-bindable actions, in the order of the Godot sheet. Defaults follow `project.godot`, except the
/// panels the Rust client already opens with a function key (F3 diplomacy, F4 technologies, F6
/// quick save: F5 is the chronicle here).
pub const ACTIONS: &[ActionDef] = &[
    action("Caméra", "map_pan_up", "Avancer", &["W", "↑"]),
    action("Caméra", "map_pan_left", "Aller à gauche", &["A", "←"]),
    action("Caméra", "map_pan_down", "Reculer", &["S", "↓"]),
    action("Caméra", "map_pan_right", "Aller à droite", &["D", "→"]),
    action("Caméra", "map_rotate_left", "Tourner à gauche", &["Q"]),
    action("Caméra", "map_rotate_right", "Tourner à droite", &["E"]),
    action(
        "Caméra",
        "map_toggle_edge_pan",
        "Défilement par les bords (oui / non)",
        &["ScrollLock"],
    ),
    action(
        "Fenêtres",
        "map_toggle_court",
        "Cour et personnages",
        &["C"],
    ),
    action("Fenêtres", "map_toggle_tech", "Technologies", &["F4"]),
    action("Fenêtres", "map_toggle_chronicle", "Chronique", &["F5"]),
    action(
        "Fenêtres",
        "map_toggle_diplomacy",
        "Diplomatie et religion",
        &["F3"],
    ),
    action("Fenêtres", "map_toggle_objectives", "Objectifs", &["O"]),
    action("Fenêtres", "map_toggle_agents", "Agents", &["G"]),
    action(
        "Fenêtres",
        "map_toggle_units",
        "Mes unités (armées et agents)",
        &["U"],
    ),
    action(
        "Fenêtres",
        "map_toggle_holdings",
        "Colonies (revenus, chantiers, menaces)",
        &["B"],
    ),
    action(
        "Fenêtres",
        "map_toggle_council",
        "Conseil du royaume (charges et conseils)",
        &["J"],
    ),
    action(
        "Fenêtres",
        "map_toggle_economy",
        "Économie (recettes, commerce, monnaie)",
        &["M"],
    ),
    action(
        "Fenêtres",
        "codex_open",
        "Codex (histoire et règles)",
        &["K"],
    ),
    action(
        "Fenêtres",
        "encyclopedia_open",
        "Codex, onglet Règles",
        &["L"],
    ),
    action("Fenêtres", "help_open", "Aide et raccourcis", &["F1"]),
    action(
        "Filtres de carte",
        "map_filters_menu",
        "Menu des filtres (richesse, population, loyauté…)",
        &["F"],
    ),
    action(
        "Filtres de carte",
        "map_toggle_unrest",
        "Mécontentement",
        &["Maj+M"],
    ),
    action(
        "Filtres de carte",
        "map_mode_diplomacy",
        "Carte diplomatique",
        &["N"],
    ),
    action(
        "Filtres de carte",
        "map_mode_religion",
        "Carte religieuse",
        &["R"],
    ),
    action(
        "Filtres de carte",
        "map_toggle_trade",
        "Routes commerciales",
        &["V"],
    ),
    action(
        "Filtres de carte",
        "campaign_commerce",
        "Économie, onglet Commerce",
        &["I"],
    ),
    action(
        "Armées et colonies",
        "campaign_next_idle",
        "Armée inactive suivante",
        &["Tab"],
    ),
    action(
        "Armées et colonies",
        "campaign_prev_idle",
        "Armée inactive précédente",
        &["Maj+Tab"],
    ),
    action(
        "Armées et colonies",
        "campaign_next_settlement",
        "Colonie suivante",
        &["Ctrl+Tab"],
    ),
    action(
        "Armées et colonies",
        "campaign_capital",
        "Aller à la capitale",
        &["Début"],
    ),
    action(
        "Armées et colonies",
        "army_center",
        "Centrer sur l’armée sélectionnée",
        &["."],
    ),
    action(
        "Armées et colonies",
        "army_follow",
        "Suivre l’armée sélectionnée",
        &["Y"],
    ),
    action(
        "Armées et colonies",
        "army_split",
        "Séparer les régiments choisis",
        &["X"],
    ),
    action(
        "Armées et colonies",
        "army_garrison",
        "Laisser les régiments choisis en garnison",
        &["H"],
    ),
    action(
        "Armées et colonies",
        "army_stance_normal",
        "Posture : normale",
        &["1"],
    ),
    action(
        "Armées et colonies",
        "army_stance_raid",
        "Posture : chevauchée",
        &["2"],
    ),
    action(
        "Armées et colonies",
        "army_stance_siege",
        "Posture : siège",
        &["3"],
    ),
    action(
        "Armées et colonies",
        "army_stance_ambush",
        "Posture : embuscade",
        &["4"],
    ),
    action(
        "Armées et colonies",
        "army_stance_forced_march",
        "Posture : marche forcée",
        &["5"],
    ),
    action(
        "Armées et colonies",
        "army_stance_entrenched",
        "Posture : camp retranché",
        &["6"],
    ),
    action(
        "Partie",
        "campaign_end_turn",
        "Finir la saison",
        &["Entrée", "Entrée (pavé)"],
    ),
    action(
        "Partie",
        "campaign_end_turn_fast",
        "Finir la saison sans confirmation",
        &["Maj+Entrée"],
    ),
    action(
        "Partie",
        "campaign_pause",
        "Fermer la fenêtre du dessus, puis menu pause",
        &["Échap"],
    ),
    action("Partie", "quick_save", "Sauvegarde rapide", &["F6"]),
    action("Partie", "quick_load", "Chargement rapide", &["F9"]),
    action("Partie", "map_screenshot", "Capture d’écran", &["F12"]),
    action("Partie", "settings", "Réglages", &["F10"]),
    action(
        "Partie",
        "toggle_fullscreen",
        "Plein écran / fenêtré",
        &["F11"],
    ),
    contextual(
        "Partie",
        "codex_pin_tooltip",
        "Maintenir ouverte la bulle ou l’infobulle",
        &["T"],
    ),
    contextual(
        "Partie",
        "tooltip_explore",
        "Maintenir : explorer les infobulles en chaîne",
        &["Alt"],
    ),
    battle("battle_fire_at_will", "Tir à volonté ou tir retenu", &["F"]),
    battle("battle_formation", "Formation suivante", &["T"]),
    battle("battle_halt", "Halte", &["H"]),
    battle("battle_pursue", "Poursuivre les fuyards", &["P"]),
    battle("battle_run", "Allure : course", &["R"]),
    battle("battle_guard", "Garde (tenir sa position)", &["G"]),
    battle("battle_skirmish", "Escarmouche", &["K"]),
    battle("battle_melee", "Mêlée", &["M"]),
    battle(
        "battle_select_all",
        "Sélectionner toutes les unités",
        &["Ctrl+A"],
    ),
    battle(
        "battle_select_shooters",
        "Sélectionner tous les tireurs",
        &["Ctrl+Maj+A"],
    ),
    battle(
        "battle_select_cavalry",
        "Sélectionner toute la cavalerie",
        &["Ctrl+Maj+C"],
    ),
    battle("battle_lock_group", "Verrouiller le groupe", &["Ctrl+G"]),
    battle("battle_next_idle", "Unité au repos suivante", &["."]),
    battle("battle_prev_idle", "Unité au repos précédente", &[","]),
    battle("battle_pause", "Pause", &["Espace"]),
    battle("battle_speed_up", "Accélérer le temps", &["Pavé +", "="]),
    battle("battle_speed_down", "Ralentir le temps", &["Pavé -", "-"]),
    battle("battle_tactical_view", "Vue tactique", &["Tab"]),
    battle(
        "battle_follow",
        "Suivre la sélection (ou le général)",
        &["C"],
    ),
    battle("battle_markers", "Masquer / afficher les bannières", &["U"]),
    battle("battle_journal", "Journal de bataille", &["J"]),
    battle("battle_order_1", "Barre des ordres : 1re case", &["Z"]),
    battle("battle_order_2", "Barre des ordres : 2e case", &["X"]),
    battle("battle_order_3", "Barre des ordres : 3e case", &["V"]),
    battle("battle_order_4", "Barre des ordres : 4e case", &["B"]),
    battle("battle_burn", "Incendier (siège)", &["I"]),
    battle("battle_ability_1", "Capacité 1", &["Alt+1"]),
    battle("battle_ability_2", "Capacité 2", &["Alt+2"]),
    battle("battle_ability_3", "Capacité 3", &["Alt+3"]),
    battle("battle_ability_4", "Capacité 4", &["Alt+4"]),
    battle(
        "battle_mark_1",
        "Repère de caméra 1 (Ctrl : enregistrer)",
        &["F2"],
    ),
    battle(
        "battle_mark_2",
        "Repère de caméra 2 (Ctrl : enregistrer)",
        &["F3"],
    ),
    battle(
        "battle_mark_3",
        "Repère de caméra 3 (Ctrl : enregistrer)",
        &["F4"],
    ),
    contextual(
        SECTION_DEV,
        "army_test_cycle",
        "Essai : armée suivante",
        &["Tab"],
    ),
    contextual(SECTION_DEV, "army_test_march", "Essai : marche", &["M"]),
];

pub fn definition(id: &str) -> Option<&'static ActionDef> {
    ACTIONS.iter().find(|a| a.id == id)
}

/// Section titles in order.
pub fn sections() -> Vec<&'static str> {
    let mut titles: Vec<&'static str> = Vec::new();
    for def in ACTIONS {
        if def.section != SECTION_DEV && !titles.contains(&def.section) {
            titles.push(def.section);
        }
    }
    titles
}

fn default_chords(def: &ActionDef) -> Vec<Chord> {
    def.defaults
        .iter()
        .filter_map(|text| Chord::parse(text))
        .collect()
}

/// Resolved chords of every action (defaults overlaid by the stored bindings).
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct KeyMap {
    chords: BTreeMap<&'static str, Vec<Chord>>,
}

impl KeyMap {
    /// Defaults overlaid by `stored` (unknown actions and unreadable chords are ignored; an
    /// action left without a valid chord keeps its defaults).
    pub fn from_bindings(stored: &BTreeMap<String, Vec<String>>) -> Self {
        let mut chords = BTreeMap::new();
        for def in ACTIONS {
            let custom: Vec<Chord> = stored
                .get(def.id)
                .map(|labels| labels.iter().filter_map(|l| Chord::parse(l)).collect())
                .unwrap_or_default();
            chords.insert(
                def.id,
                if custom.is_empty() {
                    default_chords(def)
                } else {
                    custom
                },
            );
        }
        Self { chords }
    }

    pub fn defaults() -> Self {
        Self::from_bindings(&BTreeMap::new())
    }

    pub fn chords(&self, action: &str) -> &[Chord] {
        self.chords.get(action).map_or(&[], Vec::as_slice)
    }

    /// Any chord of `action` is held.
    pub fn pressed(&self, action: &str, keys: &ButtonInput<KeyCode>) -> bool {
        self.chords(action).iter().any(|c| c.pressed(keys))
    }

    /// Any chord of `action` went down this frame.
    pub fn just_pressed(&self, action: &str, keys: &ButtonInput<KeyCode>) -> bool {
        self.chords(action).iter().any(|c| c.just_pressed(keys))
    }

    /// Labels of the chords (`W / ↑`) for the layout (`azerty` swaps A/Q, Z/W, M).
    pub fn label(&self, action: &str, layout: &str) -> String {
        self.chords(action)
            .iter()
            .map(|c| layout_label(&c.label(), layout))
            .collect::<Vec<_>>()
            .join(" / ")
    }

    /// Non-contextual action (other than `except`) using `chord`.
    pub fn action_using(&self, chord: &Chord, except: &str) -> Option<&'static str> {
        let battle_scope = definition(except).is_some_and(|def| def.battle);
        ACTIONS
            .iter()
            .filter(|def| def.id != except && !def.contextual && def.battle == battle_scope)
            .find(|def| self.chords(def.id).contains(chord))
            .map(|def| def.id)
    }

    /// Number of actions that differ from their defaults.
    pub fn custom_count(&self) -> usize {
        ACTIONS
            .iter()
            .filter(|def| self.chords(def.id) != default_chords(def).as_slice())
            .count()
    }
}

/// QWERTY-position label shown for `layout` (`azerty`: A<->Q, Z<->W, M -> `,`).
pub fn layout_label(label: &str, layout: &str) -> String {
    if layout != "azerty" {
        return label.to_owned();
    }
    let mut parts: Vec<String> = label.split('+').map(str::to_owned).collect();
    if let Some(last) = parts.last_mut() {
        *last = match last.as_str() {
            "Q" => "A",
            "A" => "Q",
            "W" => "Z",
            "Z" => "W",
            "M" => ",",
            other => other,
        }
        .to_owned();
    }
    parts.join("+")
}

/// Stores the chords of the touched actions in `bindings` (an action back on its defaults is
/// removed so the file only lists real changes).
fn store(bindings: &mut BTreeMap<String, Vec<String>>, map: &KeyMap, touched: &[&'static str]) {
    for id in touched {
        let Some(def) = definition(id) else { continue };
        let current = map.chords(id);
        if current == default_chords(def).as_slice() {
            bindings.remove(*id);
        } else {
            bindings.insert((*id).to_owned(), current.iter().map(Chord::label).collect());
        }
    }
}

/// Moves slot `slot` of `action` to `chord`. A conflicting action receives the old chord of
/// `action` (swap); returns that action. `None` without conflict (or when nothing changes).
pub fn rebind(
    bindings: &mut BTreeMap<String, Vec<String>>,
    action: &str,
    slot: usize,
    chord: Chord,
) -> Option<&'static str> {
    let def = definition(action)?;
    let mut map = KeyMap::from_bindings(bindings);
    let mut mine = map.chords(def.id).to_vec();
    if mine.is_empty() {
        return None;
    }
    let slot = slot.min(mine.len() - 1);
    let old = mine[slot];
    if old == chord {
        return None;
    }
    let other = if def.contextual {
        None
    } else {
        map.action_using(&chord, def.id)
    };
    let mut touched = vec![def.id];
    if let Some(other_id) = other {
        let mut theirs = map.chords(other_id).to_vec();
        if let Some(position) = theirs.iter().position(|c| *c == chord) {
            theirs[position] = old;
        }
        map.chords.insert(other_id, theirs);
        touched.push(other_id);
    } else if let Some(position) = mine.iter().position(|c| *c == chord) {
        mine[position] = old;
    }
    mine[slot] = chord;
    map.chords.insert(def.id, mine);
    store(bindings, &map, &touched);
    other
}

/// Puts `action` back on its default keys.
pub fn reset_action(bindings: &mut BTreeMap<String, Vec<String>>, action: &str) {
    bindings.remove(action);
}

/// Rebuilds the [`KeyMap`] when the settings change.
pub fn sync_keymap(settings: Res<super::Settings>, mut map: ResMut<KeyMap>) {
    if !settings.is_changed() {
        return;
    }
    let wanted = KeyMap::from_bindings(&settings.bindings);
    if *map != wanted {
        *map = wanted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_round_trip_and_defaults_parse() {
        for def in ACTIONS {
            for text in def.defaults {
                let chord = Chord::parse(text).unwrap_or_else(|| panic!("{} {text}", def.id));
                assert_eq!(&chord.label(), text, "{}", def.id);
            }
        }
        let ids: std::collections::BTreeSet<_> = ACTIONS.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), ACTIONS.len());
        assert_eq!(
            Chord::parse("Maj+Tab"),
            Some(Chord {
                shift: true,
                ..Chord::key(KeyCode::Tab)
            })
        );
        assert_eq!(Chord::parse("Nope"), None);
    }

    #[test]
    fn no_default_conflict_between_map_actions() {
        let map = KeyMap::defaults();
        for def in ACTIONS.iter().filter(|d| !d.contextual) {
            for chord in map.chords(def.id) {
                assert_eq!(
                    map.action_using(chord, def.id),
                    None,
                    "{} {chord:?}",
                    def.id
                );
            }
        }
        assert_eq!(map.custom_count(), 0);
    }

    #[test]
    fn rebind_swaps_on_conflict_and_stores_only_changes() {
        let mut bindings = BTreeMap::new();
        // Free key: only the action changes.
        assert_eq!(
            rebind(
                &mut bindings,
                "map_rotate_left",
                0,
                Chord::key(KeyCode::KeyJ)
            ),
            Some("map_toggle_council")
        );
        let map = KeyMap::from_bindings(&bindings);
        assert_eq!(map.label("map_rotate_left", "qwerty"), "J");
        assert_eq!(map.label("map_toggle_council", "qwerty"), "Q");
        assert_eq!(map.custom_count(), 2);
        // Back to the old key: everything returns to the defaults and the file is empty again.
        rebind(
            &mut bindings,
            "map_rotate_left",
            0,
            Chord::key(KeyCode::KeyQ),
        );
        assert!(bindings.is_empty(), "{bindings:?}");
        // Second slot of an action.
        rebind(&mut bindings, "map_pan_up", 1, Chord::key(KeyCode::KeyI));
        assert_eq!(
            KeyMap::from_bindings(&bindings).label("map_pan_up", "azerty"),
            "Z / I"
        );
        // Contextual actions never swap.
        assert_eq!(
            rebind(
                &mut bindings,
                "codex_pin_tooltip",
                0,
                Chord::key(KeyCode::KeyK)
            ),
            None
        );
    }

    #[test]
    fn exact_modifiers() {
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ShiftLeft);
        keys.press(KeyCode::Tab);
        let map = KeyMap::defaults();
        assert!(map.just_pressed("campaign_prev_idle", &keys));
        assert!(!map.just_pressed("campaign_next_idle", &keys));
        let mut alt = ButtonInput::<KeyCode>::default();
        alt.press(KeyCode::AltRight);
        assert!(map.pressed("tooltip_explore", &alt));
    }

    #[test]
    fn unreadable_stored_chords_fall_back() {
        let mut bindings = BTreeMap::new();
        bindings.insert("map_pan_up".to_owned(), vec!["???".to_owned()]);
        bindings.insert("nope".to_owned(), vec!["A".to_owned()]);
        assert_eq!(KeyMap::from_bindings(&bindings), KeyMap::defaults());
    }

    #[test]
    fn battle_actions_do_not_conflict_with_the_map() {
        // F, H, M… are map keys and battle keys at once: no swap across the two scopes.
        let mut bindings = BTreeMap::new();
        assert_eq!(
            rebind(&mut bindings, "battle_halt", 0, Chord::key(KeyCode::KeyF)),
            Some("battle_fire_at_will")
        );
        assert_eq!(
            rebind(&mut bindings, "map_pan_up", 1, Chord::key(KeyCode::KeyH)),
            Some("army_garrison")
        );
        let map = KeyMap::from_bindings(&bindings);
        assert_eq!(map.label("battle_halt", "qwerty"), "F");
        assert_eq!(map.label("battle_fire_at_will", "qwerty"), "H");
        assert!(sections().contains(&SECTION_BATTLE));
        assert!(!sections().contains(&SECTION_DEV));
    }

    #[test]
    fn campaign_defaults_have_no_duplicate_chord() {
        let map = KeyMap::defaults();
        for def in ACTIONS.iter().filter(|d| !d.contextual) {
            for chord in map.chords(def.id) {
                assert_eq!(map.action_using(chord, def.id), None, "{}", def.id);
            }
        }
    }
}
