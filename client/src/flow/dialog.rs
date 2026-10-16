//! Pre-battle dialog (`pre_battle_dialog.gd`): force balance bar and the core's verdict, both
//! armies (general, reinforcements, regiment cards), conditions, then « Combattre » /
//! « Résolution automatique » / « Retraite ». The player's side is always on the left.
//!
//! The brief itself (texts, chances, regiments) comes from `game_api::flow::battle_brief`.
//! Illustrated banner ([`banner`]: a loading plate of `ui::art_plates::ArtPlates`, cover-cropped,
//! darkened towards the gold rule, title and subtitle over it) and the arms of each side
//! ([`arms`], `heraldry/<faction>.png`); both are shared with the naval dialog.
//! Not ported: the site mini-map and the weather preview (they read a preview `BattleSim`),
//! general medallions, the hover bubbles on the buttons.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use game_api::flow::{BattleBrief, BriefSide};

use crate::app::AppState;
use crate::battle::scene::setup::side_color;
use crate::paths;
use crate::ui::art_plates::ArtPlates;
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

/// Banner height (Godot `BANNER_HEIGHT`; `BANNER_HEIGHT_SHORT` under 760 px of window).
pub const BANNER_HEIGHT: f32 = 132.0;
pub const BANNER_HEIGHT_SHORT: f32 = 84.0;
const BANNER_TITLE: Color = Color::srgb(0.99, 0.95, 0.84);
const BANNER_SUBTITLE: Color = Color::srgb(0.96, 0.88, 0.68);
const BANNER_FALLBACK: Color = Color::srgb(0.35, 0.22, 0.12);

/// What the dialogs need to draw their pictures.
pub struct DialogArt<'a> {
    pub assets: &'a AssetServer,
    pub plates: &'a ArtPlates,
    /// Window height (logical px), for the short banner.
    pub window_height: f32,
}

/// Picture of a banner: a loading plate of the context picked by `seed`, else the old miniature
/// (Godot `BANNER_FIELD` / `BANNER_SIEGE`: Crécy for a battle, Sluys for a siege or a sea fight).
pub fn banner_image(plates: &ArtPlates, context: &str, seed: u64) -> (String, String) {
    match plates.loading_screen(context, seed as usize) {
        Some(plate) => (plate.id.clone(), plate.image.clone()),
        None => {
            let id = if context == "battle" {
                "evt_crecy"
            } else {
                "evt_sluys"
            };
            (id.to_string(), format!("events/{id}.jpg"))
        }
    }
}

/// Illustrated banner: a plate of `context` (`battle`, `siege`, `naval`) picked by `seed`,
/// cover-cropped (centred), veiled towards the bottom, gold rule, title and subtitle at the bottom
/// left. Brown without a plate.
pub fn banner(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    art: &DialogArt,
    context: &str,
    seed: u64,
    title: &str,
    subtitle: &str,
) {
    let (plate_id, plate_image) = banner_image(art.plates, context, seed);
    let height = if art.window_height >= 760.0 {
        BANNER_HEIGHT
    } else {
        BANNER_HEIGHT_SHORT
    };
    parent
        .spawn((
            Name::new("flow:banner"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(height),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                border: UiRect::bottom(Val::Px(3.0)),
                ..default()
            },
            BorderColor::all(style::GOLD),
            BackgroundColor(BANNER_FALLBACK),
        ))
        .with_children(|banner| {
            {
                // Wider than tall: the full-width picture overflows and is centred (cover).
                banner.spawn((
                    Name::new(format!("flow:banner_art:{plate_id}")),
                    Node {
                        width: Val::Percent(100.0),
                        aspect_ratio: Some(16.0 / 9.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    ImageNode::new(art.assets.load(plate_image)),
                ));
            }
            banner
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexEnd,
                        padding: UiRect::new(
                            Val::Px(28.0),
                            Val::Px(28.0),
                            Val::Px(0.0),
                            Val::Px(14.0),
                        ),
                        ..default()
                    },
                    BackgroundGradient::from(LinearGradient::to_bottom(vec![
                        ColorStop::auto(Color::srgba(0.0, 0.0, 0.0, 0.05)),
                        ColorStop::auto(Color::srgba(0.08, 0.04, 0.02, 0.8)),
                    ])),
                ))
                .with_children(|texts| {
                    texts.spawn((
                        Text::new(title),
                        theme.title_font(UiType::Title.size()),
                        TextColor(BANNER_TITLE),
                        TextShadow {
                            offset: Vec2::splat(2.0),
                            color: Color::srgba(0.1, 0.05, 0.02, 0.9),
                        },
                    ));
                    if !subtitle.is_empty() {
                        texts.spawn((
                            Text::new(subtitle),
                            theme.text_font(false, true, UiType::Body.size()),
                            TextColor(BANNER_SUBTITLE),
                            TextShadow {
                                offset: Vec2::splat(1.5),
                                color: Color::srgba(0.1, 0.05, 0.02, 0.9),
                            },
                        ));
                    }
                });
        });
}

/// Arms of `faction` (`heraldry/<faction>.png`, 40 × 46, aspect kept); nothing when missing.
pub fn arms(parent: &mut ChildSpawnerCommands, assets: &AssetServer, faction: &str) {
    if faction.is_empty() {
        return;
    }
    let path = format!("heraldry/{faction}.png");
    let exists = paths::find_root()
        .ok()
        .is_some_and(|root| paths::asset_root(&root).join(&path).exists());
    if !exists {
        return;
    }
    parent.spawn((
        Name::new(format!("flow:arms:{faction}")),
        Node {
            width: Val::Px(40.0),
            height: Val::Px(46.0),
            flex_shrink: 0.0,
            ..default()
        },
        ImageNode::new(assets.load(path)).with_mode(bevy::ui::widget::NodeImageMode::Auto),
    ));
}

/// A side header: arms then name (player, left) or name then arms (opponent, right).
pub fn side_header(
    column: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    faction: &str,
    name: &str,
    slot: usize,
    color: Color,
) {
    column
        .spawn(Node {
            column_gap: Val::Px(10.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            if slot == 0 {
                arms(header, assets, faction);
            }
            header
                .spawn(typed(theme, name, UiType::Heading))
                .insert(TextColor(color.darker(0.1)));
            if slot != 0 {
                arms(header, assets, faction);
            }
        });
}

#[allow(clippy::too_many_arguments)]
pub fn spawn(
    commands: &mut Commands,
    theme: &UiTheme,
    art: &DialogArt,
    game: &GameData,
    brief: &BattleBrief,
    siege: bool,
    seed: u64,
) {
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
                    banner(
                        panel,
                        theme,
                        art,
                        if siege { "siege" } else { "battle" },
                        seed,
                        &brief.title,
                        &brief.subtitle,
                    );
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
                                side_column(columns, theme, art.assets, side, slot, colors[slot]);
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
    assets: &AssetServer,
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
            side_header(
                column,
                theme,
                assets,
                &side.faction,
                &format!("{}{you}", side.name),
                slot,
                color,
            );
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

#[cfg(test)]
mod banner_tests {
    use super::*;

    #[test]
    fn banner_falls_back_to_event_miniature() {
        let plates = ArtPlates::default();
        assert_eq!(banner_image(&plates, "battle", 3).1, "events/evt_crecy.jpg");
        assert_eq!(banner_image(&plates, "siege", 3).1, "events/evt_sluys.jpg");
        assert_eq!(banner_image(&plates, "naval", 0).1, "events/evt_sluys.jpg");
    }
}
