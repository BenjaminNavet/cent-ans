//! Pointer feedback on the campaign map: the context cursor (attack, embark, forbidden; port of
//! `AttackCursor` / `CampaignCursor`) and the hover hint following the pointer (the Godot
//! `_set_hover_text` of `army_movement_controller.gd`).

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, SystemCursorIcon};

use super::orders::{PathPreviewState, SeaHover};
use super::views::ArmyViews;
use super::{HoveredArmy, SelectedArmy};
use crate::app::AppState;

const CURSOR_DIR: &str = "ui/cursors_campaign";
const HOTSPOT: (u16, u16) = (16, 16);

/// Campaign cursor in use (`CampaignCursor.CONTEXTS`, the ones this lot uses).
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMode {
    #[default]
    Normal,
    /// The selected army would attack the hovered enemy army.
    Attack,
    /// Right click would sail to a port.
    Embark,
    Forbidden,
}

impl CursorMode {
    pub fn name(self) -> &'static str {
        match self {
            CursorMode::Normal => "normal",
            CursorMode::Attack => "attack",
            CursorMode::Embark => "embark",
            CursorMode::Forbidden => "forbidden",
        }
    }
}

/// Mode for the current hover. `hostile_hovered`: an enemy army of the selected one is under
/// the pointer.
pub fn mode_for(hostile_hovered: bool, sea: SeaHover) -> CursorMode {
    if hostile_hovered {
        CursorMode::Attack
    } else {
        match sea {
            SeaHover::Port => CursorMode::Embark,
            SeaHover::Refused => CursorMode::Forbidden,
            SeaHover::None => CursorMode::Normal,
        }
    }
}

/// Text under the pointer.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct HoverHint(pub Option<String>);

#[derive(Component)]
pub struct HintNode;

#[derive(Component)]
pub struct HintText;

pub fn spawn_hint(mut commands: Commands) {
    commands
        .spawn((
            Name::new("ArmyHoverHint"),
            HintNode,
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                max_width: Val::Px(420.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.94, 0.89, 0.76, 0.96)),
            BorderColor::all(Color::srgb(0.30, 0.20, 0.12)),
            GlobalZIndex(40),
            Visibility::Hidden,
        ))
        .with_children(|node| {
            node.spawn((
                HintText,
                Text::new(""),
                TextFont {
                    font_size: bevy::text::FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.16, 0.10, 0.05)),
            ));
        });
}

pub fn update_hint(
    hint: Res<HoverHint>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut node: Query<(&mut Node, &mut Visibility), With<HintNode>>,
    mut text: Query<&mut Text, With<HintText>>,
) {
    let Ok((mut node, mut visibility)) = node.single_mut() else {
        return;
    };
    let cursor = window.iter().next().and_then(|w| w.cursor_position());
    match (&hint.0, cursor) {
        (Some(shown), Some(at)) => {
            if let Ok(mut text) = text.single_mut()
                && text.0 != *shown
            {
                text.0 = shown.clone();
            }
            node.left = Val::Px(at.x + 18.0);
            node.top = Val::Px(at.y + 22.0);
            *visibility = Visibility::Inherited;
        }
        _ => *visibility = Visibility::Hidden,
    }
}

/// Computes the cursor mode and applies it to the window when it changes.
#[allow(clippy::too_many_arguments)]
pub fn update_cursor(
    mut commands: Commands,
    assets: Res<AssetServer>,
    hovered: Res<HoveredArmy>,
    selected: Res<SelectedArmy>,
    views: Res<ArmyViews>,
    preview: Res<PathPreviewState>,
    mut mode: ResMut<CursorMode>,
    pointer: Res<super::pointer::PointerOverUi>,
    window: Query<Entity, With<PrimaryWindow>>,
) {
    let hostile = match (&selected.0, &hovered.0) {
        (Some(own), Some(other)) => own != other && views.get(other).is_some_and(|v| v.hostile),
        _ => false,
    };
    // Over a panel the cursor is the plain arrow.
    let wanted = if pointer.over {
        CursorMode::Normal
    } else {
        mode_for(hostile && selected.0.is_some(), preview.sea)
    };
    if *mode == wanted {
        return;
    }
    *mode = wanted;
    let Ok(entity) = window.single() else {
        return;
    };
    let icon = match wanted {
        CursorMode::Normal => CursorIcon::System(SystemCursorIcon::Default),
        other => CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle: assets.load(format!("{CURSOR_DIR}/{}.png", other.name())),
            hotspot: HOTSPOT,
            ..default()
        })),
    };
    commands.entity(entity).insert(icon);
}

/// Back to the arrow when the campaign screen closes.
pub fn reset_cursor(
    mut commands: Commands,
    mut mode: ResMut<CursorMode>,
    window: Query<Entity, With<PrimaryWindow>>,
) {
    if *mode != CursorMode::Normal
        && let Ok(entity) = window.single()
    {
        commands
            .entity(entity)
            .insert(CursorIcon::System(SystemCursorIcon::Default));
    }
    *mode = CursorMode::Normal;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_wins_over_sea() {
        assert_eq!(mode_for(true, SeaHover::Port), CursorMode::Attack);
        assert_eq!(mode_for(false, SeaHover::Port), CursorMode::Embark);
        assert_eq!(mode_for(false, SeaHover::Refused), CursorMode::Forbidden);
        assert_eq!(mode_for(false, SeaHover::None), CursorMode::Normal);
    }
}
