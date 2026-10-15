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
