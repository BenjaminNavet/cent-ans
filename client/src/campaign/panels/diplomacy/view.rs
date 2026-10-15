//! Retained rebuild of the diplomacy screen: despawn the root, spawn it again from the snapshot.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use game_api::diplomacy::{ActionView, ClauseSlot, FactionRow, RelationDetail};
use sim_campaign::negotiation::Party;

use super::format::{self, FILTERS, SORTS};
use super::{
    AmountField, DiplomacyUi, GOLD_STEPS, SEARCH_FIELD, TRIBUTE_SEASONS, TRIBUTE_STEPS,
    refresh_snapshot,
};
use crate::paths;
use crate::sim::Sim;
use crate::ui::kit::text_field::{TextField, TextInputFocus, text_field_limited};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{KitWidget, ListItem, Toggled};
use crate::ui::kit::{
    UiTheme, UiTier, UiType, button_primary, button_small, caption, fr, heading, separator, style,
    tab_bar, tab_pane, typed,
};

#[derive(Component)]
pub struct DiplomacyRoot;

type Kids<'a> = ChildSpawnerCommands<'a>;

const GROUP_CAP: usize = 40;
const LIST_WIDTH: f32 = 270.0;

#[allow(clippy::too_many_arguments)]
pub fn rebuild(
    mut commands: Commands,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    sim: Res<Sim>,
    mut ui: ResMut<DiplomacyUi>,
    roots: Query<Entity, With<DiplomacyRoot>>,
    focus: Res<TextInputFocus>,
    fields: Query<(&Name, &TextField)>,
) {
    if !ui.dirty {
        return;
    }
    ui.dirty = false;
    // The panel is rebuilt as a whole: a focused text field is recreated, so remember it.
    if let Some((name, field)) = focus.0.and_then(|entity| fields.get(entity).ok()) {
        ui.refocus = Some((name.to_string(), field.cursor));
    }
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
            Name::new("DiplomacyPanel"),
            DiplomacyRoot,
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
        .with_children(|root| {
            header(root, &theme, ui);
            court_banner(root, &theme, ui);
            offers(root, &theme, ui);
            root.spawn(separator(&theme));
            root.spawn(Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                column_gap: Val::Px(14.0),
                ..default()
            })
            .with_children(|body| {
                faction_column(body, &theme, &assets, asset_root.as_deref(), ui);
                detail_column(body, &theme, &assets, asset_root.as_deref(), ui);
            });
        });
}

/// Gives the focus back to the field that had it before the panel was rebuilt.
pub fn restore_focus(
    mut ui: ResMut<DiplomacyUi>,
    mut focus: ResMut<TextInputFocus>,
    mut fields: Query<(Entity, &Name, &mut TextField)>,
) {
    let Some((wanted, cursor)) = ui.refocus.take() else {
        return;
    };
    if let Some((entity, _, mut field)) = fields.iter_mut().find(|(_, n, _)| n.as_str() == wanted) {
        field.cursor = cursor.min(field.value.chars().count());
        focus.0 = Some(entity);
    }
}

fn ink(theme: &UiTheme, content: &str, ui_type: UiType, color: Color) -> impl Bundle {
    (
        Text::new(content),
        theme.text_font(false, false, ui_type.size()),
        TextColor(color),
    )
}

fn header(root: &mut Kids, theme: &UiTheme, ui: &DiplomacyUi) {
    root.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            children![title_small(theme, "Diplomatie")],
        ))
        .with_children(|titles| {
            if let Some(religion) = &ui.snapshot.religion {
                titles.spawn((
                    Name::new("dip:chancery"),
                    ink(
                        theme,
                        &game_api::diplomacy::chancery_text(religion),
                        UiType::Body,
                        style::INK_SOFT,
                    ),
                ));
            }
        });
        if let Some(religion) = ui.snapshot.religion.as_ref().filter(|r| r.can_donate) {
            row.spawn((
                Name::new("dip:donate_tip"),
                Tooltip::new(&format!(
                    "Augmente la faveur pontificale (+1 par {} livres).",
                    religion.livres_per_favor
                )),
                Hovered::default(),
                Node::default(),
                children![button_small(
                    theme,
                    "dip:donate",
                    &format!("Don à l’Église ({})", fr::amount(religion.donation_amount)),
                )],
            ));
        }
        if let Some(result) = &ui.result {
            row.spawn((
                Name::new("dip:result"),
                ink(
                    theme,
                    &result.text,
                    UiType::Body,
                    if result.ok { style::GOOD } else { style::ERROR },
                ),
            ));
        }
        row.spawn(button_small(theme, "dip:close", "×"));
    });
}

fn title_small(theme: &UiTheme, content: &str) -> impl Bundle {
    (
        Text::new(content),
        theme.title_font(UiType::Title.size()),
        TextColor(style::INK_TITLE),
    )
}

/// « Affaires de la Cour »: offers waiting, treaties about to expire, active league. Hidden if empty.
fn court_banner(root: &mut Kids, theme: &UiTheme, ui: &DiplomacyUi) {
    if ui.snapshot.court.is_empty() {
        return;
    }
    root.spawn((
        Name::new("dip:court"),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },
    ))
    .with_children(|section| {
        section.spawn(heading(theme, "Affaires de la Cour"));
        for line in &ui.snapshot.court {
            section.spawn(ink(theme, line, UiType::Body, style::INK));
        }
    });
}

/// « Propositions reçues »: one row per offer with accept / refuse.
fn offers(root: &mut Kids, theme: &UiTheme, ui: &DiplomacyUi) {
    if ui.snapshot.offers.is_empty() {
        return;
    }
    root.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(3.0),
        ..default()
    })
    .with_children(|section| {
        section.spawn(heading(theme, "Propositions reçues"));
        for offer in &ui.snapshot.offers {
            let (yes, no) = match offer.kind.as_str() {
                "protection" => ("Intervenir", "Se dérober"),
                "arbitration" => ("Imposer la paix", "Laisser faire"),
                "peace_summons" => ("Obéir", "Passer outre"),
                "ally_call" => ("Marcher à son secours", "Se dérober"),
                _ if offer.ultimatum => ("Céder", "Refuser (la guerre)"),
                _ => ("Accepter", "Refuser"),
            };
            section
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            ..default()
                        },
                        ink(
                            theme,
                            &format!(
                                "{} ({})",
                                offer.text,
                                fr::count(i64::from(offer.expires_in), "tour", Some("tours"))
                            ),
                            UiType::Body,
                            style::INK,
                        ),
                    ));
                    row.spawn(button_primary(
                        theme,
                        &format!("dip:offer:{}:accept", offer.id),
                        yes,
                    ));
                    // Feudal arbitration: or take the side of one of the two vassals.
                    if offer.kind == "arbitration"
                        && let Some(call) = ui.snapshot.calls.iter().find(|c| c.id == offer.id)
                    {
                        for side in [&call.attacker, &call.target].into_iter().flatten() {
                            row.spawn((
                                Hovered::default(),
                                Tooltip::new(&format!(
                                    "Prendre le parti de {} : guerre contre l’autre vassal.",
                                    side.name
                                )),
                                Node::default(),
                                children![button_small(
                                    theme,
                                    &format!("dip:arbitrate:{}:{}", offer.id, side.id),
                                    &format!("Soutenir {}", side.name),
                                )],
                            ));
                        }
                    }
                    row.spawn(button_small(
                        theme,
                        &format!("dip:offer:{}:refuse", offer.id),
                        no,
                    ));
                });
        }
    });
}

fn heraldry(
    row: &mut Kids,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    id: &str,
    color: &str,
    size: f32,
) {
    let path = format!("heraldry/{id}.png");
    let exists = asset_root.is_some_and(|r| r.join(&path).exists());
    let mut node = Node {
        width: Val::Px(size),
        height: Val::Px(size),
        flex_shrink: 0.0,
        ..default()
    };
    if exists {
        row.spawn((node, ImageNode::new(assets.load(path))));
    } else {
        node.border = UiRect::all(Val::Px(1.0));
        row.spawn((
            node,
            BackgroundColor(Srgba::hex(color).map_or(Color::srgb(0.53, 0.53, 0.53), Color::from)),
            BorderColor::all(style::INK_SOFT),
        ));
    }
}

fn faction_column(
    body: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    ui: &DiplomacyUi,
) {
    body.spawn(Node {
        width: Val::Px(LIST_WIDTH),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        min_height: Val::Px(0.0),
        ..default()
    })
    .with_children(|column| {
        column.spawn(heading(theme, "Les puissances"));
        column.spawn(button_small(
            theme,
            "dip:filter",
            &format!("Filtre : {}", FILTERS[ui.filter]),
        ));
        column.spawn(button_small(
            theme,
            "dip:sort",
            &format!("Tri : {}", SORTS[ui.sort]),
        ));
        column
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|search| {
                search.spawn(text_field_limited(
                    theme,
                    SEARCH_FIELD,
                    &ui.query,
                    "Chercher une maison…",
                    LIST_WIDTH - 36.0,
                    40,
                ));
                if !ui.query.is_empty() {
                    search.spawn(button_small(theme, "dip:clear_search", "×"));
                }
            });
        column
            .spawn((
                Name::new("dip:list"),
                KitWidget("scroll_list"),
                bevy::ui_widgets::ScrollArea,
                Node {
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    padding: theme.padding("inset"),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                theme.sliced("inset"),
            ))
            .with_children(|list| {
                for row in format::visible_rows(&ui.snapshot.rows, ui.filter, ui.sort, &ui.query) {
                    faction_row(
                        list,
                        theme,
                        assets,
                        asset_root,
                        row,
                        ui.selected.as_deref() == Some(row.id.as_str()),
                        ui.snapshot.trends.get(&row.id).copied().unwrap_or(""),
                    );
                }
            });
    });
}

fn faction_row(
    list: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    row: &FactionRow,
    selected: bool,
    trend: &str,
) {
    list.spawn((
        Name::new(format!("dip:faction:{}", row.id)),
        KitWidget("list_item"),
        ListItem {
            group: "dip_factions".to_owned(),
        },
        Toggled(selected),
        Button,
        Hovered::default(),
        Tooltip::new(&format::opinion_tooltip(row)),
        Node {
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            min_height: Val::Px(54.0),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::NONE),
    ))
    .with_children(|line| {
        heraldry(line, assets, asset_root, &row.id, &row.color, 36.0);
        line.spawn(Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|names| {
            names.spawn(ink(theme, &row.name, UiType::Body, style::INK));
            if !row.ruler.is_empty() {
                names.spawn(ink(theme, &row.ruler, UiType::Caption, style::INK_FADED));
            }
        });
        line.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            ..default()
        })
        .with_children(|right| {
            right.spawn(ink(
                theme,
                format::status_label(&row.status),
                UiType::Caption,
                format::status_color(&row.status),
            ));
            right.spawn(ink(
                theme,
                format!("{} {trend}", format::signed(row.attitude)).trim(),
                UiType::Body,
                if row.attitude >= 0 {
                    style::GOOD
                } else {
                    style::POOR
                },
            ));
        });
        attitude_gauge(line, row.attitude);
    });
}

/// Vertical attitude gauge (-100..100): green upwards, red downwards from the middle line.
fn attitude_gauge(line: &mut Kids, attitude: i32) {
    let half = 18.0;
    let fill = half * (attitude.unsigned_abs() as f32 / 100.0).clamp(0.0, 1.0);
    line.spawn((
        Node {
            width: Val::Px(8.0),
            height: Val::Px(36.0),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(style::PARCHMENT_DARK),
    ))
    .with_children(|bar| {
        let (top, color) = if attitude >= 0 {
            (half - fill, style::GOOD)
        } else {
            (half, style::POOR)
        };
        bar.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                width: Val::Px(8.0),
                top: Val::Px(top),
                height: Val::Px(fill),
                ..default()
            },
            BackgroundColor(color),
        ));
        bar.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                width: Val::Px(8.0),
                top: Val::Px(half),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(style::INK),
        ));
    });
}

fn detail_column(
    body: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    ui: &DiplomacyUi,
) {
    body.spawn(Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_width: Val::Px(380.0),
        min_height: Val::Px(0.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|column| {
        let Some(detail) = &ui.snapshot.detail else {
            column.spawn(ink(
                theme,
                "Aucune puissance à afficher.",
                UiType::Body,
                style::INK_FADED,
            ));
            return;
        };
        column
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                max_height: Val::Percent(45.0),
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .insert(bevy::ui_widgets::ScrollArea)
            .with_children(|head| head_section(head, theme, assets, asset_root, detail));
        let tabs = [
            ("dip:tab:negotiation", "Négociation"),
            ("dip:tab:war", "Guerre"),
            ("dip:tab:treaties", "Traités"),
        ];
        let selected = tabs
            .iter()
            .position(|(name, _)| *name == ui.tab)
            .unwrap_or(0);
        column.spawn(tab_bar(theme, "dip_tab", &tabs, selected));
        column.spawn(separator(theme));
        for (index, (name, _)) in tabs.iter().enumerate() {
            column
                .spawn(tab_pane("dip_tab", name, index == selected))
                .insert(Node {
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    overflow: Overflow::scroll_y(),
                    display: if index == selected {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                })
                .insert(bevy::ui_widgets::ScrollArea)
                .with_children(|pane| match index {
                    0 => negotiation_tab(pane, theme, ui, detail),
                    1 => war_tab(pane, theme, detail),
                    _ => history_tab(pane, theme, detail),
                });
        }
    });
}

fn head_section(
    head: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    detail: &RelationDetail,
) {
    let row = &detail.row;
    head.spawn(Node {
        column_gap: Val::Px(10.0),
        ..default()
    })
    .with_children(|top| {
        heraldry(top, assets, asset_root, &row.id, &row.color, 64.0);
        top.spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|names| {
            names.spawn(typed(theme, &row.name, UiType::Heading));
            names.spawn(ink(
                theme,
                if row.ruler.is_empty() {
                    "Souverain inconnu"
                } else {
                    &row.ruler
                },
                UiType::Body,
                style::INK_SOFT,
            ));
            let status = if row.status == "alliance" && row.alliance_kind == "defensive" {
                "Alliance défensive"
            } else {
                format::status_label(&row.status)
            };
            let attitude = if row.attitude_band.is_empty() {
                format!("attitude {}", format::signed(row.attitude))
            } else {
                format!("{} ({})", row.attitude_band, format::signed(row.attitude))
            };
            names.spawn((
                Name::new("dip:head_status"),
                ink(
                    theme,
                    &format!("{status} — {attitude}"),
                    UiType::Body,
                    format::status_color(&row.status),
                ),
            ));
        });
    });
    head.spawn((
        Name::new("dip:facts"),
        ink(
            theme,
            &facts_line(detail).join(" · "),
            UiType::Caption,
            style::INK_SOFT,
        ),
    ));
    // Opinion block: every reason, heaviest first, one tooltip per reason.
    if !row.reasons.is_empty() {
        head.spawn((
            Name::new("dip:opinion"),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(1.0),
                ..default()
            },
        ))
        .with_children(|box_| {
            box_.spawn((
                Name::new("dip:opinion_total"),
                ink(
                    theme,
                    &format::opinion_total(&row.attitude_band, row.attitude),
                    UiType::Caption,
                    style::INK,
                ),
            ));
            for (index, reason) in row.reasons.iter().enumerate() {
                box_.spawn((
                    Name::new(format!("dip:opinion_reason:{index}")),
                    Tooltip::new(&format::reason_tooltip(reason)),
                    ink(
                        theme,
                        &format::reason_line(reason),
                        UiType::Caption,
                        if reason.value < 0 {
                            style::RUBRIC
                        } else {
                            style::INK
                        },
                    ),
                ));
            }
        });
    }
    // Allies, enemies and vassals: names that open their sheet.
    for (label, refs) in [
        ("Alliés", &row.allies),
        ("En guerre contre", &row.enemies),
        ("Vassaux", &row.vassals),
    ] {
        if refs.is_empty() {
            continue;
        }
        head.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|line| {
            line.spawn(ink(
                theme,
                &format!("{label} :"),
                UiType::Caption,
                style::INK_SOFT,
            ));
            for link in refs.iter().take(24) {
                line.spawn(button_small(
                    theme,
                    &format!("dip:link:{}", link.id),
                    &link.name,
                ));
            }
        });
    }
}

/// Facts line of the head (religion, power, truce, treaties, passage, loyalty, embargoes).
fn facts_line(detail: &RelationDetail) -> Vec<String> {
    let row = &detail.row;
    let mut facts = vec![
        format!(
            "Religion : {}",
            if row.religion_name.is_empty() {
                "?"
            } else {
                &row.religion_name
            }
        ),
        format!("Puissance : {}", fr::digits(row.power)),
        format!("Position : {}", row.stance_label),
    ];
    let turns = |n: u32| fr::count(i64::from(n), "tour", Some("tours"));
    if row.truce_turns_left > 0 {
        facts.push(format!("Trêve : encore {}", turns(row.truce_turns_left)));
    }
    if row.non_aggression_turns_left > 0 {
        facts.push(format!(
            "Pacte de non-agression : encore {}",
            turns(row.non_aggression_turns_left)
        ));
    }
    if row.hegemon {
        facts.push("Visé par la ligue des princes".into());
    }
    if row.trade_agreement {
        let suspended = row.embargo_by_us || row.embargo_on_us;
        facts.push(format!(
            "Accord commercial{}",
            if suspended { " (suspendu)" } else { "" }
        ));
    }
    if row.access_received {
        facts.push("Accès militaire accordé".into());
    }
    if row.access_given {
        facts.push("Droit de passage donné".into());
    }
    let seasons = |n: u32| fr::count(i64::from(n), "saison", Some("saisons"));
    let t = &detail.trespass;
    if t.theirs.seasons > 0 {
        facts.push(format!(
            "Leurs armées campent sur nos terres ({})",
            seasons(t.theirs.seasons)
        ));
    }
    if t.theirs.grievance {
        facts.push("Casus belli : violation de nos frontières".into());
    }
    if t.ours.seasons > 0 {
        facts.push(format!(
            "Nos armées campent chez eux sans droit de passage ({})",
            seasons(t.ours.seasons)
        ));
    }
    if t.ours.grievance {
        facts.push("Ils tiennent un casus belli contre nous (intrusion)".into());
    }
    if let Some(loyalty) = row.loyalty {
        facts.push(format!("Loyauté {loyalty}/100"));
    }
    if row.embargo_by_us {
        facts.push("Sous notre embargo".into());
    }
    if row.embargo_on_us {
        facts.push("Nous impose un embargo".into());
    }
    facts
}

fn negotiation_tab(pane: &mut Kids, theme: &UiTheme, ui: &DiplomacyUi, detail: &RelationDetail) {
    pane.spawn(heading(theme, "Clauses communes"));
    clause_section(pane, theme, ui, detail, ClauseSlot::Common);
    pane.spawn(Node {
        column_gap: Val::Px(10.0),
        ..default()
    })
    .with_children(|columns| {
        for (slot, title_text) in [
            (ClauseSlot::Offer, "Vous offrez"),
            (ClauseSlot::Demand, "Vous demandez"),
        ] {
            columns
                .spawn((
                    Node {
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        padding: UiRect::all(Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(style::PARCHMENT_LIGHT),
                    BorderColor::all(style::GOLD),
                ))
                .with_children(|column| {
                    column.spawn(ink(theme, title_text, UiType::Heading, style::RUBRIC));
                    clause_section(column, theme, ui, detail, slot);
                });
        }
    });
    chance_block(pane, theme, ui);
    // Buttons.
    pane.spawn(Node {
        column_gap: Val::Px(6.0),
        flex_wrap: FlexWrap::Wrap,
        ..default()
    })
    .with_children(|buttons| {
        buttons.spawn(button_small(theme, "dip:ask_counter", "Que faudrait-il ?"));
        buttons.spawn(button_small(theme, "dip:clear", "Effacer"));
        buttons.spawn(button_primary(theme, "dip:send", "Proposer le traité"));
    });
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Actions unilatérales"));
    pane.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(6.0),
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|actions| {
        for action in &detail.actions {
            let mut entity = actions.spawn(button_small(
                theme,
                &format!("dip:action:{}", action.key),
                &action.label,
            ));
            if let Some(tip) = action_tooltip(action, &ui.snapshot.escalation) {
                entity.insert(Tooltip::new(&tip));
            }
        }
    });
}

/// BBCode of the consequences of a unilateral action (hover).
fn action_tooltip(
    action: &ActionView,
    escalation: &[game_api::diplomacy::EscalationView],
) -> Option<String> {
    let preview = action.preview.as_ref()?;
    let head = if action.key == "declare_war" {
        "[b]Conséquences :[/b]".to_owned()
    } else if preview.accept {
        "[b][color=#2a6a2a]Accepterait[/color][/b]".to_owned()
    } else {
        "[b][color=#9e2114]Refuserait[/color][/b]".to_owned()
    };
    let mut text = head;
    for reason in &preview.reasons {
        text.push('\n');
        if reason.value == 0 {
            text.push_str(&reason.text);
        } else {
            text.push_str(&format::reason_tooltip(&game_api::diplomacy::ReasonView {
                text: reason.text.clone(),
                value: reason.value,
                turns_left: 0,
            }));
        }
    }
    if action.key == "declare_war" {
        let chain = super::war::escalation_bbcode(escalation);
        if !chain.is_empty() {
            text.push('\n');
            text.push_str(&chain);
        }
    }
    Some(text)
}

/// Draft rows of `slot` (with « × ») then the menu button and its open entries.
fn clause_section(
    parent: &mut Kids,
    theme: &UiTheme,
    ui: &DiplomacyUi,
    detail: &RelationDetail,
    slot: ClauseSlot,
) {
    let verdict = ui.snapshot.verdict.as_ref();
    let giver_of = |a: &sim_campaign::negotiation::Article| -> Option<ClauseSlot> {
        use sim_campaign::negotiation::Article as A;
        match a {
            A::MilitaryAccess { giver }
            | A::Tribute { giver, .. }
            | A::Gold { giver, .. }
            | A::CedeProvince { giver, .. }
            | A::CedeSettlement { giver, .. }
            | A::Vassalage { giver }
            | A::ReleaseCaptive { giver, .. }
            | A::Hostage { giver, .. }
            | A::DemandTitle { giver, .. }
            | A::JoinWar { giver, .. } => Some(if *giver == Party::Proposer {
                ClauseSlot::Offer
            } else {
                ClauseSlot::Demand
            }),
            _ => None,
        }
    };
    let mut any = false;
    for (index, article) in ui.draft.iter().enumerate() {
        if giver_of(article).unwrap_or(ClauseSlot::Common) != slot {
            continue;
        }
        any = true;
        let label = verdict
            .and_then(|v| v.clause_labels.get(index).cloned())
            .unwrap_or_else(|| article.key().replace('_', " "));
        let value = verdict.and_then(|v| v.clause_values.get(index).copied());
        let blocked = "";
        let tip = verdict.map_or_else(
            || "Aucun poids particulier.".to_owned(),
            |v| format::clause_tooltip(&v.lines, index, blocked),
        );
        parent
            .spawn((
                Name::new(format!("dip:clause_row:{index}")),
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Node {
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        ..default()
                    },
                    Tooltip::new(&tip),
                    ink(theme, &label, UiType::Body, style::INK),
                ));
                if let Some(value) = value {
                    row.spawn(ink(
                        theme,
                        &format::signed(value),
                        UiType::Body,
                        if value >= 0 { style::GOOD } else { style::POOR },
                    ));
                }
                row.spawn(button_small(theme, &format!("dip:remove:{index}"), "×"));
            });
    }
    if !any {
        parent.spawn(ink(
            theme,
            if slot == ClauseSlot::Common {
                "Aucune clause commune."
            } else {
                "—"
            },
            UiType::Caption,
            style::INK_FADED,
        ));
    }
    let (menu, caption_text) = match slot {
        ClauseSlot::Common => ("common", "+ Clause"),
        ClauseSlot::Offer => ("offer", "+ Ajouter"),
        ClauseSlot::Demand => ("demand", "+ Ajouter"),
    };
    let open = ui.open_menu == Some(slot);
    parent.spawn(button_small(
        theme,
        &format!("dip:menu:{menu}"),
        &format!("{caption_text} {}", if open { "▾" } else { "▸" }),
    ));
    if !open {
        return;
    }
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            align_items: AlignItems::FlexStart,
            padding: UiRect::left(Val::Px(8.0)),
            ..default()
        })
        .with_children(|menu_box| {
            // Preset amounts of gold and tribute (the Godot spin boxes).
            if slot != ClauseSlot::Common {
                let side = if slot == ClauseSlot::Offer {
                    "offer"
                } else {
                    "demand"
                };
                menu_box
                    .spawn(Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(3.0),
                        row_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|amounts| {
                        for tribute in [false, true] {
                            let field = AmountField {
                                tribute,
                                offer: slot == ClauseSlot::Offer,
                            };
                            let kind = if tribute { "tribute" } else { "gold" };
                            let typed = if ui.field == Some(field) {
                                ui.field_text.as_str()
                            } else {
                                ""
                            };
                            amounts.spawn(text_field_limited(
                                theme,
                                &field.field_name(),
                                typed,
                                if tribute { "Tribut/saison" } else { "Or" },
                                120.0,
                                7,
                            ));
                            amounts.spawn(button_small(
                                theme,
                                &format!("dip:addamt:{kind}:{side}"),
                                "Ajouter",
                            ));
                        }
                        for amount in GOLD_STEPS {
                            amounts.spawn(button_small(
                                theme,
                                &format!("dip:gold:{side}:{amount}"),
                                &format!("Or {}", fr::amount(amount)),
                            ));
                        }
                        for amount in TRIBUTE_STEPS {
                            amounts.spawn(button_small(
                                theme,
                                &format!("dip:tribute:{side}:{amount}"),
                                &format!(
                                    "Tribut {}/saison × {TRIBUTE_SEASONS}",
                                    fr::amount(amount)
                                ),
                            ));
                        }
                    });
            }
            let mut groups: Vec<&str> = Vec::new();
            for (choice, name) in detail
                .options
                .clauses
                .iter()
                .zip(&ui.snapshot.clause_names)
                .filter(|(c, _)| c.slot == slot)
            {
                if choice.group.is_empty() {
                    menu_box.spawn(button_small(theme, name, &choice.label));
                } else if !groups.contains(&choice.group.as_str()) {
                    groups.push(&choice.group);
                }
            }
            for group in groups {
                let opened = ui.open_group.as_deref() == Some(group);
                menu_box.spawn(button_small(
                    theme,
                    &format!("dip:group:{group}"),
                    &format!("{group} {}", if opened { "▾" } else { "▸" }),
                ));
                if opened {
                    for (choice, name) in detail
                        .options
                        .clauses
                        .iter()
                        .zip(&ui.snapshot.clause_names)
                        .filter(|(c, _)| c.slot == slot && c.group == group)
                        .take(GROUP_CAP)
                    {
                        menu_box
                            .spawn(Node {
                                padding: UiRect::left(Val::Px(10.0)),
                                ..default()
                            })
                            .with_child(button_small(theme, name, &choice.label));
                    }
                }
            }
        });
}

/// Acceptance bar: the threshold (score 0) in the middle, fill to the right (accept) or left.
fn chance_block(pane: &mut Kids, theme: &UiTheme, ui: &DiplomacyUi) {
    pane.spawn(separator(theme));
    let Some(verdict) = ui.snapshot.verdict.as_ref() else {
        pane.spawn((
            Name::new("dip:chance"),
            ink(theme, "Ajoutez des clauses", UiType::Heading, style::INK),
        ));
        return;
    };
    let blocked = !verdict.blocked.is_empty();
    pane.spawn((
        Name::new("dip:chance"),
        ink(
            theme,
            &format!(
                "{} ({})",
                if verdict.accept {
                    "Accepterait"
                } else {
                    "Refuserait"
                },
                format::signed(verdict.score)
            ),
            UiType::Heading,
            if verdict.accept {
                style::GOOD
            } else {
                style::RUBRIC
            },
        ),
    ));
    acceptance_gauge(pane, verdict.score, blocked);
    pane.spawn((
        Name::new("dip:margin"),
        ink(
            theme,
            &format::threshold_words(verdict.score, blocked),
            UiType::Caption,
            style::INK_SOFT,
        ),
    ));
    pane.spawn((
        Name::new("dip:summary"),
        ink(
            theme,
            &verdict.summary,
            UiType::Body,
            if verdict.accept {
                style::GOOD
            } else {
                style::RUBRIC
            },
        ),
    ));
    if blocked {
        pane.spawn(ink(
            theme,
            &format!("Impossible : {}", verdict.blocked),
            UiType::Body,
            style::RUBRIC,
        ));
    }
    let blocker = verdict.blocker.as_ref().map(|b| b.text.as_str());
    let mut shown = 0;
    for line in &verdict.lines {
        if shown >= 14 && verdict.lines.len() > 15 {
            pane.spawn(ink(
                theme,
                &format!(
                    "… {} autres raisons de moindre poids",
                    verdict.lines.len() - shown
                ),
                UiType::Caption,
                style::INK_FADED,
            ));
            break;
        }
        shown += 1;
        let is_blocker =
            blocker.is_some_and(|b| line.text == b || line.text.starts_with(&format!("{b} — ")));
        pane.spawn((
            Name::new(format!("dip:reason:{shown}")),
            ink(
                theme,
                &format!(
                    "{}  {}{}",
                    format::signed(line.value),
                    line.text,
                    if is_blocker { "  ◄" } else { "" }
                ),
                UiType::Caption,
                if line.value >= 0 {
                    style::GOOD
                } else {
                    style::RUBRIC
                },
            ),
        ));
    }
    if !verdict.accept && !verdict.counter.is_empty() {
        pane.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|box_| {
            box_.spawn((
                Name::new("dip:counter_text"),
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    ..default()
                },
                ink(
                    theme,
                    &format!(
                        "Leur contre-offre (ils accepteraient) : {}.",
                        verdict.counter_text
                    ),
                    UiType::Body,
                    style::INK,
                ),
            ));
            box_.spawn(button_small(
                theme,
                "dip:adopt_counter",
                "Reprendre leur contre-offre",
            ));
        });
    }
}

fn acceptance_gauge(parent: &mut Kids, score: i32, blocked: bool) {
    let width = 260.0;
    let mid = width / 2.0;
    let span = score.abs().max(40) as f32;
    let fill = if blocked || score == 0 {
        0.0
    } else {
        mid * (score.abs() as f32 / span).clamp(0.0, 1.0)
    };
    parent
        .spawn((
            Name::new("dip:gauge"),
            Node {
                width: Val::Px(width),
                height: Val::Px(16.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(style::PARCHMENT_DARK),
            BorderColor::all(style::INK_SOFT),
        ))
        .with_children(|bar| {
            let (left, color) = if score > 0 {
                (mid, style::GOOD)
            } else {
                (mid - fill, style::RUBRIC)
            };
            bar.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(left),
                    top: Val::Px(2.0),
                    width: Val::Px(fill),
                    height: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(color),
            ));
            bar.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(mid),
                    top: Val::Px(-1.0),
                    width: Val::Px(2.0),
                    height: Val::Px(16.0),
                    ..default()
                },
                BackgroundColor(style::INK),
            ));
        });
}

fn war_tab(pane: &mut Kids, theme: &UiTheme, detail: &RelationDetail) {
    let row = &detail.row;
    if !row.casus_belli.is_empty() {
        pane.spawn(ink(
            theme,
            &format!("Casus belli : {}", row.casus_belli),
            UiType::Body,
            style::INK,
        ));
    }
    for claim in &row.claims {
        pane.spawn(ink(
            theme,
            &format!("Prétention : {claim}"),
            UiType::Body,
            style::INK_SOFT,
        ));
    }
    let Some(war) = &detail.war else {
        pane.spawn(ink(
            theme,
            "Pas de guerre en cours avec cette faction.",
            UiType::Body,
            style::INK_FADED,
        ));
        return;
    };
    pane.spawn((
        Name::new("dip:war_score"),
        heading(
            theme,
            &format!("Score de guerre : {}", format::signed(war.war_score)),
        ),
    ));
    war_bar(pane, war.war_score);
    pane.spawn(caption(
        theme,
        "Batailles, sièges et provinces occupées remplissent le score ; les buts de guerre tenus comptent double. Plus il est haut, plus l’ennemi cédera de terres. À la paix, seules les places cédées par le traité deviennent vôtres ; les autres places occupées sont rendues.",
    ));
    pane.spawn(ink(
        theme,
        &format!(
            "Fatigue de guerre : nous {}/100, eux {}/100",
            war.weariness_ours, war.weariness_theirs
        ),
        UiType::Body,
        style::INK,
    ));
    for (title_text, goals) in [
        ("Nos buts de guerre", &war.goals_ours),
        ("Leurs buts de guerre", &war.goals_theirs),
    ] {
        pane.spawn(ink(theme, title_text, UiType::Heading, style::RUBRIC));
        if goals.is_empty() {
            pane.spawn(ink(theme, "—", UiType::Body, style::INK_FADED));
        }
        for goal in goals {
            pane.spawn(ink(
                theme,
                &format!(
                    "{} {}{}",
                    if goal.held { "★" } else { "☆" },
                    goal.name,
                    if goal.held { " (tenue)" } else { "" }
                ),
                UiType::Body,
                style::INK,
            ));
        }
    }
}

/// War score bar -100..100.
fn war_bar(parent: &mut Kids, score: i32) {
    let width = 260.0;
    let mid = width / 2.0;
    let fill = mid * (score.abs() as f32 / 100.0).clamp(0.0, 1.0);
    parent
        .spawn((
            Node {
                width: Val::Px(width),
                height: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(style::PARCHMENT_DARK),
        ))
        .with_child((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(if score >= 0 { mid } else { mid - fill }),
                width: Val::Px(fill),
                height: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(if score >= 0 { style::GOOD } else { style::POOR }),
        ));
}

fn history_tab(pane: &mut Kids, theme: &UiTheme, detail: &RelationDetail) {
    if detail.history.is_empty() {
        pane.spawn(ink(
            theme,
            "Aucun traité avec cette faction.",
            UiType::Body,
            style::INK_FADED,
        ));
        return;
    }
    for record in &detail.history {
        let outcome = match record.rupture.as_str() {
            "broken" => "pacte rompu",
            "perjury" => "PARJURE (trêve rompue)",
            "refused_call" => "appel aux armes refusé",
            _ if record.accepted => "signé",
            _ => "refusé",
        };
        pane.spawn(ink(
            theme,
            &format!("{} — {}, {outcome}", record.date, detail.row.name),
            UiType::Body,
            if record.accepted {
                style::INK
            } else {
                style::RUBRIC
            },
        ));
        pane.spawn(ink(theme, &record.text, UiType::Caption, style::INK_SOFT));
    }
}
