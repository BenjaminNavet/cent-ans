//! Folk of the roads (`life_folk/folk_pool.gd`, `folk_routine.gd`, `folk_caravans.gd`, ADR 0354):
//! villagers walking the road pieces, carts and convoys with their draught team, carter, merchant
//! and guards. Fixed world size (`folk_scale / meters_per_px`), anchored sowing (every unit has a
//! fixed random number `h` compared with the continuous thinning threshold), fades instead of
//! pops, a hard cap `pool_cap`.
//!
//! Differences with Godot (see `docs/wip/bm-life.md`): the figures are static meshes of the battle
//! figures (`figures.rs`, no animation), carts follow the road lines instead of the trade routes of
//! the bridge. The prop table (`PROPS`) lives in `data/rules/folk_props.json` (`props.rs`).

use bevy::prelude::*;

use super::data::{LifeData, RoadRun, h01, text_seed};
use super::figures::Figures;
use super::models::{Fade, FitAxis, ModelMetrics, spawn_model};
use super::props::PropGeometry;
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
    /// Roll (rad) around the heading axis (`motion.rs`).
    pub roll: f32,
    /// DN id of the cart model and of its team, keys of the motion table.
    pub model: String,
    pub team_model: Option<String>,
}

/// Draught animal child (counted).
#[derive(Component)]
pub struct Beast;

/// World scale of a figure measured in metres.
pub fn world_scale(data: &LifeData) -> f32 {
    data.props.world_scale(data.meters_per_px)
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

/// Role of a walker of the roads from a fixed random number (mostly villagers, a few porters).
fn walker_role(h: f32) -> &'static str {
    if h < 0.55 {
        "peasant"
    } else if h < 0.9 {
        "peasant_b"
    } else {
        "porter"
    }
}

pub fn spawn_walkers(
    mut commands: Commands,
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut figures: Figures,
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
                    let figure = figures.spawn(
                        &mut commands,
                        walker_role(h01(i as u64, k as u64, 12)),
                        2,
                        h01(i as u64, k as u64, 10),
                        h01(i as u64, k as u64, 11),
                        ws,
                        Vec3::ZERO,
                        "walk",
                    );
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
    skin: Res<super::skin::FolkSkin>,
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
        // (Only without the walking clip of the skinned figure, `skin.rs`.)
        let bounce = if skin.active {
            0.0
        } else {
            (view.time * 6.0 + w.phase * 20.0).sin().abs() * 0.04 * ws
        };
        t.translation = Vec3::new(p.x, y + bounce, p.y);
        t.rotation = Quat::from_rotation_y(-dir.y.atan2(dir.x));
    }
}

/// Vertex animation of a prop model: only the DN glb are animated (`AnimalMotion.apply_prop`
/// is keyed by the DN id); `yaw` is the heading fix of `prop_model` (the `yaw_deg` of the table
/// plus a quarter turn).
pub fn prop_bind(
    path: &str,
    yaw: f32,
    model: &str,
    speed_m_s: f32,
    seed: f32,
) -> Option<super::anim::AnimBind> {
    path.starts_with("models/dn/")
        .then(|| super::anim::AnimBind {
            kind: super::anim::AnimKind::Prop {
                model: model.to_string(),
                speed_m_s,
            },
            seed,
            yaw: yaw - std::f32::consts::FRAC_PI_2,
        })
}

fn cart_prop_id(main: bool, convoy: bool) -> &'static str {
    if main || convoy {
        "merchant_cart"
    } else {
        "peasant_cart"
    }
}

/// Carts and convoys of the stretches of road around the focus.
#[allow(clippy::too_many_arguments)]
pub fn spawn_carts(
    mut commands: Commands,
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut figures: Figures,
    geometry: Res<PropGeometry>,
    root: Option<Res<super::fauna::AssetRoot>>,
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
                    let prop_id = cart_prop_id(run.main, convoy);
                    let Some(prop) = data.table.props.get(prop_id) else {
                        continue;
                    };
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
                            &data,
                            &mut figures,
                            &geometry,
                            root.as_deref().map(|r| r.0.as_path()),
                            ws,
                            prop_id,
                            key,
                            h,
                            Cart {
                                run: r,
                                lo: lo_t,
                                hi: hi_t,
                                phase,
                                back: gap_px * m as f32,
                                lift: 0.0,
                                roll: 0.0,
                                model: prop.dn.clone(),
                                team_model: prop
                                    .team
                                    .as_ref()
                                    .and_then(|t| data.table.props.get(&t.role))
                                    .map(|a| a.dn.clone()),
                            },
                            mid,
                            m == 0 && !convoy && prop_id == "merchant_cart" && h01(uk, 4, 44) < 0.5,
                        );
                    }
                }
            }
        }
    }
}

/// Model path of a prop at the distant LOD (DN glb, else the FK2 file) and its yaw (rad) around
/// the heading axis +X of this module.
pub fn prop_model(
    data: &LifeData,
    root: Option<&std::path::Path>,
    id: &str,
    lod: u8,
) -> Option<(String, f32)> {
    let exists = |p: &str| root.is_none_or(|r| r.join(p).is_file());
    let model = data.table.model_of(id, lod, &exists)?;
    Some((model.path, (model.yaw_deg + 90.0).to_radians()))
}

#[allow(clippy::too_many_arguments)]
fn spawn_cart(
    commands: &mut Commands,
    asset_server: &AssetServer,
    metrics: &ModelMetrics,
    data: &LifeData,
    figures: &mut Figures,
    geometry: &PropGeometry,
    root: Option<&std::path::Path>,
    ws: f32,
    prop_id: &str,
    key: u64,
    h: f32,
    cart: Cart,
    at: Vec2,
    with_guard: bool,
) {
    let Some(prop) = data.table.props.get(prop_id) else {
        return;
    };
    let cart_world = prop.length_m * ws;
    let cart_model = cart.model.clone();
    let team_dn = cart.team_model.clone().unwrap_or_default();
    let mut n_beast = 0u64;
    let speed_m_s = WALK_SPEED_M_S * data.props.folk_speed;
    let root_entity = commands
        .spawn((
            Name::new(format!("life:{prop_id}")),
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
    if let Some((path, yaw)) = prop_model(data, root, prop_id, 2) {
        let model = spawn_model(
            commands,
            asset_server,
            metrics,
            &path,
            FitAxis::Length,
            cart_world,
            yaw,
        );
        if let Some(bind) = prop_bind(&path, yaw, &cart_model, speed_m_s, h01(key, 7, 71)) {
            commands.entity(model).try_insert(bind);
        }
        commands.entity(root_entity).add_child(model);
    }
    // Draught team ahead, side by side (`FolkModels.team`): measured positions, else the formula
    // of the table.
    if let Some(team) = &prop.team {
        let beasts: Vec<(f32, f32)> = match geometry.teams.get(prop_id) {
            Some(list) => list.iter().map(|b| (b.ahead, b.lateral)).collect(),
            None => {
                let animal_m = data.table.props.get(&team.role).map_or(2.2, |a| a.length_m);
                let front = cart_world * 0.5 / ws + animal_m * 0.5 + team.gap_m;
                (0..team.count)
                    .map(|n| {
                        (
                            front,
                            (n as f32 - (team.count as f32 - 1.0) * 0.5) * animal_m * 0.6,
                        )
                    })
                    .collect()
            }
        };
        let animal_m = data.table.props.get(&team.role).map_or(2.2, |a| a.length_m);
        if let Some((animal_path, animal_yaw)) = prop_model(data, root, &team.role, 2) {
            for (ahead, lateral) in beasts {
                let beast = spawn_model(
                    commands,
                    asset_server,
                    metrics,
                    &animal_path,
                    FitAxis::Length,
                    animal_m * ws,
                    animal_yaw,
                );
                if let Some(bind) = prop_bind(
                    &animal_path,
                    animal_yaw,
                    &team_dn,
                    speed_m_s,
                    h01(key, 8 + n_beast, 72),
                ) {
                    commands.entity(beast).try_insert(bind);
                }
                n_beast += 1;
                let holder = commands
                    .spawn((
                        Name::new("life:team"),
                        Transform::from_xyz(ahead * ws, 0.0, lateral * ws),
                        Visibility::Inherited,
                        Beast,
                    ))
                    .id();
                commands.entity(holder).add_child(beast);
                commands.entity(root_entity).add_child(holder);
            }
        }
    }
    // Carter on the right of the box, merchant behind him, guard at the tail (slots of the glb).
    let slot_at = |name: &str, fallback: Vec2| -> Vec3 {
        let s = geometry.slot(prop_id, name, fallback);
        Vec3::new(s.y * ws, 0.0, s.x * ws)
    };
    let (hc, hc2) = (h01(key, 1, 61), h01(key, 2, 62));
    let carter = figures.spawn(
        commands,
        "peasant",
        2,
        hc,
        hc2,
        ws,
        slot_at("carter", Vec2::new(0.45, 0.4 * prop.length_m * 0.5)),
        "walk",
    );
    commands.entity(root_entity).add_child(carter);
    if prop.slots.contains_key("merchant") {
        let merchant = figures.spawn(
            commands,
            "merchant",
            2,
            h01(key, 3, 63),
            h01(key, 4, 64),
            ws,
            slot_at("merchant", Vec2::new(0.9, -0.2 * prop.length_m)),
            "walk",
        );
        commands.entity(root_entity).add_child(merchant);
    }
    if with_guard {
        let guard = figures.spawn(
            commands,
            "guard",
            2,
            h01(key, 5, 65),
            h01(key, 6, 66),
            ws,
            slot_at("guard", Vec2::new(0.0, -0.6 * prop.length_m)),
            "guard_walk",
        );
        commands.entity(root_entity).add_child(guard);
    }
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
        t.rotation = Quat::from_rotation_y(-dir.y.atan2(dir.x)) * Quat::from_rotation_x(c.roll);
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
