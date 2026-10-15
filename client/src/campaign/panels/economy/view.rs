//! Retained rebuild of the economy window: despawn the root, spawn it again from the view.

use bevy::prelude::*;
use game_api::faction_panel::{CoinageSection, FactionPanelView, PartnerRow};

use super::{EconomyPanel, PANEL_ID, TAB_GROUP, TABS, refresh};
use crate::campaign::panels::faction::chart::season_label;
use crate::campaign::panels::faction::sections::{coinage_short, coinage_tooltip};
use crate::campaign::panels::faction::view::{
    budget_table, forecast_section, income_label, ink, route_text, wrapped,
};
use crate::sim::Sim;
use crate::ui::kit::icons::icon;
use crate::ui::kit::panel_stack::{PanelTier, StackedPanel};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::KitWidget;
use crate::ui::kit::{
    UiTheme, UiTier, UiType, button_small, caption, fr, heading, scroll_list, separator, style,
    tab_bar, tab_pane,
};

#[derive(Component)]
pub struct EconomyRoot;

/// A data row of a tab (history season, route, agreement, partner, coinage level).
#[derive(Component)]
pub struct EconomyRow;

type Kids<'a> = ChildSpawnerCommands<'a>;

const PANEL_WIDTH: f32 = 640.0;
const TAB_HEIGHT: f32 = 460.0;
const HISTORY_SHOWN: usize = 8;
const MAX_ROUTES: usize = 14;
const MAX_PARTNERS: usize = 12;

const CURRENCY_INTRO: &str = "La monnaie de compte du royaume est la livre tournois (₶). Les rois « muent » la monnaie : moins d’argent fin dans chaque pièce rapporte un seigneuriage immédiat, mais les prix montent et les rentes fixes des bourgeois et du clergé fondent. Une monnaie affaiblie ralentit aussi le commerce. Pour changer de monnaie : section « Monnaie » du panneau de faction.";
const REVENUE_HELP: &str = "Les recettes viennent de l’impôt des provinces (taille des classes, production, bâtiments), du commerce (routes et accords), du seigneuriage de la frappe et des aumônes de la croisade ; les embargos, la dévastation et la difficulté les modèrent.";
/// Sources of income listed at the bottom of the currency tab (`IncomeLines.LABELS`).
const INCOME_KEYS: [&str; 16] = [
    "poll_peasants",
    "poll_burghers",
    "poll_clergy",
    "poll_nobility",
    "production",
    "tax_rate",
    "building_tax",
    "building_trade",
    "devastation",
    "full_province",
    "embargo",
    "domain",
    "difficulty",
    "alms",
    "seigniorage",
    "trade_routes",
];
const COINAGE_COLUMNS: [&str; 6] = [
    "Niveau",
    "Seigneuriage",
    "Refonte",
    "Inflation",
    "Troubles bourgeois",
    "Prestige",
];

pub fn rebuild(
    mut commands: Commands,
    theme: Res<UiTheme>,
    sim: Res<Sim>,
    assets: Res<AssetServer>,
    mut panel: ResMut<EconomyPanel>,
    roots: Query<Entity, With<EconomyRoot>>,
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
            Name::new("EconomyPanel"),
            EconomyRoot,
            StackedPanel::new(PanelTier::Central, PANEL_ID),
            KitWidget("panel"),
            UiTier::Panel.index(),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(6.0),
                top: Val::Px(40.0),
                width: Val::Px(PANEL_WIDTH),
                padding: theme.inner_padding("panel", 8.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            theme.sliced("panel"),
        ))
        .with_children(|root| {
            header(root, &theme, panel);
            let Some(view) = &panel.view else {
                root.spawn(caption(&theme, "Données économiques indisponibles."));
                return;
            };
            let names: Vec<String> = TABS
                .iter()
                .map(|(k, _)| format!("economy:tab:{k}"))
                .collect();
            let tabs: Vec<(&str, &str)> = names
                .iter()
                .zip(TABS)
                .map(|(name, (_, label))| (name.as_str(), label))
                .collect();
            let selected = TABS
                .iter()
                .position(|(key, _)| *key == panel.tab)
                .unwrap_or(0);
            root.spawn(tab_bar(&theme, TAB_GROUP, &tabs, selected));
            for (index, (key, _)) in TABS.iter().enumerate() {
                root.spawn(tab_pane(TAB_GROUP, &names[index], index == selected))
                    .with_children(|pane| {
                        pane.spawn(scroll_list(
                            &theme,
                            &format!("economy:scroll:{key}"),
                            TAB_HEIGHT,
                        ))
                        .with_children(|list| match *key {
                            "overview" => overview_tab(list, &theme, &assets, view),
                            "trade" => trade_tab(list, &theme, &assets, panel, view),
                            _ => currency_tab(list, &theme, view),
                        });
                    });
            }
        });
}

fn header(root: &mut Kids, theme: &UiTheme, panel: &EconomyPanel) {
    root.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            children![(
                Text::new("Économie"),
                theme.title_font(UiType::Title.size()),
                TextColor(style::INK_TITLE),
            )],
        ));
        if let Some((ok, text)) = &panel.result
            && !text.is_empty()
        {
            row.spawn((
                Name::new("economy:result"),
                ink(
                    theme,
                    text,
                    UiType::Caption,
                    if *ok { style::GOOD } else { style::ERROR },
                ),
            ));
        }
        row.spawn(button_small(theme, "economy:close", "×"));
    });
}

// ---------------------------------------------------------------- overview

fn overview_tab(pane: &mut Kids, theme: &UiTheme, assets: &AssetServer, view: &FactionPanelView) {
    let economy = &view.economy;
    pane.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Name::new("economy:treasury"),
            ink(
                theme,
                &format!("Trésor : {}", fr::amount(economy.treasury)),
                UiType::Heading,
                fr::color_of(economy.treasury),
            ),
        ));
        row.spawn(ink(
            theme,
            &format!("Solde de la saison : {}", fr::signed(economy.net_income)),
            UiType::Body,
            fr::color_of(economy.net_income),
        ));
    });
    pane.spawn(ink(
        theme,
        &format!(
            "Trésor au prochain tour : {}",
            fr::amount(economy.treasury + economy.net_income)
        ),
        UiType::Caption,
        style::INK_SOFT,
    ));
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Budget de la saison"));
    budget_table(pane, theme, view);
    forecast_section(pane, theme, assets, view);
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Historique du solde"));
    history(pane, theme, view);
}

/// Last resolved seasons, newest first, with a bar proportional to |balance|.
fn history(pane: &mut Kids, theme: &UiTheme, view: &FactionPanelView) {
    let records: Vec<_> = view.economy.budget_history.iter().collect();
    if records.is_empty() {
        pane.spawn(caption(theme, "Pas encore de saison écoulée."));
        return;
    }
    let start = records.len().saturating_sub(HISTORY_SHOWN);
    let shown = &records[start..];
    let peak = shown.iter().map(|r| r.net.abs()).max().unwrap_or(1).max(1);
    for record in shown.iter().rev() {
        let ratio = (record.net.abs() as f32 / peak as f32).clamp(0.0, 1.0);
        pane.spawn((
            EconomyRow,
            Name::new(format!("economy:history:{}", record.turn)),
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(110.0),
                    ..default()
                },
                ink(
                    theme,
                    &season_label(record.turn),
                    UiType::Caption,
                    style::INK_SOFT,
                ),
            ));
            row.spawn((
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(style::PARCHMENT_DARK),
                children![(
                    Node {
                        width: Val::Percent(ratio * 100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(fr::color_of(record.net)),
                )],
            ));
            row.spawn((
                Node {
                    width: Val::Px(90.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                ink(
                    theme,
                    &fr::signed(record.net),
                    UiType::Caption,
                    fr::color_of(record.net),
                ),
            ));
            row.spawn(ink(
                theme,
                &format!("trésor {}", fr::amount(record.treasury)),
                UiType::Caption,
                style::INK,
            ));
        });
    }
}

// ---------------------------------------------------------------- trade

fn trade_tab(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    panel: &EconomyPanel,
    view: &FactionPanelView,
) {
    let trade = &view.trade;
    pane.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|head| {
        head.spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                ..default()
            },
            ink(
                theme,
                &format!(
                    "Revenu des routes : {} par saison (saison passée : {}).",
                    fr::amount(trade.income),
                    fr::amount(trade.income_last_turn)
                ),
                UiType::Body,
                style::INK,
            ),
        ));
        head.spawn(button_small(theme, "economy:show_map", "Voir sur la carte"))
            .insert(Tooltip::new(
                "[b]Voir sur la carte[/b]\nCentre la carte sur la première de vos routes ; « Voir » sur une ligne centre sur son premier marché.",
            ));
    });
    // Agreements in force.
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Accords en cours"));
    let signed: Vec<&PartnerRow> = view.partners.iter().filter(|p| p.signed).collect();
    if signed.is_empty() {
        pane.spawn(caption(theme, "Aucun accord commercial en vigueur."));
    }
    for partner in signed {
        let armed = panel.break_armed.as_deref() == Some(partner.id.as_str());
        partner_row(
            pane,
            theme,
            assets,
            &crate::campaign::panels::faction::sections::agreement_text(partner),
            (
                &format!("economy:break:{}", partner.id),
                if armed { "Confirmer ?" } else { "Rompre" },
            ),
        );
    }
    // New routes.
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Nouvelles routes"));
    let open: Vec<&PartnerRow> = view
        .partners
        .iter()
        .filter(|p| !p.signed && p.gain_self > 0)
        .take(MAX_PARTNERS)
        .collect();
    if open.is_empty() {
        pane.spawn(caption(
            theme,
            "Aucun nouveau partenaire : routes coupées, guerre ou accords déjà signés.",
        ));
    }
    for partner in open {
        partner_row(
            pane,
            theme,
            assets,
            &crate::campaign::panels::faction::sections::partner_text(partner),
            (
                &format!("economy:propose:{}", partner.id),
                "Proposer l’accord",
            ),
        );
    }
    // Active routes.
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Routes commerciales actives"));
    let mine: Vec<_> = trade.routes.iter().filter(|r| r.mine).collect();
    if mine.is_empty() {
        pane.spawn(caption(theme, "Aucune route commerciale."));
    }
    for route in mine.iter().take(MAX_ROUTES) {
        pane.spawn((
            EconomyRow,
            Name::new(format!("economy:route:{}", route.id)),
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn(icon(theme, assets, "bld_market", 16.0));
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    ..default()
                },
                ink(
                    theme,
                    &route_text(route),
                    UiType::Caption,
                    if route.cut { style::ERROR } else { style::INK },
                ),
            ));
            if !route.from_province.is_empty() {
                row.spawn(button_small(
                    theme,
                    &format!("economy:route_view:{}", route.id),
                    "Voir",
                ));
            }
        });
    }
    let cut = mine.iter().filter(|r| r.cut).count();
    if cut > 0 {
        pane.spawn(ink(
            theme,
            &format!("{cut} route(s) coupée(s) : voir la raison sur la ligne."),
            UiType::Caption,
            style::MUTED,
        ));
    }
    // Goods.
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Échanges de ressources"));
    pane.spawn(wrapped(ink(
        theme,
        "Les biens voyagent avec les routes ci-dessus ; le royaume ne tient pas de stocks, donc pas d’achat ni de vente directs de ressources.",
        UiType::Caption,
        style::MUTED,
    )));
    pane.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(10.0),
        ..default()
    })
    .with_children(|flow| {
        for item in view.goods.iter().flat_map(|c| &c.items) {
            flow.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(3.0),
                ..default()
            })
            .with_children(|chip| {
                chip.spawn(icon(
                    theme,
                    assets,
                    &crate::campaign::panels::faction::sections::icon_id(
                        theme, &item.id, "resource",
                    ),
                    18.0,
                ));
                chip.spawn(ink(
                    theme,
                    &format!("{} × {}", item.name, item.count),
                    UiType::Caption,
                    style::INK,
                ));
            });
        }
    });
}

fn partner_row(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    text: &str,
    button: (&str, &str),
) {
    pane.spawn((
        EconomyRow,
        Name::new(button.0.replacen("economy:", "economy:row:", 1)),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn(icon(theme, assets, "bld_market", 18.0));
        row.spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                ..default()
            },
            ink(theme, text, UiType::Caption, style::INK),
        ));
        row.spawn(button_small(theme, button.0, button.1));
    });
}

// ---------------------------------------------------------------- currency

fn currency_tab(pane: &mut Kids, theme: &UiTheme, view: &FactionPanelView) {
    pane.spawn((
        Name::new("economy:currency_name"),
        heading(theme, "Monnaie : Livre tournois (₶)"),
    ));
    pane.spawn(wrapped(ink(
        theme,
        CURRENCY_INTRO,
        UiType::Caption,
        style::INK,
    )));
    let Some(coinage) = &view.coinage else {
        pane.spawn(caption(theme, "Données de monnaie indisponibles."));
        return;
    };
    pane.spawn(ink(
        theme,
        &format!("Frappe actuelle : {}", coinage.label),
        UiType::Body,
        style::INK,
    ));
    pane.spawn(ink(
        theme,
        &format!(
            "Niveau des prix : {} (100 = prix de départ)",
            coinage.price_level
        ),
        UiType::Caption,
        style::INK_SOFT,
    ));
    pane.spawn(separator(theme));
    coinage_table(pane, theme, coinage);
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Sources de revenu"));
    pane.spawn(wrapped(ink(
        theme,
        REVENUE_HELP,
        UiType::Caption,
        style::INK,
    )));
    for key in INCOME_KEYS {
        pane.spawn(ink(
            theme,
            &format!("· {}", income_label(key)),
            UiType::Caption,
            style::INK_SOFT,
        ));
    }
}

fn change(value: i64) -> String {
    if value == 0 {
        "0".to_owned()
    } else {
        fr::signed(value)
    }
}

fn coinage_table(pane: &mut Kids, theme: &UiTheme, coinage: &CoinageSection) {
    let cell = |row: &mut Kids, text: &str, first: bool, color: Color, bold: bool| {
        let mut node = Node {
            flex_grow: if first { 1.2 } else { 1.0 },
            flex_basis: Val::Px(0.0),
            ..default()
        };
        if !first {
            node.justify_content = JustifyContent::FlexEnd;
        }
        row.spawn(node).with_children(|c| {
            c.spawn((
                Text::new(text),
                theme.text_font(bold, false, UiType::Caption.size()),
                TextColor(color),
            ));
        });
    };
    pane.spawn(Node {
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|row| {
        for (index, title) in COINAGE_COLUMNS.iter().enumerate() {
            cell(row, title, index == 0, style::INK_SOFT, false);
        }
    });
    for option in &coinage.options {
        let color = if option.current {
            fr::GAIN_COLOR
        } else {
            style::INK
        };
        let inflation = if option.inflation > 0 {
            format!("+{}", option.inflation)
        } else if option.deflation > 0 {
            format!("{}{}", fr::MINUS, option.deflation)
        } else {
            "0".to_owned()
        };
        let unrest = if option.burgher_unrest.abs() < 0.0001 {
            "0".to_owned()
        } else {
            format!("{:+.1}", option.burgher_unrest)
                .replace('.', ",")
                .replace('-', &fr::MINUS.to_string())
        };
        let label = format!(
            "{}{}",
            coinage_short(&option.label),
            if option.current { " (actuelle)" } else { "" }
        );
        pane.spawn((
            EconomyRow,
            Name::new(format!("economy:coinage:{}", option.level)),
            Node {
                column_gap: Val::Px(8.0),
                ..default()
            },
            Tooltip::new(&coinage_tooltip(option, coinage.changed_this_year)),
        ))
        .with_children(|row| {
            cell(row, &label, true, color, option.current);
            cell(
                row,
                &change(option.seigniorage),
                false,
                color,
                option.current,
            );
            cell(
                row,
                &change(-option.recoinage),
                false,
                color,
                option.current,
            );
            cell(row, &inflation, false, color, option.current);
            cell(row, &unrest, false, color, option.current);
            cell(
                row,
                &option.prestige.to_string(),
                false,
                color,
                option.current,
            );
        });
    }
}
