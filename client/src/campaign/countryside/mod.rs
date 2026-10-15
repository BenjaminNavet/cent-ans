//! Countryside dressing: countryside layer, ground clutter, rock outcrops, decor, war scars, water props, bridges.
//!
//! Ports (Godot `game/scripts/map/`): `countryside_layer.gd` (+ `cell_fade.gd`, `map_prop_scale.gd`
//! multipliers of the maquette style): `config.rs`, `placement.rs`, `terrain_data.rs`, `layer.rs`.
//! See `docs/wip/bm-countryside.md` for the subset and what remains.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod bridge_meshes;
pub mod bridges;
pub mod clutter;
pub mod config;
pub mod decor;
pub mod decor_plan;
pub mod layer;
pub mod placement;
pub mod proto;
pub mod rocks;
pub mod scar_meshes;
pub mod scars;
pub mod terrain_data;
pub mod water_props;

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::life::LifeData;
use crate::campaign::markers::places::Places;
use crate::campaign::provinces::projection::MapProjection;
use crate::campaign::terrain::TerrainWorld;
use crate::paths;

use self::config::Config;
use self::terrain_data::{Inputs, MapTerrain};

/// Loaded countryside data (kept between campaign visits).
#[derive(Resource, Clone)]
pub struct LoadedCountryside {
    pub config: Arc<Config>,
    pub terrain: Arc<MapTerrain>,
    /// Model files per prop and level of detail (`models/dn/<model>_lod<n>.glb`), a missing
    /// level falls back to its nearest neighbour.
    pub paths: Arc<Vec<[Option<String>; 3]>>,
    pub load_ms: f32,
}

#[derive(Resource)]
struct PendingLoad(Arc<Mutex<Option<Result<LoadedCountryside, String>>>>);

#[derive(Resource, Default)]
struct LoadErrors(Vec<String>);

pub struct CampaignCountrysidePlugin;

impl Plugin for CampaignCountrysidePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<layer::PropView>()
            .init_resource::<layer::Cells>()
            .init_resource::<layer::Measures>()
            .init_resource::<proto::Prototypes>()
            .init_resource::<LoadErrors>()
            .init_resource::<bridges::Bridges>()
            .init_resource::<decor::DecorView>()
            .init_resource::<scars::ScarCfg>()
            .init_resource::<scars::Scars>()
            .register_dump(dump_countryside)
            .register_dump(scars::dump)
            .register_dump(decor::dump)
            .register_dump(water_props::dump)
            .register_dump(rocks::dump)
            .register_dump(clutter::dump)
            .init_resource::<clutter::Clutter>()
            .init_resource::<rocks::RockView>()
            .init_resource::<water_props::WaterView>()
            .add_systems(
                OnExit(AppState::Campaign),
                (
                    reset,
                    proto::reset,
                    clutter::reset,
                    scars::reset,
                    decor::reset,
                    water_props::reset,
                    rocks::reset,
                ),
            )
            .add_systems(
                Update,
                (
                    start_load
                        .run_if(resource_exists::<TerrainWorld>)
                        .run_if(resource_exists::<LifeData>)
                        .run_if(not(resource_exists::<LoadedCountryside>))
                        .run_if(not(resource_exists::<PendingLoad>)),
                    finish_load.run_if(resource_exists::<PendingLoad>),
                )
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(
                Update,
                (
                    bridges::load_crossings,
                    bridges::spawn_bridges,
                    bridges::ground_bridges,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<crate::campaign::terrain::surface::Surface>),
            )
            .add_systems(
                Update,
                (
                    decor::start_plan
                        .run_if(not(resource_exists::<decor::DecorPlan>))
                        .run_if(not(resource_exists::<decor::PendingPlan>))
                        .run_if(resource_exists::<crate::campaign::markers::places::Places>),
                    decor::finish_plan.run_if(resource_exists::<decor::PendingPlan>),
                )
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<LoadedCountryside>),
            )
            .add_systems(
                Update,
                (
                    water_props::start_plan
                        .run_if(not(resource_exists::<water_props::WaterPlan>))
                        .run_if(not(resource_exists::<water_props::PendingWaterPlan>))
                        .run_if(resource_exists::<crate::campaign::markers::places::Places>)
                        .run_if(resource_exists::<LifeData>),
                    water_props::finish_plan
                        .run_if(resource_exists::<water_props::PendingWaterPlan>),
                )
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(
                Update,
                (
                    clutter::update_view,
                    clutter::stream_clutter,
                    clutter::apply_season,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<LoadedCountryside>),
            )
            .add_systems(
                Update,
                (
                    rocks::build_plan.run_if(not(resource_exists::<rocks::RockPlan>)),
                    (rocks::stream_rocks, rocks::apply_rocks)
                        .chain()
                        .run_if(resource_exists::<rocks::RockPlan>),
                )
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<LoadedCountryside>),
            )
            .add_systems(
                Update,
                (
                    water_props::stream_props,
                    water_props::fit_models,
                    water_props::apply_props,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<water_props::WaterPlan>),
            )
            .add_systems(
                Update,
                (decor::stream_decor, decor::apply_decor)
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<decor::DecorPlan>),
            )
            .add_systems(
                Update,
                (scars::load_cfg, scars::sync_scars, scars::animate_scars)
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<crate::campaign::markers::places::Places>)
                    .run_if(resource_exists::<crate::campaign::terrain::surface::Surface>),
            )
            .add_systems(
                Update,
                (
                    proto::capture_prototypes,
                    layer::update_view,
                    layer::stream_cells,
                    layer::swap_lods,
                    layer::apply_view,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<LoadedCountryside>),
            );
    }
}

fn read_json(path: &std::path::Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn start_load(
    mut commands: Commands,
    terrain: Res<TerrainWorld>,
    life: Res<LifeData>,
    places: Res<Places>,
) {
    if places.places.is_empty() {
        return;
    }
    let Ok(root) = paths::find_root() else {
        return;
    };
    let slot = Arc::new(Mutex::new(None));
    let out = slot.clone();
    let heights = terrain.heights.clone();
    let life = life.clone();
    let places: Vec<(Vec2, String)> = places
        .places
        .iter()
        .map(|p| (p.px, p.kind.clone()))
        .collect();
    std::thread::spawn(move || {
        let result = load(root, heights, life, places);
        *out.lock().expect("countryside load slot") = Some(result);
    });
    commands.insert_resource(PendingLoad(slot));
}

fn load(
    root: std::path::PathBuf,
    heights: Arc<crate::campaign::terrain::relief_floor::HeightGrid>,
    life: LifeData,
    places: Vec<(Vec2, String)>,
) -> Result<LoadedCountryside, String> {
    let started = std::time::Instant::now();
    let data = root.join("data");
    let doc = read_json(&data.join("map/map_countryside.json"))
        .ok_or("map/map_countryside.json missing or invalid")?;
    let projection = MapProjection::load(&data.join("map"))?;
    let maquette = read_json(&data.join("art/town_maquettes.json"));
    let config = config::parse(&doc, &data, &projection)
        .with_maquette(maquette.as_ref().map_or(&Value::Null, |m| &m["props"]));
    let assets = paths::asset_root(&root);
    let paths: Vec<[Option<String>; 3]> = config
        .props
        .iter()
        .map(|p| {
            let found: Vec<Option<String>> = (0..3)
                .map(|l| {
                    let path = format!("models/dn/{}_lod{l}.glb", p.model);
                    assets.join(&path).exists().then_some(path)
                })
                .collect();
            std::array::from_fn(|l| {
                found[l]
                    .clone()
                    .or_else(|| (0..l).rev().find_map(|k| found[k].clone()))
                    .or_else(|| (l + 1..3).find_map(|k| found[k].clone()))
            })
        })
        .collect();
    let terrain = MapTerrain::load(Inputs {
        root,
        heights,
        life,
        places,
        cell_px: config.render.cell_px,
    });
    Ok(LoadedCountryside {
        config: Arc::new(config),
        terrain: Arc::new(terrain),
        paths: Arc::new(paths),
        load_ms: started.elapsed().as_secs_f32() * 1000.0,
    })
}

fn finish_load(mut commands: Commands, pending: Res<PendingLoad>, mut errors: ResMut<LoadErrors>) {
    let Some(result) = pending.0.lock().expect("countryside load slot").take() else {
        return;
    };
    commands.remove_resource::<PendingLoad>();
    match result {
        Ok(loaded) => {
            info!(
                "countryside: {} props, {} rules, {} towns/hamlets, {} road segments in {:.0} ms",
                loaded.config.props.len(),
                loaded.config.rules.len(),
                loaded.terrain.town_count,
                loaded.terrain.road_segments,
                loaded.load_ms
            );
            commands.insert_resource(loaded);
        }
        Err(e) => {
            warn!("countryside: {e}");
            errors.0.push(e);
        }
    }
}

fn reset(mut cells: ResMut<layer::Cells>, mut view: ResMut<layer::PropView>) {
    cells.cells.clear();
    cells.staged.clear();
    view.active = false;
    view.lod = None;
}

fn dump_countryside(world: &World) -> (String, Value) {
    let loaded = world.get_resource::<LoadedCountryside>();
    let cells = world.resource::<layer::Cells>();
    let view = world.resource::<layer::PropView>();
    let measures = world.resource::<layer::Measures>();
    let errors = world.resource::<LoadErrors>();
    let bridges = world.resource::<bridges::Bridges>();
    let protos = world.resource::<proto::Prototypes>();
    let data = loaded.map_or(Value::Null, |l| {
        json!({
            "props": l.config.props.len(),
            "rules": l.config.rules.len(),
            "towns_and_hamlets": l.terrain.town_count,
            "villages_indexed": l.terrain.village_count,
            "road_segments": l.terrain.road_segments,
            "rasters": l.terrain.rasters.iter().map(|(n, ok)| json!({"name": n, "loaded": ok})).collect::<Vec<_>>(),
            "models_resolved": l.paths.iter().filter(|p| p[0].is_some() || p[1].is_some() || p[2].is_some()).count(),
            "maquette": [l.config.maquette_size, l.config.maquette_range, l.config.fixed_mult],
            "outer_distance": l.config.outer_distance(),
            "load_ms": l.load_ms,
        })
    });
    let mut per_prop: std::collections::BTreeMap<String, usize> = Default::default();
    if let Some(l) = loaded {
        for cell in cells.cells.values() {
            for (p, n) in &cell.counts {
                *per_prop.entry(l.config.props[*p].id.clone()).or_default() += n;
            }
        }
    }
    (
        "countryside".into(),
        json!({
            "data": data,
            "bridges": {
                "crossings": bridges.crossings.len(),
                "spawned": bridges.spawned,
                "visible": bridges.visible,
                "relief_version": bridges.relief_version,
                "meshes_built": bridges.meshes_built,
                "by_structure": bridges.by_structure,
            },
            "errors": errors.0,
            "view": {
                "active": view.active,
                "focus": [view.focus.x, view.focus.y],
                "distance": view.distance,
                "lod": view.lod,
                "season": view.season,
            },
            "cells": {
                "cached": cells.cells.len(),
                "shown": cells.shown_cells,
                "installed": cells.installed,
                "evicted": cells.evicted,
                "radius": cells.radius,
                "build_ms_max": cells.build_ms_max,
            },
            "prototypes": {
                "ready": protos.ready_count(),
                "loading": protos.loading_count(),
                "failed": protos.failed_count(),
            },
            "cells_waiting_models": cells.waiting_models,
            "lod_swaps": cells.swapped,
            "max_entities": layer::MAX_ENTITIES,
            "instances": cells.cells.values().map(|c| c.count).sum::<usize>(),
            "visible": cells.visible,
            "models_measured": measures.0.len(),
            "per_prop": per_prop,
        }),
    )
}
