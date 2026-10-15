//! Retained rebuild of the feudal panel: despawn the root, spawn it again from the snapshot.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Button;
use game_api::feudal::{FeudalCall, FeudalSheet, TreeNodeView};

use super::format::{self, Line};
use super::{FeudalUi, pick, refresh_snapshot};
use crate::paths;
use crate::sim::Sim;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::KitWidget;
use crate::ui::kit::{
    UiTheme, UiTier, UiType, button_primary, button_small, heading, separator, style,
};

#[derive(Component)]
pub struct FeudalRoot;

type Kids<'a> = ChildSpawnerCommands<'a>;

const PANEL_WIDTH: f32 = 480.0;
/// Provinces named per title before « … ».
const PROVINCE_CAP: usize = 8;

pub fn rebuild(
    mut commands: Commands,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    sim: Res<Sim>,
    mut ui: ResMut<FeudalUi>,
    roots: Query<Entity, With<FeudalRoot>>,
) {
    if !ui.dirty {
        return;
    }
    ui.dirty = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !ui.open {
        return;
    }
    refresh_snapshot(&mut ui, &sim);
    let asset_root = paths::find_root().ok().map(|r| paths::asset_root(&r));
    let ui = &*ui;
    commands
        .spawn((
            Name::new("FeudalTreePanel"),
            FeudalRoot,
            KitWidget("panel"),
            UiTier::Panel.index(),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(6.0),
                top: Val::Px(40.0),
                bottom: Val::Px(6.0),
                width: Val::Px(PANEL_WIDTH),
                padding: theme.inner_padding("panel", 8.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            theme.sliced("panel"),
        ))
        .with_children(|root| {
            header(root, &theme, ui);
            root.spawn(separator(&theme));
            root.spawn((
                Name::new("feudal:scroll"),
                KitWidget("scroll_list"),
                bevy::ui_widgets::ScrollArea,
                Node {
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::right(Val::Px(12.0)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .with_children(|body| {
                if ui.snapshot.panel.is_none() {
                    body.spawn(ink(
                        &theme,
                        "Féodalité indisponible.",
                        UiType::Body,
                        style::INK_FADED,
                    ));
                    return;
                }
                arms_block(body, &theme, &assets, asset_root.as_deref(), ui);
                position(body, &theme, ui);
                calls(body, &theme, ui);
                possessions(body, &theme, ui);
                tree(body, &theme, &assets, asset_root.as_deref(), ui);
                selection(body, &theme, ui);
                objectives(body, &theme, ui);
            });
        });
}

fn ink(theme: &UiTheme, content: &str, ui_type: UiType, color: Color) -> impl Bundle {
    (
        Text::new(content),
        theme.text_font(false, false, ui_type.size()),
        TextColor(color),
    )
}

fn column(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(gap),
        flex_shrink: 0.0,
        ..default()
    }
}

fn row() -> Node {
    Node {
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        row_gap: Val::Px(4.0),
        ..default()
    }
}

fn header(root: &mut Kids, theme: &UiTheme, ui: &FeudalUi) {
    root.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    })
    .with_children(|head| {
        head.spawn(Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|titles| {
            titles.spawn((
                Text::new("Arbre féodal"),
                theme.title_font(UiType::Title.size()),
                TextColor(style::INK_TITLE),
            ));
            if let Some(panel) = &ui.snapshot.panel {
                titles.spawn((
                    Name::new("feudal:realm"),
                    ink(
                        theme,
                        &format!("{} — vu depuis {}", panel.tree.name, panel.focus.name),
                        UiType::Caption,
                        style::INK_SOFT,
                    ),
                ));
            }
        });
        head.spawn(button_small(theme, "feudal:close", "×"));
    });
    if let Some(result) = &ui.result {
        root.spawn((
            Name::new("feudal:result"),
            ink(
                theme,
                &result.text,
                UiType::Body,
                if result.ok { style::GOOD } else { style::ERROR },
            ),
        ));
    }
}

/// Shield of the ruler's house (falls back to the faction arms, then a neutral box).
fn arms_block(
    body: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    ui: &FeudalUi,
) {
    let Some(panel) = &ui.snapshot.panel else {
        return;
    };
    let sheet = &panel.focus;
    let arms = &ui.snapshot.arms;
    let house_path = arms
        .asset_path(&sheet.house)
        .filter(|p| asset_root.is_some_and(|r| r.join(p).exists()));
    let faction_path = format!("heraldry/{}.png", sheet.id);
    let path = house_path.or_else(|| {
        asset_root
            .is_some_and(|r| r.join(&faction_path).exists())
            .then_some(faction_path)
    });
    body.spawn((
        Name::new("feudal:arms"),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|line| {
        let mut shield = line.spawn((
            Name::new("feudal:shield"),
            Node {
                width: Val::Px(64.0),
                height: Val::Px(64.0),
                flex_shrink: 0.0,
                ..default()
            },
        ));
        if let Some(path) = path {
            shield.insert(ImageNode::new(assets.load(path)));
        }
        let tip = arms.tooltip(&sheet.house);
        if !tip.is_empty() {
            shield.insert((Button, Hovered::default(), Tooltip::new(&tip)));
        }
        line.spawn(Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|names| {
            names.spawn(ink(theme, &sheet.name, UiType::Heading, style::RUBRIC));
            let mut caption = sheet.primary_name.clone();
            if !sheet.ruler.is_empty() {
                caption = format!("{caption} — {}", sheet.ruler);
            }
            names.spawn(ink(theme, &caption, UiType::Caption, style::INK_SOFT));
            if !sheet.house.is_empty() {
                names.spawn(ink(
                    theme,
                    &format!("Maison de {}", sheet.house),
                    UiType::Caption,
                    style::INK_FADED,
                ));
            }
        });
    });
}

fn lines(body: &mut Kids, theme: &UiTheme, lines: &[Line]) {
    for (text, tone, name) in lines {
        body.spawn((
            Name::new(format!("feudal:line:{name}")),
            ink(theme, text, UiType::Caption, tone.color()),
        ));
    }
}

fn disabled_button(
    parent: &mut Kids,
    theme: &UiTheme,
    name: &str,
    caption: &str,
    tip: &str,
    disabled: bool,
) {
    let mut entity = parent.spawn((button_small(theme, name, caption), Tooltip::new(tip)));
    if disabled {
        entity.insert(InteractionDisabled);
    }
}

/// « Votre position »: obligations of the player, revolt and homage.
fn position(body: &mut Kids, theme: &UiTheme, ui: &FeudalUi) {
    let (Some(sheet), Some(duties)) = (&ui.snapshot.player, &ui.snapshot.duties) else {
        return;
    };
    body.spawn(column(3.0)).with_children(|section| {
        section.spawn(heading(theme, "Votre position"));
        lines(section, theme, &format::position_lines(sheet, duties));
        section.spawn(row()).with_children(|actions| {
            if let Some(liege) = &sheet.liege {
                disabled_button(
                    actions,
                    theme,
                    "feudal:revolt",
                    "Se révolter",
                    &format!(
                        "Rompre avec {} : guerre d’indépendance, et un cas de félonie contre vous.",
                        liege.name
                    ),
                    sheet.status == "in_revolt",
                );
            }
            if let Some(lord) = pick(&duties.homage_candidates, ui.homage_index) {
                if duties.homage_candidates.len() > 1 {
                    actions.spawn(button_small(
                        theme,
                        "feudal:homage:next",
                        &format!("Seigneur : {} ▸", lord.name),
                    ));
                } else {
                    actions.spawn(ink(
                        theme,
                        &format!("Seigneur : {}", lord.name),
                        UiType::Caption,
                        style::INK,
                    ));
                }
                disabled_button(
                    actions,
                    theme,
                    "feudal:homage",
                    "Prêter hommage",
                    "Se placer sous la protection du seigneur choisi (quitter son suzerain est une félonie).",
                    false,
                );
            }
        });
    });
}

/// « Appels féodaux »: protection, private-war arbitration, peace summons.
fn calls(body: &mut Kids, theme: &UiTheme, ui: &FeudalUi) {
    let Some(panel) = &ui.snapshot.panel else {
        return;
    };
    if panel.calls.is_empty() {
        return;
    }
    body.spawn(column(3.0)).with_children(|section| {
        section.spawn(heading(theme, "Appels féodaux"));
        for call in &panel.calls {
            section.spawn((
                Name::new(format!("feudal:call:{}", call.id)),
                ink(
                    theme,
                    &format!(
                        "{} ({})",
                        call.text,
                        crate::ui::kit::fr::count(
                            i64::from(call.expires_in),
                            "tour",
                            Some("tours")
                        )
                    ),
                    UiType::Caption,
                    style::INK,
                ),
            ));
            section
                .spawn(row())
                .with_children(|actions| call_buttons(actions, theme, call));
        }
    });
}

fn call_buttons(actions: &mut Kids, theme: &UiTheme, call: &FeudalCall) {
    let id = call.id;
    match call.kind.as_str() {
        "protection" | "peace_summons" => {
            let (yes, no) = format::call_captions(&call.kind);
            actions.spawn(button_primary(theme, &format!("feudal:call:{id}:yes"), yes));
            actions.spawn(button_small(theme, &format!("feudal:call:{id}:no"), no));
        }
        _ => {
            actions.spawn(button_primary(
                theme,
                &format!("feudal:call:{id}:peace"),
                "Imposer la paix",
            ));
            for side in [&call.attacker, &call.target].into_iter().flatten() {
                actions.spawn(button_small(
                    theme,
                    &format!("feudal:call:{id}:side:{}", side.id),
                    &format!("Soutenir {}", side.name),
                ));
            }
            actions.spawn(button_small(
                theme,
                &format!("feudal:call:{id}:letbe"),
                "Laisser faire",
            ));
        }
    }
}

/// Titles of the focused faction with the provinces of their own domain.
fn possessions(body: &mut Kids, theme: &UiTheme, ui: &FeudalUi) {
    let Some(panel) = &ui.snapshot.panel else {
        return;
    };
    let sheet: &FeudalSheet = &panel.focus;
    body.spawn(column(3.0)).with_children(|section| {
        section.spawn(heading(theme, "Titres et possessions"));
        if sheet.titles.is_empty() {
            section.spawn(ink(
                theme,
                "Cette faction ne tient aucun titre.",
                UiType::Caption,
                style::INK_FADED,
            ));
        }
        for title in &sheet.titles {
            let shown: Vec<&str> = title
                .provinces
                .iter()
                .take(PROVINCE_CAP)
                .map(|p| p.name.as_str())
                .collect();
            let more = title.provinces.len().saturating_sub(PROVINCE_CAP);
            let mut text = format!(
                "{} — {}",
                title.name,
                format::rank_heading(&title.rank_label)
            );
            if !shown.is_empty() {
                text.push_str(&format!(" : {}", shown.join(", ")));
                if more > 0 {
                    text.push_str(&format!(" … (+{more})"));
                }
            }
            section.spawn((
                Name::new(format!("feudal:title:{}", title.id)),
                ink(
                    theme,
                    &text,
                    UiType::Caption,
                    if title.primary {
                        style::RUBRIC
                    } else {
                        style::INK
                    },
                ),
            ));
        }
    });
}

/// Feudal tree: one row per faction, indented, with a fold toggle and a status pill.
fn tree(
    body: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    ui: &FeudalUi,
) {
    let Some(panel) = &ui.snapshot.panel else {
        return;
    };
    body.spawn(column(2.0)).with_children(|section| {
        section.spawn(heading(theme, "Suzerains et vassaux"));
        tree_rows(
            section,
            theme,
            assets,
            asset_root,
            ui,
            &panel.tree,
            0,
            &panel.focus.id,
        );
    });
}

#[allow(clippy::too_many_arguments)]
fn tree_rows(
    list: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    ui: &FeudalUi,
    node: &TreeNodeView,
    depth: usize,
    focus: &str,
) {
    let expanded = ui.expanded.contains(&node.id);
    let selected = ui.selected.as_deref() == Some(node.id.as_str());
    let titles = node.titles.join(", ");
    let mut tip = titles.clone();
    if let Some(loyalty) = node.loyalty {
        tip.push_str(&format!(
            "\nLoyauté envers son suzerain : {loyalty} ({})",
            node.status_label
        ));
    }
    list.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(4.0),
        padding: UiRect::left(Val::Px(14.0 * depth as f32)),
        min_height: Val::Px(26.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|line| {
        if node.vassals.is_empty() {
            line.spawn(Node {
                width: Val::Px(26.0),
                flex_shrink: 0.0,
                ..default()
            });
        } else {
            line.spawn(button_small(
                theme,
                &format!("feudal:fold:{}", node.id),
                if expanded { "▾" } else { "▸" },
            ));
        }
        let path = format!("heraldry/{}.png", node.id);
        if asset_root.is_some_and(|r| r.join(&path).exists()) {
            line.spawn((
                Node {
                    width: Val::Px(18.0),
                    height: Val::Px(18.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                ImageNode::new(assets.load(path)),
            ));
        }
        let color = if node.player || node.id == focus {
            style::RUBRIC
        } else if selected {
            style::GOLD
        } else {
            style::INK
        };
        let label = format!("{}{}", node.name, if node.player { " (vous)" } else { "" });
        line.spawn((
            Name::new(format!("feudal:node:{}", node.id)),
            Button,
            Hovered::default(),
            Tooltip::new(&tip),
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                ..default()
            },
            ink(theme, &label, UiType::Body, color),
        ));
        if !node.status.is_empty() {
            line.spawn((
                Hovered::default(),
                Button,
                Tooltip::new(&format!("Loyauté {}", node.loyalty.unwrap_or(0))),
                ink(
                    theme,
                    &format!(
                        "{} {}",
                        format::status_glyph(&node.status),
                        node.status_label
                    ),
                    UiType::Caption,
                    format::status_color(&node.status),
                ),
            ));
        }
    });
    if expanded {
        for vassal in &node.vassals {
            tree_rows(
                list,
                theme,
                assets,
                asset_root,
                ui,
                vassal,
                depth + 1,
                focus,
            );
        }
    }
}

/// Sheet and actions on the selected faction: commise, grant a title.
fn selection(body: &mut Kids, theme: &UiTheme, ui: &FeudalUi) {
    let (Some(sheet), Some(duties)) = (&ui.snapshot.selected, &ui.snapshot.duties) else {
        return;
    };
    let player = ui.snapshot.player.as_ref().map(|p| p.id.as_str());
    body.spawn(column(3.0)).with_children(|section| {
        section.spawn((
            Name::new("feudal:selected"),
            ink(theme, &sheet.name, UiType::Body, style::RUBRIC),
        ));
        let line = match &sheet.liege {
            Some(liege) => format!(
                "{} — vassal de {}, loyauté {} ({})",
                sheet.primary_name,
                liege.name,
                sheet.loyalty.unwrap_or(0),
                sheet.status_label
            ),
            None => format!("{} — souverain", sheet.primary_name),
        };
        section.spawn(ink(theme, &line, UiType::Caption, style::INK_SOFT));
        // Only the lord of the selection may act on it.
        if sheet.liege.as_ref().map(|l| l.id.as_str()) != player {
            return;
        }
        section.spawn(row()).with_children(|actions| {
            let felon = duties.felons.iter().any(|c| c.vassal.id == sheet.id);
            disabled_button(
                actions,
                theme,
                "feudal:commise",
                "Prononcer la commise",
                if felon {
                    "Guerre contre ce seul vassal félon : ses fiefs vous reviennent si vous gagnez."
                } else {
                    "Aucun cas de félonie ouvert contre ce vassal."
                },
                !felon,
            );
            if let Some(title) = pick(&duties.grantable_titles, ui.grant_index) {
                if duties.grantable_titles.len() > 1 {
                    actions.spawn(button_small(
                        theme,
                        "feudal:grant:next",
                        &format!("Titre : {} ▸", title.name),
                    ));
                } else {
                    actions.spawn(ink(
                        theme,
                        &format!("Titre : {}", title.name),
                        UiType::Caption,
                        style::INK,
                    ));
                }
                disabled_button(
                    actions,
                    theme,
                    "feudal:grant",
                    "Concéder",
                    "Donner ce titre et ses terres au vassal (+ loyauté).",
                    false,
                );
            }
        });
    });
}

fn objectives(body: &mut Kids, theme: &UiTheme, ui: &FeudalUi) {
    let Some(sheet) = &ui.snapshot.player else {
        return;
    };
    body.spawn(column(3.0)).with_children(|section| {
        section.spawn(heading(theme, "Objectifs"));
        lines(section, theme, &format::objective_lines(sheet));
    });
}
