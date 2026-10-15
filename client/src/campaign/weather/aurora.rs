//! Aurora borealis of `map_atmosphere.gd` / `map_atmo_aurora.gdshader`: a vertical additive
//! curtain along the north edge of the map, visible in winter and strongest at dusk. The Godot
//! shader is replaced by an additive unlit `StandardMaterial` over a baked, horizontally tileable
//! ray texture scrolled slowly; the foot / side envelope lives in the mesh's vertex colours.
//! Settings: `data/fx/map_atmosphere.json` (`aurora`; disabled there at the moment,
//! `CENT_ANS_ATMOSPHERE_ALL=1` switches the disabled families on for proofs).

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use super::WeatherView;
use super::model::{lerp, smoothstep};
use super::sky::{arr, band_fade, flag, num, repeat_sampler, tile_noise};
use crate::app::AppState;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::terrain::season::SeasonWeights;

/// Environment switch for the families the data file disables.
pub const ALL_ENV: &str = "CENT_ANS_ATMOSPHERE_ALL";

/// Brightness of the additive curtain (the Godot albedo is multiplied by 22 in HDR).
const GAIN: f32 = 5.0;

/// Dusk factor (0 = day, 1 = golden evening). There is no turn light in the client yet: the look
/// lot sets this resource when it ports `turn_light.gd`.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct SkyDusk(pub f32);

#[derive(Resource, Debug, Clone)]
pub struct AuroraTuning {
    pub enabled: bool,
    pub north_margin: f32,
    pub height: [f32; 2],
    pub width_share: f32,
    pub opacity: f32,
    pub distance_in: [f32; 2],
    pub distance_out: [f32; 2],
    pub night_floor: f32,
    pub colors: [[f32; 3]; 3],
    pub speed: f32,
}

impl Default for AuroraTuning {
    fn default() -> Self {
        AuroraTuning {
            enabled: true,
            north_margin: 0.0,
            height: [30.0, 150.0],
            width_share: 0.8,
            opacity: 0.9,
            distance_in: [450.0, 900.0],
            distance_out: [2200.0, 3200.0],
            night_floor: 0.35,
            colors: [[0.2, 0.95, 0.55], [0.35, 0.55, 1.0], [0.75, 0.3, 0.9]],
            speed: 0.05,
        }
    }
}

impl AuroraTuning {
    pub fn from_json(root: &Value) -> Self {
        let d = AuroraTuning::default();
        let null = Value::Null;
        let rule = root.get("aurora").unwrap_or(&null);
        let mut colors = d.colors;
        if let Some(list) = rule.get("colors").and_then(Value::as_array) {
            for (slot, value) in colors.iter_mut().zip(list) {
                if let Some(rgb) = value.as_array().filter(|a| a.len() >= 3) {
                    for (c, v) in slot.iter_mut().zip(rgb) {
                        *c = v.as_f64().unwrap_or(f64::from(*c)) as f32;
                    }
                }
            }
        }
        AuroraTuning {
            enabled: flag(rule, "enabled"),
            north_margin: num(rule, "north_margin_px", d.north_margin),
            height: arr(rule, "height", d.height),
            width_share: num(rule, "width_share", d.width_share),
            opacity: num(rule, "opacity", d.opacity),
            distance_in: arr(rule, "distance_in", d.distance_in),
            distance_out: arr(rule, "distance_out", d.distance_out),
            night_floor: num(rule, "night_floor", d.night_floor),
            colors,
            speed: num(rule, "speed", d.speed),
        }
    }
}

/// Aurora strength: winter only, `night_floor` by day, full at golden dusk (`aurora_strength`).
pub fn aurora_strength(winter: f32, dusk: f32, night_floor: f32) -> f32 {
    winter.clamp(0.0, 1.0) * lerp(night_floor, 1.0, dusk.clamp(0.0, 1.0))
}

/// Ray colour at texel (u in 0..1 west to east, v in 0..1 foot to top): the Godot fragment
/// without the side fade (kept in the vertex colours so it does not scroll). RGB, linear.
pub fn aurora_pixel(u: f32, v: f32, colors: &[[f32; 3]; 3]) -> [f32; 3] {
    let wave = tile_noise(u, 0.0, 4, 1, 101) * 0.65 + tile_noise(u, 0.0, 8, 1, 131) * 0.35 - 0.5;
    let foot = 0.1 + wave * 0.18;
    let y = v - foot;
    let rays = smoothstep(0.2, 0.9, tile_noise(u + wave * 0.17, 0.0, 70, 1, 17));
    let rays = lerp(0.35, 1.0, rays);
    let band = smoothstep(0.0, 0.05, y) * (1.0 - y * 0.95).clamp(0.0, 1.0).powf(1.1);
    let patches = smoothstep(0.15, 0.5, tile_noise(u, 0.5, 3, 1, 53));
    let k = band * rays * patches;
    let mix3 = |a: [f32; 3], b: [f32; 3], t: f32| {
        [
            lerp(a[0], b[0], t),
            lerp(a[1], b[1], t),
            lerp(a[2], b[2], t),
        ]
    };
    let low_mid = mix3(colors[0], colors[1], smoothstep(0.1, 0.5, y));
    let col = mix3(low_mid, colors[2], smoothstep(0.45, 0.9, y));
    [col[0] * k, col[1] * k, col[2] * k]
}

#[derive(Component)]
struct Aurora;

#[derive(Resource)]
struct AuroraAssets {
    material: Handle<StandardMaterial>,
}

#[derive(Resource, Debug, Default)]
pub struct AuroraStats {
    pub spawned: bool,
    pub fade: f32,
    pub strength: f32,
    pub alpha: f32,
    pub visible: bool,
}

fn side_envelope(u: f32) -> f32 {
    smoothstep(0.0, 0.12, u) * smoothstep(0.0, 0.12, 1.0 - u)
}

/// Curtain quad, facing +Z, centred; side envelope in vertex colours.
fn curtain(width: f32, height: f32) -> Mesh {
    const N: usize = 24;
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    for row in 0..=1 {
        for col in 0..=N {
            let u = col as f32 / N as f32;
            let v = row as f32;
            positions.push([(u - 0.5) * width, (v - 0.5) * height, 0.0]);
            uvs.push([u, 1.0 - v]);
            colors.push([1.0, 1.0, 1.0, side_envelope(u)]);
        }
    }
    let mut indices = Vec::new();
    for col in 0..N {
        let a = col as u32;
        let c = a + (N + 1) as u32;
        indices.extend_from_slice(&[a, a + 1, c + 1, a, c + 1, c]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 2 * (N + 1)])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn load_tuning(mut tuning: ResMut<AuroraTuning>) {
    if let Ok(root) = crate::paths::find_root() {
        let path = crate::paths::data_dir(&root).join("fx/map_atmosphere.json");
        if let Some(value) = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        {
            *tuning = AuroraTuning::from_json(&value);
        }
    }
    if std::env::var_os(ALL_ENV).is_some() {
        tuning.enabled = true;
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_aurora(
    mut commands: Commands,
    tuning: Res<AuroraTuning>,
    map: Option<Res<ProvinceMap>>,
    existing: Query<(), With<Aurora>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stats: ResMut<AuroraStats>,
) {
    let Some(map) = map else {
        return;
    };
    if !tuning.enabled || !existing.is_empty() {
        return;
    }
    let (w, h) = (512u32, 128u32);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            // image row 0 is the top of the curtain
            let (u, v) = (x as f32 / w as f32, 1.0 - y as f32 / h as f32);
            let rgb = aurora_pixel(u, v, &tuning.colors);
            data.extend_from_slice(&[
                (rgb[0].clamp(0.0, 1.0) * 255.0) as u8,
                (rgb[1].clamp(0.0, 1.0) * 255.0) as u8,
                (rgb[2].clamp(0.0, 1.0) * 255.0) as u8,
                255,
            ]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = repeat_sampler();
    let material = materials.add(StandardMaterial {
        base_color: Color::LinearRgba(LinearRgba::new(GAIN, GAIN, GAIN, 0.0)),
        base_color_texture: Some(images.add(image)),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        double_sided: true,
        fog_enabled: false,
        ..default()
    });
    let (map_w, height) = (map.width as f32, tuning.height[1] - tuning.height[0]);
    commands.insert_resource(AuroraAssets {
        material: material.clone(),
    });
    commands.spawn((
        Name::new("Aurora"),
        Aurora,
        Mesh3d(meshes.add(curtain(map_w * tuning.width_share, height))),
        MeshMaterial3d(material),
        Transform::from_translation(Vec3::new(
            map_w * 0.5,
            (tuning.height[0] + tuning.height[1]) * 0.5,
            tuning.north_margin,
        )),
        Visibility::Hidden,
        NotShadowCaster,
        DespawnOnExit(AppState::Campaign),
    ));
    stats.spawned = true;
}

#[allow(clippy::too_many_arguments)]
fn update_aurora(
    time: Res<Time<Real>>,
    view: Res<WeatherView>,
    zoom: Res<ZoomView>,
    tuning: Res<AuroraTuning>,
    seasons: Res<SeasonWeights>,
    dusk: Res<SkyDusk>,
    assets: Option<Res<AuroraAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stats: ResMut<AuroraStats>,
    mut node: Query<&mut Visibility, With<Aurora>>,
) {
    let Some(assets) = assets else {
        return;
    };
    let fade =
        band_fade(view.distance, tuning.distance_in, tuning.distance_out) * (1.0 - zoom.strategic);
    let strength = aurora_strength(seasons.weights.w, dusk.0, tuning.night_floor) * tuning.opacity;
    let alpha = (fade * strength).clamp(0.0, 1.0);
    if let Some(mut m) = materials.get_mut(&assets.material) {
        m.base_color = Color::LinearRgba(LinearRgba::new(GAIN, GAIN, GAIN, alpha));
        // slow eastward drift: u' = u - t * speed * 0.75 (`quad_uv.x * 4 + t * 3` of the shader)
        m.uv_transform =
            Affine2::from_translation(Vec2::new(time.elapsed_secs() * tuning.speed * 0.75, 0.0));
    }
    for mut visibility in &mut node {
        let want = if alpha > 0.002 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        stats.visible = want == Visibility::Inherited;
    }
    stats.fade = fade;
    stats.strength = strength;
    stats.alpha = alpha;
}

pub fn dump(world: &World) -> Value {
    let s = world.resource::<AuroraStats>();
    json!({
        "enabled": world.resource::<AuroraTuning>().enabled,
        "spawned": s.spawned, "visible": s.visible,
        "fade": s.fade, "strength": s.strength, "alpha": s.alpha,
    })
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<AuroraTuning>()
        .init_resource::<AuroraStats>()
        .init_resource::<SkyDusk>()
        .add_systems(
            OnEnter(AppState::Campaign),
            (
                |mut stats: ResMut<AuroraStats>| *stats = AuroraStats::default(),
                load_tuning,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (spawn_aurora, update_aurora)
                .chain()
                .after(super::update_view)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strength_is_winter_only_and_peaks_at_dusk() {
        assert_eq!(aurora_strength(0.0, 1.0, 0.35), 0.0);
        assert!((aurora_strength(1.0, 0.0, 0.35) - 0.35).abs() < 1e-6);
        assert!((aurora_strength(1.0, 1.0, 0.35) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pixels_glow_above_the_foot_and_fade_at_the_top() {
        let colors = AuroraTuning::default().colors;
        let lit: f32 = (0..512)
            .map(|i| aurora_pixel(i as f32 / 512.0, 0.3, &colors)[1])
            .sum();
        assert!(lit > 5.0, "no light in the curtain: {lit}");
        let top: f32 = aurora_pixel(0.4, 1.0, &colors).iter().sum();
        assert!(top < 0.3);
        assert_eq!(aurora_pixel(0.4, 0.0, &colors), [0.0; 3]);
    }

    #[test]
    fn tuning_reads_the_data_file() {
        let json: Value = serde_json::from_str(
            r#"{"aurora":{"enabled":false,"height":[0,260],"colors":[[1,0,0]]}}"#,
        )
        .unwrap();
        let t = AuroraTuning::from_json(&json);
        assert!(!t.enabled);
        assert_eq!(t.height, [0.0, 260.0]);
        assert_eq!(t.colors[0], [1.0, 0.0, 0.0]);
        assert_eq!(t.colors[2], AuroraTuning::default().colors[2]);
    }
}
