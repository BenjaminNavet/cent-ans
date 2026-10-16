//! In-campaign pause menu (game/scripts/ui/pause_menu.gd): Escape opens it, Escape again (or
//! « Reprendre ») closes it. Entries: Reprendre, Sauvegarder…, Charger…, Réglages…, Menu
//! principal, Quitter le jeu. Leaving with unsaved turns asks for confirmation. While open,
//! [`GamePaused`] is true and `Time<Virtual>` is paused (the Godot tree pause); both are reset
//! when the campaign screen is left. « Aide (F1) » opens the shortcut sheet of the campaign HUD
//! (`campaign::hud::help`) as a modal above the menu (Escape closes the sheet, not the pause);
//! the difficulty line carries the `pause_difficulty_fixed` tooltip (`data/ui/tooltips.json`).
//!
//! Escape is read here only when no modal ([`ModalQueue`]) and no dialog is open; a campaign
//! panel that also closes on Escape must run `.before(pause_escape)` and set `KeyEdges::escape` to false.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;

use super::saves::{GameLoaded, OpenSaveDialog, SaveMode, SaveProgress};
use super::{MenuOverlay, OpenSettings, spawn_overlay};
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::fader::FadeToState;
use crate::ui::kit::modal::{ModalQueue, ModalSpec, PRIORITY_REPORT};
use crate::ui::kit::style::UiType;
use crate::ui::kit::{ButtonPressed, UiTheme, button_small, typed};

/// True while the pause menu is open: campaign and battle lots freeze their own animations on it.
#[derive(Resource, Default, Debug)]
pub struct GamePaused(pub bool);

/// The pause menu overlay (layer 0).
#[derive(Component)]
pub struct PauseRoot;

/// The « quitter sans sauvegarder ? » confirmation (layer 1).
#[derive(Component)]
struct ExitConfirm {
    quit: bool,
}

/// Asks to toggle the pause menu without going through the Escape edge (the « Menu » button of
/// the top bar): an open panel stays open, as in Godot.
#[derive(Message, Debug, Clone, Copy)]
pub struct TogglePause;

/// The help sheet opened from the pause menu is on screen: Escape closes it and leaves the pause
/// menu open.
#[derive(Resource, Default, Debug)]
pub struct PauseHelp(pub bool);

/// What « Aide (F1) » needs: the modal queue, the pause flag, the settings and the key map.
type HelpContext<'w> = (
    ResMut<'w, ModalQueue>,
    ResMut<'w, PauseHelp>,
    Option<Res<'w, crate::settings::Settings>>,
    Option<Res<'w, crate::settings::KeyMap>>,
);

/// Key of the shortcut sheet in the modal queue (same as `campaign::hud::help`).
const HELP_KEY: &str = "hud:shortcuts";

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamePaused>()
            .init_resource::<PauseHelp>()
            .add_message::<TogglePause>()
            .add_systems(OnExit(AppState::Campaign), force_resume)
            .add_systems(
                Update,
                (
                    pause_escape,
                    track_help.after(pause_escape),
                    pause_buttons,
                    after_load,
                )
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

fn force_resume(mut paused: ResMut<GamePaused>, mut time: ResMut<Time<Virtual>>) {
    paused.0 = false;
    time.unpause();
}

#[allow(clippy::too_many_arguments)]
pub fn pause_escape(
    mut commands: Commands,
    edges: Res<super::KeyEdges>,
    roots: Query<Entity, With<PauseRoot>>,
    overlays: Query<&MenuOverlay>,
    modals: Res<ModalQueue>,
    mut paused: ResMut<GamePaused>,
    mut time: ResMut<Time<Virtual>>,
    theme: Res<UiTheme>,
    sim: Res<Sim>,
    mut requests: MessageReader<TogglePause>,
    help: Res<PauseHelp>,
) {
    let requested = requests.read().count() > 0;
    if help.0 {
        return; // Escape belongs to the help sheet opened from the menu
    }
    if !edges.escape && !requested {
        return;
    }
    if overlays.iter().any(|overlay| overlay.layer > 0) {
        return; // a dialog is closed by `escape_overlays`
    }
    if let Ok(root) = roots.single() {
        commands.entity(root).despawn();
        paused.0 = false;
        time.unpause();
    } else if !modals.blocks_escape() {
        spawn_pause(&mut commands, &theme, &sim);
        paused.0 = true;
        time.pause();
    }
}

/// The help sheet is gone: Escape works on the pause menu again.
fn track_help(modals: Res<ModalQueue>, mut help: ResMut<PauseHelp>) {
    if help.0 && !modals.is_busy() {
        help.0 = false;
    }
}

fn spawn_pause(commands: &mut Commands, theme: &UiTheme, sim: &Sim) {
    let (root, panel) = spawn_overlay(commands, theme, "PauseMenu", "Pause", 300.0, 0);
    commands
        .entity(root)
        .insert((PauseRoot, DespawnOnExit(AppState::Campaign)));
    if let (Some(data), Some(state)) = (&sim.data, &sim.state) {
        let key = game_api::current_difficulty(state);
        let label = game_api::difficulty_levels(data)
            .into_iter()
            .find(|level| level.id == key)
            .map_or_else(|| key.to_owned(), |level| level.label);
        let texts = crate::ui::kit::tooltip_spec::TooltipTexts::load().ok();
        commands.spawn((
            Name::new("pause:difficulty"),
            typed(theme, &format!("Difficulté : {label}"), UiType::Caption),
            Node {
                align_self: AlignSelf::Center,
                ..default()
            },
            super::custom_battle::plain_tip(texts.as_ref(), "pause_difficulty_fixed"),
            ChildOf(panel),
        ));
    }
    for (name, caption) in [
        ("pause:resume", "Reprendre"),
        ("pause:save", "Sauvegarder…"),
        ("pause:load", "Charger…"),
        ("pause:history", "Batailles jouées…"),
        ("pause:settings", "Réglages…"),
        ("pause:help", "Aide (F1)"),
        ("pause:main_menu", "Menu principal"),
        ("pause:quit", "Quitter le jeu"),
    ] {
        commands.spawn((
            crate::ui::kit::button_captioned(theme, name, caption),
            ChildOf(panel),
        ));
    }
}

fn spawn_confirm(commands: &mut Commands, theme: &UiTheme, unsaved: u32, quit: bool) {
    let what = if quit {
        "quitter le jeu"
    } else {
        "revenir au menu principal"
    };
    let plural = if unsaved > 1 { "s" } else { "" };
    let (root, panel) = spawn_overlay(
        commands,
        theme,
        "ExitConfirm",
        "Partie non sauvegardée",
        460.0,
        1,
    );
    commands.entity(root).insert(ExitConfirm { quit });
    commands.spawn((
        typed(
            theme,
            &format!(
                "La partie n’a pas été sauvegardée depuis {unsaved} tour{plural}. Voulez-vous vraiment {what} ?"
            ),
            UiType::Body,
        ),
        ChildOf(panel),
    ));
    let row = commands
        .spawn((
            Node {
                column_gap: Val::Px(8.0),
                justify_content: JustifyContent::End,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    for (name, caption) in [
        ("confirm:cancel", "Annuler"),
        ("confirm:save", "Sauvegarder d’abord"),
        ("confirm:discard", "Quitter sans sauvegarder"),
    ] {
        commands.spawn((button_small(theme, name, caption), ChildOf(row)));
    }
}

#[allow(clippy::too_many_arguments)]
fn pause_buttons(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    roots: Query<Entity, With<PauseRoot>>,
    confirms: Query<(Entity, &ExitConfirm)>,
    theme: Res<UiTheme>,
    mut sim: ResMut<Sim>,
    progress: Res<SaveProgress>,
    mut paused: ResMut<GamePaused>,
    mut time: ResMut<Time<Virtual>>,
    mut next: MessageWriter<FadeToState>,
    mut exit: MessageWriter<AppExit>,
    mut settings: MessageWriter<OpenSettings>,
    mut dialogs: MessageWriter<OpenSaveDialog>,
    mut played: MessageWriter<crate::flow::history::OpenPlayedBattles>,
    mut help_context: HelpContext,
) {
    let (modals, help, settings_res, keymap) = &mut help_context;
    for press in pressed.read() {
        let leave = |kind_quit: bool,
                     sim: &mut Sim,
                     next: &mut MessageWriter<FadeToState>,
                     exit: &mut MessageWriter<AppExit>| {
            if kind_quit {
                exit.write(AppExit::Success);
            } else {
                sim.state = None;
                next.write(FadeToState(AppState::MainMenu));
            }
        };
        match press.name.as_str() {
            "pause:resume" => {
                for root in &roots {
                    commands.entity(root).despawn();
                }
                paused.0 = false;
                time.unpause();
            }
            "pause:save" => {
                dialogs.write(OpenSaveDialog {
                    mode: SaveMode::Save,
                });
            }
            "pause:load" => {
                dialogs.write(OpenSaveDialog {
                    mode: SaveMode::Load,
                });
            }
            "pause:history" => {
                played.write(crate::flow::history::OpenPlayedBattles);
            }
            "pause:settings" => {
                settings.write(OpenSettings);
            }
            "pause:help" => {
                if !modals.is_busy() {
                    let text = crate::campaign::hud::help::sheet_bbcode_with(
                        settings_res.as_deref(),
                        keymap.as_deref(),
                    );
                    let mut spec = ModalSpec::info("Aide et raccourcis", &text);
                    spec.width = 560.0;
                    modals.request(HELP_KEY, PRIORITY_REPORT, spec);
                    help.0 = true;
                }
            }
            "pause:main_menu" | "pause:quit" => {
                let quit = press.name == "pause:quit";
                let unsaved = progress.unsaved_turns(&sim);
                if unsaved == 0 {
                    leave(quit, &mut sim, &mut next, &mut exit);
                } else {
                    spawn_confirm(&mut commands, &theme, unsaved, quit);
                }
            }
            "confirm:cancel" => {
                for (entity, _) in &confirms {
                    commands.entity(entity).despawn();
                }
            }
            "confirm:save" => {
                for (entity, _) in &confirms {
                    commands.entity(entity).despawn();
                }
                dialogs.write(OpenSaveDialog {
                    mode: SaveMode::Save,
                });
            }
            "confirm:discard" => {
                let quit = confirms.iter().next().is_some_and(|(_, c)| c.quit);
                leave(quit, &mut sim, &mut next, &mut exit);
            }
            _ => {}
        }
    }
}

/// A save replaced the campaign: close the pause menu and re-enter the campaign screen so every
/// lot rebuilds from `Sim::state`.
fn after_load(
    mut loaded: MessageReader<GameLoaded>,
    mut next: MessageWriter<crate::ui::fader::FadeToState>,
    mut paused: ResMut<GamePaused>,
    mut time: ResMut<Time<Virtual>>,
) {
    if loaded.read().last().is_some() {
        paused.0 = false;
        time.unpause();
        next.write(crate::ui::fader::FadeToState(AppState::Campaign));
    }
}
