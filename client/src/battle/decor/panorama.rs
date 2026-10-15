//! Painted horizon panorama of a battle (port of `_build_panorama` / `_choose_panorama` of
//! game/scripts/battle/battle_horizon.gd and of game/shaders/battle_panorama.gdshader).
//!
//! The panorama is chosen by the first rule of `data/fx/horizon.json` (`rules`: provinces,
//! season, terrain, coastal, skyline bounds) that fits the horizon tile, painted from
//! `game/assets/horizon/panoramas/<id>.webp` (`panoramas.json` gives each painting's crest per
//! column, `skyline`). It replaces the plain ridge cylinder of `horizon.rs` (kept when no painting
//! fits). Deviation: the cylinder radius is the horizon look's `skyline_radius_m` (the battle
//! camera's far plane is 14 km, Godot's band sits at 14.6 km); the panorama being angular, the
//! picture is the same. The skyline texture is sampled per world bearing (the tile rotation is
//! baked in on the CPU).

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use sim_battle::{BattleSeason, Weather};

use super::horizon::RealRelief;
use super::look::DecorLook;

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct PanoramaParams {
    pub paint: Vec4,
    pub frame: Vec4,
    pub crest: Vec4,
    pub haze: Vec4,
    pub fog_color: Vec4,
    pub light_tint: Vec4,
    pub sea_color: Vec4,
    pub land_fallback: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PanoramaMaterial {
    #[uniform(0)]
    pub params: PanoramaParams,
    #[texture(1)]
    #[sampler(2)]
    pub painting: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub skyline: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub paint_skyline: Handle<Image>,
}

impl Material for PanoramaMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/panorama.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        // Discarded above the crest: drawn after the opaque world and the skybox, no depth.
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

pub struct PanoramaPlugin;

impl Plugin for PanoramaPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "panorama.wesl");
        app.add_plugins(MaterialPlugin::<PanoramaMaterial>::default());
    }
}

fn season_key(season: BattleSeason) -> &'static str {
    match season {
        BattleSeason::Spring => "spring",
        BattleSeason::Summer => "summer",
        BattleSeason::Autumn => "autumn",
        BattleSeason::Winter => "winter",
    }
}

fn weather_key(weather: Weather) -> &'static str {
    match weather {
        Weather::Clear => "clear",
        Weather::Fog => "fog",
        Weather::Rain => "rain",
        Weather::Snow => "snow",
    }
}

/// First panorama whose rule fits (`_choose_panorama`); `entry` = the tile's `index.json` entry.
/// `CENT_ANS_PANORAMA` forces one.
pub fn choose(
    horizon: &serde_json::Value,
    province: &str,
    entry: &serde_json::Value,
    season: BattleSeason,
    terrain: &str,
) -> Option<String> {
    if let Ok(forced) = std::env::var("CENT_ANS_PANORAMA") {
        return Some(forced);
    }
    let skyline_max = entry["skyline_max_deg"].as_f64().unwrap_or(0.0);
    let has = |list: &serde_json::Value, key: &str| {
        list.as_array()
            .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(key)))
    };
    for rule in horizon["rules"].as_array()? {
        if rule.get("provinces").is_some() && !has(&rule["provinces"], province) {
            continue;
        }
        if rule.get("season").is_some() && !has(&rule["season"], season_key(season)) {
            continue;
        }
        if rule.get("terrain").is_some() && !has(&rule["terrain"], terrain) {
            continue;
        }
        if let Some(coastal) = rule["coastal"].as_bool()
            && coastal != entry["coastal"].as_bool().unwrap_or(false)
        {
            continue;
        }
        if rule["skyline_max_deg"]
            .as_f64()
            .is_some_and(|m| skyline_max > m)
        {
            continue;
        }
        if rule["skyline_min_deg"]
            .as_f64()
            .is_some_and(|m| skyline_max < m)
        {
            continue;
        }
        return rule["panorama"].as_str().map(str::to_owned);
    }
    None
}

fn float_row(values: &[[f32; 4]]) -> Image {
    let mut bytes = Vec::with_capacity(values.len() * 16);
    for v in values {
        for c in v {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: values.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    // Rgba32Float is not filterable everywhere: nearest, repeating.
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..ImageSamplerDescriptor::nearest()
    });
    image
}

fn num(value: &serde_json::Value, default: f32) -> f32 {
    value.as_f64().map_or(default, |v| v as f32)
}

/// Everything needed to build the panorama.
pub struct PanoramaInput<'a> {
    pub relief: &'a RealRelief,
    pub province: &'a str,
    pub season: BattleSeason,
    pub terrain: &'a str,
    pub weather: Weather,
    pub seed_hash: f32,
}

/// Mesh and material of the painted panorama, or `None` (no rule, no painting, no metadata).
pub fn build(
    input: &PanoramaInput,
    look: &DecorLook,
    assets: &AssetServer,
    images: &mut Assets<Image>,
) -> Option<(Mesh, PanoramaMaterial, String)> {
    let root = crate::paths::find_root().ok()?;
    let read = |path: std::path::PathBuf| -> Option<serde_json::Value> {
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    };
    let horizon = read(crate::paths::data_dir(&root).join("fx/horizon.json"))?;
    let asset_root = crate::paths::asset_root(&root);
    let index = read(asset_root.join("horizon/relief/index.json"))?;
    let meta = read(asset_root.join("horizon/panoramas/panoramas.json"))?;
    let entry = &index["provinces"][input.province];
    let id = choose(&horizon, input.province, entry, input.season, input.terrain)?;
    let paint = &horizon["panoramas"][id.as_str()];
    let path = paint["path"].as_str()?.trim_start_matches("res://assets/");
    if !asset_root.join(path).exists() {
        return None;
    }
    let crests: Vec<f32> = meta["panoramas"][id.as_str()]["skyline"]
        .as_array()?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.2) as f32)
        .collect();
    if crests.is_empty() {
        return None;
    }
    let cfg = &horizon["panorama"];
    let weather = weather_key(input.weather);
    let radius = look.horizon.skyline_radius_m;
    let bottom = num(&cfg["bottom_deg"], -1.2);
    let top = num(&cfg["max_angle_deg"], 7.5) * 1.6 + 1.0;
    let relief = input.relief;
    let ref_y = relief.offset_y + relief.ref_m.max(0.0) + 25.0;
    // Skyline per world bearing.
    let samples = relief.skyline.len().max(64);
    let skyline: Vec<[f32; 4]> = (0..samples)
        .map(|k| {
            let bearing = (k as f32 + 0.5) / samples as f32;
            let (angle, dist) = relief.skyline_at(bearing);
            [angle, dist / 1000.0, relief.sea_share_at(bearing), 1.0]
        })
        .collect();
    let paint_row: Vec<[f32; 4]> = crests.iter().map(|c| [*c, 0.0, 0.0, 1.0]).collect();
    let mean_crest = crests.iter().sum::<f32>() / crests.len() as f32;
    let fog = look.fog_of(input.weather);
    let fog_linear = Color::srgb(fog.color[0], fog.color[1], fog.color[2]).to_linear();
    let sea = look.horizon.sea_color;
    let sea_linear = Color::srgb(sea[0], sea[1], sea[2]).to_linear();
    let params = PanoramaParams {
        paint: Vec4::new(
            num(&paint["repeats"], 4.0),
            input.seed_hash,
            mean_crest,
            num(&cfg["paint_detail"], 0.3),
        ),
        frame: Vec4::new(ref_y, radius, bottom, 1.0),
        crest: Vec4::new(
            num(&cfg["min_angle_deg"], 0.55),
            num(&cfg["max_angle_deg"], 7.5),
            num(&cfg["exaggeration"], 1.35),
            num(&cfg["sea_share"], 0.55),
        ),
        haze: Vec4::new(
            num(&cfg["haze"][weather], 0.6),
            num(&cfg["exposure"][weather], 1.0),
            fog.horizon_density,
            0.0,
        ),
        fog_color: Vec4::new(fog_linear.red, fog_linear.green, fog_linear.blue, 1.0),
        light_tint: Vec4::ONE,
        sea_color: Vec4::new(sea_linear.red, sea_linear.green, sea_linear.blue, 1.0),
        land_fallback: Vec4::new(0.073, 0.095, 0.055, 1.0),
    };
    let material = PanoramaMaterial {
        params,
        painting: assets.load(path.to_owned()),
        skyline: images.add(float_row(&skyline)),
        paint_skyline: images.add(float_row(&paint_row)),
    };
    // Cylinder: buried bottom, the bottom of the band, above the highest crest.
    let segments = 128usize;
    let ys = [
        ref_y - radius * 0.2,
        ref_y + radius * bottom.to_radians().tan(),
        ref_y + radius * top.to_radians().tan(),
    ];
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for k in 0..=segments {
        let a = std::f32::consts::TAU * k as f32 / segments as f32;
        for y in ys {
            positions.push([a.sin() * radius, y, -a.cos() * radius]);
        }
    }
    for k in 0..segments {
        for r in 0..ys.len() - 1 {
            let i = (k * ys.len() + r) as u32;
            let j = i + ys.len() as u32;
            indices.extend_from_slice(&[i, j, i + 1, i + 1, j, j + 1]);
        }
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let uvs = vec![[0.0, 0.0]; positions.len()];
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices));
    Some((mesh, material, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_pick_a_panorama_for_a_plain_province_and_winter() {
        let root = crate::paths::find_root().unwrap();
        let horizon: serde_json::Value = serde_json::from_slice(
            &std::fs::read(crate::paths::data_dir(&root).join("fx/horizon.json")).unwrap(),
        )
        .unwrap();
        let flat = serde_json::json!({"coastal": false, "skyline_max_deg": 0.2});
        assert_eq!(
            choose(&horizon, "prov_x", &flat, BattleSeason::Winter, "plains").as_deref(),
            Some("winter_lowlands")
        );
        assert_eq!(
            choose(
                &horizon,
                "prov_flandre",
                &flat,
                BattleSeason::Summer,
                "plains"
            )
            .as_deref(),
            Some("flanders_flat")
        );
    }
}
