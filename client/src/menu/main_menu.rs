//! Start menu (game/scripts/ui/start_menu.gd, battles_menu.gd, credits_screen.gd, intro_cards.gd).
//!
//! Left column over the illustrated menu map: illuminated title, then Nouvelle partie, Continuer
//! (+ faction and date of the latest save), Charger une partie, Prologue, Codex, Batailles,
//! Réglages, Crédits, Quitter. Differences from Godot: the backdrop is the 2D illustrated map
//! (Godot's headless fallback `MenuBackground`), not the living 3D Paris scene; the prologue is
//! played as a queue of information modals instead of full-screen cards.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui::InteractionDisabled;

use super::battles::{BattleScreen, OpenBattleScreen};
use super::saves::{GameLoaded, OpenSaveDialog, SaveDir, SaveMode, load_into};
use super::{MenuData, MenuOverlay, OpenSettings, menu_button, spawn_backdrop, spawn_overlay};
use crate::app::AppState;
use crate::paths;
use crate::sim::Sim;
use crate::ui::fader::FadeToState;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::modal::{ModalQueue, ModalSpec, PRIORITY_REPORT};
use crate::ui::kit::style::UiType;
use crate::ui::kit::{
    ButtonPressed, UiTheme, button, button_captioned, illuminated_title, scroll_list, typed,
};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_menu)
            .add_systems(OnExit(AppState::MainMenu), clear_prologue)
            .add_systems(
                Update,
                (handle_buttons, after_load).run_if(in_state(AppState::MainMenu)),
            );
    }
}

/// Marks the overlays opened from the main menu.
#[derive(Component)]
struct MainMenuOverlay;

const BUTTON_SIZE: f32 = 28.0;

/// Root of the start menu (title cartouche + entries); the faction screen removes any leftover.
#[derive(Component)]
pub struct MainMenuRoot;

fn spawn_menu(
    mut commands: Commands,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    dir: Res<SaveDir>,
    sim: Res<Sim>,
) {
    let root = commands
        .spawn((
            Name::new("MainMenu"),
            MainMenuRoot,
            DespawnOnExit(AppState::MainMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .id();
    spawn_backdrop(&mut commands, &assets, root);
    let latest = game_api::saves::list_saves(&dir.0).into_iter().next();
    let column = commands
        .spawn((
            Name::new("MainColumn"),
            Node {
                width: Val::Px(640.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect {
                    left: Val::Px(84.0),
                    top: Val::Px(16.0),
                    bottom: Val::Px(24.0),
                    ..default()
                },
                row_gap: Val::Px(4.0),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        illuminated_title(&theme, "Cent Ans", "La guerre de Cent Ans, 1337-1453"),
        ChildOf(column),
    ));
    commands.spawn((
        Node {
            height: Val::Px(8.0),
            ..default()
        },
        ChildOf(column),
    ));
    let entries: [(&str, &str); 9] = [
        ("Nouvelle partie", "Nouvelle partie"),
        ("Continuer", "Continuer"),
        ("Charger une partie", "Charger une partie"),
        ("Prologue", "Prologue : 1328-1337"),
        ("Codex", "Codex"),
        ("Batailles", "Batailles"),
        ("Réglages", "Réglages"),
        ("Crédits", "Crédits"),
        ("Quitter", "Quitter"),
    ];
    for (name, caption) in entries {
        if name == "Continuer" {
            let Some(latest) = &latest else { continue };
            commands.spawn((
                menu_button(&theme, name, caption, BUTTON_SIZE),
                ChildOf(column),
            ));
            let faction = sim
                .data
                .as_ref()
                .zip(data_model::FactionId::new(latest.faction.clone()).ok())
                .map_or_else(|| latest.faction.clone(), |(d, id)| d.faction_name(&id));
            commands.spawn((
                Name::new("ContinueDetail"),
                Text::new(format!("{faction} — {}", latest.date)),
                theme.text_font(false, true, UiType::Body.size()),
                TextColor(Color::srgb(0.85, 0.78, 0.62)),
                Node {
                    margin: UiRect {
                        left: Val::Px(25.0),
                        top: Val::Px(-2.0),
                        ..default()
                    },
                    ..default()
                },
                ChildOf(column),
            ));
        } else {
            commands.spawn((
                menu_button(&theme, name, caption, BUTTON_SIZE),
                ChildOf(column),
            ));
        }
    }
    if !sim.loaded() {
        commands.spawn((
            Text::new("Données de jeu introuvables : le jeu est incomplet, réinstallez-le."),
            theme.text_font(false, true, UiType::Caption.size()),
            TextColor(Color::srgb(1.0, 0.75, 0.6)),
            ChildOf(column),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    mut sim: ResMut<Sim>,
    mut next: MessageWriter<FadeToState>,
    mut exit: MessageWriter<AppExit>,
    mut settings: MessageWriter<OpenSettings>,
    mut dialogs: MessageWriter<OpenSaveDialog>,
    mut screens: MessageWriter<OpenBattleScreen>,
    mut replays: MessageWriter<crate::flow::OpenBattleReplays>,
    mut modals: ResMut<ModalQueue>,
    mut toasts: MessageWriter<ToastRequest>,
    overlays: Query<(Entity, &MenuOverlay), With<MainMenuOverlay>>,
    theme: Res<UiTheme>,
    data: Res<MenuData>,
    dir: Res<SaveDir>,
    mut progress: ResMut<super::saves::SaveProgress>,
) {
    for press in pressed.read() {
        match press.name.as_str() {
            "Nouvelle partie" => {
                next.write(FadeToState(AppState::FactionSelect));
            }
            "Continuer" => {
                if let Some(latest) = game_api::saves::list_saves(&dir.0).into_iter().next() {
                    match load_into(&mut sim, std::path::Path::new(&latest.path)) {
                        Ok(turn) => {
                            progress.saved_turn = Some(turn);
                            next.write(FadeToState(AppState::Campaign));
                        }
                        Err(message) => {
                            warn!("load {}: {message}", latest.path);
                            toasts.write(ToastRequest::new(&message));
                        }
                    }
                }
            }
            "Charger une partie" => {
                dialogs.write(OpenSaveDialog {
                    mode: SaveMode::Load,
                });
            }
            "Prologue" => {
                for (index, card) in data.front.intro.cards.iter().enumerate() {
                    modals.request(
                        &format!("prologue:{index:02}"),
                        PRIORITY_REPORT,
                        ModalSpec::info(&format!("{} — {}", card.date, card.title), &card.text),
                    );
                }
            }
            "Codex" => {
                next.write(FadeToState(AppState::Codex));
            }
            "Batailles" => spawn_battles(&mut commands, &theme),
            "Bataille personnalisée" => {
                screens.write(OpenBattleScreen(BattleScreen::Custom));
            }
            "Batailles historiques" => {
                screens.write(OpenBattleScreen(BattleScreen::Historical));
            }
            "Batailles de démonstration" => {
                screens.write(OpenBattleScreen(BattleScreen::Demos));
            }
            "Rejeux" => {
                replays.write(crate::flow::OpenBattleReplays);
            }
            "Réglages" => {
                settings.write(OpenSettings);
            }
            "Crédits" => spawn_credits(&mut commands, &theme),
            "Quitter" => {
                exit.write(AppExit::Success);
            }
            "overlay:close" => {
                for (entity, _) in &overlays {
                    commands.entity(entity).despawn();
                }
            }
            _ => {}
        }
    }
}

/// The prologue cards are menu business: forget the unread ones when the menu is left.
fn clear_prologue(mut modals: ResMut<ModalQueue>) {
    modals.clear();
}

fn after_load(mut loaded: MessageReader<GameLoaded>, mut next: MessageWriter<FadeToState>) {
    if loaded.read().last().is_some() {
        next.write(FadeToState(AppState::Campaign));
    }
}

/// « Batailles » submenu (`battles_menu.gd`): custom, historical and demo battles open their
/// screens (`battles`); « Rejeux » sends [`super::battles::OpenReplays`] (screen of the bscene2 lot).
fn spawn_battles(commands: &mut Commands, theme: &UiTheme) {
    let (overlay, panel) = spawn_overlay(commands, theme, "BattlesMenu", "Batailles", 480.0, 0);
    commands
        .entity(overlay)
        .insert((MainMenuOverlay, DespawnOnExit(AppState::MainMenu)));
    for (name, enabled) in [
        ("Bataille personnalisée", true),
        ("Batailles historiques", true),
        ("Batailles de démonstration", true),
        ("Rejeux", true),
    ] {
        let id = commands.spawn((button(theme, name), ChildOf(panel))).id();
        if !enabled {
            commands.entity(id).insert(InteractionDisabled);
        }
    }
    commands.spawn((
        button_captioned(theme, "overlay:close", "Fermer"),
        ChildOf(panel),
    ));
}

/// Credits: `CREDITS.md` of the repository root, simple Markdown turned into plain lines.
fn spawn_credits(commands: &mut Commands, theme: &UiTheme) {
    let text = paths::find_root()
        .ok()
        .and_then(|root| std::fs::read_to_string(root.join("CREDITS.md")).ok())
        .unwrap_or_else(|| {
            "Cent Ans — Benjamin Navet\nDéveloppé avec l'aide d'agents Claude.".into()
        });
    let (overlay, panel) = spawn_overlay(commands, theme, "Credits", "Crédits", 760.0, 0);
    commands
        .entity(overlay)
        .insert((MainMenuOverlay, DespawnOnExit(AppState::MainMenu)));
    let list = commands
        .spawn((scroll_list(theme, "credits:list", 420.0), ChildOf(panel)))
        .id();
    for line in text.lines() {
        let (content, variation) = if let Some(rest) = line.strip_prefix("## ") {
            (rest.to_owned(), UiType::Heading)
        } else if let Some(rest) = line.strip_prefix("# ") {
            (rest.to_owned(), UiType::Title)
        } else if let Some(rest) = line.strip_prefix("- ") {
            (format!("• {rest}"), UiType::Body)
        } else {
            (line.to_owned(), UiType::Body)
        };
        let content = content.replace("**", "").replace('`', "");
        if content.trim().is_empty() {
            continue;
        }
        commands.spawn((typed(theme, &content, variation), ChildOf(list)));
    }
    commands.spawn((
        button_captioned(theme, "overlay:close", "Fermer"),
        ChildOf(panel),
    ));
}
