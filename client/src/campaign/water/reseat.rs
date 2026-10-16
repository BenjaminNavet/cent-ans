//! Re-seats the draped ribbons (rivers, roads) on the finest relief pages near the camera.
//!
//! The ribbons store the ground altitude in metres in `position.y` (the shaders drape it on the
//! exaggerated relief). They are built when the terrain sampler first becomes ready, i.e. on the
//! world heightmap, before the fine relief pages around the camera are resident. The close terrain
//! then shows those pages, which differ from the heightmap by tens of metres (Paris: -51..+54 m),
//! and the level a terrain node shows depends on its distance (coarser levels sit tens of metres
//! higher in narrow valleys), so the rivers ended up buried under the ground at close range
//! (showfix, 16/10). Godot carves the beds into the pages (`FineBedCarver`); here every ribbon
//! vertex near the camera is re-seated on the highest resident level whenever the pages change
//! (the terrain also carves the bed near the camera, `river_carve` in `terrain.wesl`).

use bevy::prelude::*;

use super::WaterPart;
use super::height::GroundHeight;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::surface::Surface;

/// Minimum time between two re-seats (pages arrive in bursts while panning).
const MIN_INTERVAL_S: f32 = 0.5;
/// Re-seat radius around the focus: `distance * RADIUS_PER_DISTANCE + RADIUS_BASE` map px.
const RADIUS_PER_DISTANCE: f32 = 8.0;
const RADIUS_BASE: f32 = 60.0;
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
    ground: Res<GroundHeight>,
    parts: Query<(&WaterPart, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut stats: ResMut<ReseatStats>,
    time: Res<Time>,
    mut state: Local<(u64, f32, Vec2, f32)>,
) {
    let (Some(surface), Some(rig)) = (surface, rig) else {
        return;
    };
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    let radius = rig.distance * RADIUS_PER_DISTANCE + RADIUS_BASE;
    let (seen_version, last_run, last_focus, last_radius) = *state;
    let now = time.elapsed_secs();
    // Re-run when the resident pages changed, or when the view moved far enough to bring other
    // already resident pages inside the radius.
    let moved = focus.distance(last_focus) > last_radius * 0.25
        || (radius - last_radius).abs() > last_radius * 0.25;
    if (surface.version == seen_version && !moved) || now - last_run < MIN_INTERVAL_S {
        return;
    }
    let started = std::time::Instant::now();
    *state = (surface.version, now, focus, radius);
    let radius2 = radius * radius;
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
            let y = ground.0.seat_height_m(p[0], p[2]).max(0.0);
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
            stats.meshes_updated += 1;
            stats.vertices_moved += changes.len() as u64;
        }
    }
    stats.runs += 1;
    stats.last_max_shift_m = max_shift;
    stats.last_ms = started.elapsed().as_secs_f32() * 1000.0;
}
