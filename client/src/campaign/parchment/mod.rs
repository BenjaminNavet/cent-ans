//! Decoration of the strategic parchment view (camera > 1200, ADR 0124): portulan sea (wash,
//! coast water lines, rhumb lines, painted wind roses, double frame, burnt margins), sea
//! ornaments (painted sirens of the 1375 Catalan Atlas, drawn ships), ink town vignettes with
//! owner pennants, arched realm names, italic province names and heraldic army tokens. Fades
//! in and out with `ZoomView::strategic` (the only source of the two views).
//!
//! Replaces game/scripts/map/: parchment_decor.gd (ornament placement), parchment_overlay.gd
//! (2D layer), strategic_view.gd (weight, roses on the sea), map_fade.gd (fades; here every layer
//! reads `ZoomView`), and game/shaders/parchment_sea.gdshaderinc + parchment_common.gdshaderinc
//! (sea part). The land part of the parchment is `campaign/terrain` (`terrain.wesl`).

pub mod decor;
pub mod glyphs;
pub mod overlay;
pub mod realms;
pub mod sea;

use bevy::prelude::*;

/// Plugin of the parchment decoration.
pub struct CampaignParchmentPlugin;

impl Plugin for CampaignParchmentPlugin {
    fn build(&self, _app: &mut App) {}
}
