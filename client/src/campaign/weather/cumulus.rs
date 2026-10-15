//! Sky cumulus field of `map_atmosphere.gd` / `map_atmo_cumulus.gdshader`: three stacked planes
//! at the height of the cloud shadows, each carrying a thresholded low-frequency noise field
//! (a few big banks with a frayed edge; the higher planes keep only the cores, which gives
//! domed clouds), lit on the sun side. The Godot fragment noise is baked once into tileable
//! textures (one per layer) and the field drifts with the wind through `uv_transform`.
//! Settings: `data/fx/map_atmosphere.json` (`cumulus_cards`; disabled there at the moment,
//! `CENT_ANS_ATMOSPHERE_ALL=1` switches it on for proofs).
//!
//! Not ported: the sea penalty (clouds thinner over open sea), the `wet` sky lowering the
//! threshold, the depth fade and the focus clearing (a single plane per layer: the band fade
//! keeps the cards away from the close views where the focus hole would matter).

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use super::WeatherView;
use super::aurora::ALL_ENV;
use super::model::{lerp, smoothstep};
use super::sky::{arr, band_fade, flag, num, repeat_sampler, tile_noise};
use crate::app::AppState;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::provinces::data::ProvinceMap;

/// Stacked planes (`INSTANCE_CUSTOM.x` = 0, 0.5, 1 in the Godot shader).
pub const LAYERS: usize = 3;
/// Texture side (pixels) of one layer.
const SIZE: u32 = 512;
/// Wind of the map (`wind` of the data file, map units per second).
const WIND: [f32; 2] = [3.0, 0.6];

#[derive(Resource, Debug, Clone)]
pub struct CumulusTuning {
    pub enabled: bool,
    pub base_height: f32,
    pub layer_height: f32,
    pub opacity: f32,
    pub soft: f32,
    pub fray: f32,
    pub layer_step: f32,
    pub gain: f32,
    pub bank_band: [f32; 2],
    pub distance_in: [f32; 2],
    pub distance_out: [f32; 2],
    pub lit_color: [f32; 3],
    pub shade_color: [f32; 3],
}

impl Default for CumulusTuning {
    fn default() -> Self {
        CumulusTuning {
            enabled: true,
            base_height: 34.0,
            layer_height: 1.6,
            opacity: 0.6,
            soft: 0.2,
            fray: 0.05,
            layer_step: 0.06,
            gain: 5.0,
            bank_band: [0.66, 0.8],
            distance_in: [260.0, 520.0],
            distance_out: [1000.0, 1250.0],
            lit_color: [1.0, 0.97, 0.92],
            shade_color: [0.56, 0.62, 0.72],
        }
    }
}

impl CumulusTuning {
    pub fn from_json(root: &Value) -> Self {
        let d = CumulusTuning::default();
        let null = Value::Null;
        let rule = root.get("cumulus_cards").unwrap_or(&null);
        CumulusTuning {
            enabled: flag(rule, "enabled"),
            base_height: num(rule, "base_height", d.base_height),
            layer_height: num(rule, "layer_height", d.layer_height),
            opacity: num(rule, "opacity", d.opacity),
            soft: num(rule, "soft", d.soft),
            fray: num(rule, "fray", d.fray),
            layer_step: num(rule, "layer_step", d.layer_step),
            gain: num(rule, "gain", d.gain),
            bank_band: arr(rule, "bank_band", d.bank_band),
            distance_in: arr(rule, "distance_in", d.distance_in),
            distance_out: arr(rule, "distance_out", d.distance_out),
            lit_color: arr(rule, "lit_color", d.lit_color),
            shade_color: arr(rule, "shade_color", d.shade_color),
        }
    }
}

/// Bank field at (u, v), both tileable: warped low-frequency noise plus a finer octave.
fn field(u: f32, v: f32) -> f32 {
    let wx = tile_noise(u, v, 6, 6, 201) - 0.5;
    let wy = tile_noise(u, v, 6, 6, 307) - 0.5;
    let (uu, vv) = (u + wx * 0.1, v + wy * 0.1);
    tile_noise(uu, vv, 9, 9, 401) * 0.7 + tile_noise(uu, vv, 19, 19, 503) * 0.3
}

/// Texel of one layer: (colour, alpha 0..1). `layer` = 0, 0.5, 1.
pub fn cumulus_pixel(u: f32, v: f32, layer: f32, tuning: &CumulusTuning) -> ([f32; 3], f32) {
    let fray = (tile_noise(u, v, 40, 40, 601) - 0.5) * tuning.fray * 2.0;
    let region = tile_noise(u, v, 2, 2, 701);
    let f = field(u, v) + fray + (region - 0.5) * 0.25;
    let above = f - tuning.bank_band[0] - layer * tuning.layer_step;
    let width = (tuning.bank_band[1] - tuning.bank_band[0]).max(0.02) * (1.0 + tuning.soft * 3.0);
    let thick = smoothstep(0.0, width, above);
    // Sun side (fixed west-north-west light): the field falls towards the sun on lit flanks.
    let field_sun = field(u + 0.006, v + 0.01);
    let lit = (0.55 + (field(u, v) - field_sun) * tuning.gain * 4.0).clamp(0.0, 1.0);
    let lit = (lit * (0.55 + 0.5 * layer) + 0.2 * smoothstep(0.0, 0.3, above)).clamp(0.0, 1.0);
    let mix3 = |a: [f32; 3], b: [f32; 3], t: f32| {
        [
            lerp(a[0], b[0], t),
            lerp(a[1], b[1], t),
            lerp(a[2], b[2], t),
        ]
    };
    let col = mix3(
        tuning.shade_color,
        tuning.lit_color,
        smoothstep(0.1, 0.9, lit),
    );
    (
        mix3(tuning.shade_color, col, smoothstep(0.0, 0.35, thick)),
        thick.clamp(0.0, 0.7 / tuning.opacity.max(0.01)),
    )
}

#[derive(Component)]
struct CumulusPlane;

#[derive(Resource)]
struct CumulusAssets {
    materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Resource, Debug, Default)]
pub struct CumulusStats {
    pub planes: usize,
    pub alpha: f32,
    pub visible: bool,
    /// Share of texels with some cloud, per layer (diagnostic).
    pub coverage: [f32; LAYERS],
}

fn load_tuning(mut tuning: ResMut<CumulusTuning>, mut stats: ResMut<CumulusStats>) {
    *stats = CumulusStats::default();
    if let Ok(root) = crate::paths::find_root() {
        let path = crate::paths::data_dir(&root).join("fx/map_atmosphere.json");
        if let Some(value) = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        {
            *tuning = CumulusTuning::from_json(&value);
        }
    }
    if std::env::var_os(ALL_ENV).is_some() {
        tuning.enabled = true;
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_cumulus(
    mut commands: Commands,
    tuning: Res<CumulusTuning>,
    map: Option<Res<ProvinceMap>>,
    existing: Query<(), With<CumulusPlane>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stats: ResMut<CumulusStats>,
) {
    let Some(map) = map else {
        return;
    };
    if !tuning.enabled || !existing.is_empty() {
        return;
    }
    let (w, h) = (map.width as f32, map.height as f32);
    let plane = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    let mut handles = Vec::new();
    for index in 0..LAYERS {
        let layer = index as f32 / (LAYERS - 1) as f32;
        let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
        let mut covered = 0usize;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let (rgb, alpha) = cumulus_pixel(
                    x as f32 / SIZE as f32,
                    y as f32 / SIZE as f32,
                    layer,
                    &tuning,
                );
                covered += usize::from(alpha > 0.02);
                data.extend_from_slice(&[
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                    (alpha.min(1.0) * 255.0) as u8,
                ]);
            }
        }
        stats.coverage[index] = covered as f32 / (SIZE * SIZE) as f32;
        let mut image = Image::new(
            Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = repeat_sampler();
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            base_color_texture: Some(images.add(image)),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            double_sided: true,
            ..default()
        });
        handles.push(material.clone());
        commands.spawn((
            Name::new("CumulusField"),
            CumulusPlane,
            Mesh3d(plane.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(Vec3::new(
                w * 0.5,
                tuning.base_height + tuning.layer_height * index as f32,
                h * 0.5,
            ))
            .with_scale(Vec3::new(w, 1.0, h)),
            Visibility::Hidden,
            NotShadowCaster,
            DespawnOnExit(AppState::Campaign),
        ));
    }
    stats.planes = LAYERS;
    commands.insert_resource(CumulusAssets { materials: handles });
}

#[allow(clippy::too_many_arguments)]
fn update_cumulus(
    time: Res<Time<Real>>,
    view: Res<WeatherView>,
    zoom: Res<ZoomView>,
    tuning: Res<CumulusTuning>,
    map: Option<Res<ProvinceMap>>,
    assets: Option<Res<CumulusAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stats: ResMut<CumulusStats>,
    mut planes: Query<&mut Visibility, With<CumulusPlane>>,
) {
    let (Some(assets), Some(map)) = (assets, map) else {
        return;
    };
    let fade =
        band_fade(view.distance, tuning.distance_in, tuning.distance_out) * (1.0 - zoom.strategic);
    let alpha = (tuning.opacity * fade).clamp(0.0, 1.0);
    let seconds = time.elapsed_secs();
    for handle in &assets.materials {
        if let Some(mut m) = materials.get_mut(handle) {
            m.base_color = Color::srgba(1.0, 1.0, 1.0, alpha);
            // the field drifts with the wind: u' = u - t * wind / size
            m.uv_transform = Affine2::from_translation(Vec2::new(
                -seconds * WIND[0] / map.width as f32,
                -seconds * WIND[1] / map.height as f32,
            ));
        }
    }
    let want = if alpha > 0.004 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut planes {
        if *visibility != want {
            *visibility = want;
        }
    }
    stats.alpha = alpha;
    stats.visible = want == Visibility::Inherited;
}

pub fn dump(world: &World) -> Value {
    let s = world.resource::<CumulusStats>();
    json!({
        "enabled": world.resource::<CumulusTuning>().enabled,
        "planes": s.planes, "alpha": s.alpha, "visible": s.visible,
        "coverage": s.coverage,
    })
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<CumulusTuning>()
        .init_resource::<CumulusStats>()
        .add_systems(OnEnter(AppState::Campaign), load_tuning)
        .add_systems(
            Update,
            (spawn_cumulus, update_cumulus)
                .chain()
                .after(super::update_view)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage(layer: f32, tuning: &CumulusTuning) -> f32 {
        let n = 128;
        let mut covered = 0;
        for y in 0..n {
            for x in 0..n {
                let (_, a) = cumulus_pixel(x as f32 / n as f32, y as f32 / n as f32, layer, tuning);
                covered += usize::from(a > 0.02);
            }
        }
        covered as f32 / (n * n) as f32
    }

    #[test]
    fn banks_are_rare_and_higher_layers_keep_only_the_cores() {
        let tuning = CumulusTuning {
            bank_band: [0.66, 0.8],
            ..Default::default()
        };
        let low = coverage(0.0, &tuning);
        let high = coverage(1.0, &tuning);
        assert!(low > 0.0 && low < 0.5, "low layer coverage {low}");
        assert!(high <= low, "{high} > {low}");
    }

    #[test]
    fn tuning_reads_the_data_file() {
        let json: Value = serde_json::from_str(
            r#"{"cumulus_cards":{"enabled":false,"opacity":0.15,"distance_in":[640,960]}}"#,
        )
        .unwrap();
        let t = CumulusTuning::from_json(&json);
        assert!(!t.enabled);
        assert_eq!(t.opacity, 0.15);
        assert_eq!(t.distance_in, [640.0, 960.0]);
        assert_eq!(t.layer_step, 0.06);
    }
}
