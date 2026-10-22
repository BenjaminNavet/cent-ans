//! `--dump-state` sections. Each plugin contributes a JSON section by registering a function.

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use serde_json::{Map, Value};

/// Schedule run just before a dump is built: it refreshes the statistics that only the dump
/// reads (they used to be recomputed every frame in `Update`). Plugins add their systems with
/// `app.add_systems(DumpPrepare, ...)`; the schedule never runs in normal play.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DumpPrepare;

/// Runs [`DumpPrepare`] (a no-op when no plugin registered a system for it).
pub fn prepare_dump(world: &mut World) {
    let _ = world.try_run_schedule(DumpPrepare);
}

/// A dump section: reads the world, returns `(section name, JSON)`.
pub type DumpFn = fn(&World) -> (String, Value);

#[derive(Resource, Default, Clone)]
pub struct DumpRegistry(pub Vec<DumpFn>);

pub trait DumpAppExt {
    /// Registers a dump section, e.g. `app.register_dump(dump_armies)`.
    fn register_dump(&mut self, section: DumpFn) -> &mut Self;
}

impl DumpAppExt for App {
    fn register_dump(&mut self, section: DumpFn) -> &mut Self {
        self.world_mut()
            .get_resource_or_insert_with(DumpRegistry::default)
            .0
            .push(section);
        self
    }
}

/// Runs every registered section into one JSON object.
pub fn build_dump(world: &World) -> Value {
    let mut dump = Map::new();
    if let Some(registry) = world.get_resource::<DumpRegistry>() {
        for section in &registry.0 {
            let (name, value) = section(world);
            dump.insert(name, value);
        }
    }
    Value::Object(dump)
}

/// Built-in section `state`: current screen.
pub fn dump_state(world: &World) -> (String, Value) {
    let state = world
        .get_resource::<State<crate::app::AppState>>()
        .map_or_else(
            || "unknown".to_owned(),
            |state| format!("{:?}", state.get()),
        );
    ("state".to_owned(), Value::String(state))
}

/// Built-in section `entities`: total entity count.
pub fn dump_entities(world: &World) -> (String, Value) {
    ("entities".to_owned(), Value::from(world.entities().len()))
}
