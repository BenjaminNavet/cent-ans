//! Widget trees of the army panel and the recruitment screen. Pure builders: every call spawns a
//! fresh tree from views of `game-api`; the plugin rebuilds on change (no retained widget state
//! besides `PanelState`).

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use game_api::ArmySheet;
use game_api::recruit::{MercenaryView, RecruitCard, RecruitView, RegimentExtra, StanceOption};

use super::basket::{self, Basket};
use super::detail::RegimentCard;
use super::{ArmyPanelRoot, PanelState, RecruitPanelRoot};
use crate::app::AppState;
use crate::paths;
use crate::ui::kit::close_button;
use crate::ui::kit::panel_stack::{PanelTier, StackedPanel};
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::{UiTheme, fr, widgets::*};

pub const CARD_WIDTH: f32 = 150.0;
pub const REGIMENT_WIDTH: f32 = 98.0;
const ICON_SIZE: f32 = 48.0;

/// Resolves a unit's image: painted miniature, else the category glyph; checks the disk once.
#[derive(Resource, Default)]
pub struct IconCache {
    root: Option<PathBuf>,
    known: HashMap<String, bool>,
}

impl IconCache {
    fn exists(&mut self, relative: &str) -> bool {
        if self.root.is_none() {
            self.root = paths::find_root().ok().map(|r| paths::asset_root(&r));
        }
        let Some(root) = self.root.clone() else {
            return false;
        };
        *self
            .known
            .entry(relative.to_owned())
            .or_insert_with(|| root.join(relative).is_file())
    }

    /// Asset path (relative to `game/assets`) of the image for `unit_type`.
    pub fn unit_icon(&mut self, unit_type: &str, category: &str) -> String {
        let own = format!("icons/entity/{unit_type}.png");
        if self.exists(&own) {
            return own;
        }
        let glyph = match category {
            "cavalry" => "unit_category_cavalry",
            "ranged" => "unit_category_ranged",
            "siege" => "unit_category_siege",
            _ => "unit_category_infantry",
        };
        format!("icons/entity/{glyph}.png")
    }
}

use crate::ui::kit::contrast::{AccessLabel, level_symbol};

fn text(theme: &UiTheme, content: &str, variation: UiType, color: Color) -> impl Bundle {
    (
        Text::new(content),
        theme.text_font(false, false, variation.size()),
        TextColor(color),
    )
}

fn card_node(width: f32, selected: bool) -> impl Bundle {
    (
        Node {
            width: Val::Px(width),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            padding: UiRect::all(Val::Px(4.0)),
            border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
            ..default()
        },
        BackgroundColor(if selected {
            style::GOLD_PALE
        } else {
            style::PARCHMENT_LIGHT
        }),
        BorderColor::all(if selected {
            style::GOLD
        } else {
            style::INK_SOFT
        }),
        Button,
        Hovered::default(),
    )
}

fn icon_node(assets: &AssetServer, path: &str, size: f32, dimmed: bool) -> impl Bundle {
    let mut image = ImageNode::new(assets.load(path.to_owned()));
    if dimmed {
        image.color = Color::srgba(1.0, 1.0, 1.0, 0.55);
    }
    (
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            ..default()
        },
        image,
    )
}

/// Chevrons of an experience level (`▲` per level).
pub fn chevrons(level: u8) -> String {
    "▲".repeat(usize::from(level))
}

fn stance_label(key: &str) -> &'static str {
    match key {
        "normal" => "Marche",
        "raid" => "Chevauchée",
        "siege" => "Siège",
        "ambush" => "Embuscade",
        "forced_march" => "Marche forcée",
        "entrenched" => "Camp retranché",
        _ => "Posture",
    }
}

fn stance_help(key: &str) -> &'static str {
    match key {
        "normal" => "Marche et combat ; ne met pas le siège.",
        "raid" => "Chevauchée : dévaste les provinces ennemies pour le butin.",
        "siege" => "Met le siège devant la colonie ennemie où se tient l'armée.",
        "ambush" => "Embusquée : surgit sur une armée ennemie qui entre dans sa zone.",
        "forced_march" => "Marche forcée : plus de mouvement ce tour, troupes fatiguées.",
        "entrenched" => "Camp retranché : meilleure défense, moins de ravitaillement.",
        _ => "",
    }
}

fn root_node(kind: &'static str) -> impl Bundle {
    (
        Name::new(kind),
        DespawnOnExit(AppState::Campaign),
        UiTier::Panel.index(),
    )
}

/// Information the army panel shows (all from `game-api`).
pub struct ArmyPanelData<'a> {
    pub sheet: &'a ArmySheet,
    /// Display name of `sheet.location` (never the raw id).
    pub location_name: Option<String>,
    pub extras: &'a [RegimentExtra],
    /// Per regiment: (hover bubble, pinned detailed bubble), roster order.
    pub bubbles: &'a [(String, String)],
    pub upkeep: u32,
    pub stances: &'a [StanceOption],
    pub mercenaries: Option<&'a MercenaryView>,
}

/// Docked army panel (bottom centre): header, stance row, regiment cards, actions, mercenaries.
pub fn spawn_army_panel(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    icons: &mut IconCache,
    data: &ArmyPanelData,
    state: &PanelState,
) -> Entity {
    let sheet = data.sheet;
    let own = sheet.recruit_blocker.is_some();
    let wrapper = commands
        .spawn((
            root_node("ArmyPanelRoot"),
            ArmyPanelRoot,
            StackedPanel::new(PanelTier::Docked, "army"),
            crate::ui::kit::layout::FitZone::Bottom,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    let panel = commands
        .spawn((Name::new("army:panel"), panel_page(theme)))
        .insert(Node {
            max_width: Val::Px(940.0),
            padding: theme.padding("panel"),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .id();
    commands.entity(wrapper).add_child(panel);
    commands.entity(panel).with_children(|p| {
        // Header line.
        p.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .with_children(|h| {
            h.spawn((Name::new("army:title"), heading(theme, &sheet.name)));
            if !own {
                h.spawn((
                    Name::new("army:read_only"),
                    text(
                        theme,
                        &format!("Armée étrangère ({}) · lecture seule", sheet.faction_name),
                        UiType::Caption,
                        style::ERROR,
                    ),
                ));
            }
            let general = if sheet.general_name.is_empty() {
                "sans chef".to_owned()
            } else {
                format!("chef : {}", sheet.general_name)
            };
            h.spawn(text(theme, &general, UiType::Caption, style::INK_SOFT));
            let place = data
                .location_name
                .as_deref()
                .map_or_else(|| "en campagne".to_owned(), |name| format!("à {name}"));
            h.spawn(text(theme, &place, UiType::Caption, style::INK_SOFT));
            h.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            h.spawn(close_button(theme, "army:close"));
        });
        // Figures: units, men, upkeep, movement, supply.
        let cap = sheet.units.len();
        let men = format!(
            "{} · {} / {} hommes · entretien {} / saison · mouvement {} pts ({:.0} km) · vivres {}",
            fr::count(cap as i64, "régiment", None),
            fr::digits(i64::from(sheet.strength)),
            fr::digits(i64::from(sheet.max_strength)),
            fr::amount(i64::from(data.upkeep)),
            sheet.movement_left,
            sheet.movement_km,
            sheet.supply,
        );
        p.spawn((Name::new("army:figures"), text(theme, &men, UiType::Caption, style::INK)));
        // Stances.
        if own {
            p.spawn(Node {
                column_gap: Val::Px(4.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                row.spawn(text(theme, "Posture :", UiType::Caption, style::INK_SOFT));
                for option in data.stances {
                    let on = option.key == sheet.stance;
                    let name = format!("army:stance:{}", option.key);
                    let mut entity = row.spawn((
                        toggle_button(theme, &name, stance_label(&option.key), on),
                        Tooltip::new(&if option.refusal.is_empty() {
                            format!("[b]{}[/b]\n{}", stance_label(&option.key), stance_help(&option.key))
                        } else {
                            format!(
                                "[b]{}[/b]\n[color=#8b1a1a]{}[/color]",
                                stance_label(&option.key),
                                option.refusal
                            )
                        }),
                    ));
                    if !option.refusal.is_empty() && !on {
                        entity.insert(bevy::ui::InteractionDisabled);
                    }
                }
            });
        }
        // Regiment cards.
        p.spawn((
            Name::new("army:regiments"),
            Node {
                column_gap: Val::Px(4.0),
                row_gap: Val::Px(4.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
        ))
        .with_children(|row| {
            for (index, unit) in sheet.units.iter().enumerate() {
                let extra = data.extras.get(index);
                let category = extra.map_or("", |e| e.category.as_str());
                let selected = state.selection.contains(&index);
                let ratio = if unit.max_strength == 0 {
                    0.0
                } else {
                    unit.strength as f32 / unit.max_strength as f32
                };
                let (short, detailed) = data.bubbles.get(index).cloned().unwrap_or_default();
                row.spawn((
                    Name::new(format!("army:unit:{index}")),
                    card_node(REGIMENT_WIDTH, selected),
                    Tooltip::new(&short),
                    RegimentCard { detailed },
                ))
                .with_children(|c| {
                    c.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|top| {
                        top.spawn(icon_node(
                            assets,
                            &icons.unit_icon(&unit.unit_type, category),
                            ICON_SIZE,
                            false,
                        ));
                        top.spawn(text(
                            theme,
                            &chevrons(unit.experience.min(10)),
                            UiType::Caption,
                            style::GOLD,
                        ));
                    });
                    c.spawn(text(theme, &unit.name, UiType::Caption, style::INK));
                    c.spawn(gauge(
                        theme,
                        &format!("army:unit_strength:{index}"),
                        ratio,
                        REGIMENT_WIDTH - 14.0,
                    ));
                    c.spawn(text(
                        theme,
                        &format!("{} / {}", unit.strength, unit.max_strength),
                        UiType::Caption,
                        style::INK_SOFT,
                    ));
                    let morale = f32::from(unit.morale) / 100.0;
                    let moral = format!("moral {}", unit.morale);
                    c.spawn((
                        text(theme, &moral, UiType::Caption, style::gauge_color(morale.min(1.0))),
                        AccessLabel::new(level_symbol(morale.clamp(0.0, 1.0)), moral),
                    ));
                });
            }
        });
        // Actions.
        p.spawn(Node {
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            if !own {
                return;
            }
            let chosen = state.selection.len();
            let mut add = |name: &str, caption: &str, enabled: bool, tip: &str| {
                let mut entity = row.spawn((button_small(theme, name, caption), Tooltip::new(tip)));
                if !enabled {
                    entity.insert(bevy::ui::InteractionDisabled);
                }
            };
            add(
                "army:split",
                "Séparer",
                chosen > 0 && chosen < cap,
                "Les régiments choisis forment une nouvelle armée.",
            );
            add(
                "army:garrison",
                "Garnison",
                chosen > 0 && sheet.settlement.is_some(),
                "Les régiments choisis rejoignent la garnison de la colonie.",
            );
            add(
                "army:disband",
                "Licencier",
                chosen > 0,
                "Congédie les régiments choisis (sans remboursement).",
            );
            let blocker = sheet.recruit_blocker.clone().unwrap_or_default();
            add(
                "army:recruit",
                "Recruter",
                blocker.is_empty(),
                &if blocker.is_empty() {
                    "Lever des régiments depuis les places de la province : ils rejoindront cette armée.".to_owned()
                } else {
                    blocker
                },
            );
            add(
                "army:mercs",
                "Mercenaires",
                true,
                "Compagnies engageables dans la région de l'armée.",
            );
            for target in &sheet.merge_targets {
                let caption = format!("Fusionner : {} ({})", target.name, target.units);
                add(
                    &format!("army:merge:{}", target.id),
                    &caption,
                    true,
                    "L'armée se fond dans celle-ci.",
                );
            }
        });
        if !state.error.is_empty() {
            p.spawn((
                Name::new("army:error"),
                text(theme, &state.error, UiType::Caption, style::ERROR),
            ));
        }
        if state.mercs
            && let Some(market) = data.mercenaries
        {
            spawn_mercenaries(p, theme, assets, icons, market);
        }
    });
    wrapper
}

/// Mercenary section of the army panel (`MercenaryPanel`).
fn spawn_mercenaries(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    icons: &mut IconCache,
    market: &MercenaryView,
) {
    parent.spawn(separator(theme));
    let title = if market.region_name.is_empty() {
        "Mercenaires".to_owned()
    } else {
        format!("Mercenaires — {}", market.region_name)
    };
    parent.spawn((Name::new("merc:title"), heading(theme, &title)));
    let status = if !market.blocked.is_empty() {
        format!("Aucun engagement : {}.", market.blocked)
    } else {
        let mut s = format!(
            "{} encore ce tour.",
            fr::count(
                i64::from(market.hires_left),
                "engagement possible",
                Some("engagements possibles")
            )
        );
        if market.premium_last_turn > 0 {
            s += &format!(
                " Surcoût de solde la saison passée : {}.",
                fr::amount(market.premium_last_turn)
            );
        }
        s
    };
    parent.spawn((
        Name::new("merc:status"),
        text(
            theme,
            &status,
            UiType::Caption,
            if market.blocked.is_empty() {
                style::INK_SOFT
            } else {
                style::RUBRIC
            },
        ),
    ));
    parent
        .spawn(Node {
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .with_children(|grid| {
            if market.options.is_empty() {
                grid.spawn(text(
                    theme,
                    "Aucune compagnie ne se loue dans cette région en ce moment.",
                    UiType::Caption,
                    style::INK_SOFT,
                ));
            }
            let mut ordered: Vec<_> = market.options.iter().collect();
            ordered.sort_by_key(|o| !o.available);
            for option in ordered {
                let name =
                    if !option.band_name.is_empty() && !option.name.contains(&option.band_name) {
                        format!("{} ({})", option.name, option.band_name)
                    } else {
                        option.name.clone()
                    };
                let card = RecruitCard {
                    unit_type: option.unit_type.clone(),
                    name,
                    cost: option.cost,
                    upkeep: option.upkeep,
                    available: option.available,
                    reason: option.reason.clone(),
                    soldiers: option.soldiers,
                    recruit_time_turns: 1,
                    category: option.category.clone(),
                    group: String::new(),
                    import_cost: 0,
                    pool_available: option.pool_available,
                    pool_cap: option.pool_cap,
                    pool_seasons_to_next: option.pool_seasons_to_next,
                    pool_label: option.pool_label.clone(),
                    requirements: vec![],
                };
                spawn_offer_card(
                    grid,
                    theme,
                    assets,
                    icons,
                    &format!("merc:hire:{}", option.unit_type),
                    &card,
                    0,
                    false,
                    None,
                );
            }
        });
    parent.spawn(text(
        theme,
        "Engagées sur-le-champ, les compagnies coûtent cher et touchent une solde majorée ; impayées, elles désertent ou pillent la province.",
        UiType::Caption,
        style::INK_SOFT,
    ));
}

/// One offer card (recruitment and mercenaries): icon, name, men · delay, price / upkeep,
/// reserve, refusal. `count` > 0 shows `× n`; `treasury_blocked` greys it with the treasury line.
#[allow(clippy::too_many_arguments)]
fn spawn_offer_card(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    icons: &mut IconCache,
    name: &str,
    card: &RecruitCard,
    count: u32,
    treasury_blocked: bool,
    tooltip: Option<String>,
) {
    let dimmed = !card.available || treasury_blocked;
    let mut entity = parent.spawn((Name::new(name.to_owned()), card_node(CARD_WIDTH, count > 0)));
    if let Some(tip) = tooltip {
        entity.insert(Tooltip::new(&tip));
    }
    if dimmed {
        entity.insert(bevy::ui::InteractionDisabled);
    }
    entity.with_children(|c| {
        c.spawn(Node {
            column_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|head| {
            head.spawn(icon_node(
                assets,
                &icons.unit_icon(&card.unit_type, &card.category),
                ICON_SIZE,
                dimmed,
            ));
            head.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 1.0,
                ..default()
            })
            .with_children(|titles| {
                titles.spawn(text(theme, &card.name, UiType::Caption, style::INK));
                titles.spawn(text(
                    theme,
                    &basket::legend_text(card),
                    UiType::Caption,
                    style::INK_SOFT,
                ));
            });
        });
        c.spawn(text(
            theme,
            &basket::price_text(card.cost, card.upkeep, card.import_cost),
            UiType::Caption,
            style::INK,
        ));
        if card.pool_cap > 0 {
            c.spawn(text(
                theme,
                &basket::pool_drops(card.pool_available, card.pool_cap),
                UiType::Body,
                style::WAX,
            ));
        }
        if !card.pool_label.is_empty() {
            c.spawn(text(
                theme,
                &card.pool_label,
                UiType::Caption,
                if card.pool_available > 0 {
                    style::INK_SOFT
                } else {
                    style::RUBRIC
                },
            ));
        }
        if !card.available && !card.reason.is_empty() && !card.reason.starts_with("réserve") {
            c.spawn(text(
                theme,
                &format!("✗ {}", card.reason),
                UiType::Caption,
                style::RUBRIC,
            ));
        }
        if treasury_blocked {
            c.spawn(text(
                theme,
                "✗ Trésor insuffisant",
                UiType::Caption,
                style::RUBRIC,
            ));
        }
        if count > 0 {
            c.spawn((
                Name::new(format!("{name}:count")),
                text(theme, &format!("× {count}"), UiType::Heading, style::WAX),
            ));
        }
    });
}

/// BBCode tooltip of a recruitment card (requirements with ✓ / ✗).
fn offer_tooltip(card: &RecruitCard) -> String {
    let mut lines = vec![
        format!("[b]{}[/b]", card.name),
        basket::legend_text(card),
        basket::price_text(card.cost, card.upkeep, card.import_cost),
    ];
    for req in &card.requirements {
        lines.push(if req.met {
            format!("[color=#2a6a2a]✓[/color] {}", req.id)
        } else {
            format!("[color=#8b1a1a]✗[/color] {}", req.id)
        });
    }
    if !card.available && !card.reason.is_empty() {
        lines.push(format!("[color=#8b1a1a]{}[/color]", card.reason));
    }
    lines.join("\n")
}

/// Height of the « Bientôt » list (three lines) and smallest height of the card grid.
const SOON_HEIGHT: f32 = 52.0;
const CARDS_MIN_HEIGHT: f32 = 120.0;

/// Recruitment screen, docked at the right.
pub fn spawn_recruit_panel(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    icons: &mut IconCache,
    view: &RecruitView,
    state: &PanelState,
) -> Entity {
    let wrapper = commands
        .spawn((
            root_node("RecruitPanelRoot"),
            RecruitPanelRoot,
            StackedPanel::new(PanelTier::Docked, "recruitment"),
            crate::ui::kit::layout::FitZone::Side,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();
    let panel = commands
        .spawn((Name::new("rec:panel"), panel_page(theme)))
        .insert(Node {
            width: Val::Px(620.0),
            max_height: Val::Percent(88.0),
            padding: theme.padding("panel"),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .id();
    commands.entity(wrapper).add_child(panel);
    let basket: &Basket = &state.basket;
    commands.entity(panel).with_children(|p| {
        p.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|h| {
            h.spawn((
                Name::new("rec:title"),
                heading(theme, &format!("Recrutement — {}", view.settlement_name)),
            ));
            h.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            h.spawn(close_button(theme, "rec:close"));
        });
        // Queue and slots.
        let queue_line = format!(
            "Recrues attendues : {}({} ce tour sur {}) · trésor {}",
            if view.queue.is_empty() { "aucune " } else { "" },
            fr::count(view.slots_free as i64, "place libre", Some("places libres")),
            view.slots,
            fr::amount(view.treasury),
        );
        p.spawn((
            Name::new("rec:queue_label"),
            text(theme, &queue_line, UiType::Caption, style::INK),
        ));
        if !view.queue.is_empty() {
            p.spawn(Node {
                column_gap: Val::Px(4.0),
                row_gap: Val::Px(4.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                for (index, queued) in view.queue.iter().enumerate() {
                    row.spawn((
                        Name::new(format!("rec:queued:{index}")),
                        Node {
                            padding: UiRect::all(Val::Px(4.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            column_gap: Val::Px(6.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(style::PARCHMENT_LIGHT),
                        BorderColor::all(style::INK_SOFT),
                    ))
                    .with_children(|c| {
                        let line = format!(
                            "{} · {} · rembours. {}",
                            queued.name,
                            fr::count(i64::from(queued.turns_left), "tour", None),
                            fr::amount(queued.refund)
                        );
                        c.spawn(text(theme, &line, UiType::Caption, style::INK));
                        c.spawn(button_small(
                            theme,
                            &format!("rec:cancel:{index}"),
                            "Annuler",
                        ));
                    });
                }
            });
        }
        if !view.own {
            p.spawn(text(
                theme,
                "Colonie hors de votre contrôle.",
                UiType::Caption,
                style::INK_SOFT,
            ));
            return;
        }
        // Destination.
        if !view.armies.is_empty() {
            p.spawn(Node {
                column_gap: Val::Px(4.0),
                row_gap: Val::Px(4.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                row.spawn(text(
                    theme,
                    "Destination :",
                    UiType::Caption,
                    style::INK_SOFT,
                ));
                row.spawn(toggle_button(
                    theme,
                    "rec:target:garrison",
                    "Garnison",
                    state.target.is_none(),
                ));
                for army in &view.armies {
                    let on = state.target.as_ref().map(ToString::to_string).as_deref()
                        == Some(army.id.as_str());
                    let caption = format!(
                        "{} ({} unités){}",
                        army.name,
                        army.units,
                        if army.field { " · en campagne" } else { "" }
                    );
                    row.spawn(toggle_button(
                        theme,
                        &format!("rec:target:{}", army.id),
                        &caption,
                        on,
                    ));
                }
            });
        }
        // Category tabs and sort.
        let present = basket::categories_present(&view.options);
        p.spawn(Node {
            column_gap: Val::Px(4.0),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            if present.len() > 1 {
                row.spawn(toggle_button(
                    theme,
                    "rec:tab:all",
                    "Tous",
                    state.category.is_empty(),
                ));
                for (key, label) in basket::CATEGORIES {
                    if present.contains(&key) {
                        row.spawn(toggle_button(
                            theme,
                            &format!("rec:tab:{key}"),
                            label,
                            state.category == key,
                        ));
                    }
                }
            }
            row.spawn(button_small(theme, "rec:sort", state.sort.label()));
        });
        // Cards (scrolling).
        p.spawn(scroll_list(theme, "rec:cards", 330.0))
            .insert(Node {
                height: Val::Px(330.0),
                // Shrinks first when the panel is short, but never below two card rows.
                min_height: Val::Px(CARDS_MIN_HEIGHT),
                flex_shrink: 1.0,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::FlexStart,
                column_gap: Val::Px(4.0),
                row_gap: Val::Px(4.0),
                padding: {
                    let mut padding = theme.inner_padding("inset", 4.0);
                    padding.right = Val::Px(crate::ui::kit::widgets::SCROLLBAR_WIDTH + 8.0);
                    padding
                },
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .with_children(|grid| {
                let shown = basket::visible_rows(
                    &view.options,
                    &["ready", "blocked"],
                    &state.category,
                    state.sort,
                );
                if shown.is_empty() {
                    grid.spawn(text(
                        theme,
                        "Rien à lever ici pour le moment.",
                        UiType::Caption,
                        style::INK_SOFT,
                    ));
                }
                for card in shown {
                    let count = basket.counts.get(&card.unit_type).copied().unwrap_or(0);
                    let blocked = basket.treasury_blocks(view, &card.unit_type);
                    spawn_offer_card(
                        grid,
                        theme,
                        assets,
                        icons,
                        &format!("rec:unit:{}", card.unit_type),
                        card,
                        count,
                        blocked,
                        Some(offer_tooltip(card)),
                    );
                }
            });
        // Soon (own short scrolling list, so it never pushes the basket out of the panel).
        let soon = basket::visible_rows(&view.options, &["soon"], &state.category, state.sort);
        if !soon.is_empty() {
            p.spawn((Name::new("rec:soon"), heading(theme, "Bientôt")));
            p.spawn(scroll_list(theme, "rec:soon_list", SOON_HEIGHT))
                .insert(Node {
                    height: Val::Px(SOON_HEIGHT),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    padding: {
                        let mut padding = theme.inner_padding("inset", 4.0);
                        padding.right = Val::Px(crate::ui::kit::widgets::SCROLLBAR_WIDTH + 8.0);
                        padding
                    },
                    overflow: Overflow::scroll_y(),
                    ..default()
                })
                .with_children(|list| {
                    for card in soon {
                        list.spawn((
                            Name::new(format!("rec:soon:{}", card.unit_type)),
                            text(
                                theme,
                                &format!("{} — {}", card.name, card.reason),
                                UiType::Caption,
                                style::INK_SOFT,
                            ),
                        ));
                    }
                });
        }
        // Footer.
        let after = view.treasury - basket.total_cost(view);
        p.spawn((
            Name::new("rec:footer"),
            text(
                theme,
                &basket::footer_text(basket, view),
                UiType::Body,
                if after < 0 { style::RUBRIC } else { style::INK },
            ),
        ));
        if !state.error.is_empty() {
            p.spawn((
                Name::new("rec:error"),
                text(theme, &state.error, UiType::Caption, style::ERROR),
            ));
        }
        p.spawn(Node {
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            let mut seal = row.spawn(button_primary(theme, "rec:seal", "Sceller la levée"));
            if !basket.can_seal(view) {
                seal.insert(bevy::ui::InteractionDisabled);
            }
            let mut clear = row.spawn(button_small(theme, "rec:clear", "Vider"));
            if basket.total_count() == 0 {
                clear.insert(bevy::ui::InteractionDisabled);
            }
            // Minus buttons for the units in the basket.
            for (unit, n) in &basket.counts {
                let label = view
                    .options
                    .iter()
                    .find(|o| &o.unit_type == unit)
                    .map_or(unit.as_str(), |o| o.name.as_str());
                row.spawn(button_small(
                    theme,
                    &format!("rec:minus:{unit}"),
                    &format!("− {label} ({n})"),
                ));
            }
        });
    });
    // Only the card grid may shrink (down to its minimum height); header, queue, tabs, soon list,
    // footer and buttons keep their content height so nothing is squashed or pushed out.
    commands.entity(panel).queue(|mut panel: EntityWorldMut| {
        let children: Vec<Entity> = panel
            .get::<Children>()
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        panel.world_scope(|world| {
            for child in children {
                let is_cards = world
                    .get::<Name>(child)
                    .is_some_and(|n| n.as_str() == "rec:cards");
                if !is_cards && let Some(mut node) = world.get_mut::<Node>(child) {
                    node.flex_shrink = 0.0;
                }
            }
        });
    });
    wrapper
}
