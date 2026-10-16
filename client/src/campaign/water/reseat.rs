//! Re-seats the draped ribbons (rivers, roads) on the finest relief pages near the camera.
//!
//! The ribbons store the ground altitude in metres in `position.y` (the shaders drape it on the
//! exaggerated relief). They are built on the world heightmap, before the fine relief pages around
//! the camera are resident; the close terrain then shows those pages (Paris: -51..+54 m from the
//! heightmap) and its triangles bridge narrow valleys between vertices. Every ribbon vertex near
//! the camera is re-seated on the highest resident level and on the displayed patch lattice (own
//! and parent level, both diagonals) whenever the pages or the patches change. (The rivers buried
//! at close range, showfix 16/10, were mostly the relief uniform never reaching the ribbon
//! materials, see `drape::sync_drape`; this pass removes the remaining few metres.)

use bevy::prelude::*;

use super::WaterPart;
use super::height::GroundHeight;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::lod::TerrainLod;
use crate::campaign::terrain::mesh::NodeSpec;
use crate::campaign::terrain::surface::Surface;

/// Minimum time between two re-seats (pages arrive in bursts while panning).
const MIN_INTERVAL_S: f32 = 0.5;
/// Re-seat radius around the focus: `distance * RADIUS_PER_DISTANCE + RADIUS_BASE` map px.
const RADIUS_PER_DISTANCE: f32 = 30.0;
const RADIUS_BASE: f32 = 200.0;
/// Cell of the patch lookup grid (map px).
const LOOKUP_CELL: f32 = 32.0;
/// Altitude change (m) below which a vertex is left alone.
const EPSILON_M: f32 = 0.25;

/// Counters for the dump.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct ReseatStats {
    pub runs: u32,
    pub meshes_updated: u32,
    pub vertices_moved: u64,
    pub last_max_shift_m: f32,
    pub last_ms: f32,
    /// Displayed patches near the focus whose page is no longer resident on the CPU.
    pub orphan_patches: u32,
    pub patches: u32,
}

/// Whether a part's `position.y` is a draped ground altitude (metres) worth re-seating.
pub fn is_draped(part: WaterPart) -> bool {
    matches!(
        part,
        WaterPart::MajorRivers
            | WaterPart::MinorRivers
            | WaterPart::MainRoads
            | WaterPart::MinorRoads
            | WaterPart::RoadRibbons
    )
}

#[allow(clippy::too_many_arguments)]
pub fn reseat_ribbons(
    surface: Option<Res<Surface>>,
    rig: Option<Res<CampaignRig>>,
    lod: Option<Res<TerrainLod>>,
    ground: Res<GroundHeight>,
    parts: Query<(&WaterPart, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut stats: ResMut<ReseatStats>,
    time: Res<Time>,
    mut state: Local<(u64, f32, Vec2, f32, u64)>,
) {
    let (Some(surface), Some(rig)) = (surface, rig) else {
        return;
    };
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    let radius = rig.distance * RADIUS_PER_DISTANCE + RADIUS_BASE;
    let (seen_version, last_run, last_focus, last_radius, seen_built) = *state;
    let built = lod.as_ref().map_or(0, |l| l.meshes_built);
    let now = time.elapsed_secs();
    // Re-run when the resident pages changed, or when the view moved far enough to bring other
    // already resident pages inside the radius.
    let moved = focus.distance(last_focus) > last_radius * 0.25
        || (radius - last_radius).abs() > last_radius * 0.25;
    if (surface.version == seen_version && built == seen_built && !moved)
        || now - last_run < MIN_INTERVAL_S
    {
        return;
    }
    let started = std::time::Instant::now();
    *state = (surface.version, now, focus, radius, built);
    let radius2 = radius * radius;
    // Displayed patches around the focus: the ribbon must sit on the mesh as drawn (its triangles
    // bridge narrow valleys between vertices), not only on the sampled altitude.
    let patches: Vec<NodeSpec> = lod.as_ref().map_or_else(Vec::new, |l| {
        l.built_specs()
            .filter(|s| {
                let size = s.spacing * s.quads as f32;
                let cx = focus.x.clamp(s.origin.0, s.origin.0 + size);
                let cz = focus.y.clamp(s.origin.1, s.origin.1 + size);
                Vec2::new(cx, cz).distance_squared(focus) <= radius2
            })
            .collect()
    });
    let lookup = PatchLookup::new(&patches, focus, radius);
    stats.patches = patches.len() as u32;
    stats.orphan_patches = patches
        .iter()
        .filter(|s| s.fine >= 0 && surface.page_height_m(s.fine, s.origin.0 + 0.5, s.origin.1 + 0.5).is_none())
        .count() as u32;
    let mut max_shift = 0.0_f32;
    for (part, mesh3d) in &parts {
        if !is_draped(*part) {
            continue;
        }
        let Some(mesh) = meshes.get(&mesh3d.0) else {
            continue;
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        let mut changes: Vec<(usize, f32)> = Vec::new();
        for (i, p) in positions.iter().enumerate() {
            let dx = p[0] - focus.x;
            let dz = p[2] - focus.y;
            if dx * dx + dz * dz > radius2 {
                continue;
            }
            let mut y = ground.0.seat_height_m(p[0], p[2]);
            if let Some(spec) = lookup.find(&patches, p[0], p[2]) {
                y = y.max(mesh_height_m(&surface, spec, p[0], p[2]));
            }
            let y = y.max(0.0);
            let shift = (y - p[1]).abs();
            if shift > EPSILON_M {
                max_shift = max_shift.max(shift);
                changes.push((i, y));
            }
        }
        if changes.is_empty() {
            continue;
        }
        let Some(mut mesh) = meshes.get_mut(&mesh3d.0) else {
            continue;
        };
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for (i, y) in &changes {
                positions[*i][1] = *y;
            }
        }
        stats.meshes_updated += 1;
        stats.vertices_moved += changes.len() as u64;
    }
    stats.runs += 1;
    stats.last_max_shift_m = max_shift;
    stats.last_ms = started.elapsed().as_secs_f32() * 1000.0;
}

/// Uniform grid over the re-seat disc listing the patches overlapping each cell.
struct PatchLookup {
    origin: Vec2,
    side: usize,
    cells: Vec<Vec<u32>>,
}

impl PatchLookup {
    fn new(patches: &[NodeSpec], focus: Vec2, radius: f32) -> Self {
        let origin = focus - Vec2::splat(radius);
        let side = ((2.0 * radius / LOOKUP_CELL).ceil() as usize).max(1);
        let mut cells = vec![Vec::new(); side * side];
        let index = |v: f32, o: f32| (((v - o) / LOOKUP_CELL).floor().max(0.0) as usize).min(side - 1);
        for (n, spec) in patches.iter().enumerate() {
            let size = spec.spacing * spec.quads as f32;
            let (i0, i1) = (index(spec.origin.0, origin.x), index(spec.origin.0 + size, origin.x));
            let (j0, j1) = (index(spec.origin.1, origin.y), index(spec.origin.1 + size, origin.y));
            for j in j0..=j1 {
                for i in i0..=i1 {
                    cells[j * side + i].push(n as u32);
                }
            }
        }
        PatchLookup { origin, side, cells }
    }

    fn find<'a>(&self, patches: &'a [NodeSpec], x: f32, z: f32) -> Option<&'a NodeSpec> {
        let i = ((x - self.origin.x) / LOOKUP_CELL).floor();
        let j = ((z - self.origin.y) / LOOKUP_CELL).floor();
        if i < 0.0 || j < 0.0 || i as usize >= self.side || j as usize >= self.side {
            return None;
        }
        self.cells[j as usize * self.side + i as usize]
            .iter()
            .map(|&n| &patches[n as usize])
            .find(|s| contains(s, x, z))
    }
}

fn contains(spec: &NodeSpec, x: f32, z: f32) -> bool {
    let size = spec.spacing * spec.quads as f32;
    x >= spec.origin.0 && x <= spec.origin.0 + size && z >= spec.origin.1 && z <= spec.origin.1 + size
}

/// Highest altitude (m) the patch can show at (x, z): its own lattice and the parent one it
/// geomorphs towards, each with both quad diagonals.
fn mesh_height_m(surface: &Surface, spec: &NodeSpec, x: f32, z: f32) -> f32 {
    let lattice = |step: f32| {
        let fx = (x - spec.origin.0) / step;
        let fz = (z - spec.origin.1) / step;
        let (i, j) = (fx.floor(), fz.floor());
        let (tx, tz) = (fx - i, fz - j);
        let (x0, z0) = (spec.origin.0 + i * step, spec.origin.1 + j * step);
        let h = |a: f32, b: f32| surface.node_height_m(spec.fine, x0 + a * step, z0 + b * step);
        let (h00, h10, h01, h11) = (h(0.0, 0.0), h(1.0, 0.0), h(0.0, 1.0), h(1.0, 1.0));
        let diag_a = if tx > tz {
            h00 + tx * (h10 - h00) + tz * (h11 - h10)
        } else {
            h00 + tz * (h01 - h00) + tx * (h11 - h01)
        };
        let diag_b = if tx + tz < 1.0 {
            h00 + tx * (h10 - h00) + tz * (h01 - h00)
        } else {
            h11 + (1.0 - tx) * (h01 - h11) + (1.0 - tz) * (h10 - h11)
        };
        diag_a.max(diag_b)
    };
    lattice(spec.spacing).max(lattice(spec.spacing * 2.0))
}
