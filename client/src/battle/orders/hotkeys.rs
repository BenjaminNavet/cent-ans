//! Battle hotkey table (pure): which key plus modifiers is which action.
//!
//! Ports the default bindings of game/scripts/battle/battle_hotkeys.gd (orders, modes,
//! selection and groups). Keys are physical (`KeyCode`, as the camera): on an AZERTY keyboard
//! the letters follow the QWERTY positions. Not ported: rebinding (`Settings` commands
//! screen). Abilities Alt+1..4 and presets Alt+Shift+n are table entries performed in
//! `input.rs`; camera bookmarks (F2..F4) are read by `marks.rs`.

use bevy::prelude::*;
use sim_battle::UnitMode;

use super::drag::Modifiers;
use crate::settings::Chord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FireAtWill,
    /// Next formation allowed for each regiment.
    CycleFormation,
    Halt,
    Pursue,
    Withdraw,
    Mode(UnitMode),
    SelectAll,
    SelectShooters,
    SelectCavalry,
    NextIdle,
    PrevIdle,
    LockGroup,
    /// Select (or, with `save`, record) group `number`.
    Group {
        number: u8,
        save: bool,
    },
    /// Alt+1..4: ability slot of the selection.
    Ability(u8),
    /// Alt+Shift+1..9: group formation preset (1 = first of the data).
    Preset(u8),
}

/// Digit-row keys 1..9 (index 0..8), the default of `battle_group_n`.
const DIGITS: [KeyCode; 9] = [
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

/// Numeric-pad twin of a digit-row key (a digit chord also answers to its pad key).
fn pad_twin(key: KeyCode) -> Option<KeyCode> {
    DIGITS.iter().position(|d| *d == key).map(|i| match i {
        0 => KeyCode::Numpad1,
        1 => KeyCode::Numpad2,
        2 => KeyCode::Numpad3,
        3 => KeyCode::Numpad4,
        4 => KeyCode::Numpad5,
        5 => KeyCode::Numpad6,
        6 => KeyCode::Numpad7,
        7 => KeyCode::Numpad8,
        _ => KeyCode::Numpad9,
    })
}

/// `pressed` is the key of `chord` (digits also match their numeric-pad twin).
fn same_key(chord: KeyCode, pressed: KeyCode) -> bool {
    chord == pressed || pad_twin(chord) == Some(pressed)
}

/// Chords of the key map action `id`, or `fallback` when the key map does not define it (or
/// is absent): the defaults stay usable without the settings lot's actions.
fn chords_or(keymap: Option<&crate::settings::KeyMap>, id: &str, fallback: Chord) -> Vec<Chord> {
    match keymap.map(|map| map.chords(id)) {
        Some(chords) if !chords.is_empty() => chords.to_vec(),
        _ => vec![fallback],
    }
}

/// The action of `key` pressed with `mods` under the default bindings. Ctrl stands for Cmd.
pub fn action_for(key: KeyCode, mods: Modifiers) -> Option<Action> {
    action_for_bound(key, mods, None, None)
}

/// The chord of `id` (Ctrl is not part of it) that `keys` just pressed, with whether Ctrl is
/// held: Ctrl+chord is the "save" variant of the same action (groups, camera marks). Shift and
/// Alt must match the chord exactly.
pub fn just_pressed_saving(
    keymap: Option<&crate::settings::KeyMap>,
    id: &str,
    fallback: Chord,
    keys: &ButtonInput<KeyCode>,
) -> Option<bool> {
    let mods = super::input::modifiers(keys);
    chords_or(keymap, id, fallback)
        .iter()
        .any(|c| {
            keys.get_just_pressed().any(|k| same_key(c.key, *k))
                && c.shift == mods.shift
                && c.alt == mods.alt
        })
        .then_some(mods.ctrl)
}

/// A rebindable row of the table: stable id (`battle_<id>` in the key map, `<id>` in the
/// `input/battle_bindings` setting), action, default key and whether Ctrl / Shift are held.
struct Row {
    id: &'static str,
    action: Action,
    key: KeyCode,
    ctrl: bool,
    shift: bool,
}

const fn row(id: &'static str, action: Action, key: KeyCode, ctrl: bool, shift: bool) -> Row {
    Row {
        id,
        action,
        key,
        ctrl,
        shift,
    }
}

/// Defaults of battle_hotkeys.gd (`dispatch` rows with a key). Digits stay fixed (groups,
/// abilities, presets), as in Godot.
const ROWS: &[Row] = &[
    row(
        "fire_at_will",
        Action::FireAtWill,
        KeyCode::KeyF,
        false,
        false,
    ),
    row(
        "formation",
        Action::CycleFormation,
        KeyCode::KeyT,
        false,
        false,
    ),
    row("halt", Action::Halt, KeyCode::KeyH, false, false),
    row("pursue", Action::Pursue, KeyCode::KeyP, false, false),
    row(
        "run",
        Action::Mode(UnitMode::Run),
        KeyCode::KeyR,
        false,
        false,
    ),
    row(
        "guard",
        Action::Mode(UnitMode::Guard),
        KeyCode::KeyG,
        false,
        false,
    ),
    row(
        "skirmish",
        Action::Mode(UnitMode::Skirmish),
        KeyCode::KeyK,
        false,
        false,
    ),
    row(
        "melee",
        Action::Mode(UnitMode::Melee),
        KeyCode::KeyM,
        false,
        false,
    ),
    row("select_all", Action::SelectAll, KeyCode::KeyA, true, false),
    row(
        "select_shooters",
        Action::SelectShooters,
        KeyCode::KeyA,
        true,
        true,
    ),
    row(
        "select_cavalry",
        Action::SelectCavalry,
        KeyCode::KeyC,
        true,
        true,
    ),
    row("lock_group", Action::LockGroup, KeyCode::KeyG, true, false),
    row("next_idle", Action::NextIdle, KeyCode::Period, false, false),
    row("prev_idle", Action::PrevIdle, KeyCode::Comma, false, false),
];

/// Chord in force for `row`: the key map action `battle_<id>` if the settings lot defines it,
/// else the stored `input/battle_bindings` entry, else the default of the table.
fn effective(
    row: &Row,
    keymap: Option<&crate::settings::KeyMap>,
    stored: Option<&std::collections::BTreeMap<String, String>>,
) -> Vec<Chord> {
    if let Some(map) = keymap {
        let chords = map.chords(&format!("battle_{}", row.id));
        if !chords.is_empty() {
            return chords.to_vec();
        }
    }
    if let Some(chord) = stored
        .and_then(|bindings| bindings.get(row.id))
        .and_then(|text| Chord::parse(text))
    {
        return vec![chord];
    }
    vec![Chord {
        key: row.key,
        shift: row.shift,
        ctrl: row.ctrl,
        alt: false,
    }]
}

/// The action of `key` pressed with `mods` under the player's bindings (key map, then
/// settings, then defaults). Groups `battle_group_n`: the chord alone selects, with Ctrl it
/// records, with Alt+Shift it applies the formation preset (the chord's own modifiers apart).
/// Abilities `battle_ability_n` are exact chords.
pub fn action_for_bound(
    key: KeyCode,
    mods: Modifiers,
    keymap: Option<&crate::settings::KeyMap>,
    stored: Option<&std::collections::BTreeMap<String, String>>,
) -> Option<Action> {
    let exact = |c: &Chord| {
        same_key(c.key, key) && c.shift == mods.shift && c.ctrl == mods.ctrl && c.alt == mods.alt
    };
    if let Some(action) = ROWS.iter().find_map(|row| {
        effective(row, keymap, stored)
            .iter()
            .any(exact)
            .then_some(row.action)
    }) {
        return Some(action);
    }
    for (index, digit) in DIGITS.iter().enumerate() {
        let number = index as u8 + 1;
        if number <= 4 {
            let default = Chord {
                key: *digit,
                shift: false,
                ctrl: false,
                alt: true,
            };
            let ability = chords_or(keymap, &format!("battle_ability_{number}"), default);
            if ability.iter().any(exact) {
                return Some(Action::Ability(number));
            }
        }
        let default = Chord::key(*digit);
        for chord in chords_or(keymap, &format!("battle_group_{number}"), default) {
            if !same_key(chord.key, key) {
                continue;
            }
            if mods.alt && mods.shift && !chord.alt && !chord.shift {
                return Some(Action::Preset(number));
            }
            if chord.alt == mods.alt && chord.shift == mods.shift {
                return Some(Action::Group {
                    number,
                    save: mods.ctrl,
                });
            }
        }
    }
    None
}

/// French label of a hotkey action (toasts).
pub fn label_fr(action: Action) -> &'static str {
    match action {
        Action::FireAtWill => "tir à volonté",
        Action::CycleFormation => "formation",
        Action::Halt => "halte",
        Action::Pursue => "poursuite",
        Action::Withdraw => "retraite",
        Action::Mode(UnitMode::Run) => "course",
        Action::Mode(UnitMode::Guard) => "garde",
        Action::Mode(UnitMode::Skirmish) => "escarmouche",
        Action::Mode(UnitMode::Melee) => "mêlée",
        Action::Mode(UnitMode::Breach) => "brèche",
        Action::SelectAll => "tout sélectionner",
        Action::SelectShooters => "sélectionner les tireurs",
        Action::SelectCavalry => "sélectionner la cavalerie",
        Action::NextIdle | Action::PrevIdle => "unité au repos",
        Action::LockGroup => "verrouiller le groupe",
        Action::Group { .. } => "groupe",
        Action::Ability(_) => "capacité",
        Action::Preset(_) => "formation de groupe",
    }
}

/// The `use_ability` order of Alt+`slot` for regiments `ids` (selection order): ability number
/// `slot` of the first one that has that many, for every regiment of `ids` that has it. `None`
/// when no regiment has such an ability.
pub fn ability_command(
    sim: &sim_battle::BattleSim,
    ids: &[u32],
    slot: usize,
) -> Option<sim_battle::Command> {
    let ability = ids.iter().find_map(|id| {
        let unit = sim.unit(*id)?;
        sim.ability_views(unit)
            .get(slot.checked_sub(1)?)
            .map(|view| view.id.clone())
    })?;
    let having: Vec<u32> = ids
        .iter()
        .copied()
        .filter(|id| {
            sim.unit(*id)
                .is_some_and(|u| sim.ability_views(u).iter().any(|v| v.id == ability))
        })
        .collect();
    Some(sim_battle::Command::UseAbility {
        units: having,
        ability,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Modifiers = Modifiers {
        shift: false,
        ctrl: false,
        alt: false,
    };
    const CTRL: Modifiers = Modifiers {
        shift: false,
        ctrl: true,
        alt: false,
    };
    const CTRL_SHIFT: Modifiers = Modifiers {
        shift: true,
        ctrl: true,
        alt: false,
    };
    const ALT: Modifiers = Modifiers {
        shift: false,
        ctrl: false,
        alt: true,
    };

    #[test]
    fn table_matches_the_godot_defaults() {
        assert_eq!(action_for(KeyCode::KeyH, NONE), Some(Action::Halt));
        // The bound table agrees with the default one on every row.
        for row in ROWS {
            let mods = Modifiers {
                shift: row.shift,
                ctrl: row.ctrl,
                alt: false,
            };
            assert_eq!(
                action_for_bound(row.key, mods, None, None),
                Some(row.action)
            );
            assert_eq!(action_for(row.key, mods), Some(row.action));
        }
        assert_eq!(action_for(KeyCode::KeyF, NONE), Some(Action::FireAtWill));
        assert_eq!(
            action_for(KeyCode::KeyG, NONE),
            Some(Action::Mode(UnitMode::Guard))
        );
        assert_eq!(action_for(KeyCode::KeyG, CTRL), Some(Action::LockGroup));
        assert_eq!(action_for(KeyCode::KeyA, CTRL), Some(Action::SelectAll));
        assert_eq!(
            action_for(KeyCode::KeyA, CTRL_SHIFT),
            Some(Action::SelectShooters)
        );
        assert_eq!(
            action_for(KeyCode::KeyC, CTRL_SHIFT),
            Some(Action::SelectCavalry)
        );
        assert_eq!(action_for(KeyCode::KeyH, CTRL), None);
    }

    #[test]
    fn digits_save_with_ctrl_and_recall_without() {
        assert_eq!(
            action_for(KeyCode::Digit4, CTRL),
            Some(Action::Group {
                number: 4,
                save: true
            })
        );
        assert_eq!(
            action_for(KeyCode::Digit4, NONE),
            Some(Action::Group {
                number: 4,
                save: false
            })
        );
        assert_eq!(action_for(KeyCode::Digit4, ALT), Some(Action::Ability(4)));
        assert_eq!(action_for(KeyCode::Digit5, ALT), None);
        let alt_shift = Modifiers {
            shift: true,
            ctrl: false,
            alt: true,
        };
        assert_eq!(
            action_for(KeyCode::Digit2, alt_shift),
            Some(Action::Preset(2))
        );
        assert_eq!(action_for(KeyCode::Digit0, NONE), None);
    }
}

#[cfg(test)]
mod bound_tests {
    use super::*;

    const NONE: Modifiers = Modifiers {
        shift: false,
        ctrl: false,
        alt: false,
    };

    #[test]
    fn rebinding_replaces_the_default_key_and_bad_text_falls_back() {
        let mut stored = std::collections::BTreeMap::new();
        stored.insert("halt".to_owned(), "J".to_owned());
        stored.insert("pursue".to_owned(), "??".to_owned());
        assert_eq!(
            action_for_bound(KeyCode::KeyJ, NONE, None, Some(&stored)),
            Some(Action::Halt)
        );
        assert_eq!(
            action_for_bound(KeyCode::KeyH, NONE, None, Some(&stored)),
            None
        );
        assert_eq!(
            action_for_bound(KeyCode::KeyP, NONE, None, Some(&stored)),
            Some(Action::Pursue)
        );
    }

    fn map_with(action: &str, chord: &str) -> crate::settings::KeyMap {
        let mut stored = std::collections::BTreeMap::new();
        stored.insert(action.to_owned(), vec![chord.to_owned()]);
        crate::settings::KeyMap::from_bindings(&stored)
    }

    #[test]
    fn key_map_rebinding_works_and_the_old_key_is_ignored() {
        let map = map_with("battle_halt", "J");
        assert_eq!(
            action_for_bound(KeyCode::KeyJ, NONE, Some(&map), None),
            Some(Action::Halt)
        );
        assert_eq!(
            action_for_bound(KeyCode::KeyH, NONE, Some(&map), None),
            None
        );
        // Untouched actions keep their default.
        assert_eq!(
            action_for_bound(KeyCode::KeyP, NONE, Some(&map), None),
            Some(Action::Pursue)
        );
    }

    #[test]
    fn rebound_ability_replaces_alt_digit() {
        let map = map_with("battle_ability_1", "Alt+F");
        let alt = Modifiers {
            shift: false,
            ctrl: false,
            alt: true,
        };
        assert_eq!(
            action_for_bound(KeyCode::KeyF, alt, Some(&map), None),
            Some(Action::Ability(1))
        );
        assert_eq!(
            action_for_bound(KeyCode::Digit1, alt, Some(&map), None),
            None
        );
    }

    #[test]
    fn group_digits_tolerate_ctrl_and_numpad() {
        let ctrl = Modifiers {
            shift: false,
            ctrl: true,
            alt: false,
        };
        assert_eq!(
            action_for_bound(KeyCode::Numpad3, ctrl, None, None),
            Some(Action::Group {
                number: 3,
                save: true
            })
        );
    }

    #[test]
    fn saving_variant_reads_ctrl_and_rebinding() {
        let map = map_with("battle_mark_1", "F7");
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::F2);
        let default = Chord::key(KeyCode::F2);
        assert_eq!(
            just_pressed_saving(Some(&map), "battle_mark_1", default, &keys),
            None
        );
        keys.clear();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::F7);
        assert_eq!(
            just_pressed_saving(Some(&map), "battle_mark_1", default, &keys),
            Some(true)
        );
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::F7);
        assert_eq!(
            just_pressed_saving(Some(&map), "battle_mark_1", default, &keys),
            Some(false)
        );
    }
}
