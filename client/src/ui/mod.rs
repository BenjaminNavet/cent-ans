//! UI toolbox and shared screens helpers.
//!
//! Besides the kit: [`fader`] (scene transitions, ex `scene_fader*.gd`), [`art_plates`]
//! (`art_plates.gd`), [`ink_glyph`] (`ink_glyph.gd`) and [`zones`] (rectangles of
//! `ui_layout.gd`). Not ported (Godot plumbing or covered elsewhere): `ui_build.gd` (node
//! factories, replaced by kit bundles), `game_catalog.gd` (definitions come from `game-api`
//! views / `data-model`), `build_hammer.gd` / `build_companion.gd` (belong to the Colonies
//! panel lot), `window_resize_grip.gd` (panel stack lot), `audio_buses.gd` (the audio plugin
//! owns the volumes), UI scale rules (`settings::auto_ui_scale`, 1.0 at 900 px, 0.9..1.6).

pub mod art_plates;
pub mod cursor;
pub mod fader;
pub mod ink_glyph;
pub mod kit;
pub mod zones;
