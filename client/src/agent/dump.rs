//! `--dump-state` sections. Each plugin contributes a JSON section by registering a function.

use bevy::prelude::*;
use serde_json::{Map, Value};

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
