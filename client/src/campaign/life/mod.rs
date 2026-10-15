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
mod folk;
mod models;
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
    pub load_ms: u128,
}

pub struct CampaignLifePlugin;

impl Plugin for CampaignLifePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<stream::LifeView>()
            .init_resource::<stream::LifeIndex>()
            .init_resource::<models::ModelMetrics>()
            .init_resource::<effects::EffectSites>()
            .init_resource::<folk::FolkIndex>()
            .init_resource::<LifeStats>()
            .register_dump(dump_life)
            .add_systems(
                OnEnter(AppState::Campaign),
                (
                    load_data,
                    (
                        birds::build_assets,
                        effects::build_assets,
                        folk::build_assets,
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
                    effects::refresh_sites,
                    folk::build_index,
                    (
                        effects::spawn_sites,
                        folk::spawn_walkers,
                        folk::spawn_carts,
                        fauna::spawn_herds,
                        boats::spawn_boats,
                        birds::manage_flocks,
                    ),
                    stream::update_targets,
                    (
                        folk::move_walkers,
                        folk::move_carts,
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
    commands.insert_resource(fauna::AssetRoot(paths::asset_root(&root)));
    if existing.is_some() {
        return;
    }
    let started = std::time::Instant::now();
    let data = LifeData::load(&root);
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
}

fn dump_life(world: &World) -> (String, Value) {
    let stats = world.resource::<LifeStats>();
    let view = world.resource::<stream::LifeView>();
    let data = world.get_resource::<LifeData>();
    let places = world.resource::<crate::campaign::markers::places::Places>();
    let sites = world.resource::<effects::EffectSites>();
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
            "fauna_species": d.fauna.species.len(),
            "fauna_zones": d.fauna.zones.len(),
            "hamlets": d.hamlets.len(),
            "wetlands": d.wetlands.len(),
            "land_mask": d.land.is_some(),
            "fire_enabled": d.fire.enabled,
            "wind": [d.fire.wind_dir.x, d.fire.wind_dir.y],
            "folk_world_scale": d.props.world_scale(d.meters_per_px),
        })
    });
    (
        "campaign_life".into(),
        json!({
            "data": data_json,
            "load_ms": stats.load_ms,
            "places": places.places.len(),
            "sites": { "plumes": kinds[0], "mills": kinds[1], "fires": kinds[2] },
            "live": stats.live,
            "visible": stats.visible,
            "draught_beasts": stats.beasts,
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
