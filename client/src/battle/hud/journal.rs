//! Battle journal (top right, folded with J). Port of battle_hud.gd `_build_log`, `add_events`,
//! `_render_log`, `toggle_log`: the simulation's events (`BattleSim::events`, French text), the
//! newest first, at most eight lines; a repeated line within eight seconds becomes « (×2) »;
//! folded, only the newest line shows.

use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};

use super::BattleHudRoot;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{heading, panel_note};
use crate::ui::kit::{ButtonPressed, UiTheme};

pub const MAX_LOG: usize = 8;
/// Two entries with the same text closer than this merge (`GROUP_SECONDS`).
pub const GROUP_SECONDS: f64 = 8.0;
const TOGGLE_NAME: &str = "journal:toggle";

#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub time: f64,
    pub text: String,
    pub count: u32,
}

/// The journal's state (pure, unit-tested).
#[derive(Resource, Debug, Clone)]
pub struct LogBook {
    /// Newest first.
    pub entries: Vec<LogEntry>,
    pub expanded: bool,
    /// Events of the simulation already read.
    pub consumed: usize,
    pub revision: u64,
}

impl Default for LogBook {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            expanded: true,
            consumed: 0,
            revision: 0,
        }
    }
}

impl LogBook {
    pub fn add(&mut self, time: f64, text: &str) {
        let mut merged = false;
        for entry in &mut self.entries {
            if time - entry.time > GROUP_SECONDS {
                break;
            }
            if entry.text == text {
                entry.count += 1;
                entry.time = time;
                merged = true;
                break;
            }
        }
        if !merged {
            self.entries.insert(
                0,
                LogEntry {
                    time,
                    text: text.to_owned(),
                    count: 1,
                },
            );
        }
        self.entries.truncate(MAX_LOG);
        self.revision += 1;
    }

    /// Lines shown now (`_render_log`).
    pub fn lines(&self) -> Vec<String> {
        let shown = if self.expanded {
            self.entries.len()
        } else {
            self.entries.len().min(1)
        };
        self.entries[..shown].iter().map(line_text).collect()
    }
}

/// `mm:ss  text (×n)`.
pub fn line_text(entry: &LogEntry) -> String {
    let seconds = entry.time.max(0.0) as u64;
    let mut text = format!("{:02}:{:02}  {}", seconds / 60, seconds % 60, entry.text);
    if entry.count > 1 {
        text += &format!(" (×{})", entry.count);
    }
    text
}

#[derive(Component)]
struct JournalLines;

#[derive(Component)]
struct JournalPanel;

#[derive(Component)]
struct JournalToggleLabel;

pub struct JournalPlugin;

impl Plugin for JournalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogBook>()
            .add_systems(OnEnter(AppState::Battle), reset)
            .add_systems(
                Update,
                (spawn, collect, toggle, render)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

fn reset(mut book: ResMut<LogBook>) {
    *book = LogBook::default();
}

fn spawn(
    mut commands: Commands,
    roots: Query<Entity, With<BattleHudRoot>>,
    existing: Query<(), With<JournalPanel>>,
    theme: Res<UiTheme>,
) {
    let Ok(root) = roots.single() else { return };
    if !existing.is_empty() {
        return;
    }
    let wrapper = commands
        .spawn((
            Name::new("BattleLog"),
            JournalPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(70.0),
                width: Val::Px(348.0),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    let panel = commands.spawn((panel_note(&theme), ChildOf(wrapper))).id();
    let header = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    commands.spawn((heading(&theme, "Journal de bataille"), ChildOf(header)));
    let toggle = commands
        .spawn((
            Name::new(TOGGLE_NAME),
            Button,
            Tooltip::new("[b]Journal[/b]\nReplier ou déplier (J)."),
            Node {
                padding: UiRect::horizontal(Val::Px(4.0)),
                ..default()
            },
            ChildOf(header),
        ))
        .id();
    commands.spawn((
        JournalToggleLabel,
        Text::new("▾ replier"),
        theme.text_font(false, false, UiType::Caption.size()),
        TextColor(style::INK_SOFT),
        Pickable::IGNORE,
        ChildOf(toggle),
    ));
    commands.spawn((
        JournalLines,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(1.0),
            ..default()
        },
        ChildOf(panel),
    ));
}

/// Reads the new events of the simulation.
fn collect(mut session: ResMut<BattleSession>, mut book: ResMut<LogBook>) {
    let sim = session.sim_mut();
    let events = sim.events();
    if events.len() < book.consumed {
        book.consumed = 0;
    }
    let fresh: Vec<(f64, String)> = events[book.consumed..]
        .iter()
        .map(|e| (e.time, e.text_fr.clone()))
        .collect();
    book.consumed = events.len();
    for (time, text) in fresh {
        book.add(time, &text);
    }
}

fn toggle(
    mut pressed: MessageReader<ButtonPressed>,
    keys: Res<ButtonInput<KeyCode>>,
    mut book: ResMut<LogBook>,
) {
    let mut flip = keys.just_pressed(KeyCode::KeyJ);
    for press in pressed.read() {
        flip |= press.name == TOGGLE_NAME;
    }
    if flip {
        book.expanded = !book.expanded;
        book.revision += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    book: Res<LogBook>,
    theme: Res<UiTheme>,
    containers: Query<(Entity, Option<&Children>), With<JournalLines>>,
    mut labels: Query<&mut Text, With<JournalToggleLabel>>,
    mut drawn: Local<Option<u64>>,
) {
    if *drawn == Some(book.revision) {
        return;
    }
    let Ok((container, children)) = containers.single() else {
        return;
    };
    *drawn = Some(book.revision);
    for child in children.map(|c| c.to_vec()).unwrap_or_default() {
        commands.entity(child).despawn();
    }
    for (index, line) in book.lines().into_iter().enumerate() {
        commands.spawn((
            Text::new(line),
            theme.text_font(false, false, 13.0),
            TextColor(if index == 0 {
                style::INK
            } else {
                style::INK.with_alpha(0.8)
            }),
            ChildOf(container),
        ));
    }
    for mut label in &mut labels {
        label.0 = if book.expanded {
            "▾ replier"
        } else {
            "▸ déplier"
        }
        .to_owned();
    }
}

pub fn dump(world: &World) -> Value {
    let book = world.resource::<LogBook>();
    json!({
        "entries": book.entries.len(),
        "expanded": book.expanded,
        "consumed": book.consumed,
        "lines": book.lines(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_within_eight_seconds_merge() {
        let mut book = LogBook::default();
        book.add(10.0, "Les Archers plantent leurs pieux");
        book.add(14.0, "Les Archers plantent leurs pieux");
        book.add(30.0, "Les Archers plantent leurs pieux");
        assert_eq!(book.entries.len(), 2);
        assert_eq!(book.entries[1].count, 2);
        assert_eq!(
            line_text(&book.entries[1]),
            "00:14  Les Archers plantent leurs pieux (×2)"
        );
    }

    #[test]
    fn keeps_eight_newest_and_folds_to_one() {
        let mut book = LogBook::default();
        for i in 0..12 {
            book.add(f64::from(i) * 20.0, &format!("événement {i}"));
        }
        assert_eq!(book.entries.len(), MAX_LOG);
        assert!(book.entries[0].text.ends_with("11"));
        assert_eq!(book.lines().len(), MAX_LOG);
        book.expanded = false;
        assert_eq!(book.lines().len(), 1);
        assert!(book.lines()[0].starts_with("03:40"));
    }
}
