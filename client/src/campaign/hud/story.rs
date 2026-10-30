//! « Récit de la saison » (XP-POURQUOI, R4): the non-modal panel that replaces the `auto` season
//! report. Three blocks drawn from `game_api::causes::season_story`: what changed, why (the
//! causes of the journal), what comes. Collapsible in one click; the `always` mode keeps its
//! window and `off` shows nothing. No rule here: the client only draws the story.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use game_api::causes::SeasonStory;
use serde_json::{Value, json};

use super::{AdvisorColumn, HudData, HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::settings::Settings;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::{button_small, caption, panel_note};
use crate::ui::kit::{ButtonPressed, UiTheme};

const PANEL_WIDTH: f32 = 300.0;

/// Whether the player folded the panel (kept across seasons).
#[derive(Resource, Default)]
pub struct StoryPanel {
    pub collapsed: bool,
}

#[derive(Component)]
struct StoryRoot;

/// One block of the panel: its title and its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub title: String,
    pub lines: Vec<String>,
}

/// The three blocks of `story`, in order; empty blocks keep a line that says so.
pub fn blocks(story: &SeasonStory) -> Vec<Block> {
    let ui = game_api::causes::ui_text;
    let mut changed: Vec<String> = story.changed.iter().map(|l| l.text.clone()).collect();
    if let Some(act) = story.act_delta.as_ref().filter(|a| !a.text.is_empty()) {
        changed.push(act.text.clone());
    }
    if changed.is_empty() {
        changed.push(ui("story_changed_none"));
    }
    let why: Vec<String> = if story.why.is_empty() {
        vec![ui("story_why_none")]
    } else {
        story
            .why
            .iter()
            .map(|cause| {
                let top: Vec<&str> = cause
                    .factors
                    .iter()
                    .take(game_api::causes::SUMMARY_FACTORS)
                    .map(|f| f.text.as_str())
                    .collect();
                if top.is_empty() {
                    cause.summary.clone()
                } else {
                    format!("{} — {}", cause.summary, top.join(" ; "))
                }
            })
            .collect()
    };
    let mut next: Vec<String> = story.next.iter().map(|l| l.text.clone()).collect();
    if next.is_empty() {
        next.push(ui("story_next_none"));
    }
    vec![
        Block {
            title: ui("story_changed"),
            lines: changed,
        },
        Block {
            title: ui("story_why"),
            lines: why,
        },
        Block {
            title: ui("story_next"),
            lines: next,
        },
    ]
}

pub fn plugin(app: &mut App) {
    app.init_resource::<StoryPanel>()
        .register_dump(dump_story)
        .add_systems(
            Update,
            (rebuild_panel, handle_toggle).chain().in_set(HudSet::Ui),
        );
}

#[allow(clippy::too_many_arguments)]
fn rebuild_panel(
    mut commands: Commands,
    hud: Res<HudData>,
    panel: Res<StoryPanel>,
    theme: Res<UiTheme>,
    settings: Option<Res<Settings>>,
    column: Query<Entity, With<AdvisorColumn>>,
    existing: Query<Entity, With<StoryRoot>>,
    mut built: Local<Option<(Option<SeasonStory>, bool, bool)>>,
) {
    let Ok(column) = column.single() else { return };
    // The Récit is the `auto` mode of the season report (`always` keeps its window).
    let auto = settings
        .as_deref()
        .is_none_or(|s| !matches!(s.season_report.as_str(), "always" | "off"));
    let want = (hud.story.clone(), panel.collapsed, auto);
    let existing = existing.single().ok();
    if built.as_ref() == Some(&want) && existing.is_some() == (want.2 && want.0.is_some()) {
        return;
    }
    if let Some(entity) = existing {
        commands.entity(entity).despawn();
    }
    *built = Some(want.clone());
    let (Some(story), true) = (want.0, auto) else {
        return;
    };
    let arrow = if panel.collapsed { "▸" } else { "▾" };
    commands
        .spawn((
            Name::new("hud:story"),
            StoryRoot,
            ChildOf(column),
            HudWidget,
            Hovered::default(),
            panel_note(&theme),
        ))
        .insert(Node {
            width: Val::Px(PANEL_WIDTH),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        })
        .with_children(|box_| {
            box_.spawn(button_small(
                &theme,
                "hud:story_toggle",
                &format!(
                    "{arrow} {} — {}",
                    game_api::causes::ui_text("story_title"),
                    story.date_label
                ),
            ));
            if panel.collapsed {
                return;
            }
            for block in blocks(&story) {
                box_.spawn((
                    Text::new(block.title),
                    theme.text_font(true, false, UiType::Body.size()),
                    TextColor(style::INK_TITLE),
                ));
                for line in block.lines {
                    box_.spawn(caption(&theme, &line));
                }
            }
        });
}

fn handle_toggle(mut pressed: MessageReader<ButtonPressed>, mut panel: ResMut<StoryPanel>) {
    for press in pressed.read() {
        if press.name == "hud:story_toggle" {
            panel.collapsed = !panel.collapsed;
        }
    }
}

/// `--dump-state` section `season_story`.
fn dump_story(world: &World) -> (String, Value) {
    let hud = world.resource::<HudData>();
    let panel = world.resource::<StoryPanel>();
    let value = hud.story.as_ref().map_or(Value::Null, |story| {
        json!({
            "turn": story.turn,
            "date": story.date_label,
            "changed": story.changed.iter().map(|l| &l.text).collect::<Vec<_>>(),
            "why": story.why.iter().map(|c| json!({
                "id": c.id, "kind": c.kind, "turn": c.turn, "severity": c.severity,
                "summary": c.summary,
                "factors": c.factors.iter().map(|f| &f.text).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "next": story.next.iter().map(|l| &l.text).collect::<Vec<_>>(),
            "act_delta": story.act_delta.as_ref().map(|a| json!({
                "act": a.act_number, "gained": a.renown_gained, "terms": a.terms,
            })),
        })
    });
    (
        "season_story".to_owned(),
        json!({"story": value, "collapsed": panel.collapsed}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_api::causes::StoryLine;

    fn story() -> SeasonStory {
        SeasonStory {
            turn: 3,
            date_label: "Automne 1337".to_owned(),
            changed: vec![StoryLine {
                figure: "treasury".to_owned(),
                label: "Trésor".to_owned(),
                delta: -2140,
                text: "Trésor : ▼ \u{2212}2\u{202f}140".to_owned(),
            }],
            why: Vec::new(),
            next: Vec::new(),
            act_delta: None,
        }
    }

    #[test]
    fn three_blocks_always_say_something() {
        let blocks = blocks(&story());
        assert_eq!(blocks.len(), 3);
        assert!(blocks.iter().all(|b| !b.lines.is_empty()));
        assert!(blocks[0].lines[0].starts_with("Trésor : ▼"));
        assert_eq!(
            blocks[1].lines,
            [game_api::causes::ui_text("story_why_none")]
        );
    }
}
