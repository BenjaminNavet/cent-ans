//! Procedural banner on top of the generated glb of a place (`_banner_mesh`, `_dn_banner_mat` of
//! `town_maquette_layer.gd`) with the wind wave of `maquette_banner.gdshader` (lot AS5): a
//! vertex stage (`banner.wesl`) over StandardMaterial, settings `maquette_banner` of
//! `data/fx/map_fire_wind.json`. One material per controller colour (shared by its places).

use std::collections::HashMap;
use std::path::Path;

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use serde_json::Value;

/// Width of the cloth of the unit banner (`banner_size` of the DN banner material).
pub const CLOTH_WIDTH: f32 = 0.55;
const CLOTH_HEIGHT: f32 = 0.38;
const CLOTH_COLS: usize = 6;
const CLOTH_ROWS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct BannerParams {
    /// Amplitude, banner size, wave speed, wave length.
    pub wave: Vec4,
    /// Gust gain, calm.
    pub gust: Vec4,
}

impl BannerParams {
    /// `maquette_banner` of `map_fire_wind.json` (Godot defaults when a key is missing; amplitude
    /// 0 when the section or the file is missing or `enabled` is false).
    pub fn from_json(doc: &Value) -> Self {
        let enabled = doc["enabled"].as_bool().unwrap_or(true);
        let w = &doc["maquette_banner"];
        let num = |k: &str, fallback: f32| w[k].as_f64().map_or(fallback, |x| x as f32);
        let amplitude = if enabled { num("amplitude", 0.0) } else { 0.0 };
        Self {
            wave: Vec4::new(
                amplitude,
                CLOTH_WIDTH,
                num("wave_speed", 5.0),
                num("wave_length", 0.9),
            ),
            gust: Vec4::new(num("gust_gain", 0.9), num("calm", 0.45), 0.0, 0.0),
        }
    }

    pub fn load(data_dir: &Path) -> Self {
        let doc = std::fs::read(data_dir.join("fx/map_fire_wind.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .unwrap_or(Value::Null);
        Self::from_json(&doc)
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct BannerExt {
    #[uniform(100)]
    pub params: BannerParams,
}

impl MaterialExtension for BannerExt {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/markers/banner.wesl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}

pub type BannerMaterial = ExtendedMaterial<StandardMaterial, BannerExt>;

/// Banner materials by controller colour.
#[derive(Resource, Default)]
pub struct BannerMaterials {
    pub params: Option<BannerParams>,
    by_colour: HashMap<[u8; 3], Handle<BannerMaterial>>,
}

impl BannerMaterials {
    /// Shared material of `colour` (flat, two-sided, rough like Godot's 0.85).
    pub fn get(
        &mut self,
        materials: &mut Assets<BannerMaterial>,
        colour: Srgba,
    ) -> Handle<BannerMaterial> {
        let key =
            [colour.red, colour.green, colour.blue].map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8);
        let params = self.params.unwrap_or(BannerParams {
            wave: Vec4::new(0.0, CLOTH_WIDTH, 5.0, 0.9),
            gust: Vec4::new(0.9, 0.45, 0.0, 0.0),
        });
        self.by_colour
            .entry(key)
            .or_insert_with(|| {
                materials.add(BannerMaterial {
                    base: StandardMaterial {
                        base_color: colour.into(),
                        perceptual_roughness: 0.85,
                        cull_mode: None,
                        double_sided: true,
                        ..default()
                    },
                    extension: BannerExt { params },
                })
            })
            .clone()
    }

    pub fn count(&self) -> usize {
        self.by_colour.len()
    }

    pub fn clear(&mut self) {
        self.by_colour.clear();
    }
}

/// Unit banner (`_banner_mesh`): pole of height 1 and a 0.55 x 0.38 cloth on a 6 x 3 grid (so
/// the wind wave bends it).
pub fn banner_mesh() -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let pole = 0.02;
    let corners = [
        Vec3::new(-pole, 0.0, -pole),
        Vec3::new(pole, 0.0, -pole),
        Vec3::new(pole, 0.0, pole),
        Vec3::new(-pole, 0.0, pole),
    ];
    for c in 0..4 {
        let a = corners[c];
        let b = corners[(c + 1) % 4];
        let up = Vec3::Y;
        let n = (a + b).normalize();
        for v in [a, b, b + up, a, b + up, a + up] {
            positions.push(v.into());
            normals.push(n.into());
        }
    }
    for r in 0..CLOTH_ROWS {
        for c in 0..CLOTH_COLS {
            let x0 = CLOTH_WIDTH * c as f32 / CLOTH_COLS as f32;
            let x1 = CLOTH_WIDTH * (c + 1) as f32 / CLOTH_COLS as f32;
            let y0 = 1.0 - CLOTH_HEIGHT * r as f32 / CLOTH_ROWS as f32;
            let y1 = 1.0 - CLOTH_HEIGHT * (r + 1) as f32 / CLOTH_ROWS as f32;
            for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y0), (x1, y1), (x0, y1)] {
                positions.push([x, y, 0.0]);
                normals.push([0.0, 0.0, 1.0]);
            }
        }
    }
    let count = positions.len() as u32;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32((0..count).collect()))
}

pub fn add(app: &mut App) {
    embedded_asset!(app, "banner.wesl");
    let params = crate::paths::find_root()
        .ok()
        .map(|root| BannerParams::load(&crate::paths::data_dir(&root)));
    app.add_plugins(MaterialPlugin::<BannerMaterial>::default())
        .insert_resource(BannerMaterials {
            params,
            ..default()
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn params_from_json_and_defaults() {
        let p = BannerParams::from_json(&json!({
            "maquette_banner": {"amplitude": 0.16, "wave_speed": 4.6, "wave_length": 0.4}
        }));
        assert_eq!(p.wave, Vec4::new(0.16, CLOTH_WIDTH, 4.6, 0.4));
        assert_eq!(p.gust.x, 0.9);
        assert_eq!(p.gust.y, 0.45);
        let off = BannerParams::from_json(
            &json!({"enabled": false, "maquette_banner": {"amplitude": 0.16}}),
        );
        assert_eq!(off.wave.x, 0.0);
        assert_eq!(BannerParams::from_json(&Value::Null).wave.x, 0.0);
    }

    #[test]
    fn real_data_has_a_wave() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let p = BannerParams::load(&crate::paths::data_dir(&root));
        assert!(p.wave.x > 0.0, "{p:?}");
    }

    #[test]
    fn cloth_is_a_grid() {
        let mesh = banner_mesh();
        // 4 pole sides + 6 x 3 cloth quads, two triangles each.
        assert_eq!(mesh.count_vertices(), (4 + CLOTH_COLS * CLOTH_ROWS) * 6);
    }
}
