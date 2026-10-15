//! « Rejeux » of the main menu (`ui/replays_menu.gd`): the last battles fought, recorded at their
//! end (`battle::scene::replay`), with their title, armies, winner, duration and date, and a
//! « Revoir » button that starts the battle screen in replay (`PendingReplay`).
//!
//! The menu lot sends [`OpenBattleReplays`] from its « Rejeux » entry; this module owns the
//! dialog. A file of another format is listed but cannot be watched. Dev hooks:
//! `CENT_ANS_OPEN_REPLAYS=1` opens the dialog on the main menu, `CENT_ANS_PLAY_REPLAY=<file>`
//! starts that replay from the main menu, `CENT_ANS_REPLAYS=<dir>` changes the folder.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use sim_battle::SideId;

use crate::app::AppState;
use crate::battle::scene::replay::{self, PendingReplay, ReplayEntry, duration_fr};
use crate::menu::KeyEdges;
use crate::ui::kit::style::{self, UiTier};
use crate::ui::kit::widgets::{button_primary, button_small, caption, panel_sized, scroll_list};
use crate::ui::kit::{ButtonPressed, UiTheme, UiType, typed};

/// Sent by the menu's « Rejeux » entry: opens the list of battle replays.
#[derive(Message, Debug, Clone, Copy)]
pub struct OpenBattleReplays;

const CLOSE: &str = "replays:close";

/// The open dialog (read by the dump and the buttons).
#[derive(Component, Debug, Clone)]
pub struct ReplaysDialog {
    pub entries: Vec<ReplayEntry>,
}

/// Replays started from the dialog or a script.
#[derive(Resource, Debug, Default, Clone)]
pub struct ReplayStats {
    pub opened: u32,
    pub started: u32,
    pub last_path: Option<String>,
}

pub struct ReplaysPlugin;

impl Plugin for ReplaysPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenBattleReplays>()
            .init_resource::<ReplayStats>()
            .add_systems(
                Update,
                (open_dialog, dialog_buttons, dev_hooks)
                    .chain()
                    .run_if(in_state(AppState::MainMenu)),
            );
    }
}

/// « de France », « d'Angleterre » (`BattleScene.de`).
pub fn de(name: &str) -> String {
    let first = name
        .chars()
        .next()
        .map(|c| c.to_lowercase().next().unwrap_or(c));
    match first {
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'h' | 'é' | 'è' | 'ê' | 'â' | 'î' | 'ô' | 'û') =>
        {
            format!("d’{name}")
        }
        _ => format!("de {name}"),
    }
}

/// « 26/09/2026 à 14 h 05 » (UTC; the Godot menu used the local time).
pub fn date_fr(unix: u64) -> String {
    if unix == 0 {
        return String::new();
    }
    let secs = unix as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{day:02}/{month:02}/{year} à {} h {:02}",
        rest / 3_600,
        rest % 3_600 / 60
    )
}

/// Summary line of a replay: armies, winner, duration, date.
pub fn summary(entry: &ReplayEntry) -> String {
    let verdict = match entry.winner {
        Some(SideId::Attacker) => format!("victoire {}", de(&entry.attacker)),
        Some(SideId::Defender) => format!("victoire {}", de(&entry.defender)),
        None => "bataille inachevée".to_owned(),
    };
    let mut parts = vec![
        format!("{} contre {}", entry.attacker, entry.defender),
        verdict,
        duration_fr(entry.duration),
    ];
    let date = date_fr(entry.recorded_at);
    if !date.is_empty() {
        parts.push(date);
    }
    parts.join(" · ")
}

fn open_dialog(
    mut commands: Commands,
    mut messages: MessageReader<OpenBattleReplays>,
    theme: Res<UiTheme>,
    existing: Query<(), With<ReplaysDialog>>,
    mut stats: ResMut<ReplayStats>,
) {
    if messages.read().last().is_none() || !existing.is_empty() {
        return;
    }
    let entries = replay::list_replays(&replay::replays_dir());
    stats.opened += 1;
    commands
        .spawn((
            Name::new("ReplaysDialog"),
            ReplaysDialog {
                entries: entries.clone(),
            },
            DespawnOnExit(AppState::MainMenu),
            UiTier::Modal.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.03, 0.02, 0.62)),
            Pickable::default(),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(&theme, 780.0)).with_children(|panel| {
                panel.spawn(typed(&theme, "Rejeux", UiType::Title));
                panel.spawn((
                    caption(
                        &theme,
                        "Revoyez les dernières batailles livrées, du premier trait à la déroute : lecture, pause, vitesse jusqu’à ×8, saut dans le temps, caméra libre. On regarde, on ne commande pas.",
                    ),
                    Node {
                        max_width: Val::Px(720.0),
                        ..default()
                    },
                ));
                panel
                    .spawn(scroll_list(&theme, "ReplaysList", 440.0))
                    .with_children(|list| {
                        if entries.is_empty() {
                            list.spawn((
                                Name::new("ReplaysEmpty"),
                                caption(
                                    &theme,
                                    "Aucun rejeu pour l’instant : chaque bataille livrée est enregistrée à sa fin.",
                                ),
                            ));
                        }
                        for (index, entry) in entries.iter().enumerate() {
                            list.spawn(Node {
                                column_gap: Val::Px(12.0),
                                align_items: AlignItems::Center,
                                margin: UiRect::vertical(Val::Px(4.0)),
                                ..default()
                            })
                            .with_children(|line| {
                                line.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    flex_grow: 1.0,
                                    flex_basis: Val::Px(0.0),
                                    ..default()
                                })
                                .with_children(|texts| {
                                    let title = if entry.title.is_empty() {
                                        "Bataille"
                                    } else {
                                        &entry.title
                                    };
                                    texts.spawn(typed(&theme, title, UiType::Heading));
                                    texts.spawn((
                                        caption(&theme, &summary(entry)),
                                        Node {
                                            max_width: Val::Px(540.0),
                                            ..default()
                                        },
                                    ));
                                    if !entry.readable {
                                        texts
                                            .spawn(caption(
                                                &theme,
                                                &format!(
                                                    "Enregistré par une autre version du jeu (format {}) : ce rejeu ne peut plus être revu.",
                                                    entry.format
                                                ),
                                            ))
                                            .insert(TextColor(style::RUBRIC));
                                    }
                                });
                                let mut button = line.spawn(button_small(
                                    &theme,
                                    &format!("replays:play:{index}"),
                                    "Revoir",
                                ));
                                if !entry.readable {
                                    button.insert(bevy::ui::InteractionDisabled);
                                }
                            });
                        }
                    });
                panel.spawn(button_primary(&theme, CLOSE, "Fermer"));
            });
        });
}

fn dialog_buttons(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    dialogs: Query<(Entity, &ReplaysDialog)>,
    mut keys: ResMut<KeyEdges>,
    mut next: ResMut<NextState<AppState>>,
    mut stats: ResMut<ReplayStats>,
) {
    let Ok((entity, dialog)) = dialogs.single() else {
        return;
    };
    if keys.escape {
        keys.escape = false;
        commands.entity(entity).despawn();
        return;
    }
    for press in pressed.read() {
        if press.name == CLOSE {
            commands.entity(entity).despawn();
            return;
        }
        if let Some(index) = press
            .name
            .strip_prefix("replays:play:")
            .and_then(|v| v.parse::<usize>().ok())
            && let Some(entry) = dialog.entries.get(index)
            && entry.readable
        {
            stats.started += 1;
            stats.last_path = Some(entry.path.to_string_lossy().into_owned());
            commands.insert_resource(PendingReplay::file(entry.path.clone()));
            next.set(AppState::Battle);
            return;
        }
    }
}

fn dev_hooks(
    mut commands: Commands,
    mut open: MessageWriter<OpenBattleReplays>,
    mut next: ResMut<NextState<AppState>>,
    mut stats: ResMut<ReplayStats>,
    mut frames: Local<u32>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *frames += 1;
    if *frames < 20 {
        return;
    }
    *done = true;
    let set = |name: &str| std::env::var(name).is_ok_and(|v| v != "0");
    if let Ok(file) = std::env::var("CENT_ANS_PLAY_REPLAY") {
        stats.started += 1;
        stats.last_path = Some(file.clone());
        commands.insert_resource(PendingReplay::file(PathBuf::from(file)));
        next.set(AppState::Battle);
    } else if set("CENT_ANS_OPEN_REPLAYS") {
        open.write(OpenBattleReplays);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_elides_before_a_vowel() {
        assert_eq!(de("France"), "de France");
        assert_eq!(de("Angleterre"), "d’Angleterre");
        assert_eq!(de("Écosse"), "d’Écosse");
    }

    #[test]
    fn dates_are_french() {
        // 2026-09-26 14:05:00 UTC.
        assert_eq!(date_fr(1_790_431_500), "26/09/2026 à 14 h 05");
        assert_eq!(date_fr(0), "");
    }

    #[test]
    fn summary_names_the_winner() {
        let entry = ReplayEntry {
            path: PathBuf::from("x.json"),
            title: "Bataille de Guyenne".into(),
            attacker: "France".into(),
            defender: "Angleterre".into(),
            winner: Some(SideId::Defender),
            duration: 725.0,
            recorded_at: 0,
            format: 1,
            readable: true,
        };
        assert_eq!(
            summary(&entry),
            "France contre Angleterre · victoire d’Angleterre · 12 min 05 s"
        );
    }
}
