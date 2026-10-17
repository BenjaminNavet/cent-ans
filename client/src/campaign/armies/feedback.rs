//! Pointer feedback on the campaign map: the context cursor (attack, embark, forbidden; port of
//! `AttackCursor` / `CampaignCursor`) and the hover hint following the pointer (the Godot
//! `_set_hover_text` of `army_movement_controller.gd`).

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, SystemCursorIcon};

use super::SelectedArmy;
use super::orders::{PathPreviewState, SeaHover};
use crate::app::AppState;
use crate::ui::kit::UiTheme;
use crate::ui::kit::richtext::{RichStyle, rich_text};

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

pub fn spawn_hint(mut commands: Commands) {
    commands.spawn((
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
    ));
}

/// Text of the bubble: the rich description of the army or colony under the pointer, then the
/// order hint (attack, path, voyage) in italics.
pub fn compose_hint(rich: Option<&str>, hint: Option<&str>) -> Option<String> {
    match (rich, hint) {
        (Some(rich), Some(hint)) => Some(format!("{rich}\n[i]{hint}[/i]")),
        (Some(rich), None) => Some(rich.to_owned()),
        (None, Some(hint)) => Some(hint.to_owned()),
        (None, None) => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_hint(
    mut commands: Commands,
    hint: Res<HoverHint>,
    rich: Res<super::bubbles::RichHover>,
    pointer: Res<super::pointer::PointerOverUi>,
    tooltips: Option<Res<crate::ui::kit::tooltip::TooltipState>>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut node: Query<(Entity, &mut Node, &mut Visibility), With<HintNode>>,
    mut shown: Local<Option<String>>,
) {
    let Ok((entity, mut node, mut visibility)) = node.single_mut() else {
        return;
    };
    let cursor = window.iter().next().and_then(|w| w.cursor_position());
    // One bubble at a time: a kit tooltip (decor, panel) or the interface under the pointer wins.
    let suppressed = pointer.over || tooltips.is_some_and(|t| !t.chain.is_empty());
    let wanted = if suppressed {
        None
    } else {
        compose_hint(rich.text.as_deref(), hint.0.as_deref())
    };
    match (&wanted, cursor) {
        (Some(text), Some(at)) => {
            if shown.as_ref() != Some(text) {
                commands.entity(entity).despawn_children();
                let text_bundle = rich_text(&theme, &assets, text, &RichStyle::caption(&theme));
                commands.entity(entity).with_children(|inner| {
                    inner.spawn(text_bundle);
                });
                *shown = Some(text.clone());
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
    selected: Res<SelectedArmy>,
    preview: Res<PathPreviewState>,
    mut mode: ResMut<CursorMode>,
    pointer: Res<super::pointer::PointerOverUi>,
    window: Query<Entity, With<PrimaryWindow>>,
) {
    // The attack cursor also covers armies and places of a faction at peace (declaration of war
    // asked first), as computed by `preview_under_cursor`.
    let hostile = preview.attack;
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
    fn hint_composition() {
        assert_eq!(compose_hint(None, None), None);
        assert_eq!(compose_hint(Some("a"), None).as_deref(), Some("a"));
        assert_eq!(compose_hint(None, Some("h")).as_deref(), Some("h"));
        assert_eq!(
            compose_hint(Some("a"), Some("h")).as_deref(),
            Some("a\n[i]h[/i]")
        );
    }

    #[test]
    fn attack_wins_over_sea() {
        assert_eq!(mode_for(true, SeaHover::Port), CursorMode::Attack);
        assert_eq!(mode_for(false, SeaHover::Port), CursorMode::Embark);
        assert_eq!(mode_for(false, SeaHover::Refused), CursorMode::Forbidden);
        assert_eq!(mode_for(false, SeaHover::None), CursorMode::Normal);
    }
}
