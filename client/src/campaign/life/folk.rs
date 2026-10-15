//! Folk of the roads (`life_folk/folk_pool.gd`, `folk_routine.gd`, `folk_caravans.gd`, ADR 0354):
//! villagers walking the road pieces, carts and convoys with their draught team, carter, merchant
//! and guards. Fixed world size (`folk_scale / meters_per_px`), anchored sowing (every unit has a
//! fixed random number `h` compared with the continuous thinning threshold), fades instead of
//! pops, a hard cap `pool_cap`.
//!
//! Differences with Godot (see `docs/wip/bm-life.md`): the figures are capsule stand-ins with the
//! role livery (the skinned battle figures belong to the battle lot), carts follow the road lines
//! instead of the trade routes of the bridge, the prop table (`PROPS`) is mirrored here.

use bevy::prelude::*;

use super::data::{LifeData, RoadRun, h01, text_seed};
use super::models::{Fade, FitAxis, ModelMetrics, spawn_model};
use super::stream::{Cat, LifeIndex, LifeItem, LifeView};
use crate::app::AppState;
use crate::campaign::terrain::surface::GroundHeight;

/// Walking speed of a figure (m/s, `FolkModels.ACTIVITIES.walk`).
pub const WALK_SPEED_M_S: f32 = 1.2;
/// Length of one stretch of road given to a cart or a convoy (px).
pub const CART_SLOT: f32 = 40.0;
/// Reference trade value for the density of carts of a road (`carts_per_trade_value` x value).
pub const REFERENCE_TRADE_VALUE: f32 = 40.0;
/// Share of the units of the main roads that are convoys, number of carts of a convoy.
pub const CONVOY_SHARE: f32 = 0.18;
pub const CONVOY_CARTS: usize = 3;
/// Share of the minor-road stretches with a peasant cart.
pub const PEASANT_CART_SHARE: f32 = 0.12;

/// Mirror of `FolkModels.PROPS` for the carts: DN glb (`dn/props/<id>_lod2.glb`), the yaw that
/// brings its front to +Z, the draught team (`role`, `count`, `gap_m`) and its length (m).
pub struct CartProp {
    pub id: &'static str,
    pub dn: &'static str,
    pub yaw_deg: f32,
    pub length_m: f32,
    pub team_role: &'static str,
    pub team_count: usize,
    pub team_gap_m: f32,
    pub guard: bool,
}

pub const MERCHANT_CART: CartProp = CartProp {
    id: "merchant_cart",
    dn: "cart_merchant",
    yaw_deg: 90.0,
    length_m: 3.5,
    team_role: "horse",
    team_count: 2,
    team_gap_m: -0.6,
    guard: true,
};
pub const PEASANT_CART: CartProp = CartProp {
    id: "peasant_cart",
    dn: "env_cart_ox_peasant",
    yaw_deg: 90.0,
    length_m: 3.5,
    team_role: "ox",
    team_count: 1,
    team_gap_m: -0.6,
    guard: false,
};

/// Animal of a team: DN fauna id and length (m).
pub fn team_animal(role: &str) -> (&'static str, f32) {
    match role {
        "horse" => ("animal_horse_draught", 2.2),
        _ => ("animal_ox_draught", 2.2),
    }
}

#[derive(Component)]
pub struct Walker {
    pub piece: usize,
    pub phase: f32,
}

#[derive(Component)]
pub struct Cart {
    pub run: usize,
    pub lo: f32,
    pub hi: f32,
    pub phase: f32,
    /// Distance behind the lead of the train (px).
    pub back: f32,
    pub lift: f32,
}

/// Draught animal child (counted).
#[derive(Component)]
pub struct Beast;

#[derive(Resource)]
pub struct FolkAssets {
    pub body: Handle<Mesh>,
    pub head: Handle<Mesh>,
    pub liveries: Vec<Handle<StandardMaterial>>,
    pub merchant: Handle<StandardMaterial>,
    pub guard: Handle<StandardMaterial>,
}

pub fn build_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut mat = |c: [f32; 3]| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(c[0], c[1], c[2]),
            perceptual_roughness: 0.95,
            ..default()
        })
    };
    // Drab wool colours (`DRAB`) and the liveries of the merchant and the guard.
    let liveries = [
        [0.55, 0.5, 0.4],
        [0.4, 0.3, 0.2],
        [0.5, 0.32, 0.2],
        [0.42, 0.42, 0.4],
    ]
    .into_iter()
    .map(&mut mat)
    .collect();
    let merchant = mat([0.35, 0.18, 0.12]);
    let guard = mat([0.5, 0.12, 0.1]);
    commands.insert_resource(FolkAssets {
        body: meshes.add(Capsule3d::new(0.2, 1.1)),
        head: meshes.add(
            Sphere::new(0.14)
                .mesh()
                .ico(1)
                .unwrap_or_else(|_| Sphere::new(0.14).mesh().into()),
        ),
        liveries,
        merchant,
        guard,
    });
}

/// World scale of a figure measured in metres.
pub fn world_scale(data: &LifeData) -> f32 {
    data.props.world_scale(data.meters_per_px)
}

/// Spawns a figure child at the local position `at` (map units) of `parent`.
fn spawn_figure(
    commands: &mut Commands,
    assets: &FolkAssets,
    ws: f32,
    material: Handle<StandardMaterial>,
    at: Vec3,
) -> Entity {
    commands
        .spawn((
            Name::new("life:figure"),
            Transform::from_translation(at).with_scale(Vec3::splat(ws)),
            Visibility::Inherited,
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(assets.body.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(0.0, 0.85, 0.0),
            ));
            p.spawn((
                Mesh3d(assets.head.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, 1.62, 0.0),
            ));
        })
        .id()
}

/// Number of figures of a road piece: anchored stochastic rounding of the expected count.
pub fn piece_count(len: f32, per_unit: f32, density: f32, piece: usize) -> usize {
    let expected = len * per_unit * density;
    (expected + h01(piece as u64, 0, 77)) as usize
}

fn pingpong(x: f32) -> f32 {
    let t = x.rem_euclid(2.0);
    if t <= 1.0 { t } else { 2.0 - t }
}

const PIECE_CELL: f32 = 32.0;

/// Spatial index of the road pieces and of the cart stretches (built once with the data).
#[derive(Resource, Default)]
pub struct FolkIndex {
    pub pieces: std::collections::HashMap<IVec2, Vec<usize>>,
    /// (run, slot) by cell of the middle of the stretch.
    pub slots: std::collections::HashMap<IVec2, Vec<(usize, usize)>>,
    pub built: bool,
}

fn cell_of(p: Vec2) -> IVec2 {
    (p / PIECE_CELL).floor().as_ivec2()
}

pub fn build_index(data: Res<LifeData>, mut index: ResMut<FolkIndex>) {
    if index.built {
        return;
    }
    index.built = true;
    for (i, piece) in data.roads.iter().enumerate() {
        index
            .pieces
            .entry(cell_of((piece.a + piece.b) * 0.5))
            .or_default()
            .push(i);
    }
    for (r, run) in data.road_runs.iter().enumerate() {
        let slots = (run.length() / CART_SLOT).ceil() as usize;
        for j in 0..slots {
            let mid = run.at((j as f32 + 0.5) * CART_SLOT).0;
            index.slots.entry(cell_of(mid)).or_default().push((r, j));
        }
    }
}

pub fn spawn_walkers(
    mut commands: Commands,
    data: Res<LifeData>,
    view: Res<LifeView>,
    assets: Option<Res<FolkAssets>>,
    spatial: Res<FolkIndex>,
    mut index: ResMut<LifeIndex>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 || !view.ready {
        return;
    }
    *timer = 0.2;
    let Some(assets) = assets else { return };
    if view.range_target(data.props.folk_range, data.props.folk_range_fade) <= 0.0 {
        return;
    }
    let ceiling = view.ceiling();
    let radius = view.radius * 0.98;
    let reach = (radius / PIECE_CELL).ceil() as i32 + 1;
    let centre = cell_of(view.focus);
    let ws = world_scale(&data);
    let speed_px = WALK_SPEED_M_S * ws * data.props.folk_speed;
    let mut spawned = 0;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let Some(list) = spatial.pieces.get(&(centre + IVec2::new(dx, dy))) else {
                continue;
            };
            for &i in list {
                let piece = data.roads[i];
                let mid = (piece.a + piece.b) * 0.5;
                if mid.distance(view.focus) > radius {
                    continue;
                }
                let len = piece.a.distance(piece.b);
                let n = piece_count(
                    len,
                    data.scenes.road_folk_per_unit,
                    data.props.folk_density,
                    i,
                );
                for k in 0..n {
                    let key = text_seed("life:walker") ^ ((i as u64) << 8) ^ k as u64;
                    if index.0.contains(&key) {
                        continue;
                    }
                    let h = h01(i as u64, k as u64, 5);
                    if h >= ceiling
                        || view.thin_target(h) <= 0.02
                        || view.edge_target(mid, data.props.folk_edge_inner) <= 0.02
                    {
                        continue;
                    }
                    if index.0.len() >= data.scenes.pool_cap || spawned >= 80 {
                        return;
                    }
                    index.0.insert(key);
                    spawned += 1;
                    let phase = h01(i as u64, k as u64, 9);
                    let live = &assets.liveries[(h01(i as u64, k as u64, 11)
                        * assets.liveries.len() as f32)
                        as usize
                        % assets.liveries.len()];
                    let figure = spawn_figure(&mut commands, &assets, ws, live.clone(), Vec3::ZERO);
                    let p = piece.a.lerp(piece.b, pingpong(phase * 2.0));
                    let root = commands
                        .spawn((
                            Name::new("life:walker"),
                            Transform::from_xyz(p.x, 0.0, p.y).with_scale(Vec3::splat(1e-4)),
                            Visibility::Inherited,
                            Walker { piece: i, phase },
                            LifeItem {
                                key,
                                cat: Cat::Walker,
                                anchor: p,
                                h,
                            },
                            Fade::new(1.0),
                            DespawnOnExit(AppState::Campaign),
                        ))
                        .id();
                    commands.entity(root).add_child(figure);
                    let _ = speed_px;
                }
            }
        }
    }
}

pub fn move_walkers(
    data: Res<LifeData>,
    view: Res<LifeView>,
    ground: GroundHeight,
    mut walkers: Query<(&Walker, &mut LifeItem, &mut Transform)>,
) {
    let ws = world_scale(&data);
    let speed_px = WALK_SPEED_M_S * ws * data.props.folk_speed;
    for (w, mut item, mut t) in &mut walkers {
        let piece = data.roads[w.piece];
        let len = piece.a.distance(piece.b).max(1e-3);
        let x = w.phase * 2.0 + view.time * speed_px / len;
        let u = pingpong(x);
        let forward = x.rem_euclid(2.0) <= 1.0;
        let p = piece.a.lerp(piece.b, u);
        item.anchor = p;
        let dir = (piece.b - piece.a).normalize_or_zero() * if forward { 1.0 } else { -1.0 };
        let y = ground.at(p.x, p.y).unwrap_or(0.0);
        // Step bounce (a few centimetres of a man).
        let bounce = (view.time * 6.0 + w.phase * 20.0).sin().abs() * 0.04 * ws;
        t.translation = Vec3::new(p.x, y + bounce, p.y);
        t.rotation = Quat::from_rotation_y(-dir.y.atan2(dir.x));
    }
}

fn cart_prop(main: bool, convoy: bool) -> &'static CartProp {
    if main || convoy {
        &MERCHANT_CART
    } else {
        &PEASANT_CART
    }
}

/// Carts and convoys of the stretches of road around the focus.
#[allow(clippy::too_many_arguments)]
pub fn spawn_carts(
    mut commands: Commands,
    data: Res<LifeData>,
    view: Res<LifeView>,
    assets: Option<Res<FolkAssets>>,
    asset_server: Res<AssetServer>,
    metrics: Res<ModelMetrics>,
    spatial: Res<FolkIndex>,
    mut index: ResMut<LifeIndex>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 || !view.ready {
        return;
    }
    *timer = 0.25;
    let Some(assets) = assets else { return };
    if view.range_target(data.props.folk_range, data.props.folk_range_fade) <= 0.0 {
        return;
    }
    let ceiling = view.ceiling();
    let radius = view.radius * 0.98;
    let reach = (radius / PIECE_CELL).ceil() as i32 + 1;
    let centre = cell_of(view.focus);
    let ws = world_scale(&data);
    let mut spawned = 0;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let Some(list) = spatial.slots.get(&(centre + IVec2::new(dx, dy))) else {
                continue;
            };
            for &(r, j) in list {
                let run: &RoadRun = &data.road_runs[r];
                let lo = j as f32 * CART_SLOT;
                let hi = ((j + 1) as f32 * CART_SLOT).min(run.length());
                let mid = run.at((lo + hi) * 0.5).0;
                if mid.distance(view.focus) > radius {
                    continue;
                }
                // Units of this stretch: anchored stochastic rounding of the density.
                let rate_per_10 = data.scenes.carts_per_trade_value * REFERENCE_TRADE_VALUE;
                let expected = if run.main {
                    rate_per_10 * (hi - lo) / 10.0
                } else {
                    PEASANT_CART_SHARE
                } * data.props.folk_density;
                let units = (expected + h01(r as u64, j as u64, 31)) as usize;
                for u in 0..units {
                    let uk = ((r as u64) << 24) ^ ((j as u64) << 8) ^ u as u64;
                    let h = h01(uk, 1, 41);
                    let convoy = run.main && h01(uk, 2, 42) < CONVOY_SHARE && hi - lo > 18.0;
                    let carts = if convoy { CONVOY_CARTS } else { 1 };
                    if h >= ceiling
                        || view.thin_target(h) <= 0.02
                        || view.edge_target(mid, data.props.folk_edge_inner) <= 0.02
                    {
                        continue;
                    }
                    let prop = cart_prop(run.main, convoy);
                    let cart_world = prop.length_m * ws;
                    let gap_px = cart_world * 1.7;
                    let train = gap_px * (carts as f32 - 1.0);
                    let (lo_t, hi_t) = (lo + train, hi.max(lo + train + 1.0));
                    let phase = h01(uk, 3, 43);
                    for m in 0..carts {
                        let key = text_seed("life:cart") ^ (uk << 3) ^ m as u64;
                        if index.0.contains(&key) {
                            continue;
                        }
                        if index.0.len() >= data.scenes.pool_cap || spawned >= 12 {
                            return;
                        }
                        index.0.insert(key);
                        spawned += 1;
                        spawn_cart(
                            &mut commands,
                            &asset_server,
                            &metrics,
                            &assets,
                            &data,
                            ws,
                            prop,
                            key,
                            h,
                            Cart {
                                run: r,
                                lo: lo_t,
                                hi: hi_t,
                                phase,
                                back: gap_px * m as f32,
                                lift: 0.0,
                            },
                            mid,
                            m == 0 && !convoy && prop.guard && h01(uk, 4, 44) < 0.5,
                        );
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_cart(
    commands: &mut Commands,
    asset_server: &AssetServer,
    metrics: &ModelMetrics,
    assets: &FolkAssets,
    data: &LifeData,
    ws: f32,
    prop: &CartProp,
    key: u64,
    h: f32,
    cart: Cart,
    at: Vec2,
    with_guard: bool,
) {
    let cart_world = prop.length_m * ws;
    // Front of the glb on +Z after `yaw_deg`; the heading axis here is +X.
    let yaw = (prop.yaw_deg + 90.0).to_radians();
    let root = commands
        .spawn((
            Name::new(format!("life:{}", prop.id)),
            Transform::from_xyz(at.x, 0.0, at.y).with_scale(Vec3::splat(1e-4)),
            Visibility::Inherited,
            cart,
            LifeItem {
                key,
                cat: Cat::Cart,
                anchor: at,
                h,
            },
            Fade::new(1.0),
            DespawnOnExit(AppState::Campaign),
        ))
        .id();
    let model = spawn_model(
        commands,
        asset_server,
        metrics,
        &format!("models/dn/props/{}_lod2.glb", prop.dn),
        FitAxis::Length,
        cart_world,
        yaw,
    );
    commands.entity(root).add_child(model);
    // Draught team ahead, side by side (`FolkModels.team`).
    let (animal, animal_m) = team_animal(prop.team_role);
    let animal_world = animal_m * ws;
    let front = cart_world * 0.5 + animal_world * 0.5 + prop.team_gap_m * ws;
    for n in 0..prop.team_count {
        let lateral = (n as f32 - (prop.team_count as f32 - 1.0) * 0.5) * animal_world * 0.6;
        let beast = spawn_model(
            commands,
            asset_server,
            metrics,
            &format!("models/dn/fauna/{animal}_lod2.glb"),
            FitAxis::Length,
            animal_world,
            std::f32::consts::FRAC_PI_2,
        );
        let holder = commands
            .spawn((
                Name::new("life:team"),
                Transform::from_xyz(front, 0.0, lateral),
                Visibility::Inherited,
                Beast,
            ))
            .id();
        commands.entity(holder).add_child(beast);
        commands.entity(root).add_child(holder);
    }
    // Carter on the right of the box, merchant behind him, guard at the tail.
    let carter = spawn_figure(
        commands,
        assets,
        ws,
        assets.liveries[1].clone(),
        Vec3::new(cart_world * 0.12, 0.0, cart_world * 0.32),
    );
    commands.entity(root).add_child(carter);
    if prop.guard {
        let merchant = spawn_figure(
            commands,
            assets,
            ws,
            assets.merchant.clone(),
            Vec3::new(-cart_world * 0.2, 0.0, cart_world * 0.32),
        );
        commands.entity(root).add_child(merchant);
    }
    if with_guard {
        let guard = spawn_figure(
            commands,
            assets,
            ws,
            assets.guard.clone(),
            Vec3::new(-cart_world * 0.6, 0.0, 0.0),
        );
        commands.entity(root).add_child(guard);
    }
    let _ = data;
}

pub fn move_carts(
    data: Res<LifeData>,
    view: Res<LifeView>,
    ground: GroundHeight,
    mut carts: Query<(&Cart, &mut LifeItem, &mut Transform)>,
) {
    let ws = world_scale(&data);
    let speed_px = WALK_SPEED_M_S * ws * data.props.folk_speed;
    for (c, mut item, mut t) in &mut carts {
        let run = &data.road_runs[c.run];
        let span = (c.hi - c.lo).max(1.0);
        let x = c.phase * 2.0 + view.time * speed_px / span;
        let forward = x.rem_euclid(2.0) <= 1.0;
        let lead = c.lo + pingpong(x) * span;
        let s = if forward {
            lead - c.back
        } else {
            lead + c.back
        };
        let (p, dir) = run.at(s);
        let dir = if forward { dir } else { -dir };
        item.anchor = p;
        let y = ground.at(p.x, p.y).unwrap_or(0.0);
        t.translation = Vec3::new(p.x, y + c.lift, p.y);
        t.rotation = Quat::from_rotation_y(-dir.y.atan2(dir.x));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::life::data::ROAD_PIECE;

    #[test]
    fn piece_count_follows_density() {
        let total: usize = (0..2000)
            .map(|i| piece_count(ROAD_PIECE, 0.05, 1.0, i))
            .sum();
        let expected = 2000.0 * ROAD_PIECE * 0.05;
        assert!(
            (total as f32 - expected).abs() < expected * 0.15,
            "{total} vs {expected}"
        );
        let doubled: usize = (0..2000)
            .map(|i| piece_count(ROAD_PIECE, 0.05, 2.0, i))
            .sum();
        assert!(doubled > total);
    }

    #[test]
    fn pingpong_bounces() {
        assert!((pingpong(0.25) - 0.25).abs() < 1e-6);
        assert!((pingpong(1.25) - 0.75).abs() < 1e-6);
        assert!((pingpong(2.25) - 0.25).abs() < 1e-6);
    }
}
