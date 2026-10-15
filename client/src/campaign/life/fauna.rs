//! Herds and wild fauna (`fauna_layer.gd`, zones and species in `data/map/map_fauna.json`).
//! Herds are sown by map cell (`render.cell_px`) from the zone densities (herds per 1000 km2,
//! season weights), kept on land, outside lakes, away from towns, below the slope and inside the
//! altitude range of the species. Animals are glb scenes (the generated DN fauna glb when present,
//! else the FK2 placeholder), fixed world size like the folk (ADR 0354), wandering slowly with the
//! gait data (`render.gaits`).

use std::path::PathBuf;

use bevy::prelude::*;

use super::data::{FaunaSpecies, LifeData, h01, text_seed};
use super::folk::world_scale;
use super::models::{Fade, FitAxis, ModelMetrics, spawn_model};
use super::stream::{Cat, LifeIndex, LifeItem, LifeView};
use crate::app::AppState;
use crate::campaign::terrain::surface::{GroundHeight, Surface};

#[derive(Component)]
pub struct Animal {
    pub gait: String,
    pub herd: Vec2,
    pub offset: Vec2,
    pub phase: f32,
    pub length_world: f32,
    pub wander_px: f32,
    pub speed_px: f32,
    /// Species id (the model is looked up again at a LOD change).
    pub species: String,
    /// LOD of the model on show (0 fine, 2 distant).
    pub lod: u8,
}

/// LOD of a camera distance with hysteresis (`CellFade.lod_with_hysteresis`).
pub fn lod_with_hysteresis(
    distance: f32,
    thresholds: &[f32],
    current: usize,
    hysteresis: f32,
) -> usize {
    let mut level = current.min(thresholds.len());
    while level < thresholds.len() && distance >= thresholds[level] * (1.0 + hysteresis) {
        level += 1;
    }
    while level > 0 && distance < thresholds[level - 1] * (1.0 - hysteresis) {
        level -= 1;
    }
    level
}

/// Root of the assets (for the existence test of the generated glb).
#[derive(Resource)]
pub struct AssetRoot(pub PathBuf);

/// A herd found in a cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Herd {
    pub species: String,
    pub centre: Vec2,
    pub size: usize,
    pub id: u64,
}

fn in_ellipse(p: Vec2, centre: Vec2, radii: Vec2) -> bool {
    let d = (p - centre) / radii.max(Vec2::splat(1e-3));
    d.length_squared() <= 1.0
}

/// Herds of a cell (pure but for the placement tests: `valid(species, point)`).
pub fn herds_of_cell(
    data: &LifeData,
    cell: IVec2,
    season: usize,
    valid: &impl Fn(&FaunaSpecies, Vec2) -> bool,
) -> Vec<Herd> {
    let cfg = &data.fauna;
    let cell_px = cfg.cell_px.max(1.0);
    let origin = cell.as_vec2() * cell_px;
    let centre = origin + Vec2::splat(cell_px * 0.5);
    let km_per_px = data.meters_per_px / 1000.0;
    let area_km2 = (cell_px * km_per_px).powi(2);
    let cell_key = (cell.x as i64 as u64).wrapping_mul(73_856_093)
        ^ (cell.y as i64 as u64).wrapping_mul(19_349_663);
    let mut out = Vec::new();
    for (z, zone) in cfg.zones.iter().enumerate() {
        if !in_ellipse(
            centre,
            zone.center_px,
            zone.radius_px + Vec2::splat(cell_px * 0.5),
        ) {
            continue;
        }
        for (e, entry) in zone.species.iter().enumerate() {
            let Some(sp) = cfg.species.get(&entry.species) else {
                continue;
            };
            let weight = entry.seasons[season.min(3)];
            let expected = entry.per_1000km2 * area_km2 / 1000.0 * weight;
            let seed =
                text_seed(&entry.species) ^ cell_key ^ ((z as u64) << 56) ^ ((e as u64) << 48);
            let herds = (expected + h01(seed, 0, 1)) as usize;
            for j in 0..herds {
                let p =
                    origin + Vec2::new(h01(seed, j as u64, 2), h01(seed, j as u64, 3)) * cell_px;
                if !in_ellipse(p, zone.center_px, zone.radius_px) || !valid(sp, p) {
                    continue;
                }
                let lo = sp.herd_size[0];
                let hi = sp.herd_size[1].max(lo);
                let size = lo + (h01(seed, j as u64, 4) * (hi - lo + 1) as f32) as usize;
                out.push(Herd {
                    species: entry.species.clone(),
                    centre: p,
                    size: size.min(hi),
                    id: seed ^ ((j as u64) << 20),
                });
            }
        }
    }
    out
}

fn model_path(root: &std::path::Path, sp: &FaunaSpecies, lod: u8) -> Option<String> {
    let dn = format!("models/dn/fauna/{}_lod{lod}.glb", sp.id);
    if root.join(&dn).exists() {
        return Some(dn);
    }
    if sp.placeholder.is_empty() {
        return None;
    }
    let placeholder = format!("models/{}", sp.placeholder);
    root.join(&placeholder).exists().then_some(placeholder)
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_herds(
    mut commands: Commands,
    data: Res<LifeData>,
    view: Res<LifeView>,
    asset_server: Res<AssetServer>,
    metrics: Res<ModelMetrics>,
    root: Option<Res<AssetRoot>>,
    surface: Option<Res<Surface>>,
    places: Res<crate::campaign::markers::places::Places>,
    mut index: ResMut<LifeIndex>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 || !view.ready {
        return;
    }
    *timer = 0.3;
    let (Some(root), Some(surface)) = (root, surface) else {
        return;
    };
    let cfg = &data.fauna;
    if view.distance >= cfg.max_distance || data.fauna.zones.is_empty() {
        return;
    }
    let ws = world_scale(&data);
    let cell_px = cfg.cell_px.max(1.0);
    let radius = view.radius * 0.98;
    let reach = (radius / cell_px).ceil() as i32 + 1;
    let centre_cell = (view.focus / cell_px).floor().as_ivec2();
    let clearance = cfg.town_clearance_px;
    let valid =
        |sp: &FaunaSpecies, p: Vec2| -> bool {
            if !data.land.as_ref().is_some_and(|l| l.is_land(p)) || data.in_lake(p) {
                return false;
            }
            if places.places.iter().any(|t| {
                t.px.distance_squared(p) < clearance * clearance * 4.0 + t.radius * t.radius
            }) {
                return false;
            }
            let m = surface.height_m_at(p.x, p.y);
            let (lo, hi) = (-50.0f32, 2600.0f32);
            if m < lo || m > hi {
                return false;
            }
            // Slope over one pixel.
            let dz = (surface.height_m_at(p.x + 1.0, p.y) - m)
                .abs()
                .max((surface.height_m_at(p.x, p.y + 1.0) - m).abs());
            let slope = (dz / data.meters_per_px).atan().to_degrees();
            let _ = sp;
            slope < 14.0
        };
    let lod = lod_with_hysteresis(view.distance, &cfg.lod_distances, 0, cfg.lod_hysteresis) as u8;
    let mut spawned_herds = 0;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let cell = centre_cell + IVec2::new(dx, dy);
            let cell_mid = (cell.as_vec2() + Vec2::splat(0.5)) * cell_px;
            if cell_mid.distance(view.focus) > radius + cell_px {
                continue;
            }
            for herd in herds_of_cell(&data, cell, view.season, &valid) {
                if herd.centre.distance(view.focus) > radius {
                    continue;
                }
                let Some(sp) = cfg.species.get(&herd.species) else {
                    continue;
                };
                let Some(path) = model_path(&root.0, sp, lod) else {
                    continue;
                };
                let h = h01(herd.id, 9, 9);
                if view.thin_target(h) <= 0.02
                    || view.edge_target(herd.centre, data.props.folk_edge_inner) <= 0.02
                {
                    continue;
                }
                let gait = cfg.gaits.get(&sp.gait).cloned().unwrap_or_default();
                let length_world = sp.length_m * ws;
                for a in 0..herd.size {
                    let key = herd.id ^ ((a as u64 + 1) << 40);
                    if index.0.contains(&key) {
                        continue;
                    }
                    if index.0.len() >= cfg.max_visible || spawned_herds > 40 {
                        return;
                    }
                    index.0.insert(key);
                    let ang = h01(herd.id, a as u64, 5) * std::f32::consts::TAU;
                    let r = length_world * 3.0 * (0.2 + h01(herd.id, a as u64, 6)).sqrt();
                    let offset = Vec2::new(ang.cos(), ang.sin()) * r;
                    let p = herd.centre + offset;
                    let model = spawn_model(
                        &mut commands,
                        &asset_server,
                        &metrics,
                        &path,
                        FitAxis::Length,
                        length_world,
                        std::f32::consts::FRAC_PI_2,
                    );
                    let entity = commands
                        .spawn((
                            Name::new(format!("life:{}", sp.id)),
                            Transform::from_xyz(p.x, 0.0, p.y).with_scale(Vec3::splat(1e-4)),
                            Visibility::Inherited,
                            Animal {
                                gait: sp.gait.clone(),
                                herd: herd.centre,
                                offset,
                                phase: h01(herd.id, a as u64, 7) * std::f32::consts::TAU,
                                length_world,
                                wander_px: length_world * 2.0,
                                speed_px: gait.speed_m_s * ws * data.props.folk_speed,
                                species: sp.id.clone(),
                                lod,
                            },
                            LifeItem {
                                key,
                                cat: Cat::Fauna,
                                anchor: p,
                                h,
                            },
                            Fade::new(1.0),
                            DespawnOnExit(AppState::Campaign),
                        ))
                        .id();
                    commands.entity(entity).add_child(model);
                }
                spawned_herds += 1;
            }
        }
    }
}

/// Swaps the model of the animals whose LOD changed (hysteresis of the data), a few per frame.
pub fn update_lods(
    mut commands: Commands,
    data: Res<LifeData>,
    view: Res<LifeView>,
    asset_server: Res<AssetServer>,
    metrics: Res<ModelMetrics>,
    root: Option<Res<AssetRoot>>,
    mut animals: Query<(Entity, &mut Animal, &Children)>,
    models: Query<(), With<super::models::FitModel>>,
) {
    let Some(root) = root else { return };
    if !view.ready {
        return;
    }
    let cfg = &data.fauna;
    let mut swaps = 0;
    for (entity, mut animal, children) in &mut animals {
        let next = lod_with_hysteresis(
            view.distance,
            &cfg.lod_distances,
            animal.lod as usize,
            cfg.lod_hysteresis,
        ) as u8;
        if next == animal.lod {
            continue;
        }
        let Some(sp) = cfg.species.get(&animal.species) else {
            continue;
        };
        let Some(path) = model_path(&root.0, sp, next) else {
            animal.lod = next;
            continue;
        };
        if swaps >= 40 {
            break;
        }
        swaps += 1;
        for child in children.iter().filter(|c| models.contains(*c)) {
            commands.entity(child).despawn();
        }
        let model = spawn_model(
            &mut commands,
            &asset_server,
            &metrics,
            &path,
            FitAxis::Length,
            animal.length_world,
            std::f32::consts::FRAC_PI_2,
        );
        commands.entity(entity).add_child(model);
        animal.lod = next;
    }
}

pub fn move_animals(
    data: Res<LifeData>,
    motion: Res<super::motion::MotionTable>,
    view: Res<LifeView>,
    ground: GroundHeight,
    mut animals: Query<(&Animal, &mut LifeItem, &mut Transform)>,
) {
    let t = view.time;
    for (a, mut item, mut tr) in &mut animals {
        let gait = data.fauna.gaits.get(&a.gait).cloned().unwrap_or_default();
        // Slow loop around the herd spot at the gait speed.
        let omega = if a.wander_px > 0.0 {
            a.speed_px / a.wander_px
        } else {
            0.0
        };
        let ang = a.phase + t * omega;
        let wander = Vec2::new(ang.cos(), ang.sin()) * a.wander_px;
        let p = a.herd + a.offset + wander - Vec2::new(a.phase.cos(), a.phase.sin()) * a.wander_px;
        let heading = Vec2::new(-ang.sin(), ang.cos());
        item.anchor = p;
        let y = ground.at(p.x, p.y).unwrap_or(0.0);
        let step = (t * gait.cadence * std::f32::consts::TAU + a.phase).sin();
        // Grazing from the measured cycle of the species (the head lowers: the whole body pitches a
        // third of the angle, no neck bone), else the generic gait value.
        let grazing = match motion.spec(&a.species) {
            Some(spec) => {
                super::motion::graze(spec, t, u64::from(a.phase.to_bits())) * spec.graze_rad * 0.3
                    / 0.6
            }
            None => ((t * 0.3 + a.phase).sin() * 0.5 + 0.5) * gait.graze,
        };
        tr.translation = Vec3::new(
            p.x,
            y + step.abs() * gait.bounce * a.length_world * 0.2,
            p.y,
        );
        tr.rotation = Quat::from_rotation_y(-heading.y.atan2(heading.x))
            * Quat::from_rotation_z(-grazing * 0.6);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::life::data::{FaunaCfg, FaunaDensity, FaunaZone};

    fn data_with_zone() -> LifeData {
        let mut fauna = FaunaCfg {
            cell_px: 96.0,
            ..Default::default()
        };
        fauna.species.insert(
            "animal_x".into(),
            FaunaSpecies {
                id: "animal_x".into(),
                length_m: 2.0,
                placeholder: String::new(),
                gait: "walker".into(),
                herd_size: [3, 5],
                wander_m: 100.0,
            },
        );
        fauna.zones.push(FaunaZone {
            id: "z".into(),
            center_px: Vec2::new(500.0, 500.0),
            radius_px: Vec2::new(300.0, 300.0),
            species: vec![FaunaDensity {
                species: "animal_x".into(),
                per_1000km2: 40.0,
                seasons: [1.0, 1.0, 1.0, 0.0],
            }],
        });
        LifeData::for_tests(fauna)
    }

    #[test]
    fn herds_stay_in_the_zone_and_follow_the_season() {
        let data = data_with_zone();
        let mut spring = 0;
        let mut winter = 0;
        for cx in 0..10 {
            for cy in 0..10 {
                let cell = IVec2::new(cx, cy);
                for h in herds_of_cell(&data, cell, 0, &|_, _| true) {
                    assert!(h.centre.distance(Vec2::new(500.0, 500.0)) <= 300.0 * 1.0001);
                    assert!((3..=5).contains(&h.size));
                    spring += 1;
                }
                winter += herds_of_cell(&data, cell, 3, &|_, _| true).len();
            }
        }
        assert!(spring > 0);
        assert_eq!(winter, 0);
    }

    #[test]
    fn lod_switches_with_hysteresis() {
        let t = [35.0, 80.0];
        assert_eq!(lod_with_hysteresis(10.0, &t, 0, 0.06), 0);
        // Just past the threshold: still the fine LOD, then the next one.
        assert_eq!(lod_with_hysteresis(36.0, &t, 0, 0.06), 0);
        assert_eq!(lod_with_hysteresis(38.0, &t, 0, 0.06), 1);
        // Back down: stays on LOD 1 until under threshold x (1 - h).
        assert_eq!(lod_with_hysteresis(34.0, &t, 1, 0.06), 1);
        assert_eq!(lod_with_hysteresis(32.0, &t, 1, 0.06), 0);
        assert_eq!(lod_with_hysteresis(200.0, &t, 0, 0.06), 2);
    }

    #[test]
    fn placement_filter_removes_herds() {
        let data = data_with_zone();
        let cell = IVec2::new(5, 5);
        let all = herds_of_cell(&data, cell, 0, &|_, _| true).len();
        let none = herds_of_cell(&data, cell, 0, &|_, _| false).len();
        assert!(all >= none);
        assert_eq!(none, 0);
    }
}
