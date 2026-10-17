//! Missions (ports game/scripts/ui/mission_tracker.gd and the offer of the objectives panel).
//!
//! - Tracker: always visible under the minimap, collapsible, one line per active mission with its
//!   bar and « n saisons » (red when the deadline is near); a click leads to the target province.
//! - Offer (ADR 0304): when the council offers missions, a window lists the candidates; choosing
//!   one or refusing is the core's `Order::ChooseMission`.
//! - Notices of the last turn (offered, accomplished, failed) come out as toasts.
//!
//! Everything is read from `CampaignState::missions` / `mission_offer` / `mission_notices`.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use serde_json::{Value, json};
use sim_campaign::Order;
use sim_campaign::missions::{MissionOfferView, MissionView};

use super::flow::TurnAdvanced;
use super::{AdvisorColumn, HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::panels::OpenPanel;
use crate::campaign::provinces::picker::SelectProvince;
use crate::sim::Sim;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{
    button_primary, button_small, caption, gauge, heading, panel_note, panel_page, separator,
};
use crate::ui::kit::{ButtonPressed, UiTheme};

const TRACKER_WIDTH: f32 = 250.0;
/// A deadline this close lights the row (`DUE_TURNS`).
pub const DUE_TURNS: u32 = 1;

/// « dernière saison » or « n saisons ».
pub fn left_text(turns_left: u32) -> String {
    if turns_left <= 1 {
        "dernière saison".to_owned()
    } else {
        format!("{turns_left} saisons")
    }
}

pub fn is_due(mission: &MissionView) -> bool {
    mission.turns_left <= DUE_TURNS
}

#[derive(Resource, Default)]
pub struct Missions {
    pub active: Vec<MissionView>,
    pub offer: Option<MissionOfferView>,
    pub collapsed: bool,
    /// The offer window was put aside (« Plus tard »).
    pub offer_dismissed: bool,
    pub notices_shown: u32,
    pub chosen: u32,
    offer_turn: u32,
}

#[derive(Component)]
struct Tracker;

#[derive(Component)]
struct OfferWindow;

#[derive(Component)]
struct MissionTarget(Option<String>);

pub fn plugin(app: &mut App) {
    app.init_resource::<Missions>()
        .register_dump(dump_missions)
        .add_systems(
            Update,
            (
                refresh,
                notices,
                rebuild_tracker,
                rebuild_offer,
                handle_buttons,
            )
                .chain()
                .in_set(HudSet::Ui),
        );
}

fn refresh(sim: Res<Sim>, mut missions: ResMut<Missions>) {
    if !sim.is_changed() {
        return;
    }
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    let active = state.missions(data);
    let offer = state.mission_offer(data);
    let offer_turn = state.mission_offer_turn();
    if offer_turn != missions.offer_turn {
        missions.offer_turn = offer_turn;
        missions.offer_dismissed = false;
    }
    if missions.active != active {
        missions.active = active;
    }
    if missions.offer != offer {
        missions.offer = offer;
    }
}

/// Toasts of the notices left by the turn that just ended.
fn notices(
    mut advanced: MessageReader<TurnAdvanced>,
    sim: Res<Sim>,
    mut toasts: MessageWriter<ToastRequest>,
    mut missions: ResMut<Missions>,
) {
    if advanced.read().next().is_none() {
        return;
    }
    advanced.clear();
    let Some(state) = sim.state.as_ref() else {
        return;
    };
    for notice in state.mission_notices() {
        toasts.write(ToastRequest::new(&notice.text));
        missions.notices_shown += 1;
    }
}

fn rebuild_tracker(
    mut commands: Commands,
    missions: Res<Missions>,
    theme: Res<UiTheme>,
    column: Query<Entity, With<AdvisorColumn>>,
    tracker: Query<Entity, With<Tracker>>,
    mut local: Local<Option<(Vec<MissionView>, bool)>>,
) {
    let Ok(column) = column.single() else { return };
    let key = (missions.active.clone(), missions.collapsed);
    let existing = tracker.single().ok();
    if existing.is_some() && local.as_ref() == Some(&key) {
        return;
    }
    if let Some(entity) = existing {
        commands.entity(entity).despawn();
    }
    *local = Some(key);
    if missions.active.is_empty() {
        return;
    }
    let arrow = if missions.collapsed { "▸" } else { "▾" };
    commands
        .spawn((
            Name::new("hud:missions"),
            Tracker,
            ChildOf(column),
            HudWidget,
            Hovered::default(),
            panel_note(&theme),
        ))
        .insert(Node {
            width: Val::Px(TRACKER_WIDTH),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        })
        .with_children(|box_| {
            box_.spawn(button_small(
                &theme,
                "hud:missions_toggle",
                &format!("{arrow} Missions ({})", missions.active.len()),
            ));
            if missions.collapsed {
                return;
            }
            for mission in &missions.active {
                let tip = format!(
                    "{}\n{}\nRécompense : {}{}",
                    mission.objective,
                    mission.progress,
                    mission.reward,
                    if mission.source.is_empty() {
                        String::new()
                    } else {
                        format!("\nSource : {}", mission.source)
                    }
                );
                box_.spawn((
                    button_small(
                        &theme,
                        &format!("hud:mission:{}", mission.id),
                        &format!("{} — {}", mission.title, left_text(mission.turns_left)),
                    ),
                    MissionTarget(mission.province.as_ref().map(|p| p.as_str().to_owned())),
                    Tooltip::new(&tip),
                ));
                box_.spawn(gauge(
                    &theme,
                    &format!("hud:mission_bar:{}", mission.id),
                    mission.progress_ratio as f32,
                    TRACKER_WIDTH - 24.0,
                ));
            }
        });
}

fn rebuild_offer(
    mut commands: Commands,
    missions: Res<Missions>,
    theme: Res<UiTheme>,
    window: Query<Entity, With<OfferWindow>>,
    modals: Res<crate::ui::kit::modal::ModalQueue>,
    mut local: Local<Option<(Option<MissionOfferView>, bool)>>,
) {
    // The offer waits for the modal in progress (season report, decision): one window at a time.
    let want = (
        missions.offer.clone(),
        missions.offer_dismissed || missions.offer.is_none() || modals.is_busy(),
    );
    let existing = window.single().ok();
    if existing.is_some() && local.as_ref() == Some(&want) {
        return;
    }
    if existing.is_none() && local.as_ref() == Some(&want) {
        return;
    }
    if let Some(entity) = existing {
        commands.entity(entity).despawn();
    }
    *local = Some(want.clone());
    let (Some(offer), false) = (want.0, want.1) else {
        return;
    };
    commands
        .spawn((
            Name::new("hud:mission_offer"),
            OfferWindow,
            DespawnOnExit(AppState::Campaign),
            UiTier::Banner.index(),
            panel_page(&theme),
            HudWidget,
            Hovered::default(),
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(20.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-260.0)),
            width: Val::Px(520.0),
            padding: UiRect::all(Val::Px(14.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|win| {
            win.spawn(heading(&theme, "Le conseil propose des missions"));
            win.spawn(caption(
                &theme,
                &format!(
                    "Choisissez-en une ({}).",
                    if offer.turns_left <= 1 {
                        "dernière saison pour répondre".to_owned()
                    } else {
                        format!("{} saisons pour répondre", offer.turns_left)
                    }
                ),
            ));
            for candidate in &offer.candidates {
                win.spawn(separator(&theme));
                win.spawn((
                    Text::new(format!(
                        "{} — {} saisons\n{}\nRécompense : {}",
                        candidate.title, candidate.duration, candidate.objective, candidate.reward
                    )),
                    theme.text_font(false, false, UiType::Body.size()),
                    TextColor(style::INK),
                ));
                win.spawn(button_primary(
                    &theme,
                    &format!("hud:mission_choose:{}", candidate.index),
                    "Accepter",
                ));
            }
            win.spawn(Node {
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn(button_small(&theme, "hud:mission_refuse", "Refuser tout"));
                row.spawn(button_small(&theme, "hud:mission_later", "Plus tard"));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    targets: Query<&MissionTarget>,
    mut missions: ResMut<Missions>,
    mut sim: ResMut<Sim>,
    mut select: MessageWriter<SelectProvince>,
    mut panels: MessageWriter<OpenPanel>,
    mut toasts: MessageWriter<ToastRequest>,
    flow: Res<super::TurnFlow>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        if name == "hud:missions_toggle" {
            missions.collapsed = !missions.collapsed;
        } else if name == "hud:mission_later" {
            missions.offer_dismissed = true;
        } else if let Some(choice) = name
            .strip_prefix("hud:mission_choose:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            answer(&mut sim, &mut missions, &flow, Some(choice), &mut toasts);
        } else if name == "hud:mission_refuse" {
            answer(&mut sim, &mut missions, &flow, None, &mut toasts);
        } else if name.starts_with("hud:mission:")
            && let Ok(MissionTarget(Some(province))) = targets.get(press.entity)
        {
            select.write(SelectProvince(Some(province.clone())));
            panels.write(OpenPanel::Province(province.clone()));
        }
    }
}

fn answer(
    sim: &mut Sim,
    missions: &mut Missions,
    flow: &super::TurnFlow,
    choice: Option<usize>,
    toasts: &mut MessageWriter<ToastRequest>,
) {
    if flow.running() {
        return;
    }
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return;
    };
    match game_api::diplomacy::submit_order(&data, state, Order::ChooseMission { choice }) {
        result if result.ok => missions.chosen += 1,
        result => {
            toasts.write(ToastRequest::new(&result.message));
        }
    }
}

fn dump_missions(world: &World) -> (String, Value) {
    let missions = world.resource::<Missions>();
    (
        "missions".to_owned(),
        json!({
            "active": missions.active.iter().map(|m| json!({
                "id": m.id, "title": m.title, "left": m.turns_left,
                "ratio": m.progress_ratio, "province": m.province.as_ref().map(|p| p.as_str()),
            })).collect::<Vec<_>>(),
            "offer_candidates": missions.offer.as_ref().map(|o| o.candidates.len()),
            "collapsed": missions.collapsed,
            "offer_dismissed": missions.offer_dismissed,
            "notices_shown": missions.notices_shown,
            "chosen": missions.chosen,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_labels() {
        assert_eq!(left_text(1), "dernière saison");
        assert_eq!(left_text(0), "dernière saison");
        assert_eq!(left_text(4), "4 saisons");
    }
}
