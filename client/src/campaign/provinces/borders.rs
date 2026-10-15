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
use super::modes::{self, Cue, MapMode, ProvinceView, Tuning, num, text};

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
    /// Strategic (parchment) view: every realm at full intensity (`rest.parchment_focus`).
    pub parchment_focus: bool,
    /// Realm glow on (off in the Low quality preset).
    pub glow: bool,
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

/// Band widths in screen pixels BEFORE the view scale (rest / player factors are baked into the
/// mesh, the zoom factor is a uniform). Ports the width arithmetic of `fr1_borders`:
/// `wscale` (rest) multiplies every band, the player's `width_scale` only the realm bands, and the
/// glow follows `gscale` (rest, player `glow_scale`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bands {
    /// Realm core, from the edge line into the owner's side.
    pub core: f32,
    /// Dark outline under the core.
    pub outline: f32,
    /// Extent of the inner glow gradient.
    pub glow: f32,
    /// Occupation hatching band (realm edges).
    pub hatch_realm: f32,
    /// Occupation hatching band (province lines).
    pub hatch_province: f32,
    /// Province line, each side of the edge line.
    pub province: f32,
}

impl Bands {
    /// `focused`: full intensity (selected / hovered / enemy / diplomacy / strategic view);
    /// `mine`: the player's own realm.
    pub fn new(tuning: &Value, focused: bool, mine: bool) -> Self {
        let rest = if focused {
            1.0
        } else {
            num(tuning, &["rest", "width_scale"], 1.0)
        };
        let player = |key: &str| {
            if mine {
                num(tuning, &["player", key], 1.0)
            } else {
                1.0
            }
        };
        let ws = rest * player("width_scale");
        let hatch = num(tuning, &["occupied", "band_px"], 10.0);
        Self {
            core: num(tuning, &["realm", "core_px"], 1.6) * ws,
            outline: num(tuning, &["realm", "outline_px"], 2.6) * ws,
            glow: num(tuning, &["realm", "glow_px"], 9.0) * rest * player("glow_scale"),
            hatch_realm: hatch * ws,
            hatch_province: hatch * rest,
            province: num(tuning, &["province", "core_px"], 0.7) * rest,
        }
    }
}

/// View-dependent factors of the band widths (uniforms of the border material).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewScale {
    /// Multiplies every band offset (`fr1_width_scale` x parchment width scale).
    pub width: f32,
    /// Extra factor of the glow extent (parchment glow scale).
    pub glow: f32,
}

impl ViewScale {
    /// `distance`: camera distance in map units; `strategic`: weight of the parchment view [0, 1].
    pub fn at(tuning: &Value, distance: f32, strategic: f32) -> Self {
        let far = num(tuning, &["zoom", "far_distance"], 900.0);
        let far_scale = num(tuning, &["zoom", "far_width_scale"], 1.0);
        let zoom = 1.0 + (far_scale - 1.0) * smoothstep(far * 0.4, far, distance);
        let parchment = 1.0 + (num(tuning, &["parchment", "width_scale"], 1.0) - 1.0) * strategic;
        let glow = 1.0 + (num(tuning, &["parchment", "glow_scale"], 1.0) - 1.0) * strategic;
        Self {
            width: zoom * parchment,
            glow,
        }
    }

    /// Multiplier of the whole border opacity in the parchment view (`parchment.alpha_scale`).
    pub fn alpha(tuning: &Value, strategic: f32) -> f32 {
        1.0 + (num(tuning, &["parchment", "alpha_scale"], 1.0) - 1.0) * strategic
    }
}

/// Effective on-screen widths in pixels of one band set at a camera `distance`: what the player
/// sees, for the dump and the Godot comparison. Totals count both sides of the edge line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveWidths {
    /// Realm core, both sides together.
    pub realm_core: f32,
    /// Realm dark outline, both sides together.
    pub realm_outline: f32,
    /// Realm inner glow extent, one side.
    pub realm_glow: f32,
    /// Province line, both sides together.
    pub province_line: f32,
    /// Occupation hatching band (realm edge), one side.
    pub occupation_band: f32,
}

impl EffectiveWidths {
    pub fn new(
        tuning: &Value,
        distance: f32,
        strategic: f32,
        focused: bool,
        mine: bool,
    ) -> EffectiveWidths {
        let bands = Bands::new(tuning, focused, mine);
        let scale = ViewScale::at(tuning, distance, strategic);
        EffectiveWidths {
            realm_core: 2.0 * bands.core * scale.width,
            realm_outline: 2.0 * bands.outline * scale.width,
            realm_glow: bands.glow * scale.width * scale.glow,
            province_line: 2.0 * bands.province * scale.width,
            occupation_band: bands.hatch_realm * scale.width,
        }
    }
}

/// One mesh under construction: per vertex the edge point, the inward direction, the band
/// (offset in px, kind) and the colour. See `shaders/border.wesl`.
#[derive(Default)]
struct Ribbon {
    positions: Vec<[f32; 3]>,
    dirs: Vec<[f32; 2]>,
    bands: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Ribbon {
    /// Quad along `a -> b` from `near` to `far` pixels along `dir` (unit, in the map plane);
    /// `kind`: 0 solid, > 0 glow extent in px, < 0 hatching band in px.
    #[allow(clippy::too_many_arguments)]
    fn quad(
        &mut self,
        a: Vec2,
        b: Vec2,
        dir: Vec2,
        (near, far): (f32, f32),
        kind: f32,
        color: LinearRgba,
        lift: &dyn Fn(Vec2) -> f32,
    ) {
        let base = self.positions.len() as u32;
        let rgba = color.to_f32_array();
        for (point, offset) in [(a, near), (b, near), (b, far), (a, far)] {
            self.positions.push([point.x, lift(point), point.y]);
            self.dirs.push(dir.to_array());
            self.bands.push([offset, kind]);
            self.colors.push(rgba);
        }
        self.indices
            .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    /// Appends `other` after this ribbon's triangles (drawn over them).
    fn append(&mut self, other: Ribbon) {
        let base = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.dirs.extend(other.dirs);
        self.bands.extend(other.bands);
        self.colors.extend(other.colors);
        self.indices
            .extend(other.indices.into_iter().map(|i| i + base));
    }

    fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        let normals = vec![[0.0, 1.0, 0.0]; self.positions.len()];
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.dirs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, self.bands);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// Rest style of a colour (`rest.saturation`, `rest.brightness`): desaturated towards its
/// luminance, then dimmed, in linear light as in the shader.
fn rest_look(color: LinearRgba, saturation: f32, brightness: f32) -> LinearRgba {
    let luma = 0.2126 * color.red + 0.7152 * color.green + 0.0722 * color.blue;
    let mix = |c: f32| (luma + (c - luma) * saturation) * brightness;
    LinearRgba::new(
        mix(color.red),
        mix(color.green),
        mix(color.blue),
        color.alpha,
    )
}

fn scaled(color: LinearRgba, factor: f32, alpha: f32) -> LinearRgba {
    LinearRgba::new(
        color.red * factor,
        color.green * factor,
        color.blue * factor,
        alpha,
    )
}

/// Builds the border ribbon mesh, the halo mesh of the player's realm and the edge counts.
///
/// Layers are drawn in the order of `fr1_borders`: glow, occupation hatching, outline, core
/// (province lines last). Widths are in screen pixels (see [`Bands`]).
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
    let ink = text(t, &["neutral_color"])
        .and_then(modes::hex)
        .unwrap_or(Srgba::new(0.078, 0.055, 0.031, 1.0));
    let ink = LinearRgba::from(ink);
    let mut glow_layer = Ribbon::default();
    let mut hatch_layer = Ribbon::default();
    let mut outline_layer = Ribbon::default();
    let mut core_layer = Ribbon::default();
    let mut line_layer = Ribbon::default();
    let mut halo = Ribbon::default();
    let hatch_alpha = num(t, &["occupied", "alpha"], 0.75);
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
        let realm = owner != other_owner || controller != view.controller_of(edge.other);
        let occupied = !controller.is_empty() && controller != owner;
        let (own, cue) = faction_color(owner);
        let mine = !view.viewer.is_empty() && owner == view.viewer;
        let focused = full
            || style.parchment_focus
            || style.focus_owners.iter().any(|f| f == owner)
            || (enemy_focus && cue == Cue::Enemy);
        let bands = Bands::new(t, focused, mine);
        let alpha_scale = if focused { 1.0 } else { rest("alpha_scale") };
        let (saturation, brightness) = if focused {
            (1.0, 1.0)
        } else {
            (rest("saturation"), rest("brightness"))
        };
        let look = |c: Srgba| rest_look(LinearRgba::from(c), saturation, brightness);
        let own_linear = look(own);
        let realm_bright = num(t, &["realm", "core_brightness"], 1.0)
            * if mine {
                num(t, &["player", "brightness"], 1.0)
            } else {
                1.0
            };
        if style.hatch && occupied {
            let (occupier, _) = faction_color(controller);
            let (band, bright) = if realm {
                (bands.hatch_realm, realm_bright)
            } else {
                (bands.hatch_province, 1.0)
            };
            hatch_layer.quad(
                edge.a,
                edge.b,
                inward,
                (0.0, band),
                -band,
                scaled(look(occupier), bright, hatch_alpha * alpha_scale),
                lift,
            );
            counts.hatched += 1;
        }
        if !realm {
            if edge.province > edge.other {
                continue; // one line per shared edge, painted on both sides of it
            }
            let alpha = num(t, &["province", "core_alpha"], 0.32) * alpha_scale;
            line_layer.quad(
                edge.a,
                edge.b,
                inward,
                (-bands.province, bands.province),
                0.0,
                scaled(
                    own_linear,
                    num(t, &["province", "core_brightness"], 1.0),
                    alpha,
                ),
                lift,
            );
            counts.province_lines += 1;
            continue;
        }
        counts.realm += 1;
        counts.halo += usize::from(mine);
        // Inner glow: gradient from the edge line, alpha `glow_alpha` x rest glow factor.
        let rest_glow = if focused { 1.0 } else { rest("glow_scale") };
        let glow_alpha = num(t, &["realm", "glow_alpha"], 0.55) * rest_glow * alpha_scale;
        if style.glow && glow_alpha > 0.004 {
            let target = if mine { &mut halo } else { &mut glow_layer };
            target.quad(
                edge.a,
                edge.b,
                inward,
                (0.0, bands.glow),
                bands.glow,
                scaled(own_linear, realm_bright, glow_alpha),
                lift,
            );
        }
        outline_layer.quad(
            edge.a,
            edge.b,
            inward,
            (0.0, bands.outline),
            0.0,
            scaled(
                ink,
                1.0,
                num(t, &["realm", "outline_alpha"], 0.55) * alpha_scale,
            ),
            lift,
        );
        core_layer.quad(
            edge.a,
            edge.b,
            inward,
            (0.0, bands.core),
            0.0,
            scaled(
                own_linear,
                realm_bright,
                num(t, &["realm", "core_alpha"], 0.95) * alpha_scale,
            ),
            lift,
        );
    }
    let mut ribbon = glow_layer;
    for layer in [hatch_layer, outline_layer, core_layer, line_layer] {
        ribbon.append(layer);
    }
    (ribbon.into_mesh(), halo.into_mesh(), counts)
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

    fn godot_tuning() -> Value {
        let root =
            crate::paths::find_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        Tuning::load(&root.join("data/map")).borders
    }

    /// Godot arithmetic of `fr1_borders` written out longhand (data/map/faction_borders.json:
    /// core 1.8, outline 2.8, glow 13, province 0.7, hatch band 10; rest 0.6; player 1.25 / 1.35;
    /// far_width_scale 1.25 between 0.4 x 900 and 900).
    #[test]
    fn widths_match_the_godot_formula() {
        let t = godot_tuning();
        let far = |d: f32| 1.0 + 0.25 * smoothstep(360.0, 900.0, d);
        for d in [2600.0, 1200.0, 900.0, 630.0, 360.0, 200.0, 60.0] {
            let rest = EffectiveWidths::new(&t, d, 0.0, false, false);
            let full = EffectiveWidths::new(&t, d, 0.0, true, false);
            let me = EffectiveWidths::new(&t, d, 0.0, true, true);
            let eps = 1e-4;
            assert!((full.realm_core - 2.0 * 1.8 * far(d)).abs() < eps, "{d}");
            assert!((full.realm_outline - 2.0 * 2.8 * far(d)).abs() < eps);
            assert!((full.realm_glow - 13.0 * far(d)).abs() < eps);
            assert!((full.province_line - 2.0 * 0.7 * far(d)).abs() < eps);
            assert!((full.occupation_band - 10.0 * far(d)).abs() < eps);
            // Rest style: every band x 0.6 (the glow follows the rest width, not the rest glow).
            assert!((rest.realm_core - 0.6 * full.realm_core).abs() < eps);
            assert!((rest.realm_glow - 0.6 * full.realm_glow).abs() < eps);
            assert!((rest.province_line - 0.6 * full.province_line).abs() < eps);
            // The player's realm: width x 1.25, glow x 1.35, hatch band follows the width.
            assert!((me.realm_core - 1.25 * full.realm_core).abs() < eps);
            assert!((me.realm_glow - 1.35 * full.realm_glow).abs() < eps);
            assert!((me.occupation_band - 1.25 * full.occupation_band).abs() < eps);
            assert!((me.province_line - full.province_line).abs() < eps);
        }
        // Near the ground the bands keep their pixel size; far away they swell by far_width_scale.
        let near = EffectiveWidths::new(&t, 60.0, 0.0, true, false);
        let high = EffectiveWidths::new(&t, 2600.0, 0.0, true, false);
        assert!((near.realm_core - 3.6).abs() < 1e-4);
        assert!((high.realm_core - 4.5).abs() < 1e-4);
    }

    #[test]
    fn parchment_view_widens_the_bands_and_narrows_the_glow() {
        let t = godot_tuning();
        let map = ViewScale::at(&t, 60.0, 0.0);
        let parchment = ViewScale::at(&t, 60.0, 1.0);
        assert_eq!((map.width, map.glow), (1.0, 1.0));
        assert!((parchment.width - 1.7).abs() < 1e-5);
        assert!((parchment.glow - 0.6).abs() < 1e-5);
        assert!((ViewScale::alpha(&t, 1.0) - 1.15).abs() < 1e-5);
    }

    #[test]
    fn ribbon_offsets_are_in_pixels_and_layers_are_ordered() {
        let mut ribbon = Ribbon::default();
        let colour = LinearRgba::WHITE;
        ribbon.quad(
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            (0.0, 4.0),
            0.0,
            colour,
            &|_| 7.0,
        );
        assert_eq!(ribbon.positions.len(), 4);
        assert_eq!(
            ribbon.bands,
            vec![[0.0, 0.0], [0.0, 0.0], [4.0, 0.0], [4.0, 0.0]]
        );
        assert!(ribbon.positions.iter().all(|p| p[1] == 7.0));
        let mut second = Ribbon::default();
        second.quad(
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            (0.0, 1.0),
            9.0,
            colour,
            &|_| 0.0,
        );
        ribbon.append(second);
        assert_eq!(ribbon.indices.len(), 12);
        assert_eq!(ribbon.indices[6], 4);
    }

    #[test]
    fn rest_look_dims_and_desaturates() {
        let c = LinearRgba::new(0.8, 0.1, 0.1, 1.0);
        let grey = rest_look(c, 0.0, 1.0);
        assert!((grey.red - grey.green).abs() < 1e-6);
        let same = rest_look(c, 1.0, 1.0);
        assert!((same.green - c.green).abs() < 1e-6 && (same.red - c.red).abs() < 1e-6);
        assert!(rest_look(c, 1.0, 0.8).red < c.red);
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
            parchment_focus: false,
            glow: true,
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
            parchment_focus: false,
            glow: true,
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
