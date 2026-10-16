//! UI data files as `data://` assets (ADR 0363, lot `dataassets2`).
//!
//! - `ui/illustrations.json` -> [`IllustrationsDoc`] -> resource `ArtPlates`;
//! - `ui/front_end.json` -> [`FrontEndDoc`] -> `MenuData::front`.
//!
//! Both loaders keep their synchronous start-up read (tests, dumps, first frames before
//! [`super::DataLoad::Ready`]); once the asset is loaded it becomes the live source and each
//! `Modified` (file watcher, `dev` builds) rebuilds the resource with the same parser.

use bevy::asset::AssetLoadFailedEvent;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_common_assets::json::JsonAssetPlugin;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{DataLoad, ReloadStats};
use crate::agent::dump::DumpAppExt;
use crate::menu::MenuData;
use crate::menu::front_end::FrontEnd;
use crate::ui::art_plates::ArtPlates;

/// `data/ui/illustrations.json` as a JSON document (`ArtPlates::parse`).
#[derive(Asset, TypePath, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct IllustrationsDoc(pub Value);

/// `data/ui/front_end.json` as a JSON document (`FrontEnd::from_value`).
#[derive(Asset, TypePath, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct FrontEndDoc(pub Value);

/// Files of [`UiData`], relative to `data/`.
pub const UI_FILES: [&str; 2] = ["ui/illustrations.json", "ui/front_end.json"];

/// Handles kept alive so that the watcher keeps reloading the files.
#[derive(AssetCollection, Resource)]
pub struct UiData {
    #[asset(path = "data://ui/illustrations.json")]
    pub illustrations: Handle<IllustrationsDoc>,
    #[asset(path = "data://ui/front_end.json")]
    pub front_end: Handle<FrontEndDoc>,
}

/// What the systems below did (read by the dump).
#[derive(Resource, Debug, Default)]
pub struct UiReloadStats {
    pub illustrations_loads: u32,
    pub illustrations_reloads: u32,
    pub front_end_loads: u32,
    pub front_end_reloads: u32,
    /// Wall clock (ms since epoch) of the last `Modified` event.
    pub last_reload_unix_ms: u128,
}

pub struct UiDataPlugin;

impl Plugin for UiDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            JsonAssetPlugin::<IllustrationsDoc>::new(&["json"]),
            JsonAssetPlugin::<FrontEndDoc>::new(&["json"]),
        ))
        .init_resource::<UiReloadStats>()
        .add_systems(Update, collect_ui_failures)
        .add_systems(OnEnter(DataLoad::Ready), adopt_initial)
        .add_systems(
            Update,
            (reload_illustrations, reload_front_end).run_if(in_state(DataLoad::Ready)),
        )
        .register_dump(dump_ui);
    }
}

fn collect_ui_failures(
    mut illustrations: MessageReader<AssetLoadFailedEvent<IllustrationsDoc>>,
    mut front_end: MessageReader<AssetLoadFailedEvent<FrontEndDoc>>,
    mut stats: ResMut<ReloadStats>,
) {
    let mut push = |path: String, error: String| {
        let line = format!("{path}: {error}");
        if !stats.errors.contains(&line) {
            stats.errors.push(line);
        }
    };
    for e in illustrations.read() {
        push(e.path.to_string(), e.error.to_string());
    }
    for e in front_end.read() {
        push(e.path.to_string(), e.error.to_string());
    }
}

/// Adopts both documents once the whole collection is loaded: the per-asset `LoadedWithDependencies`
/// events come before [`DataLoad::Ready`] (and before [`UiData`] exists), so the first adoption is
/// done here rather than from events.
fn adopt_initial(
    mut commands: Commands,
    illustrations: Res<Assets<IllustrationsDoc>>,
    front_end: Res<Assets<FrontEndDoc>>,
    data: Res<UiData>,
    menu: Option<ResMut<MenuData>>,
    mut stats: ResMut<UiReloadStats>,
) {
    if let Some(doc) = illustrations.get(&data.illustrations) {
        commands.insert_resource(ArtPlates::parse(&doc.0));
        stats.illustrations_loads += 1;
    }
    if let Some(mut menu) = menu
        && let Some(front) = front_end_from(&front_end, &data)
    {
        menu.front = front;
        stats.front_end_loads += 1;
    }
}

fn front_end_from(assets: &Assets<FrontEndDoc>, data: &UiData) -> Option<FrontEnd> {
    let doc = assets.get(&data.front_end)?;
    let root = crate::paths::find_root().ok()?;
    FrontEnd::from_value(doc.0.clone(), &crate::paths::asset_root(&root))
        .inspect_err(|message| warn!("front_end.json: {message}"))
        .ok()
}

fn reload_illustrations(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<IllustrationsDoc>>,
    assets: Res<Assets<IllustrationsDoc>>,
    data: Res<UiData>,
    mut stats: ResMut<UiReloadStats>,
) {
    for event in events.read() {
        if matches!(event, AssetEvent::Modified { id } if *id == data.illustrations.id())
            && let Some(doc) = assets.get(&data.illustrations)
        {
            commands.insert_resource(ArtPlates::parse(&doc.0));
            stats.illustrations_reloads += 1;
            stats.last_reload_unix_ms = super::now_ms();
        }
    }
}

fn reload_front_end(
    mut events: MessageReader<AssetEvent<FrontEndDoc>>,
    assets: Res<Assets<FrontEndDoc>>,
    data: Res<UiData>,
    menu: Option<ResMut<MenuData>>,
    mut stats: ResMut<UiReloadStats>,
) {
    let Some(mut menu) = menu else { return };
    for event in events.read() {
        if matches!(event, AssetEvent::Modified { id } if *id == data.front_end.id())
            && let Some(front) = front_end_from(&assets, &data)
        {
            menu.front = front;
            stats.front_end_reloads += 1;
            stats.last_reload_unix_ms = super::now_ms();
        }
    }
}

fn dump_ui(world: &World) -> (String, Value) {
    let stats = world.resource::<UiReloadStats>();
    let plates = world.get_resource::<ArtPlates>().map(|p| {
        json!({"min_seconds": p.min_seconds, "max_seconds": p.max_seconds,
               "battle_screens": p.loading_screens("battle").len()})
    });
    let front = world.get_resource::<MenuData>().map(|m| {
        json!({"factions": m.front.factions.len(), "start_dates": m.front.start_dates.len(),
               "recommended": m.front.recommended().len(),
               "intro_cards": m.front.intro.cards.len()})
    });
    (
        "dataassets_ui".into(),
        json!({
            "illustrations_loads": stats.illustrations_loads,
            "illustrations_reloads": stats.illustrations_reloads,
            "front_end_loads": stats.front_end_loads,
            "front_end_reloads": stats.front_end_reloads,
            "last_reload_unix_ms": stats.last_reload_unix_ms,
            "file_mtime_unix_ms": {
                "illustrations": super::mtime_ms("ui/illustrations.json"),
                "front_end": super::mtime_ms("ui/front_end.json"),
            },
            "art_plates": plates,
            "front_end": front,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(relative: &str) -> Vec<u8> {
        let root = crate::paths::find_root().unwrap();
        std::fs::read(crate::paths::data_dir(&root).join(relative)).unwrap()
    }

    #[test]
    fn documents_give_the_same_data_as_the_sync_loaders() {
        let root = crate::paths::find_root().unwrap();
        let plates: IllustrationsDoc =
            serde_json::from_slice(&real("ui/illustrations.json")).unwrap();
        let sync = ArtPlates::load();
        let from_doc = ArtPlates::parse(&plates.0);
        assert_eq!(from_doc.min_seconds, sync.min_seconds);
        assert_eq!(
            from_doc.loading_screens("battle").len(),
            sync.loading_screens("battle").len()
        );
        let front: FrontEndDoc = serde_json::from_slice(&real("ui/front_end.json")).unwrap();
        let from_doc = FrontEnd::from_value(front.0, &crate::paths::asset_root(&root)).unwrap();
        let sync = FrontEnd::load(
            &crate::paths::data_dir(&root),
            &crate::paths::asset_root(&root),
        )
        .unwrap();
        assert_eq!(from_doc.factions.len(), sync.factions.len());
        assert_eq!(from_doc.recommended(), sync.recommended());
    }

    #[test]
    fn listed_ui_files_exist() {
        let root = crate::paths::find_root().unwrap();
        for relative in UI_FILES {
            assert!(
                crate::paths::data_dir(&root).join(relative).is_file(),
                "{relative}"
            );
        }
    }
}
