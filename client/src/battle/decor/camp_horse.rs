//! Tethered camp horses with a vertex animation (port of `camp_horse.gdshader` and of
//! `AnimalMotion.camp_horse_material` / `build_camp_horses`): breathing, weight shifting from one
//! hind hoof to the other, head rising and chewing, tail swishing the flies. Values from
//! `data/fx/camp_horse_motion.json` (`camp_horse` block; measured on free videos) and the
//! `enabled` switch of `data/fx/animal_motion.json`.
//!
//! The kit's horse models (`horse_0..2`, see `kit_site.rs`) are glb scenes; `site.rs` marks each one
//! with [`CampHorse`] and [`bind_camp_horses`] swaps the `StandardMaterial` of every mesh for an
//! `ExtendedMaterial` whose vertex stage (`camp_horse.wesl`, pose in `camp_horse_pose.wesl`) moves
//! the vertices; the surfaces named `Hair*` carry the mane and tail masks (`ch_hair`).
//!
//! Difference with Godot: one scene per horse (not one `MultiMesh` per model), so the horses are
//! not instanced; the visibility range is not applied.

use std::collections::HashMap;

use bevy::asset::embedded_asset;
use bevy::gltf::GltfMaterialName;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use serde_json::Value;

/// Marks the root of a camp horse scene (spawned by `site.rs`).
#[derive(Component)]
pub struct CampHorse;

/// The meshes of the horse carry the animated material.
#[derive(Component)]
pub struct CampHorseBound;

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct CampHorseParams {
    pub head: Vec4,
    pub head2: Vec4,
    pub head3: Vec4,
    pub tail: Vec4,
    pub tail2: Vec4,
    pub shift: Vec4,
    pub breath: Vec4,
    /// x: hair (mane and tail mask), y: animated.
    pub misc: Vec4,
}

impl CampHorseParams {
    /// Uniforms of `camp_horse_material` from the `camp_horse` block (keys of the json).
    pub fn from_block(block: &Value, enabled: bool, hair: bool) -> CampHorseParams {
        let f = |key: &str| block.get(key).and_then(Value::as_f64).unwrap_or(0.0) as f32;
        CampHorseParams {
            head: Vec4::new(
                f("head_pivot_x_m"),
                f("head_pivot_y_m"),
                f("head_start_x_m"),
                f("head_end_x_m"),
            ),
            head2: Vec4::new(
                f("head_lift_rad"),
                f("head_cycle_s"),
                f("head_up_share"),
                f("head_nod_rad"),
            ),
            head3: Vec4::new(
                f("head_nod_hz"),
                f("chew_hz"),
                f("chew_rad"),
                f("head_ramp_s"),
            ),
            tail: Vec4::new(
                f("tail_start_x_m"),
                f("tail_end_x_m"),
                f("tail_top_y_m"),
                f("tail_len_m").max(0.05),
            ),
            tail2: Vec4::new(
                f("tail_swing_m"),
                f("tail_hz"),
                f("tail_swish_period_s").max(1.0),
                0.0,
            ),
            shift: Vec4::new(
                f("shift_period_s").max(1.0),
                f("shift_lift_m"),
                f("shift_back_m"),
                f("shift_sway_m"),
            ),
            breath: Vec4::new(f("breath_m"), f("breath_side"), f("breath_hz"), 0.0),
            misc: Vec4::new(
                if hair { 1.0 } else { 0.0 },
                if enabled { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct CampHorseExtension {
    #[uniform(100)]
    pub params: CampHorseParams,
}

impl MaterialExtension for CampHorseExtension {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/camp_horse.wesl".into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/camp_horse_shadow.wesl".into()
    }
}

pub type CampHorseMaterial = ExtendedMaterial<StandardMaterial, CampHorseExtension>;

/// `camp_horse` block of `camp_horse_motion.json` and the `enabled` switch of `animal_motion.json`.
#[derive(Resource, Debug, Clone, Default)]
pub struct CampHorseLook {
    pub block: Value,
    pub enabled: bool,
}

impl CampHorseLook {
    pub fn load(data_dir: &std::path::Path) -> CampHorseLook {
        let read = |name: &str| -> Option<Value> {
            let text = std::fs::read_to_string(data_dir.join(name)).ok()?;
            serde_json::from_str(&text).ok()
        };
        let motion = read("fx/animal_motion.json");
        let camp = read("fx/camp_horse_motion.json");
        CampHorseLook {
            enabled: motion
                .as_ref()
                .and_then(|m| m.get("enabled"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            block: camp
                .as_ref()
                .and_then(|c| c.get("camp_horse").cloned())
                .unwrap_or(Value::Null),
        }
    }

    /// The json has the block: the horses can be animated.
    pub fn usable(&self) -> bool {
        self.enabled && self.block.is_object()
    }
}

#[derive(Resource, Default)]
pub struct CampHorseStats {
    pub horses: usize,
    pub bound: usize,
    pub meshes: usize,
    pub hair_meshes: usize,
    pub materials: usize,
}

#[derive(Resource, Default)]
struct CampHorseMaterials {
    cache: HashMap<(AssetId<StandardMaterial>, bool), Handle<CampHorseMaterial>>,
}

pub struct CampHorsePlugin;

impl Plugin for CampHorsePlugin {
    fn build(&self, app: &mut App) {
        bevy::shader::load_shader_library!(app, "camp_horse_pose.wesl");
        embedded_asset!(app, "camp_horse.wesl");
        embedded_asset!(app, "camp_horse_shadow.wesl");
        let look = crate::paths::find_root()
            .map(|root| CampHorseLook::load(&crate::paths::data_dir(&root)))
            .unwrap_or_default();
        app.add_plugins(MaterialPlugin::<CampHorseMaterial>::default())
            .insert_resource(look)
            .init_resource::<CampHorseMaterials>()
            .init_resource::<CampHorseStats>()
            .add_systems(Update, bind_camp_horses);
    }
}

/// Swaps the materials of the horses' meshes once their scene is spawned.
#[allow(clippy::too_many_arguments)]
fn bind_camp_horses(
    mut commands: Commands,
    look: Res<CampHorseLook>,
    mut stats: ResMut<CampHorseStats>,
    mut cache: ResMut<CampHorseMaterials>,
    standard: Res<Assets<StandardMaterial>>,
    mut animated: ResMut<Assets<CampHorseMaterial>>,
    pending: Query<Entity, (With<CampHorse>, Without<CampHorseBound>)>,
    children: Query<&Children>,
    parts: Query<(&MeshMaterial3d<StandardMaterial>, Option<&GltfMaterialName>), With<Mesh3d>>,
) {
    for horse in &pending {
        let found: Vec<(Entity, Handle<StandardMaterial>, bool)> = children
            .iter_descendants(horse)
            .filter_map(|d| {
                parts.get(d).ok().map(|(m, name)| {
                    let hair = name.is_some_and(|n| n.0.starts_with("Hair"));
                    (d, m.0.clone(), hair)
                })
            })
            .collect();
        if found.is_empty() {
            continue;
        }
        stats.horses += 1;
        commands.entity(horse).try_insert(CampHorseBound);
        if !look.usable() {
            continue;
        }
        stats.bound += 1;
        for (mesh, source, hair) in found {
            let handle = match cache.cache.get(&(source.id(), hair)) {
                Some(h) => h.clone(),
                None => {
                    let Some(base) = standard.get(source.id()) else {
                        continue;
                    };
                    let h = animated.add(CampHorseMaterial {
                        base: base.clone(),
                        extension: CampHorseExtension {
                            params: CampHorseParams::from_block(&look.block, true, hair),
                        },
                    });
                    cache.cache.insert((source.id(), hair), h.clone());
                    stats.materials = cache.cache.len();
                    h
                }
            };
            commands
                .entity(mesh)
                .try_remove::<MeshMaterial3d<StandardMaterial>>()
                .try_insert(MeshMaterial3d(handle));
            stats.meshes += 1;
            if hair {
                stats.hair_meshes += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> CampHorseLook {
        let root = crate::paths::find_root().expect("root");
        CampHorseLook::load(&crate::paths::data_dir(&root))
    }

    #[test]
    fn the_params_follow_the_json_block() {
        let look = look();
        assert!(look.usable());
        let p = CampHorseParams::from_block(&look.block, true, true);
        assert_eq!(p.head, Vec4::new(-0.75, 1.35, -0.75, -1.15));
        assert!((p.head2.x - 0.549).abs() < 1e-5 && p.head2.y == 15.0);
        assert!((p.head3.y - 1.11).abs() < 1e-5 && (p.head3.w - 1.0).abs() < 1e-6);
        assert!((p.tail.w - 0.75).abs() < 1e-6);
        assert!((p.tail2.y - 0.322).abs() < 1e-5);
        assert_eq!(p.shift.x, 22.0);
        assert_eq!(p.misc, Vec4::new(1.0, 1.0, 0.0, 0.0));
    }

    #[test]
    fn horse_surfaces_carry_the_hair_name() {
        // The kit's horses name their mane and tail `Hair`; the mask is read from that name.
        let root = crate::paths::find_root().expect("root");
        let bytes =
            std::fs::read(crate::paths::asset_root(&root).join("models/buildings/horse_0.glb"))
                .expect("glb");
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("\"name\":\"Hair\""), "Hair surface");
    }

    /// The pose module compiles (WESL -> WGSL) and validates (naga) with a consumer.
    #[test]
    fn pose_module_compiles_and_validates() {
        use wesl::syntax::{ModulePath, PathOrigin};
        let abs = |name: &str| ModulePath::new(PathOrigin::Absolute, vec![name.to_string()]);
        let mut resolver = wesl::resolver::VirtualResolver::new();
        resolver.add_module(abs("pose"), include_str!("camp_horse_pose.wesl").into());
        resolver.add_module(
            abs("consumer"),
            r#"
import super::pose::{CampHorseParams, camp_pose};
@group(0) @binding(0) var<uniform> horse: CampHorseParams;
@fragment
fn fs(@location(0) p: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(camp_pose(horse, p, p.xz, 1.5), 1.0);
}
"#
            .into(),
        );
        let mut compiler = wesl::Compiler::default().with_resolver(resolver);
        compiler.options.keep_main = true;
        let wgsl = compiler.compile("consumer").expect("wesl").to_string();
        let module = naga::front::wgsl::parse_str(&wgsl).unwrap_or_else(|e| panic!("{e}\n{wgsl}"));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}\n{wgsl}"));
    }
}
