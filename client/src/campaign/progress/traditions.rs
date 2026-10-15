//! Army traditions (port of `traditions_controller.gd`, TW2-T5): a side panel with the rank,
//! experience, chosen traditions and effects of an army, then the next tradition of each branch
//! (march, supply, shooting, assault, discipline) with a « Choisir » button, which is the core's
//! `Order::ChooseArmyTradition`. After a battle or a turn, a toast signals each army that has a
//! rank to spend (once per rank).
//!
//! Opens on [`OpenTraditions`] (the army strip button; shows « Traditions (n) » through
//! [`pending_label`]); closes on Escape, `traditions:close`, `OpenPanel::CloseAll`, or when the
//! army is gone. Widget names: `traditions:close`, `traditions:choose:<tradition id>`.
//! Dev hook: `CENT_ANS_OPEN_TRADITIONS=1` opens the first army of the player and grants it
//! experience through the core (`ArmyTraditions::xp`) so a choice is waiting.
//!
//! Not ported: the house banner (`HouseArms`) in the header; the rich tooltip (the description is
//! printed under each tradition instead).

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use game_api::progress::{TraditionsView, pending_traditions, player_armies, traditions_view};
use serde_json::{Value, json};
use sim_campaign::{ArmyId, Order};

use super::OpenTraditions;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::orders_open;
use crate::campaign::panels::OpenPanel;
use crate::menu::KeyEdges;
use crate::sim::Sim;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::{button_small, caption, gauge, panel_sized, separator};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

const PANEL_WIDTH: f32 = 400.0;

#[derive(Resource, Default)]
pub struct TraditionsPanel {
    /// Army shown (`None` = closed).
    pub army: Option<String>,
    root: Option<Entity>,
    dirty: bool,
    /// Rank already announced by toast, per army.
    notified: BTreeMap<String, u8>,
    pub chosen: u32,
    pub toasts: u32,
    pub last_order: String,
    pub view: Option<TraditionsView>,
}

#[derive(Component)]
struct TraditionsRoot;

/// Caption of the army strip button: « Traditions » or « Traditions (n) » when ranks are unspent.
pub fn pending_label(pending: u8) -> String {
    if pending > 0 {
        format!("Traditions ({pending})")
    } else {
        "Traditions".to_owned()
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<TraditionsPanel>()
        .register_dump(dump)
        .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
        .add_systems(
            Update,
            (
                open_close.before(crate::menu::pause::pause_escape),
                handle_buttons.run_if(orders_open),
                notify_ranks,
                rebuild,
            )
                .chain()
                .run_if(in_state(AppState::Campaign)),
        )
        .add_systems(OnExit(AppState::Campaign), close_on_exit);
}

fn reset_on_enter(mut panel: ResMut<TraditionsPanel>, mut sim: ResMut<Sim>) {
    *panel = TraditionsPanel::default();
    if std::env::var("CENT_ANS_OPEN_TRADITIONS").is_err() {
        return;
    }
    let Some(state) = sim.state.as_mut() else {
        return;
    };
    let Some(first) = player_armies(state).into_iter().next() else {
        return;
    };
    if let Some(army) = ArmyId::parse(&first).and_then(|id| state.armies.get_mut(&id)) {
        army.traditions.xp = 400;
    }
    panel.army = Some(first);
    panel.dirty = true;
}

fn close_on_exit(mut commands: Commands, mut panel: ResMut<TraditionsPanel>) {
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    panel.army = None;
}

fn open_close(
    mut panel: ResMut<TraditionsPanel>,
    mut open: MessageReader<OpenTraditions>,
    mut panels: MessageReader<OpenPanel>,
    mut edges: ResMut<KeyEdges>,
) {
    for message in open.read() {
        panel.army = Some(message.0.clone());
        panel.dirty = true;
    }
    for message in panels.read() {
        if matches!(message, OpenPanel::CloseAll) {
            panel.army = None;
        }
    }
    if panel.army.is_some() && edges.escape {
        panel.army = None;
        edges.escape = false;
    }
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut panel: ResMut<TraditionsPanel>,
    mut sim: ResMut<Sim>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("traditions:") else {
            continue;
        };
        if name == "close" {
            panel.army = None;
        } else if let Some(tradition) = name.strip_prefix("choose:") {
            let Some(army) = panel.army.as_deref().and_then(ArmyId::parse) else {
                continue;
            };
            let Some(data) = sim.data.clone() else {
                continue;
            };
            let Some(state) = sim.state.as_mut() else {
                continue;
            };
            let title = panel
                .view
                .as_ref()
                .and_then(|v| v.options.iter().find(|o| o.id == tradition))
                .map_or_else(|| tradition.to_owned(), |o| o.name.clone());
            let result = game_api::diplomacy::submit_order(
                &data,
                state,
                Order::ChooseArmyTradition {
                    army,
                    tradition: tradition.to_owned(),
                },
            );
            if result.ok {
                panel.chosen += 1;
                panel.last_order = format!("ok:{tradition}");
                toasts.write(ToastRequest::new(&format!("Tradition adoptée : {title}.")));
            } else {
                panel.last_order = format!("error:{}", result.message);
                toasts.write(ToastRequest::new(&result.message));
            }
        }
        panel.dirty = true;
    }
}

/// One toast per rank reached, as `notify_new_ranks` of the Godot controller.
fn notify_ranks(
    mut panel: ResMut<TraditionsPanel>,
    sim: Res<Sim>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    if !sim.is_changed() {
        return;
    }
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    let mut fresh: Vec<(String, String, u8)> = Vec::new();
    for entry in pending_traditions(data, state) {
        let Some(view) =
            ArmyId::parse(&entry.army).and_then(|id| traditions_view(data, state, &id))
        else {
            continue;
        };
        if panel.notified.get(&entry.army).copied().unwrap_or(0) >= view.rank {
            continue;
        }
        panel.notified.insert(entry.army.clone(), view.rank);
        fresh.push((entry.army, entry.name, view.rank));
    }
    match fresh.as_slice() {
        [] => {}
        [(_, name, rank)] => {
            toasts.write(ToastRequest::new(&format!(
                "{name} atteint le rang {rank} : une tradition d’armée est à choisir."
            )));
            panel.toasts += 1;
        }
        many => {
            toasts.write(ToastRequest::new(&format!(
                "{} armées peuvent choisir une tradition d’armée.",
                many.len()
            )));
            panel.toasts += 1;
        }
    }
    if panel.army.is_some() {
        panel.dirty = true;
    }
}

fn rebuild(
    mut commands: Commands,
    mut panel: ResMut<TraditionsPanel>,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
) {
    let Some(army) = panel.army.clone() else {
        if let Some(root) = panel.root.take() {
            commands.entity(root).despawn();
        }
        return;
    };
    if !panel.dirty && panel.root.is_some() {
        return;
    }
    panel.dirty = false;
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    let Some(view) = ArmyId::parse(&army).and_then(|id| traditions_view(data, state, &id)) else {
        // Army dissolved or destroyed: its traditions go with it.
        panel.army = None;
        panel.view = None;
        return;
    };
    panel.root = Some(spawn(&mut commands, &theme, &view));
    panel.view = Some(view);
}

fn spawn(commands: &mut Commands, theme: &UiTheme, view: &TraditionsView) -> Entity {
    let theme = theme.clone();
    commands
        .spawn((
            Name::new("TraditionsPanel"),
            TraditionsRoot,
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(84.0),
                right: Val::Px(12.0),
                ..default()
            },
            UiTier::Panel.index(),
        ))
        .with_children(|slot| {
            slot.spawn(panel_sized(&theme, PANEL_WIDTH))
                .with_children(|page| {
                    page.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|bar| {
                        bar.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            max_width: Val::Px(PANEL_WIDTH - 130.0),
                            ..default()
                        })
                        .with_children(|titles| {
                            titles.spawn(caption(&theme, "Traditions de l’armée"));
                            titles.spawn((
                                Name::new("Heading"),
                                Text::new(&view.name),
                                theme.title_font(UiType::Heading.size()),
                                TextColor(style::RUBRIC),
                            ));
                        });
                        bar.spawn(button_small(&theme, "traditions:close", "Fermer"));
                    });
                    page.spawn((
                        Name::new("RankLine"),
                        typed(&theme, &view.rank_line(), UiType::Body),
                    ));
                    let (xp, top) = (
                        view.xp as f32,
                        view.next_threshold.unwrap_or(view.xp).max(1) as f32,
                    );
                    page.spawn(gauge(&theme, "traditions:xp", xp / top, PANEL_WIDTH - 60.0));
                    page.spawn(caption(
                        &theme,
                        &if view.chosen.is_empty() {
                            "Aucune tradition encore : l’armée en gagne une par rang, en livrant bataille.".to_owned()
                        } else {
                            format!("Traditions : {}.", view.chosen.join(", "))
                        },
                    ));
                    if !view.effects_text.is_empty() {
                        page.spawn((
                            Text::new(format!("Effets : {}.", view.effects_text)),
                            theme.text_font(false, false, UiType::Caption.size()),
                            TextColor(style::GOOD),
                        ));
                    }
                    page.spawn(separator(&theme));
                    let prompt = if view.pending > 0 {
                        format!("Choisissez une tradition ({} choix) :", view.pending)
                    } else {
                        "Prochaines traditions (au prochain rang) :".to_owned()
                    };
                    page.spawn((
                        Name::new("Prompt"),
                        Text::new(prompt),
                        theme.text_font(false, false, UiType::Body.size()),
                        TextColor(if view.pending > 0 {
                            style::RUBRIC
                        } else {
                            style::INK_SOFT
                        }),
                    ));
                    for option in view.next_by_branch() {
                        page.spawn((
                            Name::new(format!("option:{}", option.id)),
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(2.0),
                                margin: UiRect::bottom(Val::Px(6.0)),
                                align_items: AlignItems::FlexStart,
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Text::new(format!(
                                    "{} — {} (palier {})",
                                    option.branch_name, option.name, option.tier
                                )),
                                theme.text_font(true, false, UiType::Body.size()),
                                TextColor(style::INK),
                            ));
                            row.spawn(caption(&theme, &option.effects_text));
                            let mut tip = option.description.clone();
                            if !option.allowed && !option.reason.is_empty() {
                                tip.push_str(&format!("\nImpossible : {}.", option.reason));
                            }
                            row.spawn(caption(&theme, &tip));
                            let mut button = row.spawn(button_small(
                                &theme,
                                &format!("traditions:choose:{}", option.id),
                                "Choisir",
                            ));
                            if !option.allowed {
                                button.insert(InteractionDisabled);
                            }
                        });
                    }
                });
        })
        .id()
}

fn dump(world: &World) -> (String, Value) {
    let panel = world.resource::<TraditionsPanel>();
    let sim = world.resource::<Sim>();
    let armies = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => pending_traditions(data, state)
            .into_iter()
            .map(|p| json!({"army": p.army, "pending": p.pending}))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    (
        "progress_traditions".to_owned(),
        json!({
            "open": panel.army,
            "visible": panel.root.is_some(),
            "chosen": panel.chosen,
            "toasts": panel.toasts,
            "last_order": panel.last_order,
            "pending_armies": armies,
            "view": panel.view.as_ref().map(|v| json!({
                "army": v.army, "name": v.name, "rank": v.rank, "xp": v.xp,
                "pending": v.pending, "chosen": v.chosen,
                "next": v.next_by_branch().iter().map(|o| json!({
                    "id": o.id, "tier": o.tier, "allowed": o.allowed,
                })).collect::<Vec<_>>(),
            })),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_caption_counts_pending_ranks() {
        assert_eq!(pending_label(0), "Traditions");
        assert_eq!(pending_label(2), "Traditions (2)");
    }
}
