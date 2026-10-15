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

/// Digit row key to number 1..9.
pub fn digit_of(key: KeyCode) -> Option<u8> {
    Some(match key {
        KeyCode::Digit1 | KeyCode::Numpad1 => 1,
        KeyCode::Digit2 | KeyCode::Numpad2 => 2,
        KeyCode::Digit3 | KeyCode::Numpad3 => 3,
        KeyCode::Digit4 | KeyCode::Numpad4 => 4,
        KeyCode::Digit5 | KeyCode::Numpad5 => 5,
        KeyCode::Digit6 | KeyCode::Numpad6 => 6,
        KeyCode::Digit7 | KeyCode::Numpad7 => 7,
        KeyCode::Digit8 | KeyCode::Numpad8 => 8,
        KeyCode::Digit9 | KeyCode::Numpad9 => 9,
        _ => return None,
    })
}

/// The action of `key` pressed with `mods`, if any. Ctrl stands for Cmd on macOS.
pub fn action_for(key: KeyCode, mods: Modifiers) -> Option<Action> {
    if let Some(number) = digit_of(key) {
        if mods.alt && mods.shift {
            return Some(Action::Preset(number));
        }
        if mods.alt {
            return (number <= 4).then_some(Action::Ability(number));
        }
        return Some(Action::Group {
            number,
            save: mods.ctrl,
        });
    }
    let plain = !mods.ctrl && !mods.alt && !mods.shift;
    let ctrl_only = mods.ctrl && !mods.alt && !mods.shift;
    let ctrl_shift = mods.ctrl && mods.shift && !mods.alt;
    match key {
        KeyCode::KeyF if plain => Some(Action::FireAtWill),
        KeyCode::KeyT if plain => Some(Action::CycleFormation),
        KeyCode::KeyH if plain => Some(Action::Halt),
        KeyCode::KeyP if plain => Some(Action::Pursue),
        KeyCode::KeyR if plain => Some(Action::Mode(UnitMode::Run)),
        KeyCode::KeyG if plain => Some(Action::Mode(UnitMode::Guard)),
        KeyCode::KeyK if plain => Some(Action::Mode(UnitMode::Skirmish)),
        KeyCode::KeyM if plain => Some(Action::Mode(UnitMode::Melee)),
        KeyCode::KeyA if ctrl_only => Some(Action::SelectAll),
        KeyCode::KeyA if ctrl_shift => Some(Action::SelectShooters),
        KeyCode::KeyC if ctrl_shift => Some(Action::SelectCavalry),
        KeyCode::KeyG if ctrl_only => Some(Action::LockGroup),
        KeyCode::Period if plain => Some(Action::NextIdle),
        KeyCode::Comma if plain => Some(Action::PrevIdle),
        _ => None,
    }
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
        assert_eq!(digit_of(KeyCode::Digit0), None);
    }
}
