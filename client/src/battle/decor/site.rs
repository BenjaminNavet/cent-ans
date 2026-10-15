//! Site elements and bridges (reduced port of game/scripts/battle/battle_decor.gd and
//! battle_bridges.gd): the houses, mills, church, props, hedges, fences, palisades and camps the
//! simulation laid on the field (`Battlefield::decor`, `obstacles`) and the bridges
//! (`Battlefield::bridges`).
//!
//! Godot builds them from the generated building kit (`BuildingKit`): here the kit models
//! (`kit_site.rs`: houses, mills, church, props, churchyard walls and lychgate, horses) are
//! spawned as glTF scenes and the coloured volumes (walls + roof, cones, prisms, merged into two
//! meshes with vertex colours) only draw what the kit has no model for (and the posts and rope of
//! the horse lines, hedges, fences, palisades, bridges). Not ported: regional materials of the
//! kit (snow variant), orchards, building lights.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::VisibilityRange;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use std::collections::HashMap;

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use sim_battle::{
    BattleSeason, Battlefield, DecorProp, DecorPropKind, House, HouseKind, ObstacleKind, SideId,
};

use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::campaign::towns::kit::Kit;

#[derive(Resource, Debug, Clone, Default)]
pub struct SiteStats {
    pub houses: usize,
    pub props: usize,
    pub bridges: usize,
    pub hedges: usize,
    pub camps: usize,
    pub vine_rows: usize,
    pub vineyards: usize,
    /// Houses, props, wall runs, lychgates and horses drawn as BuildingKit models.
    pub kit_houses: usize,
    pub kit_props: usize,
    pub wall_runs: usize,
    pub lychgates: usize,
    pub horses: usize,
    pub kit_models: usize,
}

pub struct SitePlugin;

impl Plugin for SitePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, build_site.run_if(resource_added::<BattleSession>))
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<SiteStats>();
            });
    }
}

/// Merged mesh of coloured volumes.
#[derive(Default)]
pub struct Volumes {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Volumes {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    fn quad(&mut self, corners: [Vec3; 4], color: [f32; 3]) {
        let normal = (corners[1] - corners[0])
            .cross(corners[3] - corners[0])
            .normalize_or(Vec3::Y);
        let base = self.positions.len() as u32;
        for corner in corners {
            self.positions.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.colors.push([color[0], color[1], color[2], 1.0]);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn tri(&mut self, corners: [Vec3; 3], color: [f32; 3]) {
        let normal = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .normalize_or(Vec3::Y);
        let base = self.positions.len() as u32;
        for corner in corners {
            self.positions.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.colors.push([color[0], color[1], color[2], 1.0]);
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// Box centred on `centre` (its bottom at `centre.y - half.y`), `half` = half extents along
    /// the local axes, turned by `yaw` about +y (the sim convention: local +x towards
    /// `(cos yaw, sin yaw)`).
    pub fn cuboid(&mut self, centre: Vec3, half: Vec3, yaw: f32, color: [f32; 3]) {
        let rot = Quat::from_rotation_y(-yaw);
        let p = |sx: f32, sy: f32, sz: f32| {
            centre + rot * Vec3::new(sx * half.x, sy * half.y, sz * half.z)
        };
        let (a, b, c, d) = (
            p(-1., -1., -1.),
            p(1., -1., -1.),
            p(1., -1., 1.),
            p(-1., -1., 1.),
        );
        let (e, f, g, h) = (
            p(-1., 1., -1.),
            p(1., 1., -1.),
            p(1., 1., 1.),
            p(-1., 1., 1.),
        );
        let shade = |k: f32| [color[0] * k, color[1] * k, color[2] * k];
        self.quad([e, h, g, f], color); // top
        self.quad([a, e, f, b], shade(0.9)); // -z
        self.quad([b, f, g, c], shade(0.85)); // +x
        self.quad([c, g, h, d], shade(0.9)); // +z
        self.quad([d, h, e, a], shade(0.85)); // -x
    }

    /// Gabled roof: ridge along local x, eaves at `base.y`, ridge `rise` higher.
    pub fn gable(
        &mut self,
        base: Vec3,
        half_length: f32,
        half_width: f32,
        rise: f32,
        yaw: f32,
        color: [f32; 3],
    ) {
        let rot = Quat::from_rotation_y(-yaw);
        let p = |x: f32, y: f32, z: f32| base + rot * Vec3::new(x, y, z);
        let (a, b) = (
            p(-half_length, 0.0, -half_width),
            p(half_length, 0.0, -half_width),
        );
        let (c, d) = (
            p(half_length, 0.0, half_width),
            p(-half_length, 0.0, half_width),
        );
        let (r0, r1) = (p(-half_length, rise, 0.0), p(half_length, rise, 0.0));
        let shade = |k: f32| [color[0] * k, color[1] * k, color[2] * k];
        self.quad([a, b, r1, r0], color);
        self.quad([c, d, r0, r1], shade(0.92));
        self.tri([d, a, r0], shade(0.8));
        self.tri([b, c, r1], shade(0.8));
    }

    /// Cone (or cylinder when `top_radius == bottom_radius`) with `sides` facets.
    pub fn cone(
        &mut self,
        bottom: Vec3,
        bottom_radius: f32,
        top_radius: f32,
        height: f32,
        sides: u32,
        color: [f32; 3],
    ) {
        for i in 0..sides {
            let (a0, a1) = (
                i as f32 / sides as f32 * TAU,
                (i + 1) as f32 / sides as f32 * TAU,
            );
            let at = |a: f32, r: f32, y: f32| bottom + Vec3::new(a.cos() * r, y, a.sin() * r);
            let shade = 0.85 + 0.15 * (a0.cos() * 0.5 + 0.5);
            let c = [color[0] * shade, color[1] * shade, color[2] * shade];
            if top_radius <= 0.001 {
                self.tri(
                    [
                        at(a0, bottom_radius, 0.0),
                        at(a1, bottom_radius, 0.0),
                        at(0.0, 0.0, height),
                    ],
                    c,
                );
            } else {
                self.quad(
                    [
                        at(a0, bottom_radius, 0.0),
                        at(a1, bottom_radius, 0.0),
                        at(a1, top_radius, height),
                        at(a0, top_radius, height),
                    ],
                    c,
                );
            }
        }
    }

    fn into_mesh(self) -> Mesh {
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

const WALL_COB: [f32; 3] = [0.74, 0.66, 0.5];
const WALL_TIMBER: [f32; 3] = [0.78, 0.72, 0.6];
const WALL_STONE: [f32; 3] = [0.62, 0.6, 0.56];
const WALL_WOOD: [f32; 3] = [0.45, 0.33, 0.22];
const THATCH: [f32; 3] = [0.62, 0.52, 0.28];
const TILES: [f32; 3] = [0.55, 0.28, 0.2];
const SLATE: [f32; 3] = [0.34, 0.36, 0.4];

/// One house at ground height `y`.
pub fn add_house(volumes: &mut Volumes, house: &House, y: f32) {
    let yaw = house.yaw as f32;
    let (hl, hw) = (house.length as f32 * 0.5, house.width as f32 * 0.5);
    let centre = Vec3::new(house.x as f32, y, house.z as f32);
    let walls = |v: &mut Volumes, h: f32, color| {
        v.cuboid(
            centre + Vec3::Y * (h * 0.5 - 0.3),
            Vec3::new(hl, h * 0.5 + 0.3, hw),
            yaw,
            color,
        )
    };
    match house.kind {
        HouseKind::Cottage => {
            walls(volumes, 2.5, WALL_COB);
            volumes.gable(centre + Vec3::Y * 2.5, hl + 0.4, hw + 0.4, 1.9, yaw, THATCH);
        }
        HouseKind::Timbered => {
            walls(volumes, 3.6, WALL_TIMBER);
            volumes.gable(centre + Vec3::Y * 3.6, hl + 0.4, hw + 0.4, 2.3, yaw, TILES);
        }
        HouseKind::Barn => {
            walls(volumes, 4.0, WALL_WOOD);
            volumes.gable(centre + Vec3::Y * 4.0, hl + 0.5, hw + 0.5, 3.0, yaw, THATCH);
        }
        HouseKind::Church => {
            walls(volumes, 6.0, WALL_STONE);
            volumes.gable(centre + Vec3::Y * 6.0, hl + 0.3, hw + 0.3, 3.6, yaw, SLATE);
            // Bell tower at the west end.
            let rot = Quat::from_rotation_y(-yaw);
            let tower = centre + rot * Vec3::new(-hl + 2.0, 0.0, 0.0);
            volumes.cuboid(
                tower + Vec3::Y * 6.0,
                Vec3::new(2.0, 6.3, 2.0),
                yaw,
                WALL_STONE,
            );
            volumes.cone(tower + Vec3::Y * 12.0, 2.6, 0.0, 6.0, 4, SLATE);
        }
        HouseKind::Stone => {
            walls(volumes, 4.2, WALL_STONE);
            volumes.gable(centre + Vec3::Y * 4.2, hl + 0.4, hw + 0.4, 2.4, yaw, TILES);
        }
        HouseKind::Windmill => {
            let r = (house.width.min(house.length) as f32 * 0.5).max(2.0);
            volumes.cone(centre - Vec3::Y * 0.3, r, r * 0.65, 8.0, 8, WALL_WOOD);
            volumes.cone(centre + Vec3::Y * 7.7, r * 0.8, 0.0, 2.6, 8, THATCH);
            // Four sails.
            let rot = Quat::from_rotation_y(-yaw);
            for k in 0..4 {
                let a = k as f32 * TAU / 4.0 + 0.4;
                let arm = rot * Vec3::new(1.0, 0.0, 0.0);
                let hub = centre + Vec3::Y * 6.5 + arm * (r * 0.7);
                let along = Quat::from_axis_angle(arm, a) * Vec3::Y;
                volumes.cuboid(
                    hub + along * 5.0,
                    Vec3::new(0.15, 0.1, 0.1) + along.abs() * 5.0 * Vec3::new(0.0, 1.0, 1.0),
                    yaw,
                    [0.8, 0.75, 0.62],
                );
            }
        }
        HouseKind::Watermill => {
            walls(volumes, 4.5, WALL_TIMBER);
            volumes.gable(centre + Vec3::Y * 4.5, hl + 0.4, hw + 0.4, 2.6, yaw, TILES);
            let rot = Quat::from_rotation_y(-yaw);
            let wheel = centre + rot * Vec3::new(0.0, 1.8, hw + 0.5);
            volumes.cuboid(
                wheel,
                Vec3::new(0.35, 1.8, 0.1) * Vec3::new(1.0, 1.0, 1.0) + Vec3::new(0.0, 0.0, 0.3),
                yaw,
                WALL_WOOD,
            );
        }
        #[allow(unreachable_patterns)]
        _ => {
            walls(volumes, 3.0, WALL_COB);
            volumes.gable(centre + Vec3::Y * 3.0, hl + 0.4, hw + 0.4, 2.0, yaw, THATCH);
        }
    }
}

/// A prop at ground height `y`; `side_color` tints the tents of a camp.
pub fn add_prop(volumes: &mut Volumes, prop: &DecorProp, y: f32, side_color: [f32; 3]) {
    let yaw = prop.yaw as f32;
    let (hl, hd) = (prop.length as f32 * 0.5, prop.depth as f32 * 0.5);
    let at = Vec3::new(prop.x as f32, y, prop.z as f32);
    match prop.kind {
        DecorPropKind::Haystack => volumes.cone(at, hl.max(1.2), 0.2, 2.8, 8, [0.78, 0.66, 0.3]),
        DecorPropKind::Cart | DecorPropKind::Wagon => {
            volumes.cuboid(
                at + Vec3::Y * 1.0,
                Vec3::new(hl, 0.45, hd * 0.8),
                yaw,
                WALL_WOOD,
            );
            volumes.gable(
                at + Vec3::Y * 1.5,
                hl,
                hd * 0.85,
                0.9,
                yaw,
                [0.82, 0.78, 0.68],
            );
            let rot = Quat::from_rotation_y(-yaw);
            for sign in [-1.0f32, 1.0] {
                volumes.cuboid(
                    at + rot * Vec3::new(0.0, 0.55, sign * hd * 0.95) + Vec3::Y * 0.0,
                    Vec3::new(0.55, 0.55, 0.06),
                    yaw,
                    [0.25, 0.18, 0.12],
                );
            }
        }
        DecorPropKind::Tent => {
            volumes.gable(at, hl, hd, hl.min(hd) * 1.1 + 0.8, yaw, side_color);
            volumes.cuboid(
                at + Vec3::Y * 0.5,
                Vec3::new(hl * 0.98, 0.5, hd * 0.98),
                yaw,
                [
                    side_color[0] * 0.85,
                    side_color[1] * 0.85,
                    side_color[2] * 0.85,
                ],
            );
        }
        DecorPropKind::Pavilion => {
            volumes.cuboid(at + Vec3::Y * 1.3, Vec3::new(hl, 1.3, hd), yaw, side_color);
            volumes.cone(
                at + Vec3::Y * 2.6,
                hl.max(hd) * 1.15,
                0.0,
                3.2,
                4,
                [
                    side_color[0] * 1.1,
                    side_color[1] * 1.1,
                    side_color[2] * 1.1,
                ],
            );
            volumes.cuboid(
                at + Vec3::Y * 7.0,
                Vec3::new(0.06, 1.8, 0.06),
                yaw,
                [0.3, 0.22, 0.15],
            );
        }
        DecorPropKind::Campfire => {
            volumes.cone(at, 0.9, 0.7, 0.25, 8, [0.2, 0.18, 0.16]);
            volumes.cone(at + Vec3::Y * 0.2, 0.45, 0.0, 0.9, 6, [1.0, 0.55, 0.15]);
        }
        DecorPropKind::HorseLine => {
            let rot = Quat::from_rotation_y(-yaw);
            for sign in [-1.0f32, 1.0] {
                let post = at + rot * Vec3::new(sign * hl, 0.0, 0.0);
                volumes.cuboid(
                    post + Vec3::Y * 0.8,
                    Vec3::new(0.08, 0.8, 0.08),
                    yaw,
                    WALL_WOOD,
                );
            }
            volumes.cuboid(
                at + Vec3::Y * 1.4,
                Vec3::new(hl, 0.02, 0.02),
                yaw,
                [0.55, 0.45, 0.3],
            );
        }
        DecorPropKind::Graves => {
            let rot = Quat::from_rotation_y(-yaw);
            let count = prop.count.max(3) as i32;
            for k in 0..count {
                let spread =
                    (k as f32 - (count - 1) as f32 * 0.5) * (prop.length as f32 / count as f32);
                let row = if k % 2 == 0 { -hd * 0.4 } else { hd * 0.4 };
                volumes.cuboid(
                    at + rot * Vec3::new(spread, 0.0, row) + Vec3::Y * 0.4,
                    Vec3::new(0.3, 0.4, 0.08),
                    yaw,
                    WALL_STONE,
                );
            }
        }
        DecorPropKind::Well => {
            volumes.cone(at, 0.9, 0.9, 0.9, 8, WALL_STONE);
            volumes.gable(at + Vec3::Y * 2.0, 0.8, 0.8, 0.8, yaw, TILES);
            volumes.cuboid(
                at + Vec3::Y * 1.0,
                Vec3::new(0.06, 1.0, 0.06),
                yaw,
                WALL_WOOD,
            );
        }
        DecorPropKind::Woodpile => volumes.cuboid(
            at + Vec3::Y * 0.6,
            Vec3::new(hl, 0.6, hd),
            yaw,
            [0.5, 0.36, 0.2],
        ),
        #[allow(unreachable_patterns)]
        _ => volumes.cuboid(at + Vec3::Y * 0.5, Vec3::new(hl, 0.5, hd), yaw, WALL_WOOD),
    }
}

/// Hedge, fence or palisade segment from `a` to `b`.
pub fn add_obstacle(
    volumes: &mut Volumes,
    kind: ObstacleKind,
    a: (f64, f64),
    b: (f64, f64),
    ya: f32,
    yb: f32,
    winter: bool,
) {
    let (dx, dz) = ((b.0 - a.0) as f32, (b.1 - a.1) as f32);
    let length = dx.hypot(dz);
    if length < 0.1 || matches!(kind, ObstacleKind::Ditch) {
        return;
    }
    let yaw = dz.atan2(dx);
    let centre = Vec3::new(
        ((a.0 + b.0) * 0.5) as f32,
        (ya + yb) * 0.5,
        ((a.1 + b.1) * 0.5) as f32,
    );
    let (half_height, half_width, color) = match kind {
        ObstacleKind::Hedge => (
            0.75,
            0.7,
            if winter {
                [0.34, 0.3, 0.2]
            } else {
                [0.2, 0.34, 0.12]
            },
        ),
        ObstacleKind::Fence => (0.5, 0.06, [0.5, 0.4, 0.26]),
        _ => (1.1, 0.12, [0.45, 0.33, 0.2]),
    };
    volumes.cuboid(
        centre + Vec3::Y * (half_height - 0.1),
        Vec3::new(length * 0.5, half_height, half_width),
        yaw,
        color,
    );
}

/// A bridge: deck, parapets (stone) or piles (wood), abutment piers.
pub fn add_bridge(volumes: &mut Volumes, bridge: &sim_battle::Bridge) {
    let yaw = bridge.dir.1.atan2(bridge.dir.0) as f32;
    let centre = Vec3::new(bridge.x as f32, bridge.deck as f32, bridge.z as f32);
    let (hl, hw) = (bridge.length as f32 * 0.5, bridge.width as f32 * 0.5);
    let rot = Quat::from_rotation_y(-yaw);
    if bridge.stone {
        volumes.cuboid(
            centre - Vec3::Y * 0.4,
            Vec3::new(hl, 0.4, hw),
            yaw,
            WALL_STONE,
        );
        for sign in [-1.0f32, 1.0] {
            volumes.cuboid(
                centre + rot * Vec3::new(0.0, 0.45, sign * (hw - 0.2)),
                Vec3::new(hl, 0.45, 0.2),
                yaw,
                [0.58, 0.56, 0.52],
            );
        }
        // Piers under the arches.
        let arches = bridge.arches.max(1) as f32;
        for k in 0..=(bridge.arches as i32) {
            let along = -bridge.span as f32 * 0.5 + k as f32 * bridge.span as f32 / arches;
            volumes.cuboid(
                centre + rot * Vec3::new(along, -2.5, 0.0),
                Vec3::new(0.7, 2.1, hw * 0.95),
                yaw,
                [0.5, 0.48, 0.45],
            );
        }
    } else {
        volumes.cuboid(
            centre - Vec3::Y * 0.15,
            Vec3::new(hl, 0.15, hw),
            yaw,
            WALL_WOOD,
        );
        for sign in [-1.0f32, 1.0] {
            volumes.cuboid(
                centre + rot * Vec3::new(0.0, 0.4, sign * (hw - 0.05)),
                Vec3::new(hl, 0.04, 0.05),
                yaw,
                [0.4, 0.3, 0.2],
            );
        }
        let bays = (bridge.span / 4.0).ceil().max(1.0) as i32;
        for k in 0..=bays {
            let along = -bridge.span as f32 * 0.5 + k as f32 * bridge.span as f32 / bays as f32;
            for sign in [-1.0f32, 1.0] {
                volumes.cuboid(
                    centre + rot * Vec3::new(along, -1.6, sign * (hw - 0.2)),
                    Vec3::new(0.18, 1.5, 0.18),
                    yaw,
                    [0.36, 0.27, 0.18],
                );
            }
        }
    }
}

/// Rows of a vineyard (port of `_build_vineyards` of battle_decor.gd, far version): a low hedge
/// of `segment_m` every `row_step_m` across the plot, green leaves or brown winter stocks.
/// Returns the number of segments added.
pub fn add_vineyard(
    volumes: &mut Volumes,
    area: &sim_battle::Area,
    look: &super::look::DecorLook,
    leafy: bool,
    ground: impl Fn(f64, f64) -> f32,
) -> usize {
    let cfg = &look.vineyard;
    let (length, width) = (area.length as f32, area.width as f32);
    let rows = (((width - 3.0) / cfg.row_step_m) as usize).max(1);
    let segments = (((length - 4.0) / cfg.segment_m).ceil() as usize).max(1);
    let segment = (length - 4.0) / segments as f32;
    let (height, thick, color) = if leafy {
        (cfg.height_m, cfg.thickness_m, cfg.leaf_color)
    } else {
        (cfg.bare_height_m, 0.12, cfg.bare_color)
    };
    let yaw = area.yaw as f32;
    let (axis, across) = (
        Vec2::new(yaw.cos(), yaw.sin()),
        Vec2::new(-yaw.sin(), yaw.cos()),
    );
    let centre = Vec2::new(area.x as f32, area.z as f32);
    let mut count = 0;
    for r in 0..rows {
        let v = -width * 0.5 + 1.5 + cfg.row_step_m * (r as f32 + 0.5);
        for sgm in 0..segments {
            let u = -length * 0.5 + 2.0 + segment * (sgm as f32 + 0.5);
            let q = centre + axis * u + across * v;
            let y = ground(f64::from(q.x), f64::from(q.y));
            volumes.cuboid(
                Vec3::new(q.x, y + height * 0.5 + 0.2, q.y),
                Vec3::new(segment * 0.5, height * 0.5, thick * 0.5),
                yaw,
                color,
            );
            count += 1;
        }
    }
    count
}

fn build_site(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    look: Res<super::look::DecorLook>,
    assets: Res<AssetServer>,
) {
    let colors = session.colors;
    let sim = session.sim_mut();
    let field: &Battlefield = sim.field();
    let ground = |x: f64, z: f64| field.height(x, z) as f32;
    let mut structures = Volumes::default();
    let mut stats = SiteStats::default();
    // BuildingKit models take over the kinds the kit has; the volumes keep the rest.
    let kit = crate::paths::find_root()
        .map(|root| Kit::load(&crate::paths::asset_root(&root)))
        .unwrap_or_default();
    let kit_plan = super::kit_site::plan(field, &kit, &ground);
    let mut scenes: HashMap<String, Handle<WorldAsset>> = HashMap::new();
    for piece in &kit_plan.pieces {
        let handle = scenes
            .entry(piece.model.clone())
            .or_insert_with(|| {
                assets.load(GltfAssetLabel::Scene(0).from_asset(Kit::asset_path(&piece.model)))
            })
            .clone();
        commands.spawn((
            Name::new(format!("Kit_{}", piece.model)),
            WorldAssetRoot(handle),
            piece.transform,
            Visibility::default(),
            DespawnOnExit(AppState::Battle),
        ));
    }
    stats.kit_houses = kit_plan.houses.len();
    stats.kit_props = kit_plan.props.len();
    stats.wall_runs = kit_plan.wall_runs;
    stats.lychgates = kit_plan.lychgates;
    stats.horses = kit_plan.horses;
    stats.kit_models = scenes.len();
    for (index, house) in field.decor.buildings.iter().enumerate() {
        if !kit_plan.houses.contains(&index) {
            add_house(&mut structures, house, ground(house.x, house.z));
        }
        stats.houses += 1;
    }
    let mut prop_index = 0usize;
    for prop in &field.decor.props {
        if !kit_plan.props.contains(&prop_index) {
            add_prop(
                &mut structures,
                prop,
                ground(prop.x, prop.z),
                [0.8, 0.75, 0.62],
            );
        }
        prop_index += 1;
        stats.props += 1;
    }
    for camp in &field.decor.camps {
        let side_color = match camp.side {
            SideId::Attacker => colors[0],
            SideId::Defender => colors[1],
        };
        let tint = [
            0.45 + side_color[0] * 0.55,
            0.45 + side_color[1] * 0.55,
            0.45 + side_color[2] * 0.55,
        ];
        for prop in camp.items.iter().chain(camp.convoy.iter()) {
            if !kit_plan.props.contains(&prop_index) {
                add_prop(&mut structures, prop, ground(prop.x, prop.z), tint);
            }
            prop_index += 1;
            stats.props += 1;
        }
        stats.camps += 1;
    }
    let winter = matches!(field.season, BattleSeason::Winter);
    for obstacle in &field.obstacles {
        add_obstacle(
            &mut structures,
            obstacle.kind,
            obstacle.a,
            obstacle.b,
            ground(obstacle.a.0, obstacle.a.1),
            ground(obstacle.b.0, obstacle.b.1),
            winter,
        );
        stats.hedges += 1;
    }
    for bridge in &field.bridges {
        add_bridge(&mut structures, bridge);
        stats.bridges += 1;
    }
    let leafy = field.decor.vines_leafy;
    let range = look.vineyard.range_m;
    for area in field
        .decor
        .areas
        .iter()
        .filter(|a| a.kind == sim_battle::AreaKind::Vineyard)
    {
        let mut rows = Volumes::default();
        stats.vine_rows += add_vineyard(&mut rows, area, &look, leafy, ground);
        stats.vineyards += 1;
        commands.spawn((
            Name::new("Vineyard"),
            Mesh3d(meshes.add(rows.into_mesh())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.95,
                reflectance: 0.04,
                ..default()
            })),
            Transform::default(),
            VisibilityRange {
                start_margin: 0.0..0.0,
                end_margin: range..(range + 120.0),
                use_aabb: true,
            },
            NotShadowCaster,
            DespawnOnExit(AppState::Battle),
        ));
    }
    if !structures.is_empty() {
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.95,
            reflectance: 0.04,
            cull_mode: None,
            double_sided: true,
            ..default()
        });
        commands.spawn((
            Name::new("BattleSite"),
            Mesh3d(meshes.add(structures.into_mesh())),
            MeshMaterial3d(material),
            Transform::default(),
            DespawnOnExit(AppState::Battle),
        ));
    }
    commands.insert_resource(stats);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cuboid_has_five_faces_and_a_house_adds_geometry() {
        let mut volumes = Volumes::default();
        volumes.cuboid(Vec3::ZERO, Vec3::ONE, 0.0, [1.0; 3]);
        assert_eq!(volumes.vertex_count(), 20);
        let house = House {
            x: 10.0,
            z: 10.0,
            length: 8.0,
            width: 5.0,
            yaw: 0.5,
            kind: HouseKind::Church,
        };
        let before = volumes.vertex_count();
        add_house(&mut volumes, &house, 3.0);
        assert!(volumes.vertex_count() > before);
        assert!(
            volumes
                .positions
                .iter()
                .all(|p| p.iter().all(|c| c.is_finite()))
        );
    }

    #[test]
    fn ditches_are_not_drawn() {
        let mut volumes = Volumes::default();
        add_obstacle(
            &mut volumes,
            ObstacleKind::Ditch,
            (0.0, 0.0),
            (10.0, 0.0),
            0.0,
            0.0,
            false,
        );
        assert!(volumes.is_empty());
        add_obstacle(
            &mut volumes,
            ObstacleKind::Hedge,
            (0.0, 0.0),
            (10.0, 0.0),
            0.0,
            0.0,
            false,
        );
        assert!(!volumes.is_empty());
    }
}
