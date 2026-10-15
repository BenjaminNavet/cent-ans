//! Clouds and cloud shadows as sprites: soft quads at `cloud_height` over the provinces whose
//! weather carries clouds (rain, snow, storm), drifting slowly, each with a flat shadow on the
//! ground shifted along the sun. Replaces the cloud plane / terrain cloud-shadow shaders of
//! `campaign_weather_view.gd` with entities (the terrain shader is not touched).
//!
//! Opacity follows the Godot curve (`CloudTuning::alpha_at`): none close, thin at medium range,
//! fuller far away; the wide-view cut keeps only heavy weather; the selected province stays clear.

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use super::model::{CloudTuning, Kind, hash01, lerp, smoothstep};
use super::{CampaignWeather, WeatherTuning, WeatherView};
use crate::app::AppState;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::provinces::picker::SelectedProvince;
use crate::campaign::terrain::CampaignSun;
use crate::campaign::terrain::surface::GroundHeight;

/// Half-width (map units) of the back-and-forth drift of a cloud.
const DRIFT_UNITS: f32 = 28.0;
/// Seconds of a full drift cycle.
const DRIFT_PERIOD: f32 = 140.0;
/// Shadow opacity near the camera (fades out in the medium view as the clouds thicken).
const SHADOW_ALPHA: f32 = 0.16;
/// Lift of a shadow above the highest ground under it.
const SHADOW_LIFT: f32 = 0.12;

/// Fair-weather cumulus shadow: opacity near the camera (`cumulus_shadow` of the Godot data).
const FAIR_SHADOW_ALPHA: f32 = 0.10;
/// Share of the clear provinces that carry a drifting cumulus shadow.
const FAIR_SHARE: f32 = 0.45;
/// Camera distances of the fair shadow fade (`cumulus_medium_distance`, then gone).
const FAIR_FADE: [f32; 3] = [200.0, 500.0, 900.0];
/// Fair-weather cumulus shadows as sprites: off, the terrain shader draws them (RV-C,
/// `campaign_cloud_shadow.gdshaderinc` ported in `terrain.wesl`, lot campvis4).
const FAIR_SPRITES: bool = false;
/// Opacity share kept at the end of the medium band (`cumulus_medium_scale`).
const FAIR_MEDIUM_SCALE: f32 = 0.3;

/// Fair-shadow opacity at camera `distance` on the parchment weight `strategic` (0..1):
/// full close, `FAIR_MEDIUM_SCALE` in the medium band, nothing far or on the parchment.
pub fn fair_alpha(distance: f32, strategic: f32) -> f32 {
    let medium = lerp(
        1.0,
        FAIR_MEDIUM_SCALE,
        smoothstep(FAIR_FADE[0], FAIR_FADE[1], distance),
    );
    FAIR_SHADOW_ALPHA
        * medium
        * (1.0 - smoothstep(FAIR_FADE[1], FAIR_FADE[2], distance))
        * (1.0 - strategic.clamp(0.0, 1.0))
}

/// One cloud or one shadow of a cloud.
#[derive(Component, Debug, Clone)]
pub struct CloudSprite {
    pub shadow: bool,
    /// Shadow of a fair-weather cumulus (no cloud sprite under it).
    pub fair: bool,
    pub province: u16,
    pub kind: Kind,
    pub intensity: f32,
    pub base: Vec2,
    pub phase: f32,
    pub size: f32,
    /// Highest ground under the sprite (shadows), refreshed when the relief scale changes.
    pub ground_y: f32,
    ground_for_scale: f32,
}

#[derive(Resource)]
struct CloudAssets {
    plane: Handle<Mesh>,
    /// Per kind: rain, snow, storm.
    clouds: [Handle<StandardMaterial>; 3],
    shadow: Handle<StandardMaterial>,
    fair_shadow: Handle<StandardMaterial>,
}

fn kind_slot(kind: Kind) -> usize {
    match kind {
        Kind::Snow => 1,
        Kind::Storm => 2,
        _ => 0,
    }
}

fn cloud_tint(slot: usize) -> Color {
    match slot {
        1 => Color::srgb(0.93, 0.95, 0.98),
        2 => Color::srgb(0.38, 0.40, 0.46),
        _ => Color::srgb(0.66, 0.69, 0.74),
    }
}

/// Soft cloud blob: radial falloff broken up by value noise (white, alpha only varies).
pub fn blob_pixels(size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let grid = 6u32;
    let lattice = |x: u32, y: u32| hash01(u64::from(x) * 7919 + u64::from(y) * 104_729 + 17);
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (x as f32 / size as f32, y as f32 / size as f32);
            let r = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 2.0;
            let (gx, gy) = (u * grid as f32, v * grid as f32);
            let (ix, iy) = (gx.floor() as u32, gy.floor() as u32);
            let (fx, fy) = (gx.fract(), gy.fract());
            let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
            let top = lattice(ix, iy) * (1.0 - sx) + lattice(ix + 1, iy) * sx;
            let bottom = lattice(ix, iy + 1) * (1.0 - sx) + lattice(ix + 1, iy + 1) * sx;
            let noise = top * (1.0 - sy) + bottom * sy;
            let falloff = 1.0 - smoothstep(0.25, 1.0, r);
            let alpha = (falloff * (0.55 + 0.6 * noise)).clamp(0.0, 1.0);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    data
}

fn setup_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let blob = images.add(Image::new(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        blob_pixels(64),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let material = |color: Color| StandardMaterial {
        base_color: color,
        base_color_texture: Some(blob.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        double_sided: true,
        ..default()
    };
    commands.insert_resource(CloudAssets {
        plane: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
        clouds: [0, 1, 2].map(|slot| materials.add(material(cloud_tint(slot).with_alpha(0.0)))),
        shadow: materials.add(material(Color::srgba(0.05, 0.06, 0.09, 0.0))),
        fair_shadow: materials.add(material(Color::srgba(0.05, 0.06, 0.09, 0.0))),
    });
}

/// Clouds of one province: count by weather kind and intensity.
pub fn cloud_count(kind: Kind, intensity: f32) -> usize {
    let base = match kind {
        Kind::Rain | Kind::Snow => 2,
        Kind::Storm => 3,
        _ => 0,
    };
    if base > 0 && intensity > 0.7 {
        base + 1
    } else {
        base
    }
}

/// Deterministic layout of a province's clouds: (offset from the centroid, size, phase).
pub fn layout(province: u16, index: usize, area_px: f64) -> (Vec2, f32, f32) {
    let seed = u64::from(province) * 131 + index as u64 * 17;
    let reach = (area_px.max(1.0).sqrt() as f32) * 0.3;
    let offset = Vec2::new(hash01(seed) - 0.5, hash01(seed + 1) - 0.5) * 2.0 * reach;
    let size = ((area_px.max(1.0).sqrt() as f32) * 1.7).clamp(60.0, 200.0)
        * (0.8 + 0.4 * hash01(seed + 2));
    (offset, size, hash01(seed + 3) * std::f32::consts::TAU)
}

#[allow(clippy::too_many_arguments)]
fn sync_clouds(
    mut commands: Commands,
    weather: Res<CampaignWeather>,
    tuning: Res<WeatherTuning>,
    map: Option<Res<ProvinceMap>>,
    assets: Option<Res<CloudAssets>>,
    existing: Query<Entity, With<CloudSprite>>,
    mut built: Local<u64>,
) {
    let (Some(map), Some(assets)) = (map, assets) else {
        return;
    };
    if *built == weather.revision && weather.revision != 0 {
        return;
    }
    if weather.by_province.is_empty() {
        return;
    }
    *built = weather.revision;
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for feature in map.provinces() {
        let entry = weather.entry(&feature.id);
        let index = feature.index as u16;
        let centroid = Vec2::from(feature.centroid);
        if FAIR_SPRITES
            && cloud_count(entry.kind, entry.intensity) == 0
            && hash01(u64::from(index) * 389 + 41) < FAIR_SHARE
        {
            let (offset, size, phase) = layout(index, 9, feature.area_px);
            commands.spawn((
                Name::new("FairCloudShadow"),
                CloudSprite {
                    shadow: true,
                    fair: true,
                    province: index,
                    kind: entry.kind,
                    intensity: entry.intensity,
                    base: centroid + offset,
                    phase,
                    size,
                    ground_y: 0.0,
                    ground_for_scale: -1.0,
                },
                Mesh3d(assets.plane.clone()),
                MeshMaterial3d(assets.fair_shadow.clone()),
                Transform::from_translation(Vec3::new(
                    centroid.x + offset.x,
                    tuning.clouds.height,
                    centroid.y + offset.y,
                ))
                .with_scale(Vec3::new(size, 1.0, size * 0.75)),
                Visibility::Hidden,
                NotShadowCaster,
                DespawnOnExit(AppState::Campaign),
            ));
        }
        for i in 0..cloud_count(entry.kind, entry.intensity) {
            let (offset, size, phase) = layout(index, i, feature.area_px);
            for shadow in [false, true] {
                let material = if shadow {
                    assets.shadow.clone()
                } else {
                    assets.clouds[kind_slot(entry.kind)].clone()
                };
                commands.spawn((
                    Name::new(if shadow { "CloudShadow" } else { "Cloud" }),
                    CloudSprite {
                        shadow,
                        fair: false,
                        province: index,
                        kind: entry.kind,
                        intensity: entry.intensity,
                        base: centroid + offset,
                        phase,
                        size,
                        ground_y: 0.0,
                        ground_for_scale: -1.0,
                    },
                    Mesh3d(assets.plane.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(Vec3::new(
                        centroid.x + offset.x,
                        tuning.clouds.height,
                        centroid.y + offset.y,
                    ))
                    .with_scale(Vec3::new(size, 1.0, size * 0.75)),
                    Visibility::Hidden,
                    NotShadowCaster,
                    DespawnOnExit(AppState::Campaign),
                ));
            }
        }
    }
}

/// Opacity of a cloud of `kind` and of the shared shadow at camera distance `distance`.
pub fn alphas(tuning: &CloudTuning, distance: f32) -> (f32, f32) {
    let cloud = tuning.alpha_at(distance);
    let shadow = SHADOW_ALPHA * (1.0 - smoothstep(tuning.medium[0], tuning.medium[1], distance));
    (cloud, shadow)
}

/// Whether a cloud of `intensity` survives the wide-view cut (storms always do).
pub fn survives_cut(kind: Kind, intensity: f32, cut: f32) -> bool {
    kind == Kind::Storm || intensity >= cut
}

#[allow(clippy::too_many_arguments)]
fn update_clouds(
    time: Res<Time>,
    view: Res<WeatherView>,
    zoom: Res<ZoomView>,
    tuning: Res<WeatherTuning>,
    selected: Option<Res<SelectedProvince>>,
    sun: Query<&Transform, (With<CampaignSun>, Without<CloudSprite>)>,
    assets: Option<Res<CloudAssets>>,
    ground: GroundHeight,
    relief: Option<Res<crate::campaign::terrain::relief_state::ReliefState>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stats: ResMut<CloudStats>,
    mut sprites: Query<(&mut CloudSprite, &mut Transform, &mut Visibility)>,
) {
    let Some(assets) = assets else {
        return;
    };
    // Clouds fade out on the parchment view (`ZoomView::strategic`).
    let clear = 1.0 - zoom.strategic;
    let (cloud_alpha, shadow_alpha) = alphas(&tuning.clouds, view.distance);
    let (cloud_alpha, shadow_alpha) = (cloud_alpha * clear, shadow_alpha * clear);
    let fair_alpha = fair_alpha(view.distance, zoom.strategic);
    *stats = CloudStats {
        cloud_alpha,
        shadow_alpha,
        fair_alpha,
        ..default()
    };
    for (slot, handle) in assets.clouds.iter().enumerate() {
        if let Some(mut material) = materials.get_mut(handle) {
            let factor = if slot == 2 { 1.35 } else { 1.0 };
            let alpha = (cloud_alpha * factor).clamp(0.0, 1.0);
            if (material.base_color.alpha() - alpha).abs() > 1e-3 {
                material.base_color = cloud_tint(slot).with_alpha(alpha);
            }
        }
    }
    if let Some(mut material) = materials.get_mut(&assets.shadow)
        && (material.base_color.alpha() - shadow_alpha).abs() > 1e-3
    {
        material.base_color = Color::srgba(0.05, 0.06, 0.09, shadow_alpha);
    }
    if let Some(mut material) = materials.get_mut(&assets.fair_shadow)
        && (material.base_color.alpha() - fair_alpha).abs() > 1e-3
    {
        material.base_color = Color::srgba(0.05, 0.06, 0.09, fair_alpha);
    }
    let to_sun = sun.single().map_or(Vec3::new(0.3, 0.8, 0.3), |t| {
        (-t.forward().as_vec3()).normalize()
    });
    let slope = Vec2::new(to_sun.x, to_sun.z) / to_sun.y.max(0.2);
    let cut = tuning.clouds.wide_cut(view.distance);
    let selected = selected.map_or(0, |s| s.index);
    let seconds = time.elapsed_secs();
    let scale = relief.as_ref().map_or(0.0, |r| r.vertical_scale);
    let wind = Vec2::new(1.0, 0.35).normalize();
    for (mut sprite, mut transform, mut visibility) in &mut sprites {
        let drift = wind
            * (seconds * std::f32::consts::TAU / DRIFT_PERIOD + sprite.phase).sin()
            * DRIFT_UNITS;
        let position = sprite.base + drift;
        let visible = sprite.province != selected
            && if sprite.fair {
                fair_alpha > 0.004
            } else if sprite.shadow {
                shadow_alpha > 0.004
            } else {
                cloud_alpha > 0.01 && survives_cut(sprite.kind, sprite.intensity, cut)
            };
        let want = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        if sprite.fair {
            stats.fair_shadows += 1;
            stats.visible_fair += usize::from(visible);
        } else if sprite.shadow {
            stats.shadows += 1;
            stats.visible_shadows += usize::from(visible);
        } else {
            stats.clouds += 1;
            stats.visible_clouds += usize::from(visible);
        }
        if !visible {
            continue;
        }
        if sprite.shadow {
            if (sprite.ground_for_scale - scale).abs() > scale.abs() * 0.005 + 1e-6
                && ground.is_ready()
            {
                let half = sprite.size * 0.5;
                let mut high = 0.0f32;
                for (dx, dz) in [
                    (0.0, 0.0),
                    (half, 0.0),
                    (-half, 0.0),
                    (0.0, half),
                    (0.0, -half),
                ] {
                    high = high.max(
                        ground
                            .at(sprite.base.x + dx, sprite.base.y + dz)
                            .unwrap_or(0.0),
                    );
                }
                sprite.ground_y = high;
                sprite.ground_for_scale = scale;
            }
            let lift = (tuning.clouds.height - sprite.ground_y).max(0.0);
            let shifted = position - slope * lift;
            transform.translation = Vec3::new(shifted.x, sprite.ground_y + SHADOW_LIFT, shifted.y);
        } else {
            transform.translation = Vec3::new(position.x, tuning.clouds.height, position.y);
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct CloudStats {
    pub clouds: usize,
    pub shadows: usize,
    pub visible_clouds: usize,
    pub visible_shadows: usize,
    pub cloud_alpha: f32,
    pub shadow_alpha: f32,
    pub fair_alpha: f32,
    pub fair_shadows: usize,
    pub visible_fair: usize,
}

pub fn dump(world: &World) -> Value {
    let s = world.resource::<CloudStats>();
    json!({
        "clouds": s.clouds, "visible_clouds": s.visible_clouds,
        "shadows": s.shadows, "visible_shadows": s.visible_shadows,
        "cloud_alpha": s.cloud_alpha, "shadow_alpha": s.shadow_alpha,
        "fair_shadows": s.fair_shadows, "visible_fair_shadows": s.visible_fair,
        "fair_alpha": s.fair_alpha,
    })
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<CloudStats>()
        .add_systems(Startup, setup_assets)
        .add_systems(
            Update,
            (sync_clouds, update_clouds)
                .chain()
                .after(super::update_view)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_depend_on_kind_and_intensity() {
        assert_eq!(cloud_count(Kind::Clear, 1.0), 0);
        assert_eq!(cloud_count(Kind::Fog, 1.0), 0);
        assert_eq!(cloud_count(Kind::Rain, 0.3), 2);
        assert_eq!(cloud_count(Kind::Storm, 0.9), 4);
    }

    #[test]
    fn layout_is_deterministic_and_bounded() {
        let a = layout(12, 0, 4000.0);
        assert_eq!(a, layout(12, 0, 4000.0));
        assert!(a.1 >= 48.0 && a.1 <= 240.0);
        assert!(a.0.length() < 4000f32.sqrt());
        assert_ne!(layout(12, 1, 4000.0), a);
    }

    #[test]
    fn blob_is_soft_and_transparent_at_the_edge() {
        let pixels = blob_pixels(32);
        assert_eq!(pixels.len(), 32 * 32 * 4);
        let alpha = |x: usize, y: usize| pixels[(y * 32 + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0);
        assert!(alpha(16, 16) > 100);
    }

    #[test]
    fn fair_shadow_fades_with_distance_and_parchment() {
        assert!((fair_alpha(100.0, 0.0) - FAIR_SHADOW_ALPHA).abs() < 1e-6);
        assert!(fair_alpha(400.0, 0.0) < fair_alpha(100.0, 0.0));
        assert!(fair_alpha(400.0, 0.0) > 0.0);
        assert_eq!(fair_alpha(1500.0, 0.0), 0.0);
        assert_eq!(fair_alpha(100.0, 1.0), 0.0);
    }

    #[test]
    fn alpha_and_cut() {
        let t = CloudTuning::default();
        assert_eq!(alphas(&t, 50.0).0, 0.0);
        assert!(alphas(&t, 50.0).1 > 0.1);
        assert!(alphas(&t, 5000.0).1 < 1e-6);
        assert!(survives_cut(Kind::Storm, 0.1, 0.5));
        assert!(!survives_cut(Kind::Rain, 0.3, 0.5));
    }
}
