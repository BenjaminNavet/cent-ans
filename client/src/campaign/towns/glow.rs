//! Warm halo and chimney smoke of the towns of the campaign map (lot TS-T4, ADR 0710): in Total
//! War towns and fortresses are told from afar by the orange glow of their fires and by the
//! smoke rising over them, without any nameplate.
//!
//! - Halo: one additive, unlit, horizontal disc per place (not the ones swallowed by a landmark
//!   maquette), radius proportional to the maquette radius, laid on the displayed ground. One
//!   shared material carries the opacity: season (long winter nights) x distance fade x
//!   `1 - strategic` (gone in the parchment view, ADR 0124) x a slow flicker.
//! - Plumes: `FlipbookEmitter::smoke` (the player's smoke sheet) at a few chimneys per place,
//!   spawned only near the aimed point, under a budget (`max_active` x quality factor), nearest
//!   first. Their size and rise grow with the camera distance so they stay readable.
//!
//! Settings: `data/fx/town_glow.json` (schema `fx_town_glow`). No window emissive on the maquettes:
//! their models share one baked atlas, so the halo alone stands for the lit windows.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde_json::{Value, json};

use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::markers::placement::stable_hash;
use crate::campaign::markers::places::Places;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::season::SeasonWeights;
use crate::campaign::terrain::surface::GroundHeight;
use crate::fx::flipbook::{EmitterSpec, FlipbookData, FlipbookEmitter};
use crate::settings::quality::RenderQuality;

/// Settings of `data/fx/town_glow.json`.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct GlowConfig {
    pub enabled: bool,
    pub color: [f32; 3],
    pub radius_per_unit: f32,
    pub radius_min: f32,
    pub radius_max: f32,
    pub season_alpha: [f32; 4],
    pub distance_in: [f32; 2],
    pub distance_out: [f32; 2],
    pub lift: f32,
    pub flicker: f32,
    pub flicker_hz: f32,
    pub settle_period_s: f32,
    pub units_per_plume: f32,
    pub min_per_place: usize,
    pub max_per_place: usize,
    pub max_distance: f32,
    pub max_active: usize,
    pub update_period_s: f32,
    pub smoke_color: [f32; 4],
    pub life: f32,
    pub amount: f32,
    pub size_per_unit: [f32; 2],
    pub size_min: [f32; 2],
    pub rise_per_unit: f32,
    pub rise_min: f32,
    pub distance_gain: f32,
    pub distance_ref: f32,
    pub distance_gain_max: f32,
    pub chimney_height: f32,
    pub quality_factor: HashMap<String, f32>,
}

impl Default for GlowConfig {
    fn default() -> Self {
        GlowConfig::from_json(&Value::Null)
    }
}

fn num(v: &Value, fallback: f32) -> f32 {
    v.as_f64().map_or(fallback, |x| x as f32)
}

fn array<const N: usize>(v: &Value, fallback: [f32; N]) -> [f32; N] {
    let mut out = fallback;
    if let Some(items) = v.as_array() {
        for (slot, item) in out.iter_mut().zip(items) {
            *slot = num(item, *slot);
        }
    }
    out
}

impl GlowConfig {
    /// Reads the settings; every missing value falls back to the shipped default.
    pub fn from_json(doc: &Value) -> GlowConfig {
        let (h, p) = (&doc["halo"], &doc["plumes"]);
        let mut quality_factor: HashMap<String, f32> = [
            ("low", 0.35),
            ("medium", 0.7),
            ("high", 1.0),
            ("ultra", 1.0),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
        if let Some(map) = doc["quality_factor"].as_object() {
            for (k, v) in map {
                quality_factor.insert(k.clone(), num(v, 1.0));
            }
        }
        GlowConfig {
            enabled: doc["enabled"].as_bool().unwrap_or(true),
            color: array(&h["color"], [1.0, 0.55, 0.2]),
            radius_per_unit: num(&h["radius_per_unit"], 2.4),
            radius_min: num(&h["radius_min"], 1.6),
            radius_max: num(&h["radius_max"], 16.0),
            season_alpha: array(&h["season_alpha"], [0.16, 0.12, 0.2, 0.3]),
            distance_in: array(&h["distance_in"], [25.0, 70.0]),
            distance_out: array(&h["distance_out"], [560.0, 950.0]),
            lift: num(&h["lift"], 0.15),
            flicker: num(&h["flicker"], 0.08),
            flicker_hz: num(&h["flicker_hz"], 0.45),
            settle_period_s: num(&h["settle_period_s"], 0.5),
            units_per_plume: num(&p["units_per_plume"], 1.4).max(1e-3),
            min_per_place: p["min_per_place"].as_u64().unwrap_or(1) as usize,
            max_per_place: p["max_per_place"].as_u64().unwrap_or(5) as usize,
            max_distance: num(&p["max_distance"], 520.0),
            max_active: p["max_active"].as_u64().unwrap_or(36) as usize,
            update_period_s: num(&p["update_period_s"], 0.4),
            smoke_color: array(&p["color"], [0.6, 0.57, 0.54, 0.5]),
            life: num(&p["life"], 7.0),
            amount: num(&p["amount"], 5.0),
            size_per_unit: array(&p["size_per_unit"], [0.12, 0.3]),
            size_min: array(&p["size_min"], [0.35, 0.8]),
            rise_per_unit: num(&p["rise_per_unit"], 0.14),
            rise_min: num(&p["rise_min"], 0.25),
            distance_gain: num(&p["distance_gain"], 0.35),
            distance_ref: num(&p["distance_ref"], 150.0).max(1.0),
            distance_gain_max: num(&p["distance_gain_max"], 3.0),
            chimney_height: num(&p["chimney_height"], 0.5),
            quality_factor,
        }
    }

    pub fn load() -> GlowConfig {
        let doc = crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/fx/town_glow.json")).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(Value::Null);
        GlowConfig::from_json(&doc)
    }

    /// Radius of the halo (world units) of a place whose maquette has radius `place_radius`.
    pub fn halo_radius(&self, place_radius: f32) -> f32 {
        (place_radius * self.radius_per_unit).clamp(self.radius_min, self.radius_max)
    }

    /// Number of chimney plumes of a place of radius `place_radius`.
    pub fn plume_count(&self, place_radius: f32) -> usize {
        ((place_radius / self.units_per_plume).round() as usize).clamp(
            self.min_per_place,
            self.max_per_place.max(self.min_per_place),
        )
    }

    /// Halo opacity from the season weights (spring, summer, autumn, winter).
    pub fn season_opacity(&self, weights: Vec4) -> f32 {
        (0..4).map(|i| self.season_alpha[i] * weights[i]).sum()
    }

    /// Fade of the halo with the camera distance: in, then out, 0 at the parchment.
    pub fn distance_fade(&self, distance: f32) -> f32 {
        smooth(self.distance_in[0], self.distance_in[1], distance)
            * (1.0 - smooth(self.distance_out[0], self.distance_out[1], distance))
    }

    /// Plume budget at a quality level.
    pub fn plume_budget(&self, level: &str) -> usize {
        let factor = self.quality_factor.get(level).copied().unwrap_or(1.0);
        (self.max_active as f32 * factor).round() as usize
    }

    /// Emitter settings (world units) of a plume of a place of radius `place_radius`, seen from
    /// `distance` (bigger and faster far away so the column stays readable).
    pub fn plume_spec(&self, place_radius: f32, distance: f32) -> EmitterSpec {
        let k =
            1.0 + (distance / self.distance_ref * self.distance_gain).min(self.distance_gain_max);
        let size = [0, 1].map(|i| (self.size_per_unit[i] * place_radius).max(self.size_min[i]) * k);
        let rise = (self.rise_per_unit * place_radius).max(self.rise_min) * k;
        EmitterSpec {
            amount: self.amount,
            life: self.life,
            size,
            velocity: [rise * 0.8, rise * 1.2],
            spread_m: 0.1 * size[0],
            loops: 0.0,
        }
    }
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A halo disc of place `place`.
#[derive(Component)]
pub struct TownHalo {
    place: usize,
    radius: f32,
}

/// A chimney plume of place `place`.
#[derive(Component)]
pub struct TownPlume {
    place: usize,
    radius: f32,
}

#[derive(Resource)]
struct GlowAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

/// Runtime state and counters (dump).
#[derive(Resource, Default)]
struct GlowState {
    built: bool,
    last_settle: f32,
    relief_sig: [f32; 3],
    last_plumes: f32,
    opacity: f32,
    halos_visible: usize,
    plumes: HashMap<usize, Vec<Entity>>,
}

pub fn add(app: &mut App) {
    app.insert_resource(GlowConfig::load())
        .init_resource::<GlowState>()
        .add_systems(OnExit(AppState::Campaign), reset_glow)
        .add_systems(
            Update,
            (
                build_halos,
                fade_halos,
                settle_halos,
                stream_plumes,
                update_plumes,
            )
                .chain()
                .run_if(in_state(AppState::Campaign))
                .run_if(resource_exists::<Places>),
        );
}

/// Radial white disc, alpha falling smoothly to 0 at the edge.
fn disc_image(side: u32) -> Image {
    let mut data = Vec::with_capacity((side * side * 4) as usize);
    let half = side as f32 * 0.5;
    for y in 0..side {
        for x in 0..side {
            let d =
                (((x as f32 + 0.5 - half).powi(2) + (y as f32 + 0.5 - half).powi(2)).sqrt()) / half;
            let a = (1.0 - d).clamp(0.0, 1.0);
            let a = a * a * (3.0 - 2.0 * a);
            data.extend_from_slice(&[255, 255, 255, (a * a.sqrt() * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn reset_glow(
    mut commands: Commands,
    mut state: ResMut<GlowState>,
    halos: Query<Entity, With<TownHalo>>,
    plumes: Query<Entity, With<TownPlume>>,
) {
    for e in halos.iter().chain(plumes.iter()) {
        commands.entity(e).despawn();
    }
    *state = GlowState::default();
}

fn build_halos(
    mut commands: Commands,
    config: Res<GlowConfig>,
    places: Res<Places>,
    mut state: ResMut<GlowState>,
    ground: GroundHeight,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if state.built || !config.enabled || places.places.is_empty() || !ground.is_ready() {
        return;
    }
    state.built = true;
    let [r, g, b] = config.color;
    let assets = GlowAssets {
        mesh: meshes.add(Plane3d::default().mesh().size(2.0, 2.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 0.0),
            base_color_texture: Some(images.add(disc_image(64))),
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: None,
            double_sided: true,
            depth_bias: 2.0,
            ..default()
        }),
    };
    for (i, place) in places.places.iter().enumerate() {
        if place.absorbed {
            continue;
        }
        let radius = config.halo_radius(place.radius);
        let y = ground.at(place.px.x, place.px.y).unwrap_or(0.0) + config.lift;
        commands.spawn((
            Name::new("TownHalo"),
            TownHalo { place: i, radius },
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_xyz(place.px.x, y, place.px.y)
                .with_scale(Vec3::new(radius, 1.0, radius)),
            Visibility::Hidden,
            NotShadowCaster,
        ));
    }
    commands.insert_resource(assets);
}

/// Opacity of the shared material and visibility of the discs.
fn fade_halos(
    time: Res<Time>,
    config: Res<GlowConfig>,
    view: Res<ZoomView>,
    seasons: Option<Res<SeasonWeights>>,
    assets: Option<Res<GlowAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<GlowState>,
    mut halos: Query<&mut Visibility, With<TownHalo>>,
) {
    let Some(assets) = assets else { return };
    let weights = seasons.map_or(Vec4::new(1.0, 0.0, 0.0, 0.0), |s| s.weights);
    let flicker = 1.0
        + config.flicker * (time.elapsed_secs() * config.flicker_hz * std::f32::consts::TAU).sin();
    let opacity = config.season_opacity(weights)
        * config.distance_fade(view.distance)
        * (1.0 - view.strategic)
        * flicker;
    state.opacity = opacity;
    if let Some(mut material) = materials.get_mut(&assets.material) {
        let [r, g, b] = config.color;
        let color = Color::srgba(r, g, b, opacity.clamp(0.0, 1.0));
        if material.base_color != color {
            material.base_color = color;
        }
    }
    let want = if opacity > 0.002 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let mut visible = 0;
    for mut vis in &mut halos {
        if *vis != want {
            *vis = want;
        }
        visible += usize::from(want == Visibility::Inherited);
    }
    state.halos_visible = visible;
}

/// Re-seats the discs on the ground when the relief exaggeration (zoom) changes.
fn settle_halos(
    time: Res<Time>,
    config: Res<GlowConfig>,
    places: Res<Places>,
    relief: Option<Res<ReliefState>>,
    ground: GroundHeight,
    mut state: ResMut<GlowState>,
    mut halos: Query<(&TownHalo, &mut Transform)>,
) {
    let Some(relief) = relief else { return };
    if !ground.is_ready() || !state.built || state.opacity <= 0.002 {
        return;
    }
    let sig = [relief.vertical_scale, relief.gain, relief.squash];
    let now = time.elapsed_secs();
    if sig == state.relief_sig || now - state.last_settle < config.settle_period_s {
        return;
    }
    state.relief_sig = sig;
    state.last_settle = now;
    for (halo, mut at) in &mut halos {
        let px = places.places[halo.place].px;
        // Highest of the centre and four points of the rim, so a slope does not bury the disc.
        let reach = halo.radius * 0.5;
        let y = [
            (0.0, 0.0),
            (reach, 0.0),
            (-reach, 0.0),
            (0.0, reach),
            (0.0, -reach),
        ]
        .into_iter()
        .filter_map(|(dx, dz)| ground.at(px.x + dx, px.y + dz))
        .fold(0.0_f32, f32::max)
            + config.lift;
        if (at.translation.y - y).abs() > 1e-3 {
            at.translation.y = y;
        }
    }
}

/// Spawns the plumes of the nearest places to the aimed point (within the budget) and removes
/// the ones that fell out of range.
fn stream_plumes(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<GlowConfig>,
    places: Res<Places>,
    rig: Option<Res<CampaignRig>>,
    quality: Option<Res<RenderQuality>>,
    ground: GroundHeight,
    mut state: ResMut<GlowState>,
) {
    let Some(rig) = rig else { return };
    let now = time.elapsed_secs();
    if !config.enabled || !ground.is_ready() || now - state.last_plumes < config.update_period_s {
        return;
    }
    state.last_plumes = now;
    let level = quality.as_ref().map_or("high", |q| q.level.as_str());
    let budget = config.plume_budget(level);
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    // Places around the aimed point: the visible ground spreads with the camera distance.
    let reach = (rig.distance * 0.9).max(60.0);
    let mut wanted: Vec<(f32, usize)> = Vec::new();
    if rig.distance <= config.max_distance {
        wanted = places
            .places
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.absorbed)
            .map(|(i, p)| (p.px.distance(focus), i))
            .filter(|(d, _)| *d <= reach)
            .collect();
        wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    let mut keep: HashMap<usize, usize> = HashMap::new();
    let mut used = 0;
    for (_, i) in wanted {
        let count = config.plume_count(places.places[i].radius);
        if used + count > budget {
            continue;
        }
        used += count;
        keep.insert(i, count);
    }
    let gone: Vec<usize> = state
        .plumes
        .keys()
        .filter(|i| !keep.contains_key(i))
        .copied()
        .collect();
    for i in gone {
        for e in state.plumes.remove(&i).unwrap_or_default() {
            commands.entity(e).despawn();
        }
    }
    for (i, count) in keep {
        if state.plumes.contains_key(&i) {
            continue;
        }
        let place = &places.places[i];
        let base = ground.at(place.px.x, place.px.y).unwrap_or(0.0);
        let seed = stable_hash(&place.id);
        let entities = (0..count)
            .map(|k| {
                let h = seed
                    .wrapping_add(k as u32 * 7919)
                    .wrapping_mul(2_654_435_761);
                let angle = (h & 0xffff) as f32 / 65535.0 * std::f32::consts::TAU;
                let ring = 0.15 + 0.55 * ((h >> 16) & 0xff) as f32 / 255.0;
                let offset = Vec2::from_angle(angle) * place.radius * ring;
                let spec = config.plume_spec(place.radius, 0.0);
                let mut emitter = FlipbookEmitter::smoke(&FlipbookData::default(), place.radius);
                emitter.spec = spec;
                emitter.accel = Vec3::new(0.35, 0.25, 0.1) * place.radius.max(0.5);
                emitter.damping = [0.1, 0.2];
                emitter.tint = Some(config.smoke_color);
                commands
                    .spawn((
                        Name::new("TownPlume"),
                        TownPlume {
                            place: i,
                            radius: place.radius,
                        },
                        emitter,
                        Transform::from_xyz(
                            place.px.x + offset.x,
                            base + config.chimney_height.max(place.radius * 0.25),
                            place.px.y + offset.y,
                        ),
                        Visibility::Inherited,
                    ))
                    .id()
            })
            .collect();
        state.plumes.insert(i, entities);
    }
}

/// Rescales the live plumes to the camera distance and keeps them on the ground.
fn update_plumes(
    config: Res<GlowConfig>,
    view: Res<ZoomView>,
    places: Res<Places>,
    ground: GroundHeight,
    mut plumes: Query<(&TownPlume, &mut FlipbookEmitter, &mut Transform)>,
) {
    for (plume, mut emitter, mut at) in &mut plumes {
        let spec = config.plume_spec(plume.radius, view.distance);
        emitter.spec.size = spec.size;
        emitter.spec.velocity = spec.velocity;
        emitter.spec.spread_m = spec.spread_m;
        let px = places.places[plume.place].px;
        if let Some(y) = ground.at(px.x, px.y) {
            let top = y + config.chimney_height.max(plume.radius * 0.25);
            if (at.translation.y - top).abs() > 1e-2 {
                at.translation.y = top;
            }
        }
    }
}

/// Counters for the `campaign_towns` dump (`glow` key).
pub fn glow_summary(world: &World) -> Value {
    let Some(state) = world.get_resource::<GlowState>() else {
        return json!({"ready": false});
    };
    let halos = world
        .try_query::<&TownHalo>()
        .map_or(0, |mut q| q.iter(world).count());
    let (mut plumes, mut active) = (0, 0);
    if let Some(mut q) = world.try_query::<(&TownPlume, &FlipbookEmitter)>() {
        for (_, e) in q.iter(world) {
            plumes += 1;
            active += usize::from(e.active);
        }
    }
    json!({
        "ready": state.built,
        "halos": halos,
        "halos_visible": state.halos_visible,
        "opacity": state.opacity,
        "plumes": plumes,
        "plumes_active": active,
        "places_with_plumes": state.plumes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halo_radius_grows_with_the_town_and_is_clamped() {
        let c = GlowConfig::default();
        assert_eq!(c.halo_radius(0.0), c.radius_min);
        assert!(c.halo_radius(3.0) > c.halo_radius(1.5));
        assert_eq!(c.halo_radius(1000.0), c.radius_max);
    }

    #[test]
    fn plume_count_follows_size_within_bounds() {
        let c = GlowConfig::default();
        assert_eq!(c.plume_count(0.1), c.min_per_place);
        assert!(c.plume_count(4.0) >= c.plume_count(2.0));
        assert_eq!(c.plume_count(1000.0), c.max_per_place);
    }

    #[test]
    fn winter_glows_more_than_summer() {
        let c = GlowConfig::default();
        let summer = c.season_opacity(Vec4::new(0.0, 1.0, 0.0, 0.0));
        let winter = c.season_opacity(Vec4::new(0.0, 0.0, 0.0, 1.0));
        assert!(winter > summer && summer > 0.0);
    }

    #[test]
    fn halo_is_gone_up_close_and_at_the_parchment() {
        let c = GlowConfig::default();
        assert_eq!(c.distance_fade(5.0), 0.0);
        assert!(c.distance_fade(200.0) > 0.99);
        assert_eq!(c.distance_fade(1200.0), 0.0);
    }

    #[test]
    fn plume_budget_follows_quality() {
        let c = GlowConfig::default();
        assert!(c.plume_budget("low") < c.plume_budget("medium"));
        assert!(c.plume_budget("medium") < c.plume_budget("high"));
    }

    #[test]
    fn plumes_grow_with_camera_distance() {
        let c = GlowConfig::default();
        let near = c.plume_spec(2.0, 40.0);
        let far = c.plume_spec(2.0, 400.0);
        assert!(far.size[1] > near.size[1] && far.velocity[1] > near.velocity[1]);
    }
}
