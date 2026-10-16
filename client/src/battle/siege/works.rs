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
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sim_battle::siege::House;
use sim_battle::{FireState, PieceKind, SiegeFxKind};

use super::stone::{SiegeStoneMaterial, StoneTextures};
use super::{SiegeFrame, SiegeStats};
use crate::app::AppState;
use crate::battle::decor::hash01;
use crate::battle::scene::BattleSession;
use crate::campaign::towns::atlas::{
    AtlasStyle, BuildingAtlas, BuildingAtlasMaterial, layer_alpha,
};

/// Wall materials of the player's stone by damage step (index = damage level, 0 = intact).
#[derive(Resource, Clone)]
pub struct StoneSet(pub Vec<Handle<SiegeStoneMaterial>>);

const STONE: [f32; 3] = [0.62, 0.60, 0.56];
const STONE_DARK: [f32; 3] = [0.33, 0.31, 0.29];
const TILE: [f32; 3] = [0.50, 0.26, 0.20];
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
    /// The wall volume (its material follows the damage).
    pub body: Entity,
    /// Damage step (0..=DAMAGE_STEPS) of the material shown.
    pub damage_level: u8,
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
    normal: Handle<BuildingAtlasMaterial>,
    burning: Handle<BuildingAtlasMaterial>,
    ruin: Handle<BuildingAtlasMaterial>,
}

/// Steps of the damage tint of a wall (each step is one cached atlas material).
pub const DAMAGE_STEPS: u8 = 6;

/// Damage step of a piece holding `share` of its hit points.
pub fn damage_level(share: f32) -> u8 {
    (((1.0 - share.clamp(0.0, 1.0)) * f32::from(DAMAGE_STEPS)).round() as u8).min(DAMAGE_STEPS)
}

/// Colour multiplier of the atlas masonry at damage `level`: the old flat tint went from `STONE`
/// to `STONE_DARK` over 60 % of the damage; the ratio keeps the same darkening.
pub fn damage_tint(level: u8) -> Vec3 {
    let k = f32::from(level) / f32::from(DAMAGE_STEPS) * 0.6;
    Vec3::new(
        1.0 + (STONE_DARK[0] / STONE[0] - 1.0) * k,
        1.0 + (STONE_DARK[1] / STONE[1] - 1.0) * k,
        1.0 + (STONE_DARK[2] / STONE[2] - 1.0) * k,
    )
}

/// Merged volumes with vertex colours (a tiny cousin of `decor::site::Volumes`, which keeps its
/// buffers private).
#[derive(Default)]
pub struct Boxes {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    /// `(vertex alpha of the atlas matter, reference colour)` of the volumes added from now on:
    /// the vertex colour keeps the shade relative to the reference, the atlas gives the hue.
    layer: Option<(f32, [f32; 3])>,
}

impl Boxes {
    /// The next volumes are atlas matter `name`; `reference` is the colour whose shades the
    /// caller multiplies.
    pub fn set_layer(&mut self, name: &str, reference: [f32; 3]) {
        self.layer = Some((layer_alpha(name), reference));
    }

    /// Vertex colour of `color` under the current matter.
    fn vertex(&self, color: [f32; 3]) -> [f32; 4] {
        match self.layer {
            Some((alpha, r)) => [
                (color[0] / r[0].max(0.01)).min(1.25),
                (color[1] / r[1].max(0.01)).min(1.25),
                (color[2] / r[2].max(0.01)).min(1.25),
                alpha,
            ],
            None => [color[0], color[1], color[2], 1.0],
        }
    }

    /// Vertices carrying an atlas matter (alpha below 1).
    pub fn layered_vertices(&self) -> usize {
        self.colors.iter().filter(|c| c[3] < 1.0).count()
    }

    fn quad(&mut self, c: [Vec3; 4], color: [f32; 3]) {
        let normal = (c[1] - c[0]).cross(c[3] - c[0]).normalize_or_zero();
        let base = self.positions.len() as u32;
        let vertex = self.vertex(color);
        for corner in c {
            self.positions.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.colors.push(vertex);
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
            let vertex = s.vertex(col);
            for v in t {
                s.positions.push(v.to_array());
                s.normals.push(normal.to_array());
                s.colors.push(vertex);
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

pub struct WorksPlugin;

impl Plugin for WorksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            build_works
                .run_if(resource_added::<BattleSession>)
                .run_if(resource_exists::<BuildingAtlas>),
        )
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
        boxes.set_layer("Ashlar", STONE);
        boxes.cuboid(Vec3::ZERO, half, nave_h, 0.0, STONE);
        boxes.set_layer("RoofSlate", SLATE);
        boxes.gable(Vec3::Y * nave_h, half, 4.0, 0.0, SLATE);
        let tower = Vec2::splat((half.y * 0.55).max(2.0));
        boxes.set_layer("Ashlar", STONE);
        boxes.cuboid(
            Vec3::new(-half.x + tower.x, 0.0, 0.0),
            tower,
            nave_h + 7.0,
            0.0,
            STONE,
        );
        boxes.set_layer("RoofSlate", SLATE);
        boxes.gable(
            Vec3::new(-half.x + tower.x, nave_h + 7.0, 0.0),
            tower,
            4.5,
            PI * 0.5,
            SLATE,
        );
    } else {
        let wall_h = if house.rows >= 2 { 6.0 } else { 4.5 } + jitter * 1.2;
        boxes.set_layer("Plaster", PLASTER);
        boxes.cuboid(Vec3::ZERO, half, wall_h, 0.0, PLASTER);
        let roof = if jitter > 0.5 { TILE } else { SLATE };
        boxes.set_layer(
            if jitter > 0.5 {
                "RoofTile"
            } else {
                "RoofSlate"
            },
            roof,
        );
        boxes.gable(Vec3::Y * wall_h, half, wall_h * 0.45, 0.0, roof);
    }
    boxes
}

fn rubble_mesh(length: f32, thickness: f32, seed: f32) -> Boxes {
    let mut boxes = Boxes::default();
    boxes.set_layer("Rubble", STONE);
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

#[allow(clippy::too_many_arguments)]
fn build_works(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut atlas_materials: ResMut<Assets<BuildingAtlasMaterial>>,
    mut atlas: ResMut<BuildingAtlas>,
    mut stats: ResMut<SiegeStats>,
    assets: Res<AssetServer>,
    stone_textures: Option<Res<StoneTextures>>,
    mut stone_materials: ResMut<Assets<SiegeStoneMaterial>>,
) {
    // The player's castle wall texture, in Godot's triplanar mapping, for the walls and towers
    // (one material per damage step); the atlas masonry stays the fallback.
    let stone: Option<StoneSet> = stone_textures.as_deref().filter(|t| t.present).map(|t| {
        StoneSet(
            (0..=DAMAGE_STEPS)
                .map(|level| {
                    stone_materials
                        .add(t.material(1.0 - f32::from(level) / f32::from(DAMAGE_STEPS)))
                })
                .collect(),
        )
    });
    if let Some(set) = &stone {
        commands.insert_resource(set.clone());
        stats.stone_walls = true;
    }
    let sim = session.sim_mut();
    let Some(works) = sim.siege() else {
        return;
    };
    let field = sim.field();
    let ground = |x: f64, z: f64| field.height(x, z) as f32;
    let scope = DespawnOnExit(AppState::Battle);
    let thickness = works.thickness as f32;
    let wall_h = works.wall_height as f32;
    // Atlas materials: masonry by forced layer (walls, towers), the others code their matter in
    // the vertex alpha (crenels, rubble, houses). Box UVs from the world position everywhere.
    let masonry = atlas.layer("Masonry");
    let wall_style = |layer: u32, color: Vec3| AtlasStyle {
        layer: Some(layer),
        box_uv: true,
        color,
        ..AtlasStyle::default()
    };
    let planks = atlas.layer("Planks");
    let door_mat = atlas.material(&mut atlas_materials, &wall_style(planks, Vec3::ONE));
    let rubble_mat = atlas.material(
        &mut atlas_materials,
        &AtlasStyle {
            box_uv: true,
            ..AtlasStyle::default()
        },
    );
    let wall_mat = atlas.material(&mut atlas_materials, &wall_style(masonry, damage_tint(0)));
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
        let intact = piece.intact();
        let mut door = None;
        let mut body = None;
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
                let mut body_entity = p.spawn((
                    Name::new("SiegeWall"),
                    Mesh3d(meshes.add(Cuboid::new(
                        length,
                        body_h,
                        thickness + if gate { 0.6 } else { 0.0 },
                    ))),
                    Transform::from_xyz(0.0, body_y, 0.0),
                ));
                match &stone {
                    Some(set) => body_entity.insert(MeshMaterial3d(set.0[0].clone())),
                    None => body_entity.insert(MeshMaterial3d(wall_mat.clone())),
                };
                body = Some(body_entity.id());
                // Crenels along the outer edge of the wall walk.
                let mut crenels = Boxes::default();
                crenels.set_layer("Masonry", STONE);
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
                let mut crenels_entity = p.spawn((
                    Name::new("SiegeCrenels"),
                    Mesh3d(meshes.add(crenels.into_mesh())),
                    Transform::default(),
                ));
                match &stone {
                    Some(set) => crenels_entity.insert(MeshMaterial3d(set.0[0].clone())),
                    None => crenels_entity.insert(MeshMaterial3d(rubble_mat.clone())),
                };
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
                    body: body.unwrap_or(Entity::PLACEHOLDER),
                    damage_level: 0,
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
        stats.atlas_pieces += 1;
    }
    // Towers.
    let tower_mat = atlas.material(
        &mut atlas_materials,
        &wall_style(masonry, Vec3::splat(0.95)),
    );
    // Slate cones, darker than the house roofs (`battle_siege.gd`: slate 0.55/0.56/0.60, roughness
    // 0.75), on one tower out of two; the others get a crenellated platform.
    let roof_slate = atlas.layer("RoofSlate");
    let roof_mat = atlas.material(
        &mut atlas_materials,
        &wall_style(roof_slate, Vec3::new(0.55, 0.56, 0.60)),
    );
    let merlon_mesh = meshes.add(Cuboid::new(0.9, 1.3, 0.7));
    for tower in &works.towers {
        let base = ground(tower.x, tower.z) - 1.4;
        let top = ground(tower.x, tower.z) + tower.height as f32;
        let radius = tower.radius as f32;
        let mut tower_entity = commands.spawn((
            Name::new("SiegeTower"),
            Mesh3d(meshes.add(Cylinder::new(radius, top - base).mesh().resolution(12))),
            Transform::from_xyz(tower.x as f32, (base + top) * 0.5, tower.z as f32),
            scope.clone(),
        ));
        match &stone {
            Some(set) => tower_entity.insert(MeshMaterial3d(set.0[0].clone())),
            None => tower_entity.insert(MeshMaterial3d(tower_mat.clone())),
        };
        // `_build_tower`: corbel ring, then a proportioned cone (height r x 1.55) above a parapet
        // ring, or merlons around the platform.
        let (tx, tz) = (tower.x as f32, tower.z as f32);
        let corbel = Cylinder::new(radius * 1.14, 1.4).mesh().resolution(14);
        let corbel = commands
            .spawn((
                Name::new("SiegeTowerCorbel"),
                Mesh3d(meshes.add(corbel)),
                Transform::from_xyz(tx, top - 0.2, tz),
                scope.clone(),
            ))
            .id();
        match &stone {
            Some(set) => commands
                .entity(corbel)
                .insert(MeshMaterial3d(set.0[0].clone())),
            None => commands
                .entity(corbel)
                .insert(MeshMaterial3d(tower_mat.clone())),
        };
        let roofed = ((tower.x * 7.0 + tower.z * 13.0).abs() as i64) % 2 == 0;
        if roofed {
            let cone_h = radius * 1.55;
            commands.spawn((
                Name::new("SiegeTowerRoof"),
                Mesh3d(meshes.add(Cone::new(radius * 1.12, cone_h).mesh().resolution(14))),
                MeshMaterial3d(roof_mat.clone()),
                Transform::from_xyz(tx, top + 1.5 + cone_h * 0.5, tz),
                scope.clone(),
            ));
            stats.tower_roofs += 1;
        } else {
            let count = ((std::f32::consts::TAU * radius * 1.1 / 2.0) as usize).max(6);
            for i in 0..count {
                let a = std::f32::consts::TAU * i as f32 / count as f32;
                let mut merlon = commands.spawn((
                    Name::new("SiegeTowerMerlon"),
                    Mesh3d(merlon_mesh.clone()),
                    Transform::from_xyz(
                        tx + a.cos() * radius * 1.08,
                        top + 1.1,
                        tz + a.sin() * radius * 1.08,
                    )
                    .with_rotation(Quat::from_rotation_y(-a)),
                    scope.clone(),
                ));
                match &stone {
                    Some(set) => merlon.insert(MeshMaterial3d(set.0[0].clone())),
                    None => merlon.insert(MeshMaterial3d(tower_mat.clone())),
                };
            }
        }
        stats.towers += 1;
        stats.atlas_towers += 1;
    }
    // Paved streets of an emblematic town.
    if let Some(landmark) = works.landmark.as_ref()
        && let Some((mesh, quads)) = super::streets::street_mesh(&landmark.streets, ground)
    {
        commands.spawn((
            Name::new("SiegeStreets"),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(rubble_mat.clone()),
            Transform::default(),
            NotShadowCaster,
            scope.clone(),
        ));
        stats.street_quads = quads;
    }
    // Houses.
    let house_style = |color: Vec3, soot: f32| AtlasStyle {
        box_uv: true,
        color,
        soot,
        ..AtlasStyle::default()
    };
    let normal = atlas.material(&mut atlas_materials, &house_style(Vec3::ONE, 0.0));
    let burning = atlas.material(
        &mut atlas_materials,
        &house_style(Vec3::new(1.0, 0.55, 0.32), 0.0),
    );
    let ruin = atlas.material(
        &mut atlas_materials,
        &house_style(Vec3::new(0.22, 0.2, 0.19) / 0.5, 0.6),
    );
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
        stats.atlas_house_vertices += boxes.layered_vertices();
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
#[allow(clippy::too_many_arguments)]
fn update_pieces(
    time: Res<Time>,
    frame: Res<SiegeFrame>,
    mut stats: ResMut<SiegeStats>,
    mut atlas: ResMut<BuildingAtlas>,
    mut atlas_materials: ResMut<Assets<BuildingAtlasMaterial>>,
    mut bodies: Query<&mut MeshMaterial3d<BuildingAtlasMaterial>>,
    mut stone_bodies: Query<&mut MeshMaterial3d<SiegeStoneMaterial>>,
    stone: Option<Res<StoneSet>>,
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
            let level = damage_level(share);
            if level != view.damage_level {
                view.damage_level = level;
                if let Some(set) = stone.as_ref() {
                    if let Ok(mut material) = stone_bodies.get_mut(view.body) {
                        material.0 = set.0[usize::from(level).min(set.0.len() - 1)].clone();
                    }
                } else {
                    let style = AtlasStyle {
                        layer: Some(atlas.layer("Masonry")),
                        box_uv: true,
                        color: damage_tint(level),
                        ..AtlasStyle::default()
                    };
                    let handle = atlas.material(&mut atlas_materials, &style);
                    if let Ok(mut material) = bodies.get_mut(view.body) {
                        material.0 = handle;
                    }
                }
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
        &mut MeshMaterial3d<BuildingAtlasMaterial>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_steps_follow_the_hit_points() {
        assert_eq!(damage_level(1.0), 0);
        assert_eq!(damage_level(0.5), 3);
        assert_eq!(damage_level(0.0), DAMAGE_STEPS);
        assert_eq!(damage_level(2.0), 0);
    }

    #[test]
    fn damage_darkens_the_masonry_down_to_the_old_flat_tint() {
        assert_eq!(damage_tint(0), Vec3::ONE);
        let worst = damage_tint(DAMAGE_STEPS);
        // 60 % of the way from STONE to STONE_DARK.
        let expected = 1.0 + (STONE_DARK[0] / STONE[0] - 1.0) * 0.6;
        assert!((worst.x - expected).abs() < 1e-5);
        assert!(worst.x < damage_tint(2).x);
    }

    #[test]
    fn house_volumes_carry_atlas_matter_in_the_vertex_alpha() {
        let house: House = serde_json::from_value(serde_json::json!({
            "x": 0.0, "z": 0.0, "radius": 0.0, "length": 10.0, "depth": 8.0
        }))
        .unwrap();
        let boxes = house_mesh(&house, 0);
        assert!(!boxes.is_empty());
        // Every vertex is on the atlas (plaster walls, tile or slate roof), none is plain.
        assert_eq!(boxes.layered_vertices(), boxes.positions.len());
        let alphas: std::collections::BTreeSet<u32> =
            boxes.colors.iter().map(|c| (c[3] * 16.0) as u32).collect();
        assert!(alphas.contains(&0), "plaster walls");
        assert!(
            alphas.iter().any(|a| (7..=9).contains(a)),
            "tile or slate roof"
        );
    }

    #[test]
    fn rubble_is_rubble_matter() {
        let boxes = rubble_mesh(20.0, 3.0, 1.0);
        assert_eq!(boxes.layered_vertices(), boxes.positions.len());
        assert!(boxes.colors.iter().all(|c| (c[3] * 16.0) as u32 == 1));
    }
}
