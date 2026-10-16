//! Accessibility of the kit: high-contrast panel style and live text scaling.
//!
//! Godot (`accessibility.gd::apply_contrast`) darkens the dark inks, lightens the light fills and
//! thickens every parchment border. The Bevy kit paints its panels from textures, so the border
//! is a dark outline around kit panels and cards, and flat fills (cards) get a lighter vellum.
//! The ink colours themselves are darkened by `settings::palette::sync_palette`.

use bevy::prelude::*;

use super::UiTheme;
use super::widgets::KitWidget;
use crate::settings::palette::Palette;

/// Outline width (px) of kit panels in high-contrast mode (Godot: borders + 1 px).
pub const OUTLINE_PX: f32 = 2.0;
/// Share of the cream fill mixed into light flat fills (`_contrast_box`, 0.45).
const LIGHTEN: f32 = 0.45;

/// Kit widgets that get the contrast frame.
pub fn is_framed(kind: &str) -> bool {
    matches!(
        kind,
        "panel" | "panel_illuminated" | "panel_inset" | "panel_note" | "card"
    )
}

/// Light flat fill made lighter and closer to cream (luminance > 0.6 only, like Godot).
pub fn contrast_fill(color: Color) -> Color {
    if color.luminance() > 0.6 {
        color.mix(&Color::srgb(1.0, 0.98, 0.92), LIGHTEN)
    } else {
        color
    }
}

/// Outline colour: the border ink pushed towards black.
pub fn contrast_outline(border: Color) -> Color {
    border.mix(&Color::BLACK, 0.5)
}

/// Original fill of a card, kept while it is lightened.
#[derive(Component, Debug)]
pub struct ContrastOriginal(pub Option<Color>);

/// Applies or removes the contrast frame on kit panels (on a mode change and for new widgets).
#[allow(clippy::type_complexity)]
pub fn apply_contrast_frames(
    palette: Option<Res<Palette>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
    widgets: Query<(
        Entity,
        &KitWidget,
        Option<&BackgroundColor>,
        Option<&ContrastOriginal>,
        Has<Outline>,
    )>,
    fresh: Query<(), Added<KitWidget>>,
    mut last: Local<Option<bool>>,
) {
    let on = palette.is_some_and(|p| p.high_contrast);
    let changed = *last != Some(on) && (on || last.is_some());
    *last = Some(on);
    if !changed && fresh.is_empty() {
        return;
    }
    for (entity, kind, fill, original, has_outline) in &widgets {
        if !is_framed(kind.0) {
            continue;
        }
        if on {
            if has_outline {
                continue;
            }
            let mut entity_commands = commands.entity(entity);
            entity_commands.try_insert(Outline::new(
                Val::Px(OUTLINE_PX),
                Val::ZERO,
                contrast_outline(theme.border),
            ));
            if let Some(fill) = fill
                && original.is_none()
            {
                entity_commands.try_insert((
                    ContrastOriginal(Some(fill.0)),
                    BackgroundColor(contrast_fill(fill.0)),
                ));
            }
        } else if has_outline || original.is_some() {
            let mut entity_commands = commands.entity(entity);
            entity_commands.try_remove::<Outline>();
            if let Some(ContrastOriginal(Some(color))) = original {
                entity_commands.try_insert(BackgroundColor(*color));
            }
            entity_commands.try_remove::<ContrastOriginal>();
        }
    }
}

/// Live text scale: a change of [`UiTheme::text_scale`] rescales the text already on screen
/// (text built later reads the new scale from the theme).
pub fn rescale_existing_text(
    theme: Res<UiTheme>,
    mut fonts: Query<&mut TextFont>,
    mut applied: Local<Option<f32>>,
) {
    // First run: adopt the scale in force (text on screen was built with it).
    let previous = applied.unwrap_or(theme.text_scale);
    *applied = Some(theme.text_scale);
    let ratio = theme.text_scale / previous;
    if (ratio - 1.0).abs() < 1e-4 {
        return;
    }
    for mut font in &mut fonts {
        if let bevy::text::FontSize::Px(size) = font.font_size {
            font.font_size = bevy::text::FontSize::Px(size * ratio);
        }
    }
}

/// Dump: scale in force, framed panels and cards.
pub fn dump(world: &World) -> serde_json::Value {
    let outlined = world
        .try_query_filtered::<&KitWidget, With<Outline>>()
        .map_or(0, |mut query| {
            query.iter(world).filter(|kind| is_framed(kind.0)).count()
        });
    let theme = world.resource::<UiTheme>();
    serde_json::json!({
        "text_scale": theme.text_scale,
        "framed_panels": outlined,
        "high_contrast": world.get_resource::<Palette>().is_some_and(|p| p.high_contrast),
    })
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            rescale_existing_text
                .run_if(resource_changed::<UiTheme>)
                .after(super::apply_text_scale),
            apply_contrast_frames,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(high: bool) -> App {
        let mut app = App::new();
        app.init_resource::<UiTheme>()
            .insert_resource(Palette {
                colorblind: false,
                high_contrast: high,
            })
            .add_systems(Update, apply_contrast_frames);
        app
    }

    #[test]
    fn frames_appear_and_disappear_with_the_mode() {
        let mut app = app(true);
        let card = app
            .world_mut()
            .spawn((KitWidget("card"), BackgroundColor(Color::srgb(0.97, 0.93, 0.82))))
            .id();
        let button = app.world_mut().spawn(KitWidget("button")).id();
        app.update();
        assert!(app.world().get::<Outline>(card).is_some());
        assert!(app.world().get::<Outline>(button).is_none());
        let lit = app.world().get::<BackgroundColor>(card).unwrap().0;
        assert!(lit.luminance() > Color::srgb(0.97, 0.93, 0.82).luminance());
        app.world_mut().resource_mut::<Palette>().high_contrast = false;
        app.update();
        assert!(app.world().get::<Outline>(card).is_none());
        assert_eq!(
            app.world().get::<BackgroundColor>(card).unwrap().0,
            Color::srgb(0.97, 0.93, 0.82)
        );
    }

    #[test]
    fn dark_fills_are_left_alone() {
        let dark = Color::srgb(0.2, 0.15, 0.1);
        assert_eq!(contrast_fill(dark), dark);
    }

    #[test]
    fn existing_text_follows_the_scale() {
        let mut app = App::new();
        app.init_resource::<UiTheme>()
            .add_systems(Update, rescale_existing_text);
        let text = app
            .world_mut()
            .spawn(TextFont {
                font_size: 20.0.into(),
                ..default()
            })
            .id();
        app.update();
        app.world_mut().resource_mut::<UiTheme>().text_scale = 1.5;
        app.update();
        let size = |app: &App| match app.world().get::<TextFont>(text).unwrap().font_size {
            bevy::text::FontSize::Px(px) => px,
            _ => 0.0,
        };
        assert!((size(&app) - 30.0).abs() < 1e-3);
        app.world_mut().resource_mut::<UiTheme>().text_scale = 1.0;
        app.update();
        assert!((size(&app) - 20.0).abs() < 1e-3);
    }
}
