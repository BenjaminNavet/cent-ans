//! Retained rebuild of the court screens: despawn the root, spawn it again from the snapshot.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Button, ScrollArea};
use game_api::court::{
    CharacterSheet, CompanionView, FamilyNode, SkillNode, SkillState, TargetRef, branch_label,
};

use super::{COUNCIL_TABS, CourtUi, OrderMenu, SHEET_TABS, Screen, refresh_snapshot};
use crate::campaign::panels::tech::layout::effect_label;
use crate::paths;
use crate::sim::Sim;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{KitWidget, ListItem, Toggled};
use crate::ui::kit::{
    UiTheme, UiTier, UiType, button_primary, button_small, caption, fr, gauge, heading, separator,
    style, tab_bar, tab_pane,
};

#[derive(Component)]
pub struct CourtRoot;

type Kids<'a> = ChildSpawnerCommands<'a>;

const LEFT_WIDTH: f32 = 250.0;
const PORTRAIT: f32 = 200.0;
const SKILL_MAX: f32 = 10.0;

/// Asset path of the portrait of a character (`game/assets/portraits/<id>.png`).
pub fn portrait_path(id: &str) -> String {
    format!("portraits/{id}.png")
}

fn ink(theme: &UiTheme, content: &str, ui_type: UiType, color: Color) -> impl Bundle {
    (
        Text::new(content),
        theme.text_font(false, false, ui_type.size()),
        TextColor(color),
    )
}

fn flex_text(theme: &UiTheme, content: &str, ui_type: UiType, color: Color) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        },
        ink(theme, content, ui_type, color),
    )
}

fn column(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(gap),
        ..default()
    }
}

fn row(gap: f32) -> Node {
    Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(gap),
        ..default()
    }
}

/// Scrolling content pane of a tab.
fn pane<'a>(parent: &'a mut Kids, group: &str, name: &str, visible: bool) -> EntityCommands<'a> {
    let mut pane = parent.spawn(tab_pane(group, name, visible));
    pane.insert(Node {
        flex_grow: 1.0,
        min_height: Val::Px(0.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(6.0),
        overflow: Overflow::scroll_y(),
        padding: UiRect::right(Val::Px(10.0)),
        display: if visible {
            Display::Flex
        } else {
            Display::None
        },
        ..default()
    })
    .insert(ScrollArea);
    pane
}

fn tip(text: &str) -> impl Bundle {
    (Tooltip::new(text), Hovered::default())
}

fn years(sheet_birth: i32, death: Option<i32>) -> String {
    match death {
        Some(death) if death > 0 => format!("{sheet_birth}–{death}"),
        _ => format!("né en {sheet_birth}"),
    }
}

fn effects_tip(description: &str, effects: &[game_api::court::EffectView]) -> String {
    let mut text = description.to_owned();
    for effect in effects {
        text.push_str(&format!("\n• {}", effect_label(effect)));
    }
    text
}

#[allow(clippy::too_many_arguments)]
pub fn rebuild(
    mut commands: Commands,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    sim: Res<Sim>,
    mut ui: ResMut<CourtUi>,
    roots: Query<Entity, With<CourtRoot>>,
) {
    if !ui.dirty {
        return;
    }
    ui.dirty = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if ui.screen == Screen::None {
        return;
    }
    refresh_snapshot(&mut ui, &sim);
    let asset_root = paths::find_root().ok().map(|r| paths::asset_root(&r));
    let ui = &*ui;
    commands
        .spawn((
            Name::new("CourtPanel"),
            CourtRoot,
            KitWidget("panel"),
            UiTier::Panel.index(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(6.0),
                right: Val::Px(6.0),
                top: Val::Px(40.0),
                bottom: Val::Px(6.0),
                padding: theme.padding("panel"),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            theme.sliced("panel"),
        ))
        .with_children(|root| match ui.screen {
            Screen::Sheet => sheet_screen(root, &theme, &assets, asset_root.as_deref(), ui),
            _ => council_screen(root, &theme, ui),
        });
}

fn result_line(row_: &mut Kids, theme: &UiTheme, ui: &CourtUi) {
    if let Some(result) = &ui.result {
        let (text, color) = if result.ok {
            ("Ordre donné.".to_owned(), style::GOOD)
        } else {
            (result.message.clone(), style::ERROR)
        };
        row_.spawn((
            Name::new("court:result"),
            ink(theme, &text, UiType::Body, color),
        ));
    }
}

fn title_text(theme: &UiTheme, content: &str) -> impl Bundle {
    (
        Text::new(content),
        theme.title_font(UiType::Title.size()),
        TextColor(style::INK_TITLE),
    )
}

// ------------------------------------------------------------------------ character sheet

fn sheet_screen(
    root: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    ui: &CourtUi,
) {
    let Some(sheet) = &ui.snapshot.sheet else {
        root.spawn(ink(theme, "Personnage inconnu.", UiType::Body, style::INK));
        root.spawn(button_small(theme, "court:close", "Fermer"));
        return;
    };
    root.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    })
    .with_children(|header| {
        let title = if sheet.epithet.is_empty() {
            sheet.name.clone()
        } else {
            format!("{} {}", sheet.name, sheet.epithet)
        };
        header.spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            children![title_text(theme, &title)],
        ));
        result_line(header, theme, ui);
        if !ui.history.is_empty() {
            header.spawn(button_small(theme, "court:back", "Retour"));
        }
        header.spawn(button_small(theme, "court:council", "Conseil"));
        header.spawn(button_small(theme, "court:close", "×"));
    });
    root.spawn(separator(theme));
    root.spawn(Node {
        flex_grow: 1.0,
        min_height: Val::Px(0.0),
        column_gap: Val::Px(14.0),
        ..default()
    })
    .with_children(|body| {
        left_column(body, theme, assets, asset_root, sheet);
        body.spawn(Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            min_width: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|right| {
            let selected = SHEET_TABS
                .iter()
                .position(|(name, _)| *name == ui.sheet_tab)
                .unwrap_or(0);
            right.spawn(tab_bar(theme, "court_sheet_tab", &SHEET_TABS, selected));
            right.spawn(separator(theme));
            for (index, (name, _)) in SHEET_TABS.iter().enumerate() {
                let mut pane = pane(right, "court_sheet_tab", name, index == selected);
                pane.with_children(|pane| match index {
                    0 => sheet_tab(pane, theme, ui, sheet),
                    1 => skills_tab(pane, theme, ui, sheet),
                    2 => family_tab(pane, theme, ui),
                    _ => orders_tab(pane, theme, ui, sheet),
                });
            }
        });
    });
}

fn left_column(
    body: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    sheet: &CharacterSheet,
) {
    body.spawn(Node {
        width: Val::Px(LEFT_WIDTH),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        overflow: Overflow::scroll_y(),
        ..default()
    })
    .insert(ScrollArea)
    .with_children(|left| {
        let portrait = portrait_path(&sheet.id);
        let arms = format!("heraldry/{}.png", sheet.faction);
        let exists = |path: &str| asset_root.is_some_and(|r| r.join(path).exists());
        left.spawn((
            Name::new("court:portrait"),
            Node {
                width: Val::Px(PORTRAIT),
                height: Val::Px(PORTRAIT),
                align_self: AlignSelf::Center,
                border: UiRect::all(Val::Px(3.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(style::GOLD),
            BackgroundColor(style::PARCHMENT_DARK),
        ))
        .with_children(|frame| {
            if exists(&portrait) {
                frame.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode::new(assets.load(portrait)),
                ));
            } else if exists(&arms) {
                frame.spawn((
                    Node {
                        width: Val::Px(PORTRAIT * 0.6),
                        height: Val::Px(PORTRAIT * 0.6),
                        ..default()
                    },
                    ImageNode::new(assets.load(arms)),
                ));
            } else {
                let initials: String = sheet
                    .name
                    .split_whitespace()
                    .filter_map(|w| w.chars().next())
                    .take(2)
                    .collect();
                frame.spawn(ink(theme, &initials, UiType::Title, style::INK_SOFT));
            }
        });
        left.spawn(heading(theme, &sheet.title));
        left.spawn(ink(theme, &sheet.faction_name, UiType::Body, style::INK));
        if !sheet.house.is_empty() {
            left.spawn(caption(theme, &format!("Maison {}", sheet.house)));
        }
        left.spawn(caption(theme, &sheet.activity));
        let sex = if sheet.sex == "female" { "Femme" } else { "Homme" };
        let life = if sheet.alive {
            format!("{sex}, {} ans", sheet.age)
        } else {
            format!("{sex}, défunt(e) ({})", years(sheet.birth_year, sheet.death_year))
        };
        left.spawn(ink(theme, &life, UiType::Body, style::INK));
        left.spawn((
            ink(
                theme,
                &format!("Piété {}   Prestige {}", sheet.piety, sheet.prestige),
                UiType::Body,
                style::INK,
            ),
            tip("[b]Piété et prestige[/b]\nLa piété pèse sur l'Église et le clergé ; le prestige sur la loyauté des vassaux et les actes royaux."),
        ));
        if let Some(loyalty) = sheet.loyalty {
            left.spawn(caption(theme, &format!("Loyauté {loyalty} / 100")));
        }
        for (label, value, tooltip) in [
            ("Commandement", sheet.skills.command, "Mène les armées : moral, manœuvre."),
            ("Gouvernance", sheet.skills.governance, "Administre les provinces : impôts, ordre public."),
            ("Cour", sheet.skills.court, "Diplomatie et influence à la cour."),
        ] {
            left.spawn((column(1.0), tip(tooltip))).with_children(|stat| {
                stat.spawn(ink(
                    theme,
                    &format!("{label} {value}"),
                    UiType::Body,
                    style::INK,
                ));
                stat.spawn(gauge(
                    theme,
                    &format!("court:gauge:{label}"),
                    (f32::from(value) / SKILL_MAX).min(1.0),
                    LEFT_WIDTH - 20.0,
                ));
            });
        }
        left.spawn(caption(
            theme,
            &format!(
                "Niveau {} — {} / {} XP — {} point(s) de compétence",
                sheet.level, sheet.experience, sheet.xp_to_next, sheet.skill_points
            ),
        ));
    });
}

fn person_button(row_: &mut Kids, theme: &UiTheme, id: &str, text: &str) {
    row_.spawn(button_small(theme, &format!("court:char:{id}"), text));
}

fn sheet_tab(pane: &mut Kids, theme: &UiTheme, ui: &CourtUi, sheet: &CharacterSheet) {
    let _ = ui;
    if sheet.captive {
        pane.spawn(ink(
            theme,
            &format!("Captif de {}.", sheet.captor_name),
            UiType::Body,
            style::RUBRIC,
        ));
        if sheet.ransom > 0 {
            pane.spawn(ink(
                theme,
                &format!("Rançon : {}", fr::amount(sheet.ransom)),
                UiType::Body,
                style::INK,
            ));
        }
        match sheet.ransom_action.as_str() {
            "pay" => {
                pane.spawn(button_primary(theme, "court:ransom:pay", "Payer la rançon"));
            }
            "release" => {
                pane.spawn(button_primary(
                    theme,
                    "court:ransom:release",
                    "Libérer contre rançon",
                ));
            }
            _ => {}
        }
    }
    pane.spawn(heading(theme, "Traits"));
    if sheet.traits.is_empty() {
        pane.spawn(caption(theme, "Aucun trait remarquable."));
    }
    pane.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(6.0),
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|pills| {
        for t in &sheet.traits {
            let tint = match t.category.as_str() {
                "martial" => Color::srgb(0.62, 0.13, 0.08),
                "governance" => Color::srgb(0.24, 0.34, 0.18),
                "personality" => Color::srgb(0.42, 0.29, 0.16),
                "physical" => Color::srgb(0.55, 0.40, 0.12),
                "acquired" => Color::srgb(0.20, 0.25, 0.45),
                _ => style::INK_SOFT,
            };
            let mut text = t.description.clone();
            if t.turns_left > 0 {
                text.push_str(&format!(
                    "\nDisparaît dans {}.",
                    fr::count(i64::from(t.turns_left), "tour", Some("tours"))
                ));
            }
            pills.spawn((
                Name::new(format!("court:trait:{}", t.id)),
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(tint),
                tip(&text),
                children![ink(theme, &t.name, UiType::Caption, style::PRIMARY_INK)],
            ));
        }
    });
    pane.spawn(heading(theme, "Faits d'armes"));
    let f = &sheet.feats;
    let last = match f.last_battle_turns_ago {
        None => "aucune bataille".to_owned(),
        Some(turns) => format!(
            "dernière bataille il y a {} ({})",
            fr::count(i64::from(turns), "tour", Some("tours")),
            if f.last_battle_won {
                "victoire"
            } else {
                "défaite"
            }
        ),
    };
    pane.spawn(ink(
        theme,
        &format!(
            "{} batailles, {} victoires, {} sièges emportés, {} raids menés — {last}",
            f.battles_fought, f.battles_won, f.sieges_won, f.raids_led
        ),
        UiType::Body,
        style::INK,
    ));
    pane.spawn(heading(theme, "Famille"));
    pane.spawn(row(6.0)).with_children(|line| {
        let mut any = false;
        for (label, who) in [
            ("Père", &sheet.father),
            ("Mère", &sheet.mother),
            ("Époux(se)", &sheet.spouse),
        ] {
            if let Some(person) = who {
                any = true;
                line.spawn(caption(theme, label));
                person_button(line, theme, &person.id, &person.name);
            }
        }
        if !any {
            line.spawn(caption(theme, "Aucun proche connu."));
        }
    });
    if !sheet.children.is_empty() {
        pane.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|line| {
            line.spawn(caption(theme, "Enfants"));
            for child in &sheet.children {
                person_button(
                    line,
                    theme,
                    &child.id,
                    &format!("{} ({})", child.name, child.age),
                );
            }
        });
    }
    pane.spawn(heading(
        theme,
        &format!("Suite ({} / {})", sheet.retinue.len(), sheet.retinue_max),
    ));
    if sheet.retinue.is_empty() {
        pane.spawn(caption(theme, "Aucun compagnon."));
    }
    for companion in &sheet.retinue {
        companion_row(
            pane,
            theme,
            companion,
            sheet.can_act && !sheet.transfer_targets.is_empty(),
        );
    }
}

fn companion_row(pane: &mut Kids, theme: &UiTheme, companion: &CompanionView, transferable: bool) {
    pane.spawn((
        Name::new(format!("court:companion_row:{}", companion.id)),
        row(8.0),
        tip(&effects_tip(&companion.description, &companion.effects)),
    ))
    .with_children(|line| {
        line.spawn(flex_text(
            theme,
            &format!(
                "{} {} ({})",
                companion.glyph, companion.name, companion.category
            ),
            UiType::Body,
            style::INK,
        ));
        if transferable {
            line.spawn(button_small(
                theme,
                &format!("court:companion:{}", companion.id),
                "Confier",
            ));
        }
    });
}

// ------------------------------------------------------------------------ skill tree

fn skills_tab(pane: &mut Kids, theme: &UiTheme, ui: &CourtUi, sheet: &CharacterSheet) {
    pane.spawn(ink(
        theme,
        &format!(
            "{} point(s) de compétence disponible(s). Appris : {}.",
            sheet.skill_points,
            sheet.learned.len()
        ),
        UiType::Body,
        style::INK,
    ));
    pane.spawn(Node {
        column_gap: Val::Px(10.0),
        align_items: AlignItems::FlexStart,
        ..default()
    })
    .with_children(|columns| {
        for branch in ["command", "governance", "court"] {
            columns
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn(heading(theme, branch_label(branch)));
                    let nodes: Vec<&SkillNode> = ui
                        .snapshot
                        .skills
                        .iter()
                        .filter(|n| n.branch == branch)
                        .collect();
                    let mut tier = 0;
                    for node in nodes {
                        if node.tier != tier {
                            tier = node.tier;
                            col.spawn(caption(theme, &format!("Rang {tier}")));
                        }
                        skill_node(col, theme, node, sheet.can_act, &ui.snapshot.skills);
                    }
                });
        }
    });
}

fn skill_node(col: &mut Kids, theme: &UiTheme, node: &SkillNode, can_act: bool, all: &[SkillNode]) {
    let name = format!("court:learn:{}", node.id);
    let (mark, tint) = match node.state {
        SkillState::Learned => ("✓ ", style::GOOD),
        SkillState::Available => ("▲ ", style::INK),
        SkillState::NeedsPoints => ("", style::INK_FADED),
        SkillState::Locked => ("", style::INK_FADED),
    };
    let caption_text = format!("{mark}{} ({})", node.name, node.cost);
    let mut text = node.description.clone();
    for effect in &node.effects {
        text.push_str(&format!("\n• {}", effect_label(effect)));
    }
    if !node.prerequisites.is_empty() {
        let names: Vec<&str> = node
            .prerequisites
            .iter()
            .map(|p| {
                all.iter()
                    .find(|n| &n.id == p)
                    .map_or(p.as_str(), |n| n.name.as_str())
            })
            .collect();
        text.push_str(&format!("\nPrérequis : {}.", names.join(", ")));
    }
    let learnable = node.state == SkillState::Available && can_act;
    let mut entity = if learnable {
        col.spawn(button_primary(theme, &name, &caption_text))
    } else {
        let mut e = col.spawn(button_small(theme, &name, &caption_text));
        e.insert(InteractionDisabled);
        e
    };
    entity.insert(tip(&text));
    let _ = tint;
}

// ------------------------------------------------------------------------ family tree

fn family_tab(pane: &mut Kids, theme: &UiTheme, ui: &CourtUi) {
    let Some(tree) = &ui.snapshot.family else {
        pane.spawn(caption(theme, "Aucune généalogie connue."));
        return;
    };
    let mut generations: Vec<i32> = tree.nodes.iter().map(|n| n.generation).collect();
    generations.sort_unstable();
    generations.dedup();
    for generation in generations {
        let label = match generation {
            0 => "Génération du personnage".to_owned(),
            g if g < 0 => format!("{} génération(s) avant", -g),
            g => format!("{g} génération(s) après"),
        };
        pane.spawn(heading(theme, &label));
        pane.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|line| {
            for node in tree.nodes.iter().filter(|n| n.generation == generation) {
                family_card(line, theme, tree, node);
            }
        });
    }
}

fn family_card(
    line: &mut Kids,
    theme: &UiTheme,
    tree: &game_api::court::FamilyTree,
    node: &FamilyNode,
) {
    let is_root = node.id == tree.root;
    let mut marks = String::new();
    if node.id == tree.ruler {
        marks.push_str(" ♛");
    }
    if node.id == tree.heir {
        marks.push_str(" ◆");
    }
    if !node.blood {
        marks.push_str(" ⚭");
    }
    let name = if node.epithet.is_empty() {
        node.name.clone()
    } else {
        format!("{} {}", node.name, node.epithet)
    };
    let dates = years(node.birth_year, node.death_year);
    line.spawn((
        Name::new(format!("court:char:{}", node.id)),
        KitWidget("list_item"),
        ListItem {
            group: "court_family".to_owned(),
        },
        Toggled(is_root),
        Button,
        Hovered::default(),
        Tooltip::new(&format!(
            "{name}\n{}{}",
            if node.title.is_empty() {
                "Membre de la maison"
            } else {
                &node.title
            },
            if node.alive { "" } else { "\nDéfunt(e)" }
        )),
        Node {
            width: Val::Px(170.0),
            padding: UiRect::all(Val::Px(5.0)),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::NONE),
    ))
    .with_children(|card| {
        card.spawn(ink(
            theme,
            &format!("{name}{marks}"),
            UiType::Body,
            if node.alive {
                style::INK
            } else {
                style::INK_FADED
            },
        ));
        card.spawn(caption(theme, &dates));
    });
}

// ------------------------------------------------------------------------ orders

fn target_menu(pane: &mut Kids, theme: &UiTheme, prefix: &str, title: &str, targets: &[TargetRef]) {
    pane.spawn(heading(theme, title));
    if targets.is_empty() {
        pane.spawn(caption(theme, "Aucun choix possible."));
    }
    pane.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(6.0),
        row_gap: Val::Px(4.0),
        ..default()
    })
    .with_children(|list| {
        for target in targets {
            list.spawn(button_small(
                theme,
                &format!("court:{prefix}:{}", target.id),
                &target.name,
            ));
        }
    });
}

fn orders_tab(pane: &mut Kids, theme: &UiTheme, ui: &CourtUi, sheet: &CharacterSheet) {
    if !sheet.can_act {
        pane.spawn(caption(
            theme,
            if sheet.own {
                "Ce personnage ne peut recevoir d'ordre (défunt ou captif)."
            } else {
                "Seuls les membres de votre maison reçoivent vos ordres."
            },
        ));
        return;
    }
    pane.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(6.0),
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|bar| {
        let menus = [
            (OrderMenu::Marriage, "court:menu:marriage", "Marier"),
            (
                OrderMenu::Governor,
                "court:menu:governor",
                "Nommer gouverneur",
            ),
            (
                OrderMenu::General,
                "court:menu:general",
                "Confier une armée",
            ),
        ];
        for (menu, name, text) in menus {
            bar.spawn(crate::ui::kit::toggle_button(
                theme,
                name,
                text,
                ui.menu == Some(menu),
            ));
        }
        if sheet.heir_candidate {
            bar.spawn(button_small(theme, "court:heir", "Désigner l'héritier"));
        }
    });
    match ui.menu {
        Some(OrderMenu::Marriage) => {
            pane.spawn(heading(theme, "Époux possibles"));
            if sheet.marriage_candidates.is_empty() {
                pane.spawn(caption(theme, "Aucun parti convenable."));
            }
            for candidate in &sheet.marriage_candidates {
                pane.spawn(button_small(
                    theme,
                    &format!("court:marry:{}", candidate.id),
                    &format!("{} ({} ans)", candidate.name, candidate.age),
                ));
            }
        }
        Some(OrderMenu::Governor) => {
            target_menu(
                pane,
                theme,
                "gov",
                "Province à gouverner",
                &sheet.governor_options,
            );
        }
        Some(OrderMenu::General) => {
            target_menu(
                pane,
                theme,
                "gen",
                "Armée à commander",
                &sheet.general_options,
            );
        }
        Some(OrderMenu::Transfer) => {
            pane.spawn(heading(theme, "Confier le compagnon à"));
            if sheet.transfer_targets.is_empty() {
                pane.spawn(caption(
                    theme,
                    "Aucun général réuni avec cette armée dans la même place.",
                ));
            }
            for target in &sheet.transfer_targets {
                pane.spawn(button_small(
                    theme,
                    &format!("court:transfer:{}", target.id),
                    &target.name,
                ));
            }
        }
        None => {
            pane.spawn(caption(
                theme,
                "Choisissez un ordre. Les refus de la simulation s'affichent en haut de l'écran.",
            ));
        }
    }
}

// ------------------------------------------------------------------------ council

fn council_screen(root: &mut Kids, theme: &UiTheme, ui: &CourtUi) {
    root.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    })
    .with_children(|header| {
        header.spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            children![title_text(theme, "Conseil du royaume")],
        ));
        result_line(header, theme, ui);
        header.spawn(button_small(theme, "court:close", "×"));
    });
    let selected = COUNCIL_TABS
        .iter()
        .position(|(name, _)| *name == ui.council_tab)
        .unwrap_or(0);
    root.spawn(tab_bar(theme, "court_council_tab", &COUNCIL_TABS, selected));
    root.spawn(separator(theme));
    root.spawn(Node {
        flex_grow: 1.0,
        min_height: Val::Px(0.0),
        flex_direction: FlexDirection::Column,
        ..default()
    })
    .with_children(|body| {
        for (index, (name, _)) in COUNCIL_TABS.iter().enumerate() {
            let mut pane = pane(body, "court_council_tab", name, index == selected);
            pane.with_children(|pane| match index {
                0 => offices_tab(pane, theme, ui),
                1 => court_tab(pane, theme, ui),
                2 => acts_tab(pane, theme, ui),
                _ => chivalry_tab(pane, theme, ui),
            });
        }
    });
}

fn offices_tab(pane: &mut Kids, theme: &UiTheme, ui: &CourtUi) {
    let snap = &ui.snapshot;
    if snap.offices.is_empty() {
        pane.spawn(caption(theme, "Ce royaume n'a pas de conseil."));
        return;
    }
    pane.spawn(Node {
        column_gap: Val::Px(14.0),
        align_items: AlignItems::FlexStart,
        ..default()
    })
    .with_children(|body| {
        body.spawn(Node {
            width: Val::Px(340.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|list| {
            for office in &snap.offices {
                let holder = if office.holder_id.is_empty() {
                    "charge vacante".to_owned()
                } else {
                    format!("{} ({})", office.holder_name, office.holder_skill)
                };
                list.spawn((
                    Name::new(format!("court:office:{}", office.id)),
                    KitWidget("list_item"),
                    ListItem {
                        group: "court_offices".to_owned(),
                    },
                    Toggled(ui.office.as_deref() == Some(office.id.as_str())),
                    Button,
                    Hovered::default(),
                    Tooltip::new(&office.effect_text),
                    Node {
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|card| {
                    card.spawn(ink(theme, &office.name, UiType::Body, style::INK_TITLE));
                    card.spawn(caption(theme, &holder));
                });
            }
        });
        let Some(office) = snap
            .offices
            .iter()
            .find(|o| Some(&o.id) == ui.office.as_ref())
        else {
            return;
        };
        body.spawn(Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|sheet| {
            sheet.spawn(heading(theme, &office.name));
            sheet.spawn(ink(theme, &office.effect_text, UiType::Body, style::INK));
            if office.holder_id.is_empty() {
                sheet.spawn(caption(theme, "Charge vacante."));
            } else {
                sheet.spawn(row(8.0)).with_children(|line| {
                    line.spawn(ink(
                        theme,
                        &format!(
                            "Titulaire : {} (compétence {})",
                            office.holder_name, office.holder_skill
                        ),
                        UiType::Body,
                        style::INK,
                    ));
                    person_button(line, theme, &office.holder_id, "Fiche");
                    line.spawn(button_small(
                        theme,
                        &format!("court:dismiss:{}", office.id),
                        "Révoquer",
                    ));
                });
            }
            sheet.spawn(heading(theme, "Candidats"));
            if snap.candidates.is_empty() {
                sheet.spawn(caption(theme, "Personne ne peut occuper cette charge."));
            }
            for candidate in &snap.candidates {
                sheet.spawn(row(8.0)).with_children(|line| {
                    let busy = !candidate.current_office.is_empty();
                    line.spawn(flex_text(
                        theme,
                        &format!(
                            "{} (compétence {}){}",
                            candidate.name,
                            candidate.skill_value,
                            if busy {
                                " — siège déjà au conseil"
                            } else {
                                ""
                            }
                        ),
                        UiType::Body,
                        if busy { style::INK_FADED } else { style::INK },
                    ));
                    let name = format!("court:appoint:{}:{}", office.id, candidate.id);
                    if busy {
                        line.spawn(button_small(theme, &name, "Nommer"))
                            .insert(InteractionDisabled);
                    } else {
                        line.spawn(button_primary(theme, &name, "Nommer"));
                    }
                });
            }
        });
    });
}

fn court_tab(pane: &mut Kids, theme: &UiTheme, ui: &CourtUi) {
    pane.spawn(caption(
        theme,
        &format!("{} membres de la cour.", ui.snapshot.court.len()),
    ));
    for member in &ui.snapshot.court {
        pane.spawn((
            Name::new(format!("court:char:{}", member.id)),
            KitWidget("list_item"),
            ListItem {
                group: "court_members".to_owned(),
            },
            Toggled(false),
            Button,
            Hovered::default(),
            Node {
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|line| {
            let mut title = member.title.clone();
            if member.ruler {
                title = "Souverain(e)".to_owned();
            } else if member.heir {
                title = "Héritier(e)".to_owned();
            }
            if !member.office.is_empty() {
                title.push_str(&format!(" — {}", member.office));
            }
            line.spawn(flex_text(
                theme,
                &format!("{} ({} ans)", member.name, member.age),
                UiType::Body,
                style::INK,
            ));
            line.spawn(flex_text(theme, &title, UiType::Caption, style::INK_SOFT));
            line.spawn(flex_text(
                theme,
                &member.activity,
                UiType::Caption,
                style::INK_SOFT,
            ));
            line.spawn(ink(
                theme,
                &format!(
                    "C {}  G {}  Cr {}",
                    member.skills.command, member.skills.governance, member.skills.court
                ),
                UiType::Caption,
                style::INK,
            ));
            if member.skill_points > 0 {
                line.spawn(ink(
                    theme,
                    &format!("▲ {}", member.skill_points),
                    UiType::Caption,
                    style::GOOD,
                ));
            }
        });
    }
}

fn acts_tab(pane: &mut Kids, theme: &UiTheme, ui: &CourtUi) {
    let snap = &ui.snapshot;
    pane.spawn(caption(
        theme,
        &format!("Trésorerie : {}", fr::amount(snap.treasury)),
    ));
    if let Some(captain) = &snap.captain {
        pane.spawn(heading(theme, "Chevaliers bannerets"));
        pane.spawn(row(10.0)).with_children(|line| {
            line.spawn(flex_text(
                theme,
                &format!(
                    "{} / {} capitaines — engager : {}{}",
                    captain.count,
                    captain.cap,
                    fr::amount(captain.cost),
                    if captain.cooldown_left > 0 {
                        format!(" (dans {} tours)", captain.cooldown_left)
                    } else {
                        String::new()
                    }
                ),
                UiType::Body,
                style::INK,
            ));
            let name = format!("court:captain:{}", captain.settlement);
            if captain.can_hire {
                line.spawn(button_primary(theme, &name, "Engager un capitaine"));
            } else {
                line.spawn(button_small(theme, &name, "Engager un capitaine"))
                    .insert(InteractionDisabled);
            }
        });
        if !captain.can_hire && !captain.reason.is_empty() {
            pane.spawn(caption(theme, &captain.reason));
        }
    }
    pane.spawn(heading(theme, "Actes royaux"));
    for act in &snap.acts {
        pane.spawn((
            Name::new(format!("court:act_card:{}", act.id)),
            Node {
                padding: UiRect::all(Val::Px(6.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(style::INK_SOFT),
            BackgroundColor(style::PARCHMENT_LIGHT),
        ))
        .with_children(|card| {
            card.spawn(row(10.0)).with_children(|line| {
                line.spawn(flex_text(theme, &act.name, UiType::Body, style::INK_TITLE));
                let name = format!("court:act:{}", act.id);
                if act.available {
                    line.spawn(button_primary(theme, &name, "Accomplir"));
                } else {
                    line.spawn(button_small(theme, &name, "Accomplir"))
                        .insert(InteractionDisabled);
                }
            });
            card.spawn(ink(
                theme,
                &act.description,
                UiType::Caption,
                style::INK_SOFT,
            ));
            let mut cost = Vec::new();
            if act.cost_livres > 0 {
                cost.push(fr::amount(act.cost_livres));
            }
            if act.cost_prestige > 0 {
                cost.push(format!("{} prestige", act.cost_prestige));
            }
            let mut gain = Vec::new();
            if act.gain_prestige != 0 {
                gain.push(format!(
                    "{} prestige",
                    fr::signed(i64::from(act.gain_prestige))
                ));
            }
            if act.gain_piety != 0 {
                gain.push(format!("{} piété", fr::signed(i64::from(act.gain_piety))));
            }
            for effect in &act.effects {
                gain.push(effect_label(effect));
            }
            let mut line = format!(
                "Coût : {}",
                if cost.is_empty() {
                    "aucun".into()
                } else {
                    cost.join(", ")
                }
            );
            if !gain.is_empty() {
                line.push_str(&format!(" — Effets : {}", gain.join(", ")));
            }
            if act.duration_turns > 0 {
                line.push_str(&format!(" ({} tours)", act.duration_turns));
            }
            card.spawn(ink(theme, &line, UiType::Caption, style::INK));
            if act.active_left > 0 {
                card.spawn(ink(
                    theme,
                    &format!("En vigueur : encore {} tours.", act.active_left),
                    UiType::Caption,
                    style::GOOD,
                ));
            } else if act.cooldown_left > 0 {
                card.spawn(ink(
                    theme,
                    &format!("Possible de nouveau dans {} tours.", act.cooldown_left),
                    UiType::Caption,
                    style::FAIR,
                ));
            } else if !act.available && !act.reason.is_empty() {
                card.spawn(ink(theme, &act.reason, UiType::Caption, style::ERROR));
            }
        });
    }
}

fn chivalry_tab(pane: &mut Kids, theme: &UiTheme, ui: &CourtUi) {
    let Some(chivalry) = &ui.snapshot.chivalry else {
        return;
    };
    if let Some(order) = &chivalry.founded {
        pane.spawn(heading(theme, &order.name));
        pane.spawn(ink(
            theme,
            &format!(
                "Fondé au tour {}{}. {} chevalier(s).",
                order.founded_turn,
                if order.collapsed { " — dissous" } else { "" },
                order.members.len()
            ),
            UiType::Body,
            style::INK,
        ));
        pane.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|list| {
            for member in &order.members {
                person_button(list, theme, &member.id, &member.name);
            }
        });
        return;
    }
    pane.spawn(heading(theme, "Fonder un ordre de chevalerie"));
    if chivalry.options.is_empty() {
        pane.spawn(caption(theme, "Aucun ordre ne s'offre à votre maison."));
    }
    for option in &chivalry.options {
        pane.spawn((
            Name::new(format!("court:order_card:{}", option.id)),
            Node {
                padding: UiRect::all(Val::Px(6.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(style::INK_SOFT),
            BackgroundColor(style::PARCHMENT_LIGHT),
        ))
        .with_children(|card| {
            card.spawn(row(10.0)).with_children(|line| {
                line.spawn(flex_text(
                    theme,
                    &option.name,
                    UiType::Body,
                    style::INK_TITLE,
                ));
                let name = format!("court:found:{}", option.id);
                if option.available {
                    line.spawn(button_primary(theme, &name, "Fonder"));
                } else {
                    line.spawn(button_small(theme, &name, "Fonder"))
                        .insert(InteractionDisabled);
                }
            });
            card.spawn(ink(
                theme,
                &option.description,
                UiType::Caption,
                style::INK_SOFT,
            ));
            card.spawn(ink(
                theme,
                &format!(
                    "Coût : {} — prestige requis : {} — {} chevaliers. {}",
                    fr::amount(option.cost),
                    option.prestige_required,
                    option.members,
                    option.historical_members
                ),
                UiType::Caption,
                style::INK,
            ));
            if !option.available && !option.reason.is_empty() {
                card.spawn(ink(theme, &option.reason, UiType::Caption, style::ERROR));
            }
        });
    }
}
