//! Faction borders. Ports game/scripts/map/faction_borders.gd (ADR 0074, DZ, EN, TB2).
//!
//! Godot paints the borders in the terrain fragment shader; here they are their own ribbon mesh
//! (flat, or draped on a [`SurfaceHeight`] when the terrain lot installs one). Geometry comes from
//! the province rings of `provinces.geojson`: each ring edge is probed against the province raster
//! to find the province across it ([`extract_edges`], once). Which edges are realm borders (owner
//! differs) or province lines (same owner), their colours (stance cues, heraldic) and their widths
//! are recomputed when the simulation, the map mode or the hover/selection change ([`build_mesh`]).
//!
//! Not ported yet: hatching of occupied provinces, breathing halo of the player's realm.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde_json::Value;

use super::data::ProvinceMap;
use super::modes::{self, Cue, MapMode, ProvinceView, Tuning, num};

/// Distance (map px) at which the raster is probed across an edge.
const PROBE_PX: f32 = 1.5;

/// One ring edge between two provinces. `side` is `+1` when the other province lies on the left
/// normal `(-dy, dx)` of the edge, `-1` on the right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub a: Vec2,
    pub b: Vec2,
    pub province: u16,
    pub other: u16,
    pub side: f32,
}

/// Ring edges that border another land province, from the geojson rings and the raster.
pub fn extract_edges(map: &ProvinceMap) -> Vec<Edge> {
    let mut edges = Vec::new();
    for province in map.provinces() {
        let index = province.index as u16;
        for ring in &province.rings {
            let n = ring.len();
            for i in 0..n {
                let a = Vec2::from(ring[i]);
                let b = Vec2::from(ring[(i + 1) % n]);
                let delta = b - a;
                if delta.length_squared() < 1e-6 {
                    continue;
                }
                let normal = Vec2::new(-delta.y, delta.x).normalize();
                let mid = (a + b) * 0.5;
                let left = map.index_at(mid.x + normal.x * PROBE_PX, mid.y + normal.y * PROBE_PX);
                let right = map.index_at(mid.x - normal.x * PROBE_PX, mid.y - normal.y * PROBE_PX);
                let (other, side) = match (left == index, right == index) {
                    (false, true) => (left, 1.0),
                    (true, false) => (right, -1.0),
                    _ => continue,
                };
                if other != 0 {
                    edges.push(Edge {
                        a,
                        b,
                        province: index,
                        other,
                        side,
                    });
                }
            }
        }
    }
    edges
}

/// How the borders look right now.
#[derive(Debug, Clone)]
pub struct BorderStyle {
    pub mode: MapMode,
    /// Owner of the hovered / selected province (their realms are drawn at full intensity).
    pub focus_owners: [String; 2],
    /// World size of one screen pixel of the 1080p tuning.
    pub world_per_px: f32,
}

/// Counts of the classified edges, for the dump.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BorderCounts {
    pub realm: usize,
    pub province_lines: usize,
}

struct Ribbon<'a> {
    positions: &'a mut Vec<[f32; 3]>,
    colors: &'a mut Vec<[f32; 4]>,
    indices: &'a mut Vec<u32>,
}

impl Ribbon<'_> {
    /// Quad along `a -> b`, from `offset0` to `offset1` along the edge normal (`inward` unit
    /// vector, scaled), `color` constant.
    #[allow(clippy::too_many_arguments)]
    fn quad(
        &mut self,
        a: Vec2,
        b: Vec2,
        dir: Vec2,
        near: f32,
        far: f32,
        color: Srgba,
        lift: &dyn Fn(Vec2) -> f32,
    ) {
        let corners = [a + dir * near, b + dir * near, b + dir * far, a + dir * far];
        let base = self.positions.len() as u32;
        let linear = LinearRgba::from(color).to_f32_array();
        for corner in corners {
            self.positions.push([corner.x, lift(corner), corner.y]);
            self.colors.push(linear);
        }
        self.indices
            .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
}

/// Builds the border ribbon mesh and its edge counts.
pub fn build_mesh(
    edges: &[Edge],
    view: &ProvinceView,
    heraldic: &HashMap<String, Srgba>,
    tuning: &Tuning,
    style: &BorderStyle,
    lift: &dyn Fn(Vec2) -> f32,
) -> (Mesh, BorderCounts) {
    let (_, full) = modes::mode_border_style(style.mode, tuning);
    let t = &tuning.borders;
    let rest = |key: &str| num(t, &["rest", key], 1.0);
    let enemy_focus = num(&tuning.cues, &["border", "enemy_focus"], 0.0) > 0.0;
    let (mut positions, mut colors, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    let mut ribbon = Ribbon {
        positions: &mut positions,
        colors: &mut colors,
        indices: &mut indices,
    };
    let mut counts = BorderCounts::default();
    let faction_color = |faction: &str| -> (Srgba, Cue) {
        let base = heraldic
            .get(faction)
            .copied()
            .unwrap_or(Srgba::new(0.35, 0.30, 0.25, 1.0));
        if style.mode == MapMode::Diplomacy {
            let key = view.stance_of(faction);
            return (
                modes::diplomacy_color(key).unwrap_or(Srgba::new(0.35, 0.30, 0.25, 1.0)),
                Cue::Other,
            );
        }
        let cue = view.cue_of(faction, &tuning.cues);
        (modes::border_color(base, cue, &tuning.cues), cue)
    };
    for edge in edges {
        let owner = view.owner_of(edge.province);
        let other_owner = view.owner_of(edge.other);
        if owner.is_empty() {
            continue;
        }
        let delta = edge.b - edge.a;
        let normal = Vec2::new(-delta.y, delta.x).normalize();
        // Inward = away from the other province.
        let inward = -normal * edge.side;
        if owner == other_owner {
            if edge.province > edge.other {
                continue; // one line per shared edge
            }
            let (color, _) = faction_color(owner);
            let width = num(t, &["province", "core_px"], 0.7) * style.world_per_px;
            let alpha = num(t, &["province", "core_alpha"], 0.32)
                * num(t, &["province", "core_brightness"], 1.0).min(1.0);
            ribbon.quad(
                edge.a,
                edge.b,
                inward,
                -width * 0.5,
                width * 0.5,
                color.with_alpha(alpha),
                lift,
            );
            counts.province_lines += 1;
            continue;
        }
        counts.realm += 1;
        let (color, cue) = faction_color(owner);
        let focused = full
            || style.focus_owners.iter().any(|f| f == owner)
            || (enemy_focus && cue == Cue::Enemy);
        let (width_scale, alpha_scale) = if focused {
            (1.0, 1.0)
        } else {
            (rest("width_scale"), rest("alpha_scale"))
        };
        let core = num(t, &["realm", "core_px"], 1.8) * style.world_per_px * width_scale;
        let core_alpha = (num(t, &["realm", "core_alpha"], 0.95) * alpha_scale).clamp(0.0, 1.0);
        let glow = num(t, &["realm", "glow_px"], 13.0) * style.world_per_px * width_scale;
        let glow_alpha = (num(t, &["realm", "glow_alpha"], 0.62)
            * alpha_scale
            * if focused {
                0.35
            } else {
                rest("glow_scale") * 0.35
            })
        .clamp(0.0, 1.0);
        // Soft halo inside the realm, then the bright core on the edge.
        if glow_alpha > 0.01 {
            ribbon.quad(
                edge.a,
                edge.b,
                inward,
                core,
                glow,
                color.with_alpha(glow_alpha),
                lift,
            );
        }
        ribbon.quad(
            edge.a,
            edge.b,
            inward,
            0.0,
            core,
            color.with_alpha(core_alpha),
            lift,
        );
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    (mesh, counts)
}

/// Fade of the border layer with the camera distance (`zoom` block): 0 under `fade_out_m`
/// (metres), 1 beyond `fade_in_m`. `distance` is in world units.
pub fn zoom_alpha(distance: f32, meters_per_px: f32, tuning: &Value) -> f32 {
    let out = num(tuning, &["zoom", "fade_out_m"], 200.0);
    let into = num(tuning, &["zoom", "fade_in_m"], 1400.0);
    smoothstep(out, into, distance * meters_per_px)
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::provinces::data::tests::real_map;

    #[test]
    fn edges_match_the_geojson_neighbours() {
        let map = real_map();
        let edges = extract_edges(map);
        assert!(edges.len() > 1000, "{} edges", edges.len());
        let paris = map.index_of_id("prov_ile_de_france");
        let feature = map.province(paris).unwrap();
        let across: std::collections::BTreeSet<u16> = edges
            .iter()
            .filter(|e| e.province == paris)
            .map(|e| e.other)
            .collect();
        assert!(!across.is_empty());
        let listed = across
            .iter()
            .filter(|i| feature.neighbors.contains(&map.province(**i).unwrap().id))
            .count();
        assert!(
            listed * 10 >= across.len() * 9,
            "{listed}/{} listed as neighbours",
            across.len()
        );
    }

    #[test]
    fn zoom_fade_is_monotone() {
        let t = serde_json::json!({"zoom": {"fade_out_m": 200.0, "fade_in_m": 1400.0}});
        assert_eq!(zoom_alpha(0.0, 719.0, &t), 0.0);
        assert_eq!(zoom_alpha(10.0, 719.0, &t), 1.0);
        assert!(zoom_alpha(1.0, 719.0, &t) > zoom_alpha(0.4, 719.0, &t));
    }

    #[test]
    fn realm_edges_split_by_owner() {
        let map = real_map();
        let edges = extract_edges(map);
        let root =
            crate::paths::find_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let tuning = Tuning::load(&root.join("data/map"));
        let owners: Vec<String> = std::iter::once(String::new())
            .chain(
                (1..=map.max_index())
                    .map(|i| map.province(i).map_or(String::new(), |p| p.owner.clone())),
            )
            .collect();
        let view = ProvinceView {
            owners,
            controllers: vec![],
            viewer: "fac_france".into(),
            ..Default::default()
        };
        let style = BorderStyle {
            mode: MapMode::Political,
            focus_owners: Default::default(),
            world_per_px: 3.0,
        };
        let (mesh, counts) = build_mesh(&edges, &view, &HashMap::new(), &tuning, &style, &|_| 0.0);
        assert!(
            counts.realm > 100 && counts.province_lines > 100,
            "{counts:?}"
        );
        assert!(mesh.count_vertices() >= (counts.realm + counts.province_lines) * 4);
    }
}
