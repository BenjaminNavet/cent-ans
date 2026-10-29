//! VIS-R2: the diplomatic stance in 3D reads as a gradient inner band along the edge of a
//! stance region, not as a flat wash over the whole territory (ADR "position diplomatique en
//! bande"). Pure parameters and weights here; `shaders/fill.wesl` mirrors `band_alpha`.
//! Tuning: `data/map/stance_fill.json` (`band`, `zoom.flat_from_distance`).

use bevy::prelude::*;
use serde_json::Value;

use super::borders::smoothstep;
use super::modes::{MapMode, ProvinceView, Tuning, fill_color, num};

/// Band parameters read from `stance_fill.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandParams {
    pub width_near: f32,
    pub width_mid: f32,
    pub near_distance: f32,
    pub mid_distance: f32,
    pub falloff: f32,
    pub flat_from: f32,
    pub flat_fade: f32,
}

impl Default for BandParams {
    fn default() -> Self {
        Self {
            width_near: 8.0,
            width_mid: 4.0,
            near_distance: 150.0,
            mid_distance: 500.0,
            falloff: 1.6,
            flat_from: 900.0,
            flat_fade: 300.0,
        }
    }
}

impl BandParams {
    pub fn from_tuning(fill: &Value) -> Self {
        let d = Self::default();
        Self {
            width_near: num(fill, &["band", "width_px", "near"], d.width_near),
            width_mid: num(fill, &["band", "width_px", "mid"], d.width_mid),
            near_distance: num(
                fill,
                &["band", "width_px", "near_distance"],
                d.near_distance,
            ),
            mid_distance: num(fill, &["band", "width_px", "mid_distance"], d.mid_distance),
            falloff: num(fill, &["band", "falloff"], d.falloff),
            flat_from: num(fill, &["zoom", "flat_from_distance"], d.flat_from),
            flat_fade: num(fill, &["zoom", "flat_fade_distance"], d.flat_fade).max(1.0),
        }
    }

    /// Band width in screen pixels at a camera distance (near width close, mid width further).
    pub fn width_px(&self, distance: f32) -> f32 {
        let t = smoothstep(
            self.near_distance,
            self.mid_distance.max(self.near_distance + 1.0),
            distance,
        );
        self.width_near + (self.width_mid - self.width_near) * t
    }

    /// Weight of the flat wash: 0 in the 3D view under `flat_from`, 1 from `flat_from + flat_fade`
    /// and whenever the parchment (strategic weight) is in force.
    pub fn flat_weight(&self, distance: f32, strategic: f32) -> f32 {
        smoothstep(self.flat_from, self.flat_from + self.flat_fade, distance)
            .max(strategic.clamp(0.0, 1.0))
    }
}

/// Band opacity at `distance_px` from the stance edge (Rust mirror of the shader): `edge_alpha`
/// at the edge, 0 from `width_px` inward.
pub fn band_alpha(distance_px: f32, width_px: f32, falloff: f32, edge_alpha: f32) -> f32 {
    let t = (distance_px / width_px.max(1e-3)).clamp(0.0, 1.0);
    edge_alpha * (1.0 - t).powf(falloff)
}

/// Band colours per raster index (stance colour, alpha = edge opacity of the stance), political
/// mode with the stance wash on only; transparent elsewhere (other modes paint their own fills).
pub fn province_band_colors(
    mode: MapMode,
    max_index: u16,
    view: &ProvinceView,
    tuning: &Tuning,
    stance_wash: bool,
) -> Vec<Srgba> {
    let mut colors = vec![Srgba::NONE; usize::from(max_index) + 1];
    if mode != MapMode::Political || !stance_wash {
        return colors;
    }
    let mode_alpha = num(&tuning.fill, &["modes", "political"], 1.0);
    for index in 1..=max_index {
        let controller = view.controller_of(index);
        if controller.is_empty() {
            continue;
        }
        let cue = view.cue_of(controller, &tuning.cues);
        let edge = num(&tuning.fill, &["band", "alpha", cue.key()], 0.0) * mode_alpha;
        let base = fill_color(cue, &tuning.fill, &tuning.cues);
        if edge > 0.0 && base != Srgba::NONE {
            colors[usize::from(index)] = base.with_alpha(edge);
        }
    }
    colors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn shipped() -> Value {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/map/stance_fill.json");
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn stance_fill_band_defaults_parse() {
        let fill = shipped();
        let band = BandParams::from_tuning(&fill);
        assert_eq!(band, BandParams::default(), "code defaults mirror the data");
        assert!((4.0..=8.0).contains(&band.width_mid) && (4.0..=8.0).contains(&band.width_near));
        assert_eq!(band.flat_from, 900.0);
        for key in ["self", "enemy", "friend", "other"] {
            assert!(
                num(&fill, &["band", "alpha", key], -1.0) > 0.0,
                "band.alpha.{key}"
            );
        }
        assert_eq!(
            BandParams::from_tuning(&json!({})),
            BandParams::default(),
            "missing data falls back to the defaults"
        );
    }

    #[test]
    fn band_alpha_is_zero_in_the_core_and_full_at_the_edge() {
        let (width, falloff, edge) = (6.0, 1.6, 0.5);
        assert_eq!(band_alpha(0.0, width, falloff, edge), edge);
        assert_eq!(band_alpha(width, width, falloff, edge), 0.0);
        assert_eq!(band_alpha(40.0, width, falloff, edge), 0.0);
        let mid = band_alpha(3.0, width, falloff, edge);
        assert!(
            mid > 0.0 && mid < edge,
            "gradient between edge and core: {mid}"
        );
    }

    #[test]
    fn flat_wash_only_beyond_flat_from_distance_or_on_parchment() {
        let band = BandParams::default();
        assert_eq!(band.flat_weight(300.0, 0.0), 0.0);
        assert_eq!(band.flat_weight(900.0, 0.0), 0.0);
        assert_eq!(band.flat_weight(1200.0, 0.0), 1.0);
        assert_eq!(
            band.flat_weight(300.0, 1.0),
            1.0,
            "strategic forces the wash"
        );
        assert_eq!(band.flat_weight(2600.0, 1.0), 1.0);
    }

    #[test]
    fn band_width_follows_the_zoom() {
        let band = BandParams::default();
        assert_eq!(band.width_px(60.0), band.width_near);
        assert_eq!(band.width_px(900.0), band.width_mid);
        let w = band.width_px(300.0);
        assert!(w < band.width_near && w > band.width_mid);
    }
}
