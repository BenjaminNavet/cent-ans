//! Retained rebuild of the faction panel: despawn the root, spawn it again from the view.

use bevy::prelude::*;
use game_api::faction_panel::{FactionPanelView, TableProvince, TradeRow};
use game_api::hud::BudgetLineView;

use super::chart::{self, season_label, short};
use super::{FactionPanel, PANEL_ID, TAB_GROUP, TABS, refresh, sections};
use crate::campaign::hud::text;
use crate::codex::store::CodexStore;
use crate::sim::Sim;
use crate::ui::kit::close_button;
use crate::ui::kit::icons::icon;
use crate::ui::kit::panel_stack::{PanelTier, StackedPanel};
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::KitWidget;
use crate::ui::kit::{
    UiTheme, UiTier, UiType, button_small, caption, fr, heading, scroll_list, scroll_list_fill,
    separator, style, tab_bar, tab_pane_fit, toggle_button,
};

#[derive(Component)]
pub struct FactionRoot;

/// A bar of the treasury chart.
#[derive(Component)]
pub struct ChartBar;

/// A segment of the treasury curve.
#[derive(Component)]
pub struct ChartSegment;

/// A route row of the trade tab.
#[derive(Component)]
pub struct TradeRowMarker;

pub(super) type Kids<'a> = ChildSpawnerCommands<'a>;

const PANEL_WIDTH: f32 = 580.0;
const CHART_HEIGHT: f32 = 100.0;
const TAB_HEIGHT: f32 = 440.0;
/// Smallest tab height when the zone is short.
const MIN_TAB_HEIGHT: f32 = 120.0;
const AXIS_WIDTH: f32 = 58.0;
const NUMBER_WIDTH: f32 = 92.0;
const TABLE_ROWS: usize = 12;

const RUBRICS: [(&str, &str, &str); 6] = [
    (
        "receipts",
        "Impôts et commerce",
        "Impôts des colonies, commerce et seigneuriage, selon le taux d’imposition.",
    ),
    (
        "armies",
        "Entretien des armées",
        "Solde et vivres des armées et des garnisons (plus chers si la monnaie est affaiblie).",
    ),
    (
        "buildings",
        "Entretien des bâtiments",
        "Entretien des bâtiments des colonies ; réduit par la dévastation.",
    ),
    (
        "table",
        "Table des provinces",
        "Régimes alimentaires des provinces (La Table).",
    ),
    (
        "administration",
        "Cour et administration",
        "Part du revenu prise par la cour et l’administration, refonte des monnaies et prix des édits en vigueur compris.",
    ),
    (
        "other",
        "Autres mouvements",
        "Ce que le budget n’explique pas : rançons, tributs, agents, choix de la chronique, butin. Connu seulement après coup.",
    ),
];

pub(crate) fn income_label(key: &str) -> &str {
    match key {
        "poll_peasants" => "Taille des paysans",
        "poll_burghers" => "Taille des bourgeois",
        "poll_clergy" => "Taille du clergé",
        "poll_nobility" => "Taille de la noblesse",
        "production" => "Production (moulins, ateliers)",
        "tax_rate" => "Taux d'imposition",
        "building_tax" => "Bâtiments (recettes)",
        "building_trade" => "Bâtiments (commerce)",
        "devastation" => "Dévastation",
        "full_province" => "Province entière",
        "embargo" => "Embargos",
        "domain" => "Domaine du seigneur",
        "difficulty" => "Difficulté",
        "alms" => "Aumônes de la croisade",
        "seigniorage" => "Seigneuriage",
        "trade_routes" => "Routes commerciales",
        other => other,
    }
}

fn mode_label(mode: &str) -> &str {
    match mode {
        "land" => "terre",
        "sea" => "mer",
        "mixed" => "terre et mer",
        other => other,
    }
}

fn status_text(key: &str) -> &str {
    match key {
        "war" => "guerre",
        "truce" => "trêve",
        "peace" => "paix",
        "alliance" => "alliance",
        other => other,
    }
}

pub fn rebuild(
    mut commands: Commands,
    theme: Res<UiTheme>,
    sim: Res<Sim>,
    assets: Res<AssetServer>,
    codex: Res<CodexStore>,
    mut panel: ResMut<FactionPanel>,
    roots: Query<Entity, With<FactionRoot>>,
) {
    if !panel.dirty {
        return;
    }
    panel.dirty = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !panel.open {
        return;
    }
    refresh(&mut panel, &sim);
    let panel = &*panel;
    commands
        .spawn((
            Name::new("FactionPanel"),
            FactionRoot,
            StackedPanel::new(PanelTier::Central, PANEL_ID),
            crate::ui::kit::layout::FitZone::Side,
            KitWidget("panel"),
            UiTier::Panel.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(PANEL_WIDTH),
                padding: theme.inner_padding("panel", 8.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            theme.sliced("panel"),
        ))
        .with_children(|root| {
            let Some(view) = &panel.view else {
                root.spawn(heading(&theme, "Faction inconnue"));
                root.spawn(close_button(&theme, "faction:close"));
                return;
            };
            header(root, &theme, panel, view);
            let selected = TABS.iter().position(|t| *t == panel.tab).unwrap_or(0);
            let mut tabs = vec![
                (TABS[0], "Fiche"),
                (TABS[1], "Budget"),
                (TABS[2], "Commerce"),
            ];
            if view.is_player {
                tabs.push((TABS[3], "Cour"));
            }
            let selected = selected.min(tabs.len() - 1);
            root.spawn(tab_bar(&theme, TAB_GROUP, &tabs, selected));
            root.spawn(tab_pane_fit(
                TAB_GROUP,
                TABS[0],
                selected == 0,
                TAB_HEIGHT,
                MIN_TAB_HEIGHT,
            ))
            .with_children(|pane| {
                pane.spawn(scroll_list_fill(&theme, "faction:scroll:sheet"))
                    .with_children(|list| sheet_tab(list, &theme, &assets, view));
            });
            root.spawn(tab_pane_fit(
                TAB_GROUP,
                TABS[1],
                selected == 1,
                TAB_HEIGHT,
                MIN_TAB_HEIGHT,
            ))
            .with_children(|pane| {
                pane.spawn(scroll_list_fill(&theme, "faction:scroll:budget"))
                    .with_children(|list| budget_tab(list, &theme, &assets, view));
            });
            root.spawn(tab_pane_fit(
                TAB_GROUP,
                TABS[2],
                selected == 2,
                TAB_HEIGHT,
                MIN_TAB_HEIGHT,
            ))
            .with_children(|pane| {
                pane.spawn(scroll_list_fill(&theme, "faction:scroll:trade"))
                    .with_children(|list| trade_tab(list, &theme, &assets, panel, view));
            });
            if view.is_player {
                root.spawn(tab_pane_fit(
                    TAB_GROUP,
                    TABS[3],
                    selected == 3,
                    TAB_HEIGHT,
                    MIN_TAB_HEIGHT,
                ))
                .with_children(|pane| {
                    pane.spawn(scroll_list_fill(&theme, "faction:scroll:court"))
                        .with_children(|list| {
                            court_tab(list, &theme, &assets, &codex, panel, view)
                        });
                });
            }
        });
}

pub(crate) fn ink(theme: &UiTheme, content: &str, ui_type: UiType, color: Color) -> impl Bundle {
    (
        Text::new(content),
        theme.text_font(false, false, ui_type.size()),
        TextColor(color),
    )
}

/// `content` as a text that wraps inside the width of its column.
pub(crate) fn wrapped(content: impl Bundle) -> impl Bundle {
    (
        content,
        Node {
            width: Val::Percent(100.0),
            ..default()
        },
    )
}

fn header(root: &mut Kids, theme: &UiTheme, panel: &FactionPanel, view: &FactionPanelView) {
    root.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            children![
                (
                    Text::new(view.sheet.name.clone()),
                    theme.title_font(UiType::Title.size()),
                    TextColor(style::INK_TITLE),
                ),
                caption(theme, &view.sheet.status_label),
            ],
        ));
        if let Some((ok, text)) = &panel.result
            && !text.is_empty()
        {
            row.spawn((
                Name::new("faction:result"),
                ink(
                    theme,
                    text,
                    UiType::Caption,
                    if *ok { style::GOOD } else { style::ERROR },
                ),
            ));
        }
        row.spawn(close_button(theme, "faction:close"));
    });
}

/// `label` on the left, `value` on the right of one line.
pub(super) fn pair(parent: &mut Kids, theme: &UiTheme, label: &str, value: &str, color: Color) {
    parent
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(ink(theme, label, UiType::Body, style::INK_SOFT));
            row.spawn(ink(theme, value, UiType::Body, color));
        });
}

fn sheet_tab(pane: &mut Kids, theme: &UiTheme, assets: &AssetServer, view: &FactionPanelView) {
    let sheet = &view.sheet;
    pair(pane, theme, "Seigneur", &sheet.ruler, style::INK);
    if let Some(title) = &sheet.primary_title {
        pair(pane, theme, "Titre principal", &title.name, style::INK);
    }
    pair(
        pane,
        theme,
        "Provinces",
        &fr::digits(sheet.provinces_count as i64),
        style::INK,
    );
    pair(
        pane,
        theme,
        "Prestige",
        &fr::digits(i64::from(sheet.prestige)),
        style::INK,
    );
    pair(
        pane,
        theme,
        "Trésor",
        &fr::amount(sheet.treasury),
        fr::color_of(sheet.treasury),
    );
    if let Some(liege) = &sheet.liege {
        let loyalty = sheet
            .loyalty
            .map_or(String::new(), |value| format!(" (loyauté {value})"));
        pair(
            pane,
            theme,
            "Suzerain",
            &format!("{}{loyalty}", liege.name),
            style::INK,
        );
    } else if sheet.sovereign.id != sheet.id {
        pair(pane, theme, "Souverain", &sheet.sovereign.name, style::INK);
    }
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Titres"));
    if sheet.titles.is_empty() {
        pane.spawn(caption(theme, "Aucun titre."));
    }
    for title in sheet.titles.iter().take(8) {
        pane.spawn(ink(
            theme,
            &format!("• {}", title.name),
            UiType::Body,
            style::INK,
        ));
    }
    if !sheet.direct_vassals.is_empty() {
        let names: Vec<&str> = sheet
            .direct_vassals
            .iter()
            .take(8)
            .map(|v| v.name.as_str())
            .collect();
        let more = sheet.direct_vassals.len().saturating_sub(names.len());
        let tail = if more > 0 {
            format!(" et {more} autres")
        } else {
            String::new()
        };
        pane.spawn(heading(theme, "Vassaux"));
        pane.spawn(ink(
            theme,
            &format!("{}{tail}", names.join(", ")),
            UiType::Body,
            style::INK,
        ));
    }
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Relations"));
    let relations = &sheet.relations;
    pane.spawn(ink(
        theme,
        &format!(
            "{} · {} · {} · {}",
            fr::count(relations.wars as i64, "guerre", Some("guerres")),
            fr::count(relations.truces as i64, "trêve", Some("trêves")),
            fr::count(relations.alliances as i64, "alliance", Some("alliances")),
            fr::count(relations.vassals as i64, "vassal", Some("vassaux")),
        ),
        UiType::Body,
        style::INK,
    ));
    for entry in relations
        .entries
        .iter()
        .filter(|e| e.status == "war")
        .take(5)
    {
        pane.spawn(ink(
            theme,
            &format!("• {} : {}", entry.name, status_text(&entry.status)),
            UiType::Caption,
            style::RUBRIC,
        ));
    }
    if !sheet.objectives.is_empty() {
        pane.spawn(separator(theme));
        pane.spawn(heading(theme, "Objectifs"));
        for objective in &sheet.objectives {
            let mark = if objective.met { "✓" } else { "•" };
            let mut line = pane.spawn(ink(
                theme,
                &format!("{mark} {}", objective.title),
                UiType::Body,
                if objective.met {
                    style::GOOD
                } else {
                    style::INK
                },
            ));
            line.insert(Tooltip::new(&format!(
                "[b]{}[/b]\n{}",
                objective.title, objective.description
            )));
        }
    }
    sections::feudal(pane, theme, view);
    sections::goods(pane, theme, assets, &view.goods);
}

/// Court tab (player only): order of chivalry and, for the crusaders, fervour.
fn court_tab(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    codex: &CodexStore,
    panel: &FactionPanel,
    view: &FactionPanelView,
) {
    if let Some(call) = &view.papal_call {
        sections::papal_call(pane, theme, assets, call, panel.join_armed);
    }
    if let Some(chivalry) = &view.chivalry {
        sections::chivalry(pane, theme, assets, codex, chivalry);
    }
    if let Some(crusade) = &view.crusade {
        sections::crusade(pane, theme, crusade);
    }
}

fn tax_note(rate: &str, multiplier: f64) -> String {
    let times = format!("×{multiplier:.1}").replace('.', ",");
    match rate {
        "low" => format!("{times} sur le revenu fiscal ; apaise le mécontentement."),
        "high" => format!("{times} sur le revenu fiscal ; augmente le mécontentement."),
        _ => format!("{times} sur le revenu fiscal."),
    }
}

fn budget_tab(pane: &mut Kids, theme: &UiTheme, assets: &AssetServer, view: &FactionPanelView) {
    let economy = &view.economy;
    pane.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Text::new("Trésor"),
            theme.text_font(true, false, UiType::Heading.size()),
            TextColor(style::INK),
        ));
        row.spawn((
            Name::new("faction:treasury"),
            Text::new(fr::amount(economy.treasury)),
            theme.text_font(true, false, UiType::Heading.size()),
            TextColor(fr::color_of(economy.treasury)),
            Tooltip::new("[b]Trésor[/b]\nLivres tournois (₶) en caisse ; le solde de la saison s’y ajoute en fin de tour."),
        ));
    });
    // Tax brackets.
    pane.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn(ink(theme, "Impôt", UiType::Body, style::INK_SOFT));
        for option in &view.tax {
            let caption = match option.key.as_str() {
                "low" => "Bas",
                "high" => "Haut",
                _ => "Normal",
            };
            let mut button = row.spawn(toggle_button(
                theme,
                &format!("faction:tax:{}", option.key),
                caption,
                option.selected,
            ));
            if !view.is_player {
                button.insert(bevy::ui::InteractionDisabled);
            }
        }
    });
    if let Some(current) = view.tax.iter().find(|t| t.selected) {
        pane.spawn(ink(
            theme,
            &tax_note(&current.key, current.multiplier),
            UiType::Caption,
            style::MUTED,
        ));
    }
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Budget de la saison"));
    budget_table(pane, theme, view);
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Trésor sur douze saisons"));
    treasury_chart(pane, theme, view);
    forecast_section(pane, theme, assets, view);
    if let Some(coinage) = &view.coinage {
        sections::coinage(pane, theme, coinage, view.is_player);
    }
}

fn cell(parent: &mut Kids, theme: &UiTheme, text: &str, size: UiType, color: Color, right: bool) {
    let mut node = Node {
        justify_content: if right {
            JustifyContent::FlexEnd
        } else {
            JustifyContent::FlexStart
        },
        ..default()
    };
    if right {
        node.width = Val::Px(NUMBER_WIDTH);
        node.flex_shrink = 0.0;
    } else {
        node.flex_grow = 1.0;
        node.flex_basis = Val::Px(0.0);
    }
    parent.spawn(node).with_children(|cell| {
        cell.spawn(ink(theme, text, size, color));
    });
}

fn table_row(
    pane: &mut Kids,
    theme: &UiTheme,
    cells: [&str; 4],
    size: UiType,
    colors: [Color; 4],
    hint: Option<(&str, &str)>,
) {
    let mut row = pane.spawn(Node {
        column_gap: Val::Px(8.0),
        ..default()
    });
    if let Some((title, body)) = hint {
        row.insert(Tooltip::new(&format!("[b]{title}[/b]\n{body}")));
    }
    row.with_children(|row| {
        for (index, (text, color)) in cells.iter().zip(colors).enumerate() {
            cell(row, theme, text, size, color, index > 0);
        }
    });
}

fn line_colour(value: i64) -> Color {
    if value < 0 {
        style::POOR
    } else if value > 0 {
        style::GOOD
    } else {
        style::INK
    }
}

fn budget_row(
    pane: &mut Kids,
    theme: &UiTheme,
    line: &BudgetLineView,
    has_past: bool,
    table: &[TableProvince],
) {
    let (name, hint) = RUBRICS
        .iter()
        .find(|(key, _, _)| *key == line.key)
        .map_or((line.key.as_str(), ""), |(_, name, hint)| (*name, *hint));
    let hint = if line.key == "table" {
        table_hint(hint, line, table)
    } else {
        hint.to_owned()
    };
    let other = line.key == "other";
    let projected = if other {
        "—".to_owned()
    } else {
        fr::signed(line.projected)
    };
    let last = match (has_past, line.last) {
        (true, Some(last)) => fr::signed(last),
        _ => "—".to_owned(),
    };
    let delta = match (has_past, line.delta) {
        (true, Some(delta)) if !other => fr::delta(delta),
        _ => "—".to_owned(),
    };
    let delta_color = line.delta.map_or(style::INK, fr::color_of);
    table_row(
        pane,
        theme,
        [&format!("   {name}"), &projected, &last, &delta],
        UiType::Body,
        [
            style::INK,
            if other {
                style::INK
            } else {
                line_colour(line.projected)
            },
            style::INK_SOFT,
            delta_color,
        ],
        Some((name, &hint)),
    );
}

pub(crate) fn budget_table(pane: &mut Kids, theme: &UiTheme, view: &FactionPanelView) {
    let economy = &view.economy;
    let has_past = economy.net_change.is_some();
    let head = [style::INK_SOFT; 4];
    table_row(
        pane,
        theme,
        ["Rubrique", "Prévu", "Saison passée", "Écart"],
        UiType::Caption,
        head,
        None,
    );
    table_row(
        pane,
        theme,
        ["RECETTES", "", "", ""],
        UiType::Caption,
        head,
        None,
    );
    for line in economy.budget_lines.iter().filter(|l| l.key == "receipts") {
        budget_row(pane, theme, line, has_past, &view.table);
        for source in &economy.income_lines {
            table_row(
                pane,
                theme,
                [
                    &format!("      · {}", income_label(&source.key)),
                    &fr::signed(source.value),
                    "",
                    "",
                ],
                UiType::Caption,
                [
                    style::INK_SOFT,
                    if source.value < 0 {
                        style::POOR
                    } else {
                        style::INK
                    },
                    style::INK,
                    style::INK,
                ],
                None,
            );
        }
    }
    table_row(
        pane,
        theme,
        ["DÉPENSES", "", "", ""],
        UiType::Caption,
        head,
        None,
    );
    for line in economy.budget_lines.iter().filter(|l| l.charge) {
        budget_row(pane, theme, line, has_past, &view.table);
    }
    for line in economy
        .budget_lines
        .iter()
        .filter(|l| l.key == "other" && has_past)
    {
        budget_row(pane, theme, line, has_past, &view.table);
    }
    if economy.mercenary_premium != 0 || economy.mercenary_premium_last_turn != 0 {
        table_row(
            pane,
            theme,
            [
                "   Surprime des mercenaires",
                &fr::signed(-economy.mercenary_premium),
                &fr::signed(-economy.mercenary_premium_last_turn),
                "",
            ],
            UiType::Body,
            [style::INK, style::POOR, style::INK_SOFT, style::INK],
            Some((
                "Surprime des mercenaires",
                "Ce que les compagnies coûtent au-delà de la solde ordinaire ; prélevé en fin de tour, en plus du solde.",
            )),
        );
    }
    let net = economy.net_income;
    let last = if has_past {
        fr::signed(economy.net_income_last_turn)
    } else {
        "—".to_owned()
    };
    let delta = economy.net_change.map_or("—".to_owned(), fr::delta);
    table_row(
        pane,
        theme,
        ["Solde de la saison", &fr::signed(net), &last, &delta],
        UiType::Body,
        [
            style::INK,
            fr::color_of(net),
            style::INK_SOFT,
            economy.net_change.map_or(style::INK, fr::color_of),
        ],
        Some((
            "Solde de la saison",
            "Recettes moins toutes les charges : le « Solde » de la barre du haut, ajouté au trésor en fin de tour.",
        )),
    );
}

/// Tooltip of the Table line: the diet of each province and what it costs
/// (`faction_panel.gd::_show_table_upkeep`).
fn table_hint(hint: &str, line: &BudgetLineView, table: &[TableProvince]) -> String {
    let mut lines = vec![
        hint.to_owned(),
        match line.last {
            Some(last) => format!("Saison passée : {}", fr::amount(last.abs())),
            None => "Saison passée : —".to_owned(),
        },
    ];
    for row in table.iter().take(TABLE_ROWS) {
        lines.push(format!(
            "• {} : {} — {}",
            row.name,
            row.diet,
            fr::amount(row.cost)
        ));
    }
    if table.len() > TABLE_ROWS {
        lines.push(format!(
            "… et {} autre(s) province(s).",
            table.len() - TABLE_ROWS
        ));
    }
    lines.join("\n")
}

/// Colour of a forecast delta (`ForecastText.color_of`).
fn tone(delta: f64, bad_when_up: bool) -> Color {
    if delta.abs() < 0.05 {
        style::INK
    } else if (delta > 0.0) == bad_when_up {
        style::POOR
    } else {
        style::GOOD
    }
}

/// Forecast of the three gauges and the main drivers of the balance (the overview tab of
/// `economy_overview_tab.gd`).
pub(crate) fn forecast_section(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    view: &FactionPanelView,
) {
    let Some(forecast) = &view.forecast else {
        return;
    };
    let body = RichStyle::caption(theme);
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Prévisions du prochain tour"));
    for (name, gauge, unit, bad_when_up, decimals) in [
        ("prestige", &forecast.prestige, "", false, false),
        ("men", &forecast.men, "", false, false),
        ("unrest", &forecast.unrest, " %", true, true),
    ] {
        let label = match name {
            "prestige" => "Prestige",
            "men" => "Hommes sous les armes",
            _ => "Troubles",
        };
        pane.spawn((
            Name::new(format!("faction:forecast:{name}")),
            ink(
                theme,
                &format!(
                    "{label} : {} → {}{unit} {}",
                    fr::digits(gauge.current),
                    fr::digits(gauge.next),
                    text::arrow(gauge)
                ),
                UiType::Body,
                tone(gauge.delta as f64, bad_when_up),
            ),
        ));
        pane.spawn(wrapped(rich_text(
            theme,
            assets,
            &text::gauge_tooltip(gauge, unit.trim_end_matches(' '), bad_when_up, decimals),
            &body,
        )));
    }
    let drivers = text::income_drivers(&view.economy, &view.top_provinces);
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Principales causes du solde"));
    if drivers.is_empty() {
        pane.spawn(caption(theme, "Aucune recette."));
    } else {
        pane.spawn(wrapped(rich_text(theme, assets, &drivers, &body)));
    }
}

/// Bars (balance of each season) and dots (the curve) drawn with UI nodes.
fn treasury_chart(pane: &mut Kids, theme: &UiTheme, view: &FactionPanelView) {
    let values: Vec<i64> = view.chart.iter().map(|p| p.treasury).collect();
    let geometry = chart::layout(&values, CHART_HEIGHT);
    pane.spawn(Node {
        height: Val::Px(CHART_HEIGHT),
        column_gap: Val::Px(2.0),
        ..default()
    })
    .with_children(|plot| {
        plot.spawn(Node {
            width: Val::Px(AXIS_WIDTH),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexEnd,
            padding: UiRect::right(Val::Px(4.0)),
            ..default()
        })
        .with_children(|axis| {
            axis.spawn(ink(
                theme,
                &short(geometry.high),
                UiType::Caption,
                style::INK_SOFT,
            ));
            axis.spawn(ink(
                theme,
                &short(geometry.low),
                UiType::Caption,
                style::INK_SOFT,
            ));
        });
        plot.spawn((
            Node {
                width: Val::Px(chart::PLOT_WIDTH),
                flex_shrink: 0.0,
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(style::PARCHMENT_LIGHT.with_alpha(0.5)),
        ))
        .with_children(|area| {
            area.spawn((
                Name::new("faction:chart_zero"),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(geometry.zero_y),
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(style::INK_SOFT.with_alpha(0.6)),
            ));
            // The curve: one rotated segment per pair of seasons.
            for segment in chart::segments(&geometry, chart::PLOT_WIDTH) {
                area.spawn((
                    ChartSegment,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(segment.mid_x - segment.length / 2.0),
                        top: Val::Px(segment.mid_y - chart::LINE_THICKNESS / 2.0),
                        width: Val::Px(segment.length),
                        height: Val::Px(chart::LINE_THICKNESS),
                        ..default()
                    },
                    UiTransform::from_rotation(Rot2::radians(segment.angle)),
                    BackgroundColor(if segment.negative {
                        style::POOR
                    } else {
                        style::INK
                    }),
                ));
            }
            area.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            })
            .with_children(|columns| {
                for (index, (bar, point)) in geometry.bars.iter().zip(&view.chart).enumerate() {
                    let colour = if bar.negative {
                        style::POOR
                    } else {
                        style::GOLD
                    };
                    let change = if index > 0 || point.change != 0 {
                        format!(" ({})", fr::signed(point.change))
                    } else {
                        String::new()
                    };
                    let width = 100.0 / view.chart.len().max(1) as f32;
                    columns
                        .spawn((
                            Name::new(format!("faction:chart:{index}")),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(width * index as f32),
                                width: Val::Percent(width),
                                top: Val::Px(0.0),
                                bottom: Val::Px(0.0),
                                ..default()
                            },
                            Tooltip::new(&format!(
                                "[b]{}[/b]\n{}{change}",
                                if point.now {
                                    "Maintenant".to_owned()
                                } else {
                                    season_label(point.turn)
                                },
                                fr::amount(point.treasury)
                            )),
                        ))
                        .with_children(|column| {
                            column.spawn((
                                ChartBar,
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Percent(20.0),
                                    width: Val::Percent(60.0),
                                    top: Val::Px(bar.top),
                                    height: Val::Px(bar.height),
                                    ..default()
                                },
                                BackgroundColor(colour.with_alpha(0.45)),
                            ));
                            column.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Percent(50.0),
                                    margin: UiRect::left(Val::Px(-3.0)),
                                    top: Val::Px(bar.dot_y - 3.0),
                                    width: Val::Px(6.0),
                                    height: Val::Px(6.0),
                                    border_radius: BorderRadius::all(Val::Px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(colour),
                            ));
                        });
                }
            });
        });
    });
    pane.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        margin: UiRect::left(Val::Px(AXIS_WIDTH + 2.0)),
        ..default()
    })
    .with_children(|labels| {
        let first = view
            .chart
            .first()
            .map_or(String::new(), |p| season_label(p.turn));
        labels.spawn(ink(theme, &first, UiType::Caption, style::INK_SOFT));
        if view.chart.len() > 1 {
            labels.spawn(ink(theme, "maintenant", UiType::Caption, style::INK_SOFT));
        }
    });
    if view.chart.len() < 2 {
        pane.spawn(caption(
            theme,
            "La courbe se remplit au fil des saisons (douze au plus).",
        ));
    }
}

/// « A ↔ B (terre) — 12 ₶ dont 4 ₶ pour nous — accord » or « … — coupée (guerre) ».
pub fn route_text(route: &TradeRow) -> String {
    let head = format!(
        "{} ↔ {} ({})",
        route.from_hub,
        route.to_hub,
        mode_label(&route.mode)
    );
    if route.cut {
        return if route.cut_reason.is_empty() {
            format!("{head} — coupée")
        } else {
            format!("{head} — coupée ({})", route.cut_reason)
        };
    }
    let mut text = format!("{head} — {}", fr::amount(route.total_value));
    if route.mine {
        text.push_str(&format!(" dont {} pour nous", fr::amount(route.my_value)));
    }
    if route.agreement {
        text.push_str(" — accord");
    }
    text
}

fn trade_tab(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    panel: &FactionPanel,
    view: &FactionPanelView,
) {
    let trade = &view.trade;
    let shown: Vec<&TradeRow> = trade
        .routes
        .iter()
        .filter(|route| !panel.mine_only || route.mine)
        .collect();
    let cut = shown.iter().filter(|route| route.cut).count();
    pane.spawn(ink(
        theme,
        &format!(
            "Revenu de la saison : {} (saison passée : {}) — {} affichée(s), {} coupée(s).",
            fr::amount(trade.income),
            fr::amount(trade.income_last_turn),
            shown.len(),
            cut
        ),
        UiType::Caption,
        style::INK_SOFT,
    ));
    pane.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn(toggle_button(
            theme,
            "faction:mine_only",
            "Mes routes",
            panel.mine_only,
        ));
        let mut button = row.spawn(button_small(theme, "faction:show_map", "Voir sur la carte"));
        button.insert(Tooltip::new(
            "[b]Voir sur la carte[/b]\nCentre la carte sur la première de vos routes affichées ; « Voir » sur une ligne centre sur son premier marché.",
        ));
    });
    pane.spawn(scroll_list(theme, "faction:routes", 230.0))
        .with_children(|list| {
            if shown.is_empty() {
                list.spawn(ink(
                    theme,
                    if panel.mine_only {
                        "Aucune route commerciale."
                    } else {
                        "Aucune route."
                    },
                    UiType::Caption,
                    style::MUTED,
                ));
            }
            for route in shown {
                let color = if route.cut {
                    style::ERROR
                } else if !route.mine {
                    style::MUTED
                } else {
                    style::INK
                };
                list.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(icon(theme, assets, "bld_market", 16.0));
                    row.spawn((
                        TradeRowMarker,
                        Name::new(format!("faction:route:{}", route.id)),
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            ..default()
                        },
                        ink(theme, &route_text(route), UiType::Caption, color),
                    ));
                    if !route.from_province.is_empty() {
                        row.spawn(button_small(
                            theme,
                            &format!("faction:route_view:{}", route.id),
                            "Voir",
                        ));
                    }
                });
            }
        });
    sections::partners(pane, theme, assets, panel, view);
}
