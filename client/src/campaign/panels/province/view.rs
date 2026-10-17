//! Widget tree of the province panel, built from a [`Snapshot`] (pure data from `game-api`).
//! Ports game/scripts/map/province_panel.gd, province_tax_section.gd, classes_section.gd,
//! settlement_buildings_view.gd (tree of buildings with tiers) and settlements_section.gd.

use bevy::prelude::*;
use game_api::pprovince::{
    BuildNodeView, BuildingTreeView, CancelQuoteView, DemolitionView, EdictView, PossessionView,
    SlotBarView, SlotView, TableView, VocationView,
};
use game_api::province::{ClassView, ProvinceSheet, SettlementView};
use serde::Serialize;

use super::choices;
use super::images::{self, BuildingImages};
use super::labels::{self, UiLabels};
use crate::ui::kit::close_button;
use crate::ui::kit::icons::icon;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{
    button_primary, button_small, caption, checkbox, heading, list_item, separator, tab_bar,
    tab_pane,
};
use crate::ui::kit::{UiTheme, fr, typed};

pub const TAB_GROUP: &str = "prov";
pub const SETTLEMENT_GROUP: &str = "provset";
pub const TABS: [(&str, &str); 4] = [
    ("prov:tab_city", "Ville"),
    ("prov:tab_buildings", "Bâtiments"),
    ("prov:tab_settlements", "Colonies"),
    ("prov:tab_garrison", "Garnison"),
];

/// Everything one rebuild of the panel shows.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub sheet: ProvinceSheet,
    pub tree: Option<BuildingTreeView>,
    pub demolitions: Vec<DemolitionView>,
    /// Quote per queue row (same order as `tree.queue`).
    pub quotes: Vec<Option<CancelQuoteView>>,
    /// The player controls the province (orders available).
    pub mine: bool,
    pub selected_settlement: String,
    /// Development tier (1..=6) of the selected settlement and progress to the next (0..=100).
    pub tier: u8,
    pub tier_progress: u8,
    /// Last refused order, French.
    pub error: String,
    pub possession: Option<PossessionView>,
    pub table: Option<TableView>,
    pub edict: Option<EdictView>,
    /// Vocation of the selected settlement.
    pub vocation: Option<VocationView>,
    /// Edicts and diets can be changed (the player holds the whole province).
    pub edict_editable: bool,
    /// Regiments an army may hold at most.
    pub army_cap: u32,
    /// Garrison indices ticked for the new army.
    pub picked: Vec<usize>,
    /// Unfolded choice list (`table`, `edict`, `vocation`, or empty).
    pub open_choice: String,
    /// Slot bar of every colony of the province (Colonies tab), then of the selected one.
    pub bars: Vec<SlotBarView>,
    /// Building row marked after a click on a slot cell (empty = none).
    pub focus_building: String,
    /// Full Lent text (BBCode, rule values and Codex link resolved) when it is Lent.
    pub lent_text: Option<String>,
    /// Descriptions with Codex words linked, by choice key (`table`, `edict`).
    pub linked: std::collections::BTreeMap<String, String>,
    /// « Can build here » verdict of every colony the player holds (hammer, companion selector).
    pub verdicts: Vec<super::companion::Verdict>,
    /// The build companion is open under the Colonies list.
    pub companion: bool,
}

pub type Kids<'a, 'b> = ChildSpawnerCommands<'a>;

fn text_cell(theme: &UiTheme, content: &str, variation: UiType) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            min_width: Val::Px(0.0),
            ..default()
        },
        typed(theme, content, variation),
    )
}

fn row() -> Node {
    Node {
        column_gap: Val::Px(6.0),
        align_items: AlignItems::Center,
        width: Val::Percent(100.0),
        ..default()
    }
}

fn wrap_row() -> Node {
    Node {
        column_gap: Val::Px(4.0),
        row_gap: Val::Px(2.0),
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        width: Val::Percent(100.0),
        ..default()
    }
}

fn colored(theme: &UiTheme, content: &str, color: Color, variation: UiType) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            min_width: Val::Px(0.0),
            ..default()
        },
        Text::new(content),
        theme.text_font(false, false, variation.size()),
        TextColor(color),
    )
}

fn money(value: i64) -> String {
    fr::amount(value)
}

/// Header, tab bar and the four panes.
pub fn build(
    root: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    labels: &UiLabels,
    images: &BuildingImages,
    snap: &Snapshot,
    tab: usize,
) {
    header(root, theme, snap);
    root.spawn(tab_bar(theme, TAB_GROUP, &TABS, tab));
    root.spawn((
        Name::new("prov:body"),
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            overflow: Overflow::scroll_y(),
            row_gap: Val::Px(6.0),
            ..default()
        },
        bevy::ui_widgets::ScrollArea,
    ))
    .with_children(|body| {
        body.spawn(tab_pane(TAB_GROUP, TABS[0].0, tab == 0))
            .with_children(|pane| city_tab(pane, theme, assets, labels, snap));
        body.spawn(tab_pane(TAB_GROUP, TABS[1].0, tab == 1))
            .with_children(|pane| buildings_tab(pane, theme, assets, labels, images, snap));
        body.spawn(tab_pane(TAB_GROUP, TABS[2].0, tab == 2))
            .with_children(|pane| settlements_tab(pane, theme, assets, labels, images, snap));
        body.spawn(tab_pane(TAB_GROUP, TABS[3].0, tab == 3))
            .with_children(|pane| garrison_tab(pane, theme, snap));
    });
    if !snap.error.is_empty() {
        root.spawn((
            Name::new("prov:error"),
            Text::new(snap.error.clone()),
            theme.text_font(false, false, UiType::Caption.size()),
            TextColor(style::ERROR),
        ));
    }
}

fn header(root: &mut Kids, theme: &UiTheme, snap: &Snapshot) {
    let sheet = &snap.sheet;
    root.spawn(row()).with_children(|line| {
        line.spawn((
            Name::new("prov:title"),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            heading(theme, &sheet.name),
        ));
        line.spawn(close_button(theme, "prov:close"));
    });
    if !sheet.breadcrumb.is_empty() {
        root.spawn((Name::new("prov:breadcrumb"), wrap_row()))
            .with_children(|crumbs| {
                for (index, link) in sheet.breadcrumb.iter().enumerate() {
                    if index > 0 {
                        crumbs.spawn(caption(theme, "›"));
                    }
                    let holder = if link.holder_name.is_empty() {
                        "vacant"
                    } else {
                        &link.holder_name
                    };
                    let tip = format!("[b]{}[/b]\n{holder}", link.title_name);
                    if link.holder.is_empty() {
                        crumbs.spawn((Tooltip::new(&tip), caption(theme, &link.title_name)));
                    } else {
                        crumbs.spawn((
                            button_small(
                                theme,
                                &format!("prov:crumb:{}", link.holder),
                                &link.title_name,
                            ),
                            Tooltip::new(&format!("{tip}\nOuvre la fiche de la faction.")),
                        ));
                    }
                }
            });
    }
    if let Some(possession) = &snap.possession {
        root.spawn((
            Name::new("prov:lord"),
            Tooltip::new(&choices::possession_tooltip(possession)),
            Text::new(choices::status_line(possession)),
            theme.text_font(false, false, UiType::Body.size()),
            TextColor(choices::cue_color(&possession.cue)),
        ));
    } else {
        let mut lord = format!("Seigneur : {}", sheet.owner_name);
        if sheet.controller != sheet.owner {
            lord.push_str(&format!(" (occupée par {})", sheet.controller_name));
        }
        root.spawn((Name::new("prov:lord"), typed(theme, &lord, UiType::Body)));
    }
    root.spawn(wrap_row()).with_children(|gauges| {
        let revolt = format!(
            "Au-delà de {} pendant {} saisons : révolte (actuellement {} saison(s)).",
            sheet.revolt_threshold, sheet.revolt_seasons_needed, sheet.revolt_seasons
        );
        let chips = [
            (
                "prov:gauge_population",
                format!("Pop. {}", fr::digits(sheet.population_total as i64)),
                "[b]Population[/b]\nHabitants de la province, toutes classes.".to_owned(),
            ),
            (
                "prov:gauge_unrest",
                format!("Mécont. {}", sheet.unrest),
                format!("[b]Mécontentement[/b]\nPondéré par classe. {revolt}"),
            ),
            (
                "prov:gauge_devastation",
                format!("Dévast. {} %", sheet.devastation),
                "[b]Dévastation[/b]\nRuines des pillages et des sièges ; pèse sur la richesse et le mécontentement.".to_owned(),
            ),
        ];
        for (name, text, tip) in chips {
            gauges.spawn((
                Name::new(name),
                Tooltip::new(&tip),
                Node {
                    margin: UiRect::right(Val::Px(8.0)),
                    ..default()
                },
                caption(theme, &text),
            ));
        }
    });
    if let Some(siege) = &sheet.siege {
        root.spawn((
            Name::new("prov:siege"),
            Text::new(format!(
                "Siège par {} : tour {} (reste {}), ravitaillement {}, brèche {}",
                siege.attacker, siege.turns_elapsed, siege.turns_left, siege.supplies, siege.breach
            )),
            theme.text_font(true, false, UiType::Caption.size()),
            TextColor(style::RUBRIC),
        ));
    }
}

fn class_row(pane: &mut Kids, theme: &UiTheme, labels: &UiLabels, class: &ClassView) {
    let mut tip = format!(
        "[b]{}[/b]\n{} habitants",
        labels.class(&class.class),
        fr::digits(class.count as i64)
    );
    if !class.unrest_terms.is_empty() {
        tip.push_str("\n\n[b]Cible du mécontentement[/b]");
        for term in class.unrest_terms.iter().filter(|t| t.value != 0) {
            tip.push_str(&format!(
                "\n{} : {}",
                labels::unrest_term(&term.key),
                labels::signed_value(term.value as f64, false)
            ));
        }
    }
    pane.spawn((
        Name::new(format!("prov:class_{}", class.class)),
        Tooltip::new(&tip),
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ))
    .with_children(|col| {
        col.spawn(row()).with_children(|line| {
            line.spawn(text_cell(theme, &labels.class(&class.class), UiType::Body));
            line.spawn(typed(theme, &fr::digits(class.count as i64), UiType::Body));
        });
        col.spawn(caption(
            theme,
            &format!(
                "Mécont. {} · Santé {} · Richesse {} · Biens {}",
                class.unrest, class.health, class.wealth, class.goods_satisfaction
            ),
        ));
    });
}

fn city_tab(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    labels: &UiLabels,
    snap: &Snapshot,
) {
    let sheet = &snap.sheet;
    if let Some(edict) = &snap.edict {
        let mut choice = choices::edict_choice(labels, edict);
        choice.description_rich = snap.linked.get("edict").cloned();
        let pending = choices::edict_pending_line(edict);
        choices::choice_section(
            pane,
            theme,
            assets,
            &choice,
            snap.edict_editable,
            snap.open_choice == "edict",
            pending.as_deref().filter(|_| snap.edict_editable),
            None,
        );
        pane.spawn(separator(theme));
    }
    if let Some(table) = &snap.table {
        let mut choice = choices::table_choice(labels, table);
        choice.description_rich = snap.linked.get("table").cloned();
        choices::choice_section(
            pane,
            theme,
            assets,
            &choice,
            snap.mine,
            snap.open_choice == "table",
            choices::table_banner(table),
            snap.lent_text.as_deref().filter(|_| table.lent),
        );
        pane.spawn(separator(theme));
    }
    if let Some(governor) = &sheet.governor_name {
        pane.spawn(typed(
            theme,
            &format!("Gouverneur : {governor}"),
            UiType::Body,
        ));
    } else {
        pane.spawn(caption(theme, "Pas de gouverneur."));
    }
    pane.spawn(heading(theme, "Impôt"));
    let origin = if sheet.tax.own {
        "propre à la province"
    } else {
        "taux de la faction"
    };
    pane.spawn((
        Name::new("prov:tax_line"),
        typed(
            theme,
            &format!(
                "Taux {} ({origin}) · revenu {}",
                labels::tax_rate(&sheet.tax.rate).to_lowercase(),
                money(sheet.income)
            ),
            UiType::Body,
        ),
    ));
    if snap.mine {
        pane.spawn(wrap_row()).with_children(|line| {
            for (key, text) in [("low", "Faible"), ("normal", "Normal"), ("high", "Lourd")] {
                let name = format!("prov:tax:{key}");
                let active = sheet.tax.own && sheet.tax.rate == key;
                let caption_text = if active {
                    format!("✓ {text}")
                } else {
                    text.to_owned()
                };
                line.spawn((
                    button_small(theme, &name, &caption_text),
                    Tooltip::new(&format!(
                        "Fixe l'impôt de cette province seule : {}.",
                        text.to_lowercase()
                    )),
                ));
            }
            line.spawn((
                button_small(theme, "prov:tax:default", "Taux de la faction"),
                Tooltip::new("Revient au taux de la faction."),
            ));
        });
    }
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Population"));
    for class in &sheet.classes {
        class_row(pane, theme, labels, class);
    }
    pane.spawn(separator(theme));
    pane.spawn(typed(
        theme,
        &format!(
            "Fortification niveau {} · capacité {}",
            sheet.fortification_level,
            fr::digits(sheet.capacity as i64)
        ),
        UiType::Body,
    ));
    if !sheet.resources.is_empty() {
        pane.spawn((
            Name::new("prov:resources"),
            typed(
                theme,
                &format!(
                    "Ressources : {}",
                    sheet.resources.join(", ").replace('_', " ")
                ),
                UiType::Body,
            ),
        ));
    }
    let shown: Vec<_> = sheet
        .effects
        .iter()
        .filter(|e| e.flat != 0.0 || e.percent != 0.0)
        .collect();
    if !shown.is_empty() {
        pane.spawn(heading(theme, "Effets des bâtiments"));
        for effect in shown {
            let mut parts = Vec::new();
            if effect.flat != 0.0 {
                parts.push(labels::signed_value(effect.flat, false));
            }
            if effect.percent != 0.0 {
                parts.push(labels::signed_value(effect.percent, true));
            }
            pane.spawn(caption(
                theme,
                &format!("{} {}", labels.effect(&effect.key), parts.join(" ")),
            ));
        }
    }
    if snap.mine {
        pane.spawn(separator(theme));
        pane.spawn((
            button_small(theme, "prov:recruit", "Recruter"),
            Tooltip::new("Ouvre le recrutement de la cité."),
        ));
    }
}

fn node_tooltip(node: &BuildNodeView, labels: &UiLabels) -> String {
    let mut tip = format!(
        "[b]{}[/b]  [i]{} · palier {}[/i]\nCoût {} · {} tour(s) · entretien {}",
        node.name,
        labels.category(&node.category),
        node.tier,
        money(i64::from(node.cost)),
        node.turns,
        money(i64::from(node.upkeep))
    );
    if node.takes_slot {
        tip.push_str("\nOccupe un nouvel emplacement.");
    } else if node.depth > 1 {
        tip.push_str("\nAmélioration : remplace le palier précédent.");
    }
    if !node.effects.is_empty() {
        tip.push_str("\n\n[b]Effets[/b]");
        for effect in &node.effects {
            let scope = if effect.class.is_empty() {
                String::new()
            } else {
                format!(" ({})", labels.class(&effect.class).to_lowercase())
            };
            tip.push_str(&format!(
                "\n{} {}{scope}",
                labels.effect(&effect.key),
                labels::signed_value(effect.value, effect.percent)
            ));
        }
    }
    if !node.before_after.is_empty() {
        tip.push_str("\n\n[b]Avant → après[/b]");
        for line in &node.before_after {
            let key = line.key.split(':').next().unwrap_or_default();
            let scope = line.key.split_once(':').map_or(String::new(), |(_, c)| {
                format!(" ({})", labels.class(c).to_lowercase())
            });
            tip.push_str(&format!(
                "\n{}{scope} : {} → {}",
                labels.effect(key),
                round2(line.before),
                round2(line.after)
            ));
        }
    }
    if !node.requirements.is_empty() {
        tip.push_str("\n\n[b]Prérequis[/b]");
        for req in &node.requirements {
            let (mark, color) = if req.met {
                ("✓", "#3d5a1f")
            } else {
                ("✗", "#8b2a1a")
            };
            tip.push_str(&format!("\n[color={color}]{mark}[/color] {}", req.text));
        }
    }
    if !node.reason.is_empty() {
        tip.push_str(&format!("\n\n[color=#8b2a1a]{}[/color]", node.reason));
    } else if !node.blocked_now.is_empty() {
        tip.push_str(&format!("\n\n[color=#8b2a1a]{}[/color]", node.blocked_now));
    }
    tip
}

fn round2(value: f64) -> String {
    if (value - value.round()).abs() < 0.005 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.2}").replace('.', ",")
    }
}

fn buildings_tab(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    labels: &UiLabels,
    images: &BuildingImages,
    snap: &Snapshot,
) {
    let sheet = &snap.sheet;
    pane.spawn(caption(theme, "Colonie :"));
    pane.spawn(wrap_row()).with_children(|line| {
        for place in &sheet.settlements {
            let name = format!("prov:set:{}", place.id);
            line.spawn(list_item(
                theme,
                SETTLEMENT_GROUP,
                &name,
                &format!("{} ({})", place.name, labels::settlement_kind(&place.kind)),
                place.id == snap.selected_settlement,
            ));
        }
    });
    if let Some(vocation) = &snap.vocation {
        let choice = choices::vocation_choice(labels, vocation);
        choices::choice_section(
            pane,
            theme,
            assets,
            &choice,
            snap.mine,
            snap.open_choice == "vocation",
            None,
            None,
        );
    }
    buildings_view(pane, theme, assets, labels, images, snap);
}

/// Tree of buildings of the selected colony: tier plate, slots, queue, buildable nodes
/// (`SettlementBuildingsView`). Shared by the Buildings tab and the Colonies companion.
pub(super) fn buildings_view(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    labels: &UiLabels,
    images: &BuildingImages,
    snap: &Snapshot,
) {
    let Some(tree) = &snap.tree else {
        pane.spawn(caption(theme, "Arbre de construction indisponible."));
        return;
    };
    header_plate(pane, theme, assets, images, snap, &tree.kind);
    let slots = match tree.slots_max {
        Some(max) => format!("{}/{}", tree.slots_used, max),
        None => format!("{}", tree.slots_used),
    };
    pane.spawn((
        Name::new("prov:slots"),
        Tooltip::new("[b]Emplacements[/b]\nChaque chaîne de bâtiments en occupe un ; les paliers suivants l'améliorent sans en prendre de nouveau."),
        typed(
            theme,
            &format!(
                "{} · emplacements {slots} · chantiers {}/{}",
                tree.kind_label,
                tree.queue.len(),
                tree.queue_size
            ),
            UiType::Body,
        ),
    ));
    if let Some(bar) = snap
        .bars
        .iter()
        .find(|b| b.settlement == snap.selected_settlement)
    {
        slot_bar(pane, theme, assets, bar, "Emplacements");
    }
    if !tree.queue.is_empty() {
        pane.spawn(heading(theme, "Chantiers"));
        for (index, work) in tree.queue.iter().enumerate() {
            pane.spawn((Name::new(format!("prov:queue_{index}")), row()))
                .with_children(|line| {
                    line.spawn(text_cell(
                        theme,
                        &format!(
                            "{} — {}/{} tours",
                            work.name,
                            work.total_turns.saturating_sub(work.turns_left),
                            work.total_turns
                        ),
                        UiType::Body,
                    ));
                    if snap.mine {
                        let tip = snap.quotes.get(index).and_then(Option::as_ref).map_or_else(
                            || "Annule le chantier.".to_owned(),
                            |q| {
                                format!(
                                    "Annuler : {} remboursés sur {} payés.",
                                    money(i64::from(q.refund)),
                                    money(i64::from(q.paid))
                                )
                            },
                        );
                        line.spawn((
                            button_small(theme, &format!("prov:cancel:{index}"), "Annuler"),
                            Tooltip::new(&tip),
                        ));
                    }
                });
        }
    }
    let mut category = String::new();
    for node in tree.nodes.iter().filter(|n| n.allowed_here) {
        if node.category != category {
            category = node.category.clone();
            pane.spawn(typed(theme, &labels.category(&category), UiType::Heading));
        }
        node_row(pane, theme, assets, labels, images, snap, node);
    }
}

/// Tier caption, progress line and plate (`SettlementBuildingsView._fill_header`).
fn header_plate(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    images: &BuildingImages,
    snap: &Snapshot,
    kind: &str,
) {
    pane.spawn(typed(
        theme,
        &images.tier_caption(kind, snap.tier),
        UiType::Heading,
    ));
    let progress = if snap.tier >= images::MAX_TIER {
        "Palier le plus haut atteint.".to_owned()
    } else {
        format!(
            "Vers le palier {} : {} %. Bâtir et améliorer fait grandir la colonie.",
            snap.tier + 1,
            snap.tier_progress.min(100)
        )
    };
    pane.spawn((Name::new("prov:tier_progress"), caption(theme, &progress)));
    if let Some(path) = images::header_image(images, snap) {
        pane.spawn((
            Name::new("prov:tier_image"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(images::HEADER_IMAGE_HEIGHT),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(style::INK_FADED),
            ImageNode::new(assets.load(path)).with_mode(NodeImageMode::Auto),
        ));
    }
}

/// Card illustration of a building: its `bld_*` plate, else the painted icon.
fn node_image(
    line: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    images: &BuildingImages,
    building: &str,
) {
    let name = Name::new(format!("prov:node_image:{building}"));
    let slot = Node {
        width: Val::Px(images::CARD_IMAGE_HEIGHT * 1.4),
        height: Val::Px(images::CARD_IMAGE_HEIGHT),
        flex_shrink: 0.0,
        justify_content: JustifyContent::Center,
        ..default()
    };
    if let Some(path) = images.building_image(building) {
        line.spawn((name, slot, ImageNode::new(assets.load(path))));
    } else {
        line.spawn((name, slot)).with_children(|holder| {
            holder.spawn(icon(theme, assets, building, images::CARD_IMAGE_HEIGHT));
        });
    }
}

fn node_row(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    labels: &UiLabels,
    images: &BuildingImages,
    snap: &Snapshot,
    node: &BuildNodeView,
) {
    let color = match node.state.as_str() {
        "built" => style::GOOD,
        "building" | "queued" => style::FAIR,
        "locked" => style::INK_FADED,
        _ => style::INK,
    };
    let indent = f32::from(node.depth.saturating_sub(1)) * 12.0;
    let focused = !snap.focus_building.is_empty() && snap.focus_building == node.building;
    pane.spawn((
        Name::new(format!("prov:node:{}", node.building)),
        Tooltip::new(&node_tooltip(node, labels)),
        Node {
            padding: UiRect::left(Val::Px(indent)),
            border: UiRect::all(Val::Px(if focused { 2.0 } else { 0.0 })),
            ..row()
        },
        BorderColor::all(if focused { style::GOLD } else { Color::NONE }),
    ))
    .with_children(|line| {
        node_image(line, theme, assets, images, &node.building);
        let status = match node.state.as_str() {
            "available" => format!("{} ₶ · {} t", fr::digits(i64::from(node.cost)), node.turns),
            "building" | "queued" => format!(
                "{} ({} t)",
                labels::node_state(&node.state),
                node.turns_left
            ),
            other => labels::node_state(other).to_owned(),
        };
        line.spawn(colored(
            theme,
            &format!("{} · palier {}", node.name, node.tier),
            color,
            UiType::Body,
        ));
        line.spawn((
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Text::new(status),
            theme.text_font(false, false, UiType::Caption.size()),
            TextColor(color),
        ));
        if !snap.mine {
            return;
        }
        match node.state.as_str() {
            "available" if node.can_build => {
                line.spawn(button_primary(
                    theme,
                    &format!("prov:build:{}", node.building),
                    "Construire",
                ));
            }
            "available" => {
                line.spawn((
                    button_small(
                        theme,
                        &format!("prov:build:{}", node.building),
                        "Construire",
                    ),
                    bevy::ui::InteractionDisabled,
                ));
            }
            "built" if node.current => {
                if let Some(preview) = snap
                    .demolitions
                    .iter()
                    .find(|d| d.building == node.building)
                {
                    let mut tip = format!(
                        "Raser {} : remboursement {}, entretien économisé {}.",
                        node.name,
                        money(preview.refund),
                        money(preview.upkeep_saved)
                    );
                    if !preview.can_demolish {
                        tip = format!("{}\n[color=#8b2a1a]{}[/color]", tip, preview.reason);
                    }
                    let mut bundle = line.spawn((
                        button_small(theme, &format!("prov:raze:{}", node.building), "Raser"),
                        Tooltip::new(&tip),
                    ));
                    if !preview.can_demolish {
                        bundle.insert(bevy::ui::InteractionDisabled);
                    }
                }
            }
            _ => {}
        }
    });
}

/// Tooltip (BBCode) of one slot cell.
pub fn slot_tooltip(slot: &SlotView) -> String {
    match slot.state.as_str() {
        "built" | "upgradable" => {
            let mut tip = format!(
                "[b]{}[/b]\nNiveau {}/{} · entretien {}",
                slot.built_name,
                slot.level,
                slot.max_level,
                fr::amount(i64::from(slot.upkeep))
            );
            if slot.state == "upgradable" {
                tip.push_str(&format!(
                    "\nAmélioration : {} ({})",
                    slot.next_name,
                    fr::amount(i64::from(slot.next_cost))
                ));
            }
            tip
        }
        "locked" => format!("[b]Emplacement fermé[/b]\n{}", slot.locked_reason),
        _ if slot.next_name.is_empty() => "[b]Emplacement libre[/b]".to_owned(),
        _ => format!(
            "[b]Emplacement libre[/b]\nConstruire : {} ({})",
            slot.next_name,
            fr::amount(i64::from(slot.next_cost))
        ),
    }
}

/// Short text of a slot cell (`SettlementSlotBar._cell`).
pub fn slot_cell_text(slot: &SlotView) -> String {
    match slot.state.as_str() {
        "built" if slot.max_level > 1 => format!("Niv. {}", slot.level),
        "built" => "Bâti".to_owned(),
        "upgradable" => format!("{}/{} +", slot.level, slot.max_level),
        "locked" => "—".to_owned(),
        _ => "+".to_owned(),
    }
}

/// Slot bar of a colony: one cell per slot (icon of the building or of the first option, level,
/// « + » for an empty one, greyed when locked), a click opens the colony in the Buildings tab
/// (`prov:slot:<settlement>:<root>`). Title: `used/max` of the colony's kind.
fn slot_bar(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    bar: &SlotBarView,
    title: &str,
) {
    let built = bar.slots.iter().filter(|s| !s.built.is_empty()).count();
    let usage = match bar.max {
        // A historical start can stand above the cap (kept, ADR 0185 rule): say so.
        Some(max) if bar.used > max => {
            format!("{title} {}/{max} (au-delà du plafond, conservés)", bar.used)
        }
        Some(max) => format!("{title} {}/{max}", bar.used),
        None => format!("{title} {built}/{}", bar.slots.len()),
    };
    pane.spawn((
        Name::new(format!("prov:slot_bar:{}", bar.settlement)),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },
    ))
    .with_children(|column| {
        column.spawn(caption(theme, &usage));
        column.spawn(wrap_row()).with_children(|cells| {
            for slot in &bar.slots {
                let shown = if slot.built.is_empty() {
                    &slot.root
                } else {
                    &slot.built
                };
                let mut cell = cells.spawn((
                    button_small(
                        theme,
                        &format!("prov:slot:{}:{}", bar.settlement, slot.root),
                        &slot_cell_text(slot),
                    ),
                    Tooltip::new(&slot_tooltip(slot)),
                ));
                let glyph = cell.commands().spawn(icon(theme, assets, shown, 22.0)).id();
                cell.insert_children(0, &[glyph]);
                if slot.state == "locked" {
                    cell.insert(bevy::ui::InteractionDisabled);
                }
            }
        });
    });
}

fn settlement_card(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    snap: &Snapshot,
    place: &SettlementView,
) {
    pane.spawn((
        Name::new(format!("prov:colony_card:{}", place.id)),
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ))
    .with_children(|card| {
        card.spawn(row()).with_children(|line| {
            line.spawn(text_cell(
                theme,
                &format!("{} ({})", place.name, labels::settlement_kind(&place.kind)),
                UiType::Body,
            ));
            if let Some(verdict) = snap.verdicts.iter().find(|v| v.settlement == place.id) {
                super::companion::spawn_hammer(line, theme, assets, verdict);
            }
            line.spawn(button_small(
                theme,
                &format!("prov:colony:{}", place.id),
                "Bâtiments",
            ));
        });
        let mut detail = format!(
            "{} bâtiment(s) · fortification {} · garnison {}",
            place.buildings.len(),
            place.fortification_level,
            place.garrison_size
        );
        if let Some(work) = &place.construction {
            detail.push_str(&format!(
                " · chantier : {} ({} t)",
                work.name, work.turns_left
            ));
        }
        if place.besieged {
            detail.push_str(" · assiégée");
        }
        if place.controller != snap.sheet.controller {
            detail.push_str(&format!(" · tenue par {}", place.controller));
        }
        card.spawn(caption(theme, &detail));
        if let Some(bar) = snap.bars.iter().find(|b| b.settlement == place.id) {
            slot_bar(card, theme, assets, bar, "Emplacements");
        }
    });
}

fn settlements_tab(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    labels: &UiLabels,
    images: &BuildingImages,
    snap: &Snapshot,
) {
    pane.spawn(typed(
        theme,
        &format!("{} colonie(s)", snap.sheet.settlements.len()),
        UiType::Body,
    ));
    for place in &snap.sheet.settlements {
        settlement_card(pane, theme, assets, snap, place);
    }
    if snap.companion {
        super::companion::section(pane, theme, assets, labels, images, snap);
    }
}

fn garrison_tab(pane: &mut Kids, theme: &UiTheme, snap: &Snapshot) {
    let sheet = &snap.sheet;
    if sheet.garrison.is_empty() {
        pane.spawn(caption(theme, "Aucune troupe en garnison."));
    }
    let men: u32 = sheet.garrison.iter().map(|u| u.strength).sum();
    if !sheet.garrison.is_empty() {
        pane.spawn(typed(
            theme,
            &format!(
                "{} unité(s), {} hommes",
                sheet.garrison.len(),
                fr::digits(i64::from(men))
            ),
            UiType::Body,
        ));
    }
    for (index, unit) in sheet.garrison.iter().enumerate() {
        let tip = format!(
            "[b]{}[/b]\nEffectif {}/{}\nMoral {}\nExpérience {}",
            unit.name, unit.strength, unit.max_strength, unit.morale, unit.experience
        );
        let detail = format!(
            "{} · {}/{} · moral {}",
            unit.name, unit.strength, unit.max_strength, unit.morale
        );
        if snap.mine {
            pane.spawn((
                Tooltip::new(&tip),
                checkbox(
                    theme,
                    &format!("prov:garrison_pick:{index}"),
                    &detail,
                    snap.picked.contains(&index),
                ),
            ));
        } else {
            pane.spawn((
                Name::new(format!("prov:garrison_{index}")),
                Tooltip::new(&tip),
                typed(theme, &detail, UiType::Body),
            ));
        }
    }
    if snap.mine && !sheet.garrison.is_empty() {
        let picked = snap.picked.len();
        let over = picked as u32 > snap.army_cap;
        pane.spawn(caption(
            theme,
            &format!(
                "{picked} unité(s) choisie(s) sur {} au plus par armée.",
                snap.army_cap
            ),
        ));
        let mut button = pane.spawn((
            button_primary(theme, "prov:form_army", "Former une armée"),
            Tooltip::new("Réunit les unités cochées en une armée qui quitte la garnison."),
        ));
        if picked == 0 || over {
            button.insert(bevy::ui::InteractionDisabled);
        }
    }
}

#[cfg(test)]
mod slot_tests {
    use super::*;

    fn slot(state: &str, level: u8, max_level: u8) -> SlotView {
        SlotView {
            root: "bld_a".into(),
            built: if state == "empty" || state == "locked" {
                String::new()
            } else {
                "bld_a".into()
            },
            built_name: "Halle".into(),
            level,
            max_level,
            category: "economy".into(),
            upkeep: 2,
            state: state.into(),
            locked_reason: "pas de port".into(),
            next_building: "bld_b".into(),
            next_name: "Grande halle".into(),
            next_cost: 300,
        }
    }

    #[test]
    fn cell_texts_follow_the_godot_bar() {
        assert_eq!(slot_cell_text(&slot("built", 2, 3)), "Niv. 2");
        assert_eq!(slot_cell_text(&slot("built", 1, 1)), "Bâti");
        assert_eq!(slot_cell_text(&slot("upgradable", 1, 3)), "1/3 +");
        assert_eq!(slot_cell_text(&slot("locked", 0, 3)), "—");
        assert_eq!(slot_cell_text(&slot("empty", 0, 3)), "+");
    }

    #[test]
    fn tooltips_name_building_option_or_reason() {
        assert!(slot_tooltip(&slot("built", 2, 3)).contains("Halle"));
        assert!(slot_tooltip(&slot("empty", 0, 3)).contains("Grande halle"));
        assert!(slot_tooltip(&slot("locked", 0, 3)).contains("pas de port"));
    }
}
