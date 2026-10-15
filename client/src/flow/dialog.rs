//! Pre-battle dialog (`pre_battle_dialog.gd`): force balance bar and the core's verdict, both
//! armies (general, reinforcements, regiment cards), conditions, then « Combattre » /
//! « Résolution automatique » / « Retraite ». The player's side is always on the left.
//!
//! The brief itself (texts, chances, regiments) comes from `game_api::flow::battle_brief`.
//! Not ported: the illustrated banner, the site mini-map and the weather preview (they read a
//! preview `BattleSim`), heraldry and general medallions, the hover bubbles on the buttons.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use game_api::flow::{BattleBrief, BriefSide};

use crate::app::AppState;
use crate::battle::scene::setup::side_color;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::{button_primary, button_small, caption, panel_sized, separator};
use crate::ui::kit::{UiTheme, fr, typed};
use data_model::GameData;
use sim_battle::SideId;

/// The open dialog and what it shows (read by the dump and the button handler).
#[derive(Component, Debug, Clone)]
pub struct PreBattleDialog {
    pub index: usize,
    pub title: String,
    pub verdict: String,
    pub share: f64,
    pub regiments: [usize; 2],
    pub can_withdraw: bool,
}

/// Names of the three buttons.
pub const FIGHT: &str = "flow:fight";
pub const AUTO: &str = "flow:auto";
pub const WITHDRAW: &str = "flow:withdraw";

fn tone_color(tone: &str) -> Color {
    match tone {
        "good" => style::GOOD,
        "fair" => Color::srgb(0.55, 0.40, 0.08),
        _ => style::RUBRIC,
    }
}

pub fn spawn(commands: &mut Commands, theme: &UiTheme, game: &GameData, brief: &BattleBrief) {
    let dialog = PreBattleDialog {
        index: brief.index,
        title: brief.title.clone(),
        verdict: brief.verdict.clone(),
        share: brief.share,
        regiments: [brief.sides[0].regiments, brief.sides[1].regiments],
        can_withdraw: brief.can_withdraw,
    };
    let colors: [Color; 2] = [0, 1].map(|slot| {
        let side = &brief.sides[slot];
        let id = if side.side == "attacker" {
            SideId::Attacker
        } else {
            SideId::Defender
        };
        let [r, g, b] = side_color(game, &side.faction, id);
        Color::srgb(r, g, b)
    });
    let colors = if colors[0] == colors[1] {
        [colors[0], colors[1].darker(0.2)]
    } else {
        colors
    };
    commands
        .spawn((
            Name::new("flow:pre_battle"),
            dialog,
            DespawnOnExit(AppState::Campaign),
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
            root.spawn(panel_sized(theme, 1080.0))
                .with_children(|panel| {
                    panel.spawn(typed(theme, &brief.title, UiType::Title));
                    panel.spawn(caption(theme, &brief.subtitle));
                    panel.spawn(separator(theme));
                    // Verdict and balance bar.
                    panel
                        .spawn((
                            Name::new("flow:verdict"),
                            typed(theme, &brief.verdict, UiType::Heading),
                        ))
                        .insert(TextColor(tone_color(&brief.verdict_tone)));
                    panel
                        .spawn((
                            Name::new("flow:balance"),
                            Node {
                                width: Val::Px(620.0),
                                height: Val::Px(18.0),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderColor::all(style::INK),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                Node {
                                    width: Val::Percent(
                                        (brief.share.clamp(0.0, 1.0) * 100.0) as f32,
                                    ),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(colors[0]),
                            ));
                            bar.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(colors[1]),
                            ));
                        });
                    panel.spawn((
                        caption(theme, &brief.chance_text),
                        Node {
                            max_width: Val::Px(1000.0),
                            ..default()
                        },
                    ));
                    panel.spawn(separator(theme));
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            column_gap: Val::Px(24.0),
                            align_items: AlignItems::FlexStart,
                            ..default()
                        })
                        .with_children(|columns| {
                            for (slot, side) in brief.sides.iter().enumerate() {
                                side_column(columns, theme, side, slot, colors[slot]);
                            }
                        });
                    panel.spawn(separator(theme));
                    panel.spawn(caption(theme, &brief.conditions));
                    if !brief.modifiers.is_empty() {
                        panel.spawn((
                            Name::new("flow:modifiers"),
                            caption(theme, &brief.modifiers.join(" · ")),
                            Node {
                                max_width: Val::Px(1000.0),
                                ..default()
                            },
                        ));
                    }
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            column_gap: Val::Px(18.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            let mut withdraw =
                                row.spawn(button_small(theme, WITHDRAW, &brief.withdraw_label));
                            if !brief.can_withdraw {
                                withdraw.insert(bevy::ui::InteractionDisabled);
                            }
                            if !brief.auto_risk.is_empty() {
                                row.spawn((
                                    Name::new("flow:auto_risk"),
                                    caption(theme, &brief.auto_risk),
                                ))
                                .insert(TextColor(style::RUBRIC));
                            }
                            row.spawn(button_small(theme, AUTO, "Résolution automatique"));
                            row.spawn(button_primary(theme, FIGHT, &brief.fight_label));
                        });
                });
        });
}

fn side_column(
    columns: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    side: &BriefSide,
    slot: usize,
    color: Color,
) {
    columns
        .spawn((
            Name::new(format!("flow:side:{}", side.side)),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                row_gap: Val::Px(4.0),
                align_items: if slot == 0 {
                    AlignItems::FlexStart
                } else {
                    AlignItems::FlexEnd
                },
                ..default()
            },
        ))
        .with_children(|column| {
            let you = if slot == 0 { "  (vous)" } else { "" };
            column
                .spawn(typed(
                    theme,
                    &format!("{}{you}", side.name),
                    UiType::Heading,
                ))
                .insert(TextColor(color.darker(0.1)));
            let role = {
                let mut chars = side.role.chars();
                chars
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            };
            column.spawn(caption(
                theme,
                &format!(
                    "{role} · {} hommes en {} régiments",
                    fr::digits(i64::from(side.soldiers)),
                    side.regiments
                ),
            ));
            let general = side.general.as_ref().map_or_else(
                || "L’ost combat sans général : moral fragile.".to_owned(),
                |g| {
                    format!(
                        "{} — {}{}",
                        g.name,
                        "★".repeat(usize::from(g.command.min(10))),
                        "☆".repeat(usize::from(10 - g.command.min(10)))
                    )
                },
            );
            column.spawn((Name::new("flow:general"), caption(theme, &general)));
            let reinforcements = if side.reinforcements.is_empty() {
                "Aucun renfort à portée.".to_owned()
            } else {
                format!("Renforts : {}", side.reinforcements)
            };
            column.spawn(caption(theme, &reinforcements));
            // Regiment cards.
            column
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    justify_content: if slot == 0 {
                        JustifyContent::FlexStart
                    } else {
                        JustifyContent::FlexEnd
                    },
                    ..default()
                })
                .with_children(|grid| {
                    for unit in &side.units {
                        grid.spawn((
                            Name::new("flow:regiment"),
                            Node {
                                width: Val::Px(76.0),
                                min_height: Val::Px(52.0),
                                padding: UiRect::all(Val::Px(3.0)),
                                flex_direction: FlexDirection::Column,
                                border: UiRect::all(Val::Px(if unit.is_general {
                                    2.0
                                } else {
                                    1.0
                                })),
                                ..default()
                            },
                            BackgroundColor(style::PARCHMENT_LIGHT),
                            BorderColor::all(if unit.is_general { style::GOLD } else { color }),
                            children![
                                (
                                    Text::new(unit.name.clone()),
                                    theme.text_font(false, false, 11.0),
                                    TextColor(style::INK),
                                ),
                                (
                                    Text::new(if unit.soldiers == unit.max_soldiers {
                                        format!("{}", unit.soldiers)
                                    } else {
                                        format!("{}/{}", unit.soldiers, unit.max_soldiers)
                                    }),
                                    theme.text_font(true, false, 12.0),
                                    TextColor(style::INK_SOFT),
                                ),
                            ],
                        ));
                    }
                });
            column.spawn((
                caption(theme, &side.composition),
                Node {
                    max_width: Val::Px(500.0),
                    ..default()
                },
            ));
        });
}
