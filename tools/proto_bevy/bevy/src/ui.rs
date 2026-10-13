//! Parchment panel (top right) with a hover tooltip on its button, in `bevy_ui`.

use bevy::{picking::hover::Hovered, prelude::*};

use crate::scene::UiPanel;

const PARCHMENT: Color = Color::srgb(0.909, 0.851, 0.710); // #e8d9b5
const BROWN: Color = Color::srgb(0.420, 0.290, 0.169); // #6b4a2b
const INK: Color = Color::srgb(0.20, 0.13, 0.07);

#[derive(Component)]
pub struct PanelButton;

#[derive(Component)]
pub struct Tooltip;

fn text(content: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(content),
        TextFont {
            font: FontSource::families("Georgia, Times New Roman, serif"),
            font_size: size.into(),
            ..default()
        },
        TextColor(color),
    )
}

pub fn spawn_panel(commands: &mut Commands, panel: &UiPanel) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                right: Val::Px(16.0),
                width: Val::Px(300.0),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(3.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(PARCHMENT),
            BorderColor::all(BROWN),
        ))
        .with_children(|parent| {
            parent.spawn(text(&panel.title, 24.0, BROWN));
            parent.spawn(text(&panel.body, 16.0, INK));
            parent
                .spawn((
                    PanelButton,
                    Hovered::default(),
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(BROWN),
                ))
                .with_children(|button| {
                    button.spawn(text(&panel.button, 18.0, PARCHMENT));
                    button
                        .spawn((
                            Tooltip,
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Percent(110.0),
                                right: Val::Px(0.0),
                                width: Val::Px(260.0),
                                padding: UiRect::all(Val::Px(8.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.96, 0.93, 0.82)),
                            BorderColor::all(BROWN),
                            Visibility::Hidden,
                            GlobalZIndex(10),
                        ))
                        .with_children(|tip| {
                            tip.spawn(text(&panel.tooltip, 14.0, INK));
                        });
                });
        });
}

/// Shows the tooltip while the pointer hovers (or presses) the button.
pub fn update_tooltip(
    button: Query<&Hovered, (With<PanelButton>, Changed<Hovered>)>,
    mut tooltip: Query<&mut Visibility, With<Tooltip>>,
) {
    let Some(hovered) = button.iter().next() else {
        return;
    };
    for mut visibility in &mut tooltip {
        *visibility = if hovered.get() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
