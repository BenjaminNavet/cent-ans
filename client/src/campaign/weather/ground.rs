//! Per-province weather for the ground shader (lot weather5): port of the weather mask,
//! the baked weather field and the selection clearing of `campaign_weather_view.gd`
//! (`mask_image`, `_bake_field`, `_update_selection_clear`) and `campaign_weather.gdshaderinc`
//! (`wx_sample`, `wx_selection_clear`).
//!
//! Godot reads the weather PER PROVINCE under every ground pixel (mask texel of the province id,
//! averaged over three noise-warped reads for a soft fringe) and keeps the selected province free
//! of mist. Bevy used one scalar taken at the aim point. This module bakes the same two textures
//! on the CPU:
//!   * `field` (RGBA8, map / [`FIELD_DOWNSAMPLE`]): R rain, G snow, B fog, A storm per map point,
//!     fringe included, rebaked when the turn's weather changes;
//!   * `clear` (R8, map / [`CLEAR_DOWNSAMPLE`]): 1 inside the selected province (feathered edge),
//!     rebaked when the selection changes. `1 - clear` is Godot's `wx_selection_clear`.
//!
//! The WGSL side is `ground.wesl` (`cent_ans_client::campaign::weather::ground`). The terrain
//! shader must bind both textures (see the integration note in `docs/wip/bm-weather.md`).

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde_json::{Value, json};

use super::model::Kind;
use super::{CampaignWeather, WeatherTuning};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::provinces::picker::SelectedProvince;

/// Map px per field texel.
pub const FIELD_DOWNSAMPLE: u32 = 8;
/// Map px per selection texel.
pub const CLEAR_DOWNSAMPLE: u32 = 4;
/// `weather_edge_px` of the Godot shader (fringe width between two provinces).
pub const EDGE_PX: f32 = 30.0;

/// Mask colour of a weather entry (`mask_image`): R rain, G snow, B fog, A storm.
pub fn mask_colour(kind: Kind, intensity: f32) -> [f32; 4] {
    let k = 0.45 + 0.55 * intensity.clamp(0.0, 1.0);
    match kind {
        Kind::Rain => [k, 0.0, 0.0, 0.0],
        Kind::Storm => [1.0, 0.0, 0.0, k],
        Kind::Snow => [0.0, k, 0.0, 0.0],
        Kind::Fog => [0.0, 0.0, k, 0.0],
        _ => [0.0; 4],
    }
}

fn hash(x: f32, y: f32) -> f32 {
    let h = (x * 127.1 + y * 311.7).sin() * 43_758.545;
    h - h.floor()
}

/// Value noise, smooth, in 0..1 (stands for `wx_fbm` at a low frequency).
fn noise(x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = hash(ix, iy);
    let b = hash(ix + 1.0, iy);
    let c = hash(ix, iy + 1.0);
    let d = hash(ix + 1.0, iy + 1.0);
    (a + (b - a) * ux) + ((c + (d - c) * ux) - (a + (b - a) * ux)) * uy
}

fn fbm(x: f32, y: f32) -> f32 {
    noise(x, y) * 0.65 + noise(x * 2.07 + 5.2, y * 2.07 + 1.3) * 0.35
}

/// Weather at map point `p` (`wx_sample`): three noise-warped province reads averaged.
pub fn sample(map: &ProvinceMap, masks: &[[f32; 4]], p: Vec2) -> [f32; 4] {
    let warp = Vec2::new(
        (fbm(p.x * 0.011, p.y * 0.011) - 0.5) * EDGE_PX * 2.0,
        (fbm(p.x * 0.011 + 7.3, p.y * 0.011 + 7.3) - 0.5) * EDGE_PX * 2.0,
    );
    let mut sum = [0.0_f32; 4];
    for i in 0..3 {
        let angle = i as f32 * 2.094 + 0.5;
        let q = p + warp + Vec2::new(angle.cos(), angle.sin()) * EDGE_PX * 0.8;
        let index = map.index_at(q.x, q.y) as usize;
        if index > 0
            && let Some(m) = masks.get(index)
        {
            for c in 0..4 {
                sum[c] += m[c];
            }
        }
    }
    sum.map(|v| v / 3.0)
}

/// 1 inside the selected province, 0 outside, edge feathered over `feather` map px (five reads,
/// `wx_selection_clear`; returns the "inside" share, the shader uses `1 - inside`).
pub fn selection_inside(map: &ProvinceMap, selected: u16, p: Vec2, feather: f32) -> f32 {
    if selected == 0 {
        return 0.0;
    }
    let at = |dx: f32, dy: f32| f32::from(map.index_at(p.x + dx, p.y + dy) == selected);
    at(0.0, 0.0) * 0.4
        + (at(feather, 0.0) + at(-feather, 0.0) + at(0.0, feather) + at(0.0, -feather)) * 0.15
}

/// Baked textures and their bookkeeping.
#[derive(Resource, Default)]
pub struct WeatherGround {
    pub field: Handle<Image>,
    pub clear: Handle<Image>,
    pub field_size: UVec2,
    pub clear_size: UVec2,
    /// `CampaignWeather::revision` the field was baked for.
    pub field_revision: Option<u64>,
    pub selected: Option<u16>,
    /// True when any province has weather (`weather_enabled`).
    pub any: bool,
    pub field_bakes: u32,
    pub clear_bakes: u32,
    /// Texels of the field with weather / texels inside the selection (dump).
    pub field_covered: u32,
    pub clear_covered: u32,
}

fn make_image(size: UVec2, format: TextureFormat, data: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

fn bake_field_data(map: &ProvinceMap, masks: &[[f32; 4]], size: UVec2) -> (Vec<u8>, u32) {
    let mut data = vec![0u8; (size.x * size.y * 4) as usize];
    let mut covered = 0;
    for y in 0..size.y {
        for x in 0..size.x {
            let p = Vec2::new(
                (x as f32 + 0.5) * FIELD_DOWNSAMPLE as f32,
                (y as f32 + 0.5) * FIELD_DOWNSAMPLE as f32,
            );
            let w = sample(map, masks, p);
            let at = ((y * size.x + x) * 4) as usize;
            for c in 0..4 {
                data[at + c] = (w[c].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            if w.iter().any(|v| *v > 0.0) {
                covered += 1;
            }
        }
    }
    (data, covered)
}

fn bake_clear_data(map: &ProvinceMap, selected: u16, size: UVec2, feather: f32) -> (Vec<u8>, u32) {
    let mut data = vec![0u8; (size.x * size.y) as usize];
    let mut covered = 0;
    if selected == 0 {
        return (data, 0);
    }
    for y in 0..size.y {
        for x in 0..size.x {
            let p = Vec2::new(
                (x as f32 + 0.5) * CLEAR_DOWNSAMPLE as f32,
                (y as f32 + 0.5) * CLEAR_DOWNSAMPLE as f32,
            );
            let inside = selection_inside(map, selected, p, feather);
            if inside > 0.0 {
                covered += 1;
            }
            data[(y * size.x + x) as usize] = (inside.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    (data, covered)
}

fn masks_of(weather: &CampaignWeather, map: &ProvinceMap) -> (Vec<[f32; 4]>, bool) {
    let mut masks = vec![[0.0_f32; 4]; map.max_index() as usize + 1];
    let mut any = false;
    for feature in map.provinces() {
        let entry = weather.entry(&feature.id);
        let colour = mask_colour(entry.kind, entry.intensity);
        any |= entry.kind != Kind::Clear;
        if let Some(slot) = masks.get_mut(feature.index as usize) {
            *slot = colour;
        }
    }
    (masks, any)
}

fn bake_field(
    weather: Res<CampaignWeather>,
    map: Option<Res<ProvinceMap>>,
    mut ground: ResMut<WeatherGround>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(map) = map else { return };
    if ground.field_revision == Some(weather.revision) || weather.by_province.is_empty() {
        return;
    }
    let size = UVec2::new(
        map.width.div_ceil(FIELD_DOWNSAMPLE),
        map.height.div_ceil(FIELD_DOWNSAMPLE),
    );
    let (masks, any) = masks_of(&weather, &map);
    let (data, covered) = bake_field_data(&map, &masks, size);
    let image = make_image(size, TextureFormat::Rgba8Unorm, data);
    if ground.field == Handle::default() {
        ground.field = images.add(image);
    } else if let Some(mut existing) = images.get_mut(&ground.field) {
        *existing = image;
    }
    ground.field_size = size;
    ground.field_revision = Some(weather.revision);
    ground.any = any;
    ground.field_bakes += 1;
    ground.field_covered = covered;
}

fn bake_clear(
    selected: Option<Res<SelectedProvince>>,
    tuning: Res<WeatherTuning>,
    map: Option<Res<ProvinceMap>>,
    mut ground: ResMut<WeatherGround>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(map) = map else { return };
    let index = selected.map_or(0, |s| s.index);
    if ground.selected == Some(index) {
        return;
    }
    let size = UVec2::new(
        map.width.div_ceil(CLEAR_DOWNSAMPLE),
        map.height.div_ceil(CLEAR_DOWNSAMPLE),
    );
    let (data, covered) = bake_clear_data(&map, index, size, tuning.clouds.clear_feather_px);
    let image = make_image(size, TextureFormat::R8Unorm, data);
    if ground.clear == Handle::default() {
        ground.clear = images.add(image);
    } else if let Some(mut existing) = images.get_mut(&ground.clear) {
        *existing = image;
    }
    ground.clear_size = size;
    ground.selected = Some(index);
    ground.clear_bakes += 1;
    ground.clear_covered = covered;
}

fn reset(mut ground: ResMut<WeatherGround>) {
    ground.field_revision = None;
    ground.selected = None;
}

fn dump(world: &World) -> (String, Value) {
    let g = world.resource::<WeatherGround>();
    (
        "weather_ground".to_owned(),
        json!({
            "field_size": [g.field_size.x, g.field_size.y],
            "clear_size": [g.clear_size.x, g.clear_size.y],
            "field_revision": g.field_revision,
            "field_bakes": g.field_bakes,
            "field_covered_texels": g.field_covered,
            "any_weather": g.any,
            "selected": g.selected,
            "clear_bakes": g.clear_bakes,
            "clear_covered_texels": g.clear_covered,
        }),
    )
}

pub(super) fn build(app: &mut App) {
    bevy::shader::load_shader_library!(app, "ground.wesl");
    app.init_resource::<WeatherGround>()
        .add_systems(OnEnter(AppState::Campaign), reset)
        .add_systems(
            Update,
            (bake_field.after(super::refresh_weather), bake_clear)
                .run_if(in_state(AppState::Campaign)),
        );
    app.register_dump(dump);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_matches_godot_colours() {
        assert_eq!(mask_colour(Kind::Clear, 1.0), [0.0; 4]);
        let rain = mask_colour(Kind::Rain, 0.0);
        assert!((rain[0] - 0.45).abs() < 1e-6 && rain[3] == 0.0);
        let storm = mask_colour(Kind::Storm, 1.0);
        assert_eq!((storm[0], storm[3]), (1.0, 1.0));
        assert_eq!(mask_colour(Kind::Fog, 1.0)[2], 1.0);
        assert_eq!(mask_colour(Kind::Snow, 1.0)[1], 1.0);
    }

    fn real_map() -> &'static ProvinceMap {
        static MAP: std::sync::OnceLock<ProvinceMap> = std::sync::OnceLock::new();
        MAP.get_or_init(|| {
            let root =
                crate::paths::find_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
                    .expect("repository root");
            ProvinceMap::load(&root.join("data/map")).expect("map data")
        })
    }

    #[test]
    fn weather_is_read_per_province_with_a_fringe() {
        let map = real_map();
        let paris = map.index_of_id("prov_ile_de_france");
        let mut masks = vec![[0.0_f32; 4]; map.max_index() as usize + 1];
        masks[paris as usize] = mask_colour(Kind::Fog, 1.0);
        let capital = map.province(paris).unwrap().capital_px;
        let inside = sample(map, &masks, Vec2::new(capital[0], capital[1]));
        assert!(inside[2] > 0.3 && inside[0] == 0.0, "{inside:?}");
        // Far from Ile-de-France (Moscow side of the map): nothing.
        let far = sample(map, &masks, Vec2::new(6500.0, 1500.0));
        assert_eq!(far, [0.0; 4]);
    }

    #[test]
    fn selected_province_is_inside_and_neighbours_are_not() {
        let map = real_map();
        let paris = map.index_of_id("prov_ile_de_france");
        let c = map.province(paris).unwrap().capital_px;
        let p = Vec2::new(c[0], c[1]);
        let inside = selection_inside(map, paris, p, 8.0);
        assert!(inside >= 0.4, "{inside}");
        assert_eq!(
            selection_inside(map, paris, Vec2::new(6500.0, 1500.0), 8.0),
            0.0
        );
        assert_eq!(selection_inside(map, 0, p, 8.0), 0.0);
    }

    #[test]
    fn noise_is_bounded() {
        for i in 0..200 {
            let v = fbm(i as f32 * 0.37, i as f32 * 0.91);
            assert!((0.0..=1.0).contains(&v), "{v}");
        }
    }
}
