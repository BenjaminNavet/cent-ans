//! Pending battles of the campaign: offer the dialog, execute its three choices, launch the 3D
//! battle behind the loading card and apply the result when the battle is left.
//!
//! Ports the battle half of `campaign_map.gd` (`_offer_pending_battles`, `_on_battle_auto`,
//! `_on_battle_withdraw`, `_on_battle_fight`, `_on_battle_returned`) and the result handling of
//! `battle_scene.gd` (`_resolve_now`). Every rule is the core's: `game_api::flow` shapes the
//! views, `CampaignState::{auto_resolve_pending_report, withdraw_pending_battle,
//! resolve_pending_battle}` decide.
//!
//! Differences from Godot: the aftermath of a 3D battle is shown in a campaign modal after the
//! return (the battle result screen belongs to the battle HUD lot); an abandoned or unfinished
//! battle counts as a retreat (`withdraw`, else the core auto-resolves it) where Godot left the
//! battle pending; no autosave before the battle (`autosave_before_battle`).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use data_model::GameData;
use game_api::flow::{self, Aftermath, OutcomeView, PendingBattleView};
use sim_battle::BattleOutcome;
use sim_campaign::CampaignState;
use sim_campaign::GameEvent;

use super::dialog::{self, PreBattleDialog};
use super::launch::{self, CampaignBattle};
use super::loading::{self, LoadingCard};
use super::notice;
use super::{
    ActiveBattle, BattleResolved, FlowCommand, FlowState, LeaveBattle, ResolvedKind, Stage,
};
use crate::app::AppState;
use crate::battle::decor::horizon::HorizonProvince;
use crate::battle::hud::aftermath::ResultExtras;
use crate::battle::scene::BattleSession;
use crate::campaign::hud::HudData;
use crate::campaign::hud::flow::TurnFlow;
use crate::menu::saves::{SaveDir, save_now};
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::art_plates::ArtPlates;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::UiTheme;
use crate::ui::kit::modal::{ModalQueue, ModalSpec, PRIORITY_DECISION, ToastRequest};

const RESULT_KEY: &str = "flow:battle_result";

/// The core's switch « batailles interactives » follows the setting.
pub fn apply_settings(settings: Res<Settings>, mut sim: ResMut<Sim>) {
    let Some(state) = sim.state.as_ref() else {
        return;
    };
    if state.interactive_battles != settings.interactive_battles
        && let Some(state) = sim.state.as_mut()
    {
        state.interactive_battles = settings.interactive_battles;
    }
}

/// Opens the dialog on the first pending battle (not during the end of turn, another modal, a
/// launch).
#[allow(clippy::too_many_arguments)]
pub fn offer_pending(
    mut commands: Commands,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    queue: Res<ModalQueue>,
    turn: Res<TurnFlow>,
    mut flow: ResMut<FlowState>,
    dialogs: Query<(Entity, &PreBattleDialog)>,
    settings: Res<Settings>,
    save_dir: Option<Res<SaveDir>>,
) {
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    let pending = flow::pending_battles(data, state);
    // A dialog whose battle is gone closes; the first pending battle gets its dialog.
    for (entity, open) in &dialogs {
        if !pending.iter().any(|p| p.index == open.index) {
            commands.entity(entity).despawn();
        }
    }
    if !dialogs.is_empty() || flow.stage != Stage::Idle || turn.running() || queue.is_busy() {
        return;
    }
    let Some(first) = pending.first() else {
        return;
    };
    match flow::battle_brief(data, state, first) {
        Ok(brief) => {
            flow.stats.dialogs_opened += 1;
            autosave_before_battle(&settings, save_dir.as_deref(), &sim, &mut flow);
            dialog::spawn(&mut commands, &theme, data, &brief);
        }
        Err(message) => {
            // A stale request the core cannot set up: let it be settled by the end of turn.
            if flow.errors.last() != Some(&message) {
                warn!("pre-battle dialog: {message}");
                flow.errors.push(message);
            }
        }
    }
}

/// `autosave_before_battle` (flow_controller.gd): the slot `auto_battle`, written just before the
/// dialog opens, unless the autosave setting is off.
pub fn autosave_before_battle(
    settings: &Settings,
    dir: Option<&SaveDir>,
    sim: &Sim,
    flow: &mut FlowState,
) {
    if settings.autosave_interval == 0 {
        return;
    }
    let Some(dir) = dir else { return };
    match save_now(sim, &dir.0, "auto_battle") {
        Ok(turn) => {
            flow.stats.autosaves += 1;
            flow.last_autosave = Some(Ok(format!("auto_battle (tour {turn})")));
        }
        Err(message) => {
            warn!("autosave before battle: {message}");
            flow.last_autosave = Some(Err(message));
        }
    }
}

/// Dialog buttons → commands.
pub fn dialog_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    dialogs: Query<&PreBattleDialog>,
    mut commands: MessageWriter<FlowCommand>,
) {
    let Ok(open) = dialogs.single() else {
        pressed.clear();
        return;
    };
    for press in pressed.read() {
        match press.name.as_str() {
            dialog::FIGHT => {
                commands.write(FlowCommand::Fight(open.index));
            }
            dialog::AUTO => {
                commands.write(FlowCommand::AutoResolve(open.index));
            }
            dialog::WITHDRAW if open.can_withdraw => {
                commands.write(FlowCommand::Withdraw(open.index));
            }
            _ => {}
        }
    }
}

fn view_of(sim: &Sim, index: usize) -> Option<PendingBattleView> {
    let (data, state) = (sim.data.as_ref()?, sim.state.as_ref()?);
    flow::pending_battles(data, state)
        .into_iter()
        .find(|view| view.index == index)
}

/// Runs the player's choices on the dialog and the encounter window.
#[allow(clippy::too_many_arguments)]
pub fn execute_commands(
    mut messages: MessageReader<FlowCommand>,
    mut commands: Commands,
    mut sim: ResMut<Sim>,
    mut flow: ResMut<FlowState>,
    mut queue: ResMut<ModalQueue>,
    mut toasts: MessageWriter<ToastRequest>,
    mut resolved: MessageWriter<BattleResolved>,
    mut hud: ResMut<HudData>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    screens: Res<ArtPlates>,
    windows: Query<&Window, With<PrimaryWindow>>,
    dialogs: Query<Entity, With<PreBattleDialog>>,
    encounter_windows: Query<Entity, With<super::encounters::EncounterWindow>>,
    settings: Res<Settings>,
) {
    for command in messages.read() {
        match command.clone() {
            FlowCommand::AutoResolve(index) => {
                for dialog in &dialogs {
                    commands.entity(dialog).despawn();
                }
                let Some(view) = view_of(&sim, index) else {
                    continue;
                };
                let Some(data) = sim.data.clone() else {
                    continue;
                };
                let Some(state) = sim.state.as_mut() else {
                    continue;
                };
                let army = flow::player_army_of(state, &view);
                let before = flow::aftermath_snapshot(&data, state, army.as_ref());
                match flow::auto_resolve(&data, state, index) {
                    Ok(result) => {
                        let after = flow::aftermath_snapshot(&data, state, army.as_ref());
                        let aftermath = flow::aftermath_diff(&before, &after);
                        let outcome = flow::last_battle_outcome(state);
                        let (title, body) =
                            notice::auto_summary(&result, outcome.as_ref(), &aftermath);
                        flow.stats.auto_resolved += 1;
                        flow.last_result_title = title.clone();
                        flow.last_aftermath = Some(aftermath);
                        flow.log_events(&result.events);
                        if result.report.is_some() {
                            queue.request(
                                RESULT_KEY,
                                PRIORITY_DECISION,
                                ModalSpec::info(&title, &body),
                            );
                        }
                        resolved.write(BattleResolved {
                            kind: ResolvedKind::Auto,
                            events: result.events,
                        });
                        hud.force_refresh();
                    }
                    Err(message) => {
                        flow.errors.push(message.clone());
                        toasts.write(ToastRequest::new(&message));
                    }
                }
            }
            FlowCommand::Withdraw(index) => {
                for dialog in &dialogs {
                    commands.entity(dialog).despawn();
                }
                let Some(data) = sim.data.clone() else {
                    continue;
                };
                let Some(state) = sim.state.as_mut() else {
                    continue;
                };
                match flow::withdraw(&data, state, index) {
                    Ok(events) => {
                        flow.stats.withdrawn += 1;
                        flow.log_events(&events);
                        resolved.write(BattleResolved {
                            kind: ResolvedKind::Withdrawn,
                            events,
                        });
                        hud.force_refresh();
                    }
                    Err(message) => {
                        flow.errors.push(message.clone());
                        toasts.write(ToastRequest::new(&message));
                    }
                }
            }
            FlowCommand::Fight(index) => {
                if flow.stage != Stage::Idle {
                    continue;
                }
                for dialog in &dialogs {
                    commands.entity(dialog).despawn();
                }
                let Some(view) = view_of(&sim, index) else {
                    continue;
                };
                let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
                    continue;
                };
                let army = flow::player_army_of(state, &view);
                let before = flow::aftermath_snapshot(data, state, army.as_ref());
                let context = if view.siege { "siege" } else { "battle" };
                let min_seconds = card_seconds(&screens, &settings, &flow);
                let card = loading::open_card(
                    &mut commands,
                    &theme,
                    &assets,
                    &screens,
                    context,
                    view.seed,
                    loading::window_height(&windows),
                    min_seconds,
                );
                flow.card = Some(card);
                flow.launched = false;
                flow.cards_opened += 1;
                flow.stage = Stage::Launching;
                flow.from_menu = false;
                flow.active = Some(ActiveBattle {
                    index,
                    view,
                    army,
                    before,
                });
            }
            FlowCommand::ChooseEncounter { army, site, option } => {
                let Some(data) = sim.data.clone() else {
                    continue;
                };
                let Some(state) = sim.state.as_mut() else {
                    continue;
                };
                match flow::choose_encounter_option(&data, state, &army, site, option) {
                    Ok(()) => {
                        flow.stats.encounters_chosen += 1;
                        toasts.write(ToastRequest::new("Décision prise."));
                    }
                    Err(message) => {
                        flow.errors.push(message.clone());
                        toasts.write(ToastRequest::new(&message));
                    }
                }
                for window in &encounter_windows {
                    commands.entity(window).despawn();
                }
                hud.force_refresh();
            }
            FlowCommand::CloseEncounterWindow => {
                for window in &encounter_windows {
                    commands.entity(window).despawn();
                }
            }
        }
    }
}

/// Minimum time of a card: the data's, shortened to nothing by the « passable » setting except
/// for the first card of the session (`skippable_by_setting`); `CENT_ANS_LOADING_SECONDS`
/// overrides it for scripts.
fn card_seconds(screens: &ArtPlates, settings: &Settings, flow: &FlowState) -> f32 {
    if let Some(seconds) = std::env::var("CENT_ANS_LOADING_SECONDS")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        return seconds;
    }
    if settings.skip_loading && flow.cards_opened > 0 {
        0.0
    } else {
        screens.min_seconds
    }
}

/// Once the card is on screen, builds the launch package and enters the battle.
pub fn drive_launch(
    mut commands: Commands,
    sim: Res<Sim>,
    mut flow: ResMut<FlowState>,
    mut next: ResMut<NextState<AppState>>,
    mut toasts: MessageWriter<ToastRequest>,
    cards: Query<&LoadingCard>,
) {
    if flow.stage != Stage::Launching || flow.launched {
        return;
    }
    let Some(card) = flow.card.and_then(|e| cards.get(e).ok()) else {
        return;
    };
    if !card.drawn() {
        return;
    }
    flow.launched = true;
    let (Some(data), Some(state), Some(active)) =
        (sim.data.as_ref(), sim.state.as_ref(), flow.active.as_ref())
    else {
        return;
    };
    match flow::battle_launch(data, state, active.index) {
        Ok(launch) => {
            // EP7: a field battle in the province and years of a historical map is fought on
            // the real site.
            let site = crate::paths::find_root().ok().and_then(|root| {
                launch::campaign_site(&crate::paths::data_dir(&root), &launch.setup, state.year())
            });
            // The horizon ring uses the real relief of the battle's province.
            commands.insert_resource(HorizonProvince(launch::horizon_province(
                &launch.setup.province,
            )));
            flow.stats.site_battles += u32::from(site.is_some());
            flow.last_site = site.as_ref().map(|map| map.id.clone());
            commands.insert_resource(CampaignBattle { launch, site });
            flow.stats.launched += 1;
            flow.stage = Stage::InBattle;
            next.set(AppState::Battle);
        }
        Err(message) => {
            flow.errors.push(message.clone());
            toasts.write(ToastRequest::new(&format!(
                "Bataille impossible : {message}"
            )));
            flow.stage = Stage::Idle;
            flow.active = None;
            if let Some(card) = flow.card.take() {
                commands.entity(card).despawn();
            }
        }
    }
}

/// Closes the card once the battle exists and the minimum time has passed.
pub fn close_card(
    mut commands: Commands,
    mut flow: ResMut<FlowState>,
    session: Option<Res<BattleSession>>,
    cards: Query<&LoadingCard>,
) {
    let Some(entity) = flow.card else { return };
    let Ok(card) = cards.get(entity) else {
        flow.card = None;
        return;
    };
    let ready = session.is_some() || flow.stage == Stage::Idle;
    if ready && card.may_close() {
        commands.entity(entity).despawn();
        flow.card = None;
        flow.stats.cards_closed += 1;
    }
}

/// What applying a battle to the campaign gave (kept until the player leaves the battle screen).
#[derive(Debug, Clone)]
pub struct Settled {
    pub kind: ResolvedKind,
    pub events: Vec<GameEvent>,
    pub error: Option<String>,
    pub aftermath: Aftermath,
    pub class: Option<OutcomeView>,
}

/// Applies the outcome of a played battle to the campaign, or settles an abandoned / unfinished
/// one (a retreat when the rules allow it, else the core resolves it).
fn settle(
    data: &GameData,
    state: &mut CampaignState,
    active: &ActiveBattle,
    outcome: Option<&BattleOutcome>,
    abandoned: bool,
) -> Settled {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut error = None;
    let kind;
    match (outcome, abandoned) {
        (Some(outcome), false) => {
            kind = ResolvedKind::Played;
            match flow::resolve_battle(data, state, active.index, outcome) {
                Ok(list) => events = list,
                Err(message) => error = Some(message),
            }
        }
        _ => {
            kind = ResolvedKind::Abandoned;
            match flow::withdraw(data, state, active.index) {
                Ok(list) => events = list,
                Err(_) => match flow::auto_resolve(data, state, active.index) {
                    Ok(result) => events = result.events,
                    Err(message) => error = Some(message),
                },
            }
        }
    }
    let after = flow::aftermath_snapshot(data, state, active.army.as_ref());
    let aftermath = flow::aftermath_diff(&active.before, &after);
    let class = flow::last_battle_outcome(state);
    Settled {
        kind,
        events,
        error,
        aftermath,
        class,
    }
}

/// The moment a played battle ends, its outcome is applied to the campaign (as Godot's
/// `_resolve_now`), so the result screen can show the « suites » (captives, ransoms, experience,
/// loot) from [`ResultExtras::aftermath`]. Runs right after the battle's step, before the HUD
/// builds the screen; a replay never settles anything.
pub fn settle_on_finish(
    mut session: ResMut<BattleSession>,
    mut sim: ResMut<Sim>,
    mut flow: ResMut<FlowState>,
    mut extras: ResMut<ResultExtras>,
) {
    if session.replaying || flow.settled.is_some() || flow.stage != Stage::InBattle {
        return;
    }
    let Some(active) = flow.active.clone() else {
        return;
    };
    let battle = session.sim_mut();
    if !battle.is_finished() {
        return;
    }
    let Some(outcome) = battle.outcome() else {
        return;
    };
    let Some(data) = sim.data.clone() else {
        return;
    };
    let Some(state) = sim.state.as_mut() else {
        return;
    };
    let settled = settle(&data, state, &active, Some(&outcome), false);
    extras.aftermath = Some(settled.aftermath.clone());
    flow.stats.settled_at_end += 1;
    flow.settled = Some(settled);
}

/// Where leaving a battle leads: a battle launched from the main menu (or with no campaign battle
/// behind it) returns to the menu, a campaign battle to the campaign.
pub fn leave_destination(from_menu: bool, has_campaign_battle: bool) -> AppState {
    if from_menu || !has_campaign_battle {
        AppState::MainMenu
    } else {
        AppState::Campaign
    }
}

/// `LeaveBattle`: apply the result to the campaign (unless it already was) and go back to it; a
/// battle that came from the main menu (custom, historical, replay) returns to the menu.
#[allow(clippy::too_many_arguments)]
pub fn leave_battle(
    mut leaves: MessageReader<LeaveBattle>,
    mut commands: Commands,
    mut sim: ResMut<Sim>,
    mut flow: ResMut<FlowState>,
    mut session: Option<ResMut<BattleSession>>,
    mut next: ResMut<NextState<AppState>>,
    mut resolved: MessageWriter<BattleResolved>,
) {
    let Some(leave) = leaves.read().last().copied() else {
        return;
    };
    let from_menu = std::mem::take(&mut flow.from_menu);
    let active = flow.active.take();
    if leave_destination(from_menu, active.is_some()) == AppState::MainMenu {
        // Not a campaign battle (custom battle screen, historical, demo, replay of the menu):
        // nothing to apply, back to the menu.
        flow.stage = Stage::Idle;
        flow.settled = None;
        commands.remove_resource::<CampaignBattle>();
        next.set(AppState::MainMenu);
        return;
    }
    let Some(active) = active else {
        return;
    };
    let Some(data) = sim.data.clone() else {
        return;
    };
    let Some(state) = sim.state.as_mut() else {
        return;
    };
    let settled = match flow.settled.take() {
        Some(settled) => settled,
        None => {
            let outcome = session.as_mut().and_then(|s| {
                let battle = s.sim_mut();
                if battle.is_finished() {
                    battle.outcome()
                } else {
                    None
                }
            });
            settle(&data, state, &active, outcome.as_ref(), leave.abandoned)
        }
    };
    let Settled {
        kind,
        events,
        error,
        aftermath,
        class,
    } = settled;
    // The 3D battle showed its own class: no band for it.
    flow.seen_outcome = class.clone();
    flow.primed_outcome = true;
    match &error {
        Some(message) => {
            warn!("battle result refused: {message}");
            flow.errors.push(message.clone());
            flow.stats.result_refused += 1;
        }
        None => {
            flow.stats.left_battle += 1;
            if matches!(kind, ResolvedKind::Abandoned) {
                flow.stats.abandoned += 1;
            }
        }
    }
    let mut lines: Vec<String> = Vec::new();
    if let Some(label) = class.as_ref().and_then(|c| c.player_label.as_deref()) {
        lines.push(format!("[b]{label}[/b]"));
    }
    if matches!(kind, ResolvedKind::Abandoned) {
        lines.push("Vous avez quitté le champ de bataille.".to_owned());
    }
    if let Some(message) = &error {
        lines.push(format!("Résultat de bataille refusé : {message}"));
    }
    lines.extend(notice::aftermath_lines(&aftermath));
    flow.last_result_title = format!("Bataille de {}", active.view.province_name);
    flow.notice = Some((flow.last_result_title.clone(), lines.join("\n")));
    flow.last_aftermath = Some(aftermath);
    flow.log_events(&events);
    resolved.write(BattleResolved { kind, events });
    commands.remove_resource::<CampaignBattle>();
    commands.insert_resource(HorizonProvince(None));
    flow.stage = Stage::Idle;
    next.set(AppState::Campaign);
}

/// Back on the campaign: the result modal of the battle just left.
pub fn show_return_notice(
    mut flow: ResMut<FlowState>,
    mut queue: ResMut<ModalQueue>,
    mut hud: ResMut<HudData>,
) {
    hud.force_refresh();
    if let Some((title, body)) = flow.notice.take() {
        queue.request(
            RESULT_KEY,
            PRIORITY_DECISION,
            ModalSpec::info(&title, &body),
        );
    }
}

#[cfg(test)]
mod leave_tests {
    use super::*;

    #[test]
    fn menu_battles_return_to_the_menu() {
        assert_eq!(leave_destination(true, true), AppState::MainMenu);
        assert_eq!(leave_destination(true, false), AppState::MainMenu);
        assert_eq!(leave_destination(false, false), AppState::MainMenu);
    }

    #[test]
    fn campaign_battles_return_to_the_campaign() {
        assert_eq!(leave_destination(false, true), AppState::Campaign);
    }
}
