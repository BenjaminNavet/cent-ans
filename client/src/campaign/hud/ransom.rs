//! « Captifs et rançons » window (ports game/scripts/ui/ransom_panel.gd, H11).
//!
//! Opened by the HUD button « Rançons » (shown while there is a captive or a debt). « Nos
//! captifs »: pay in cash, by 2 to 3 installments (the plans of the core, up to `MAX_INSTALLMENTS`)
//! or give the province the captor demands. « Nos prisonniers »: set the terms (money / keep),
//! free on parole, execute (asks to confirm). Debts and their due dates close the list. Amounts,
//! plans and refusals all come from `sim_campaign::ransom` and `Order`; nothing is computed here.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use data_model::{CharacterId, GameData};
use serde_json::{Value, json};
use sim_campaign::{CampaignState, Order, RansomTerms};

use super::{HudSet, HudWidget, TurnFlow};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::close_button;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_small, caption, heading, panel_page, scroll_list, separator};
use crate::ui::kit::{ButtonPressed, UiTheme, fr};

/// Installment counts offered next to « Payer comptant ».
const PLANS: [u32; 2] = [2, 4];

pub use game_api::ransoms::{Captive, Debt, RansomView as RansomData};

/// Captives of both sides and debts of the player (`game_api::ransoms::get_ransoms`).
pub fn collect(state: &CampaignState, data: &GameData) -> RansomData {
    game_api::ransoms::get_ransoms(state, data)
}

#[derive(Resource, Default)]
pub struct Ransoms {
    pub data: RansomData,
    pub open: bool,
    /// Captive whose « Exécuter » waits for its confirmation.
    pub armed: Option<String>,
    pub error: String,
    pub orders: u32,
    built: Option<(RansomData, bool, Option<String>, String)>,
}

#[derive(Component)]
struct RansomWindow;

#[derive(Component)]
struct RansomButton;

pub fn plugin(app: &mut App) {
    app.init_resource::<Ransoms>()
        .register_dump(dump_ransoms)
        .add_systems(OnEnter(AppState::Campaign), reset)
        .add_systems(
            Update,
            (refresh, handle_buttons, rebuild_window, update_button)
                .chain()
                .in_set(HudSet::Ui),
        );
}

fn reset(mut ransoms: ResMut<Ransoms>) {
    *ransoms = Ransoms::default();
}

fn refresh(sim: Res<Sim>, mut ransoms: ResMut<Ransoms>) {
    if !sim.is_changed() {
        return;
    }
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    let fresh = collect(state, data);
    if ransoms.data != fresh {
        ransoms.data = fresh;
    }
    if ransoms.data.is_empty() {
        ransoms.open = false;
    }
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut ransoms: ResMut<Ransoms>,
    mut sim: ResMut<Sim>,
    flow: Res<TurnFlow>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        if name == "hud:open_ransoms" {
            ransoms.open = !ransoms.open;
            ransoms.armed = None;
            continue;
        }
        let Some(rest) = name.strip_prefix("ransom:") else {
            continue;
        };
        if rest == "close" {
            ransoms.open = false;
            continue;
        }
        let mut parts = rest.split(':');
        let (Some(action), Some(id)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Ok(character) = CharacterId::new(id) else {
            continue;
        };
        let order = match action {
            "pay" => Order::PayRansom {
                character,
                installments: parts.next().and_then(|n| n.parse().ok()).unwrap_or(1),
            },
            "terms" => Order::SetRansomTerms {
                character,
                terms: if parts.next() == Some("hold") {
                    RansomTerms::Hold
                } else {
                    RansomTerms::Money
                },
            },
            "parole" => Order::ReleaseOnParole { character },
            "execute" => {
                if ransoms.armed.as_deref() != Some(id) {
                    ransoms.armed = Some(id.to_owned());
                    continue;
                }
                ransoms.armed = None;
                Order::ExecuteCaptive { character }
            }
            _ => continue,
        };
        if flow.running() {
            ransoms.error = "La saison se résout : patientez.".to_owned();
            continue;
        }
        let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
            continue;
        };
        let result = game_api::diplomacy::submit_order(&data, state, order);
        ransoms.error = result.message;
        if result.ok {
            ransoms.orders += 1;
        }
    }
}

fn update_button(
    mut commands: Commands,
    ransoms: Res<Ransoms>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    column: Query<Entity, With<super::PanelButtons>>,
    button: Query<Entity, With<RansomButton>>,
) {
    let Ok(column) = column.single() else { return };
    let wanted = !ransoms.data.is_empty();
    match (button.single().ok(), wanted) {
        (None, true) => {
            commands
                .spawn((
                    super::bar::launcher_button(
                        &theme,
                        &assets,
                        "hud:open_ransoms",
                        "hud_ransom",
                        "",
                    ),
                    RansomButton,
                    ChildOf(column),
                    Tooltip::new(&super::bar::launcher_tooltip(
                        "Rançons",
                        "",
                        "Captifs, prisonniers et dettes de rançon.",
                    )),
                ))
                .with_children(|button| {
                    button.spawn(super::bar::launcher_badge(
                        &theme,
                        ransoms.data.ours.len()
                            + ransoms.data.held.len()
                            + ransoms.data.debts.len(),
                    ));
                });
        }
        (Some(entity), false) => commands.entity(entity).despawn(),
        _ => {}
    }
}

fn rebuild_window(
    mut commands: Commands,
    mut ransoms: ResMut<Ransoms>,
    theme: Res<UiTheme>,
    window: Query<Entity, With<RansomWindow>>,
    sim: Res<Sim>,
) {
    let key = (
        ransoms.data.clone(),
        ransoms.open,
        ransoms.armed.clone(),
        ransoms.error.clone(),
    );
    let existing = window.single().ok();
    if ransoms.built.as_ref() == Some(&key) && (existing.is_some() == ransoms.open) {
        return;
    }
    ransoms.built = Some(key);
    if let Some(entity) = existing {
        commands.entity(entity).despawn();
    }
    if !ransoms.open {
        return;
    }
    let turn = sim.state.as_ref().map_or(0, |s| s.turn());
    let data = ransoms.data.clone();
    let armed = ransoms.armed.clone();
    let error = ransoms.error.clone();
    commands
        .spawn((
            Name::new("hud:ransoms"),
            RansomWindow,
            DespawnOnExit(AppState::Campaign),
            UiTier::Panel.index(),
            panel_page(&theme),
            HudWidget,
            Hovered::default(),
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(12.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-290.0)),
            width: Val::Px(580.0),
            padding: UiRect::all(Val::Px(14.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|win| {
            win.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|head| {
                head.spawn(heading(&theme, "Captifs et rançons"));
                head.spawn(close_button(&theme, "ransom:close"));
            });
            if !error.is_empty() {
                win.spawn((
                    Text::new(error),
                    theme.text_font(false, false, UiType::Caption.size()),
                    TextColor(style::POOR),
                ));
            }
            win.spawn(scroll_list(&theme, "ransom:list", 440.0))
                .with_children(|list| {
                    section(
                        list,
                        &theme,
                        "Nos captifs",
                        &data.ours,
                        true,
                        armed.as_deref(),
                    );
                    section(
                        list,
                        &theme,
                        "Nos prisonniers",
                        &data.held,
                        false,
                        armed.as_deref(),
                    );
                    if !data.debts.is_empty() {
                        list.spawn(heading(&theme, "Dettes de rançon"));
                        for debt in &data.debts {
                            let due = if debt.next_due_turn > turn {
                                format!("dans {} saisons", debt.next_due_turn - turn)
                            } else {
                                "échue".to_owned()
                            };
                            list.spawn(caption(
                                &theme,
                                &format!(
                                    "{} : reste {}, échéance de {} ({due}){}",
                                    debt.name,
                                    fr::amount(debt.remaining),
                                    fr::amount(debt.installment),
                                    if debt.missed > 0 {
                                        format!(", {} manquée(s)", debt.missed)
                                    } else {
                                        String::new()
                                    }
                                ),
                            ));
                        }
                    }
                });
        });
}

fn section(
    list: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    title: &str,
    captives: &[Captive],
    ours: bool,
    armed: Option<&str>,
) {
    list.spawn(heading(theme, title));
    if captives.is_empty() {
        list.spawn(caption(theme, "Aucun."));
        return;
    }
    for c in captives {
        list.spawn(separator(theme));
        let terms = match c.terms {
            "money" => "rançon en argent".to_owned(),
            "province" => format!("exige la province {}", c.province.as_deref().unwrap_or("?")),
            "hold" => "refuse toute rançon".to_owned(),
            "parole" => "libération sur parole".to_owned(),
            other => other.to_owned(),
        };
        let line = if ours {
            format!(
                "{} ({}) — retenu par {} : {}, {}",
                c.name,
                c.rank,
                c.captor,
                fr::amount(c.ransom),
                terms
            )
        } else {
            format!(
                "{} ({}) — de {} : {}, {}",
                c.name,
                c.rank,
                c.faction,
                fr::amount(c.ransom),
                terms
            )
        };
        list.spawn((
            Text::new(line),
            theme.text_font(false, false, UiType::Body.size()),
            TextColor(style::INK),
        ));
        list.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|row| {
            if ours {
                let pay = if c.terms == "province" {
                    "Céder la province"
                } else {
                    "Payer comptant"
                };
                row.spawn(button_small(theme, &format!("ransom:pay:{}:1", c.id), pay));
                if c.terms == "money" {
                    for n in PLANS {
                        if let Some((_, total, installment)) = c.plans.iter().find(|p| p.0 == n) {
                            row.spawn((
                                button_small(
                                    theme,
                                    &format!("ransom:pay:{}:{n}", c.id),
                                    &format!("En {n} échéances"),
                                ),
                                Tooltip::new(&format!(
                                    "{n} versements de {} (total {}).",
                                    fr::amount(*installment),
                                    fr::amount(*total)
                                )),
                            ));
                        }
                    }
                }
            } else {
                row.spawn(button_small(
                    theme,
                    &format!("ransom:terms:{}:money", c.id),
                    "Rançon en argent",
                ));
                row.spawn(button_small(
                    theme,
                    &format!("ransom:terms:{}:hold", c.id),
                    "Garder",
                ));
                row.spawn(button_small(
                    theme,
                    &format!("ransom:parole:{}", c.id),
                    "Libérer sur parole",
                ));
                let label = if armed == Some(c.id.as_str()) {
                    "Confirmer l’exécution ?"
                } else {
                    "Exécuter"
                };
                row.spawn(button_small(
                    theme,
                    &format!("ransom:execute:{}", c.id),
                    label,
                ));
            }
        });
    }
}

fn dump_ransoms(world: &World) -> (String, Value) {
    let r = world.resource::<Ransoms>();
    (
        "ransoms".to_owned(),
        json!({
            "ours": r.data.ours.len(), "held": r.data.held.len(), "debts": r.data.debts.len(),
            "open": r.open, "orders": r.orders, "error": r.error,
            "armed": r.armed,
        }),
    )
}
