//! The walls of a siege battle (port of `battle_siege.gd` ring, towers, gate and houses, and of
//! the state side of `wall_collapse_fx.gd` / `siege_marks_fx.gd`).
//!
//! Each [`sim_battle::WallPiece`] is one entity: `WallView` root (placed at the middle of the
//! piece, local +x along it) -> `Body` pivot (sinks when the piece is breached) -> wall volume,
//! crenels, the door of the gate, scars. Rubble (hidden) stands next to the pivot and shows once
//! the piece has fallen. Towers are cylinders with a conical roof. Houses are merged volumes
//! (block + gabled roof; the church has a tower) swapped to a "burning" tint and finally a low
//! ruin when the core's fire says so.

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sim_battle::siege::House;
use sim_battle::{FireState, PieceKind, SiegeFxKind};

use super::{SiegeFrame, SiegeStats};
use crate::app::AppState;
use crate::battle::decor::hash01;
use crate::battle::scene::BattleSession;

const STONE: [f32; 3] = [0.62, 0.60, 0.56];
const STONE_DARK: [f32; 3] = [0.33, 0.31, 0.29];
const TILE: [f32; 3] = [0.50, 0.26, 0.20];
const WOOD: [f32; 3] = [0.36, 0.25, 0.15];
const PLASTER: [f32; 3] = [0.76, 0.70, 0.58];
const SLATE: [f32; 3] = [0.34, 0.36, 0.40];
/// Seconds of the collapse animation of a breached piece.
pub const COLLAPSE_S: f32 = 2.4;
/// Share of the height a fallen piece keeps (rubble heap).
const FALLEN_SCALE: f32 = 0.14;
/// Scars kept on one piece (oldest are not removed: the count is capped).
const MAX_SCARS: usize = 14;

/// Lifecycle of a piece.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PieceLife {
    Standing,
    Collapsing(f32),
    Fallen,
}

/// Root of one wall piece.
#[derive(Component)]
pub struct WallView {
    pub index: usize,
    pub gate: bool,
    pub life: PieceLife,
    pub scars: usize,
    pub shown_share: f32,
    pub material: Handle<StandardMaterial>,
    pub pivot: Entity,
    pub rubble: Entity,
    pub door: Option<Entity>,
    /// +1 when local +z is the outside of the town.
    pub outward_sign: f32,
    /// Roll of the fall, radians (sign = direction).
    pub roll: f32,
    pub base_y: f32,
    pub half_thickness: f32,
}

#[derive(Component)]
pub struct HouseView {
    pub index: usize,
    pub state: FireState,
    pub height_scale: f32,
}

#[derive(Resource)]
struct HouseMaterials {
    normal: Handle<StandardMaterial>,
    burning: Handle<StandardMaterial>,
    ruin: Handle<StandardMaterial>,
}

/// Merged volumes with vertex colours (a tiny cousin of `decor::site::Volumes`, which keeps its
/// buffers private).
#[derive(Default)]
pub struct Boxes {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Boxes {
    fn quad(&mut self, c: [Vec3; 4], color: [f32; 3]) {
        let normal = (c[1] - c[0]).cross(c[3] - c[0]).normalize_or_zero();
        let base = self.positions.len() as u32;
        for corner in c {
            self.positions.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.colors.push([color[0], color[1], color[2], 1.0]);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Box with its centre bottom at `foot`, half extents `half` (x, z) and height `height`,
    /// turned by `yaw` (sim convention, local +x = `(cos yaw, sin yaw)`).
    pub fn cuboid(&mut self, foot: Vec3, half: Vec2, height: f32, yaw: f32, color: [f32; 3]) {
        let rot = Quat::from_rotation_y(-yaw);
        let p = |sx: f32, sy: f32, sz: f32| {
            foot + rot * Vec3::new(sx * half.x, sy * height, sz * half.y)
        };
        let (a, b, c, d) = (
            p(-1., 0., -1.),
            p(1., 0., -1.),
            p(1., 0., 1.),
            p(-1., 0., 1.),
        );
        let (e, f, g, h) = (
            p(-1., 1., -1.),
            p(1., 1., -1.),
            p(1., 1., 1.),
            p(-1., 1., 1.),
        );
        let shade = |k: f32| [color[0] * k, color[1] * k, color[2] * k];
        self.quad([e, h, g, f], color);
        self.quad([a, e, f, b], shade(0.88));
        self.quad([b, f, g, c], shade(0.82));
        self.quad([c, g, h, d], shade(0.9));
        self.quad([d, h, e, a], shade(0.82));
    }

    /// Gabled roof over a `half` footprint, ridge along local x, eaves at `foot.y`.
    pub fn gable(&mut self, foot: Vec3, half: Vec2, rise: f32, yaw: f32, color: [f32; 3]) {
        let rot = Quat::from_rotation_y(-yaw);
        let p = |sx: f32, y: f32, sz: f32| foot + rot * Vec3::new(sx * half.x, y, sz * half.y);
        let (a, b, c, d) = (
            p(-1., 0., -1.),
            p(1., 0., -1.),
            p(1., 0., 1.),
            p(-1., 0., 1.),
        );
        let (r0, r1) = (p(-1., rise, 0.), p(1., rise, 0.));
        let shade = |k: f32| [color[0] * k, color[1] * k, color[2] * k];
        self.quad([a, r0, r1, b], shade(0.9));
        self.quad([c, r1, r0, d], color);
        let tri = |s: &mut Boxes, t: [Vec3; 3], col: [f32; 3]| {
            let normal = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
            let base = s.positions.len() as u32;
            for v in t {
                s.positions.push(v.to_array());
                s.normals.push(normal.to_array());
                s.colors.push([col[0], col[1], col[2], 1.0]);
            }
            s.indices.extend_from_slice(&[base, base + 1, base + 2]);
        };
        tri(self, [a, d, r0], shade(0.8));
        tri(self, [b, r1, c], shade(0.8));
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

fn vertex_material(
    materials: &mut Assets<StandardMaterial>,
    tint: Color,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: tint,
        perceptual_roughness: 0.95,
        reflectance: 0.04,
        cull_mode: None,
        double_sided: true,
        ..default()
    })
}

pub struct WorksPlugin;

impl Plugin for WorksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, build_works.run_if(resource_added::<BattleSession>))
            .add_systems(
                Update,
                (update_pieces, update_houses, add_scars)
                    .chain()
                    .after(super::read_frame)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<HouseMaterials>),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<HouseMaterials>();
            });
    }
}

/// Church or house block, as merged volumes centred on the origin (placed by the entity).
fn house_mesh(house: &House, index: usize) -> Boxes {
    let mut boxes = Boxes::default();
    let (length, depth) = if house.length > 0.0 {
        (house.length as f32, house.depth as f32)
    } else {
        let r = house.radius as f32;
        (r * 1.7, r * 1.7)
    };
    let half = Vec2::new(length * 0.5 * 0.95, depth * 0.5 * 0.95);
    let jitter = hash01(house.x as f32, house.z as f32, index as f32);
    if house.church {
        let nave_h = 8.0;
        boxes.cuboid(Vec3::ZERO, half, nave_h, 0.0, STONE);
        boxes.gable(Vec3::Y * nave_h, half, 4.0, 0.0, SLATE);
        let tower = Vec2::splat((half.y * 0.55).max(2.0));
        boxes.cuboid(
            Vec3::new(-half.x + tower.x, 0.0, 0.0),
            tower,
            nave_h + 7.0,
            0.0,
            STONE,
        );
        boxes.gable(
            Vec3::new(-half.x + tower.x, nave_h + 7.0, 0.0),
            tower,
            4.5,
            PI * 0.5,
            SLATE,
        );
    } else {
        let wall_h = if house.rows >= 2 { 6.0 } else { 4.5 } + jitter * 1.2;
        boxes.cuboid(Vec3::ZERO, half, wall_h, 0.0, PLASTER);
        let roof = if jitter > 0.5 { TILE } else { SLATE };
        boxes.gable(Vec3::Y * wall_h, half, wall_h * 0.45, 0.0, roof);
    }
    boxes
}

fn rubble_mesh(length: f32, thickness: f32, seed: f32) -> Boxes {
    let mut boxes = Boxes::default();
    let count = ((length / 2.4) as usize).clamp(3, 14);
    for k in 0..count {
        let t = (k as f32 + 0.5) / count as f32 - 0.5;
        let r = |salt: f32| hash01(seed + k as f32, seed * 0.7, salt);
        let s = 0.7 + r(1.0) * 1.3;
        boxes.cuboid(
            Vec3::new(t * length, 0.0, (r(2.0) - 0.5) * thickness * 2.2),
            Vec2::new(s, s * 0.9),
            0.9 + r(3.0) * 1.8,
            r(4.0) * TAU,
            if r(5.0) > 0.5 { STONE } else { STONE_DARK },
        );
    }
    boxes
}

fn build_works(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stats: ResMut<SiegeStats>,
    assets: Res<AssetServer>,
) {
    let sim = session.sim_mut();
    let Some(works) = sim.siege() else {
        return;
    };
    let field = sim.field();
    let ground = |x: f64, z: f64| field.height(x, z) as f32;
    let scope = DespawnOnExit(AppState::Battle);
    let thickness = works.thickness as f32;
    let wall_h = works.wall_height as f32;
    let door_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(WOOD[0], WOOD[1], WOOD[2]),
        perceptual_roughness: 0.9,
        ..default()
    });
    let rubble_mat = vertex_material(&mut materials, Color::WHITE);
    // Wall stone (Godot: `castle_wall_varriation`, one tile per 4.5 m), repeating.
    let repeat = |linear: bool| {
        move |settings: &mut bevy::image::ImageLoaderSettings| {
            settings.is_srgb = !linear;
            settings.sampler =
                bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                    address_mode_u: bevy::image::ImageAddressMode::Repeat,
                    address_mode_v: bevy::image::ImageAddressMode::Repeat,
                    address_mode_w: bevy::image::ImageAddressMode::Repeat,
                    mag_filter: bevy::image::ImageFilterMode::Linear,
                    min_filter: bevy::image::ImageFilterMode::Linear,
                    mipmap_filter: bevy::image::ImageFilterMode::Linear,
                    anisotropy_clamp: 8,
                    ..default()
                });
        }
    };
    let wall_diffuse: Handle<Image> = assets
        .load_builder()
        .with_settings(repeat(false))
        .load("textures/battle/castle_wall_varriation_diff.jpg");
    let wall_normal: Handle<Image> = assets
        .load_builder()
        .with_settings(repeat(true))
        .load("textures/battle/castle_wall_varriation_nor.jpg");
    for (index, piece) in works.pieces.iter().enumerate() {
        let (ax, az) = piece.a;
        let (bx, bz) = piece.b;
        let (mx, mz) = piece.midpoint();
        let length = (((bx - ax).powi(2) + (bz - az).powi(2)).sqrt()) as f32;
        let theta = ((bz - az) as f32).atan2((bx - ax) as f32);
        let (ga, gb, gm) = (ground(ax, az), ground(bx, bz), ground(mx, mz));
        let base = ga.min(gb).min(gm) - 1.4;
        let top = (ga + gb + gm) / 3.0 + wall_h;
        let height = top - base;
        let gate = piece.kind == PieceKind::Gate;
        let (ox, oz) = piece.outward();
        // Local +z in the world is (-sin theta, cos theta).
        let outward_sign = if (ox as f32) * -theta.sin() + (oz as f32) * theta.cos() >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(STONE[0] * 1.5, STONE[1] * 1.5, STONE[2] * 1.5),
            base_color_texture: Some(wall_diffuse.clone()),
            normal_map_texture: Some(wall_normal.clone()),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(length / 4.5, height / 4.5)),
            perceptual_roughness: 0.95,
            reflectance: 0.04,
            ..default()
        });
        let intact = piece.intact();
        let mut door = None;
        let pivot = commands
            .spawn((
                Name::new(format!("SiegePivot{index}")),
                Transform::default(),
                Visibility::default(),
            ))
            .with_children(|p| {
                let (body_h, body_y) = if gate {
                    // Lintel above a 5 m opening.
                    (height - 5.0, 5.0 + (height - 5.0) * 0.5)
                } else {
                    (height, height * 0.5)
                };
                p.spawn((
                    Name::new("SiegeWall"),
                    Mesh3d(meshes.add(Cuboid::new(
                        length,
                        body_h,
                        thickness + if gate { 0.6 } else { 0.0 },
                    ))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, body_y, 0.0),
                ));
                // Crenels along the outer edge of the wall walk.
                let mut crenels = Boxes::default();
                let count = ((length / 2.2) as usize).max(2);
                for k in 0..count {
                    let x = -length * 0.5 + (k as f32 + 0.5) * length / count as f32;
                    if !gate || x.abs() > 0.0 {
                        crenels.cuboid(
                            Vec3::new(x, height, outward_sign * (thickness * 0.5 - 0.3)),
                            Vec2::new(0.55, 0.3),
                            1.0,
                            0.0,
                            STONE,
                        );
                    }
                }
                p.spawn((
                    Name::new("SiegeCrenels"),
                    Mesh3d(meshes.add(crenels.into_mesh())),
                    MeshMaterial3d(rubble_mat.clone()),
                    Transform::default(),
                ));
                if gate {
                    door = Some(
                        p.spawn((
                            Name::new("SiegeGateDoor"),
                            Mesh3d(meshes.add(Cuboid::new(length - 1.0, 5.0, 0.5))),
                            MeshMaterial3d(door_mat.clone()),
                            Transform::from_xyz(0.0, 2.5, outward_sign * (thickness * 0.5 - 0.2)),
                            if intact {
                                Visibility::Inherited
                            } else {
                                Visibility::Hidden
                            },
                        ))
                        .id(),
                    );
                }
            })
            .id();
        let rubble = commands
            .spawn((
                Name::new("SiegeRubble"),
                Mesh3d(meshes.add(rubble_mesh(length, thickness, index as f32 * 3.1).into_mesh())),
                MeshMaterial3d(rubble_mat.clone()),
                Transform::default(),
                if intact {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
            ))
            .id();
        let roll = if hash01(mx as f32, mz as f32, 9.0) > 0.5 {
            0.14
        } else {
            -0.14
        };
        let root = commands
            .spawn((
                Name::new(format!("SiegeWall{index}")),
                WallView {
                    index,
                    gate,
                    life: if intact {
                        PieceLife::Standing
                    } else {
                        PieceLife::Fallen
                    },
                    scars: 0,
                    shown_share: 1.0,
                    material,
                    pivot,
                    rubble,
                    door,
                    outward_sign,
                    roll,
                    base_y: base,
                    half_thickness: thickness * 0.5,
                },
                Transform::from_xyz(mx as f32, base, mz as f32)
                    .with_rotation(Quat::from_rotation_y(-theta)),
                Visibility::default(),
                scope.clone(),
            ))
            .add_children(&[pivot, rubble])
            .id();
        let _ = root;
        if !intact {
            let mut pivot_transform = Transform::default();
            pivot_transform.scale.y = FALLEN_SCALE;
            commands.entity(pivot).insert(pivot_transform);
            stats.breaches += 1;
        }
        stats.pieces += 1;
    }
    // Towers.
    let tower_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(STONE[0] * 1.4, STONE[1] * 1.4, STONE[2] * 1.4),
        base_color_texture: Some(wall_diffuse.clone()),
        normal_map_texture: Some(wall_normal.clone()),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(2.0, 2.0)),
        perceptual_roughness: 0.95,
        ..default()
    });
    let roof_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(TILE[0], TILE[1], TILE[2]),
        perceptual_roughness: 0.9,
        ..default()
    });
    for tower in &works.towers {
        let base = ground(tower.x, tower.z) - 1.4;
        let top = ground(tower.x, tower.z) + tower.height as f32;
        let radius = tower.radius as f32;
        commands.spawn((
            Name::new("SiegeTower"),
            Mesh3d(meshes.add(Cylinder::new(radius, top - base).mesh().resolution(12))),
            MeshMaterial3d(tower_mat.clone()),
            Transform::from_xyz(tower.x as f32, (base + top) * 0.5, tower.z as f32),
            scope.clone(),
        ));
        commands.spawn((
            Name::new("SiegeTowerRoof"),
            Mesh3d(meshes.add(Cone::new(radius * 1.25, 5.0).mesh().resolution(12))),
            MeshMaterial3d(roof_mat.clone()),
            Transform::from_xyz(tower.x as f32, top + 2.5, tower.z as f32),
            scope.clone(),
        ));
        stats.towers += 1;
    }
    // Houses.
    let normal = vertex_material(&mut materials, Color::WHITE);
    let burning = vertex_material(&mut materials, Color::srgb(1.0, 0.55, 0.32));
    let ruin = vertex_material(&mut materials, Color::srgb(0.22, 0.2, 0.19));
    // The kit town takes over every house it has a model for; the blocks draw the rest.
    let kit = crate::paths::find_root()
        .map(|root| crate::campaign::towns::kit::Kit::load(&crate::paths::asset_root(&root)))
        .unwrap_or_default();
    let kit_houses =
        super::kit_town::spawn_town(&mut commands, &assets, works, &kit, &ground, &mut stats);
    for (index, house) in works.houses.iter().enumerate() {
        if kit_houses.contains(&index) {
            stats.houses += 1;
            continue;
        }
        let boxes = house_mesh(house, index);
        let (hx, hz) = (house.x, house.z);
        let low = ground(hx, hz);
        let state = house.fire.state;
        let (material, scale) = match state {
            FireState::Intact => (normal.clone(), 1.0),
            FireState::Burning => (burning.clone(), 1.0),
            FireState::Burnt => (ruin.clone(), 0.3),
        };
        commands.spawn((
            Name::new(format!("SiegeHouse{index}")),
            HouseView {
                index,
                state,
                height_scale: scale,
            },
            Mesh3d(meshes.add(boxes.into_mesh())),
            MeshMaterial3d(material),
            Transform::from_xyz(hx as f32, low - 0.8, hz as f32)
                .with_rotation(Quat::from_rotation_y(-(house.yaw as f32)))
                .with_scale(Vec3::new(1.0, scale, 1.0)),
            scope.clone(),
        ));
        stats.houses += 1;
    }
    commands.insert_resource(HouseMaterials {
        normal,
        burning,
        ruin,
    });
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Damage tint, collapse animation and the door of the gate.
fn update_pieces(
    time: Res<Time>,
    frame: Res<SiegeFrame>,
    mut stats: ResMut<SiegeStats>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut views: Query<&mut WallView>,
    mut transforms: Query<&mut Transform>,
    mut visibility: Query<&mut Visibility>,
) {
    if !frame.active {
        return;
    }
    for mut view in &mut views {
        let Some(state) = frame.pieces.get(view.index).copied() else {
            continue;
        };
        let share = if state.max_hp > 0.0 {
            (state.hp / state.max_hp).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if (share - view.shown_share).abs() > 0.02 {
            view.shown_share = share;
            if let Some(mut material) = materials.get_mut(&view.material) {
                let k = (1.0 - share) * 0.6;
                material.base_color = Color::srgb(
                    STONE[0] + (STONE_DARK[0] - STONE[0]) * k,
                    STONE[1] + (STONE_DARK[1] - STONE[1]) * k,
                    STONE[2] + (STONE_DARK[2] - STONE[2]) * k,
                );
            }
        }
        if state.hp <= 0.0 && view.life == PieceLife::Standing {
            view.life = PieceLife::Collapsing(0.0);
            stats.collapses_played += 1;
            if let Some(door) = view.door
                && let Ok(mut v) = visibility.get_mut(door)
            {
                *v = Visibility::Hidden;
            }
            if let Ok(mut v) = visibility.get_mut(view.rubble) {
                *v = Visibility::Inherited;
            }
        }
        if let PieceLife::Collapsing(t) = view.life {
            let t = t + time.delta_secs();
            let k = smooth(t / COLLAPSE_S);
            if let Ok(mut transform) = transforms.get_mut(view.pivot) {
                transform.scale.y = 1.0 + (FALLEN_SCALE - 1.0) * k;
                let wobble = (k * PI).sin() * view.roll;
                transform.rotation = Quat::from_rotation_x(wobble);
            }
            view.life = if t >= COLLAPSE_S {
                if let Ok(mut transform) = transforms.get_mut(view.pivot) {
                    transform.rotation = Quat::IDENTITY;
                }
                PieceLife::Fallen
            } else {
                PieceLife::Collapsing(t)
            };
        }
    }
}

/// Burning houses take the fire tint, burnt ones sink into a low ruin.
fn update_houses(
    frame: Res<SiegeFrame>,
    materials: Res<HouseMaterials>,
    mut views: Query<(
        &mut HouseView,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Transform,
    )>,
) {
    for (mut view, mut material, mut transform) in &mut views {
        let Some(fire) = frame.houses.get(view.index) else {
            continue;
        };
        if fire.state == view.state {
            continue;
        }
        view.state = fire.state;
        match fire.state {
            FireState::Intact => {
                material.0 = materials.normal.clone();
                transform.scale.y = 1.0;
            }
            FireState::Burning => {
                material.0 = materials.burning.clone();
                transform.scale.y = 1.0;
            }
            FireState::Burnt => {
                material.0 = materials.ruin.clone();
                transform.scale.y = 0.3;
            }
        }
        view.height_scale = transform.scale.y;
    }
}

/// A dark scar where an engine's stone or the ram struck (the pivot carries it down with the wall).
fn add_scars(
    mut commands: Commands,
    frame: Res<SiegeFrame>,
    mut stats: ResMut<SiegeStats>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut views: Query<(&mut WallView, &Transform)>,
    mut scar_assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let (mesh, material) = scar_assets.get_or_insert_with(|| {
        (
            meshes.add(Cuboid::new(1.0, 1.0, 0.25)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.14, 0.12, 0.11),
                perceptual_roughness: 1.0,
                ..default()
            }),
        )
    });
    for event in &frame.events {
        let (piece, world, size) = match event.kind {
            SiegeFxKind::EngineShot { piece, .. } => (piece, event.pos, 1.3),
            SiegeFxKind::RamStrike { piece, .. } => (piece, event.pos + Vec3::Y * 2.0, 1.6),
            _ => continue,
        };
        for (mut view, transform) in &mut views {
            if view.index != piece || view.scars >= MAX_SCARS || view.life != PieceLife::Standing {
                continue;
            }
            let local = transform.rotation.inverse() * (world - transform.translation);
            let local = Vec3::new(
                local.x,
                (world.y - transform.translation.y).max(0.5),
                view.outward_sign * (view.half_thickness + 0.1),
            );
            let scale = size * (0.7 + hash01(world.x, world.z, 3.0) * 0.8);
            let pivot = view.pivot;
            commands.entity(pivot).with_children(|p| {
                p.spawn((
                    Name::new("SiegeScar"),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(local).with_scale(Vec3::new(
                        scale,
                        scale * 0.8,
                        1.0,
                    )),
                ));
            });
            view.scars += 1;
            stats.scars += 1;
        }
    }
}
