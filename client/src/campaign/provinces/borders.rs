//! Faction borders. Ports game/scripts/map/faction_borders.gd (ADR 0074, DZ, EN, TB2).
//!
//! Godot paints the borders in the terrain fragment shader; here they are their own ribbon mesh
//! (flat, or draped on a [`SurfaceHeight`] when the terrain lot installs one). Geometry comes from
//! the province rings of `provinces.geojson`: each ring edge is probed against the province raster
//! to find the province across it ([`extract_edges`], once). Which edges are realm borders (owner
//! differs) or province lines (same owner), their colours (stance cues, heraldic) and their widths
//! are recomputed when the simulation, the map mode or the hover/selection change ([`build_mesh`]).
//!
//! Occupied provinces (controller differs from owner) get diagonal hatching in the occupier's
//! colour along their borders ([`hatch_edge`]); the glow of the player's realm goes to its own mesh
//! so that the material can breathe it (`player.pulse_*`, `overlay::pulse_halo`).

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
    /// Occupation hatching on (off in the Low quality preset).
    pub hatch: bool,
}

/// Counts of the classified edges, for the dump.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BorderCounts {
    pub realm: usize,
    pub province_lines: usize,
    /// Edges carrying occupation hatching.
    pub hatched: usize,
    /// Edges of the player's realm (their glow is in the halo mesh).
    pub halo: usize,
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
        self.corners(corners, color, lift);
    }

    fn corners(&mut self, corners: [Vec2; 4], color: Srgba, lift: &dyn Fn(Vec2) -> f32) {
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

/// Diagonal hatching (lines `x + z = const`, fixed to the map) of the band of `band` map units
/// inside the edge `a -> b`, stripes of `duty * period` every `period`. Edges nearly parallel to
/// the stripes get a solid band at the duty-weighted alpha. Returns the number of quads.
#[allow(clippy::too_many_arguments)]
fn hatch_edge(
    ribbon: &mut Ribbon,
    a: Vec2,
    b: Vec2,
    inward: Vec2,
    band: f32,
    period: f32,
    duty: f32,
    color: Srgba,
    lift: &dyn Fn(Vec2) -> f32,
) -> usize {
    let mut stripe = Vec2::new(1.0, -1.0).normalize();
    if stripe.dot(inward) < 0.0 {
        stripe = -stripe;
    }
    let slant = stripe.dot(inward);
    if slant < 0.3 {
        let solid = color.with_alpha(color.alpha * duty);
        ribbon.quad(a, b, inward, 0.0, band, solid, lift);
        return 1;
    }
    let reach = stripe * (band / slant);
    // Stripes are the level sets of `f = x + z`, `period_f` apart along `f`.
    let period_f = period * std::f32::consts::SQRT_2;
    let (fa, fb) = (a.x + a.y, b.x + b.y);
    if (fb - fa).abs() < 1e-4 {
        return 0;
    }
    let (lo, hi) = (fa.min(fb), fa.max(fb));
    let first = ((lo - duty * period_f) / period_f).floor() as i32;
    let last = (hi / period_f).ceil() as i32;
    let at = |f: f32| a + (b - a) * ((f - fa) / (fb - fa)).clamp(0.0, 1.0);
    let mut quads = 0;
    for k in first..=last {
        let f0 = k as f32 * period_f;
        let f1 = f0 + duty * period_f;
        if f1 < lo || f0 > hi {
            continue;
        }
        let (e0, e1) = (at(f0), at(f1));
        ribbon.corners([e0, e1, e1 + reach, e0 + reach], color, lift);
        quads += 1;
    }
    quads
}

/// Builds the border ribbon mesh, the halo mesh of the player's realm and the edge counts.
pub fn build_mesh(
    edges: &[Edge],
    view: &ProvinceView,
    heraldic: &HashMap<String, Srgba>,
    tuning: &Tuning,
    style: &BorderStyle,
    lift: &dyn Fn(Vec2) -> f32,
) -> (Mesh, Mesh, BorderCounts) {
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
    let (mut halo_positions, mut halo_colors, mut halo_indices) =
        (Vec::new(), Vec::new(), Vec::new());
    let mut halo = Ribbon {
        positions: &mut halo_positions,
        colors: &mut halo_colors,
        indices: &mut halo_indices,
    };
    let occupied = &tuning.borders["occupied"];
    let hatch_period = num(occupied, &["hatch_period_px"], 7.0) * style.world_per_px;
    let hatch_duty = num(occupied, &["hatch_duty"], 0.5).clamp(0.05, 0.95);
    let hatch_band = num(occupied, &["band_px"], 10.0) * style.world_per_px;
    let hatch_alpha = num(occupied, &["alpha"], 0.75);
    let player = |key: &str, default: f32| num(t, &["player", key], default);
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
        let controller = view.controller_of(edge.province);
        if style.hatch && !controller.is_empty() && controller != owner {
            let (occupier, _) = faction_color(controller);
            let quads = hatch_edge(
                &mut ribbon,
                edge.a,
                edge.b,
                inward,
                hatch_band,
                hatch_period,
                hatch_duty,
                occupier.with_alpha(hatch_alpha),
                lift,
            );
            counts.hatched += usize::from(quads > 0);
        }
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
        let mine = !view.viewer.is_empty() && owner == view.viewer;
        counts.halo += usize::from(mine);
        let (own_width, own_glow, own_bright) = if mine {
            (
                player("width_scale", 1.0),
                player("glow_scale", 1.0),
                player("brightness", 1.0),
            )
        } else {
            (1.0, 1.0, 1.0)
        };
        let color = if mine {
            let l = LinearRgba::from(color);
            Srgba::from(LinearRgba::new(
                l.red * own_bright,
                l.green * own_bright,
                l.blue * own_bright,
                l.alpha,
            ))
        } else {
            color
        };
        let width_scale = width_scale * own_width;
        let core = num(t, &["realm", "core_px"], 1.8) * style.world_per_px * width_scale;
        let core_alpha = (num(t, &["realm", "core_alpha"], 0.95) * alpha_scale).clamp(0.0, 1.0);
        let glow =
            num(t, &["realm", "glow_px"], 13.0) * style.world_per_px * width_scale * own_glow;
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
            let target = if mine { &mut halo } else { &mut ribbon };
            target.quad(
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
    (
        ribbon_mesh(positions, colors, indices),
        ribbon_mesh(halo_positions, halo_colors, halo_indices),
        counts,
    )
}

fn ribbon_mesh(positions: Vec<[f32; 3]>, colors: Vec<[f32; 4]>, indices: Vec<u32>) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
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
            hatch: true,
        };
        let (mesh, _halo, counts) =
            build_mesh(&edges, &view, &HashMap::new(), &tuning, &style, &|_| 0.0);
        assert!(
            counts.realm > 100 && counts.province_lines > 100,
            "{counts:?}"
        );
        assert!(mesh.count_vertices() >= (counts.realm + counts.province_lines) * 4);
    }

    #[test]
    fn occupied_provinces_are_hatched_and_player_halo_is_separate() {
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
        let mut controllers = owners.clone();
        let paris = map.index_of_id("prov_ile_de_france");
        controllers[usize::from(paris)] = "fac_england".into();
        let view = ProvinceView {
            owners,
            controllers,
            viewer: "fac_france".into(),
            ..Default::default()
        };
        let style = BorderStyle {
            mode: MapMode::Political,
            focus_owners: Default::default(),
            world_per_px: 3.0,
            hatch: true,
        };
        let (_, halo, counts) =
            build_mesh(&edges, &view, &HashMap::new(), &tuning, &style, &|_| 0.0);
        assert!(counts.hatched > 5, "{counts:?}");
        assert!(counts.halo > 10 && halo.count_vertices() > 0, "{counts:?}");
        let (plain, _, none) = build_mesh(
            &edges,
            &view,
            &HashMap::new(),
            &tuning,
            &BorderStyle {
                hatch: false,
                ..style
            },
            &|_| 0.0,
        );
        assert_eq!(none.hatched, 0);
        assert!(plain.count_vertices() > 0);
    }
}
