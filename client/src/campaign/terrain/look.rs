//! Look of the ground that depends on the game state: the fog-of-war veil (`FogOfWar::mask` as an
//! R8 texture, ports the `fog_*` uniforms of `terrain.gdshader`, data `data/ui/campaign_map.json`
//! `fog_of_war`), the parchment fade of the strategic view (`ZoomView::strategic`, ADR 0124) and
//! the wet ground / fresh snow of the weather under the camera focus. Everything is a uniform or
//! one small texture of [`TerrainMaterial`]; the shading lives in `terrain.wesl`.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, ShaderType, TextureDimension, TextureFormat};
use serde_json::Value;

use super::TerrainMaterialHandle;
use super::material::TerrainMaterial;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::weather::WeatherView;
use crate::campaign::weather::fog::FogOfWar;
use crate::campaign::weather::model::Kind;

/// Uniform block `LookParams` of `terrain.wesl`.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct LookParams {
    /// Fog of war: desaturation, dim, veil strength, 1 when a mask is bound.
    pub fog_a: Vec4,
    /// Veil colour (linear), jitter of the veil edge (map px).
    pub fog_veil: Vec4,
    /// Mist colour (linear), mist max opacity.
    pub fog_mist: Vec4,
    /// Mask extent in map px (xy), mist min opacity, mist drift scale (per map px).
    pub fog_map: Vec4,
    /// Wet ground, fresh snow, parchment weight, 0.
    pub weather: Vec4,
    /// Geomorph: 1 when on, vertex px of the quadtree, morph ratio, 0.
    pub morph: Vec4,
    /// RV-C fair-weather cumulus shadows (`campaign_cloud_shadow.gdshaderinc`, block `clouds`
    /// `cumulus_*` of `campaign_map.json`): strength now (0 = off), noise scale, edge, drift.
    pub cumulus_a: Vec4,
    /// Cover (low, high), height of the cloud plane above the ground, fade (cell px, full).
    pub cumulus_b: Vec4,
    /// Cool tint at the core (rgb), fade (cell px, none).
    pub cumulus_c: Vec4,
    /// CPU side: base strength, medium-range scale, medium band (camera distances).
    pub cumulus_d: Vec4,
    /// Morning mist of the "fog" weather (`campaign_weather.gdshaderinc`): fog weight under the
    /// focus (eased), morning share (1 at the start of the turn), wet dim (`clouds.wet_dim`),
    /// valley mist opacity (`morning_mist.opacity`, 0 = off).
    pub mist: Vec4,
    /// TB6 valley mist (`morning_mist` block): valley depth (m, from, full), plain altitude (m,
    /// full, none).
    pub valley_a: Vec4,
    /// Crest height (m, from, gone), plain share, bank scale (1 / map px).
    pub valley_b: Vec4,
    /// Drift (share of the wind), grazing gain, lifted share, near share.
    pub valley_c: Vec4,
    /// Valley mist colour (linear), 0.
    pub valley_color: Vec4,
}

impl Default for LookParams {
    fn default() -> Self {
        let veil = Color::srgb(0.74, 0.65, 0.48).to_linear();
        let mist = Color::srgb(0.80, 0.74, 0.62).to_linear();
        LookParams {
            fog_a: Vec4::new(0.62, 0.72, 0.10, 0.0),
            fog_veil: Vec4::new(veil.red, veil.green, veil.blue, 7.0),
            fog_mist: Vec4::new(mist.red, mist.green, mist.blue, 0.10),
            fog_map: Vec4::new(1.0, 1.0, 0.0, 0.030),
            weather: Vec4::ZERO,
            // `relief_lod::Config::default()`: max_vertex_px 4, morph_ratio 0.7.
            morph: Vec4::new(1.0, 4.0, 0.7, 0.0),
            cumulus_a: Vec4::new(0.0, 0.032, 0.10, 0.3),
            cumulus_b: Vec4::new(0.22, 0.5, 34.0, 12.0),
            cumulus_c: Vec4::new(0.94, 0.97, 1.06, 4.0),
            cumulus_d: Vec4::new(0.4, 0.3, 200.0, 500.0),
            // Shader defaults of `campaign_weather.gdshaderinc` (valley mist off without data).
            mist: Vec4::new(0.0, 1.0, 0.04, 0.0),
            valley_a: Vec4::new(10.0, 140.0, 40.0, 220.0),
            valley_b: Vec4::new(30.0, 150.0, 0.6, 0.006),
            valley_c: Vec4::new(0.5, 0.6, 0.35, 0.5),
            valley_color: Vec4::new(0.69, 0.67, 0.6, 0.0),
        }
    }
}

fn hex_color(text: &str) -> Option<Vec3> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let color = Color::srgb_u8(channel(0)?, channel(2)?, channel(4)?).to_linear();
    Some(Vec3::new(color.red, color.green, color.blue))
}

impl LookParams {
    /// Fog-of-war tuning from the text of `data/ui/campaign_map.json` (defaults on any error).
    pub fn from_map_json(text: &str) -> LookParams {
        let mut out = LookParams::default();
        let Ok(data) = serde_json::from_str::<Value>(text) else {
            return out;
        };
        let fog = &data["fog_of_war"];
        let number = |key: &str, fallback: f32| fog[key].as_f64().map_or(fallback, |v| v as f32);
        out.fog_a.x = number("desaturation", out.fog_a.x);
        out.fog_a.y = number("dim", out.fog_a.y);
        out.fog_a.z = number("veil_strength", out.fog_a.z);
        if let Some(c) = fog["veil_color"].as_str().and_then(hex_color) {
            out.fog_veil = c.extend(out.fog_veil.w);
        }
        if let Some(c) = fog["mist_color"].as_str().and_then(hex_color) {
            out.fog_mist = c.extend(out.fog_mist.w);
        }
        out.fog_mist.w = number("mist_max", out.fog_mist.w);
        out.fog_map.z = number("mist_min", out.fog_map.z);
        // RV-C cumulus shadows (`CampaignWeatherView._apply_cumulus_tuning`); cloud plane at
        // `cloud_height` 36 - 2 above the mean ground.
        let clouds = &data["clouds"];
        let num = |key: &str, fallback: f32| clouds[key].as_f64().map_or(fallback, |v| v as f32);
        let pair = |key: &str, fallback: [f32; 2]| match clouds[key].as_array() {
            Some(a) if a.len() >= 2 => [
                a[0].as_f64().unwrap_or(fallback[0] as f64) as f32,
                a[1].as_f64().unwrap_or(fallback[1] as f64) as f32,
            ],
            _ => fallback,
        };
        let cover = pair("cumulus_cover", [0.22, 0.5]);
        let fade = pair("cumulus_fade_px", [12.0, 4.0]);
        let band = pair("cumulus_medium_distance", [200.0, 500.0]);
        out.cumulus_a.y = num("cumulus_scale", out.cumulus_a.y);
        out.cumulus_a.z = num("cumulus_edge", out.cumulus_a.z);
        out.cumulus_a.w = num("cumulus_drift", out.cumulus_a.w);
        out.cumulus_b = Vec4::new(cover[0], cover[1], out.cumulus_b.z, fade[0]);
        if let Some(t) = clouds["cumulus_tint"].as_array()
            && t.len() >= 3
        {
            let c = |i: usize| t[i].as_f64().unwrap_or(1.0) as f32;
            out.cumulus_c = Vec4::new(c(0), c(1), c(2), fade[1]);
        } else {
            out.cumulus_c.w = fade[1];
        }
        out.cumulus_d = Vec4::new(
            num("cumulus_shadow", out.cumulus_d.x),
            num("cumulus_medium_scale", out.cumulus_d.y),
            band[0],
            band[1],
        );
        out.mist.z = num("wet_dim", out.mist.z);
        // TB6 valley mist (`CampaignWeatherView._apply_ground_tuning`).
        let mist = &data["morning_mist"];
        let mnum = |key: &str, fallback: f32| mist[key].as_f64().map_or(fallback, |v| v as f32);
        let mpair = |key: &str, fallback: (f32, f32)| match mist[key].as_array() {
            Some(a) if a.len() >= 2 => (
                a[0].as_f64().map_or(fallback.0, |v| v as f32),
                a[1].as_f64().map_or(fallback.1, |v| v as f32),
            ),
            _ => fallback,
        };
        let depth = mpair("valley_depth_m", (out.valley_a.x, out.valley_a.y));
        let plain = mpair("plain_altitude_m", (out.valley_a.z, out.valley_a.w));
        let crest = mpair("crest_height_m", (out.valley_b.x, out.valley_b.y));
        out.mist.w = mnum("opacity", out.mist.w).clamp(0.0, 1.0);
        out.valley_a = Vec4::new(depth.0, depth.1, plain.0, plain.1);
        out.valley_b = Vec4::new(
            crest.0,
            crest.1,
            mnum("plain_share", out.valley_b.z),
            mnum("bank_scale", out.valley_b.w),
        );
        out.valley_c = Vec4::new(
            mnum("drift", out.valley_c.x),
            mnum("grazing_gain", out.valley_c.y),
            mnum("lifted_share", out.valley_c.z),
            mnum("near_share", out.valley_c.w),
        );
        if let Some(c) = mist["color"].as_str().and_then(hex_color) {
            out.valley_color = c.extend(0.0);
        }
        out
    }

    /// Cumulus shadow strength at camera `distance` (`_update_cumulus_medium`), off on the
    /// parchment (`strategic` weight).
    pub fn cumulus_strength(&self, distance: f32, strategic: f32) -> f32 {
        let d = self.cumulus_d;
        let t = ((distance - d.z) / (d.w - d.z).max(1.0)).clamp(0.0, 1.0);
        let medium = 1.0 + (d.y - 1.0) * t * t * (3.0 - 2.0 * t);
        d.x * medium * (1.0 - strategic.clamp(0.0, 1.0))
    }
}

/// RV-C: fair-weather cumulus shadow strength from the camera distance and the parchment weight.
pub fn sync_cumulus(
    view: Option<Res<WeatherView>>,
    zoom: Option<Res<ZoomView>>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let (Some(view), Some(handle)) = (view, handle) else {
        return;
    };
    let strategic = zoom.map_or(0.0, |z| z.strategic);
    let Some(current) = materials.get(&handle.0).map(|m| {
        (
            m.look.cumulus_a.x,
            m.look.cumulus_strength(view.distance, strategic),
        )
    }) else {
        return;
    };
    if (current.0 - current.1).abs() > 1e-3
        && let Some(mut material) = materials.get_mut(&handle.0)
    {
        material.look.cumulus_a.x = current.1;
    }
}

/// Eased wet ground / fresh snow (0..1) following the weather under the camera focus.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct GroundWeather {
    pub wet: f32,
    pub fresh_snow: f32,
    /// Fog weight under the focus (the B channel of the Godot weather mask), eased.
    pub fog: f32,
}

/// Wet and snow targets of a weather kind at an intensity.
pub fn weather_targets(kind: Kind, intensity: f32) -> (f32, f32) {
    let k = 0.45 + 0.55 * intensity.clamp(0.0, 1.0);
    match kind {
        // Godot wets the ground under rain and storm only (`max(w.r, w.a)`).
        Kind::Rain => (0.9 * k, 0.0),
        Kind::Storm => (1.0, 0.0),
        Kind::Snow => (0.0, 0.8 * k),
        Kind::Fog | Kind::Clear => (0.0, 0.0),
    }
}

/// Fog weight of a weather kind at an intensity (B channel of the Godot weather mask).
pub fn fog_target(kind: Kind, intensity: f32) -> f32 {
    if kind == Kind::Fog {
        intensity.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// R8 image of a fog mask (linear, clamped).
pub fn mask_image(width: u32, height: u32, coverage: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        coverage,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// Uploads the fog mask when the fog revision changes.
pub fn sync_fog_mask(
    fog: Res<FogOfWar>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut seen_revision: Local<Option<u64>>,
) {
    let Some(handle) = handle else {
        return;
    };
    if *seen_revision == Some(fog.revision) && !handle.is_changed() {
        return;
    }
    let Some(mut material) = materials.get_mut(&handle.0) else {
        return;
    };
    *seen_revision = Some(fog.revision);
    // Dev hook for visual checks of the ground: `CENT_ANS_NO_FOG=1` lifts the veil.
    let veil_on = fog.active && std::env::var_os("CENT_ANS_NO_FOG").is_none();
    match (&fog.mask, veil_on) {
        (Some(mask), true) if !mask.coverage.is_empty() => {
            let image = mask_image(mask.width, mask.height, mask.coverage.clone());
            material.fog_mask = Some(images.add(image));
            material.look.fog_a.w = 1.0;
            material.look.fog_map.x = mask.width as f32 * mask.texel_px;
            material.look.fog_map.y = mask.height as f32 * mask.texel_px;
        }
        _ => {
            material.fog_mask = None;
            material.look.fog_a.w = 0.0;
        }
    }
}

/// Parchment weight of the strategic view into the material.
pub fn sync_zoom(
    view: Option<Res<ZoomView>>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let (Some(view), Some(handle)) = (view, handle) else {
        return;
    };
    let weight = view.strategic.clamp(0.0, 1.0);
    let current = materials.get(&handle.0).map(|m| m.look.weather.z);
    if current.is_some_and(|c| (c - weight).abs() > 1e-4)
        && let Some(mut material) = materials.get_mut(&handle.0)
    {
        material.look.weather.z = weight;
    }
}

/// Eases wet ground / fresh snow towards the weather under the camera focus.
pub fn sync_weather(
    time: Res<Time<Real>>,
    view: Option<Res<WeatherView>>,
    mut ground: ResMut<GroundWeather>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let (Some(view), Some(handle)) = (view, handle) else {
        return;
    };
    let (wet, snow) = weather_targets(view.focus_entry.kind, view.focus_entry.intensity);
    // Ground dries slowly and wets quickly.
    let rate = |from: f32, to: f32| {
        let speed = if to > from { 0.8 } else { 0.25 };
        from + (to - from) * (speed * time.delta_secs()).clamp(0.0, 1.0)
    };
    ground.wet = rate(ground.wet, wet);
    ground.fresh_snow = rate(ground.fresh_snow, snow);
    ground.fog = rate(
        ground.fog,
        fog_target(view.focus_entry.kind, view.focus_entry.intensity),
    );
    let morning = view.morning_mist.clamp(0.0, 1.0);
    let changed = materials.get(&handle.0).is_some_and(|m| {
        (m.look.weather.x - ground.wet).abs() > 1e-4
            || (m.look.weather.y - ground.fresh_snow).abs() > 1e-4
            || (m.look.mist.x - ground.fog).abs() > 1e-4
            || (m.look.mist.y - morning).abs() > 1e-3
    });
    if changed && let Some(mut material) = materials.get_mut(&handle.0) {
        material.look.weather.x = ground.wet;
        material.look.weather.y = ground.fresh_snow;
        material.look.mist.x = ground.fog;
        material.look.mist.y = morning;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cumulus_shadow_reads_the_data_and_fades_in_the_medium_band() {
        let text = std::fs::read_to_string(
            crate::paths::find_root()
                .unwrap()
                .join("data/ui/campaign_map.json"),
        )
        .unwrap();
        let p = LookParams::from_map_json(&text);
        assert!((p.cumulus_d.x - 0.4).abs() < 1e-6);
        assert!((p.cumulus_strength(60.0, 0.0) - 0.4).abs() < 1e-6);
        assert!((p.cumulus_strength(800.0, 0.0) - 0.4 * 0.3).abs() < 1e-5);
        assert_eq!(p.cumulus_strength(60.0, 1.0), 0.0);
    }

    #[test]
    fn reads_the_real_fog_tuning() {
        let text = include_str!("../../../../data/ui/campaign_map.json");
        let p = LookParams::from_map_json(text);
        assert!((p.fog_a.x - 0.5).abs() < 1e-5);
        assert!((p.fog_a.y - 0.8).abs() < 1e-5);
        assert!(p.fog_veil.x > 0.3, "veil colour parsed (linear)");
        assert!((p.fog_mist.w - 0.1).abs() < 1e-5);
    }

    #[test]
    fn weather_targets_are_wet_in_rain_and_white_in_snow() {
        assert_eq!(weather_targets(Kind::Clear, 1.0), (0.0, 0.0));
        let (wet, snow) = weather_targets(Kind::Rain, 0.5);
        assert!(wet > 0.5 && snow == 0.0);
        let (wet, snow) = weather_targets(Kind::Snow, 1.0);
        assert!(snow > 0.7 && wet < snow);
        // Fog does not wet the ground (Godot `max(w.r, w.a)`), it lays the morning mist.
        assert_eq!(weather_targets(Kind::Fog, 1.0), (0.0, 0.0));
        assert_eq!(fog_target(Kind::Fog, 0.8), 0.8);
        assert_eq!(fog_target(Kind::Rain, 0.8), 0.0);
    }

    #[test]
    fn morning_mist_reads_the_data() {
        let text = std::fs::read_to_string(
            crate::paths::find_root()
                .unwrap()
                .join("data/ui/campaign_map.json"),
        )
        .unwrap();
        let look = LookParams::from_map_json(&text);
        assert!((look.mist.z - 0.04).abs() < 1e-6, "wet_dim {}", look.mist.z);
        assert!((look.mist.w - 0.32).abs() < 1e-6, "opacity {}", look.mist.w);
        assert_eq!(look.valley_a, Vec4::new(10.0, 140.0, 40.0, 220.0));
        assert!((look.valley_c.w - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mask_image_is_r8() {
        let image = mask_image(4, 2, vec![255; 8]);
        assert_eq!(image.texture_descriptor.format, TextureFormat::R8Unorm);
    }
}
