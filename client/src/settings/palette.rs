//! Accessibility palette: the `access/colorblind` and `access/high_contrast` settings as a
//! resource other plugins read (port of `ui/accessibility.gd` and `battle/battle_access.gd`).
//!
//! Consumers take `Option<Res<Palette>>` and call [`Palette::side_color`] /
//! [`Palette::enemy_red`]; with the flag off these return their input unchanged. High contrast
//! darkens the kit's ink and border colours (restored when switched off).

use bevy::prelude::*;

use super::Settings;
use crate::ui::kit::UiTheme;

/// Okabe-Ito blue used for the player's side in colour-blind mode.
pub const FRIEND: Color = Color::srgb(0.0, 0.447, 0.698);
/// Okabe-Ito vermilion used for enemies in colour-blind mode.
pub const ENEMY: Color = Color::srgb(0.835, 0.369, 0.0);

/// Darkening applied to ink and borders in high-contrast mode (`apply_contrast`, 0.55).
const CONTRAST_DARKEN: f32 = 0.55;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct Palette {
    pub colorblind: bool,
    pub high_contrast: bool,
}

impl Palette {
    /// Colour of a side: its own, or blue / vermilion in colour-blind mode.
    pub fn side_color(&self, color: Color, is_player: bool) -> Color {
        match (self.colorblind, is_player) {
            (false, _) => color,
            (true, true) => FRIEND,
            (true, false) => ENEMY,
        }
    }

    /// The "enemy red" of borders and targeting decals.
    pub fn enemy_red(&self, normal: Color) -> Color {
        if self.colorblind { ENEMY } else { normal }
    }
}

/// `enemy_red` for a consumer that may run without the plugin.
pub fn enemy_red(palette: &Option<Res<Palette>>, normal: Color) -> Color {
    palette.as_ref().map_or(normal, |p| p.enemy_red(normal))
}

/// `side_color` for a consumer that may run without the plugin.
pub fn side_color(palette: &Option<Res<Palette>>, color: Color, is_player: bool) -> Color {
    palette
        .as_ref()
        .map_or(color, |p| p.side_color(color, is_player))
}

/// Original kit colours, kept while high contrast is on.
#[derive(Default)]
pub(super) struct KitInk {
    saved: Option<(Color, Color)>,
}

pub(super) fn sync_palette(
    settings: Res<Settings>,
    mut palette: ResMut<Palette>,
    theme: Option<ResMut<UiTheme>>,
    mut ink: Local<KitInk>,
) {
    let wanted = Palette {
        colorblind: settings.colorblind,
        high_contrast: settings.high_contrast,
    };
    if *palette != wanted {
        *palette = wanted;
    }
    let Some(mut theme) = theme else { return };
    match (wanted.high_contrast, ink.saved) {
        (true, None) => {
            ink.saved = Some((theme.text, theme.border));
            theme.text = theme.text.mix(&Color::BLACK, CONTRAST_DARKEN);
            theme.border = theme.border.mix(&Color::BLACK, CONTRAST_DARKEN);
        }
        (false, Some((text, border))) => {
            theme.text = text;
            theme.border = border;
            ink.saved = None;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorblind_swaps_side_and_enemy_colours() {
        let red = Color::srgb(0.75, 0.12, 0.1);
        let off = Palette::default();
        assert_eq!(off.enemy_red(red), red);
        assert_eq!(off.side_color(red, true), red);
        let on = Palette {
            colorblind: true,
            ..default()
        };
        assert_eq!(on.enemy_red(red), ENEMY);
        assert_eq!(on.side_color(red, true), FRIEND);
        assert_eq!(on.side_color(red, false), ENEMY);
    }

    #[test]
    fn sync_follows_settings_and_restores_kit_ink() {
        let mut app = App::new();
        app.init_resource::<Palette>()
            .init_resource::<Settings>()
            .init_resource::<UiTheme>()
            .add_systems(Update, sync_palette);
        let base = app.world().resource::<UiTheme>().text;
        app.update();
        assert_eq!(*app.world().resource::<Palette>(), Palette::default());
        {
            let mut settings = app.world_mut().resource_mut::<Settings>();
            settings.colorblind = true;
            settings.high_contrast = true;
        }
        app.update();
        assert!(app.world().resource::<Palette>().colorblind);
        let dark = app.world().resource::<UiTheme>().text;
        assert!(dark.luminance() < base.luminance());
        app.update();
        assert_eq!(app.world().resource::<UiTheme>().text, dark);
        app.world_mut().resource_mut::<Settings>().high_contrast = false;
        app.update();
        assert_eq!(app.world().resource::<UiTheme>().text, base);
    }
}
