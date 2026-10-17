//! Widget tree of the siege panel. Pure builder: every call spawns a fresh tree from a
//! `SiegeSheet`; the plugin rebuilds on change.

use bevy::prelude::*;
use game_api::siege::{BesiegerView, EngineView, SiegeRole, SiegeSheet};

use super::SiegePanelRoot;
use crate::app::AppState;
use crate::ui::kit::close_button;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::{UiTheme, fr, widgets::*};

const WIDTH: f32 = 460.0;

fn text(theme: &UiTheme, content: &str, variation: UiType, color: Color) -> impl Bundle {
    (
        Text::new(content),
        theme.text_font(false, false, variation.size()),
        TextColor(color),
    )
}

fn root(name: &'static str) -> impl Bundle {
    (
        Name::new(name),
        SiegePanelRoot,
        crate::ui::kit::layout::FitZone::Left,
        DespawnOnExit(AppState::Campaign),
        UiTier::Panel.index(),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
    )
}

/// « Engins de siège — échelles : prêtes, bélier : 2 tours, beffroi : 5 tours. »
pub fn engines_text(engines: &[EngineView]) -> String {
    let parts: Vec<String> = engines
        .iter()
        .map(|engine| {
            let name = engine.name.to_lowercase();
            if engine.ready {
                format!(
                    "{name} : {}",
                    if engine.kind == "ladders" {
                        "prêtes"
                    } else {
                        "prêt"
                    }
                )
            } else {
                format!(
                    "{name} : {}",
                    fr::count(i64::from(engine.turns_left), "tour", None)
                )
            }
        })
        .collect();
    format!("Engins de siège — {}.", parts.join(", "))
}

fn role_line(sheet: &SiegeSheet) -> String {
    match sheet.role {
        SiegeRole::Attacker => format!(
            "Vous assiégez {} (garnison de {}).",
            sheet.settlement_name, sheet.defender_name
        ),
        SiegeRole::Defender => format!(
            "{} est assiégée par {}.",
            sheet.settlement_name, sheet.attacker_name
        ),
        SiegeRole::Observer => format!(
            "{} ({}) est assiégée par {}.",
            sheet.settlement_name, sheet.defender_name, sheet.attacker_name
        ),
    }
}

fn gauge_row(
    p: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    name: &str,
    label: &str,
    percent: u8,
    tip: &str,
) {
    p.spawn((
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        },
        Tooltip::new(tip),
    ))
    .with_children(|row| {
        row.spawn((
            Node {
                width: Val::Px(90.0),
                ..default()
            },
            text(theme, label, UiType::Caption, style::INK_SOFT),
        ));
        row.spawn(gauge(theme, name, f32::from(percent) / 100.0, 200.0));
        row.spawn(text(
            theme,
            &format!("{percent} %"),
            UiType::Caption,
            style::INK,
        ));
    });
}

fn besieger_line(b: &BesiegerView) -> String {
    format!(
        "{} ({}) — {} hommes, {}{}",
        b.name,
        b.faction_name,
        fr::digits(i64::from(b.men)),
        fr::count(b.regiments as i64, "régiment", None),
        if b.sieging { "" } else { ", hors siège" }
    )
}

/// The siege panel (docked top left).
pub fn spawn_siege_panel(
    commands: &mut Commands,
    theme: &UiTheme,
    sheet: &SiegeSheet,
    error: &str,
) {
    let wrapper = commands.spawn(root("SiegePanelRoot")).id();
    let panel = commands
        .spawn((Name::new("siege:panel"), panel_sized(theme, WIDTH)))
        .insert(Node {
            width: Val::Px(WIDTH),
            padding: theme.padding("panel"),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .id();
    commands.entity(wrapper).add_child(panel);
    let attacker = sheet.role == SiegeRole::Attacker;
    commands.entity(panel).with_children(|p| {
        p.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|h| {
            h.spawn((Name::new("siege:title"), heading(theme, &format!("Siège de {}", sheet.settlement_name))));
            h.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            h.spawn(close_button(theme, "siege:close"));
        });
        p.spawn(text(theme, &role_line(sheet), UiType::Caption, style::INK));
        p.spawn(text(
            theme,
            &format!(
                "Tour {} du siège · reddition dans ~{} · {}",
                sheet.turns_elapsed.max(1),
                fr::count(i64::from(sheet.turns_left), "tour", None),
                if sheet.walls_stand { "murailles intactes" } else { "brèche ouverte" }
            ),
            UiType::Caption,
            style::INK_SOFT,
        ));
        gauge_row(p, theme, "siege:supplies", "Vivres", sheet.supplies,
            "[b]Vivres[/b]\nÀ 0 %, la garnison capitule de faim.");
        gauge_row(p, theme, "siege:breach", "Brèche", sheet.breach,
            "[b]Brèche[/b]\nDégâts aux murailles ; dès 50 %, l'assaut ne subit plus leur pénalité.");
        p.spawn(separator(theme));
        // Garrison and besiegers.
        p.spawn(text(
            theme,
            &format!(
                "Garnison : {} hommes en {}, murailles niveau {}",
                fr::digits(i64::from(sheet.garrison_men)),
                fr::count(sheet.garrison.len() as i64, "régiment", None),
                sheet.fortification_level
            ),
            UiType::Body,
            style::INK,
        ));
        p.spawn((Name::new("siege:garrison"), panel_inset(theme))).with_children(|list| {
            if sheet.garrison.is_empty() {
                list.spawn(text(theme, "Aucun défenseur.", UiType::Caption, style::INK_SOFT));
            }
            for unit in &sheet.garrison {
                list.spawn(text(
                    theme,
                    &format!(
                        "{} — {} / {} hommes, moral {}",
                        unit.name, unit.strength, unit.max_strength, unit.morale
                    ),
                    UiType::Caption,
                    style::INK,
                ));
            }
        });
        p.spawn(text(
            theme,
            &format!("Assiégeants : {} hommes", fr::digits(i64::from(sheet.besieger_men))),
            UiType::Body,
            style::INK,
        ));
        for besieger in &sheet.besiegers {
            p.spawn(text(theme, &besieger_line(besieger), UiType::Caption, style::INK_SOFT));
        }
        if !sheet.engines.is_empty() {
            p.spawn((
                Name::new("siege:engines"),
                text(theme, &engines_text(&sheet.engines), UiType::Caption, style::INK),
                Tooltip::new("[b]Engins de siège[/b]\nLes assiégeants les bâtissent sur place, l'un après l'autre ; derrière des murailles intactes, il faut au moins un engin prêt pour donner l'assaut."),
            ));
        }
        p.spawn(separator(theme));
        // Actions.
        p.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            if attacker {
                let blocked = !sheet.assault_blocker.is_empty();
                let caption = if blocked {
                    "Donner l'assaut (aucun engin prêt)".to_owned()
                } else {
                    format!("Donner l'assaut (≈ {} %)", sheet.assault_odds.unwrap_or(0))
                };
                let mut assault = row.spawn(button_primary(theme, "siege:assault", &caption));
                if blocked {
                    assault.insert((
                        bevy::ui::InteractionDisabled,
                        Tooltip::new(&format!("Assaut impossible : {}.", sheet.assault_blocker)),
                    ));
                }
                let summon_caption = if sheet.surrender_chance > 0 {
                    format!("Sommer la garnison (≈ {} %)", sheet.surrender_chance)
                } else {
                    "Sommer la garnison (vivres abondants)".to_owned()
                };
                let mut summon = row.spawn((
                    button_small(theme, "siege:summon", &summon_caption),
                    Tooltip::new("La garnison cède d'autant plus volontiers qu'elle a faim et que les murs sont ouverts ; elle refuse tant que ses vivres sont au-dessus de la moitié."),
                ));
                if sheet.surrender_chance == 0 {
                    summon.insert(bevy::ui::InteractionDisabled);
                }
                if !sheet.liftable_armies.is_empty() {
                    row.spawn((
                        button_small(theme, "siege:lift", "Lever le siège"),
                        Tooltip::new("Les armées quittent la posture de siège ; le siège est levé à la fin du tour."),
                    ));
                }
            }
            if sheet.role == SiegeRole::Defender {
                let mut sortie = row.spawn((
                    button_primary(theme, "siege:sortie", "Faire une sortie"),
                    Tooltip::new("La garnison sort au-devant des assiégeants (résolu aussitôt)."),
                ));
                if !sheet.can_sortie {
                    sortie.insert(bevy::ui::InteractionDisabled);
                }
            }
            row.spawn((
                button_small(theme, "siege:wait", "Attendre"),
                Tooltip::new("Aucun ordre : le siège se poursuit à la fin du tour."),
            ));
        });
        if !error.is_empty() {
            p.spawn((Name::new("siege:error"), text(theme, error, UiType::Caption, style::ERROR)));
        }
    });
}

/// Shown once when the siege ended while the panel was open.
pub fn spawn_ended_panel(commands: &mut Commands, theme: &UiTheme, place: &str) {
    let wrapper = commands.spawn(root("SiegePanelRoot")).id();
    let panel = commands
        .spawn((Name::new("siege:panel"), panel_sized(theme, WIDTH)))
        .insert(Node {
            width: Val::Px(WIDTH),
            padding: theme.padding("panel"),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .id();
    commands.entity(wrapper).add_child(panel);
    commands.entity(panel).with_children(|p| {
        p.spawn(heading(theme, "Aucun siège en cours"));
        p.spawn(text(
            theme,
            &format!(
                "Il n'y a pas (ou plus) de siège à {}.",
                place.replace("set_", "").replace('_', " ")
            ),
            UiType::Caption,
            style::INK_SOFT,
        ));
        p.spawn(close_button(theme, "siege:close"));
    });
}
