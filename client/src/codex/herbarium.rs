//! Herbarium (port of game/scripts/codex/herbarium.gd): the plants (`herbs`, Codex ids) of the
//! technologies the player knows are marked discovered in the [`CodexStore`] (meta-progression,
//! not a rule). Entries missing from the Codex are ignored. The Codex list shows the plants of
//! a mixed family under a « — Herbier — » header ([`HEADER`], see `refresh` in `mod.rs`).

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde_json::{Value, json};

use super::store::CodexStore;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::paths;
use crate::sim::Sim;
use crate::ui::kit::modal::ToastRequest;

/// Codex category of the plants.
pub const CATEGORY: &str = "plante";
/// Header of the plants sub-section of a family list.
pub const HEADER: &str = "— Herbier —";

/// Technology id -> plant ids (`data/technologies/*.json`, key `herbs`).
#[derive(Resource, Default, Debug)]
pub struct HerbMap(pub BTreeMap<String, Vec<String>>);

impl HerbMap {
    pub fn load(data_dir: &std::path::Path) -> Self {
        let mut map = BTreeMap::new();
        let Ok(read) = std::fs::read_dir(data_dir.join("technologies")) else {
            return Self::default();
        };
        for path in read.filter_map(Result::ok).map(|entry| entry.path()) {
            let Some(value) = std::fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            else {
                continue;
            };
            let herbs: Vec<String> = value
                .get("herbs")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|herb| herb.as_str().map(str::to_owned))
                .collect();
            if let (Some(id), false) = (value.get("id").and_then(Value::as_str), herbs.is_empty()) {
                map.insert(id.to_owned(), herbs);
            }
        }
        Self(map)
    }
}

/// Marks the plants of the `known` technologies discovered; returns the newly discovered ids.
pub fn sync<'a>(
    known: impl IntoIterator<Item = &'a str>,
    herbs: &HerbMap,
    store: &mut CodexStore,
) -> Vec<String> {
    let mut found = Vec::new();
    for tech in known {
        for herb in herbs.0.get(tech).into_iter().flatten() {
            if store.has(herb) && store.discover(herb) {
                found.push(herb.clone());
            }
        }
    }
    found
}

/// « Nouvelle plante dans l’herbier : Sauge, Rue ».
pub fn message(store: &CodexStore, ids: &[String]) -> String {
    let names: Vec<String> = ids.iter().map(|id| store.title(id)).collect();
    format!(
        "{} dans l’herbier : {}",
        if ids.len() == 1 {
            "Nouvelle plante"
        } else {
            "Nouvelles plantes"
        },
        names.join(", ")
    )
}

/// Number of known technologies at the last sync (`None` = not synced since the campaign began).
#[derive(Resource, Default)]
struct LastSync(Option<usize>);

#[derive(Resource, Default)]
struct HerbStats {
    announced: usize,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<HerbMap>()
        .init_resource::<LastSync>()
        .init_resource::<HerbStats>()
        .register_dump(dump)
        .add_systems(Startup, load_map)
        .add_systems(OnEnter(AppState::Campaign), reset_sync)
        .add_systems(Update, sync_player.run_if(in_state(AppState::Campaign)));
}

fn load_map(mut map: ResMut<HerbMap>) {
    if let Ok(root) = paths::find_root() {
        *map = HerbMap::load(&paths::data_dir(&root));
    }
}

fn reset_sync(mut last: ResMut<LastSync>) {
    last.0 = None;
}

fn sync_player(
    sim: Res<Sim>,
    herbs: Res<HerbMap>,
    mut store: ResMut<CodexStore>,
    mut last: ResMut<LastSync>,
    mut stats: ResMut<HerbStats>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    let Some(faction) = sim
        .state
        .as_ref()
        .and_then(|state| state.factions.get(&sim.player))
    else {
        return;
    };
    let count = faction.technologies.len();
    if last.0 == Some(count) {
        return;
    }
    last.0 = Some(count);
    let found = sync(
        faction.technologies.iter().map(|tech| tech.as_str()),
        &herbs,
        &mut store,
    );
    if !found.is_empty() {
        stats.announced += found.len();
        toasts.write(ToastRequest::new(&message(&store, &found)));
    }
}

fn dump(world: &World) -> (String, Value) {
    let herbs = world.resource::<HerbMap>();
    let store = world.resource::<CodexStore>();
    let plants: Vec<&str> = herbs
        .0
        .values()
        .flatten()
        .map(String::as_str)
        .filter(|id| store.has(id))
        .collect();
    let known = plants.iter().filter(|id| store.is_discovered(id)).count();
    (
        "codex_herbarium".to_owned(),
        json!({
            "techs_with_herbs": herbs.0.len(),
            "plants_linked": plants.len(),
            "plants_discovered": known,
            "announced": world.resource::<HerbStats>().announced,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_technologies_discover_their_plants_once() {
        let mut store = CodexStore::parse(
            br#"{"entries":[{"id":"cdx_sauge","title":"Sauge","category":"plante"}]}"#,
        )
        .unwrap();
        store.use_save_file(std::env::temp_dir().join(format!("herb-{}.json", std::process::id())));
        let herbs = HerbMap(BTreeMap::from([
            (
                "tech_garden".to_owned(),
                vec!["cdx_sauge".to_owned(), "cdx_absent".to_owned()],
            ),
            ("tech_mill".to_owned(), vec![]),
        ]));
        let found = sync(["tech_garden", "tech_x"], &herbs, &mut store);
        assert_eq!(found, ["cdx_sauge"]);
        assert!(sync(["tech_garden"], &herbs, &mut store).is_empty());
        assert_eq!(
            message(&store, &found),
            "Nouvelle plante dans l’herbier : Sauge"
        );
        let _ = std::fs::remove_file(
            std::env::temp_dir().join(format!("herb-{}.json", std::process::id())),
        );
    }
}
