//! Vertex animation of the beasts, carts and herds of the living map.
//!
//! Port of `visual/animal_motion.gd` + `shaders/animal_motion.gdshaderinc` (draught beasts, carts,
//! ploughs) and of the gait part of `shaders/fauna.gdshader` (herds). The glb models have no bones:
//! like Godot, the vertices are displaced in the vertex stage (`anim.wesl`) from the measured data
//! of `data/fx/animal_motion.json` (step curves, jolt lines, head, tail, wheels) and
//! `data/fx/camp_horse_motion.json` (chewing of the horse). The scene's `StandardMaterial`s are
//! swapped for an `ExtendedMaterial` whose extension carries the parameters of the model; the
//! per-instance values (speed, seed, leg phase) go through the `MeshTag`.
//!
//! Differences with Godot: FK2 stand-in models (not DN glb) stay rigid; the herd wander stays on the
//! CPU (`fauna::move_animals`, which now uses the warped time of `fauna.gdshader`).

use std::collections::HashMap;

use bevy::camera::primitives::Aabb;
use bevy::mesh::MeshTag;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::data::Gait;
use super::motion::{self, MotionSpec, MotionTable};

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct AnimParams {
    pub body: Vec4,
    pub neck: Vec4,
    pub gait: Vec4,
    pub head: Vec4,
    pub tail: Vec4,
    pub breath: Vec4,
    pub cart: Vec4,
    pub axle: Vec4,
    pub extra: Vec4,
    pub jolt_amp: Vec4,
    pub jolt_ratio: Vec4,
    pub frame: Vec4,
    pub herd_gait: Vec4,
    pub herd_origin: Vec4,
    pub herd_size: Vec4,
    pub swing: [Vec4; 4],
    pub lift: [Vec4; 4],
}

fn pack_lut(lut: &[f32; 16]) -> [Vec4; 4] {
    std::array::from_fn(|i| Vec4::new(lut[i * 4], lut[i * 4 + 1], lut[i * 4 + 2], lut[i * 4 + 3]))
}

impl AnimParams {
    /// Parameters of a prop model (`AnimalMotion.apply_prop`); `yaw` turns the glb to the Godot
    /// frame (front along +Z).
    pub fn for_prop(spec: &MotionSpec, yaw: f32) -> AnimParams {
        let period = if spec.has_beast {
            spec.stride_m
        } else {
            spec.jolt_period_m
        };
        let mut amp = Vec4::ZERO;
        let mut ratio = Vec4::ONE;
        for (k, (a, r)) in spec.jolt_lines.iter().take(3).enumerate() {
            amp[k] = *a;
            ratio[k] = *r;
        }
        let axle2 = spec.axle_z_m.get(1).copied();
        AnimParams {
            body: Vec4::new(spec.anchor_m, spec.half_m, spec.belly_m, spec.withers_m),
            neck: Vec4::new(
                spec.neck_start_m,
                spec.neck_len_m,
                spec.neck_pivot_m,
                spec.tail_len_m,
            ),
            gait: Vec4::new(spec.stride_m, spec.swing_m, spec.lift_m, spec.bob_m),
            head: Vec4::new(
                spec.graze_rad,
                spec.nod_rad,
                spec.graze_cycle_s,
                spec.graze_share,
            ),
            tail: Vec4::new(
                spec.tail_swing_m,
                spec.tail_hz,
                spec.tail_walk_gain,
                spec.shift_m,
            ),
            breath: Vec4::new(spec.breath_m, spec.breath_hz, spec.chew_hz, spec.chew_rad),
            cart: Vec4::new(spec.wheel_radius_m, spec.wheel_x_min_m, 1.0, period),
            axle: Vec4::new(
                spec.axle_z_m[0],
                axle2.unwrap_or(0.0),
                if axle2.is_some() { 1.0 } else { 0.0 },
                0.0,
            ),
            extra: Vec4::new(spec.graze_ramp_s, spec.roll_rad, 0.0, 0.0),
            jolt_amp: amp,
            jolt_ratio: ratio,
            frame: Vec4::new(yaw, 0.0, if spec.has_beast { 1.0 } else { 0.0 }, 0.0),
            swing: pack_lut(&spec.swing_lut),
            lift: pack_lut(&spec.lift_lut),
            ..default()
        }
    }

    /// Parameters of a herd species from its gait and the bounds of its mesh
    /// (`min`/`max` in mesh units, `FaunaLayer._species_state`).
    pub fn for_herd(gait: &Gait, min: Vec3, max: Vec3) -> AnimParams {
        let size = max - min;
        let long_x = size.x >= size.z;
        AnimParams {
            frame: Vec4::new(0.0, 1.0, 0.0, 0.0),
            herd_gait: Vec4::new(gait.cadence.max(1e-3), gait.swing, gait.lift, gait.bounce),
            herd_origin: Vec4::new(
                (min.x + max.x) * 0.5,
                min.y,
                (min.z + max.z) * 0.5,
                gait.graze,
            ),
            herd_size: Vec4::new(
                size.x.max(size.z).max(1e-4),
                size.y,
                if long_x { 1.0 } else { 0.0 },
                0.0,
            ),
            ..default()
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct AnimExtension {
    #[uniform(100)]
    pub params: AnimParams,
}

impl MaterialExtension for AnimExtension {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/life/anim.wesl".into()
    }
}

pub type AnimMaterial = ExtendedMaterial<StandardMaterial, AnimExtension>;

/// What an instance's model animates as.
#[derive(Debug, Clone)]
pub enum AnimKind {
    /// Cart, plough or draught beast: key of `animal_motion.json` (the DN id), speed in m/s.
    Prop { model: String, speed_m_s: f32 },
    /// Herd animal of a species (gait from `map_fauna.json`); the tag is driven every frame.
    Herd { gait: String },
}

/// Marks the model entity of an instance whose materials are to be swapped.
#[derive(Component, Debug, Clone)]
pub struct AnimBind {
    pub kind: AnimKind,
    /// Seed in [0, 1) of the instance (phase of the legs, of the wheels, of the grazing cycle).
    pub seed: f32,
    /// `yaw_deg` of the prop table, in radians (glb front to +Z). 0 for herds.
    pub yaw: f32,
}

/// The model's meshes carry the animated material.
#[derive(Component, Debug)]
pub struct AnimBound {
    pub meshes: Vec<Entity>,
    pub herd: bool,
}

/// Models seen by the binder (dump).
#[derive(Resource, Default)]
pub struct AnimStats {
    pub bound_props: usize,
    pub bound_herds: usize,
    pub rigid: usize,
    pub materials: usize,
    /// (kind, model/species, speed or moving, seed or phase) of a few instances.
    pub samples: Vec<(String, String, f32, f32)>,
}

#[derive(Resource, Default)]
pub struct AnimMaterials {
    cache: HashMap<(AssetId<StandardMaterial>, String), Handle<AnimMaterial>>,
}

pub fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "anim.wesl");
    app.add_plugins(MaterialPlugin::<AnimMaterial>::default())
        .init_resource::<AnimMaterials>()
        .init_resource::<AnimStats>();
}

/// Swaps the materials of the models flagged `AnimBind` once their scene is spawned.
pub fn bind_models(
    mut commands: Commands,
    table: Res<MotionTable>,
    data: Res<super::data::LifeData>,
    mut cache: ResMut<AnimMaterials>,
    mut stats: ResMut<AnimStats>,
    standard: Res<Assets<StandardMaterial>>,
    mut animated: ResMut<Assets<AnimMaterial>>,
    pending: Query<(Entity, &AnimBind), Without<AnimBound>>,
    children: Query<&Children>,
    parts: Query<(&MeshMaterial3d<StandardMaterial>, Option<&Aabb>), With<Mesh3d>>,
) {
    for (entity, bind) in &pending {
        let found: Vec<(Entity, Handle<StandardMaterial>, Option<Aabb>)> = children
            .iter_descendants(entity)
            .filter_map(|d| parts.get(d).ok().map(|(m, a)| (d, m.0.clone(), a.copied())))
            .collect();
        if found.is_empty() {
            continue;
        }
        let (params, key, tag, herd) = match &bind.kind {
            AnimKind::Prop { model, speed_m_s } => {
                let Some(spec) = table.spec(model) else {
                    commands.entity(entity).try_insert(AnimBound {
                        meshes: Vec::new(),
                        herd: false,
                    });
                    stats.rigid += 1;
                    continue;
                };
                (
                    AnimParams::for_prop(spec, bind.yaw),
                    format!("prop:{model}"),
                    motion::pack_prop_tag(*speed_m_s, bind.seed),
                    false,
                )
            }
            AnimKind::Herd { gait } => {
                // The bounds come from the Aabb of the meshes, ready one frame after the scene.
                if found.iter().any(|f| f.2.is_none()) {
                    continue;
                }
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for aabb in found.iter().filter_map(|f| f.2) {
                    lo = lo.min(Vec3::from(aabb.min()));
                    hi = hi.max(Vec3::from(aabb.max()));
                }
                let g = data.fauna.gaits.get(gait).cloned().unwrap_or_default();
                (
                    AnimParams::for_herd(&g, lo, hi),
                    format!("herd:{gait}:{:.3}:{:.3}", hi.x - lo.x, hi.z - lo.z),
                    motion::pack_herd_tag(bind.seed, 0.0),
                    true,
                )
            }
        };
        let mut meshes = Vec::new();
        for (mesh, source, _) in found {
            let id = source.id();
            let handle = match cache.cache.get(&(id, key.clone())) {
                Some(h) => h.clone(),
                None => {
                    let Some(base) = standard.get(id) else {
                        continue;
                    };
                    let h = animated.add(AnimMaterial {
                        base: base.clone(),
                        extension: AnimExtension { params },
                    });
                    cache.cache.insert((id, key.clone()), h.clone());
                    stats.materials = cache.cache.len();
                    h
                }
            };
            commands
                .entity(mesh)
                .try_remove::<MeshMaterial3d<StandardMaterial>>()
                .try_insert((MeshMaterial3d(handle), MeshTag::new(tag)));
            meshes.push(mesh);
        }
        if meshes.is_empty() {
            continue;
        }
        if herd {
            stats.bound_herds += 1;
        } else {
            stats.bound_props += 1;
        }
        if stats.samples.len() < 24 && (stats.bound_props + stats.bound_herds) % 7 == 1 {
            let (kind, name) = match &bind.kind {
                AnimKind::Prop { model, .. } => ("prop", model.clone()),
                AnimKind::Herd { gait } => ("herd", gait.clone()),
            };
            let (a, b) = if herd {
                motion::unpack_herd_tag(tag)
            } else {
                motion::unpack_prop_tag(tag)
            };
            stats.samples.push((kind.into(), name, a, b));
        }
        commands
            .entity(entity)
            .try_insert(AnimBound { meshes, herd });
    }
}

/// Writes the per-frame tag of a herd animal (leg phase and moving) on the meshes of its model.
pub fn write_herd_tags(
    mut animals: Query<(&mut super::fauna::Animal, &Children)>,
    bound: Query<&AnimBound>,
    mut tags: Query<&mut MeshTag>,
) {
    for (mut animal, children) in &mut animals {
        for child in children.iter() {
            let Ok(b) = bound.get(child) else { continue };
            if !b.herd || b.meshes.is_empty() {
                continue;
            }
            if !animal.shader {
                animal.shader = true;
            }
            let tag = motion::pack_herd_tag(animal.leg_phase, animal.moving);
            for mesh in &b.meshes {
                if let Ok(mut t) = tags.get_mut(*mesh)
                    && t.value != tag
                {
                    t.value = tag;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> MotionTable {
        let root = crate::paths::find_root().expect("root");
        MotionTable::load(&crate::paths::data_dir(&root))
    }

    #[test]
    fn prop_params_follow_the_data() {
        let t = table();
        let horse = t.spec("animal_horse_draught").unwrap();
        let p = AnimParams::for_prop(horse, 1.0);
        assert_eq!(p.frame.z, 1.0, "a beast is animated");
        assert!((p.gait.x - 1.76).abs() < 1e-4, "stride of the horse");
        assert!((p.body.y - 0.94).abs() < 1e-4, "half length of the model");
        // The chewing of the camp horse file reaches the campaign horse.
        assert!((p.breath.z - 1.11).abs() < 1e-4, "{}", p.breath.z);
        assert_eq!(p.swing[0].y, horse.swing_lut[1]);
        assert_eq!(p.lift[3].w, horse.lift_lut[15]);
        let cart = t.spec("cart_merchant").unwrap();
        let c = AnimParams::for_prop(cart, 0.0);
        assert_eq!(c.frame.z, 0.0);
        assert!((c.cart.x - 0.6).abs() < 1e-4, "wheel radius");
        assert_eq!(c.axle.z, 1.0, "two axles");
        assert!((c.axle.x + 1.32).abs() < 1e-4);
        assert!((c.cart.w - cart.jolt_period_m).abs() < 1e-4);
        assert!(c.jolt_amp.x > 0.0 && c.jolt_ratio.x > 1.0);
    }

    #[test]
    fn herd_params_measure_the_mesh() {
        let gait = Gait {
            cadence: 1.2,
            swing: 0.15,
            lift: 0.06,
            bounce: 0.02,
            graze: 0.2,
            speed_m_s: 0.4,
        };
        let p = AnimParams::for_herd(&gait, Vec3::new(-0.5, 0.0, -1.0), Vec3::new(0.5, 1.0, 1.0));
        assert_eq!(p.herd_size.z, 0.0, "long axis is Z");
        assert!((p.herd_size.x - 2.0).abs() < 1e-6);
        let q = AnimParams::for_herd(&gait, Vec3::new(-1.0, 0.0, -0.5), Vec3::new(1.0, 1.0, 0.5));
        assert_eq!(q.herd_size.z, 1.0, "long axis is X");
        assert_eq!(q.herd_origin.w, 0.2);
    }
}
