//! Save directory and the save / load dialog (game/scripts/ui/{save_load_dialog,save_slots}.gd).
//!
//! Files are the Godot ones (`game_api::saves`: `<name>.json` wrapper around
//! `CampaignState::save_json`, `<name>.meta.json` sheet). The directory is `CENT_ANS_SAVES` if set,
//! else the per-user data folder of the game: `~/Library/Application Support/Cent Ans/saves`
//! (macOS), `%APPDATA%/Cent Ans/saves` (Windows), `~/.local/share/Cent Ans/saves` (elsewhere). It
//! is the same `Cent Ans` folder name as Godot's custom user dir, but Godot keeps its files in
//! `<that>/saves` only on a fresh install: both clients read the same wrapper format.
//! Thumbnails are not written (Godot's `.png` is ignored by the list, the heraldry is shown).

use std::path::PathBuf;

use bevy::prelude::*;
use data_model::FactionId;
use game_api::saves::{self, SaveMeta};
use serde_json::{Value, json};

use super::{OverlayClosed, spawn_overlay};
use crate::app::AppState;
use crate::settings::KeyMap;
use crate::sim::Sim;
use crate::ui::kit::modal::ModalQueue;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::UiType;
use crate::ui::kit::text_field::{
    TextField, TextFieldSubmitted, TextInputFocus, keyboard_free, text_field_limited,
};
use crate::ui::kit::{
    ButtonPressed, ListSelected, UiTheme, button_small, error_line, list_item, scroll_list,
    set_disabled, typed,
};

/// Folder of the save slots.
#[derive(Resource, Clone, Debug)]
pub struct SaveDir(pub PathBuf);

/// `CENT_ANS_SAVES`, else the per-user data folder (see module docs).
pub fn default_save_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CENT_ANS_SAVES") {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = home.map(|h| h.join("Library/Application Support"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".local/share")));
    base.unwrap_or_else(std::env::temp_dir)
        .join("Cent Ans")
        .join("saves")
}

/// Turn of the last save or load, to warn about unsaved turns when leaving the campaign.
#[derive(Resource, Default, Debug)]
pub struct SaveProgress {
    pub saved_turn: Option<u32>,
}

impl SaveProgress {
    /// Turns played since the last save or load (0 when none was ever recorded).
    pub fn unsaved_turns(&self, sim: &Sim) -> u32 {
        match (&sim.state, self.saved_turn) {
            (Some(state), Some(saved)) => state.turn().saturating_sub(saved),
            (Some(state), None) => state.turn(),
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveMode {
    Save,
    Load,
}

/// Opens the save (name field + list) or load (list) dialog.
#[derive(Message, Debug, Clone, Copy)]
pub struct OpenSaveDialog {
    pub mode: SaveMode,
}

/// A save replaced `Sim::state` (and `Sim::player`); the screen must re-enter the campaign.
#[derive(Message, Debug, Clone, Copy)]
pub struct GameLoaded;

#[derive(Component)]
struct SaveDialog {
    mode: SaveMode,
    slots: Vec<SaveMeta>,
    selected: Option<usize>,
    overwrite_armed: Option<String>,
    delete_armed: Option<String>,
}

#[derive(Component)]
struct DialogStatus;

pub struct SavesPlugin;

impl Plugin for SavesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SaveDir(default_save_dir()))
            .init_resource::<SaveProgress>()
            .add_message::<OpenSaveDialog>()
            .add_message::<GameLoaded>()
            .add_systems(
                Update,
                (open_dialog, dialog_buttons, select_slot, submit_name),
            )
            .add_systems(
                Update,
                quick_keys
                    .run_if(keyboard_free)
                    .run_if(in_state(AppState::Campaign))
                    .run_if(crate::campaign::hud::flow::orders_open),
            );
    }
}

/// Slot of the quick save (F6) and quick load (F9), as `map_ui.gd QUICK_SAVE_NAME`.
pub const QUICK_SAVE_NAME: &str = "Sauvegarde rapide";

/// Quick save / quick load through the key map (`quick_save`, `quick_load`), unless a window
/// or the pause menu is up.
#[allow(clippy::too_many_arguments)]
fn quick_keys(
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Option<Res<KeyMap>>,
    modals: Res<ModalQueue>,
    paused: Res<super::pause::GamePaused>,
    overlays: Query<(), With<super::MenuOverlay>>,
    mut sim: ResMut<Sim>,
    dir: Res<SaveDir>,
    mut progress: ResMut<SaveProgress>,
    mut loaded: MessageWriter<GameLoaded>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    let Some(keymap) = keymap else { return };
    if modals.is_busy() || paused.0 || !overlays.is_empty() {
        return;
    }
    if keymap.just_pressed("quick_save", &keys) {
        match save_now(&sim, &dir.0, QUICK_SAVE_NAME) {
            Ok(turn) => {
                progress.saved_turn = Some(turn);
                toasts.write(ToastRequest::new("Sauvegarde rapide faite."));
            }
            Err(message) => {
                toasts.write(ToastRequest::new(&message));
            }
        }
    } else if keymap.just_pressed("quick_load", &keys) {
        let path = dir
            .0
            .join(format!("{}.json", saves::file_stem(QUICK_SAVE_NAME)));
        if !path.exists() {
            toasts.write(ToastRequest::new(&format!(
                "Aucune sauvegarde rapide ({} pour en faire une).",
                keymap.label("quick_save", "qwerty")
            )));
            return;
        }
        match load_into(&mut sim, &path) {
            Ok(turn) => {
                progress.saved_turn = Some(turn);
                loaded.write(GameLoaded);
            }
            Err(message) => {
                toasts.write(ToastRequest::new(&message));
            }
        }
    }
}

/// Default name proposed by the save dialog.
pub const DEFAULT_NAME: &str = "partie";

/// French « 24 sept. 2026, 23 h 19 » from `2026-09-24T23:19:05`; the input when unreadable.
pub fn french_timestamp(iso: &str) -> String {
    const MONTHS: [&str; 12] = [
        "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
        "déc.",
    ];
    let (date, time) = iso.split_once('T').unwrap_or((iso, ""));
    let parts: Vec<&str> = date.split('-').collect();
    let (Some(year), Some(month), Some(day)) = (
        parts.first(),
        parts.get(1).and_then(|m| m.parse::<usize>().ok()),
        parts.get(2).and_then(|d| d.parse::<u32>().ok()),
    ) else {
        return iso.to_owned();
    };
    if !(1..=12).contains(&month) {
        return iso.to_owned();
    }
    let mut text = format!("{day} {} {year}", MONTHS[month - 1]);
    let mut clock = time.split(':');
    if let (Some(hour), Some(minute)) = (
        clock.next().and_then(|h| h.parse::<u32>().ok()),
        clock.next(),
    ) {
        text.push_str(&format!(", {hour} h {minute}"));
    }
    text
}

fn difficulty_label(sim: &Sim, key: &str) -> String {
    sim.data
        .as_ref()
        .and_then(|data| {
            game_api::difficulty_levels(data)
                .into_iter()
                .find(|level| level.id == key)
        })
        .map_or_else(|| key.to_owned(), |level| level.label.to_lowercase())
}

fn faction_name(sim: &Sim, id: &str) -> String {
    match (&sim.data, FactionId::new(id)) {
        (Some(data), Ok(id)) => data.faction_name(&id),
        _ => id.to_owned(),
    }
}

fn open_dialog(
    mut commands: Commands,
    mut requests: MessageReader<OpenSaveDialog>,
    existing: Query<Entity, With<SaveDialog>>,
    theme: Res<UiTheme>,
    dir: Res<SaveDir>,
    sim: Res<Sim>,
    mut focus: ResMut<TextInputFocus>,
) {
    let Some(request) = requests.read().last().copied() else {
        return;
    };
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let slots = saves::list_saves(&dir.0);
    let title = match request.mode {
        SaveMode::Save => "Sauvegarder la partie",
        SaveMode::Load => "Charger une partie",
    };
    let (overlay, panel) = spawn_overlay(&mut commands, &theme, "SaveDialog", title, 760.0, 2);
    commands.entity(overlay).insert(SaveDialog {
        mode: request.mode,
        slots: slots.clone(),
        selected: None,
        overwrite_armed: None,
        delete_armed: None,
    });
    let list = commands
        .spawn((scroll_list(&theme, "dialog:slots", 300.0), ChildOf(panel)))
        .id();
    if slots.is_empty() {
        commands.spawn((
            typed(&theme, "Aucune sauvegarde.", UiType::Body),
            ChildOf(list),
        ));
    }
    for slot in &slots {
        let caption = format!(
            "{} — {}, {}, tour {} — difficulté {} — le {}",
            slot.label,
            faction_name(&sim, &slot.faction),
            slot.date,
            slot.turn + 1,
            difficulty_label(&sim, &slot.difficulty),
            french_timestamp(&slot.timestamp),
        );
        commands.spawn((
            list_item(
                &theme,
                "slots",
                &format!("slot:{}", slot.name),
                &caption,
                false,
            ),
            ChildOf(list),
        ));
    }
    if request.mode == SaveMode::Save {
        commands.spawn((
            typed(&theme, "Nom de la sauvegarde", UiType::Caption),
            ChildOf(panel),
        ));
        let field = commands
            .spawn((
                text_field_limited(
                    &theme,
                    "dialog:name",
                    DEFAULT_NAME,
                    "Nom de la sauvegarde",
                    420.0,
                    40,
                ),
                ChildOf(panel),
            ))
            .id();
        focus.0 = Some(field);
    }
    commands.spawn((DialogStatus, error_line(&theme), ChildOf(panel)));
    let row = commands
        .spawn((
            Node {
                column_gap: Val::Px(8.0),
                justify_content: JustifyContent::End,
                align_self: AlignSelf::Stretch,
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    let confirm_caption = match request.mode {
        SaveMode::Save => "Sauvegarder",
        SaveMode::Load => "Charger",
    };
    let delete = commands
        .spawn((
            button_small(&theme, "dialog:delete", "Supprimer"),
            ChildOf(row),
        ))
        .id();
    set_disabled(&mut commands, delete, true);
    commands.spawn((
        button_small(&theme, "dialog:cancel", "Annuler"),
        ChildOf(row),
    ));
    let confirm = commands
        .spawn((
            button_small(&theme, "dialog:confirm", confirm_caption),
            ChildOf(row),
        ))
        .id();
    if request.mode == SaveMode::Load && slots.is_empty() {
        set_disabled(&mut commands, confirm, true);
    }
}

fn select_slot(
    mut selected: MessageReader<ListSelected>,
    mut dialogs: Query<&mut SaveDialog>,
    mut fields: Query<&mut TextField>,
    mut commands: Commands,
    buttons: Query<(Entity, &Name)>,
) {
    for event in selected.read() {
        if event.group != "slots" {
            continue;
        }
        let Ok(mut dialog) = dialogs.single_mut() else {
            continue;
        };
        let stem = event.name.trim_start_matches("slot:");
        dialog.selected = dialog.slots.iter().position(|slot| slot.name == stem);
        dialog.delete_armed = None;
        dialog.overwrite_armed = None;
        if dialog.mode == SaveMode::Save
            && let Ok(mut field) = fields.single_mut()
        {
            field.set(stem);
        }
        for (entity, name) in &buttons {
            if name.as_str() == "dialog:delete" {
                set_disabled(&mut commands, entity, dialog.selected.is_none());
            }
        }
    }
}

/// Enter in the name field confirms the dialog.
fn submit_name(
    mut submitted: MessageReader<TextFieldSubmitted>,
    mut presses: MessageWriter<ButtonPressed>,
    confirm: Query<(Entity, &Name)>,
) {
    for event in submitted.read() {
        if event.name != "dialog:name" {
            continue;
        }
        if let Some((entity, _)) = confirm.iter().find(|(_, n)| n.as_str() == "dialog:confirm") {
            presses.write(ButtonPressed {
                entity,
                name: "dialog:confirm".to_owned(),
            });
        }
    }
}

fn set_status(
    world_status: &mut Query<(&mut Text, &mut Visibility), With<DialogStatus>>,
    message: &str,
) {
    for (mut text, mut visibility) in world_status {
        text.0 = message.to_owned();
        *visibility = if message.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn dialog_buttons(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    mut dialogs: Query<(Entity, &mut SaveDialog)>,
    fields: Query<&TextField>,
    mut status: Query<(&mut Text, &mut Visibility), With<DialogStatus>>,
    mut sim: ResMut<Sim>,
    dir: Res<SaveDir>,
    mut progress: ResMut<SaveProgress>,
    mut loaded: MessageWriter<GameLoaded>,
    mut toasts: MessageWriter<ToastRequest>,
    mut reopen: MessageWriter<OpenSaveDialog>,
    mut closed: MessageWriter<OverlayClosed>,
    buttons: Query<(&Name, &Children)>,
    mut captions: Query<&mut Text, Without<DialogStatus>>,
) {
    for press in pressed.read() {
        let Ok((overlay, mut dialog)) = dialogs.single_mut() else {
            return;
        };
        match press.name.as_str() {
            "dialog:cancel" => {
                commands.entity(overlay).despawn();
                closed.write(OverlayClosed);
            }
            "dialog:delete" => {
                let Some(slot) = dialog.selected.and_then(|i| dialog.slots.get(i)).cloned() else {
                    continue;
                };
                if dialog.delete_armed.as_deref() != Some(slot.name.as_str()) {
                    dialog.delete_armed = Some(slot.name.clone());
                    set_status(
                        &mut status,
                        &format!("Cliquez encore pour supprimer « {} ».", slot.label),
                    );
                    continue;
                }
                saves::delete_save(&dir.0, &slot.name);
                reopen.write(OpenSaveDialog { mode: dialog.mode });
            }
            "dialog:confirm" => match dialog.mode {
                SaveMode::Save => {
                    let name = fields
                        .single()
                        .map(|f| f.value.trim().to_owned())
                        .unwrap_or_default();
                    if name.is_empty() {
                        set_status(&mut status, "Donnez un nom à la sauvegarde.");
                        continue;
                    }
                    let stem = saves::file_stem(&name);
                    let exists = dialog.slots.iter().any(|slot| slot.name == stem);
                    if exists && dialog.overwrite_armed.as_deref() != Some(name.as_str()) {
                        dialog.overwrite_armed = Some(name.clone());
                        set_status(
                            &mut status,
                            &format!("« {name} » existe déjà. Écraser cette sauvegarde ?"),
                        );
                        // The confirm button turns into « Écraser » so the second press is explicit.
                        for (button, children) in &buttons {
                            if button.as_str() == "dialog:confirm" {
                                for child in children.iter() {
                                    if let Ok(mut text) = captions.get_mut(child) {
                                        text.0 = "Écraser".to_owned();
                                    }
                                }
                            }
                        }
                        continue;
                    }
                    match save_now(&sim, &dir.0, &name) {
                        Ok(turn) => {
                            progress.saved_turn = Some(turn);
                            toasts.write(ToastRequest::new("Partie sauvegardée"));
                            commands.entity(overlay).despawn();
                            closed.write(OverlayClosed);
                        }
                        Err(message) => set_status(&mut status, &message),
                    }
                }
                SaveMode::Load => {
                    let Some(slot) = dialog.selected.and_then(|i| dialog.slots.get(i)).cloned()
                    else {
                        set_status(&mut status, "Choisissez une sauvegarde.");
                        continue;
                    };
                    match load_into(&mut sim, std::path::Path::new(&slot.path)) {
                        Ok(turn) => {
                            progress.saved_turn = Some(turn);
                            commands.entity(overlay).despawn();
                            loaded.write(GameLoaded);
                        }
                        Err(message) => set_status(&mut status, &message),
                    }
                }
            },
            _ => {}
        }
    }
}

/// Writes slot `name` from the running campaign; returns the saved turn.
pub fn save_now(sim: &Sim, dir: &std::path::Path, name: &str) -> Result<u32, String> {
    let state = sim.state.as_ref().ok_or("Aucune campagne en cours.")?;
    saves::write_save_with_client(
        dir,
        name,
        state,
        &saves::timestamp_now(),
        super::save_extras::snapshot(),
    )?;
    crate::flow::history::write_sidecar(&saves::save_path(dir, name));
    Ok(state.turn())
}

/// Replaces the running campaign by the save at `path`; returns its turn. The running campaign
/// is kept when the save is refused (as `SimFacade.load_game`).
pub fn load_into(sim: &mut Sim, path: &std::path::Path) -> Result<u32, String> {
    let data = sim.data.clone().ok_or("Données de jeu absentes.")?;
    let wrapper = saves::read_save(path)?;
    let state = saves::load_state(&data, &wrapper)?;
    sim.player = state.player_faction().clone();
    let turn = state.turn();
    sim.state = Some(state);
    super::save_extras::stash_loaded(&wrapper.client);
    crate::flow::history::read_sidecar(path);
    Ok(turn)
}

/// Section `menu.saves`.
pub fn dump(world: &World) -> Value {
    let dir = world.resource::<SaveDir>();
    let progress = world.resource::<SaveProgress>();
    let count = saves::list_saves(&dir.0).len();
    let mut query = world.try_query::<&SaveDialog>().unwrap();
    let dialog = query.iter(world).next().map(|d| {
        json!({
            "mode": format!("{:?}", d.mode),
            "slots": d.slots.len(),
            "selected": d.selected.and_then(|i| d.slots.get(i)).map(|s| s.name.clone()),
            "overwrite_armed": d.overwrite_armed,
        })
    });
    // Digest of the whole campaign state (RNG included): equal before a save and after its load.
    let sim = world.resource::<Sim>();
    let digest = sim.state.as_ref().map(|state| {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        state.save_json().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    });
    json!({
        "state_digest": digest,
        "turn": sim.state.as_ref().map(|state| state.turn()),
        "dir": dir.0.to_string_lossy(),
        "count": count,
        "saved_turn": progress.saved_turn,
        "dialog": dialog,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_loads_back_to_the_same_state() {
        use crate::paths;
        let root = paths::find_root().expect("repo root");
        let (data, _) = data_model::GameData::load(&paths::data_dir(&root)).expect("game data");
        let data = std::sync::Arc::new(data);
        let player = data_model::FactionId::new(crate::sim::DEFAULT_FACTION).unwrap();
        let mut sim = Sim {
            data: Some(data),
            state: None,
            player,
            seed: 7,
        };
        sim.new_campaign().expect("campaign");
        let dir = std::env::temp_dir().join(format!("cent-ans-saves-{}", std::process::id()));
        let turn = save_now(&sim, &dir, QUICK_SAVE_NAME).expect("saved");
        let before = sim.state.as_ref().unwrap().save_json();
        sim.state = None;
        let path = dir.join(format!("{}.json", saves::file_stem(QUICK_SAVE_NAME)));
        assert_eq!(load_into(&mut sim, &path).expect("loaded"), turn);
        assert_eq!(sim.state.as_ref().unwrap().save_json(), before);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn timestamps_are_french() {
        assert_eq!(
            french_timestamp("2026-09-24T23:19:05"),
            "24 sept. 2026, 23 h 19"
        );
        assert_eq!(french_timestamp("n'importe quoi"), "n'importe quoi");
    }
}
