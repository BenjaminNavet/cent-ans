//! Player settings (window mode, volumes, accessibility) and their persistence.
//!
//! Stub (V0). Will port: game/scripts/ui/settings.gd, settings_menu.gd, accessibility.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, _app: &mut App) {}
}
