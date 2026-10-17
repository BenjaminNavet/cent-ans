//! Pause menu of the battle: Escape opens it (when nothing is selected and no dialog, modal,
//! cinematic or speech holds the key), Escape again or « Reprendre » closes it. Entries:
//! Reprendre, Réglages…, Menu principal, Quitter le jeu (the last two ask for confirmation: the
//! battle in progress is lost). The battle simulation is held through `BattleSession::paused`
//! while the menu is open; a battle the player had already paused stays paused afterwards.

use bevy::prelude::*;

use super::pause::PauseRoot;
use super::{KeyEdges, MenuOverlay, OpenSettings, spawn_overlay};
use crate::app::AppState;
use crate::battle::orders::BattleSelection;
use crate::battle::scene::BattleSession;
use crate::sim::Sim;
use crate::ui::fader::FadeToState;
use crate::ui::kit::modal::ModalQueue;
use crate::ui::kit::style::UiType;
use crate::ui::kit::{ButtonPressed, UiTheme, button_captioned, button_small, typed};

/// The battle pause menu root, with whether the battle was already paused when it opened.
#[derive(Component)]
pub struct BattlePauseRoot {
    pub was_paused: bool,
}

/// The « quitter la bataille ? » confirmation (layer 1).
#[derive(Component)]
struct BattleExitConfirm {
    quit: bool,
}

pub struct BattlePausePlugin;

impl Plugin for BattlePausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                battle_pause_escape
                    .after(crate::battle::orders::cinematic::run_cinematic)
                    .after(crate::battle::orders::speech::run_speech),
                battle_pause_buttons,
            )
                .run_if(in_state(AppState::Battle)),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_pause_escape(
    mut commands: Commands,
    edges: Res<KeyEdges>,
    roots: Query<(Entity, &BattlePauseRoot)>,
    overlays: Query<&MenuOverlay>,
    modals: Res<ModalQueue>,
    selection: Option<Res<BattleSelection>>,
    typing: Res<crate::ui::kit::text_field::TextInputFocus>,
    mut session: ResMut<BattleSession>,
    theme: Res<UiTheme>,
) {
    if !edges.escape || typing.is_active() {
        return;
    }
    if overlays.iter().any(|overlay| overlay.layer > 0) {
        return; // a dialog is closed by `escape_overlays`
    }
    if let Ok((root, state)) = roots.single() {
        commands.entity(root).despawn();
        session.paused = state.was_paused;
    } else if !modals.blocks_escape()
        && !overlays.iter().any(|_| true)
        && selection.is_none_or(|s| s.units.is_empty())
    {
        let was_paused = session.paused;
        spawn_pause(&mut commands, &theme, was_paused);
        session.paused = true;
    }
}

fn spawn_pause(commands: &mut Commands, theme: &UiTheme, was_paused: bool) {
    let (root, panel) = spawn_overlay(commands, theme, "BattlePauseMenu", "Pause", 300.0, 0);
    commands.entity(root).insert((
        PauseRoot,
        BattlePauseRoot { was_paused },
        DespawnOnExit(AppState::Battle),
    ));
    for (name, caption) in [
        ("bpause:resume", "Reprendre"),
        ("bpause:settings", "Réglages…"),
        ("bpause:main_menu", "Menu principal"),
        ("bpause:quit", "Quitter le jeu"),
    ] {
        commands.spawn((button_captioned(theme, name, caption), ChildOf(panel)));
    }
}

fn spawn_confirm(commands: &mut Commands, theme: &UiTheme, quit: bool) {
    let what = if quit {
        "quitter le jeu"
    } else {
        "revenir au menu principal"
    };
    let (root, panel) = spawn_overlay(
        commands,
        theme,
        "BattleExitConfirm",
        "Bataille en cours",
        460.0,
        1,
    );
    commands.entity(root).insert(BattleExitConfirm { quit });
    commands.spawn((
        typed(
            theme,
            &format!("La bataille en cours sera perdue. Voulez-vous vraiment {what} ?"),
            UiType::Body,
        ),
        ChildOf(panel),
    ));
    let row = commands
        .spawn((
            Node {
                column_gap: Val::Px(8.0),
                justify_content: JustifyContent::End,
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    for (name, caption) in [
        ("bconfirm:cancel", "Annuler"),
        ("bconfirm:discard", "Quitter la bataille"),
    ] {
        commands.spawn((button_small(theme, name, caption), ChildOf(row)));
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_pause_buttons(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    roots: Query<(Entity, &BattlePauseRoot)>,
    confirms: Query<(Entity, &BattleExitConfirm)>,
    theme: Res<UiTheme>,
    mut sim: ResMut<Sim>,
    mut session: ResMut<BattleSession>,
    mut next: MessageWriter<FadeToState>,
    mut exit: MessageWriter<AppExit>,
    mut settings: MessageWriter<OpenSettings>,
) {
    for press in pressed.read() {
        match press.name.as_str() {
            "bpause:resume" => {
                for (root, state) in &roots {
                    commands.entity(root).despawn();
                    session.paused = state.was_paused;
                }
            }
            "bpause:settings" => {
                settings.write(OpenSettings);
            }
            "bpause:main_menu" => spawn_confirm(&mut commands, &theme, false),
            "bpause:quit" => spawn_confirm(&mut commands, &theme, true),
            "bconfirm:cancel" => {
                for (entity, _) in &confirms {
                    commands.entity(entity).despawn();
                }
            }
            "bconfirm:discard" => {
                if confirms.iter().next().is_some_and(|(_, c)| c.quit) {
                    exit.write(AppExit::Success);
                } else {
                    sim.state = None;
                    next.write(FadeToState(AppState::MainMenu));
                }
            }
            _ => {}
        }
    }
}
