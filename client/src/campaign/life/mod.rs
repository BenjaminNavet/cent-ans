//! Living map: folk, carts and convoys, mills, birds, boats, fauna, smoke and fire.
//!
//! Ports (Godot `game/scripts/map/`): `campaign_life.gd`, `life_ambient.gd` (boats, birds),
//! `life_effects.gd` (chimney plumes, windmills, fires), `map_bird_flocks.gd` (birds),
//! `life_folk/` (road folk, carts, convoys; ADR 0354 fixed world size, no pop), `fauna_layer.gd`,
//! `dn_water_models.gd` (boat models), `map_fire_wind.gd` (fire and wind data).
//! See `docs/wip/bm-life.md` for the subset and what remains. Heights come from
//! `campaign::terrain::surface::GroundHeight`; every number is read from `data/` (`data.rs`).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod birds;
mod boats;
mod data;
mod effects;
mod fauna;
mod figures;
mod folk;
mod incidents;
mod models;
mod motion;
mod props;
mod scene_layers;
mod scenes;
mod stream;

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::paths;

pub use data::LifeData;
pub use stream::{Cat, LifeView};

/// Live counts per category (refreshed twice a second).
#[derive(Resource, Default)]
pub struct LifeStats {
    pub live: BTreeMap<&'static str, usize>,
    pub visible: BTreeMap<&'static str, usize>,
    pub beasts: usize,
    pub bird_individuals: usize,
    pub bird_glb: usize,
    pub load_ms: u128,
    pub figures: usize,
    pub figures_fallback: usize,
    pub baked_meshes: usize,
    pub scene_figures: usize,
    pub scene_props: usize,
    pub fauna_lods: [usize; 3],
    pub figure_roles: BTreeMap<String, usize>,
    pub scene_roots: BTreeMap<String, usize>,
    pub figures_available: bool,
    pub beasts_bobbing: usize,
}

pub struct CampaignLifePlugin;

impl Plugin for CampaignLifePlugin {
    fn build(&self, app: &mut App) {
        incidents::plugin(app);
        app.init_resource::<stream::LifeView>()
            .init_resource::<stream::LifeIndex>()
            .init_resource::<models::ModelMetrics>()
            .init_resource::<effects::EffectSites>()
            .init_resource::<folk::FolkIndex>()
            .init_resource::<scenes::StagedScenes>()
            .init_resource::<LifeStats>()
            .register_dump(dump_life)
            .add_systems(
                OnEnter(AppState::Campaign),
                (
                    load_data,
                    (
                        birds::build_assets,
                        effects::build_assets,
                        effects::spawn_fire_lights,
                    ),
                )
                    .chain(),
            )
            .add_systems(OnExit(AppState::Campaign), reset)
            .add_systems(
                Update,
                (
                    stream::update_view,
                    scenes::refresh_scenes,
                    effects::refresh_sites,
                    folk::build_index,
                    (
                        effects::spawn_sites,
                        folk::spawn_walkers,
                        folk::spawn_carts,
                        scenes::spawn_scenes,
                        fauna::spawn_herds,
                        boats::spawn_boats,
                        birds::manage_flocks,
                    ),
                    stream::update_targets,
                    scenes::scene_targets,
                    effects::retire_stale_plumes,
                    (
                        folk::move_walkers,
                        scenes::move_scene_movers,
                        motion::animate_carts,
                        folk::move_carts,
                        fauna::update_lods,
                        fauna::move_animals,
                        boats::move_boats,
                        birds::move_birds,
                        effects::animate_plumes,
                        effects::animate_flames,
                        effects::spin_sails,
                        effects::update_fire_lights,
                        stream::ground_statics,
                    ),
                    (models::fit_models, models::apply_fades, update_stats),
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<LifeData>)
                    .run_if(resource_exists::<figures::FigureLibrary>)
                    .run_if(resource_exists::<props::PropGeometry>)
                    .run_if(resource_exists::<motion::MotionTable>)
                    .run_if(resource_exists::<birds::BirdAssets>)
                    .run_if(resource_exists::<effects::EffectAssets>),
            );
    }
}

/// Reads `data/` once (kept across campaigns: the data does not change).
fn load_data(mut commands: Commands, existing: Option<Res<LifeData>>) {
    let Ok(root) = paths::find_root() else {
        warn!("life: repository root not found");
        return;
    };
    let assets = paths::asset_root(&root);
    commands.insert_resource(fauna::AssetRoot(assets.clone()));
    if existing.is_some() {
        return;
    }
    let started = std::time::Instant::now();
    let data = LifeData::load(&root);
    commands.insert_resource(motion::MotionTable::load(&paths::data_dir(&root)));
    commands.insert_resource(props::PropGeometry::build(&data.table, &assets));
    commands.insert_resource(figures::FigureLibrary::new(assets));
    let ms = started.elapsed().as_millis();
    info!(
        "life: data loaded in {ms} ms ({} road pieces, {} boat routes)",
        data.roads.len(),
        data.river_routes.len()
    );
    commands.insert_resource(LifeStats {
        load_ms: ms,
        ..default()
    });
    commands.insert_resource(data);
}

fn reset(
    mut index: ResMut<stream::LifeIndex>,
    mut sites: ResMut<effects::EffectSites>,
    mut view: ResMut<stream::LifeView>,
) {
    index.0.clear();
    *sites = effects::EffectSites::default();
    view.ready = false;
}

fn update_stats(
    time: Res<Time>,
    mut timer: Local<f32>,
    mut stats: ResMut<LifeStats>,
    items: Query<(&stream::LifeItem, &models::Fade)>,
    beasts: Query<(), With<folk::Beast>>,
    individuals: Query<(), With<birds::Bird>>,
    glb_birds: Query<(), With<birds::ModelBird>>,
    figures: Query<&figures::Figure>,
    library: Option<Res<figures::FigureLibrary>>,
    scene_items: Query<&scenes::SceneItem>,
    scene_roots: Query<&scenes::SceneRoot>,
    animals: Query<&fauna::Animal>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.5;
    stats.live.clear();
    stats.visible.clear();
    for cat in Cat::ALL {
        stats.live.insert(cat.name(), 0);
        stats.visible.insert(cat.name(), 0);
    }
    for (item, fade) in &items {
        *stats.live.entry(item.cat.name()).or_default() += 1;
        if fade.current > 0.5 {
            *stats.visible.entry(item.cat.name()).or_default() += 1;
        }
    }
    stats.beasts = beasts.iter().count();
    stats.bird_individuals = individuals.iter().count();
    stats.bird_glb = glb_birds.iter().count();
    stats.figures = figures.iter().count();
    stats.figures_fallback = figures.iter().filter(|f| f.fallback).count();
    stats.baked_meshes = library.as_ref().map_or(0, |l| l.baked());
    stats.figures_available = library.is_some_and(|l| l.available());
    stats.figure_roles.clear();
    for f in &figures {
        *stats.figure_roles.entry(f.role.clone()).or_default() += 1;
    }
    stats.scene_roots.clear();
    for r in &scene_roots {
        *stats.scene_roots.entry(r.kind.clone()).or_default() += r.items;
    }
    stats.scene_figures = scene_items.iter().filter(|i| i.figure).count();
    stats.scene_props = scene_items.iter().filter(|i| !i.figure).count();
    stats.fauna_lods = [0; 3];
    for a in &animals {
        stats.fauna_lods[(a.lod as usize).min(2)] += 1;
    }
}

fn dump_life(world: &World) -> (String, Value) {
    let stats = world.resource::<LifeStats>();
    let view = world.resource::<stream::LifeView>();
    let data = world.get_resource::<LifeData>();
    let places = world.resource::<crate::campaign::markers::places::Places>();
    let sites = world.resource::<effects::EffectSites>();
    let staged = world.resource::<scenes::StagedScenes>();
    let incident_stats = world.resource::<incidents::IncidentStats>();
    let mut kinds = [0usize; 3];
    for s in &sites.sites {
        match s.kind {
            effects::SiteKind::Plume { .. } => kinds[0] += 1,
            effects::SiteKind::Mill { .. } => kinds[1] += 1,
            effects::SiteKind::Fire { .. } => kinds[2] += 1,
        }
    }
    let data_json = data.map_or(Value::Null, |d| {
        let sea = d.river_routes.iter().filter(|r| !r.river).count();
        json!({
            "road_pieces": d.roads.len(),
            "road_runs": d.road_runs.len(),
            "river_boats": d.river_routes.len() - sea,
            "sea_ships": sea,
            "bird_species": d.birds.species.len(),
            "bird_species_glb": d.birds.species.iter().filter(|s| d.water_models.scene_path(&s.dn_model).is_some()).count(),
            "bird_glb_ids": d.birds.species.iter().filter(|s| !s.dn_model.is_empty()).map(|s| s.dn_model.clone()).collect::<Vec<_>>(),
            "fauna_species": d.fauna.species.len(),
            "fauna_zones": d.fauna.zones.len(),
            "hamlets": d.hamlets.len(),
            "wetlands": d.wetlands.len(),
            "land_mask": d.land.is_some(),
            "fire_enabled": d.fire.enabled,
            "wind": [d.fire.wind_dir.x, d.fire.wind_dir.y],
            "folk_world_scale": d.props.world_scale(d.meters_per_px),
            "props": d.table.props.len(),
            "roles": d.table.roles.len(),
            "rivers": d.rivers.len(),
            "scene_kinds": d.scene_layouts.as_object().map_or(0, |o| o.len()),
        })
    });
    (
        "campaign_life".into(),
        json!({
            "data": data_json,
            "prop_geometry": world.get_resource::<props::PropGeometry>().map(|g| json!({
                "measured": g.bounds.len(),
                "with_slots": g.slots.len(),
                "teams": g.teams.iter().map(|(k, v)| (k.clone(), v.len())).collect::<BTreeMap<_, _>>(),
            })),
            "motion": world.get_resource::<motion::MotionTable>().map(|t| json!({
                "enabled": t.enabled,
                "models": t.models.len(),
            })),
            "load_ms": stats.load_ms,
            "places": places.places.len(),
            "sites": { "plumes": kinds[0], "mills": kinds[1], "fires": kinds[2] },
            "live": stats.live,
            "visible": stats.visible,
            "draught_beasts": stats.beasts,
            "figures": {
                "total": stats.figures,
                "capsule_fallback": stats.figures_fallback,
                "baked_meshes": stats.baked_meshes,
                "battle_figures_readable": stats.figures_available,
                "by_role": stats.figure_roles,
            },
            "scenes": {
                "staged": staged.scenes.iter().map(|s| json!({
                    "kind": s.kind,
                    "settlement": s.settlement,
                    "province": s.province,
                    "intensity": s.intensity,
                })).collect::<Vec<_>>(),
                "skipped": staged.skipped,
                "quiet_settlements": staged.quiet_settlements.len(),
                "fire_points": staged.fire_points.len(),
                "idle_provinces": staged.idle_provinces.len(),
                "figures": stats.scene_figures,
                "props": stats.scene_props,
                "live_items_by_kind": stats.scene_roots,
            },
            "incidents": {
                "seals": incident_stats.seals,
                "scene_hits": incident_stats.scene_hits,
                "visible_seals": incident_stats.visible_seals,
                "clicks": incident_stats.clicks,
            },
            "fauna_lods": stats.fauna_lods,
            "bird_individuals": stats.bird_individuals,
            "bird_glb_individuals": stats.bird_glb,
            "view": {
                "ready": view.ready,
                "focus": [view.focus.x, view.focus.y],
                "distance": view.distance,
                "radius": view.radius,
                "keep": view.keep,
                "season": view.season,
            },
        }),
    )
}
