//! Parchment UI kit (ADR 0357): fonts, theme, and bundle helpers every screen builds with.
//!
//! Will port game/scripts/ui/hud_style.gd (theme, colours, panel styles) and the shared
//! widgets of game/scripts/ui/ (buttons, panels, tooltips). Lot A extends it; keep the API small.
//!
//! - [`UiTheme`]: colours and the default font (EB Garamond, French accents).
//! - [`panel`], [`title`], [`label`], [`button`]: bundles; a button carries `Name::new(name)`.
//! - [`ButtonPressed`]: message sent when a kit button is pressed, by the mouse or by the agent
//!   script (`click <Name>`). Screens read it; they never poll `Interaction` themselves.
//! - [`UiCamera`]: the 2D camera that draws the UI (default UI camera, offscreen-aware).

use bevy::camera::RenderTarget;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;
use bevy::ui_widgets::{Activate, Button};

use crate::agent::OffscreenTarget;
use crate::paths;

/// Default UI font, relative to `game/assets`.
pub const FONT_PATH: &str = "third_party/fonts/eb_garamond/EBGaramond-VariableFont_wght.ttf";

const PARCHMENT: Color = Color::srgb(0.909, 0.851, 0.710); // #e8d9b5
const BORDER: Color = Color::srgb(0.420, 0.290, 0.169); // #6b4a2b
const INK: Color = Color::srgb(0.231, 0.165, 0.094); // #3b2a18

#[derive(Resource, Clone)]
pub struct UiTheme {
    pub background: Color,
    pub border: Color,
    pub text: Color,
    /// Button fill, and its fill under the pointer.
    pub button: Color,
    pub button_hover: Color,
    pub font: FontSource,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            background: PARCHMENT,
            border: BORDER,
            text: INK,
            button: BORDER,
            button_hover: Color::srgb(0.54, 0.38, 0.23),
            font: FontSource::default(),
        }
    }
}

/// A kit button was pressed (mouse or agent).
#[derive(Message, Debug, Clone)]
pub struct ButtonPressed {
    pub entity: Entity,
    /// The button's `Name`.
    pub name: String,
}

#[derive(Component)]
pub struct UiCamera;

pub struct UiKitPlugin;

impl Plugin for UiKitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiTheme>()
            .add_message::<ButtonPressed>()
            .add_systems(Startup, (load_font, spawn_ui_camera).chain())
            .add_observer(on_activate)
            .add_systems(Update, button_feedback);
    }
}

/// Loads EB Garamond from the repository: the built-in font is ASCII-only.
fn load_font(mut theme: ResMut<UiTheme>, mut fonts: ResMut<Assets<Font>>) {
    let path = paths::find_root().map(|root| paths::asset_root(&root).join(FONT_PATH));
    match path.and_then(|path| std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display())))
    {
        Ok(bytes) => theme.font = FontSource::Handle(fonts.add(Font::from_bytes(bytes))),
        Err(message) => warn!("{message}; using the default font"),
    }
}

fn spawn_ui_camera(mut commands: Commands, target: Option<Res<OffscreenTarget>>) {
    let mut camera = commands.spawn((
        Name::new("UiCamera"),
        UiCamera,
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.13, 0.10, 0.07)),
            ..default()
        },
    ));
    if let Some(target) = target {
        camera.insert(RenderTarget::Image(target.0.clone().into()));
    }
}

/// Tints the button under the pointer.
#[allow(clippy::type_complexity)]
fn button_feedback(
    theme: Res<UiTheme>,
    mut buttons: Query<(&Hovered, &mut BackgroundColor), (With<Button>, Changed<Hovered>)>,
) {
    for (hovered, mut fill) in &mut buttons {
        fill.0 = if hovered.get() {
            theme.button_hover
        } else {
            theme.button
        };
    }
}

/// A kit button was activated by the mouse (or keyboard): forwards it as [`ButtonPressed`].
fn on_activate(
    activate: On<Activate>,
    names: Query<&Name>,
    mut pressed: MessageWriter<ButtonPressed>,
) {
    if let Ok(name) = names.get(activate.entity) {
        pressed.write(ButtonPressed {
            entity: activate.entity,
            name: name.to_string(),
        });
    }
}

fn text(theme: &UiTheme, content: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(content),
        TextFont {
            font: theme.font.clone(),
            font_size: size.into(),
            ..default()
        },
        TextColor(color),
    )
}

/// Parchment panel: column container with a brown border. Add children with `with_children`
/// or `children![]`.
pub fn panel(theme: &UiTheme) -> impl Bundle {
    (
        Node {
            padding: UiRect::all(Val::Px(24.0)),
            border: UiRect::all(Val::Px(3.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(theme.background),
        BorderColor::all(theme.border),
    )
}

/// Large heading text.
pub fn title(theme: &UiTheme, content: &str) -> impl Bundle {
    text(theme, content, 48.0, theme.border)
}

/// Body text.
pub fn label(theme: &UiTheme, content: &str) -> impl Bundle {
    text(theme, content, 20.0, theme.text)
}

/// Button named `name` (also its caption); press it by mouse or `click <name>` in a script.
pub fn button(theme: &UiTheme, name: &str) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        Button,
        Hovered::default(),
        Node {
            min_width: Val::Px(240.0),
            padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(theme.button),
        children![text(theme, name, 24.0, theme.background)],
    )
}
