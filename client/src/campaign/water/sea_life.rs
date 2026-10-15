//! Living sea settings. Ports game/scripts/map/sea_life.gd (ME1): clear shallows with caustics,
//! offshore crest lines, bounded saturation and brightness, readable surf and tidal flats. The
//! numbers come from `data/fx/sea_life.json`; the look is in `shaders/sea.wesl`. Missing file:
//! `sea_life_on` stays 0 and the sea keeps the plain look.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde_json::Value;

use super::materials::SeaParams;

/// Tuning tree of `data/fx/sea_life.json`.
pub struct SeaLife {
    spec: Value,
}

fn number(tree: &Value, path: &[&str], default: f32) -> f32 {
    path.iter()
        .try_fold(tree, |node, key| node.get(key))
        .and_then(Value::as_f64)
        .map_or(default, |v| v as f32)
}

fn array<const N: usize>(tree: &Value, path: &[&str], default: [f32; N]) -> [f32; N] {
    let Some(list) = path
        .iter()
        .try_fold(tree, |node, key| node.get(key))
        .and_then(Value::as_array)
        .filter(|l| l.len() >= N)
    else {
        return default;
    };
    let mut out = default;
    for (slot, v) in out.iter_mut().zip(list) {
        *slot = v.as_f64().map_or(*slot, |x| x as f32);
    }
    out
}

impl SeaLife {
    /// Reads `<root>/data/fx/sea_life.json`; `None` when absent or invalid.
    pub fn load(root: &std::path::Path) -> Option<SeaLife> {
        let bytes = std::fs::read(root.join("data/fx/sea_life.json")).ok()?;
        let spec: Value = serde_json::from_slice(&bytes).ok()?;
        spec.get("shallows")?;
        Some(SeaLife { spec })
    }

    pub fn from_value(spec: Value) -> SeaLife {
        SeaLife { spec }
    }

    /// Cell size (map px) of the tide zone texture.
    pub fn cell_px(&self) -> u32 {
        number(&self.spec, &["cell_px"], 16.0).max(1.0) as u32
    }

    /// Writes the settings into the sea uniforms and switches the look on.
    pub fn apply(&self, params: &mut SeaParams) {
        let s = &self.spec;
        let v3 = |a: [f32; 3], w: f32| Vec4::new(a[0], a[1], a[2], w);
        let shallows = array(s, &["shallows", "color"], [0.2, 0.4, 0.38]);
        let crest = array(s, &["crests", "color"], [0.46, 0.55, 0.58]);
        let crest_depth = array(s, &["crests", "depth_m"], [40.0, 140.0]);
        let crest_dir = array(s, &["crests", "direction"], [0.8, 0.6]);
        let mud = array(s, &["tide", "mud_color"], [0.13, 0.115, 0.085]);
        let wet = array(s, &["tide", "wet_color"], [0.22, 0.22, 0.19]);
        params.life_a = Vec4::new(
            1.0,
            number(s, &["color", "brightness"], 1.0),
            number(s, &["color", "saturation_max"], 1.0),
            number(s, &["surf", "min_screen_px"], 0.0),
        );
        params.shallows_a = v3(shallows, number(s, &["shallows", "depth_m"], 40.0));
        params.shallows_b = Vec4::new(
            number(s, &["shallows", "amount"], 0.0),
            number(s, &["shallows", "caustic"], 0.0),
            number(s, &["shallows", "caustic_scale"], 0.9),
            0.0,
        );
        params.crest_a = v3(crest, number(s, &["crests", "amount"], 0.0));
        params.crest_b = Vec4::new(
            crest_depth[0],
            crest_depth[1],
            number(s, &["crests", "frequency"], 2.4),
            number(s, &["crests", "speed"], 0.55),
        );
        params.crest_dir = Vec4::new(crest_dir[0], crest_dir[1], 0.0, 0.0);
        params.tide_a = Vec4::new(
            number(s, &["tide", "period_s"], 150.0),
            number(s, &["tide", "max_px"], 8.0),
            number(s, &["tide", "high_cover"], 0.9),
            number(s, &["tide", "mud_alpha"], 0.96),
        );
        params.tide_mud = v3(mud, 0.0);
        params.tide_wet = v3(wet, 0.0);
    }

    /// `(id, width_px, polygon)` of the tide zones, in file order.
    pub fn tide_zones(&self) -> Vec<(String, f32, Vec<Vec2>)> {
        let Some(zones) = self.spec["tide"]["zones"].as_array() else {
            return Vec::new();
        };
        zones
            .iter()
            .filter_map(|zone| {
                let polygon: Vec<Vec2> = zone["polygon_px"]
                    .as_array()?
                    .iter()
                    .filter_map(|p| Some(Vec2::new(p[0].as_f64()? as f32, p[1].as_f64()? as f32)))
                    .collect();
                Some((
                    zone["id"].as_str()?.to_owned(),
                    zone["width_px"].as_f64()? as f32,
                    polygon,
                ))
            })
            .collect()
    }

    /// R8 grid, one texel per `cell_px`: R = zone mud width / `tide.max_px` (the last zone that
    /// contains the cell centre wins).
    pub fn tide_pixels(&self, map_size: Vec2) -> (u32, u32, Vec<u8>) {
        let cell = self.cell_px();
        let (w, h) = (
            (map_size.x / cell as f32).ceil() as u32,
            (map_size.y / cell as f32).ceil() as u32,
        );
        let max_px = number(&self.spec, &["tide", "max_px"], 8.0).max(0.001);
        let mut pixels = vec![0u8; (w * h) as usize];
        for (_, width, polygon) in self.tide_zones() {
            let value = ((width / max_px).clamp(0.0, 1.0) * 255.0).round() as u8;
            for y in 0..h {
                for x in 0..w {
                    let centre = Vec2::new(
                        (x as f32 + 0.5) * cell as f32,
                        (y as f32 + 0.5) * cell as f32,
                    );
                    if inside(centre, &polygon) {
                        pixels[(y * w + x) as usize] = value;
                    }
                }
            }
        }
        (w, h, pixels)
    }

    pub fn tide_image(&self, map_size: Vec2) -> Image {
        let (w, h, pixels) = self.tide_pixels(map_size);
        let mut image = Image::new(
            Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::R8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::linear();
        image
    }
}

/// Even-odd point in polygon.
fn inside(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut hit = false;
    let n = polygon.len();
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[(i + 1) % n]);
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            hit = !hit;
        }
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> SeaLife {
        let root =
            crate::paths::find_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        SeaLife::load(&root).expect("data/fx/sea_life.json")
    }

    #[test]
    fn settings_reach_the_uniforms() {
        let life = real();
        let mut params = SeaParams::new(Vec2::new(7080.0, 6104.0), &Default::default());
        assert_eq!(params.life_a.x, 0.0);
        life.apply(&mut params);
        assert_eq!(params.life_a.x, 1.0);
        assert!((params.life_a.y - 1.45).abs() < 1e-5, "brightness");
        assert!((params.life_a.z - 0.4).abs() < 1e-5, "saturation max");
        assert!((params.shallows_a.w - 60.0).abs() < 1e-5);
        assert!((params.crest_b.x - 40.0).abs() < 1e-5 && (params.crest_b.y - 140.0).abs() < 1e-5);
        assert!((params.tide_a.x - 150.0).abs() < 1e-5);
    }

    #[test]
    fn tide_zones_are_rasterised() {
        let life = real();
        assert_eq!(life.tide_zones().len(), 8);
        let (w, h, pixels) = life.tide_pixels(Vec2::new(7080.0, 6104.0));
        assert_eq!((w, h), (443, 382));
        let lit = pixels.iter().filter(|p| **p > 0).count();
        assert!(lit > 50, "{lit} tidal cells");
        // Mont-Saint-Michel (1778..1850, 3160..3198): width 6 / 8 px.
        let at = (3180 / 16) * w + 1810 / 16;
        assert_eq!(pixels[at as usize], (6.0f32 / 8.0 * 255.0).round() as u8);
    }
}
