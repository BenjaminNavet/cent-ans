//! Zoom tiers of the campaign map and their cross-fade weights.
//!
//! Ports `game/scripts/map/zoom_tiers.gd` (values of `game/resources/zoom_tiers.tres`, ADR 0124 /
//! ADR 0036 / ZF). Thresholds apply to the rig distance (map units); every transition is a
//! smoothstep over a band centred on the threshold. [`ZoomView`] holds the weights of the current
//! frame for every layer that fades with the zoom; [`update_zoom_view`] also drives
//! [`WaterView`] (the camera lot owns it).

use bevy::prelude::*;
use serde_json::{Value, json};

use super::rig::CampaignRig;
use crate::campaign::water::WaterView;

/// Dominant tier at a camera distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Near,
    Valley,
    Site,
    Strategic,
}

impl Tier {
    pub fn name(self) -> &'static str {
        match self {
            Tier::Near => "near",
            Tier::Valley => "valley",
            Tier::Site => "site",
            Tier::Strategic => "strategic",
        }
    }
}

/// Thresholds and fade widths (`zoom_tiers.tres`).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ZoomTiers {
    pub strategic_threshold: f32,
    pub strategic_fade: f32,
    pub near_threshold: f32,
    pub near_fade: f32,
    pub fine_terrain_distance: f32,
    pub model_range: f32,
    pub hamlet_range: f32,
    pub valley_threshold: f32,
    pub valley_fade: f32,
    pub site_threshold: f32,
    pub site_fade: f32,
    pub site_border_alpha: f32,
    pub site_fog_alpha: f32,
    pub close_label_range_factor: f32,
}

impl Default for ZoomTiers {
    fn default() -> Self {
        Self {
            strategic_threshold: 1200.0,
            strategic_fade: 200.0,
            near_threshold: 150.0,
            near_fade: 40.0,
            fine_terrain_distance: 170.0,
            model_range: 1250.0,
            hamlet_range: 600.0,
            valley_threshold: 8.0,
            valley_fade: 3.0,
            site_threshold: 1.8,
            site_fade: 0.8,
            site_border_alpha: 0.2,
            site_fog_alpha: 0.35,
            close_label_range_factor: 40.0,
        }
    }
}

/// GLSL / GDScript `smoothstep`.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 == edge1 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl ZoomTiers {
    /// Reads the `[resource]` block of `zoom_tiers.tres` (`key = value` lines); missing keys keep
    /// the defaults.
    pub fn from_tres(text: &str) -> ZoomTiers {
        let mut tiers = ZoomTiers::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let Ok(number) = value.trim().parse::<f32>() else {
                continue;
            };
            match key.trim() {
                "strategic_threshold" => tiers.strategic_threshold = number,
                "strategic_fade" => tiers.strategic_fade = number,
                "near_threshold" => tiers.near_threshold = number,
                "near_fade" => tiers.near_fade = number,
                "fine_terrain_distance" => tiers.fine_terrain_distance = number,
                "model_range" => tiers.model_range = number,
                "hamlet_range" => tiers.hamlet_range = number,
                "valley_threshold" => tiers.valley_threshold = number,
                "valley_fade" => tiers.valley_fade = number,
                "site_threshold" => tiers.site_threshold = number,
                "site_fade" => tiers.site_fade = number,
                "site_border_alpha" => tiers.site_border_alpha = number,
                "site_fog_alpha" => tiers.site_fog_alpha = number,
                "close_label_range_factor" => tiers.close_label_range_factor = number,
                _ => {}
            }
        }
        tiers
    }

    pub fn tier_at(&self, distance: f32) -> Tier {
        if distance < self.site_threshold {
            Tier::Site
        } else if distance < self.valley_threshold {
            Tier::Valley
        } else if distance < self.strategic_threshold {
            Tier::Near
        } else {
            Tier::Strategic
        }
    }

    /// 0 in the normal view, 1 on the parchment.
    pub fn strategic_weight(&self, d: f32) -> f32 {
        smoothstep(
            self.strategic_threshold - self.strategic_fade * 0.5,
            self.strategic_threshold + self.strategic_fade * 0.5,
            d,
        )
    }

    /// Close detail (hamlets, every road, life effects): 1 well below the threshold.
    pub fn near_weight(&self, d: f32) -> f32 {
        1.0 - smoothstep(
            self.near_threshold - self.near_fade * 0.5,
            self.near_threshold + self.near_fade * 0.5,
            d,
        )
    }

    /// Valley tier and below (cumulative).
    pub fn valley_weight(&self, d: f32) -> f32 {
        1.0 - smoothstep(
            self.valley_threshold - self.valley_fade * 0.5,
            self.valley_threshold + self.valley_fade * 0.5,
            d,
        )
    }

    pub fn site_weight(&self, d: f32) -> f32 {
        1.0 - smoothstep(
            self.site_threshold - self.site_fade * 0.5,
            self.site_threshold + self.site_fade * 0.5,
            d,
        )
    }

    /// Border opacity (1 until the valley tier, `site_border_alpha` at the site tier).
    pub fn border_alpha(&self, d: f32) -> f32 {
        let inner = 0.5 + (self.site_border_alpha - 0.5) * self.site_weight(d);
        1.0 + (inner - 1.0) * self.valley_weight(d)
    }

    /// Opacity of the fog-of-war veil (same principle).
    pub fn fog_alpha(&self, d: f32) -> f32 {
        let inner = 0.7 + (self.site_fog_alpha - 0.7) * self.site_weight(d);
        1.0 + (inner - 1.0) * self.valley_weight(d)
    }
}

/// Weights of the current frame, derived from the rig distance.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ZoomView {
    pub distance: f32,
    pub tier: Tier,
    pub strategic: f32,
    pub near: f32,
    pub valley: f32,
    pub site: f32,
    pub border_alpha: f32,
    pub fog_alpha: f32,
}

impl Default for ZoomView {
    fn default() -> Self {
        ZoomView::at(&ZoomTiers::default(), 0.0)
    }
}

impl ZoomView {
    pub fn at(tiers: &ZoomTiers, distance: f32) -> ZoomView {
        ZoomView {
            distance,
            tier: tiers.tier_at(distance),
            strategic: tiers.strategic_weight(distance),
            near: tiers.near_weight(distance),
            valley: tiers.valley_weight(distance),
            site: tiers.site_weight(distance),
            border_alpha: tiers.border_alpha(distance),
            fog_alpha: tiers.fog_alpha(distance),
        }
    }
}

/// Loads `game/resources/zoom_tiers.tres` (defaults when absent).
pub fn load_tiers(root: &std::path::Path) -> ZoomTiers {
    std::fs::read_to_string(root.join("game/resources/zoom_tiers.tres"))
        .map(|text| ZoomTiers::from_tres(&text))
        .unwrap_or_default()
}

/// Recomputes the weights from the rig distance and pushes them to the layers the camera lot
/// drives (`campaign_map.gd::_process`: roads line weight, ribbon weight, lakes).
pub fn update_zoom_view(
    rig: Option<Res<CampaignRig>>,
    tiers: Res<ZoomTiers>,
    mut view: ResMut<ZoomView>,
    water: Option<ResMut<WaterView>>,
) {
    let Some(rig) = rig else {
        return;
    };
    let next = ZoomView::at(&tiers, rig.distance);
    if *view != next {
        *view = next;
    }
    if let Some(mut water) = water {
        let site_hide = 1.0 - next.site;
        let medium = (1.0 - next.strategic - next.near).clamp(0.0, 1.0);
        let near = next.near * site_hide;
        if water.medium != medium
            || water.near != near
            || water.strategic_weight != next.strategic
            || water.camera_distance != rig.distance
        {
            water.medium = medium;
            water.near = near;
            water.strategic_weight = next.strategic;
            water.camera_distance = rig.distance;
        }
    }
}

pub fn dump(view: &ZoomView) -> Value {
    json!({
        "distance": view.distance,
        "tier": view.tier.name(),
        "strategic": view.strategic,
        "near": view.near,
        "valley": view.valley,
        "site": view.site,
        "border_alpha": view.border_alpha,
        "fog_alpha": view.fog_alpha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_fade_across_the_bands() {
        let t = ZoomTiers::default();
        assert_eq!(t.strategic_weight(1000.0), 0.0);
        assert!((t.strategic_weight(1200.0) - 0.5).abs() < 1e-6);
        assert_eq!(t.strategic_weight(1400.0), 1.0);
        assert_eq!(t.near_weight(100.0), 1.0);
        assert_eq!(t.near_weight(200.0), 0.0);
        assert_eq!(t.tier_at(1.0), Tier::Site);
        assert_eq!(t.tier_at(5.0), Tier::Valley);
        assert_eq!(t.tier_at(300.0), Tier::Near);
        assert_eq!(t.tier_at(2000.0), Tier::Strategic);
        assert_eq!(t.border_alpha(500.0), 1.0);
        assert!((t.border_alpha(0.5) - 0.2).abs() < 1e-6);
    }

    #[test]
    fn tres_overrides_defaults() {
        let t = ZoomTiers::from_tres(
            "[resource]\nnear_threshold = 99.0\nscript = ExtResource(\"x\")\n",
        );
        assert_eq!(t.near_threshold, 99.0);
        assert_eq!(t.strategic_threshold, 1200.0);
    }
}
