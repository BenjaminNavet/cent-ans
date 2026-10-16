//! Sea lanes drawn in ink on the sea like a portolan chart (lot campwater2). Port of
//! `game/scripts/map/sea_lane_layer.gd` + `sea_lane.gdshader`.
//!
//! Geometry: `data/map/sea_lanes_px.json` (map pixels); turn state: `sim_campaign::sea_lanes::sea_lanes`
//! (sea held, blockade, storms, trade routes). Three meshes per ink (free sea, sea held by the
//! player or a non-hostile ally, sea held by an enemy) and per kind (open sea: long dashes,
//! coasting: short dashes). The stretches shared with a longer lane are drawn once (`build_runs`).
//! The layer hides at the valley tier (`ZoomView::valley > 0.5`). Rendering only.
//!
//! Departures from Godot: the hover tooltip (`nearest_lane` / `tooltip`) is exposed as
//! [`nearest_lane`] and [`lane_tooltip`] but no HUD hooks it yet; the ink is blended without the
//! `depth_test_disabled` of Godot but with an always-passing depth test, which is the same.

use std::collections::HashMap;

use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, CompareFunction, ShaderType};
use bevy::shader::{Shader, ShaderRef};
use bevy::state::state_scoped::DespawnOnExit;
use data_model::SeaLaneKind;
use serde_json::{Value, json};
use sim_campaign::sea_lanes::{SeaLaneView, sea_lanes};

use super::RebuildWater;
use super::drape::{Drape, sync_drape};
use super::height::{GroundHeight as WaterGround, HeightSampler};
use super::materials::linear;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::terrain::relief_gpu::ReliefParams;
use crate::paths;
use crate::sim::Sim;

const LANE_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0c02");

/// World width of the ribbons (the screen width comes from the shader).
const WIDTH: f32 = 1.6;
/// World lift above the sea / draped ground.
const LIFT: f32 = 0.4;
/// Discreet ink (DN-MER): thin, not very opaque lines.
const INK: [f32; 4] = [0.12, 0.20, 0.33, 0.5];
const INK_OWN: [f32; 4] = [0.05, 0.22, 0.45, 0.6];
const INK_HOSTILE: [f32; 4] = [0.55, 0.10, 0.07, 0.65];
/// Resampling step (map px) and distance under which a stretch counts as already drawn.
const RESAMPLE_STEP: f32 = 6.0;
const MERGE_DISTANCE: f32 = 22.0;
/// Dashes in screen pixels: long on the open sea, short when coasting.
const DASH_OPEN: f32 = 16.0;
const DASH_COASTAL: f32 = 8.0;
const DASH_FILL: f32 = 0.62;

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct LaneParams {
    pub color: Vec4,
    /// x: min px, y: max px, z: dash period px, w: dash fill.
    pub a: Vec4,
    /// x: world lift, y: alpha.
    pub b: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LaneMaterial {
    #[uniform(0)]
    pub params: LaneParams,
    #[uniform(1)]
    pub relief: ReliefParams,
    #[texture(2, sample_type = "float", filterable = false)]
    pub relief_floor: Handle<Image>,
}

impl Drape for LaneMaterial {
    fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>) {
        self.relief = params;
        self.relief_floor = floor.clone();
    }
    fn relief(&self) -> (ReliefParams, &Handle<Image>) {
        (self.relief, &self.relief_floor)
    }
}

impl Material for LaneMaterial {
    fn vertex_shader() -> ShaderRef {
        LANE_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        LANE_SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn depth_bias(&self) -> f32 {
        // Godot `render_priority = 1`.
        1.0
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        // `depth_test_disabled`: the ink is drawn over everything.
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_compare = Some(CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

// --- Geometry (pure) -----------------------------------------------------------------------------

/// A lane of the geometry file.
#[derive(Debug, Clone)]
pub struct LaneGeometry {
    pub id: String,
    pub from: String,
    pub to: String,
    pub points: Vec<Vec2>,
}

/// A stretch to draw: points and arc length from the start of the lane.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub points: Vec<Vec2>,
    pub arcs: Vec<f32>,
}

pub fn parse_lanes(text: &str) -> Vec<LaneGeometry> {
    let Ok(root) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    root.get("lanes")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|lane| {
                    let points: Vec<Vec2> = lane
                        .get("points")?
                        .as_array()?
                        .iter()
                        .filter_map(|p| {
                            Some(Vec2::new(
                                p.get(0)?.as_f64()? as f32,
                                p.get(1)?.as_f64()? as f32,
                            ))
                        })
                        .collect();
                    let id = lane.get("id")?.as_str()?.to_owned();
                    (points.len() >= 2).then(|| LaneGeometry {
                        id,
                        from: lane
                            .get("from")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned(),
                        to: lane
                            .get("to")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned(),
                        points,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn polyline_length(points: &[Vec2]) -> f32 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn cell_of(p: Vec2) -> IVec2 {
    IVec2::new(
        (p.x / MERGE_DISTANCE).floor() as i32,
        (p.y / MERGE_DISTANCE).floor() as i32,
    )
}

fn near_drawn(grid: &HashMap<IVec2, Vec<Vec2>>, p: Vec2) -> bool {
    let c = cell_of(p);
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let Some(others) = grid.get(&(c + IVec2::new(dx, dy)))
                && others.iter().any(|o| o.distance(p) < MERGE_DISTANCE)
            {
                return true;
            }
        }
    }
    false
}

/// De-duplicates the drawing (`_build_runs`): longest lanes first, each keeps only the stretches
/// further than `MERGE_DISTANCE` from the lanes already drawn.
pub fn build_runs(lanes: &[LaneGeometry]) -> HashMap<String, Vec<Run>> {
    let mut order: Vec<&LaneGeometry> = lanes.iter().collect();
    order.sort_by(|a, b| {
        let (la, lb) = (polyline_length(&a.points), polyline_length(&b.points));
        if (la - lb).abs() > 1e-4 {
            lb.total_cmp(&la)
        } else {
            a.id.cmp(&b.id)
        }
    });
    let mut grid: HashMap<IVec2, Vec<Vec2>> = HashMap::new();
    let mut out = HashMap::new();
    for lane in order {
        let mut samples = vec![lane.points[0]];
        let mut arcs = vec![0.0_f32];
        let mut arc = 0.0;
        for pair in lane.points.windows(2) {
            let seg = pair[0].distance(pair[1]);
            let steps = ((seg / RESAMPLE_STEP).ceil() as usize).max(1);
            for k in 1..=steps {
                samples.push(pair[0].lerp(pair[1], k as f32 / steps as f32));
                arcs.push(arc + seg * k as f32 / steps as f32);
            }
            arc += seg;
        }
        let mut runs = Vec::new();
        let mut current = Run {
            points: Vec::new(),
            arcs: Vec::new(),
        };
        for (p, a) in samples.iter().zip(&arcs) {
            if near_drawn(&grid, *p) {
                if current.points.len() >= 2 {
                    runs.push(std::mem::replace(
                        &mut current,
                        Run {
                            points: Vec::new(),
                            arcs: Vec::new(),
                        },
                    ));
                } else {
                    current = Run {
                        points: Vec::new(),
                        arcs: Vec::new(),
                    };
                }
            } else {
                current.points.push(*p);
                current.arcs.push(*a);
            }
        }
        if current.points.len() >= 2 {
            runs.push(current);
        }
        out.insert(lane.id.clone(), runs);
        for sample in samples {
            grid.entry(cell_of(sample)).or_default().push(sample);
        }
    }
    out
}

/// Mesh of the runs of `ids`: centre line, perpendicular in `normal`, `uv = (WIDTH, side)`,
/// `uv2.x` = arc length. `position.y` = altitude in metres (draped by the shader).
pub fn lane_mesh(
    ids: &[&str],
    runs: &HashMap<String, Vec<Run>>,
    ground: &dyn HeightSampler,
) -> Option<Mesh> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut uv2s: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for id in ids {
        for run in runs.get(*id).into_iter().flatten() {
            let base = positions.len() as u32;
            let count = run.points.len();
            for i in 0..count {
                let p = run.points[i];
                let dir = (run.points[(i + 1).min(count - 1)] - run.points[i.saturating_sub(1)])
                    .normalize_or_zero();
                let dir = if dir == Vec2::ZERO { Vec2::X } else { dir };
                let perp = Vec2::new(-dir.y, dir.x);
                let y = ground.seat_height_m(p.x, p.y).max(0.0);
                for side in [1.0_f32, -1.0] {
                    positions.push([p.x, y, p.y]);
                    normals.push([perp.x * side, 0.0, perp.y * side]);
                    uvs.push([WIDTH, side]);
                    uv2s.push([run.arcs[i], 0.0]);
                }
            }
            for i in 0..count as u32 - 1 {
                let a = base + i * 2;
                indices.extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
            }
        }
    }
    if positions.is_empty() {
        return None;
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv2s);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

/// Ink holder of a lane for the viewer: `hostile`, `own` (held by the player) or `neutral`.
pub fn holder_of(view: &SeaLaneView, player: &str) -> &'static str {
    if view.hostile {
        "hostile"
    } else if !player.is_empty()
        && view
            .control_faction
            .as_ref()
            .is_some_and(|f| f.as_str() == player)
    {
        "own"
    } else {
        "neutral"
    }
}

/// Style key `<holder>_<open|coastal>` (`refresh`).
pub fn style_of(view: &SeaLaneView, player: &str) -> String {
    let kind = if view.kind == SeaLaneKind::OpenSea {
        "open"
    } else {
        "coastal"
    };
    format!("{}_{kind}", holder_of(view, player))
}

fn material_for(style: &str) -> LaneMaterial {
    let holder = style.split('_').next().unwrap_or("neutral");
    let (ink, min_px) = match holder {
        "hostile" => (INK_HOSTILE, 1.8),
        "own" => (INK_OWN, 1.2),
        _ => (INK, 1.2),
    };
    LaneMaterial {
        params: LaneParams {
            color: linear([ink[0], ink[1], ink[2]], ink[3]),
            a: Vec4::new(
                min_px,
                2.2,
                if style.ends_with("open") {
                    DASH_OPEN
                } else {
                    DASH_COASTAL
                },
                DASH_FILL,
            ),
            b: Vec4::new(LIFT, 1.0, 0.0, 0.0),
        },
        relief: ReliefParams::default(),
        relief_floor: Handle::default(),
    }
}

/// Distance from `p` to the segment `a`-`b`.
pub fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Id of the lane closest to `world_xz` within `max_distance` map units (`nearest_lane`).
pub fn nearest_lane(
    layer: &SeaLaneLayer,
    world_xz: Vec2,
    max_distance: f32,
) -> Option<&SeaLaneView> {
    if layer.hidden {
        return None;
    }
    let mut best: Option<(&SeaLaneView, f32)> = None;
    for view in &layer.views {
        let Some(geometry) = layer.geometry.iter().find(|g| g.id == view.id) else {
            continue;
        };
        for pair in geometry.points.windows(2) {
            let d = distance_to_segment(world_xz, pair[0], pair[1]);
            if d < best.map_or(max_distance, |b| b.1) {
                best = Some((view, d));
            }
        }
    }
    best.map(|(view, _)| view)
}

/// Tooltip text of a lane (`SeaLaneLayer.tooltip`).
pub fn lane_tooltip(lane: &SeaLaneView) -> String {
    let mut lines = vec![format!(
        "{} : {} ↔ {}",
        lane.name,
        lane.from.as_str(),
        lane.to.as_str()
    )];
    lines.push(format!(
        "{}, {}, {} km — une saison de traversée",
        capitalize(&lane.kind_name),
        lane.sea_name,
        lane.length_km.round() as i64
    ));
    if lane.control_level > 0 {
        let holder = lane.control_faction.as_ref().map_or("", |f| f.as_str());
        if lane.hostile {
            let threat = if lane.blockade {
                "blocus, commerce coupé"
            } else {
                "traversées et commerce menacés"
            };
            lines.push(format!(
                "Mer tenue par l’ennemi ({holder}, {} %) : {threat}",
                lane.control_level
            ));
        } else {
            lines.push(format!("Mer tenue par {holder} ({} %)", lane.control_level));
        }
    }
    if lane.storm_loss_percent > 0.0 {
        lines.push(format!(
            "Gros temps : ~{} % des hommes perdus en traversée cette saison",
            lane.storm_loss_percent.ceil() as i64
        ));
    }
    if lane.trade_season_factor < 0.99 {
        lines.push(format!(
            "Commerce réduit par la saison (×{:.2})",
            lane.trade_season_factor
        ));
    }
    if !lane.trade_routes.is_empty() {
        lines.push(format!("Routes commerciales : {}", lane.trade_routes.len()));
    }
    lines.join("\n")
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

// --- Plugin --------------------------------------------------------------------------------------

#[derive(Component)]
struct LaneEntity;

/// State of the layer.
#[derive(Resource, Default)]
pub struct SeaLaneLayer {
    pub geometry: Vec<LaneGeometry>,
    runs: HashMap<String, Vec<Run>>,
    pub views: Vec<SeaLaneView>,
    /// Style of each lane in the last refresh; the meshes are rebuilt only when it changes.
    styles: HashMap<String, String>,
    pub meshes_by_style: HashMap<String, usize>,
    pub hidden: bool,
    refreshes: usize,
    /// Set when the geometry is (re)loaded: the next frame redraws whatever the sim state.
    dirty: bool,
}

pub struct SeaLanesPlugin;

impl Plugin for SeaLanesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SeaLaneLayer>()
            .register_dump(dump_lanes);
        let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() else {
            return;
        };
        let source = super::drape::with_relief(include_str!("shaders/lane.wesl"));
        if let Err(error) = shaders.insert(
            LANE_SHADER.id(),
            Shader::from_wesl(source, "water/lane.wesl"),
        ) {
            error!("water: shader water/lane.wesl: {error:?}");
        }
        app.add_plugins(MaterialPlugin::<LaneMaterial>::default())
            .add_systems(PostUpdate, sync_drape::<LaneMaterial>)
            .add_systems(OnEnter(AppState::Campaign), load_lanes)
            .add_systems(
                Update,
                (reload_on_rebuild, refresh_lanes, hide_lanes)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

fn load_lanes(
    mut layer: ResMut<SeaLaneLayer>,
    mut commands: Commands,
    old: Query<Entity, With<LaneEntity>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let geometry = paths::find_root()
        .ok()
        .and_then(|root| {
            std::fs::read_to_string(paths::data_dir(&root).join("map/sea_lanes_px.json")).ok()
        })
        .map(|text| parse_lanes(&text))
        .unwrap_or_default();
    layer.runs = build_runs(&geometry);
    layer.geometry = geometry;
    layer.views.clear();
    layer.styles.clear();
    layer.meshes_by_style.clear();
    layer.dirty = true;
}

/// The ground sampler may change (terrain loaded): redraw from the same geometry.
fn reload_on_rebuild(mut requests: MessageReader<RebuildWater>, mut layer: ResMut<SeaLaneLayer>) {
    if requests.read().count() > 0 {
        layer.styles.clear();
        layer.dirty = true;
    }
}

/// Redraws the lanes when the turn state changes the ink of one of them.
fn refresh_lanes(
    sim: Res<Sim>,
    ground: Res<WaterGround>,
    mut layer: ResMut<SeaLaneLayer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LaneMaterial>>,
    existing: Query<(Entity, &LaneEntity)>,
) {
    if layer.geometry.is_empty() || !(layer.dirty || sim.is_changed()) {
        return;
    }
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    let player = sim.player.as_str().to_owned();
    let views: Vec<SeaLaneView> = sea_lanes(state, data, &sim.player)
        .into_iter()
        .filter(|v| layer.runs.contains_key(&v.id))
        .collect();
    let styles: HashMap<String, String> = views
        .iter()
        .map(|v| (v.id.clone(), style_of(v, &player)))
        .collect();
    let unchanged = styles == layer.styles;
    layer.dirty = false;
    layer.views = views;
    if unchanged {
        return;
    }
    layer.styles = styles;
    layer.refreshes += 1;
    for (entity, _) in &existing {
        commands.entity(entity).despawn();
    }
    let mut groups: HashMap<&str, Vec<&str>> = HashMap::new();
    for (id, style) in &layer.styles {
        groups.entry(style.as_str()).or_default().push(id.as_str());
    }
    let mut counts = HashMap::new();
    for (style, mut ids) in groups {
        ids.sort_unstable();
        let Some(mesh) = lane_mesh(&ids, &layer.runs, ground.0.as_ref()) else {
            continue;
        };
        counts.insert(style.to_owned(), ids.len());
        commands.spawn((
            Name::new(format!("Lanes_{style}")),
            LaneEntity,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material_for(style))),
            if layer.hidden {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
            Transform::default(),
            NotShadowCaster,
            NoFrustumCulling,
            DespawnOnExit(AppState::Campaign),
        ));
    }
    layer.meshes_by_style = counts;
}

/// Hides the layer at the valley tier (`set_close_hidden(valley_weight > 0.5)`).
fn hide_lanes(
    zoom: Option<Res<ZoomView>>,
    mut layer: ResMut<SeaLaneLayer>,
    mut lanes: Query<&mut Visibility, With<LaneEntity>>,
) {
    let hidden = zoom.is_some_and(|z| z.valley > 0.5);
    if hidden != layer.hidden {
        layer.hidden = hidden;
    }
    for mut visibility in &mut lanes {
        let wanted = if hidden {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

fn dump_lanes(world: &World) -> (String, Value) {
    let Some(layer) = world.get_resource::<SeaLaneLayer>() else {
        return ("campaign_sea_lanes".into(), json!({"built": false}));
    };
    let runs: usize = layer.runs.values().map(Vec::len).sum();
    let samples: usize = layer.runs.values().flatten().map(|r| r.points.len()).sum();
    let mut styles: Vec<_> = layer
        .meshes_by_style
        .iter()
        .map(|(k, v)| json!({"style": k, "lanes": v}))
        .collect();
    styles.sort_by_key(|v| v["style"].as_str().unwrap_or("").to_owned());
    let hostile = layer.views.iter().filter(|v| v.hostile).count();
    (
        "campaign_sea_lanes".into(),
        json!({
            "lanes": layer.geometry.len(),
            "runs": runs,
            "run_samples": samples,
            "state_lanes": layer.views.len(),
            "hostile": hostile,
            "meshes": styles,
            "refreshes": layer.refreshes,
            "hidden": layer.hidden,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::water::height::FlatGround;

    fn lane(id: &str, pts: &[(f32, f32)]) -> LaneGeometry {
        LaneGeometry {
            id: id.into(),
            from: "a".into(),
            to: "b".into(),
            points: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
        }
    }

    #[test]
    fn parses_the_geometry_file() {
        let text = r#"{"lanes":[{"id":"l1","from":"a","to":"b","points":[[0,0],[10,0]]},{"id":"bad","points":[[1,1]]}]}"#;
        let lanes = parse_lanes(text);
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].points.len(), 2);
        assert!(parse_lanes("not json").is_empty());
    }

    #[test]
    fn shared_stretches_are_drawn_once() {
        let long = lane("long", &[(0.0, 0.0), (300.0, 0.0)]);
        let short = lane("short", &[(0.0, 5.0), (150.0, 5.0)]);
        let runs = build_runs(&[short, long]);
        assert_eq!(runs["long"].len(), 1);
        assert!(
            runs["short"].is_empty(),
            "the short lane is inside the corridor of the long one"
        );
        assert!((runs["long"][0].arcs.last().unwrap() - 300.0).abs() < 1e-3);
    }

    #[test]
    fn a_lane_leaving_the_corridor_keeps_its_own_part() {
        let long = lane("long", &[(0.0, 0.0), (400.0, 0.0)]);
        let branch = lane("branch", &[(0.0, 0.0), (100.0, 0.0), (100.0, 200.0)]);
        let runs = build_runs(&[long, branch]);
        let kept: f32 = runs["branch"].iter().map(|r| r.points.len() as f32).sum();
        assert!(kept > 10.0, "the vertical branch is drawn");
        assert!(
            runs["branch"]
                .iter()
                .all(|r| r.points.iter().all(|p| p.y > 5.0))
        );
    }

    #[test]
    fn mesh_has_two_vertices_per_sample() {
        let runs = build_runs(&[lane("l", &[(0.0, 0.0), (60.0, 0.0)])]);
        let mesh = lane_mesh(&["l"], &runs, &FlatGround::default()).unwrap();
        let samples = runs["l"][0].points.len();
        assert_eq!(mesh.count_vertices(), samples * 2);
        assert!(lane_mesh(&["none"], &runs, &FlatGround::default()).is_none());
    }

    #[test]
    fn segment_distance() {
        let (a, b) = (Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        assert_eq!(distance_to_segment(Vec2::new(5.0, 3.0), a, b), 3.0);
        assert_eq!(distance_to_segment(Vec2::new(-4.0, 3.0), a, b), 5.0);
    }
}
