//! Birds (`map_bird_flocks.gd`, species in `data/map/map_birds.json`): orbiting crows, starlings,
//! gulls and raptors, geese in V in transit, waders on the wetlands, storks perched on towns.
//! Flocks are anchored on the world cell around the focus (`flock_area`) and picked by habitat
//! (land mask, coast, altitude, wetland sites, listed towns) and season weights; a flock whose
//! cell is left fades out and a new one fades in. Silhouette = a V of two wing triangles (no
//! generated model is wired in `map_birds.json`).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::anim::{AnimBind, AnimKind};
use super::data::{BirdSpecies, LifeData, h01, text_seed};
use super::models::{Fade, FitAxis, ModelMetrics, spawn_model};
use super::stream::{Cat, LifeIndex, LifeItem, LifeView};
use crate::app::AppState;
use crate::campaign::terrain::surface::{GroundHeight, Surface};

#[derive(Component)]
pub struct Flock {
    pub species: usize,
    pub slot: usize,
    pub cell: IVec2,
    pub anchor: Vec2,
}

#[derive(Component)]
pub struct Bird {
    pub phase: f32,
    pub radius: f32,
    pub speed: f32,
    pub height: f32,
    pub jitter_phase: f32,
    /// Offset in the V (transit) or walking centre (wade).
    pub slot: Vec2,
}

/// Marker on a bird drawn from a generated glb (`dn_model`) instead of the wing-triangle V: its
/// wings beat in the vertex stage (`AnimKind::Bird`).
#[derive(Component)]
pub struct ModelBird;

#[derive(Component)]
pub struct Wing {
    pub side: f32,
}

#[derive(Resource)]
pub struct BirdAssets {
    pub wing: Handle<Mesh>,
    pub materials: Vec<Handle<StandardMaterial>>,
}

pub fn wing_mesh() -> Mesh {
    // Triangle in the XZ plane pointing along +X (tip) with the root on the body axis, bent
    // slightly up; double sided by listing both windings.
    let positions = vec![
        [0.0, 0.0, -0.06],
        [0.0, 0.0, 0.10],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ];
    let _ = positions;
    // Wing along +Z (span), chord along X: root at the origin, tip at z = 0.5.
    let v = vec![[-0.08f32, 0.0, 0.0], [0.10, 0.0, 0.0], [0.0, 0.03, 0.5]];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; 3]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; 3]);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
    mesh
}

pub fn build_assets(
    mut commands: Commands,
    data: Res<LifeData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wing = meshes.add(wing_mesh());
    let mats = data
        .birds
        .species
        .iter()
        .map(|s| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(s.color[0], s.color[1], s.color[2]),
                perceptual_roughness: 1.0,
                cull_mode: None,
                double_sided: true,
                ..default()
            })
        })
        .collect();
    commands.insert_resource(BirdAssets {
        wing,
        materials: mats,
    });
}

fn season_count(sp: &BirdSpecies, season: usize) -> usize {
    (sp.flocks as f32 * sp.seasons[season.min(3)]).ceil() as usize
}

fn coast_near(data: &LifeData, p: Vec2, max_px: f32) -> bool {
    let Some(land) = data.land.as_ref() else {
        return false;
    };
    let here = land.is_land(p);
    for step in [max_px * 0.34, max_px * 0.67, max_px] {
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            if land.is_land(p + Vec2::new(a.cos(), a.sin()) * step) != here {
                return true;
            }
        }
    }
    false
}

/// Anchor of the flock `(species, slot)` in `cell`, or `None` if no habitat there.
pub fn pick_anchor(
    data: &LifeData,
    sp: &BirdSpecies,
    species: usize,
    slot: usize,
    cell: IVec2,
    height_m: &impl Fn(Vec2) -> f32,
) -> Option<Vec2> {
    let area = data.birds.flock_area.max(1.0);
    let seed = text_seed(&sp.id) ^ ((slot as u64) << 32);
    let cell_key = (cell.x as i64 as u64).wrapping_mul(73_856_093)
        ^ (cell.y as i64 as u64).wrapping_mul(19_349_663);
    if !sp.towns.is_empty() {
        let town = sp.towns[slot % sp.towns.len()].as_str();
        let px = data.towns.get(town)?;
        // Perched on the town of the slot, only while the focus is within a cell of it.
        let c = (*px / area).floor().as_ivec2();
        if (c - cell).abs().max_element() > 0 {
            return None;
        }
        let a = h01(seed, 1, 0) * std::f32::consts::TAU;
        let r = sp.town_radius * h01(seed, 2, 0);
        return Some(*px + Vec2::new(a.cos(), a.sin()) * r * 0.3);
    }
    if !sp.wetland_ids.is_empty() {
        let id = &sp.wetland_ids[(h01(seed, cell_key, 5) * sp.wetland_ids.len() as f32) as usize
            % sp.wetland_ids.len()];
        let (c, rad) = data.wetlands.get(id)?;
        let wc = (*c / area).floor().as_ivec2();
        if (wc - cell).abs().max_element() > 1 {
            return None;
        }
        let a = h01(seed, cell_key, 6) * std::f32::consts::TAU;
        return Some(*c + Vec2::new(a.cos(), a.sin()) * rad * h01(seed, cell_key, 7).sqrt());
    }
    for k in 0..16u64 {
        let p = (cell.as_vec2()
            + Vec2::new(h01(seed, cell_key, 10 + k), h01(seed, cell_key, 40 + k)))
            * area;
        let land = data.land.as_ref().is_some_and(|l| l.is_land(p));
        let ok_surface = match sp.surface.as_str() {
            "land" => land && !data.in_lake(p),
            "water" => !land,
            _ => true,
        };
        if !ok_surface {
            continue;
        }
        if let Some(c) = sp.coast_max_px
            && !coast_near(data, p, c)
        {
            continue;
        }
        if sp.min_height_m.is_some() || sp.max_height_m.is_some() {
            let m = height_m(p);
            if sp.min_height_m.is_some_and(|lo| m < lo) || sp.max_height_m.is_some_and(|hi| m > hi)
            {
                continue;
            }
        }
        return Some(p);
    }
    let _ = species;
    None
}

pub fn manage_flocks(
    mut commands: Commands,
    data: Res<LifeData>,
    view: Res<LifeView>,
    assets: Option<Res<BirdAssets>>,
    asset_server: Res<AssetServer>,
    metrics: Res<ModelMetrics>,
    surface: Option<Res<Surface>>,
    mut index: ResMut<LifeIndex>,
    mut flocks: Query<(&Flock, &mut Fade)>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = data.birds.retarget_seconds.max(0.2) * 0.5;
    let Some(assets) = assets else { return };
    if !view.ready {
        return;
    }
    // Birds are drawn in the near tier (`ZoomView::near`).
    let near = view.zoom_near > 0.0;
    let area = data.birds.flock_area.max(1.0);
    let cell = (view.focus / area).floor().as_ivec2();
    let height_m = |p: Vec2| surface.as_ref().map_or(0.0, |s| s.height_m_at(p.x, p.y));
    // Retire flocks of another cell or of a closed season.
    for (flock, mut fade) in &mut flocks {
        let sp = &data.birds.species[flock.species];
        let stale = flock.cell != cell || flock.slot >= season_count(sp, view.season) || !near;
        fade.target = if stale { 0.0 } else { 1.0 };
    }
    if !near {
        return;
    }
    for (s, sp) in data.birds.species.iter().enumerate() {
        for slot in 0..season_count(sp, view.season) {
            let key = text_seed("life:flock")
                ^ (s as u64) << 48
                ^ (slot as u64) << 40
                ^ (cell.x as i64 as u64).wrapping_mul(73_856_093)
                ^ (cell.y as i64 as u64).wrapping_mul(19_349_663);
            if index.0.contains(&key) {
                continue;
            }
            let Some(anchor) = pick_anchor(&data, sp, s, slot, cell, &height_m) else {
                continue;
            };
            index.0.insert(key);
            let seed = text_seed(&sp.id) ^ key;
            let flock_entity = commands
                .spawn((
                    Name::new(format!("life:flock:{}", sp.id)),
                    Transform::from_xyz(anchor.x, 0.0, anchor.y).with_scale(Vec3::splat(1e-4)),
                    Visibility::Inherited,
                    Flock {
                        species: s,
                        slot,
                        cell,
                        anchor,
                    },
                    LifeItem {
                        key,
                        cat: Cat::Bird,
                        anchor,
                        h: 0.0,
                    },
                    Fade::new(1.0),
                    DespawnOnExit(AppState::Campaign),
                ))
                .id();
            let n = sp.per_flock;
            for b in 0..n {
                let hb = |salt: u64| h01(seed, b as u64, salt);
                let lerp = |r: [f32; 2], t: f32| r[0] + (r[1] - r[0]) * t;
                // Slot in the V (transit): two arms behind the leader.
                let arm = if b == 0 { 0.0 } else { b.div_ceil(2) as f32 };
                let side = if b % 2 == 1 { 1.0 } else { -1.0 };
                let ang = sp.v_angle_deg.to_radians();
                let v_slot = Vec2::new(
                    -arm * sp.v_spacing * ang.cos(),
                    side * arm * sp.v_spacing * ang.sin(),
                );
                let glb = (!sp.dn_model.is_empty())
                    .then(|| {
                        let m = data.water_models.models.get(&sp.dn_model)?;
                        Some((data.water_models.scene_path(&sp.dn_model)?, m))
                    })
                    .flatten();
                let bird_data = Bird {
                    phase: hb(1) * std::f32::consts::TAU,
                    radius: lerp(sp.radius, hb(2)),
                    speed: lerp(sp.speed, hb(3)),
                    height: lerp(sp.height, hb(4)),
                    jitter_phase: hb(5) * std::f32::consts::TAU,
                    slot: v_slot,
                };
                let bird = if let Some((path, model)) = &glb {
                    // Beak on -Z after `yaw_deg` (as the Godot bake), then turned to the +X heading axis.
                    let yaw_fix = model.yaw_deg.to_radians() - std::f32::consts::FRAC_PI_2;
                    let flapping = matches!(sp.behavior.as_str(), "transit" | "orbit");
                    let body = spawn_model(
                        &mut commands,
                        &asset_server,
                        &metrics,
                        path,
                        FitAxis::Length,
                        model.length * sp.scale,
                        yaw_fix,
                    );
                    // The wing beat is a vertex shader (`anim_pose.wesl::bird_pose`, as
                    // `life_birds.gdshader`); the glb is turned by `yaw_deg` into the Godot frame.
                    commands.entity(body).try_insert(AnimBind {
                        kind: AnimKind::Bird {
                            flap_rate: sp.flap_rate,
                            flap_amount: sp.flap_amount,
                            wing_fold: sp.wing_fold,
                            flapping,
                        },
                        seed: bird_data.phase / std::f32::consts::TAU,
                        yaw: model.yaw_deg.to_radians(),
                    });
                    commands
                        .spawn((
                            Name::new("life:bird"),
                            Transform::default(),
                            Visibility::Inherited,
                            bird_data,
                            ModelBird,
                        ))
                        .add_child(body)
                        .id()
                } else {
                    commands
                        .spawn((
                            Name::new("life:bird"),
                            Transform::default(),
                            Visibility::Inherited,
                            bird_data,
                        ))
                        .with_children(|p| {
                            for side in [-1.0f32, 1.0] {
                                p.spawn((
                                    Mesh3d(assets.wing.clone()),
                                    MeshMaterial3d(assets.materials[s].clone()),
                                    Transform::from_scale(Vec3::new(
                                        sp.scale,
                                        sp.scale,
                                        side * sp.scale,
                                    )),
                                    Wing { side },
                                ));
                            }
                        })
                        .id()
                };
                commands.entity(flock_entity).add_child(bird);
            }
        }
    }
}

/// Moves the birds of every flock (positions are local to the flock anchor).
pub fn move_birds(
    data: Res<LifeData>,
    view: Res<LifeView>,
    ground: GroundHeight,
    flocks: Query<(&Flock, &Children, &Transform), Without<Bird>>,
    mut birds: Query<(&Bird, &mut Transform, &Children, Has<ModelBird>), Without<Wing>>,
    mut wings: Query<(&Wing, &mut Transform), (Without<Bird>, Without<Flock>)>,
) {
    let t = view.time;
    for (flock, children, flock_t) in &flocks {
        let sp = &data.birds.species[flock.species];
        let ground_y = ground.at(flock.anchor.x, flock.anchor.y).unwrap_or(0.0);
        let base_y = ground_y - flock_t.translation.y;
        for child in children.iter() {
            let Ok((bird, mut bt, wing_children, _model_bird)) = birds.get_mut(child) else {
                continue;
            };
            let jitter = |k: f32| sp.jitter * (t * 0.9 + bird.jitter_phase + k).sin() * 0.5;
            let (pos, heading, flap) = match sp.behavior.as_str() {
                "transit" => {
                    // Travels along +X of the wind-free flight line, wrapping over the span.
                    let dir = Vec2::new(0.9, 0.44).normalize();
                    let along = (t * bird.speed + bird.phase * 0.0).rem_euclid(sp.span.max(1.0))
                        - sp.span * 0.5;
                    let rot = Vec2::new(
                        bird.slot.x * dir.x - bird.slot.y * dir.y,
                        bird.slot.x * dir.y + bird.slot.y * dir.x,
                    );
                    let p = dir * along + rot;
                    (Vec3::new(p.x, bird.height, p.y), dir, true)
                }
                "orbit" => {
                    let a = bird.phase + t * bird.speed;
                    let p = Vec2::new(a.cos(), a.sin()) * bird.radius;
                    let tangent = Vec2::new(-a.sin(), a.cos());
                    (
                        Vec3::new(
                            p.x + jitter(0.0),
                            bird.height + jitter(2.0) * 0.5,
                            p.y + jitter(1.0),
                        ),
                        tangent,
                        true,
                    )
                }
                "wade" => {
                    let a = bird.phase + t * bird.speed;
                    let p = Vec2::new(a.cos(), (a * 1.3).sin()) * bird.radius;
                    let tangent = Vec2::new(-a.sin(), a.cos() * 1.3);
                    (Vec3::new(p.x, bird.height * 0.2, p.y), tangent, false)
                }
                _ => {
                    // perch: on the spot, heading drifts a little.
                    let a = bird.phase + t * 0.05;
                    (
                        Vec3::new(
                            bird.slot.x * 0.0 + bird.radius * a.cos() * 0.0,
                            bird.height,
                            0.0,
                        ),
                        Vec2::new(a.cos(), a.sin()),
                        false,
                    )
                }
            };
            bt.translation = pos + Vec3::Y * base_y;
            bt.rotation = Quat::from_rotation_y(-heading.y.atan2(heading.x));
            // Wing flap: both wings rotate about the body axis (X).
            let angle = if flap {
                (t * sp.flap_rate + bird.phase).sin() * sp.flap_amount
            } else {
                0.1
            };
            for wing_entity in wing_children.iter() {
                if let Ok((wing, mut wt)) = wings.get_mut(wing_entity) {
                    wt.rotation = Quat::from_rotation_x(-(angle * wing.side));
                }
            }
        }
    }
}
