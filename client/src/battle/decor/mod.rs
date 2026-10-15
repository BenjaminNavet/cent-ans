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
pub mod grass;
pub mod ground;
pub mod site;
pub mod trees;

use bevy::prelude::*;

use crate::agent::dump::DumpAppExt;

pub struct BattleDecorPlugin;

impl Plugin for BattleDecorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ground::GroundPlugin,
            atmosphere::AtmospherePlugin,
            trees::TreesPlugin,
            grass::GrassPlugin,
            site::SitePlugin,
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
            })),
            "trees": world.get_resource::<trees::TreeStats>().map(|s| serde_json::json!({
                "trees": s.trees, "bushes": s.bushes, "species": s.species, "zones": s.zones,
            })),
            "grass": world.get_resource::<grass::GrassStats>().map(|s| serde_json::json!({
                "tufts": s.tufts, "chunks": s.chunks,
            })),
            "site": world.get_resource::<site::SiteStats>().map(|s| serde_json::json!({
                "houses": s.houses, "props": s.props, "bridges": s.bridges, "hedges": s.hedges,
                "camps": s.camps,
            })),
        }),
    )
}
