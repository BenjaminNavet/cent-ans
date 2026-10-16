//! Formation outline draped on the relief, with states (port of
//! game/scripts/battle/battle_formation_outline.gd, spec CB-M « Contour de formation »).
//!
//! - selected: solid line, pale gold lightly tinted by the livery of the side;
//! - hovered (own): pale line;
//! - hovered enemy: red, dashed;
//! - targeted enemy (target of a selected regiment): red, pulsing;
//! - in melee, without another state: line in the livery (pale red for the enemy), so the
//!   regiments can be read in the mêlée;
//! - routing, absent: nothing.
//!
//! Presentation only; the state of a regiment comes from the simulation.

use bevy::prelude::*;
use sim_battle::Battlefield;

use super::overlay::{OrdersGizmos, dashed_line, ground_line};

/// Pale gold of the selection (DA bible: never pure yellow).
const PALE_GOLD: Color = Color::srgb(0.93, 0.84, 0.6);
/// Garance red of the enemy.
pub const ENEMY_RED: Color = Color::srgb(0.8, 0.16, 0.11);
/// Share of the livery mixed into the gold of the selection.
const LIVERY_TINT: f32 = 0.2;
/// Metres added to the width and the depth (as the old ring).
pub const MARGIN: f32 = 3.0;
const DASH_M: f32 = 4.0;
const GAP_M: f32 = 2.5;
const PULSE_HZ: f32 = 1.1;
/// Opacity at the trough of the pulse.
const PULSE_MIN: f32 = 0.35;
const LIFT: f32 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutlineState {
    #[default]
    None,
    Selected,
    Hovered,
    EnemyHovered,
    EnemyTargeted,
    Engaged,
}

/// State of the outline of a regiment (`outline_state` of Godot): the selection wins over the
/// hover, the target over the hover; rout and absence give nothing. `melee` only applies when
/// no other state does.
pub fn outline_state(
    present: bool,
    routing: bool,
    own: bool,
    selected: bool,
    hovered: bool,
    targeted: bool,
    melee: bool,
) -> OutlineState {
    if !present || routing {
        return OutlineState::None;
    }
    let state = if own {
        if selected {
            OutlineState::Selected
        } else if hovered {
            OutlineState::Hovered
        } else {
            OutlineState::None
        }
    } else if targeted {
        OutlineState::EnemyTargeted
    } else if hovered {
        OutlineState::EnemyHovered
    } else {
        OutlineState::None
    };
    if state == OutlineState::None && melee {
        OutlineState::Engaged
    } else {
        state
    }
}

/// Opacity of the pulse at `seconds`.
pub fn pulse_alpha(seconds: f32) -> f32 {
    let wave = 0.5 + 0.5 * (seconds * std::f32::consts::TAU * PULSE_HZ).sin();
    PULSE_MIN + (1.0 - PULSE_MIN) * wave
}

/// Colour of the outline for `state`; `livery` is the colour of the side of the regiment.
pub fn state_color(state: OutlineState, livery: Color, enemy_red: Color, own: bool) -> Color {
    let white = Color::WHITE;
    match state {
        OutlineState::None => Color::NONE,
        OutlineState::Selected => PALE_GOLD.mix(&livery, LIVERY_TINT).with_alpha(1.0),
        OutlineState::Hovered => PALE_GOLD.mix(&white, 0.4).with_alpha(0.6),
        OutlineState::EnemyHovered => enemy_red.with_alpha(0.95),
        OutlineState::EnemyTargeted => enemy_red.with_alpha(1.0),
        OutlineState::Engaged => {
            if own {
                livery.mix(&white, 0.45).with_alpha(0.7)
            } else {
                enemy_red.mix(&white, 0.25).with_alpha(0.6)
            }
        }
    }
}

/// Draws the outline of one regiment (centre, facing, size without the margin).
#[allow(clippy::too_many_arguments)]
pub fn draw_outline(
    gizmos: &mut Gizmos<OrdersGizmos>,
    field: &Battlefield,
    centre: Vec2,
    facing: f32,
    width: f32,
    depth: f32,
    state: OutlineState,
    color: Color,
) {
    let half_w = ((width + MARGIN) * 0.5).max(2.0);
    let half_d = ((depth + MARGIN) * 0.5).max(2.0);
    let right = Vec2::new(facing.cos(), -facing.sin());
    let forward = Vec2::new(facing.sin(), facing.cos());
    let corner = |sr: f32, sf: f32| centre + right * (half_w * sr) + forward * (half_d * sf);
    let corners = [
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
        corner(-1.0, -1.0),
    ];
    if state == OutlineState::EnemyHovered {
        dashed_line(gizmos, field, &corners, DASH_M, GAP_M, LIFT, color);
    } else {
        for pair in corners.windows(2) {
            gizmos.linestrip(ground_line(field, pair[0], pair[1], LIFT), color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(own: bool, sel: bool, hov: bool, tgt: bool, melee: bool) -> OutlineState {
        outline_state(true, false, own, sel, hov, tgt, melee)
    }

    #[test]
    fn selection_wins_over_hover_and_target_over_hover() {
        assert_eq!(
            state(true, true, true, false, false),
            OutlineState::Selected
        );
        assert_eq!(
            state(true, false, true, false, false),
            OutlineState::Hovered
        );
        assert_eq!(
            state(false, false, true, true, false),
            OutlineState::EnemyTargeted
        );
        assert_eq!(
            state(false, false, true, false, false),
            OutlineState::EnemyHovered
        );
        assert_eq!(state(false, false, false, false, false), OutlineState::None);
    }

    #[test]
    fn melee_only_when_nothing_else() {
        assert_eq!(
            state(true, false, false, false, true),
            OutlineState::Engaged
        );
        assert_eq!(
            state(false, false, false, false, true),
            OutlineState::Engaged
        );
        assert_eq!(
            state(true, true, false, false, true),
            OutlineState::Selected
        );
    }

    #[test]
    fn rout_and_absence_draw_nothing() {
        assert_eq!(
            outline_state(true, true, true, true, true, true, true),
            OutlineState::None
        );
        assert_eq!(
            outline_state(false, false, true, true, true, true, true),
            OutlineState::None
        );
    }

    #[test]
    fn pulse_stays_between_trough_and_one() {
        for i in 0..100 {
            let a = pulse_alpha(i as f32 * 0.037);
            assert!((PULSE_MIN - 1e-4..=1.0 + 1e-4).contains(&a));
        }
    }

    #[test]
    fn selected_is_opaque_pale_gold_and_engaged_is_thin() {
        let livery = Color::srgb(0.1, 0.1, 0.6);
        let sel = state_color(OutlineState::Selected, livery, ENEMY_RED, true);
        assert_eq!(sel.alpha(), 1.0);
        let eng = state_color(OutlineState::Engaged, livery, ENEMY_RED, false);
        assert!(eng.alpha() < 0.7);
    }
}
