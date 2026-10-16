//! Animated tethered horses of the camps (port of `AnimalMotion.build_camp_horses` /
//! `camp_horse_material` of game/scripts/visual/animal_motion.gd and of
//! game/shaders/camp_horse.gdshader).
//!
//! The kit horse models have no bones: like Godot the vertices are displaced in the vertex stage
//! (`camp_horse.wesl`): breathing, weight passing from one hind hoof to the other, head raised
//! from time to time and chewing, tail swishing. Values: `camp_horse` of
//! `data/fx/camp_horse_motion.json` (measured on free videos), switched off by `enabled` of
//! `data/fx/animal_motion.json`. The scene's `StandardMaterial`s of a model flagged [`CampHorse`]
//! are swapped for an `ExtendedMaterial` (the glb colours are kept); the `Hair*` materials
//! (mane and tail) carry the tail mask.

use std::collections::HashMap;

use bevy::asset::embedded_asset;
use bevy::gltf::GltfMaterialName;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use serde_json::Value;

use crate::app::AppState;

/// Marks the scene root of a tethered horse.
#[derive(Component)]
pub struct CampHorse;

/// Marks a horse whose materials were swapped.
#[derive(Component)]
struct CampHorseBound;

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct CampHorseParams {
    pub head: Vec4,
    pub head2: Vec4,
    pub head3: Vec4,
    pub tail: Vec4,
    pub tail2: Vec4,
    pub shift: Vec4,
    /// (vertical, lateral, frequency, hair flag)
    pub breath: Vec4,
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
}

pub type CampHorseMaterial = ExtendedMaterial<StandardMaterial, CampHorseExtension>;

/// Settings of the horses (`camp_horse` object), `None` when the motion is switched off.
#[derive(Resource, Clone, Debug, Default)]
pub struct CampHorseSettings {
    pub enabled: bool,
    pub params: CampHorseParams,
}

fn f(doc: &Value, key: &str) -> f32 {
    doc[key].as_f64().unwrap_or(0.0) as f32
}

impl CampHorseSettings {
    /// Parameters from the two data files (`AnimalMotion.camp_horse_material`).
    pub fn from_json(motion: &Value, camp: &Value) -> CampHorseSettings {
        let s = &camp["camp_horse"];
        let params = CampHorseParams {
            head: Vec4::new(
                f(s, "head_pivot_x_m"),
                f(s, "head_pivot_y_m"),
                f(s, "head_start_x_m"),
                f(s, "head_end_x_m"),
            ),
            head2: Vec4::new(
                f(s, "head_lift_rad"),
                f(s, "head_cycle_s"),
                f(s, "head_up_share"),
                f(s, "head_nod_rad"),
            ),
            head3: Vec4::new(
                f(s, "head_nod_hz"),
                f(s, "chew_hz"),
                f(s, "chew_rad"),
                f(s, "head_ramp_s"),
            ),
            tail: Vec4::new(
                f(s, "tail_start_x_m"),
                f(s, "tail_end_x_m"),
                f(s, "tail_top_y_m"),
                f(s, "tail_len_m").max(0.05),
            ),
            tail2: Vec4::new(
                f(s, "tail_swing_m"),
                f(s, "tail_hz"),
                f(s, "tail_swish_period_s").max(1.0),
                0.0,
            ),
            shift: Vec4::new(
                f(s, "shift_period_s").max(1.0),
                f(s, "shift_lift_m"),
                f(s, "shift_back_m"),
                f(s, "shift_sway_m"),
            ),
            breath: Vec4::new(
                f(s, "breath_m"),
                f(s, "breath_side"),
                f(s, "breath_hz"),
                0.0,
            ),
        };
        CampHorseSettings {
            enabled: motion["enabled"].as_bool().unwrap_or(false) && s.is_object(),
            params,
        }
    }

    pub fn load() -> CampHorseSettings {
        let read = |name: &str| -> Value {
            crate::paths::find_root()
                .ok()
                .and_then(|root| {
                    std::fs::read(crate::paths::data_dir(&root).join("fx").join(name)).ok()
                })
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or(Value::Null)
        };
        CampHorseSettings::from_json(&read("animal_motion.json"), &read("camp_horse_motion.json"))
    }
}

#[derive(Resource, Default)]
pub struct CampHorseStats {
    pub horses: usize,
    pub materials: usize,
    pub hair_materials: usize,
}

#[derive(Resource, Default)]
struct HorseMaterials {
    cache: HashMap<(AssetId<StandardMaterial>, bool), Handle<CampHorseMaterial>>,
}

pub struct CampHorsePlugin;

impl Plugin for CampHorsePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "camp_horse.wesl");
        app.add_plugins(MaterialPlugin::<CampHorseMaterial>::default())
            .insert_resource(CampHorseSettings::load())
            .init_resource::<HorseMaterials>()
            .init_resource::<CampHorseStats>()
            .add_systems(Update, bind.run_if(in_state(AppState::Battle)))
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.insert_resource(HorseMaterials::default());
                commands.insert_resource(CampHorseStats::default());
            });
    }
}

/// Swaps the materials of the horses once their scene is spawned.
#[allow(clippy::too_many_arguments)]
fn bind(
    mut commands: Commands,
    settings: Res<CampHorseSettings>,
    mut cache: ResMut<HorseMaterials>,
    mut stats: ResMut<CampHorseStats>,
    standard: Res<Assets<StandardMaterial>>,
    mut animated: ResMut<Assets<CampHorseMaterial>>,
    pending: Query<Entity, (With<CampHorse>, Without<CampHorseBound>)>,
    children: Query<&Children>,
    parts: Query<(&MeshMaterial3d<StandardMaterial>, Option<&GltfMaterialName>), With<Mesh3d>>,
) {
    if !settings.enabled {
        return;
    }
    for horse in &pending {
        let found: Vec<_> = children
            .iter_descendants(horse)
            .filter_map(|d| parts.get(d).ok().map(|(m, n)| (d, m.0.clone(), n)))
            .collect();
        if found.is_empty() {
            continue;
        }
        for (mesh, source, name) in found {
            let hair = name.is_some_and(|n| n.0.starts_with("Hair"));
            let handle = match cache.cache.get(&(source.id(), hair)) {
                Some(h) => h.clone(),
                None => {
                    let Some(base) = standard.get(source.id()) else {
                        continue;
                    };
                    let mut params = settings.params;
                    params.breath.w = if hair { 1.0 } else { 0.0 };
                    let h = animated.add(CampHorseMaterial {
                        base: base.clone(),
                        extension: CampHorseExtension { params },
                    });
                    cache.cache.insert((source.id(), hair), h.clone());
                    stats.materials += 1;
                    stats.hair_materials += usize::from(hair);
                    h
                }
            };
            commands
                .entity(mesh)
                .try_remove::<MeshMaterial3d<StandardMaterial>>()
                .try_insert(MeshMaterial3d(handle));
        }
        stats.horses += 1;
        commands.entity(horse).try_insert(CampHorseBound);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_follow_the_data_files() {
        let root = crate::paths::find_root().expect("root");
        let read = |n: &str| -> Value {
            serde_json::from_slice(
                &std::fs::read(crate::paths::data_dir(&root).join("fx").join(n)).unwrap(),
            )
            .unwrap()
        };
        let s = CampHorseSettings::from_json(
            &read("animal_motion.json"),
            &read("camp_horse_motion.json"),
        );
        assert!(s.enabled);
        assert!((s.params.head2.x - 0.549).abs() < 1e-4, "head lift");
        assert!((s.params.head3.y - 1.11).abs() < 1e-4, "chew frequency");
        assert!((s.params.shift.x - 22.0).abs() < 1e-4);
        assert!((s.params.tail.w - 0.75).abs() < 1e-4);
    }

    #[test]
    fn a_missing_file_switches_the_motion_off() {
        let s = CampHorseSettings::from_json(&Value::Null, &Value::Null);
        assert!(!s.enabled);
    }
}
