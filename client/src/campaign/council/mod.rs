//! The 3D council hall (ports `game/scripts/ui/council_room.gd`, the stage of
//! `council_chamber.gd` and the entry points of `map/council_controller.gd`).
//!
//! The room is a small scene on its own render layer ([`ROOM_LAYER`], so the campaign sun and
//! cameras never see it), drawn by a fixed camera into an offscreen image. The image sits at the
//! top of the existing council panel (`panels::court`, tab « Charges »): hovering a seated
//! councillor lights him up and shows a card, a click selects his office in the panel (the sheet,
//! candidates, advice, appoint / dismiss are the panel's own widgets); a click on the sovereign
//! opens his character sheet. Vacant offices show an empty stool and a ghost label.
//!
//! Entry from the campaign (unchanged, already in the HUD): the « Conseil » button, the
//! `map_toggle_council` key and the `council_vacancy` alert all send `OpenPanel::Council`.
//!
//! Figures follow the Godot default style `painted` (cut-out illustrations in
//! `illustrations/council/` on upright quads, hand-made furniture glb in `models/council/handmade/`);
//! `CENT_ANS_COUNCIL_STYLE=mesh|mesh_multiview` (or `figure_style` of `data/art/council_catalog.json`)
//! selects the image-to-3D figure glb instead, as `--council-style=` did.

mod figures;
mod pick;
mod room;
mod stage;

use bevy::app::HierarchyPropagatePlugin;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::AmbientLight;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::panels::court::{CourtUi, Screen};
use crate::sim::Sim;

pub use stage::{STAGE_SIZE, spawn_stage};

/// Width of the left column of the offices tab (the stage's width).
pub const STAGE_WIDTH: f32 = STAGE_SIZE.x;

/// Render layer of the room (layer 0 is the campaign).
pub const ROOM_LAYER: usize = 7;
/// Size of the offscreen image (`SubViewport.size` of the Godot chamber).
pub const STAGE_PIXELS: UVec2 = UVec2::new(760, 480);
/// Light powers (lumens); Godot's `light_energy` has no physical unit, tuned on pixel statistics.
pub const CANDLE_LUMENS: f32 = 360_000.0;
pub const EMBER_LUMENS: f32 = 240_000.0;
pub const WINDOW_LUMENS: f32 = 2_400_000.0;
pub const SWAY_AMPLITUDE: f32 = 0.05;

pub struct CouncilRoomPlugin;

impl Plugin for CouncilRoomPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<HierarchyPropagatePlugin<RenderLayers>>() {
            app.add_plugins(HierarchyPropagatePlugin::<RenderLayers>::new(PostUpdate));
        }
        app.init_resource::<CouncilState>()
            .init_resource::<ProbeResult>()
            .init_resource::<CouncilProps>()
            .register_dump(dump_council)
            .add_systems(Startup, make_stage_image)
            .add_systems(
                Update,
                (
                    ensure_room,
                    figures::populate,
                    figures::fit_painted,
                    stage::input,
                    figures::apply_hover,
                    figures::apply_overlay,
                    animate,
                    stage::update_overlays,
                    dev_probe,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

/// Root of the room scene (decor, figures, camera), built once per campaign and faction.
#[derive(Component)]
pub struct CouncilRoomRoot {
    pub faction: String,
}

/// The room camera; active only while the council screen is open.
#[derive(Component)]
pub struct CouncilCamera;

/// Parent of the figures and seat furniture, rebuilt when the council changes.
#[derive(Component)]
pub struct FiguresRoot;

/// A glb of the room, for the dump (`role`: table, throne, stool, figure).
#[derive(Component)]
pub struct CouncilModel {
    pub role: String,
    pub path: String,
    pub scene: Handle<bevy::world_serialization::WorldAsset>,
}

/// A candle flame light (index = phase of the flicker).
#[derive(Component)]
pub struct CandleFlame {
    pub index: usize,
}

/// Counters of what the decor contains (dump).
#[derive(Resource, Default)]
pub struct CouncilProps {
    pub meshes: usize,
    pub lights: usize,
    pub candles: usize,
    pub tapestries: usize,
    pub halos: usize,
    pub emblem: bool,
}

/// Which figure rendering is used (`STYLES` of the Godot room).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Style {
    #[default]
    Painted,
    Mesh,
    MeshMultiview,
}

impl Style {
    pub fn name(self) -> &'static str {
        match self {
            Style::Painted => "painted",
            Style::Mesh => "mesh",
            Style::MeshMultiview => "mesh_multiview",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "painted" => Some(Style::Painted),
            "mesh" => Some(Style::Mesh),
            "mesh_multiview" => Some(Style::MeshMultiview),
            _ => None,
        }
    }

    /// Environment variable, else `figure_style` of the catalogue, else `painted`.
    fn resolve(root: Option<&std::path::Path>) -> Self {
        if let Some(style) = std::env::var("CENT_ANS_COUNCIL_STYLE")
            .ok()
            .and_then(|v| Self::parse(&v))
        {
            return style;
        }
        root.and_then(|root| {
            let text = std::fs::read_to_string(root.join("data/art/council_catalog.json")).ok()?;
            let value: Value = serde_json::from_str(&text).ok()?;
            Self::parse(value.get("figure_style")?.as_str()?)
        })
        .unwrap_or_default()
    }
}

#[derive(Resource)]
pub struct CouncilState {
    pub image: Handle<Image>,
    pub style: Style,
    /// Office id (or `ruler`) under the cursor.
    pub hovered: Option<String>,
    /// Occupancy of the seats the figures were built for.
    pub signature: Vec<(String, bool)>,
    pub time: f32,
    pub hovers: u32,
    pub clicks: u32,
    pub last_click: Option<String>,
    pub populated: u32,
    pub asset_root: Option<std::path::PathBuf>,
    /// Cursor over the stage image, in displayed pixels.
    pub cursor: Option<Vec2>,
    pub card: String,
    /// Asset path of the portrait shown in the card (empty: none).
    pub card_portrait: String,
    /// Colour of the player's faction (the sovereign's stand-in figure).
    pub faction_color: Color,
}

impl Default for CouncilState {
    fn default() -> Self {
        let root = crate::paths::find_root().ok();
        Self {
            image: Handle::default(),
            style: Style::resolve(root.as_deref()),
            hovered: None,
            signature: Vec::new(),
            time: 0.0,
            hovers: 0,
            clicks: 0,
            last_click: None,
            populated: 0,
            asset_root: root.map(|r| crate::paths::asset_root(&r)),
            cursor: None,
            card: String::new(),
            card_portrait: String::new(),
            faction_color: Color::srgb(0.6, 0.15, 0.15),
        }
    }
}

fn make_stage_image(mut state: ResMut<CouncilState>, mut images: ResMut<Assets<Image>>) {
    state.image = images.add(Image::new_target_texture(
        STAGE_PIXELS.x,
        STAGE_PIXELS.y,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
}

/// Result of the dev pixel probe (`CENT_ANS_COUNCIL_PROBE=1`), shown in the dump.
#[derive(Resource, Default, Clone)]
pub struct ProbeResult(pub std::sync::Arc<std::sync::Mutex<Option<Value>>>);

/// Dev probe: once the council has been open for 120 frames, reads the room image back and stores
/// pixel statistics (mean / percentiles of the luminance, black and blown shares, mean per seat
/// around the head) in the dump; with `CENT_ANS_COUNCIL_PROBE=<path>` also writes the PNG.
fn dev_probe(
    mut commands: Commands,
    ui: Res<CourtUi>,
    state: Res<CouncilState>,
    result: Res<ProbeResult>,
    mut frames: Local<u32>,
    mut done: Local<bool>,
) {
    let Ok(request) = std::env::var("CENT_ANS_COUNCIL_PROBE") else {
        return;
    };
    if *done || !council_open(&ui) {
        return;
    }
    *frames += 1;
    if *frames < 120 {
        return;
    }
    *done = true;
    let slot = result.0.clone();
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::image(
            state.image.clone(),
        ))
        .observe(
            move |captured: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                let Ok(image) = captured.image.clone().try_into_dynamic() else {
                    return;
                };
                if request != "1" {
                    let _ = image.to_rgb8().save(&request);
                }
                let luma = image.to_luma8();
                let (w, h) = (luma.width() as usize, luma.height() as usize);
                let mut values: Vec<u8> = luma.as_raw().clone();
                values.sort_unstable();
                let pick = |q: f32| values[((values.len() - 1) as f32 * q) as usize];
                let mean = values.iter().map(|v| f64::from(*v)).sum::<f64>() / values.len() as f64;
                let black = values.iter().filter(|v| **v < 8).count() as f64 / values.len() as f64;
                let blown =
                    values.iter().filter(|v| **v > 247).count() as f64 / values.len() as f64;
                let rotation = Transform::from_translation(pick::CAMERA_POSITION)
                    .looking_at(pick::CAMERA_TARGET, Vec3::Y)
                    .rotation;
                let aspect = w as f32 / h as f32;
                let seats: Vec<Value> = pick::seat_keys()
                    .map(|key| {
                        let head = pick::seat_position(key) + Vec3::Y * 1.0;
                        let at = pick::project(
                            head,
                            pick::CAMERA_POSITION,
                            rotation,
                            pick::CAMERA_FOV_DEG,
                            aspect,
                        );
                        let mean_here = at.map(|uv| {
                            let (cx, cy) = ((uv.x * w as f32) as i32, (uv.y * h as f32) as i32);
                            let mut sum = 0.0;
                            let mut n = 0.0;
                            for y in (cy - 20).max(0)..(cy + 20).min(h as i32) {
                                for x in (cx - 20).max(0)..(cx + 20).min(w as i32) {
                                    sum += f64::from(luma.get_pixel(x as u32, y as u32).0[0]);
                                    n += 1.0;
                                }
                            }
                            if n > 0.0 { sum / n } else { 0.0 }
                        });
                        json!({"seat": key, "pixel_mean_near_head": mean_here})
                    })
                    .collect();
                if let Ok(mut slot) = slot.lock() {
                    *slot = Some(json!({
                        "size": [w, h], "mean": mean,
                        "p5": pick(0.05), "p50": pick(0.5), "p95": pick(0.95), "p99": pick(0.99),
                        "black_share": black, "blown_share": blown, "seats": seats,
                    }));
                }
            },
        );
}

fn council_open(ui: &CourtUi) -> bool {
    ui.screen == Screen::Council
}

/// Builds the room each time the council opens (and again when the player's faction changes,
/// after loading a save), tears it down when the screen closes; the camera is on only while the
/// screen is open.
#[allow(clippy::too_many_arguments)]
fn ensure_room(
    mut commands: Commands,
    ui: Res<CourtUi>,
    sim: Res<Sim>,
    mut state: ResMut<CouncilState>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut props: ResMut<CouncilProps>,
    roots: Query<(Entity, &CouncilRoomRoot)>,
    mut cameras: Query<&mut Camera, With<CouncilCamera>>,
) {
    let player = sim.player.as_str().to_owned();
    let wanted = council_open(&ui);
    for mut camera in &mut cameras {
        if camera.is_active != wanted {
            camera.is_active = wanted;
        }
    }
    if !council_open(&ui) {
        // The room is torn down with the screen: its candle and spot lights (shadow maps on) are
        // extracted and rendered whatever the camera and the visibility say, which doubled the
        // campaign frame time after the first visit. It is rebuilt on the next opening.
        for (entity, _) in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }
    let stale = roots.iter().any(|(_, root)| root.faction != player);
    if !stale && !roots.is_empty() {
        return;
    }
    for (entity, _) in &roots {
        commands.entity(entity).despawn();
    }
    let Some(data) = &sim.data else {
        return;
    };
    let faction_color = data
        .factions
        .get(&sim.player)
        .map_or(Color::srgb(0.6, 0.15, 0.15), |f| {
            let c = hex_color(&f.heraldry.primary_color);
            Color::srgb(c[0], c[1], c[2])
        });
    state.faction_color = faction_color;
    *props = CouncilProps::default();
    let root = commands
        .spawn((
            Name::new("CouncilRoom"),
            CouncilRoomRoot {
                faction: player.clone(),
            },
            Transform::default(),
            Visibility::default(),
            bevy::app::Propagate(RenderLayers::layer(ROOM_LAYER)),
            DespawnOnExit(AppState::Campaign),
        ))
        .id();
    commands.spawn((
        Name::new("FiguresRoot"),
        FiguresRoot,
        Transform::default(),
        Visibility::default(),
        ChildOf(root),
    ));
    let mut builder = room::Builder {
        commands: &mut commands,
        meshes: &mut meshes,
        materials: &mut materials,
        assets: &assets,
        root,
        asset_root: state.asset_root.clone(),
        style: state.style,
        props: &mut props,
    };
    builder.build(faction_color, &player);
    commands.spawn((
        Name::new("CouncilCamera"),
        CouncilCamera,
        Camera3d::default(),
        Camera {
            order: -5,
            is_active: true,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.018, 0.016)),
            ..default()
        },
        RenderTarget::Image(state.image.clone().into()),
        Projection::Perspective(PerspectiveProjection {
            fov: pick::CAMERA_FOV_DEG.to_radians(),
            near: 0.05,
            far: 60.0,
            ..default()
        }),
        Tonemapping::TonyMcMapface,
        Msaa::Sample4,
        AmbientLight {
            color: Color::srgb(0.16, 0.14, 0.13),
            brightness: AMBIENT_CD,
            ..default()
        },
        // Low warm haze, cheap (`environment.fog_density = 0.035`).
        DistanceFog {
            color: Color::srgb(0.16, 0.12, 0.09),
            falloff: FogFalloff::Exponential { density: 0.035 },
            ..default()
        },
        Transform::from_translation(pick::CAMERA_POSITION).looking_at(pick::CAMERA_TARGET, Vec3::Y),
        ChildOf(root),
    ));
}

/// Ambient light of the room (cd/m²), tuned on pixel statistics.
pub const AMBIENT_CD: f32 = 2800.0;

/// Gentle camera sway and candle flicker (`_process`), still when motion is reduced.
fn animate(
    time: Res<Time>,
    ui: Res<CourtUi>,
    reduce: Option<Res<crate::ui::kit::modal::ReduceMotion>>,
    mut state: ResMut<CouncilState>,
    mut cameras: Query<&mut Transform, With<CouncilCamera>>,
    mut candles: Query<(&CandleFlame, &mut PointLight)>,
) {
    if !council_open(&ui) {
        return;
    }
    if reduce.is_some_and(|r| r.0) {
        return;
    }
    state.time += time.delta_secs();
    let t = state.time;
    for mut transform in &mut cameras {
        let offset = Vec3::new((t * 0.21).sin(), (t * 0.33).sin() * 0.5, 0.0) * SWAY_AMPLITUDE;
        *transform = Transform::from_translation(pick::CAMERA_POSITION + offset)
            .looking_at(pick::CAMERA_TARGET, Vec3::Y);
    }
    for (flame, mut light) in &mut candles {
        let index = flame.index as f32;
        light.intensity = CANDLE_LUMENS * (1.0 + 0.15 * (t * (7.0 + index) + index * 1.7).sin());
    }
}

fn dump_council(world: &World) -> (String, Value) {
    let state = world.resource::<CouncilState>();
    let props = world.resource::<CouncilProps>();
    let assets = world.resource::<AssetServer>();
    let style = state.style.name();
    let (hovered, signature, clicks, hovers, last_click, populated) = (
        state.hovered.clone(),
        state.signature.clone(),
        state.clicks,
        state.hovers,
        state.last_click.clone(),
        state.populated,
    );
    let mut models = Vec::new();
    // try_query is None until a component is registered (council never built).
    let mut query = world.try_query::<(&CouncilModel, &GlobalTransform)>();
    for (model, transform) in query.iter_mut().flat_map(|q| q.iter(world)) {
        let (scale, _, translation) = transform.to_scale_rotation_translation();
        models.push(json!({
            "role": model.role, "path": model.path,
            "loaded": assets.is_loaded_with_dependencies(&model.scene),
            "position": [translation.x, translation.y, translation.z],
            "scale": [scale.x, scale.y, scale.z],
        }));
    }
    models.sort_by_key(|m| (m["role"].to_string(), m["position"].to_string()));
    let mut painted = Vec::new();
    let mut query = world.try_query::<(&figures::Figure, &GlobalTransform)>();
    for (figure, transform) in query.iter_mut().flat_map(|q| q.iter(world)) {
        painted.push(json!({
            "key": figure.key, "kind": figure.kind,
            "image": figure.image, "image_loaded": figure.fitted,
            "position": transform.translation().to_array(),
        }));
    }
    painted.sort_by_key(|f| f["key"].to_string());
    let camera = world
        .try_query_filtered::<(&Transform, &Projection, &Camera), With<CouncilCamera>>()
        .and_then(|mut q| {
            q.iter(world)
                .next()
                .map(|(t, p, c)| (*t, p.clone(), c.is_active))
        })
        .map(|(t, p, active)| {
            let fov = match p {
                Projection::Perspective(p) => p.fov.to_degrees(),
                _ => 0.0,
            };
            json!({
                "position": t.translation.to_array(),
                "forward": t.forward().as_vec3().to_array(),
                "fov_deg": fov, "active": active,
                "target": pick::CAMERA_TARGET.to_array(),
                "pixels": [STAGE_PIXELS.x, STAGE_PIXELS.y],
            })
        });
    let lights = world
        .try_query::<&PointLight>()
        .map_or(0, |mut q| q.iter(world).count())
        + world
            .try_query::<&SpotLight>()
            .map_or(0, |mut q| q.iter(world).count());
    let vacant_labels = world
        .try_query::<&stage::StageLabel>()
        .map_or(0, |mut q| q.iter(world).count());
    let overlays = world
        .try_query::<&figures::HoverOverlay>()
        .map_or(0, |mut q| q.iter(world).count());
    (
        "council".to_owned(),
        json!({
            "built": camera.is_some(),
            "style": style,
            "camera": camera,
            "models": models,
            "figures": painted,
            "decor": {
                "meshes": props.meshes, "lights_decor": props.lights, "candles": props.candles,
                "tapestries": props.tapestries, "flame_halos": props.halos, "heraldry_emblem": props.emblem,
                "point_and_spot_lights_in_world": lights,
            },
            "seats": signature.iter().map(|(k, occupied)| json!({"office": k, "occupied": occupied})).collect::<Vec<_>>(),
            "stage_vacant_labels": vacant_labels,
            "highlight_overlays": overlays,
            "card": {"text": state.card, "portrait": state.card_portrait},
            "hovered": hovered, "hovers": hovers, "clicks": clicks, "last_click": last_click,
            "populated": populated,
            "pixel_probe": world.resource::<ProbeResult>().0.lock().ok().and_then(|g| g.clone()),
        }),
    )
}

/// `#rrggbb` to linear-free sRGB components; mid grey when malformed.
fn hex_color(text: &str) -> [f32; 3] {
    let digits = text.trim_start_matches('#');
    if digits.len() != 6 {
        return [0.5; 3];
    }
    let channel = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&digits[range], 16).map_or(0.5, |v| f32::from(v) / 255.0)
    };
    [channel(0..2), channel(2..4), channel(4..6)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors() {
        assert_eq!(hex_color("#FF0000"), [1.0, 0.0, 0.0]);
        assert_eq!(hex_color("zz"), [0.5; 3]);
    }

    #[test]
    fn styles_parse() {
        assert_eq!(Style::parse("mesh_multiview"), Some(Style::MeshMultiview));
        assert_eq!(Style::parse("x"), None);
    }
}
