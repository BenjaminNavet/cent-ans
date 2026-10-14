//! Main menu screen: title, "Nouvelle partie" (-> Campaign with the default faction), "Quitter".
//!
//! Will port game/scenes/start_menu.tscn and game/scripts/ui/ start-menu scripts (battles menu,
//! demos menu); the faction selection (game/scripts/ui/faction_select.gd) comes with its own lot.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;

use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::{ButtonPressed, UiTheme, button, label, panel, title};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_menu)
            .add_systems(Update, handle_buttons.run_if(in_state(AppState::MainMenu)));
    }
}

fn spawn_menu(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            Name::new("MainMenu"),
            DespawnOnExit(AppState::MainMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(panel(&theme)).with_children(|card| {
                card.spawn(title(&theme, "Cent Ans"));
                card.spawn(label(&theme, "Grande stratégie de la guerre de Cent Ans"));
                card.spawn(button(&theme, "Nouvelle partie"));
                card.spawn(button(&theme, "Quitter"));
            });
        });
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut sim: ResMut<Sim>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for press in pressed.read() {
        match press.name.as_str() {
            "Nouvelle partie" => {
                sim.state = None;
                next.set(AppState::Campaign);
            }
            "Quitter" => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}
