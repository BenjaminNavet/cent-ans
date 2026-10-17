//! Dump section `perf_probe` (hunting frame-time drift): rolling frame time, live entity count,
//! the busiest archetypes (entity count, component names) and the size of the main asset
//! collections. Cheap enough to leave on: one `Time<Real>` read per frame, the rest only when a
//! dump is taken.

use std::collections::VecDeque;

use bevy::prelude::*;
use serde_json::{Value, json};

use super::dump::DumpAppExt;

/// Frames kept for the rolling statistics.
const WINDOW: usize = 60;
/// Archetypes listed in the dump.
const TOP_ARCHETYPES: usize = 14;

#[derive(Resource, Default)]
pub struct FrameWindow {
    ms: VecDeque<f32>,
}

pub struct PerfProbePlugin;

impl Plugin for PerfProbePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameWindow>()
            .register_dump(dump)
            .add_systems(Last, record);
    }
}

fn record(time: Res<Time<Real>>, mut window: ResMut<FrameWindow>) {
    window.ms.push_back(time.delta_secs() * 1000.0);
    if window.ms.len() > WINDOW {
        window.ms.pop_front();
    }
}

fn assets_len<A: Asset>(world: &World) -> usize {
    world.get_resource::<Assets<A>>().map_or(0, Assets::len)
}

fn dump(world: &World) -> (String, Value) {
    let (mut mean, mut max) = (0.0f32, 0.0f32);
    if let Some(window) = world.get_resource::<FrameWindow>()
        && !window.ms.is_empty()
    {
        mean = window.ms.iter().sum::<f32>() / window.ms.len() as f32;
        max = window.ms.iter().copied().fold(0.0, f32::max);
    }
    let mut archetypes: Vec<(usize, Vec<String>)> = world
        .archetypes()
        .iter()
        .filter(|a| !a.is_empty())
        .map(|a| {
            let names = a
                .components()
                .iter()
                .filter_map(|id| world.components().get_info(*id))
                .map(|info| {
                    let full = info.name().to_string();
                    full.rsplit("::").next().unwrap_or(&full).to_owned()
                })
                .collect();
            (a.len() as usize, names)
        })
        .collect();
    archetypes.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    let total: usize = archetypes.iter().map(|(n, _)| n).sum();
    let top: Vec<Value> = archetypes
        .iter()
        .take(TOP_ARCHETYPES)
        .map(|(n, names)| {
            let mut shown: Vec<&str> = names.iter().map(String::as_str).collect();
            shown.truncate(8);
            json!({"entities": n, "components": shown})
        })
        .collect();
    (
        "perf_probe".into(),
        json!({
            "frame_ms_mean": mean,
            "frame_ms_max": max,
            "live_entities": total,
            "archetypes": world.archetypes().len(),
            "top_archetypes": top,
            "assets": {
                "mesh": assets_len::<Mesh>(world),
                "image": assets_len::<Image>(world),
                "standard_material": assets_len::<StandardMaterial>(world),
                "shader": assets_len::<Shader>(world),
            },
        }),
    )
}
