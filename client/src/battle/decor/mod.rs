//! Battle decor: textured ground, trees, grass, site elements, bridges, atmosphere.
//!
//! Ports game/scripts/battle/battle_terrain_splat.gd + battle_ground_textures.gd (`ground`),
//! battle_trees.gd / battle_terrain_scatter.gd (`trees`), battle_vegetation.gd (`grass`),
//! battle_decor.gd + battle_bridges.gd (`site`), battle_atmosphere.gd + battle_time_of_day.gd
//! (`atmosphere`). Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! Dump section `battle_decor`: sun / fog / time of day, ground pack, counts of trees, grass tufts,
//! houses, props, bridges.

pub mod atmosphere;
pub mod blood;
pub mod camp_loot;
pub mod features;
pub mod flatten;
pub mod grass;
pub mod ground;
pub mod horizon;
pub mod kit_site;
pub mod look;
pub mod site;
pub mod trees;
pub mod wind;

use bevy::prelude::*;

use crate::agent::dump::DumpAppExt;

pub struct BattleDecorPlugin;

impl Plugin for BattleDecorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(look::DecorLook::load())
            .add_plugins((
                wind::WindPlugin,
                horizon::HorizonPlugin,
                ground::GroundPlugin,
                atmosphere::AtmospherePlugin,
                trees::TreesPlugin,
                grass::GrassPlugin,
                features::FeaturesPlugin,
                flatten::FlattenPlugin,
                blood::BloodPlugin,
                site::SitePlugin,
                camp_loot::CampLootPlugin,
            ))
            .register_dump(dump_decor);
    }
}

/// Hash of a position into `0..1` (deterministic scatter).
pub fn hash01(x: f32, z: f32, salt: f32) -> f32 {
    let n = (x * 12.9898 + z * 78.233 + salt * 37.719).sin() * 43758.547;
    n - n.floor()
}

fn dump_decor(world: &World) -> (String, serde_json::Value) {
    (
        "battle_decor".to_owned(),
        serde_json::json!({
            "atmosphere": world.get_resource::<atmosphere::AtmosphereState>().map(|a| serde_json::json!({
                "time_key": a.time_key, "phase": a.phase, "weather": a.weather,
                "fog_density": a.fog_density, "sun_illuminance": a.sun_illuminance,
                "sun_elevation_deg": a.sun_elevation_deg, "sun_azimuth_deg": a.sun_azimuth_deg,
                "sun_direction": a.sun_direction, "fog_on_camera": a.fog_on_camera,
            })),
            "ground": world.get_resource::<ground::GroundStats>().map(|g| serde_json::json!({
                "biome": g.biome, "pack_loaded": g.pack_loaded,
                "vertices": g.vertices, "splat_share": g.splat_share,
                "arrays_converted": g.arrays_converted, "mip_levels": g.mip_levels,
            })),
            "trees": world.get_resource::<trees::TreeStats>().map(|s| serde_json::json!({
                "trees": s.trees, "bushes": s.bushes, "species": s.species, "zones": s.zones, "ring_trees": s.ring_trees,
            })),
            "grass": world.get_resource::<grass::GrassStats>().map(|s| serde_json::json!({
                "tufts": s.tufts, "chunks": s.chunks, "group": s.group, "biome": s.biome,
                "tufts_by_card": s.tufts_by_card,
            })),
            "flatten": world.get_resource::<flatten::FlattenMap>().map(|m| serde_json::json!({
                "size": [m.width, m.height], "texel": m.texel, "corpse_marks": m.corpse_marks,
                "uploads": m.uploads, "flattened_texels": m.flattened_texels(),
                "bloody_texels": m.bloody_texels(),
            })),
            "features": world.get_resource::<features::FeatureStats>().map(|f| serde_json::json!({
                "fires": f.fires, "smoking_fires": f.smoking_fires, "moats": f.moats,
                "sea": f.sea, "sea_area_m2": f.sea_area_m2,
            })),
            "blood": world.get_resource::<blood::BloodDecals>().map(|b| serde_json::json!({
                "decals": b.decals.len(), "added": b.added, "rebuilds": b.rebuilds,
            })),
            "horizon": world.get_resource::<horizon::HorizonStats>().map(|h| serde_json::json!({
                "active": h.active, "province": h.province, "real_relief": h.real_relief,
                "offset_y": h.offset_y, "rotation_deg": h.rotation_deg,
                "ring_vertices": h.ring_vertices, "ring_triangles": h.ring_triangles,
                "ring_max_height": h.ring_max_height, "skyline_segments": h.skyline_segments,
                "sky_vertices": h.sky_vertices,
            })),
            "site": world.get_resource::<site::SiteStats>().map(|s| serde_json::json!({
                "vine_rows": s.vine_rows, "vineyards": s.vineyards, "houses": s.houses, "props": s.props, "bridges": s.bridges, "bridge_bays": s.bridge_bays, "bridge_ends": s.bridge_ends, "site_layered_vertices": s.layered_vertices, "site_plain_vertices": s.plain_vertices, "hedges": s.hedges,
                "camps": s.camps, "kit_houses": s.kit_houses, "kit_props": s.kit_props,
                "wall_runs": s.wall_runs, "lychgates": s.lychgates, "horses": s.horses, "kit_models": s.kit_models, "camp_tents": s.camp_tents,
            })),
            "camp_loot": world.get_resource::<camp_loot::CampLootStats>().map(|s| serde_json::json!({
                "looted_camps": s.looted_camps, "tents_down": s.tents_down, "smoke_columns": s.smoke_columns,
            })),
        }),
    )
}
