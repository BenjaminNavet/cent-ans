//! Backdrop of the siege of an emblematic town (port of `landmark_backdrop.gd` and of
//! `game/shaders/landmark.gdshader`, siege use).
//!
//! The besieged town itself is drawn from the plan by `works.rs`; what lies beyond its far wall is
//! a model made by `landmark_city.py --siege`: `models/landmarks/<id>_siege.glb` (metres, origin on
//! the plan's anchor, the plan turned by `siege.rotate_deg`, north towards +Z once laid). Paris has
//! the Seine, the Cite with Notre-Dame and the right bank; London the Thames, the bridge and
//! Southwark; Rouen the Seine and Saint-Sever; and so on for the seven towns that have a plan.
//!
//! - Placement: the model is turned by pi about Y and stands `bank_offset_m` behind the bank, which
//!   is `BANK_MARGIN_M` beyond the farthest wall piece (`center_x_m` re-centres it sideways).
//! - Draping: the heights of the battle ground under the model (maximum over the texel's centre and
//!   corners, +0.3 m) are baked into a 192 x 192 `R32Float` texture; the vertex stage of
//!   `backdrop.wesl` lifts each building by the height under its anchor (UV), so the water and the
//!   quays never go under the ground between two samples.
//! - Matter: every glTF material is replaced by [`BackdropMaterial`] (atlas of the landmark
//!   textures, `textures/landmarks`, 16 layers; palette colour and roughness taken from the glb
//!   material, `Water` flagged).
//!
//! Dev hook: `CENT_ANS_LANDMARK_BACKDROP=<id>` forces the backdrop whatever the province
//! (`--landmark-backdrop=<id>` of Godot). Dump section `battle_backdrop`.
//!
//! Not ported: the shadow of the model (it casts none: the materials skip the prepass), and the
//! `fade` / `tint_strength` / `height_in_meters` / debug views of the shader, which only the
//! campaign maquettes use.

use std::collections::HashMap;
use std::path::Path;

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::gltf::{GltfAssetLabel, GltfMaterialName};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use bevy::world_serialization::WorldAssetRoot;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::decor::ground::{PendingStrip, strip_to_array};
use crate::battle::scene::BattleSession;

/// Side of the baked height grid.
pub const HEIGHT_RES: usize = 192;
/// Margin between the far wall and the bank (m).
pub const BANK_MARGIN_M: f64 = 40.0;
/// Lift of the baked heights (m), so that water and quays do not sink into the ground.
pub const HEIGHT_LIFT_M: f32 = 0.3;
/// Layers of the landmark atlas.
const LAYERS: u32 = 16;
const ALBEDO: &str = "textures/landmarks/landmark_albedo_array.jpg";
const NORMAL: &str = "textures/landmarks/landmark_normal_array.jpg";
/// Ageing of the matter (`aging` of `landmark.gdshader`).
const AGING: f32 = 1.0;
/// Height of the ground plates at the foot of the walls (`ground_height` default: 0.010 map unit x
/// 205 m).
const GROUND_HEIGHT_M: f32 = 2.05;

/// The `siege` block of `data/landmarks/<id>.json` that the backdrop needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackdropPlan {
    /// Half-size of the baked height grid (m).
    pub radius_m: f32,
    /// Distance of the model behind the bank (m, negative: in front).
    pub bank_offset_m: f32,
    /// Sideways shift of the model (m).
    pub center_x_m: f32,
}

impl BackdropPlan {
    /// Reads the plan of `id`; `None` when the town has no `siege` block or no file.
    pub fn load(data_dir: &Path, id: &str) -> Option<Self> {
        let text = std::fs::read_to_string(data_dir.join(format!("landmarks/{id}.json"))).ok()?;
        Self::from_json(&serde_json::from_str(&text).ok()?)
    }

    pub fn from_json(doc: &Value) -> Option<Self> {
        let siege = doc.get("siege")?;
        let num =
            |key: &str, default: f64| siege.get(key).and_then(Value::as_f64).unwrap_or(default);
        Some(Self {
            radius_m: num("radius_m", 1400.0) as f32,
            bank_offset_m: num("bank_offset_m", 100.0) as f32,
            center_x_m: num("center_x_m", 0.0) as f32,
        })
    }
}

/// Where the model stands: `(position, bank_z)`. `center` is the middle of the ring, `far_z` the
/// farthest z of the wall pieces (at least `center.1`).
pub fn placement(center: (f64, f64), far_z: f64, plan: &BackdropPlan) -> (Vec3, f64) {
    let bank_z = far_z.max(center.1) + BANK_MARGIN_M;
    let position = Vec3::new(
        (center.0 + f64::from(plan.center_x_m)) as f32,
        0.0,
        (bank_z + f64::from(plan.bank_offset_m)) as f32,
    );
    (position, bank_z)
}

/// Heights of the field over the square of half-size `radius` around `(cx, cz)`, row-major
/// `HEIGHT_RES`², each texel the maximum over its centre and four corners (+ [`HEIGHT_LIFT_M`]).
pub fn bake_heights(
    cx: f32,
    cz: f32,
    radius: f32,
    height_at: impl Fn(f64, f64) -> f32,
) -> Vec<f32> {
    let extent = radius * 2.0;
    let cell = extent / HEIGHT_RES as f32;
    let (ox, oz) = (cx - radius, cz - radius);
    let mut out = Vec::with_capacity(HEIGHT_RES * HEIGHT_RES);
    for j in 0..HEIGHT_RES {
        for i in 0..HEIGHT_RES {
            let x = ox + (i as f32 + 0.5) * cell;
            let z = oz + (j as f32 + 0.5) * cell;
            let mut h = height_at(f64::from(x), f64::from(z));
            for (dx, dz) in [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)] {
                h = h.max(height_at(
                    f64::from(x + dx * cell),
                    f64::from(z + dz * cell),
                ));
            }
            out.push(h + HEIGHT_LIFT_M);
        }
    }
    out
}

/// Uniform block of the material (mirrors `BackdropParams` in `backdrop.wesl`).
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct BackdropParams {
    /// x, y: origin of the height grid (world), z: its extent (m).
    pub map: Vec4,
    /// xyz: position of the model's root, w: its yaw (rad).
    pub root: Vec4,
    /// x: water (1), y: ground height (m), z: metres per unit, w: ageing.
    pub look: Vec4,
    /// rgb: sky colour the water reflects (linear-ish tint of the decor look), w: unused.
    pub sky: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct BackdropExt {
    #[uniform(100)]
    pub params: BackdropParams,
    #[texture(101, sample_type = "float", filterable = false)]
    pub height: Handle<Image>,
    #[texture(102, dimension = "2d_array")]
    #[sampler(103)]
    pub albedo: Handle<Image>,
    #[texture(104, dimension = "2d_array")]
    #[sampler(105)]
    pub normal: Handle<Image>,
}

impl MaterialExtension for BackdropExt {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/siege/backdrop.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/siege/backdrop.wesl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}

pub type BackdropMaterial = ExtendedMaterial<StandardMaterial, BackdropExt>;

/// The model laid behind the siege.
#[derive(Component, Debug, Clone)]
pub struct LandmarkBackdrop {
    pub id: String,
}

/// What the build left for the material remap and the dump.
#[derive(Resource, Default, Debug)]
pub struct BackdropState {
    pub id: String,
    pub position: Vec3,
    pub bank_z: f64,
    pub radius_m: f32,
    pub height_min: f32,
    pub height_max: f32,
    params: BackdropParams,
    height: Handle<Image>,
    albedo: Handle<Image>,
    normal: Handle<Image>,
    /// Atlas strips waiting to become arrays.
    strips: Vec<PendingStrip>,
    ready: bool,
    cache: HashMap<[u32; 6], Handle<BackdropMaterial>>,
    pub meshes_remapped: usize,
    pub water_meshes: usize,
    pub materials: usize,
}

/// Id of the landmark whose backdrop this battle gets: the dev override, else the town of the
/// plan the siege was drawn from.
fn wanted_id(layout_id: Option<&str>) -> Option<String> {
    std::env::var("CENT_ANS_LANDMARK_BACKDROP")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| layout_id.map(str::to_string))
        .filter(|v| !v.is_empty())
}

fn build_backdrop(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    look: Option<Res<crate::battle::decor::look::DecorLook>>,
) {
    let sim = session.sim_mut();
    let sky = crate::battle::scene::water::sky_tint(look.as_deref(), sim.weather());
    let Some(works) = sim.siege() else { return };
    let Some(id) = wanted_id(works.landmark.as_ref().map(|l| l.id.as_str())) else {
        return;
    };
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let asset_root = crate::paths::asset_root(&root);
    let model = format!("models/landmarks/{id}_siege.glb");
    if !asset_root.join(&model).is_file() {
        return;
    }
    let Some(plan) = BackdropPlan::load(&crate::paths::data_dir(&root), &id) else {
        return;
    };
    let far_z = works.pieces.iter().fold(works.center.1, |far, piece| {
        far.max(piece.a.1).max(piece.b.1)
    });
    let (position, bank_z) = placement(works.center, far_z, &plan);
    let field = sim.field();
    let heights = bake_heights(position.x, position.z, plan.radius_m, |x, z| {
        field.height(x, z) as f32
    });
    let (height_min, height_max) = heights
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), h| (lo.min(*h), hi.max(*h)));
    let bytes: Vec<u8> = heights.iter().flat_map(|h| h.to_le_bytes()).collect();
    let height = images.add(Image::new(
        Extent3d {
            width: HEIGHT_RES as u32,
            height: HEIGHT_RES as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let albedo: Handle<Image> = assets.load(ALBEDO);
    let normal: Handle<Image> = assets
        .load_builder()
        .with_settings(|s: &mut bevy::image::ImageLoaderSettings| s.is_srgb = false)
        .load(NORMAL);
    let params = BackdropParams {
        map: Vec4::new(
            position.x - plan.radius_m,
            position.z - plan.radius_m,
            plan.radius_m * 2.0,
            0.0,
        ),
        root: Vec4::new(position.x, position.y, position.z, std::f32::consts::PI),
        look: Vec4::new(0.0, GROUND_HEIGHT_M, 1.0, AGING),
        sky: sky.extend(0.0),
    };
    commands.insert_resource(BackdropState {
        id: id.clone(),
        position,
        bank_z,
        radius_m: plan.radius_m,
        height_min,
        height_max,
        params,
        height,
        albedo: albedo.clone(),
        normal: normal.clone(),
        strips: vec![
            PendingStrip {
                handle: albedo,
                layers: LAYERS,
                anisotropy: 8,
            },
            PendingStrip {
                handle: normal,
                layers: LAYERS,
                anisotropy: 8,
            },
        ],
        ..default()
    });
    commands.spawn((
        Name::new("LandmarkBackdrop"),
        LandmarkBackdrop { id },
        WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(model))),
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        Visibility::default(),
        DespawnOnExit(AppState::Battle),
    ));
}

/// Gives the atlas strips their array shape once loaded.
fn convert_atlas(mut state: ResMut<BackdropState>, mut images: ResMut<Assets<Image>>) {
    if state.ready {
        return;
    }
    state.strips.retain(|strip| {
        let Some(mut image) = images.get_mut(&strip.handle) else {
            return true;
        };
        if !strip_to_array(&mut image, strip.layers, strip.anisotropy) {
            warn!("landmark backdrop: atlas strip has an unexpected shape, not converted");
        }
        false
    });
    state.ready = state.strips.is_empty();
}

/// Replaces the glTF materials of the model by the backdrop material (colour and roughness of the
/// glb material, water flagged by its name), once the atlas is ready.
#[allow(clippy::too_many_arguments)]
fn remap_materials(
    mut commands: Commands,
    mut state: ResMut<BackdropState>,
    mut materials: ResMut<Assets<BackdropMaterial>>,
    standard: Res<Assets<StandardMaterial>>,
    pending: Query<
        (Entity, &GltfMaterialName, &MeshMaterial3d<StandardMaterial>),
        Without<BackdropRemapped>,
    >,
    parents: Query<&ChildOf>,
    roots: Query<(), With<LandmarkBackdrop>>,
) {
    if !state.ready {
        return;
    }
    for (entity, name, handle) in &pending {
        if !parents.iter_ancestors(entity).any(|a| roots.contains(a)) {
            continue;
        }
        let Some(source) = standard.get(&handle.0) else {
            continue;
        };
        let water = name.0.split('.').next() == Some("Water");
        let linear = source.base_color.to_linear();
        let key = [
            linear.red.to_bits(),
            linear.green.to_bits(),
            linear.blue.to_bits(),
            source.perceptual_roughness.to_bits(),
            u32::from(water),
            0,
        ];
        let params = BackdropParams {
            look: Vec4::new(f32::from(u8::from(water)), GROUND_HEIGHT_M, 1.0, AGING),
            ..state.params
        };
        let (height, albedo, normal) = (
            state.height.clone(),
            state.albedo.clone(),
            state.normal.clone(),
        );
        let material = state
            .cache
            .entry(key)
            .or_insert_with(|| {
                materials.add(ExtendedMaterial {
                    base: StandardMaterial {
                        base_color: Color::LinearRgba(LinearRgba::new(
                            linear.red,
                            linear.green,
                            linear.blue,
                            1.0,
                        )),
                        perceptual_roughness: source.perceptual_roughness,
                        metallic: 0.0,
                        double_sided: true,
                        cull_mode: None,
                        alpha_mode: AlphaMode::Opaque,
                        ..default()
                    },
                    extension: BackdropExt {
                        params,
                        height,
                        albedo,
                        normal,
                    },
                })
            })
            .clone();
        commands
            .entity(entity)
            .try_remove::<MeshMaterial3d<StandardMaterial>>()
            .try_insert((MeshMaterial3d(material), BackdropRemapped, NoFrustumCulling));
        state.meshes_remapped += 1;
        state.water_meshes += usize::from(water);
        state.materials = state.cache.len();
    }
}

/// Mesh of the model already given the backdrop material.
#[derive(Component)]
pub struct BackdropRemapped;

fn dump_backdrop(world: &World) -> (String, Value) {
    let Some(state) = world.get_resource::<BackdropState>() else {
        return ("battle_backdrop".into(), json!({"present": false}));
    };
    (
        "battle_backdrop".into(),
        json!({
            "present": true,
            "id": state.id,
            "position": [state.position.x, state.position.y, state.position.z],
            "bank_z": state.bank_z,
            "radius_m": state.radius_m,
            "height_range": [state.height_min, state.height_max],
            "atlas_ready": state.ready,
            "meshes_remapped": state.meshes_remapped,
            "water_meshes": state.water_meshes,
            "materials": state.materials,
        }),
    )
}

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "backdrop.wesl");
        app.add_plugins(MaterialPlugin::<BackdropMaterial>::default())
            .register_dump(dump_backdrop)
            .add_systems(
                Update,
                build_backdrop
                    .run_if(resource_added::<BattleSession>)
                    .run_if(in_state(AppState::Battle)),
            )
            .add_systems(
                Update,
                (convert_atlas, remap_materials)
                    .chain()
                    .run_if(resource_exists::<BackdropState>),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<BackdropState>();
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_dir() -> std::path::PathBuf {
        crate::paths::data_dir(&crate::paths::find_root().expect("root"))
    }

    #[test]
    fn the_seven_towns_have_a_plan_and_a_model() {
        let root = crate::paths::find_root().expect("root");
        for id in [
            "paris", "london", "rouen", "calais", "bordeaux", "bruges", "avignon",
        ] {
            assert!(
                BackdropPlan::load(&crate::paths::data_dir(&root), id).is_some(),
                "{id} plan"
            );
            assert!(
                crate::paths::asset_root(&root)
                    .join(format!("models/landmarks/{id}_siege.glb"))
                    .is_file(),
                "{id} model"
            );
        }
        // Cairo has no siege block: no backdrop.
        assert!(BackdropPlan::load(&data_dir(), "cairo").is_none());
    }

    #[test]
    fn plans_read_the_siege_block() {
        let paris = BackdropPlan::load(&data_dir(), "paris").unwrap();
        assert_eq!(paris.radius_m, 1000.0);
        assert_eq!(paris.bank_offset_m, 106.0);
        assert_eq!(paris.center_x_m, 0.0);
        let london = BackdropPlan::load(&data_dir(), "london").unwrap();
        assert_eq!(london.center_x_m, -1500.0);
        assert_eq!(london.bank_offset_m, -155.0);
        let defaults = BackdropPlan::from_json(&json!({"siege": {}})).unwrap();
        assert_eq!(defaults.radius_m, 1400.0);
        assert_eq!(defaults.bank_offset_m, 100.0);
    }

    #[test]
    fn the_model_stands_behind_the_far_wall() {
        let plan = BackdropPlan {
            radius_m: 1000.0,
            bank_offset_m: 106.0,
            center_x_m: -30.0,
        };
        let (position, bank_z) = placement((600.0, 560.0), 700.0, &plan);
        assert_eq!(bank_z, 740.0);
        assert_eq!(position, Vec3::new(570.0, 0.0, 846.0));
        // A ring whose pieces are all in front of the centre still starts at the centre.
        let (_, bank_z) = placement((600.0, 560.0), 100.0, &plan);
        assert_eq!(bank_z, 600.0);
    }

    #[test]
    fn baked_heights_take_the_highest_corner_and_lift_it() {
        let grid = bake_heights(0.0, 0.0, 96.0, |x, _| if x > -0.25 { 10.0 } else { 0.0 });
        assert_eq!(grid.len(), HEIGHT_RES * HEIGHT_RES);
        let row = &grid[..HEIGHT_RES];
        assert!((row[0] - HEIGHT_LIFT_M).abs() < 1e-5, "far from the step");
        assert!((row[HEIGHT_RES - 1] - (10.0 + HEIGHT_LIFT_M)).abs() < 1e-5);
        // The texel straddling x = 0 takes the high side (its right corners).
        assert!((row[HEIGHT_RES / 2 - 1] - (10.0 + HEIGHT_LIFT_M)).abs() < 1e-5);
        assert!((row[HEIGHT_RES / 2 - 2] - HEIGHT_LIFT_M).abs() < 1e-5);
    }
}
