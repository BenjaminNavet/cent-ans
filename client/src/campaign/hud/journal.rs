//! Event journal, top left under the bar (port of the `EventLog` panel of `map_ui.gd` and of the
//! title / toggle of `journal_view.gd`): « Journal (n) » with a Replier / Déplier button, folded at
//! the start of a campaign, the lines of [`ChronicleLog`] (newest turn first) when unfolded. It is
//! the first child of the advisor column, so the hint card and the missions stack under it.
//! The genre filter (`journal_genres.json`) and the « Monde » block (far news, folded) are the
//! same state as in the Chronicle window ([`ChroniclePanel`]), like Godot's static
//! `remembered_genre`; the buttons are `hud:journal_genre:<id>` and `hud:journal_world`.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use serde_json::{Value, json};

use super::{AdvisorColumn, HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::campaign::panels::chronicle::ChroniclePanel;
use crate::campaign::panels::chronicle::log::ChronicleLog;
use crate::ui::art_plates::ArtPlates;
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::widgets::{button_small, caption, panel_note, scroll_list, toggle_button};
use crate::ui::kit::{ButtonPressed, UiTheme};
use game_api::chronicle::genre_choices;

const WIDTH: f32 = 280.0;
const LIST_HEIGHT: f32 = 150.0;
/// Height of the vignette of the newest turn above the lines.
const VIGNETTE_HEIGHT: f32 = 64.0;
const TOGGLE: &str = "hud:journal_toggle";
const GENRE_PREFIX: &str = "hud:journal_genre:";
const WORLD: &str = "hud:journal_world";

/// Folded at the start (Godot `set_log_expanded(false)`).
#[derive(Resource, Debug, Default)]
pub struct JournalState {
    pub expanded: bool,
}

/// What the card was built for: unfolded, turns, lines, genre, « Monde » open.
type BuiltKey = (bool, u32, usize, String, bool);

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

/// Asset path of the vignette of the newest turn's most urgent genre
/// (`ArtPlates.vignette_for_events`, as for the season report).
pub fn vignette_path(log: &ChronicleLog, plates: Option<&ArtPlates>) -> Option<String> {
    plates?
        .vignette_for_events(log.newest_kinds.iter().map(String::as_str))
        .filter(|v| !v.image.is_empty())
        .map(|v| v.image.clone())
}

pub fn plugin(app: &mut App) {
    app.init_resource::<JournalState>()
        .register_dump(dump_journal)
        .add_systems(Update, (toggle, rebuild).chain().in_set(HudSet::Ui));
}

fn toggle(
    mut pressed: MessageReader<ButtonPressed>,
    mut state: ResMut<JournalState>,
    mut panel: ResMut<ChroniclePanel>,
) {
    for press in pressed.read() {
        if press.name == TOGGLE {
            state.expanded = !state.expanded;
        } else if press.name == WORLD {
            panel.toggle_world();
        } else if let Some(genre) = press.name.strip_prefix(GENRE_PREFIX) {
            panel.set_genre(genre);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    state: Res<JournalState>,
    log: Res<ChronicleLog>,
    panel: Res<ChroniclePanel>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    plates: Option<Res<ArtPlates>>,
    column: Query<Entity, With<AdvisorColumn>>,
    card: Query<Entity, With<JournalCard>>,
    mut built: Local<Option<BuiltKey>>,
) {
    let Ok(column) = column.single() else { return };
    let genre = panel.genre().to_owned();
    let key = (
        state.expanded,
        log.turns,
        log.lines.len(),
        genre.clone(),
        panel.world_open(),
    );
    let existing = card.single().ok();
    if existing.is_some() && built.as_ref() == Some(&key) {
        return;
    }
    if let Some(entity) = existing {
        commands.entity(entity).despawn();
    }
    *built = Some(key);
    let world_open = panel.world_open();
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
            if let Some(path) = vignette_path(&log, plates.as_deref()) {
                box_.spawn(super::cover::cover_image(
                    &assets,
                    &path,
                    "hud:journal_vignette".to_owned(),
                    WIDTH - 12.0,
                    VIGNETTE_HEIGHT,
                ));
            }
            box_.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(3.0),
                row_gap: Val::Px(3.0),
                ..default()
            })
            .with_children(|bar| {
                for choice in genre_choices() {
                    let name = format!("{GENRE_PREFIX}{}", choice.id);
                    bar.spawn(toggle_button(
                        &theme,
                        &name,
                        &choice.label,
                        genre == choice.id,
                    ));
                }
            });
            box_.spawn(scroll_list(&theme, "hud:journal_list", LIST_HEIGHT))
                .with_children(|list| {
                    let shown = log.shown_lines(&genre);
                    if log.lines.is_empty() {
                        list.spawn(caption(&theme, "Aucun événement pour l’instant."));
                    } else if shown.is_empty() {
                        list.spawn(caption(&theme, "Rien de ce genre."));
                    }
                    for (_, text) in &shown {
                        list.spawn(rich_text(&theme, &assets, text, &body));
                    }
                    if log.world_count() > 0 {
                        let label = format!(
                            "{} Monde ({})",
                            if world_open { "▾" } else { "▸" },
                            log.world_count()
                        );
                        list.spawn(toggle_button(&theme, WORLD, &label, world_open));
                        if world_open {
                            for (_, text) in log.shown_world(&genre) {
                                list.spawn(rich_text(&theme, &assets, &text, &body));
                            }
                        }
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
    let panel = world.resource::<ChroniclePanel>();
    let mut cards = world.try_query_filtered::<(), With<JournalCard>>();
    (
        "journal".to_owned(),
        json!({
            "expanded": state.expanded,
            "title": title(log),
            "lines": log.lines.len(),
            "genre": panel.genre(),
            "world_open": panel.world_open(),
            "world_lines": log.world_count(),
            "shown": log.shown_lines(panel.genre()).len(),
            "vignette": vignette_path(log, world.get_resource::<ArtPlates>()),
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
    fn genre_and_world_buttons_share_the_chronicle_state() {
        let mut app = App::new();
        app.init_resource::<JournalState>()
            .init_resource::<ChroniclePanel>()
            .add_message::<ButtonPressed>()
            .add_systems(Update, toggle);
        assert_eq!(app.world().resource::<ChroniclePanel>().genre(), "all");
        for name in ["hud:journal_genre:war", "hud:journal_world"] {
            app.world_mut().write_message(ButtonPressed {
                entity: Entity::PLACEHOLDER,
                name: name.to_owned(),
            });
            app.update();
        }
        let panel = app.world().resource::<ChroniclePanel>();
        assert_eq!(panel.genre(), "war");
        assert!(panel.world_open());
        assert!(!app.world().resource::<JournalState>().expanded);
    }

    #[test]
    fn journal_vignette_follows_the_newest_turn_genres() {
        let plates = ArtPlates::parse(&json!({"vignettes": [
            {"id": "vg_war", "kinds": ["war_declared"], "image": "res://assets/art/vignettes/w.jpg"},
            {"id": "vg_birth", "kinds": ["birth"], "image": "res://assets/art/vignettes/b.jpg"}]}));
        let mut log = ChronicleLog::default();
        assert_eq!(vignette_path(&log, Some(&plates)), None);
        log.newest_kinds = vec!["birth".into(), "war_declared".into()];
        assert_eq!(
            vignette_path(&log, Some(&plates)).as_deref(),
            Some("art/vignettes/w.jpg")
        );
        assert_eq!(vignette_path(&log, None), None);
    }

    #[test]
    fn starts_folded() {
        assert!(!JournalState::default().expanded);
    }
}
