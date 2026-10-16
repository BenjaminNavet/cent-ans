//! Map layers of the filters menu: « Routes commerciales » (key V, `map_toggle_trade`) and
//! « Signes » (worksites, incidents, encounters).
//!
//! Ports `map/trade_route_layer.gd` (ribbons draped on the relief, ink-sepia for live routes,
//! grey for cut ones, hidden when neither end is in sight, hidden at the valley / site zoom),
//! `campaign_map.gd::_toggle_trade_layer` and `MapReadability.sign_shown` (rules of the `signs`
//! block of `data/ui/campaign_map.json`). Routes come from `sim_campaign::trade_routes`.
//!
//! The hover tooltip of a route is `hud::hover` (`nearest_route`, `route_tooltip`). Difference
//! from Godot: a port-to-port leg is a straight line (the sea lanes are `water::lanes`).

use std::collections::BTreeMap;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde_json::{Value, json};

use super::HudSet;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::markers::places::Places;
use crate::campaign::markers::signs::WorksiteSign;
use crate::campaign::provinces::MapModeState;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::weather::fog::FogOfWar;
use crate::settings::KeyMap;
use crate::sim::Sim;
use crate::ui::kit::widgets::{ToggleChanged, Toggled};

/// Ribbon widths (map units): `MIN_WIDTH`, `MAX_WIDTH` of `trade_route_layer.gd`.
pub const MIN_WIDTH: f32 = 1.2;
pub const MAX_WIDTH: f32 = 5.0;
/// Value that widens a ribbon by one unit (`value / 40`).
const VALUE_PER_UNIT: f32 = 40.0;
/// Lift of the ribbon above the ground (`PolylineMesh.build(..., 0.55)`).
const LIFT: f32 = 0.55;
/// Distance between two ground samples along a ribbon (map units).
const STEP: f32 = 6.0;
/// Valley weight above which the ribbons hide (`set_close_hidden`).
const CLOSE_HIDDEN_WEIGHT: f32 = 0.5;
/// Ink-sepia of live routes and the grey of cut ones (`COLOR`, `COLOR_CUT`).
const COLOR: Color = Color::srgba(0.45, 0.18, 0.08, 0.9);
const COLOR_CUT: Color = Color::srgba(0.4, 0.4, 0.4, 0.35);

/// Names of the checkboxes of the filters menu.
pub const TRADE_CHECK: &str = "hud:layer:trade";
pub const SIGNS_CHECK: &str = "hud:layer:signs";

/// State of the optional map layers; off at the start of every game.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct MapLayers {
    pub trade: bool,
    pub signs: bool,
}

/// `signs` block of `campaign_map.json`: when does a sign of a kind show.
#[derive(Resource, Default, Debug, Clone)]
pub struct SignRules {
    kinds: BTreeMap<String, SignRule>,
}

#[derive(Debug, Clone, Default)]
struct SignRule {
    modes: Vec<String>,
    keep_urgent: bool,
    with_selected_army: bool,
}

impl SignRules {
    pub fn from_json(block: &Value) -> Self {
        let mut kinds = BTreeMap::new();
        if let Some(map) = block.as_object() {
            for (kind, rule) in map.iter().filter(|(_, v)| v.is_object()) {
                kinds.insert(
                    kind.clone(),
                    SignRule {
                        modes: rule["modes"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|m| m.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        keep_urgent: rule["keep_urgent"].as_bool().unwrap_or(false),
                        with_selected_army: rule["with_selected_army"].as_bool().unwrap_or(false),
                    },
                );
            }
        }
        Self { kinds }
    }

    pub fn load(root: &std::path::Path) -> Self {
        std::fs::read(root.join("data/ui/campaign_map.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map(|v| Self::from_json(&v["signs"]))
            .unwrap_or_default()
    }

    /// `MapReadability.sign_shown`: the « Signes » layer shows everything; else the rule of the
    /// kind (no rule: shown), by map mode, urgency and army selection.
    pub fn shown(
        &self,
        layer_on: bool,
        kind: &str,
        mode: &str,
        urgent: bool,
        army_selected: bool,
    ) -> bool {
        if layer_on {
            return true;
        }
        let Some(rule) = self.kinds.get(kind) else {
            return true;
        };
        rule.modes.iter().any(|m| m == mode)
            || (urgent && rule.keep_urgent)
            || (army_selected && rule.with_selected_army)
    }
}

/// Root of the ribbons (two meshes: live and cut routes).
#[derive(Component)]
struct TradeLayer;

#[derive(Component)]
struct TradeMesh {
    cut: bool,
}

/// One route to draw: points in map units, width, cut flag.
#[derive(Debug, Clone, PartialEq)]
pub struct RibbonRoute {
    pub id: String,
    pub points: Vec<Vec2>,
    pub width: f32,
    pub cut: bool,
    /// Hover text (`campaign_map.gd::_trade_route_tooltip`).
    pub tooltip: String,
}

/// Hover text of a trade route: « A ↔ B : route coupée (raison) » or
/// « A ↔ B — 12 livres/saison (drap, laine) — accord commercial — menacée ».
#[allow(clippy::too_many_arguments)]
pub fn route_tooltip(
    from: &str,
    to: &str,
    cut_reason: Option<&str>,
    total_value: i64,
    goods: &[String],
    agreement: bool,
    security: f64,
    cut: bool,
) -> String {
    if cut {
        return format!(
            "{from} ↔ {to} : route coupée ({})",
            cut_reason.unwrap_or("")
        );
    }
    let mut text = format!("{from} ↔ {to} — {total_value} livres/saison");
    if !goods.is_empty() {
        text.push_str(&format!(" ({})", goods.join(", ")));
    }
    if agreement {
        text.push_str(" — accord commercial");
    }
    if security < 0.99 {
        text.push_str(" — menacée");
    }
    text
}

/// Route of `routes` closest to `point` within `max_distance` (`TradeRouteLayer.nearest_route`).
pub fn nearest_route(
    routes: &[RibbonRoute],
    point: Vec2,
    max_distance: f32,
) -> Option<&RibbonRoute> {
    let mut best: Option<(&RibbonRoute, f32)> = None;
    for route in routes {
        for pair in route.points.windows(2) {
            let d = crate::campaign::water::lanes::distance_to_segment(point, pair[0], pair[1]);
            if d < best.map_or(max_distance, |b| b.1) {
                best = Some((route, d));
            }
        }
    }
    best.map(|b| b.0)
}

/// Width of a ribbon from the value of the route.
pub fn route_width(total_value: i64) -> f32 {
    (MIN_WIDTH + total_value as f32 / VALUE_PER_UNIT).clamp(MIN_WIDTH, MAX_WIDTH)
}

/// Points of a polyline with a sample at least every `STEP`.
fn densify(points: &[Vec2]) -> Vec<Vec2> {
    let mut out = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let pieces = ((a.distance(b) / STEP).ceil() as usize).max(1);
        for i in 0..pieces {
            out.push(a.lerp(b, i as f32 / pieces as f32));
        }
    }
    if let Some(last) = points.last() {
        out.push(*last);
    }
    out
}

/// Ribbon mesh of `routes` (`PolylineMesh.build`): two vertices per sample, offset along the
/// normal of the polyline, at `height(x, z) + LIFT`.
pub fn ribbon_mesh(routes: &[&RibbonRoute], height: &dyn Fn(f32, f32) -> f32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for route in routes {
        let samples = densify(&route.points);
        if samples.len() < 2 {
            continue;
        }
        let half = route.width * 0.5;
        let base = positions.len() as u32;
        for (i, p) in samples.iter().enumerate() {
            let before = samples[i.saturating_sub(1)];
            let after = samples[(i + 1).min(samples.len() - 1)];
            let tangent = (after - before).normalize_or_zero();
            let normal = Vec2::new(-tangent.y, tangent.x);
            for side in [-1.0f32, 1.0] {
                let q = *p + normal * half * side;
                positions.push([q.x, height(q.x, q.y) + LIFT, q.y]);
            }
        }
        for i in 0..(samples.len() as u32 - 1) {
            let a = base + i * 2;
            indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let uvs = vec![[0.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Statistics of the layer (dump).
#[derive(Resource, Default, Debug)]
pub struct TradeStats {
    pub routes_total: usize,
    pub routes_drawn: usize,
    pub routes_cut: usize,
    pub rebuilds: u32,
    pub visible: bool,
}

#[derive(Resource, Default)]
pub struct DrawnRoutes(pub Vec<RibbonRoute>);

pub fn plugin(app: &mut App) {
    app.init_resource::<MapLayers>()
        .init_resource::<SignRules>()
        .init_resource::<TradeStats>()
        .init_resource::<DrawnRoutes>()
        .register_dump(dump_layers)
        .add_systems(
            OnEnter(AppState::Campaign),
            (reset_layers, load_rules, spawn_layer),
        )
        .add_systems(
            Update,
            (
                layer_key.run_if(super::flow::orders_open),
                layer_toggles,
                sync_checks,
                rebuild_trade,
                show_trade,
                show_signs,
            )
                .chain()
                .in_set(HudSet::Ui),
        );
}

fn reset_layers(mut layers: ResMut<MapLayers>, mut drawn: ResMut<DrawnRoutes>) {
    *layers = MapLayers::default();
    drawn.0.clear();
}

fn load_rules(mut rules: ResMut<SignRules>) {
    if let Ok(root) = crate::paths::find_root() {
        *rules = SignRules::load(&root);
    }
}

fn spawn_layer(mut commands: Commands) {
    commands
        .spawn((
            Name::new("TradeRoutesLayer"),
            TradeLayer,
            Transform::default(),
            Visibility::Hidden,
            bevy::state::state_scoped::DespawnOnExit(AppState::Campaign),
        ))
        .with_children(|layer| {
            for cut in [false, true] {
                layer.spawn((
                    Name::new(if cut { "CutRoutes" } else { "LiveRoutes" }),
                    TradeMesh { cut },
                    Transform::default(),
                    Visibility::Inherited,
                ));
            }
        });
}

/// `map_toggle_trade` (V).
fn layer_key(
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Option<Res<KeyMap>>,
    mut layers: ResMut<MapLayers>,
) {
    let pressed = keymap.map_or_else(
        || keys.just_pressed(KeyCode::KeyV),
        |map| map.just_pressed("map_toggle_trade", &keys),
    );
    if pressed {
        layers.trade = !layers.trade;
    }
}

fn layer_toggles(mut changed: MessageReader<ToggleChanged>, mut layers: ResMut<MapLayers>) {
    for change in changed.read() {
        match change.name.as_str() {
            TRADE_CHECK => layers.trade = change.on,
            SIGNS_CHECK => layers.signs = change.on,
            _ => {}
        }
    }
}

/// Keeps the checkboxes of the menu on the layer state (the V key flips it behind their back).
fn sync_checks(layers: Res<MapLayers>, mut checks: Query<(&Name, &mut Toggled)>) {
    if !layers.is_changed() {
        return;
    }
    for (name, mut toggled) in &mut checks {
        let wanted = match name.as_str() {
            TRADE_CHECK => layers.trade,
            SIGNS_CHECK => layers.signs,
            _ => continue,
        };
        if toggled.0 != wanted {
            toggled.0 = wanted;
        }
    }
}

/// The routes to draw this turn: path through the settlements, fog filter, widths.
fn collect_routes(sim: &Sim, places: &Places, fog: Option<&FogOfWar>) -> (usize, Vec<RibbonRoute>) {
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return (0, Vec::new());
    };
    let all = sim_campaign::trade_routes(state, data);
    let total = all.len();
    let mut routes = Vec::new();
    for route in all {
        if route.path.len() < 2 {
            continue;
        }
        let visible = |id: &data_model::SettlementId| {
            places
                .get(id.as_str())
                .is_some_and(|p| fog.is_none_or(|f| f.is_province_visible(&p.province)))
        };
        if !visible(&route.from_settlement) && !visible(&route.to_settlement) {
            continue;
        }
        let points: Vec<Vec2> = route
            .path
            .iter()
            .filter_map(|id| places.get(id.as_str()).map(|p| p.px))
            .collect();
        if points.len() < 2 {
            continue;
        }
        let hub_name = |hub: &str| -> String {
            data.trade
                .as_ref()
                .and_then(|catalog| catalog.hub(hub))
                .map_or_else(|| hub.to_owned(), |h| h.name.display.clone())
        };
        let goods: Vec<String> = route
            .goods
            .iter()
            .map(|g| {
                data.resources
                    .get(g)
                    .map_or_else(|| g.as_str().to_owned(), |r| r.name.display.to_lowercase())
            })
            .collect();
        routes.push(RibbonRoute {
            id: route.id.clone(),
            points,
            width: route_width(route.total_value()),
            cut: route.cut,
            tooltip: route_tooltip(
                &hub_name(&route.from_hub),
                &hub_name(&route.to_hub),
                route.cut_reason.as_deref(),
                route.total_value(),
                &goods,
                route.agreement,
                route.security,
                route.cut,
            ),
        });
    }
    (total, routes)
}

#[allow(clippy::too_many_arguments)]
fn rebuild_trade(
    layers: Res<MapLayers>,
    sim: Res<Sim>,
    places: Res<Places>,
    fog: Option<Res<FogOfWar>>,
    ground: GroundHeight,
    relief: Option<Res<ReliefState>>,
    zoom: Option<Res<ZoomView>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    layer: Query<&Children, With<TradeLayer>>,
    parts: Query<(Entity, &TradeMesh)>,
    mut drawn: ResMut<DrawnRoutes>,
    mut stats: ResMut<TradeStats>,
    mut commands: Commands,
    mut last: Local<(u32, u64, usize, i32)>,
) {
    if !layers.trade || places.places.is_empty() {
        return;
    }
    let Ok(children) = layer.single() else { return };
    let (Some(state), Some(_)) = (sim.state.as_ref(), sim.data.as_ref()) else {
        return;
    };
    let fog_revision = fog.as_deref().map_or(0, |f| f.revision);
    // The displayed relief follows the camera distance: re-drape when it moved by ~4 %.
    let bucket = zoom
        .as_deref()
        .map_or(0, |z| (z.distance.max(1.0).ln() * 25.0) as i32);
    let relief_changed = relief.is_some_and(|r| r.is_changed());
    let key = (state.turn, fog_revision, places.places.len(), bucket);
    if *last == key && !relief_changed && !drawn.0.is_empty() {
        return;
    }
    *last = key;
    let (total, routes) = collect_routes(&sim, &places, fog.as_deref());
    let height = |x: f32, z: f32| ground.at(x, z).unwrap_or(0.0);
    for (entity, part) in &parts {
        if !children.contains(&entity) {
            continue;
        }
        let selected: Vec<&RibbonRoute> = routes.iter().filter(|r| r.cut == part.cut).collect();
        if selected.is_empty() {
            commands
                .entity(entity)
                .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>();
            continue;
        }
        let mut mesh_routes = selected;
        if part.cut {
            // Cut routes are drawn thinner (`MIN_WIDTH * 0.6`).
            let thin: Vec<RibbonRoute> = mesh_routes
                .iter()
                .map(|r| RibbonRoute {
                    width: MIN_WIDTH * 0.6,
                    ..(*r).clone()
                })
                .collect();
            let refs: Vec<&RibbonRoute> = thin.iter().collect();
            let mesh = ribbon_mesh(&refs, &height);
            insert_part(
                &mut commands,
                entity,
                mesh,
                COLOR_CUT,
                &mut meshes,
                &mut materials,
            );
            mesh_routes.clear();
        } else {
            let mesh = ribbon_mesh(&mesh_routes, &height);
            insert_part(
                &mut commands,
                entity,
                mesh,
                COLOR,
                &mut meshes,
                &mut materials,
            );
        }
    }
    stats.routes_total = total;
    stats.routes_drawn = routes.len();
    stats.routes_cut = routes.iter().filter(|r| r.cut).count();
    stats.rebuilds += 1;
    drawn.0 = routes;
}

fn insert_part(
    commands: &mut Commands,
    entity: Entity,
    mesh: Mesh,
    color: Color,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.entity(entity).insert((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            depth_bias: 4.0,
            ..default()
        })),
    ));
}

fn show_trade(
    layers: Res<MapLayers>,
    zoom: Option<Res<ZoomView>>,
    mut layer: Query<&mut Visibility, With<TradeLayer>>,
    mut stats: ResMut<TradeStats>,
) {
    let close = zoom.is_some_and(|z| z.valley > CLOSE_HIDDEN_WEIGHT);
    let shown = layers.trade && !close;
    stats.visible = shown;
    if let Ok(mut visibility) = layer.single_mut() {
        let wanted = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visibility.set_if_neq(wanted);
    }
}

/// Worksite hammers follow the `construction` rule of the `signs` block.
fn show_signs(
    layers: Res<MapLayers>,
    rules: Res<SignRules>,
    mode: Option<Res<MapModeState>>,
    selected: Option<Res<SelectedArmy>>,
    mut worksites: Query<&mut Visibility, With<WorksiteSign>>,
) {
    let key = mode.map_or("political", |m| m.mode.key());
    let army = selected.is_some_and(|s| s.0.is_some());
    let wanted = if rules.shown(layers.signs, "construction", key, false, army) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut worksites {
        visibility.set_if_neq(wanted);
    }
}

fn dump_layers(world: &World) -> (String, Value) {
    let layers = world.resource::<MapLayers>();
    let stats = world.resource::<TradeStats>();
    let rules = world.resource::<SignRules>();
    let mode = world
        .get_resource::<MapModeState>()
        .map_or("political", |m| m.mode.key());
    (
        "map_layers".to_owned(),
        json!({
            "trade_on": layers.trade,
            "signs_on": layers.signs,
            "trade_visible": stats.visible,
            "routes_total": stats.routes_total,
            "routes_drawn": stats.routes_drawn,
            "routes_cut": stats.routes_cut,
            "rebuilds": stats.rebuilds,
            "sign_rules": rules.kinds.keys().collect::<Vec<_>>(),
            "construction_shown": rules.shown(layers.signs, "construction", mode, false, false),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> SignRules {
        SignRules::from_json(&json!({
            "max_per_town": 1,
            "incident": {"modes": ["unrest"], "keep_urgent": true},
            "encounter": {"modes": [], "keep_urgent": true, "with_selected_army": true},
            "construction": {"modes": ["wealth", "population"]}
        }))
    }

    #[test]
    fn construction_sign_follows_mode_and_layer() {
        let r = rules();
        assert!(!r.shown(false, "construction", "political", false, false));
        assert!(r.shown(false, "construction", "wealth", false, false));
        assert!(r.shown(true, "construction", "political", false, false));
        assert!(r.shown(false, "unknown", "political", false, false));
    }

    #[test]
    fn urgent_and_selected_army_keep_signs() {
        let r = rules();
        assert!(r.shown(false, "incident", "political", true, false));
        assert!(!r.shown(false, "incident", "political", false, false));
        assert!(r.shown(false, "encounter", "political", false, true));
        assert!(!r.shown(false, "construction", "political", true, true));
    }

    #[test]
    fn route_width_is_clamped() {
        assert_eq!(route_width(0), MIN_WIDTH);
        assert!((route_width(80) - 3.2).abs() < 1e-4);
        assert_eq!(route_width(100_000), MAX_WIDTH);
    }

    #[test]
    fn ribbon_has_two_vertices_per_sample_and_follows_the_ground() {
        let route = RibbonRoute {
            id: "r".into(),
            points: vec![Vec2::new(0.0, 0.0), Vec2::new(12.0, 0.0)],
            width: 2.0,
            cut: false,
            tooltip: String::new(),
        };
        let mesh = ribbon_mesh(&[&route], &|x, _| x);
        // 12 units at STEP 6: three samples, six vertices, two quads.
        assert_eq!(mesh.count_vertices(), 6);
        assert_eq!(mesh.indices().map(|i| i.len()), Some(12));
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        assert!((p[4][1] - (12.0 + LIFT)).abs() < 1e-4);
        assert!((p[0][2].abs() - 1.0).abs() < 1e-4, "offset by half width");
    }

    #[test]
    fn route_tooltip_reads_like_godot() {
        let goods = vec!["drap".to_owned(), "laine".to_owned()];
        assert_eq!(
            route_tooltip("Bruges", "Londres", None, 24, &goods, true, 0.5, false),
            "Bruges ↔ Londres — 24 livres/saison (drap, laine) — accord commercial — menacée"
        );
        assert_eq!(
            route_tooltip("A", "B", Some("guerre"), 0, &[], false, 0.0, true),
            "A ↔ B : route coupée (guerre)"
        );
    }

    #[test]
    fn nearest_route_respects_the_radius() {
        let route = RibbonRoute {
            id: "r".into(),
            points: vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)],
            width: 2.0,
            cut: false,
            tooltip: "t".into(),
        };
        let routes = [route];
        assert!(nearest_route(&routes, Vec2::new(50.0, 2.0), 3.0).is_some());
        assert!(nearest_route(&routes, Vec2::new(50.0, 9.0), 3.0).is_none());
    }
}
