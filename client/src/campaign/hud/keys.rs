//! Keyboard helpers shared by the campaign shortcuts: key-map lookup with a documented fallback.
//! Shortcuts must also be gated on `ui::kit::text_field::keyboard_free` (no typing in a field).

use bevy::prelude::*;

use crate::settings::KeyMap;

/// `action` held (`edge = false`) or just pressed (`edge = true`) through the key map; without a
/// key map (or an action with no chord) the default `fallback` key is used.
pub fn action_down(
    keymap: Option<&KeyMap>,
    action: &str,
    fallback: KeyCode,
    keys: &ButtonInput<KeyCode>,
    edge: bool,
) -> bool {
    match keymap {
        Some(map) if !map.chords(action).is_empty() => {
            if edge {
                map.just_pressed(action, keys)
            } else {
                map.pressed(action, keys)
            }
        }
        _ => {
            if edge {
                keys.just_pressed(fallback)
            } else {
                keys.pressed(fallback)
            }
        }
    }
}

/// Window launchers that also have a key: `(key map action, button name)`. A key press sends the
/// same `ButtonPressed` as a click on the launcher, so the window opens identically.
pub const WINDOW_SHORTCUTS: [(&str, &str); 3] = [
    ("map_toggle_recruitment", "hud:open_recruitment"),
    ("map_toggle_faction", "hud:open_faction"),
    ("map_toggle_feudal", "hud:open_feudal"),
];

/// Key -> launcher press for [`WINDOW_SHORTCUTS`] (the Codex has its own system in `codex`).
pub fn window_shortcuts(
    keymap: Res<KeyMap>,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Option<Res<crate::ui::kit::text_field::TextInputFocus>>,
    paused: Option<Res<crate::menu::pause::GamePaused>>,
    mut pressed: MessageWriter<crate::ui::kit::ButtonPressed>,
) {
    if !crate::ui::kit::text_field::keyboard_free(focus) || paused.is_some_and(|paused| paused.0) {
        return;
    }
    for (action, name) in WINDOW_SHORTCUTS {
        if keymap.just_pressed(action, &keys) {
            pressed.write(crate::ui::kit::ButtonPressed {
                entity: Entity::PLACEHOLDER,
                name: name.to_owned(),
            });
        }
    }
}
