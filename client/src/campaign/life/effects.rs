//! Smoke plumes, windmills and fires of the settlements (`life_effects.gd`, wind and flame data
//! from `fx/map_fire_wind.json`). Rendering only: the sites are derived from the simulation
//! state (devastation of the province, siege of the settlement) exactly as `LifeEffects.rebuild`
//! does, and re-derived when that state changes.
//!
//! - **Plume**: a column of puffs bent along the wind; a light one above each settlement (all
//!   cities, towns, villages and abbeys, a share of the hamlets), a dark, thick one above
//!   besieged cities and devastated provinces. Size is a share of the screen height at the aimed
//!   point (RV-F), as in `life_plume.gdshader`.
//! - **Windmill**: body + turning sails (glb of `settlements/`), `WINDMILL_ONE_IN` per kind,
//!   stopped when the province is ruined.
//! - **Fire**: flickering flames (cones, size/colour/flicker from the data) in the faubourgs of a
//!   besieged city and in ruined villages, plus a few point lights on the nearest ones.

use std::collections::{HashMap, HashSet};

use bevy::mesh::MeshTag;
use bevy::prelude::*;

use super::data::{LifeData, h01, text_seed};
use super::models::{Fade, FitAxis, ModelMetrics, spawn_instance};
use super::stream::{Cat, Grounded, LifeIndex, LifeItem, LifeView};
use crate::app::AppState;
use crate::campaign::markers::places::Places;
use crate::sim::Sim;

const PLUME_KINDS: [&str; 4] = ["city", "town", "village", "abbey"];
const PLUME_HAMLET_ONE_IN: u64 = 6;
const PLUME_DENSITY: f32 = 0.42;
const WINTER_BOOST: f32 = 1.6;
const RUIN_MIN_DEVASTATION: u8 = 45;
const FIRE_MIN_DEVASTATION: u8 = 25;
const WINDMILL_SCALE: f32 = 4.6;
const WINDMILL_HUB: Vec3 = Vec3::new(0.0, 0.3, 0.08);
const WINDMILL_RING_MIN: f32 = 1.6;
const FIRE_RADIUS_RATIO: f32 = 0.9;
const SAIL_SPEED: f32 = 0.9;
/// `tan(fov / 2) * 2` of the campaign camera (45 degrees): world height of the screen per unit
/// of distance.
const SCREEN_PER_DISTANCE: f32 = 0.83;
const PUFFS: usize = 5;
const WINDMILL_BODY: &str = "models/settlements/windmill_body.glb";
const WINDMILL_SAILS: &str = "models/settlements/windmill_sails.glb";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SiteKind {
    Plume { dark: bool, rank: f32 },
    Mill { yaw: f32, phase: f32, running: bool },
    Fire { phase: f32 },
}

#[derive(Debug, Clone)]
pub struct Site {
    pub key: u64,
    pub px: Vec2,
    pub lift: f32,
    pub kind: SiteKind,
}

/// Sites derived from the settlements and the sim state.
#[derive(Resource, Default)]
pub struct EffectSites {
    pub sites: Vec<Site>,
    pub grid: HashMap<IVec2, Vec<usize>>,
    pub fingerprint: u64,
    /// Keys of the current sites (a plume whose site is gone fades out).
    pub keys: HashSet<u64>,
}

pub const CELL: f32 = 32.0;

fn cell_of(p: Vec2) -> IVec2 {
    (p / CELL).floor().as_ivec2()
}

/// State of the provinces and settlements that matters to the effects.
#[derive(Default)]
struct WorldState {
    devastation: HashMap<String, u8>,
    sieged: HashSet<String>,
}

fn world_state(sim: &Sim) -> WorldState {
    let mut w = WorldState::default();
    if let Some(state) = sim.state.as_ref() {
        for (id, p) in &state.provinces {
            w.devastation.insert(id.to_string(), p.devastation);
        }
        for (id, s) in &state.settlements {
            if s.siege.is_some() {
                w.sieged.insert(id.to_string());
            }
        }
    }
    w
}

/// Pure derivation of the sites (tested).
pub fn derive_sites(
    places: &[(String, String, String, Vec2, f32)],
    hamlets: &[(Vec2, String, String)],
    devastation: &HashMap<String, u8>,
    sieged: &HashSet<String>,
    fire_enabled: bool,
) -> Vec<Site> {
    let mut sites = Vec::new();
    for (id, province, kind, px, radius) in places {
        let seed = text_seed(id);
        let dev = devastation.get(province).copied().unwrap_or(0);
        let besieged = sieged.contains(id) && (kind == "city" || kind == "town");
        let ravaged = dev >= RUIN_MIN_DEVASTATION && kind != "castle";
        let top = radius * 0.6;
        let rank = (seed % 1000) as f32 / 1000.0;
        if besieged || ravaged {
            sites.push(Site {
                key: seed ^ 0x01,
                px: *px,
                lift: top,
                kind: SiteKind::Plume { dark: true, rank },
            });
        } else if PLUME_KINDS.contains(&kind.as_str()) || seed.is_multiple_of(2) {
            sites.push(Site {
                key: seed ^ 0x01,
                px: *px,
                lift: top,
                kind: SiteKind::Plume { dark: false, rank },
            });
        }
        if fire_enabled {
            if sieged.contains(id) && (kind == "city" || kind == "town") {
                for k in 0..3u64 {
                    let a = ((seed / (k + 5)) % 628) as f32 / 100.0;
                    let at = *px + Vec2::new(a.cos(), a.sin()) * radius * FIRE_RADIUS_RATIO;
                    sites.push(Site {
                        key: seed ^ (0x10 + k),
                        px: at,
                        lift: 0.2,
                        kind: SiteKind::Fire {
                            phase: k as f32 / 3.0,
                        },
                    });
                }
            } else if kind == "village" && dev >= RUIN_MIN_DEVASTATION {
                sites.push(Site {
                    key: seed ^ 0x20,
                    px: *px,
                    lift: 0.3,
                    kind: SiteKind::Fire {
                        phase: (seed % 97) as f32 / 97.0,
                    },
                });
            }
        }
        let (count, one_in) = match kind.as_str() {
            "city" => (1, 1),
            "town" => (1, 2),
            "village" => (1, 3),
            _ => (0, 1),
        };
        let mseed = text_seed(&format!("{id}mill"));
        if count > 0 && mseed.is_multiple_of(one_in) {
            let angle = ((mseed / 2) % 628) as f32 / 100.0;
            let distance = radius * (WINDMILL_RING_MIN + ((mseed / 5) % 60) as f32 / 100.0);
            sites.push(Site {
                key: mseed ^ 0x30,
                px: *px + Vec2::new(angle.cos(), angle.sin()) * distance,
                lift: 0.0,
                kind: SiteKind::Mill {
                    yaw: ((mseed / 9) % 628) as f32 / 100.0,
                    phase: (mseed % 1000) as f32 / 1000.0,
                    running: dev < RUIN_MIN_DEVASTATION,
                },
            });
        }
    }
    for (px, name, province) in hamlets {
        let seed = text_seed(name);
        let dev = devastation.get(province).copied().unwrap_or(0);
        let rank = (seed % 1000) as f32 / 1000.0;
        let burned = dev >= FIRE_MIN_DEVASTATION && seed.is_multiple_of(PLUME_HAMLET_ONE_IN);
        if burned {
            sites.push(Site {
                key: seed ^ 0x40,
                px: *px,
                lift: 0.1,
                kind: SiteKind::Plume { dark: true, rank },
            });
            if fire_enabled {
                sites.push(Site {
                    key: seed ^ 0x41,
                    px: *px,
                    lift: 0.1,
                    kind: SiteKind::Fire { phase: rank },
                });
            }
        } else if seed % PLUME_HAMLET_ONE_IN == 1 {
            sites.push(Site {
                key: seed ^ 0x40,
                px: *px,
                lift: 0.1,
                kind: SiteKind::Plume { dark: false, rank },
            });
        }
    }
    sites
}

/// Rebuilds the sites when the settlements, the devastation or the sieges change.
pub fn refresh_sites(
    places: Res<Places>,
    data: Res<LifeData>,
    sim: Res<Sim>,
    scenes: Res<super::scenes::StagedScenes>,
    mut sites: ResMut<EffectSites>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 || places.places.is_empty() {
        return;
    }
    *timer = 1.0;
    let world = world_state(&sim);
    let mut fp: u64 = places.places.len() as u64;
    for (k, v) in &world.devastation {
        fp = fp.wrapping_add(text_seed(k).wrapping_mul(u64::from(*v) + 1));
    }
    for s in &world.sieged {
        fp = fp.wrapping_add(text_seed(s).rotate_left(7));
    }
    // The scenes of the turn change the plumes (quiet chimneys, pyre and riot smoke).
    fp = fp.wrapping_add(u64::from(scenes.version).wrapping_mul(0x9E37_79B9));
    if fp == sites.fingerprint && !sites.sites.is_empty() {
        return;
    }
    sites.fingerprint = fp;
    let list: Vec<(String, String, String, Vec2, f32)> = places
        .places
        .iter()
        .filter(|p| !p.absorbed)
        .map(|p| {
            (
                p.id.clone(),
                p.province.clone(),
                p.kind.clone(),
                p.px,
                p.radius,
            )
        })
        .collect();
    sites.sites = derive_sites(
        &list,
        &data.hamlets,
        &world.devastation,
        &world.sieged,
        data.fire.enabled,
    );
    apply_scene_effects(&mut sites.sites, &scenes);
    let mut grid: HashMap<IVec2, Vec<usize>> = HashMap::new();
    for (i, s) in sites.sites.iter().enumerate() {
        grid.entry(cell_of(s.px)).or_default().push(i);
    }
    sites.grid = grid;
    let keys: HashSet<u64> = sites
        .sites
        .iter()
        .map(|s| s.key.wrapping_add(0x5000_0000))
        .collect();
    sites.keys = keys;
}

/// A plume whose site disappeared (chimneys put out by a plague scene) fades out. Runs after
/// `stream::update_targets`.
pub fn retire_stale_plumes(sites: Res<EffectSites>, mut items: Query<(&LifeItem, &mut Fade)>) {
    if sites.keys.is_empty() {
        return;
    }
    for (item, mut fade) in &mut items {
        if item.cat == Cat::Plume && !sites.keys.contains(&item.key) {
            fade.target = 0.0;
        }
    }
}

/// Scene effects outside the pool (`FolkScenes.quiet_settlements`, `fire_points`): the chimneys of
/// the settlements of a plague scene are put out, the pyres and the riots send up a dark plume.
pub fn apply_scene_effects(sites: &mut Vec<Site>, scenes: &super::scenes::StagedScenes) {
    for id in &scenes.quiet_settlements {
        let key = text_seed(id) ^ 0x01;
        sites.retain(|s| !(s.key == key && matches!(s.kind, SiteKind::Plume { dark: false, .. })));
    }
    for (n, p) in scenes.fire_points.iter().enumerate() {
        let key = text_seed("scene_smoke") ^ ((n as u64) << 8);
        sites.push(Site {
            key,
            px: *p,
            lift: 0.2,
            kind: SiteKind::Plume {
                dark: true,
                rank: (key % 1000) as f32 / 1000.0,
            },
        });
    }
}

#[derive(Resource)]
pub struct EffectAssets {
    pub puff: Handle<Mesh>,
    pub smoke_light: Handle<StandardMaterial>,
    pub smoke_dark: Handle<StandardMaterial>,
    pub flame: Handle<Mesh>,
    pub flame_mid: Handle<StandardMaterial>,
    pub flame_core: Handle<StandardMaterial>,
}

pub fn build_assets(
    mut commands: Commands,
    data: Res<LifeData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let f = &data.fire;
    let smoke = |c: [f32; 3], a: f32| StandardMaterial {
        base_color: Color::srgba(c[0], c[1], c[2], a),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    };
    let emissive = |c: [f32; 3], k: f32, a: f32| StandardMaterial {
        base_color: Color::srgba(c[0], c[1], c[2], a),
        emissive: LinearRgba::rgb(c[0] * k, c[1] * k, c[2] * k),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    };
    commands.insert_resource(EffectAssets {
        puff: meshes.add(
            Sphere::new(0.5)
                .mesh()
                .ico(1)
                .unwrap_or_else(|_| Sphere::new(0.5).mesh().into()),
        ),
        smoke_light: materials.add(smoke([0.78, 0.78, 0.77], 0.35)),
        smoke_dark: materials.add(smoke([0.12, 0.11, 0.10], 0.6)),
        flame: meshes.add(Cone::new(0.5, 1.0)),
        flame_mid: materials.add(emissive(f.color_mid, f.emission, f.opacity)),
        flame_core: materials.add(emissive(f.color_hot, f.emission, f.opacity)),
    });
}

#[derive(Component)]
pub struct Plume {
    pub dark: bool,
}

#[derive(Component)]
pub struct Puff {
    pub phase: f32,
}

#[derive(Component)]
pub struct Flame {
    pub phase: f32,
}

#[derive(Component)]
pub struct Sail {
    pub yaw: f32,
    pub phase: f32,
    pub speed: f32,
}

/// Places the sites of the zone around the focus (a few per frame at most).
pub fn spawn_sites(
    mut commands: Commands,
    assets: Res<AssetServer>,
    metrics: Res<ModelMetrics>,
    effects: Option<Res<EffectAssets>>,
    mut flipbook: Option<ResMut<crate::fx::flipbook::FlipbookLooks>>,
    mut flipbook_materials: ResMut<Assets<crate::fx::flipbook::FlipbookMaterial>>,
    data: Res<LifeData>,
    view: Res<LifeView>,
    sites: Res<EffectSites>,
    mut index: ResMut<LifeIndex>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 || !view.ready {
        return;
    }
    *timer = 0.2;
    let Some(effects) = effects else { return };
    if view.range_target(data.props.chimney_range.max(500.0), 0.7) <= 0.0 {
        return;
    }
    let density = PLUME_DENSITY * if view.season == 3 { WINTER_BOOST } else { 1.0 };
    let radius = view.radius * 0.98;
    let reach = (radius / CELL).ceil() as i32 + 1;
    let centre = cell_of(view.focus);
    let mut spawned = 0;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let Some(list) = sites.grid.get(&(centre + IVec2::new(dx, dy))) else {
                continue;
            };
            for &i in list {
                let site = &sites.sites[i];
                let key = site.key.wrapping_add(0x5000_0000);
                if index.0.contains(&key) || site.px.distance(view.focus) > radius {
                    continue;
                }
                if view.edge_target(site.px, 0.85) <= 0.02 {
                    continue;
                }
                if spawned >= 60 {
                    return;
                }
                match site.kind {
                    SiteKind::Plume { dark, rank } => {
                        if !dark && rank >= density {
                            continue;
                        }
                        index.0.insert(key);
                        spawned += 1;
                        let smoke = flipbook
                            .as_deref_mut()
                            .map(|looks| PlumeSmoke::new(looks, &mut flipbook_materials, dark));
                        spawn_plume(&mut commands, &effects, smoke, site, key, dark, rank);
                    }
                    SiteKind::Mill {
                        yaw,
                        phase,
                        running,
                    } => {
                        index.0.insert(key);
                        spawned += 1;
                        let scale = WINDMILL_SCALE * data.props.windmill_ratio;
                        let p = site.px;
                        let heading = Quat::from_rotation_y(yaw);
                        spawn_instance(
                            &mut commands,
                            &assets,
                            &metrics,
                            WINDMILL_BODY,
                            FitAxis::Native,
                            scale,
                            0.0,
                            Vec3::new(p.x, 0.0, p.y),
                            heading,
                            (
                                LifeItem {
                                    key,
                                    cat: Cat::Mill,
                                    anchor: p,
                                    h: 0.0,
                                },
                                Grounded::new(-0.05 * data.props.windmill_ratio),
                                DespawnOnExit(AppState::Campaign),
                            ),
                        );
                        let hub = heading * (WINDMILL_HUB * scale);
                        spawn_instance(
                            &mut commands,
                            &assets,
                            &metrics,
                            WINDMILL_SAILS,
                            FitAxis::Native,
                            scale,
                            0.0,
                            Vec3::new(p.x, 0.0, p.y) + hub,
                            heading,
                            (
                                LifeItem {
                                    key: key ^ 0xA5A5,
                                    cat: Cat::Sail,
                                    anchor: p,
                                    h: 0.0,
                                },
                                Grounded::new(hub.y - 0.05 * data.props.windmill_ratio),
                                Sail {
                                    yaw,
                                    phase: phase * std::f32::consts::TAU,
                                    speed: if running { SAIL_SPEED } else { 0.0 },
                                },
                                DespawnOnExit(AppState::Campaign),
                            ),
                        );
                        index.0.insert(key ^ 0xA5A5);
                    }
                    SiteKind::Fire { phase } => {
                        index.0.insert(key);
                        spawned += 1;
                        let f = &data.fire;
                        let root = commands
                            .spawn((
                                Name::new("life:fire"),
                                Transform::from_xyz(site.px.x, 0.0, site.px.y)
                                    .with_scale(Vec3::splat(1e-4)),
                                Visibility::Inherited,
                                LifeItem {
                                    key,
                                    cat: Cat::Fire,
                                    anchor: site.px,
                                    h: 0.0,
                                },
                                Grounded::new(site.lift),
                                Fade::new(1.0),
                                DespawnOnExit(AppState::Campaign),
                            ))
                            .id();
                        // The player's FA flame sheet (`life_flame.gdshader`); the cones stay
                        // as the fallback while the flipbook plugin is absent.
                        if let Some(looks) = flipbook.as_deref() {
                            let flame = crate::fx::flipbook::spawn_static_flame(
                                &mut commands,
                                looks,
                                phase,
                                Transform::from_scale(Vec3::new(
                                    f.flame_size[0],
                                    f.flame_size[1],
                                    1.0,
                                )),
                            );
                            commands.entity(root).add_child(flame);
                            continue;
                        }
                        for (n, (mat, k)) in
                            [(&effects.flame_mid, 1.0f32), (&effects.flame_core, 0.55)]
                                .into_iter()
                                .enumerate()
                        {
                            let flame = commands
                                .spawn((
                                    Mesh3d(effects.flame.clone()),
                                    MeshMaterial3d(mat.clone()),
                                    Transform::from_xyz(0.0, f.flame_size[1] * k * 0.5, 0.0)
                                        .with_scale(Vec3::new(
                                            f.flame_size[0] * k,
                                            f.flame_size[1] * k,
                                            f.flame_size[0] * k,
                                        )),
                                    Flame {
                                        phase: phase + n as f32 * 0.37,
                                    },
                                ))
                                .id();
                            commands.entity(root).add_child(flame);
                        }
                    }
                }
            }
        }
    }
}

/// Flipbook look of a plume's puffs (`life_smoke.gdshader` colours, the FA smoke sheet).
struct PlumeSmoke {
    quad: Handle<Mesh>,
    material: Handle<crate::fx::flipbook::FlipbookMaterial>,
}

/// Light and dark plume colours (sRGB) and opacity, as the two `StandardMaterial`s they replace.
const PLUME_LIGHT: [f32; 4] = [0.78, 0.78, 0.77, 0.55];
const PLUME_DARK: [f32; 4] = [0.12, 0.11, 0.10, 0.9];

impl PlumeSmoke {
    fn new(
        looks: &mut crate::fx::flipbook::FlipbookLooks,
        materials: &mut Assets<crate::fx::flipbook::FlipbookMaterial>,
        dark: bool,
    ) -> Self {
        let color = if dark { PLUME_DARK } else { PLUME_LIGHT };
        PlumeSmoke {
            quad: looks.quad.clone(),
            material: looks.smoke_tint(color, materials),
        }
    }
}

fn spawn_plume(
    commands: &mut Commands,
    effects: &EffectAssets,
    smoke: Option<PlumeSmoke>,
    site: &Site,
    key: u64,
    dark: bool,
    rank: f32,
) {
    let root = commands
        .spawn((
            Name::new("life:plume"),
            Transform::from_xyz(site.px.x, 0.0, site.px.y).with_scale(Vec3::splat(1e-4)),
            Visibility::Inherited,
            Plume { dark },
            LifeItem {
                key,
                cat: Cat::Plume,
                anchor: site.px,
                h: rank,
            },
            Grounded::new(site.lift),
            Fade::new(1.0),
            DespawnOnExit(AppState::Campaign),
        ))
        .id();
    for n in 0..PUFFS {
        let phase = n as f32 / PUFFS as f32 + rank;
        let puff = match &smoke {
            Some(smoke) => commands
                .spawn((
                    Mesh3d(smoke.quad.clone()),
                    MeshMaterial3d(smoke.material.clone()),
                    MeshTag::new(0),
                    Transform::from_scale(Vec3::splat(1e-4)),
                    Puff { phase },
                    bevy::light::NotShadowCaster,
                ))
                .id(),
            None => commands
                .spawn((
                    Mesh3d(effects.puff.clone()),
                    MeshMaterial3d(if dark {
                        effects.smoke_dark.clone()
                    } else {
                        effects.smoke_light.clone()
                    }),
                    Transform::from_scale(Vec3::splat(1e-4)),
                    Puff { phase },
                ))
                .id(),
        };
        commands.entity(root).add_child(puff);
    }
}

/// Puffs rise along the plume, bend with the wind and swell, then shrink away (no pop).
pub fn animate_plumes(
    data: Res<LifeData>,
    view: Res<LifeView>,
    plumes: Query<(&Plume, &Children)>,
    mut puffs: Query<(&Puff, &mut Transform, Option<&mut MeshTag>)>,
) {
    let wind = Vec3::new(data.fire.wind_dir.x, 0.0, data.fire.wind_dir.y);
    let d = view.distance;
    let lean = 0.35;
    for (plume, children) in &plumes {
        let gain = if plume.dark { 1.8 } else { 1.0 };
        let width = 0.0133 * d * gain * SCREEN_PER_DISTANCE / 0.83;
        let height = 0.028 * d * gain * SCREEN_PER_DISTANCE / 0.83;
        let rate = if plume.dark { 0.22 } else { 0.14 };
        for child in children.iter() {
            let Ok((puff, mut t, tag)) = puffs.get_mut(child) else {
                continue;
            };
            let age = (view.time * rate + puff.phase).fract();
            if let Some(mut tag) = tag {
                // The sheet plays once over the life of the puff, turning slowly.
                tag.value = crate::fx::flipbook::pack_tag(age, 0.0, puff.phase * 0.5);
            }
            let grow = 0.45 + 1.1 * age;
            let appear = (age / 0.12).clamp(0.0, 1.0);
            let vanish = ((1.0 - age) / 0.3).clamp(0.0, 1.0);
            let s = width * grow * appear * vanish;
            t.translation = Vec3::Y * height * age + wind * (height * lean * age * age);
            t.scale = Vec3::splat(s.max(1e-4));
        }
    }
}

pub fn animate_flames(
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut flames: Query<(&Flame, &mut Transform)>,
) {
    let f = &data.fire;
    for (flame, mut t) in &mut flames {
        let w = (view.time * f.flicker_hz + flame.phase) * std::f32::consts::TAU;
        let flick = 1.0 + 0.12 * w.sin() + 0.06 * (w * 2.3 + 1.0).sin();
        t.scale.y = t.scale.x * (f.flame_size[1] / f.flame_size[0]) * flick;
        t.translation.y = t.scale.y * 0.5;
        // Leans downwind by `lean`.
        let wind = Vec3::new(data.fire.wind_dir.x, 0.0, data.fire.wind_dir.y);
        t.rotation = Quat::from_rotation_arc(
            Vec3::Y,
            (Vec3::Y + wind * f.lean * (0.8 + 0.2 * w.cos())).normalize(),
        );
    }
}

pub fn spin_sails(view: Res<LifeView>, mut sails: Query<(&Sail, &mut Transform)>) {
    for (sail, mut t) in &mut sails {
        let angle = sail.phase + view.time * sail.speed;
        t.rotation = Quat::from_rotation_y(sail.yaw) * Quat::from_rotation_z(angle);
    }
}

/// Point lights on the nearest fires (`fire.light`).
#[derive(Component)]
pub struct FireLight {
    pub slot: usize,
}

pub fn spawn_fire_lights(mut commands: Commands, data: Res<LifeData>) {
    let f = &data.fire;
    for slot in 0..f.max_lights {
        commands.spawn((
            Name::new("life:fire_light"),
            PointLight {
                color: Color::srgb(f.light_color[0], f.light_color[1], f.light_color[2]),
                intensity: 0.0,
                range: f.light_range,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            FireLight { slot },
            DespawnOnExit(AppState::Campaign),
        ));
    }
}

/// Lumens per unit of Godot light energy.
const LUMENS_PER_ENERGY: f32 = 40_000.0;

pub fn update_fire_lights(
    data: Res<LifeData>,
    view: Res<LifeView>,
    fires: Query<(&LifeItem, &GlobalTransform, &Fade), Without<FireLight>>,
    mut lights: Query<(&FireLight, &mut PointLight, &mut Transform)>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    let f = &data.fire;
    let on = f.max_lights > 0 && view.distance < f.light_max_distance;
    let mut near: Vec<(f32, Vec3)> = Vec::new();
    if on {
        near = fires
            .iter()
            .filter(|(i, _, fade)| i.cat == Cat::Fire && fade.current > 0.3)
            .map(|(i, g, _)| (i.anchor.distance(view.focus), g.translation()))
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    *timer += time.delta_secs();
    for (light, mut pl, mut t) in &mut lights {
        match near.get(light.slot) {
            Some((_, p)) => {
                let flick = 1.0
                    + 0.35 * ((view.time * 9.0 + light.slot as f32 * 2.1).sin() * 0.5 + 0.5 - 0.5);
                pl.intensity = f.light_energy * LUMENS_PER_ENERGY * flick;
                t.translation = *p + Vec3::Y * f.light_height;
            }
            None => pl.intensity = 0.0,
        }
    }
    let _ = h01(0, 0, 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(id: &str, province: &str, kind: &str) -> (String, String, String, Vec2, f32) {
        (
            id.into(),
            province.into(),
            kind.into(),
            Vec2::new(100.0, 100.0),
            3.0,
        )
    }

    #[test]
    fn quiet_state_gives_light_plumes_and_no_fire() {
        let places = vec![
            place("set_a", "prov_a", "city"),
            place("set_b", "prov_a", "village"),
        ];
        let sites = derive_sites(&places, &[], &HashMap::new(), &HashSet::new(), true);
        assert!(
            sites
                .iter()
                .any(|s| matches!(s.kind, SiteKind::Plume { dark: false, .. }))
        );
        assert!(
            !sites
                .iter()
                .any(|s| matches!(s.kind, SiteKind::Fire { .. }))
        );
        assert!(
            !sites
                .iter()
                .any(|s| matches!(s.kind, SiteKind::Plume { dark: true, .. }))
        );
    }

    #[test]
    fn siege_gives_dark_plume_and_three_fires() {
        let places = vec![place("set_a", "prov_a", "city")];
        let sieged: HashSet<String> = ["set_a".to_string()].into();
        let sites = derive_sites(&places, &[], &HashMap::new(), &sieged, true);
        let fires = sites
            .iter()
            .filter(|s| matches!(s.kind, SiteKind::Fire { .. }))
            .count();
        assert_eq!(fires, 3);
        assert!(
            sites
                .iter()
                .any(|s| matches!(s.kind, SiteKind::Plume { dark: true, .. }))
        );
        // Fire effects off: no flames, plume stays.
        let off = derive_sites(&places, &[], &HashMap::new(), &sieged, false);
        assert_eq!(
            off.iter()
                .filter(|s| matches!(s.kind, SiteKind::Fire { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn ruined_province_stops_mills_and_burns_villages() {
        let places = vec![place("set_a", "prov_a", "city")];
        let calm = derive_sites(&places, &[], &HashMap::new(), &HashSet::new(), true);
        let dev: HashMap<String, u8> = [("prov_a".to_string(), 80u8)].into();
        let ruined = derive_sites(&places, &[], &dev, &HashSet::new(), true);
        let running = |v: &[Site]| {
            v.iter()
                .filter_map(|s| match s.kind {
                    SiteKind::Mill { running, .. } => Some(running),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(running(&calm), vec![true]);
        assert_eq!(running(&ruined), vec![false]);
    }
}
