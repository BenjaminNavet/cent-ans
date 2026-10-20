//! Streaming of the understory and ground tufts around the camera (lot TS-T7): see
//! `understory.rs` for the placement. Cells are sown on worker threads, meshed one per frame
//! (one merged mesh per category and cell, the heights those of the displayed ground, re-seated
//! when the relief changes), shown nearest first under the quality budget and faded out by the
//! alpha cutoff of the shared materials as the camera leaves the range of the level.
//!
//! Sown in the season of the camera: a season change sows again (the pale tufts follow the snow).
//! The summer and winter tints come from the settings; the glide of the season weights blends them.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde_json::{Value, json};

use super::clutter::{Cover, bush_image, mipmapped_sampler};
use super::understory::{
    CATEGORIES, CATEGORY_COUNT, Config, Env, FAR_CATEGORIES, NEAR_CATEGORIES, Piece, SnowModel,
    build_mesh_data, cells_within_budget, seed_far, seed_near,
};
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::season::{SeasonParams, SeasonWeights};
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::vegetation::VegWorld;
use crate::campaign::vegetation::data::VegetationWorld;
use crate::campaign::vegetation::mask::{fbm, smoothstep};
use crate::campaign::vegetation::mips;
use crate::paths;
use crate::settings::RenderQuality;

/// Cells meshed (or re-seated) per frame and level.
const BUILDS_PER_FRAME: usize = 2;
const MAX_JOBS: usize = 4;
/// Alpha cutoff of a fully visible card, and of a card that has just faded out.
const CUTOFF_VISIBLE: f32 = 0.4;
const CUTOFF_GONE: f32 = 0.98;

/// The world seen by the sowing (read-only, built on the worker thread).
struct WorldEnv<'a> {
    world: &'a VegetationWorld,
    cover: Cover<'a>,
    cfg: &'a Config,
    season: usize,
}

impl Env for WorldEnv<'_> {
    fn forest(&self, p: Vec2) -> f32 {
        self.world.sample(p.x as f64, p.y as f64).forest
    }

    fn crops(&self, p: Vec2) -> f32 {
        self.world.sample(p.x as f64, p.y as f64).crops
    }

    fn open(&self, p: Vec2) -> bool {
        self.cover.open(p)
    }

    fn snow(&self, p: Vec2) -> f32 {
        let model = &self.cfg.snow;
        let line = model.snow_lines[self.season.min(3)];
        let mut cover = 0.0;
        if line > 0.0 {
            let height = self.world.heights.height_m_at(p.x, p.y);
            let jitter = fbm(p.x, p.y, 911, 0.03, 3, 0.5) * 0.5 * model.line_jitter_m;
            cover = smoothstep(line - 250.0, line + 250.0, height + jitter);
        }
        if self.season == 3 {
            let map_height = self.world.size.1 as f32;
            let north = 1.0
                - smoothstep(
                    model.snow_plain.x * map_height,
                    model.snow_plain.y * map_height,
                    p.y,
                );
            cover = cover.max(north * model.snow_plain.z);
        }
        cover
    }

    fn slope(&self, p: Vec2) -> f32 {
        self.world.slope(p.x as f64, p.y as f64)
    }

    fn rock(&self, p: Vec2) -> f32 {
        let altitude = self.world.heights.height_m_at(p.x, p.y);
        self.cfg.rock_share(self.slope(p), altitude)
    }

    fn pale(&self, p: Vec2) -> f32 {
        let pale = &self.cfg.pale;
        let patch = fbm(p.x, p.y, 733, pale.patch_frequency, 3, 0.5) * 0.5 + 0.5;
        let snow = self.snow(p);
        let from_snow = smoothstep(pale.snow_min, pale.snow_min + 0.3, snow)
            * (0.35 + 0.65 * smoothstep(0.35, 0.65, patch));
        let height = self.world.heights.height_m_at(p.x, p.y);
        let treeline = self.world.treeline_m(p.y as f64);
        let tundra = smoothstep(
            treeline - pale.tundra_start_m,
            treeline - pale.tundra_end_m,
            height,
        ) * pale.tundra_share
            * (0.5 + 0.5 * patch);
        from_snow.max(tundra)
    }
}

type Slot = Arc<Mutex<Option<(Vec<Piece>, f32)>>>;

enum Seeding {
    Pending(Slot),
    Sown(Arc<Vec<Piece>>),
}

struct UCell {
    seeding: Seeding,
    entities: Vec<(usize, Entity, Handle<Mesh>)>,
    seated: Option<u32>,
    counts: [usize; CATEGORY_COUNT],
    last_seen: u64,
}

#[derive(Default)]
struct LevelState {
    cells: HashMap<IVec2, UCell>,
    shown: [usize; CATEGORY_COUNT],
    sown: [usize; CATEGORY_COUNT],
    cells_shown: usize,
    built: usize,
    reseated: usize,
    evicted: usize,
    over_budget: usize,
    budget: usize,
    fade: f32,
    active: bool,
    build_ms_max: f32,
    seed_ms_max: f32,
}

#[derive(Resource, Default)]
pub struct Understory {
    config: Option<Arc<Config>>,
    config_tried: bool,
    levels: [LevelState; 2],
    materials: Option<Vec<Handle<StandardMaterial>>>,
    mips_done: [bool; CATEGORY_COUNT],
    focus: Vec2,
    distance: f32,
    quality: String,
    relief_version: u32,
    season: usize,
    weights: Vec4,
    frame: u64,
    clear_pending: bool,
}

/// Camera, quality, season and relief.
pub fn update_view(
    rig: Option<Res<CampaignRig>>,
    quality: Option<Res<RenderQuality>>,
    season: Option<Res<SeasonWeights>>,
    relief: Option<Res<super::ReliefVersion>>,
    mut state: ResMut<Understory>,
) {
    if state.config.is_none() && !state.config_tried {
        state.config_tried = true;
        state.config = load_config().map(Arc::new);
    }
    if relief.is_some_and(|r| r.is_changed()) {
        state.relief_version = state.relief_version.wrapping_add(1);
    }
    if let Some(season) = season {
        state.weights = season.weights;
        let strongest = (0..4)
            .max_by(|a, b| season.weights[*a].total_cmp(&season.weights[*b]))
            .unwrap_or(0);
        if strongest != state.season {
            // The pale tufts follow the snow: sow again.
            state.season = strongest;
            state.relief_version = state.relief_version.wrapping_add(1);
            state.clear_pending = true;
        }
    }
    state.quality = quality.map_or_else(|| "high".to_string(), |q| q.level.clone());
    let Some(rig) = rig else {
        for level in &mut state.levels {
            level.active = false;
        }
        return;
    };
    state.focus = Vec2::new(rig.focus.x, rig.focus.z);
    state.distance = rig.distance;
    let Some(cfg) = state.config.clone() else {
        return;
    };
    for (index, level_cfg) in [&cfg.far, &cfg.near_level].into_iter().enumerate() {
        let budget = cfg.budget_for(index, &state.quality);
        let fade = level_cfg.fade(rig.distance);
        let level = &mut state.levels[index];
        level.budget = budget;
        level.fade = fade;
        level.active = budget > 0 && fade > 0.02;
    }
}

fn load_config() -> Option<Config> {
    let root = paths::find_root().ok()?;
    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("data/fx/understory.json")).ok()?)
            .ok()?;
    let snow = std::fs::read_to_string(root.join("data/ui/campaign_seasons.json"))
        .map(|text| SeasonParams::from_json(&text))
        .map(|season| SnowModel {
            snow_lines: season.snow_lines,
            snow_plain: season.snow_plain,
            line_jitter_m: season.snow_mask.y,
            rock_band: season.rock_band,
        })
        .unwrap_or_default();
    Some(Config::from_json(&doc, snow))
}

/// A few crossed branches with twigs, grey-brown (tinted by the material), on a transparent card.
pub fn deadwood_image() -> Image {
    const N: usize = 128;
    let mut data = vec![0u8; N * N * 4];
    let mut seed = 0x51F1_5EEDu32;
    let mut next = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / 16_777_216.0
    };
    // (start, end, half-width) in [0, 1] card units.
    let mut branches: Vec<(Vec2, Vec2, f32)> = Vec::new();
    for k in 0..3 {
        let angle = (k as f32 - 1.0) * 0.5 + (next() - 0.5) * 0.5;
        let centre = Vec2::new(0.5 + (next() - 0.5) * 0.15, 0.5 + (next() - 0.5) * 0.3);
        let half = Vec2::new(angle.cos(), angle.sin()) * (0.36 + next() * 0.1);
        branches.push((centre - half, centre + half, 0.022 + next() * 0.01));
        for _ in 0..2 {
            let t = 0.2 + next() * 0.6;
            let at = centre - half + half * 2.0 * t;
            let twig_angle = angle + (if next() < 0.5 { -1.0 } else { 1.0 }) * (0.6 + next() * 0.5);
            let twig = Vec2::new(twig_angle.cos(), twig_angle.sin()) * (0.1 + next() * 0.08);
            branches.push((at, at + twig, 0.012));
        }
    }
    for j in 0..N {
        for i in 0..N {
            let p = Vec2::new(i as f32 + 0.5, j as f32 + 0.5) / N as f32;
            let mut alpha = 0.0f32;
            for (a, b, half_width) in &branches {
                let ab = *b - *a;
                let t = ((p - *a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                let d = (*a + ab * t - p).length();
                alpha = alpha.max(1.0 - smoothstep(*half_width * 0.6, *half_width, d));
            }
            let grain = 0.8 + 0.2 * fbm(p.x * 90.0, p.y * 90.0, 17, 1.0, 2, 0.5);
            let o = (j * N + i) * 4;
            data[o] = (150.0 * grain) as u8;
            data[o + 1] = (122.0 * grain) as u8;
            data[o + 2] = (92.0 * grain) as u8;
            data[o + 3] = (alpha * 255.0) as u8;
        }
    }
    let levels = 6;
    let chain = mips::chain(&data, N, N, levels, true);
    let mut image = Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = mipmapped_sampler();
    image
}

fn ensure_materials(
    server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    cfg: &Config,
    state: &mut Understory,
) {
    if state.materials.is_none() {
        let handles = cfg
            .kinds
            .iter()
            .map(|kind| {
                let texture = match kind.texture.as_str() {
                    "@bush" => images.add(bush_image()),
                    "@deadwood" => images.add(deadwood_image()),
                    path => server.load::<Image>(path.to_string()),
                };
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(texture),
                    alpha_mode: AlphaMode::Mask(CUTOFF_VISIBLE),
                    cull_mode: None,
                    double_sided: true,
                    perceptual_roughness: 0.95,
                    reflectance: 0.05,
                    ..default()
                })
            })
            .collect();
        state.materials = Some(handles);
    }
    // The cards load without mipmaps: build the chain once they are there.
    let Some(handles) = state.materials.clone() else {
        return;
    };
    for (i, handle) in handles.iter().enumerate() {
        if state.mips_done[i] || cfg.kinds[i].texture.starts_with('@') {
            state.mips_done[i] = true;
            continue;
        }
        let Some(texture) = materials
            .get(handle)
            .and_then(|m| m.base_color_texture.clone())
        else {
            continue;
        };
        let Some(mut image) = images.get_mut(&texture) else {
            continue;
        };
        let (w, h) = (image.width() as usize, image.height() as usize);
        let usable = image.texture_descriptor.mip_level_count == 1
            && image.data.as_ref().is_some_and(|d| d.len() == w * h * 4)
            && w % 2 == 0
            && h % 2 == 0;
        if usable {
            let levels = mips::level_count(w.min(h) as u32).max(1) + 2;
            let levels = levels.min((w.min(h) as f32).log2() as u32);
            let base = image.data.take().expect("image data");
            image.data = Some(mips::chain(&base, w, h, levels, true));
            image.texture_descriptor.mip_level_count = levels;
            image.sampler = mipmapped_sampler();
            state.mips_done[i] = true;
        } else if image.data.is_some() {
            state.mips_done[i] = true;
        }
    }
}

/// Season tint (blended by the season weights) and fade-out (alpha cutoff) of the materials.
pub fn apply_look(state: Res<Understory>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let (Some(handles), Some(cfg)) = (&state.materials, &state.config) else {
        return;
    };
    for (i, handle) in handles.iter().enumerate() {
        let tint: Vec3 = (0..4)
            .map(|s| Vec3::from_array(cfg.kinds[i].season_tint[s]) * state.weights[s])
            .sum();
        let want = Color::linear_rgb(tint.x, tint.y, tint.z);
        let level = if FAR_CATEGORIES.contains(&i) { 0 } else { 1 };
        let cutoff =
            CUTOFF_VISIBLE + (1.0 - state.levels[level].fade) * (CUTOFF_GONE - CUTOFF_VISIBLE);
        let Some(material) = materials.get(handle) else {
            continue;
        };
        let cutoff_now = match material.alpha_mode {
            AlphaMode::Mask(c) => c,
            _ => cutoff,
        };
        if (material.base_color != want || (cutoff_now - cutoff).abs() > 0.01)
            && let Some(mut material) = materials.get_mut(handle)
        {
            material.base_color = want;
            material.alpha_mode = AlphaMode::Mask(cutoff);
        }
    }
}

fn cell_rect(key: IVec2, side: f32) -> Rect {
    let min = key.as_vec2() * side;
    Rect::from_corners(min, min + Vec2::splat(side))
}

/// Sows, meshes, shows and evicts the cells of both levels.
pub fn stream(
    mut commands: Commands,
    world: Option<Res<VegWorld>>,
    ground: GroundHeight,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<Understory>,
    mut visibilities: Query<&mut Visibility>,
) {
    let (Some(world), Some(cfg)) = (world, state.config.clone()) else {
        return;
    };
    state.frame += 1;
    if state.clear_pending {
        state.clear_pending = false;
        for level in &mut state.levels {
            for cell in level.cells.drain().map(|(_, c)| c) {
                for (_, e, _) in cell.entities {
                    commands.entity(e).despawn();
                }
            }
            level.shown = [0; CATEGORY_COUNT];
            level.sown = [0; CATEGORY_COUNT];
        }
    }
    ensure_materials(&server, &mut materials, &mut images, &cfg, &mut state);
    let Some(handles) = state.materials.clone() else {
        return;
    };
    for index in 0..2 {
        let mut level = std::mem::take(&mut state.levels[index]);
        let view = View {
            focus: state.focus,
            distance: state.distance,
            relief_version: state.relief_version,
            season: state.season,
            frame: state.frame,
        };
        stream_level(
            index,
            &cfg,
            &world.0,
            &view,
            &handles,
            &mut level,
            &mut commands,
            &mut meshes,
            &ground,
            &mut visibilities,
        );
        state.levels[index] = level;
    }
}

struct View {
    focus: Vec2,
    distance: f32,
    relief_version: u32,
    season: usize,
    frame: u64,
}

#[allow(clippy::too_many_arguments)]
fn stream_level(
    index: usize,
    cfg: &Arc<Config>,
    world: &Arc<VegetationWorld>,
    view: &View,
    handles: &[Handle<StandardMaterial>],
    level: &mut LevelState,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    ground: &GroundHeight,
    visibilities: &mut Query<&mut Visibility>,
) {
    let settings = if index == 0 {
        &cfg.far
    } else {
        &cfg.near_level
    };
    let categories = if index == 0 {
        FAR_CATEGORIES
    } else {
        NEAR_CATEGORIES
    };
    let side = settings.cell_px;
    if !level.active {
        level.cells_shown = 0;
        level.shown = [0; CATEGORY_COUNT];
        for cell in level.cells.values() {
            for (_, e, _) in &cell.entities {
                if let Ok(mut v) = visibilities.get_mut(*e) {
                    v.set_if_neq(Visibility::Hidden);
                }
            }
        }
        return;
    }
    let map_size = Vec2::new(world.size.0 as f32, world.size.1 as f32);
    let focus = view.focus;
    let radius = settings.radius(view.distance);
    let lo = ((focus - Vec2::splat(radius)) / side).floor().as_ivec2();
    let hi = ((focus + Vec2::splat(radius)) / side).floor().as_ivec2();
    let mut wanted: Vec<(f32, IVec2)> = Vec::new();
    for cy in lo.y..=hi.y {
        for cx in lo.x..=hi.x {
            let key = IVec2::new(cx, cy);
            let rect = cell_rect(key, side);
            let nearest = focus.clamp(rect.min, rect.max);
            if nearest.distance_squared(focus) > radius * radius
                || rect.max.x <= 0.0
                || rect.max.y <= 0.0
                || rect.min.x >= map_size.x
                || rect.min.y >= map_size.y
            {
                continue;
            }
            wanted.push((rect.center().distance_squared(focus), key));
        }
    }
    wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Sowing of the missing cells.
    let mut jobs = level
        .cells
        .values()
        .filter(|c| matches!(c.seeding, Seeding::Pending(_)))
        .count();
    for (_, key) in &wanted {
        if jobs >= MAX_JOBS {
            break;
        }
        if level.cells.contains_key(key) {
            continue;
        }
        let slot: Slot = Arc::default();
        let out = slot.clone();
        let (world, cfg, key_copy, season) = (world.clone(), cfg.clone(), *key, view.season);
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let pieces = sow(&world, &cfg, index, key_copy, season);
            *out.lock().expect("understory slot") =
                Some((pieces, started.elapsed().as_secs_f32() * 1000.0));
        });
        level.cells.insert(
            *key,
            UCell {
                seeding: Seeding::Pending(slot),
                entities: Vec::new(),
                seated: None,
                counts: [0; CATEGORY_COUNT],
                last_seen: view.frame,
            },
        );
        jobs += 1;
    }
    let mut seed_ms = 0.0f32;
    for cell in level.cells.values_mut() {
        let done = match &cell.seeding {
            Seeding::Pending(slot) => slot.lock().expect("understory slot").take(),
            Seeding::Sown(_) => None,
        };
        if let Some((pieces, ms)) = done {
            cell.seeding = Seeding::Sown(Arc::new(pieces));
            seed_ms = seed_ms.max(ms);
        }
    }
    level.seed_ms_max = level.seed_ms_max.max(seed_ms);
    // Budget: the nearest cells whose pieces fit.
    let counts: Vec<usize> = wanted
        .iter()
        .map(|(_, key)| match level.cells.get(key).map(|c| &c.seeding) {
            Some(Seeding::Sown(p)) => p.len(),
            _ => 0,
        })
        .collect();
    let allowed_count = cells_within_budget(&counts, level.budget);
    let allowed: HashSet<IVec2> = wanted[..allowed_count].iter().map(|w| w.1).collect();
    level.over_budget = wanted.len() - allowed_count;
    // Mesh (or re-seat) the nearest allowed cells that need it.
    let mut budget = BUILDS_PER_FRAME;
    if ground.is_ready() {
        for (_, key) in wanted.iter().take(allowed_count) {
            if budget == 0 {
                break;
            }
            let Some(cell) = level.cells.get_mut(key) else {
                continue;
            };
            let Seeding::Sown(pieces) = &cell.seeding else {
                continue;
            };
            if cell.seated == Some(view.relief_version) {
                continue;
            }
            let pieces = pieces.clone();
            let started = std::time::Instant::now();
            let origin = cell_rect(*key, side).min;
            let heights: Vec<f32> = pieces
                .iter()
                .map(|p| ground.at(p.pos.x, p.pos.y).unwrap_or(0.0))
                .collect();
            let first = cell.seated.is_none();
            for category in categories.clone() {
                let (data, count) = build_mesh_data(
                    &cfg.kinds[category],
                    category as u8,
                    &pieces,
                    &heights,
                    origin,
                );
                cell.counts[category] = count;
                let existing = cell.entities.iter().position(|(c, _, _)| *c == category);
                if count == 0 {
                    if let Some(i) = existing {
                        let (_, entity, _) = cell.entities.remove(i);
                        commands.entity(entity).despawn();
                    }
                    continue;
                }
                let mesh = data.to_mesh();
                if let Some(i) = existing {
                    let (_, entity, handle) = &cell.entities[i];
                    meshes.insert(handle, mesh).ok();
                    commands
                        .entity(*entity)
                        .remove::<bevy::camera::primitives::Aabb>();
                } else {
                    let handle = meshes.add(mesh);
                    let entity = commands
                        .spawn((
                            Name::new(format!("understory:{}", CATEGORIES[category])),
                            Mesh3d(handle.clone()),
                            MeshMaterial3d(handles[category].clone()),
                            Transform::from_xyz(origin.x, 0.0, origin.y),
                            Visibility::Hidden,
                            bevy::light::NotShadowCaster,
                            DespawnOnExit(AppState::Campaign),
                        ))
                        .id();
                    cell.entities.push((category, entity, handle));
                }
            }
            cell.seated = Some(view.relief_version);
            level.build_ms_max = level
                .build_ms_max
                .max(started.elapsed().as_secs_f32() * 1000.0);
            if first {
                level.built += 1;
            } else {
                level.reseated += 1;
            }
            budget -= 1;
        }
    }
    // Show the allowed cells, touch the wanted ones.
    let wanted_set: HashSet<IVec2> = wanted.iter().map(|w| w.1).collect();
    let mut shown = [0usize; CATEGORY_COUNT];
    let mut cells_shown = 0;
    for (key, cell) in level.cells.iter_mut() {
        if wanted_set.contains(key) {
            cell.last_seen = view.frame;
        }
        let visible = allowed.contains(key) && cell.seated.is_some();
        if visible {
            cells_shown += 1;
            for (total, n) in shown.iter_mut().zip(cell.counts) {
                *total += n;
            }
        }
        for (_, e, _) in &cell.entities {
            if let Ok(mut v) = visibilities.get_mut(*e) {
                v.set_if_neq(if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
    }
    level.shown = shown;
    level.cells_shown = cells_shown;
    // Evict the oldest cells beyond the cache size.
    while level.cells.len() > settings.max_cells {
        let Some((&oldest, _)) = level
            .cells
            .iter()
            .filter(|(_, c)| c.last_seen != view.frame)
            .min_by_key(|(_, c)| c.last_seen)
        else {
            break;
        };
        if let Some(cell) = level.cells.remove(&oldest) {
            for (_, e, _) in cell.entities {
                commands.entity(e).despawn();
            }
            level.evicted += 1;
        }
    }
    // Pieces sown per category.
    let mut sown = [0usize; CATEGORY_COUNT];
    for cell in level.cells.values() {
        if let Seeding::Sown(pieces) = &cell.seeding {
            for p in pieces.iter() {
                sown[p.category as usize] += 1;
            }
        }
    }
    level.sown = sown;
}

/// Sowing of one cell (worker thread).
fn sow(
    world: &VegetationWorld,
    cfg: &Config,
    level: usize,
    key: IVec2,
    season: usize,
) -> Vec<Piece> {
    let side = if level == 0 {
        cfg.far.cell_px
    } else {
        cfg.near_level.cell_px
    };
    let low = key.as_vec2() * side;
    let pad = Vec2::splat(cfg.belt.width_px);
    let env = WorldEnv {
        world,
        cover: Cover::new(world, low - pad, low + Vec2::splat(side) + pad),
        cfg,
        season,
    };
    if level == 0 {
        seed_far(cfg, &env, key)
    } else {
        let frequency = cfg.near.flower_patch_frequency;
        let flower = move |p: Vec2| fbm(p.x, p.y, 5051, frequency, 2, 0.5);
        seed_near(cfg, &env, &flower, key)
    }
}

pub fn reset(mut state: ResMut<Understory>) {
    // Entities go with the campaign state; the settings and the materials are kept.
    let kept = Understory {
        config: state.config.take(),
        config_tried: state.config_tried,
        materials: state.materials.take(),
        mips_done: state.mips_done,
        ..default()
    };
    *state = kept;
}

/// Counts for the `clutter` section of the state dump.
pub fn summary(world: &World) -> Value {
    let Some(state) = world.get_resource::<Understory>() else {
        return Value::Null;
    };
    let level = |index: usize, name: &str| {
        let l = &state.levels[index];
        let categories = if index == 0 {
            FAR_CATEGORIES
        } else {
            NEAR_CATEGORIES
        };
        let counts: serde_json::Map<String, Value> = categories
            .map(|c| {
                (
                    CATEGORIES[c].to_string(),
                    json!({"sown": l.sown[c], "shown": l.shown[c]}),
                )
            })
            .collect();
        (
            name.to_string(),
            json!({
                "active": l.active,
                "fade": l.fade,
                "budget": l.budget,
                "cells_cached": l.cells.len(),
                "cells_shown": l.cells_shown,
                "cells_over_budget": l.over_budget,
                "cells_built": l.built,
                "cells_reseated": l.reseated,
                "cells_evicted": l.evicted,
                "pieces": counts,
                "build_ms_max": l.build_ms_max,
                "seed_ms_max": l.seed_ms_max,
            }),
        )
    };
    let mut out = serde_json::Map::new();
    let (k, v) = level(0, "far");
    out.insert(k, v);
    let (k, v) = level(1, "near");
    out.insert(k, v);
    out.insert("season".into(), json!(state.season));
    out.insert("quality".into(), json!(state.quality));
    out.insert("loaded".into(), json!(state.config.is_some()));
    Value::Object(out)
}
