//! Event journal, top left under the bar (port of the `EventLog` panel of `map_ui.gd` and of the
//! title / toggle of `journal_view.gd`): « Journal (n) » with a Replier / Déplier button, folded at
//! the start of a campaign, the lines of [`ChronicleLog`] (newest turn first) when unfolded. It is
//! the first child of the advisor column, so the hint card and the missions stack under it.
//! The genre filter and the « Monde » tab stay in the Chronicle window (F-key panel).

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use serde_json::{Value, json};

use super::{AdvisorColumn, HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::campaign::panels::chronicle::log::ChronicleLog;
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::widgets::{button_small, caption, panel_note, scroll_list};
use crate::ui::kit::{ButtonPressed, UiTheme};

const WIDTH: f32 = 280.0;
const LIST_HEIGHT: f32 = 150.0;
const TOGGLE: &str = "hud:journal_toggle";

/// Folded at the start (Godot `set_log_expanded(false)`).
#[derive(Resource, Debug, Default)]
pub struct JournalState {
    pub expanded: bool,
}

#[derive(Component)]
struct JournalCard;

/// Lines of the newest turn (what Godot's title counts), date header excluded.
pub fn newest_count(log: &ChronicleLog) -> usize {
    log.lines
        .iter()
        .skip(1)
        .take_while(|(genre, _)| !genre.is_empty())
        .count()
}

pub fn title(log: &ChronicleLog) -> String {
    match newest_count(log) {
        0 => "Journal".to_owned(),
        n => format!("Journal ({n})"),
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<JournalState>()
        .register_dump(dump_journal)
        .add_systems(Update, (toggle, rebuild).chain().in_set(HudSet::Ui));
}

fn toggle(mut pressed: MessageReader<ButtonPressed>, mut state: ResMut<JournalState>) {
    if pressed.read().any(|p| p.name == TOGGLE) {
        state.expanded = !state.expanded;
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    state: Res<JournalState>,
    log: Res<ChronicleLog>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    column: Query<Entity, With<AdvisorColumn>>,
    card: Query<Entity, With<JournalCard>>,
    mut built: Local<Option<(bool, u32, usize)>>,
) {
    let Ok(column) = column.single() else { return };
    let key = (state.expanded, log.turns, log.lines.len());
    let existing = card.single().ok();
    if existing.is_some() && *built == Some(key) {
        return;
    }
    if let Some(entity) = existing {
        commands.entity(entity).despawn();
    }
    *built = Some(key);
    let body = RichStyle::caption(&theme);
    let arrow = if state.expanded {
        "▾ Replier"
    } else {
        "▸ Déplier"
    };
    let card = commands
        .spawn((
            Name::new("hud:journal"),
            JournalCard,
            HudWidget,
            Hovered::default(),
            panel_note(&theme),
        ))
        .insert(Node {
            width: Val::Px(WIDTH),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        })
        .with_children(|box_| {
            box_.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|header| {
                header.spawn(caption(&theme, &title(&log)));
                header.spawn(button_small(&theme, TOGGLE, arrow));
            });
            if !state.expanded {
                return;
            }
            box_.spawn(scroll_list(&theme, "hud:journal_list", LIST_HEIGHT))
                .with_children(|list| {
                    if log.lines.is_empty() {
                        list.spawn(caption(&theme, "Aucun événement pour l’instant."));
                    }
                    for (_, text) in &log.lines {
                        list.spawn(rich_text(&theme, &assets, text, &body));
                    }
                });
        })
        .id();
    // First of the column: the hint and the missions stack under the journal.
    commands.entity(column).insert_children(0, &[card]);
}

fn dump_journal(world: &World) -> (String, Value) {
    let state = world.resource::<JournalState>();
    let log = world.resource::<ChronicleLog>();
    let mut cards = world.try_query_filtered::<(), With<JournalCard>>();
    (
        "journal".to_owned(),
        json!({
            "expanded": state.expanded,
            "title": title(log),
            "lines": log.lines.len(),
            "built": cards.as_mut().map_or(0, |q| q.iter(world).count()),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_counts_newest_turn_only() {
        let mut log = ChronicleLog::default();
        assert_eq!(title(&log), "Journal");
        log.lines = vec![
            (String::new(), "[b]— a —[/b]".into()),
            ("war".into(), "x".into()),
            ("war".into(), "y".into()),
            (String::new(), "[b]— b —[/b]".into()),
            ("war".into(), "z".into()),
        ];
        assert_eq!(title(&log), "Journal (2)");
    }

    #[test]
    fn starts_folded() {
        assert!(!JournalState::default().expanded);
    }
}
