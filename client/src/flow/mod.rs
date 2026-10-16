//! Campaign ↔ battle flow: encounter → pre-battle dialog → battle or auto-resolution → result
//! back to the campaign.
//!
//! Ports the flow glue of game/scripts/: `map/encounter_controller.gd` + `ui/encounter_window.gd`
//! (`encounters`), `battle/pre_battle_dialog.gd` (`dialog`), `ui/battle_loading_card.gd`
//! (`loading`), `map/outcome_notice.gd` + `battle/battle_aftermath.gd` (`notice`) and the battle
//! launch / return half of `map/campaign_map.gd` (`battles`, `launch`). The campaign's
//! « enveloppe » of `map/flow_controller.gd` (pause, saves, season report) is the menu and HUD
//! lots'. Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! Shared interface (fixed before wave 3, other lots may use it):
//! - flow hands the battle to `battle::scene` through [`CampaignBattle`] (a campaign battle built
//!   from the core's `BattleSetup`) and puts `AppState::Battle`;
//! - the battle HUD's result screen sends [`LeaveBattle`] when the player clicks « Continuer »;
//!   flow then applies the outcome to the campaign and returns to `AppState::Campaign`.
//!
//! Everything a rule decides is the core's; views and comparisons live in `game_api::flow`.
//! Dev hooks (environment, forwarded by `run_bg.sh`): `CENT_ANS_STAGE_BATTLE=1` stages a battle
//! of a player army against an enemy at war, `CENT_ANS_STAGE_ENCOUNTER=<id>` stages a site next
//! to a player army and marches it there, `CENT_ANS_FLOW_SIM_TO_END=1` plays a battle to its end
//! at once and leaves it, `CENT_ANS_FLOW_ABANDON=1` abandons it, `CENT_ANS_LOADING_SECONDS`
//! overrides the card's minimum time.

pub mod battles;
pub mod dev;
pub mod dialog;
pub mod encounters;
pub mod history;
pub mod launch;
pub mod loading;
pub mod loading_veil;
pub mod naval;
pub mod notice;
pub mod replays;

use std::collections::HashSet;

use bevy::prelude::*;
use game_api::flow::{Aftermath, AftermathSnapshot, OutcomeView, PendingBattleView};
use serde_json::{Value, json};
use sim_campaign::{ArmyId, GameEvent};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::HudSet;
use crate::sim::Sim;

pub use launch::CampaignBattle;
pub use replays::OpenBattleReplays;

/// Sent by the battle result screen (battle/hud) when the player leaves a finished (or
/// abandoned) battle. Flow owns what happens next.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaveBattle {
    /// True when the player quit before the end (counts as a retreat).
    pub abandoned: bool,
}

/// The player's choices (buttons, scripts, tests).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub enum FlowCommand {
    Fight(usize),
    AutoResolve(usize),
    Withdraw(usize),
    ChooseEncounter {
        army: String,
        site: u32,
        option: usize,
    },
    CloseEncounterWindow,
}

/// How a battle ended for the campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedKind {
    Auto,
    Withdrawn,
    Played,
    Abandoned,
}

/// A battle was settled: the core's journal lines, for the news log of other lots.
#[derive(Message, Debug, Clone)]
pub struct BattleResolved {
    pub kind: ResolvedKind,
    pub events: Vec<GameEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Idle,
    /// The loading card is up, the battle is being set up.
    Launching,
    /// `AppState::Battle` is on.
    InBattle,
}

/// The battle being launched or fought.
#[derive(Debug, Clone)]
pub struct ActiveBattle {
    pub index: usize,
    pub view: PendingBattleView,
    pub army: Option<ArmyId>,
    pub before: AftermathSnapshot,
}

#[derive(Debug, Clone, Default)]
pub struct FlowStats {
    pub dialogs_opened: u32,
    pub auto_resolved: u32,
    pub withdrawn: u32,
    pub launched: u32,
    pub cards_closed: u32,
    pub left_battle: u32,
    pub abandoned: u32,
    pub result_refused: u32,
    pub bands_shown: u32,
    pub encounter_windows: u32,
    pub encounters_chosen: u32,
    pub encounters_postponed: u32,
    pub site_battles: u32,
    pub settled_at_end: u32,
    pub autosaves: u32,
    pub naval_dialogs: u32,
    pub naval_resolved: u32,
    pub naval_withdrawn: u32,
}

#[derive(Resource, Default)]
pub struct FlowState {
    pub stage: Stage,
    pub active: Option<ActiveBattle>,
    /// The battle on screen was launched from the main menu (custom, historical, demo): leaving
    /// it goes back to the menu, whatever campaign may be loaded.
    pub from_menu: bool,
    pub stats: FlowStats,
    pub card: Option<Entity>,
    pub cards_opened: u32,
    pub(crate) launched: bool,
    pub(crate) primed_outcome: bool,
    pub(crate) seen_outcome: Option<OutcomeView>,
    /// Encounters put off with « Plus tard » (`army:site`).
    pub dismissed: HashSet<String>,
    pub last_aftermath: Option<Aftermath>,
    /// The played battle applied to the campaign, waiting for the player to leave the screen.
    pub settled: Option<battles::Settled>,
    /// Historical site (map id) of the last launched battle, if any.
    pub last_site: Option<String>,
    /// Result of the last autosave before a battle (`Ok(file)` or the error).
    pub last_autosave: Option<Result<String, String>>,
    pub last_result_title: String,
    /// Modal to show on the campaign after leaving a battle: (title, BBCode).
    pub notice: Option<(String, String)>,
    pub errors: Vec<String>,
    pub journal: Vec<String>,
}

impl FlowState {
    pub(crate) fn log_events(&mut self, events: &[GameEvent]) {
        self.journal
            .extend(events.iter().map(|e| e.text_fr.clone()));
        let excess = self.journal.len().saturating_sub(200);
        self.journal.drain(..excess);
    }
}

pub struct FlowPlugin;

impl Plugin for FlowPlugin {
    fn build(&self, app: &mut App) {
        naval::enable_dialog();
        app.add_plugins(loading_veil::plugin)
            .add_message::<LeaveBattle>()
            .add_message::<FlowCommand>()
            .add_message::<BattleResolved>()
            .init_resource::<FlowState>()
            .init_resource::<history::PlayedBattles>()
            .add_message::<history::OpenPlayedBattles>()
            .init_resource::<history::OverlayStats>()
            .add_systems(
                OnEnter(AppState::FactionSelect),
                history::clear_for_new_game,
            )
            .add_systems(
                Update,
                (
                    history::sync_after_load,
                    history::open_overlay,
                    history::close_overlay,
                ),
            )
            .register_dump(dump_flow)
            .add_systems(
                OnEnter(AppState::Campaign),
                (
                    encounters::spawn_layer,
                    battles::show_return_notice,
                    reset_for_campaign,
                    loading::open_campaign_card,
                ),
            )
            // Campaign: offers, choices, markers, band; after the HUD's own flow (a season
            // report is shown first, battles wait behind it).
            .add_systems(
                Update,
                (
                    battles::apply_settings,
                    battles::offer_pending,
                    encounters::offer_pending,
                    battles::dialog_buttons,
                    naval::offer_pending,
                    naval::dialog_buttons,
                    encounters::window_buttons,
                    battles::execute_commands,
                    encounters::marker_clicks,
                    encounters::sync_markers,
                    encounters::place_markers,
                    notice::watch_outcome,
                    notice::tick_band,
                )
                    .chain()
                    .after(HudSet::Flow)
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(
                Update,
                (
                    loading::tick_cards,
                    loading::close_campaign_card,
                    battles::drive_launch,
                    battles::close_card,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (history::record_played_battle, battles::leave_battle)
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            )
            // Right after the battle's step (before the HUD builds the result screen).
            .add_systems(
                Update,
                battles::settle_on_finish
                    .in_set(crate::battle::scene::StepSet)
                    .after(crate::battle::scene::step_session)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<crate::battle::scene::BattleSession>),
            )
            .add_plugins((dev::FlowDevPlugin, replays::ReplaysPlugin));
    }
}

/// A fresh campaign screen forgets what the last one put off (a loaded game, a new game).
fn reset_for_campaign(mut flow: ResMut<FlowState>, sim: Res<Sim>) {
    if flow.stage == Stage::Idle {
        flow.dismissed.clear();
    }
    if sim.state.is_none() {
        flow.primed_outcome = false;
    }
}

/// The fields added by the scene lot (kept apart: `json!` has a recursion limit).
fn dump_extra(world: &World, flow: &FlowState) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "settled".into(),
        json!(flow.settled.as_ref().map(|s| json!({
            "kind": format!("{:?}", s.kind), "events": s.events.len(), "error": s.error,
            "aftermath": serde_json::to_value(&s.aftermath).unwrap_or(Value::Null),
        }))),
    );
    map.insert("last_site".into(), json!(flow.last_site));
    let sim = world.resource::<Sim>();
    if let (Some(data), Some(state)) = (&sim.data, &sim.state) {
        map.insert(
            "naval_pending".into(),
            json!(state.pending_naval_views(data).len()),
        );
    }
    map.insert(
        "last_autosave".into(),
        json!(flow.last_autosave.as_ref().map(|r| match r {
            Ok(name) => json!({ "ok": name }),
            Err(message) => json!({ "error": message }),
        })),
    );
    map.insert(
        "horizon_province".into(),
        json!(
            world
                .get_resource::<crate::battle::decor::horizon::HorizonProvince>()
                .map(|h| h.0.clone())
        ),
    );
    let extras = world.get_resource::<crate::battle::hud::aftermath::ResultExtras>();
    map.insert(
        "result_extras".into(),
        json!(extras.map(|e| json!({
            "aftermath": e.aftermath.is_some(),
            "history": e.history.as_ref().map(|h| h.name.clone()),
        }))),
    );
    let naval = world
        .try_query::<&naval::NavalDialog>()
        .and_then(|mut q| q.iter(world).next().cloned());
    map.insert(
        "naval_dialog".into(),
        json!(naval.map(|d| json!({
            "index": d.index, "title": d.title, "verdict": d.verdict, "share": d.share,
            "chance": d.chance, "ships": d.ships, "men": d.men,
            "withdraw_label": d.withdraw_label,
        }))),
    );
    let listed = world
        .try_query::<&replays::ReplaysDialog>()
        .and_then(|mut q| q.iter(world).next().map(|d| d.entries.len()));
    map.insert(
        "replays".into(),
        json!(world.get_resource::<replays::ReplayStats>().map(|r| json!({
            "opened": r.opened, "started": r.started, "last_path": r.last_path, "dialog": listed,
        }))),
    );
    Value::Object(map)
}

fn dump_flow(world: &World) -> (String, Value) {
    let flow = world.resource::<FlowState>();
    let sim = world.resource::<Sim>();
    let (pending, sites, encounters) = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => (
            game_api::flow::pending_battles(data, state)
                .iter()
                .map(|p| {
                    json!({
                        "index": p.index, "attacker": p.attacker_name, "defender": p.defender_name,
                        "province": p.province, "siege": p.siege, "sortie": p.sortie,
                        "player_side": p.player_side,
                        "attacker_strength": p.attacker_strength, "defender_strength": p.defender_strength,
                    })
                })
                .collect::<Vec<_>>(),
            game_api::flow::encounter_sites(data, state).len(),
            game_api::flow::pending_encounters(data, state).len(),
        ),
        _ => (Vec::new(), 0, 0),
    };
    let mut dialog_query = world.try_query::<&dialog::PreBattleDialog>();
    let dialog = dialog_query
        .as_mut()
        .and_then(|q| q.iter(world).next().cloned())
        .map(|d| {
            json!({
                "index": d.index, "title": d.title, "verdict": d.verdict, "share": d.share,
                "regiments": d.regiments, "can_withdraw": d.can_withdraw,
                "preview": d.preview.as_ref().map(|p| json!({
                    "weather": p.weather, "site_label": p.site_label,
                    "own_dots": p.own_dots, "enemy_dots": p.enemy_dots,
                    "generals": p.generals, "medallions": p.medallions,
                })),
            })
        });
    let mut window_query = world.try_query::<&encounters::EncounterWindow>();
    let window = window_query
        .as_mut()
        .and_then(|q| q.iter(world).next().cloned())
        .map(|w| json!({ "site": w.site, "army": w.army, "title": w.title, "options": w.options }));
    let mut marker_query = world.try_query::<&encounters::SiteMarker>();
    let markers = marker_query.as_mut().map_or(0, |q| q.iter(world).count());
    let mut card_query = world.try_query::<&loading::LoadingCard>();
    let card = card_query
        .as_mut()
        .and_then(|q| q.iter(world).next().cloned())
        .map(|c| {
            json!({
                "context": c.context, "screen": c.screen_id, "elapsed": c.elapsed,
                "frames": c.frames, "skipped": c.skipped, "min_seconds": c.min_seconds,
            })
        });
    let mut band_query = world.try_query::<&notice::OutcomeBand>();
    let band = band_query
        .as_mut()
        .and_then(|q| q.iter(world).next().cloned())
        .map(|b| json!({ "class": b.class, "text": b.text }));
    let aftermath = flow
        .last_aftermath
        .as_ref()
        .map(|a| serde_json::to_value(a).unwrap_or(Value::Null));
    let s = &flow.stats;
    let extra = dump_extra(world, flow);
    let mut value = json!({
            "stage": format!("{:?}", flow.stage),
            "active_battle": flow.active.as_ref().map(|a| a.index),
            "pending_battles": pending,
            "dialog": dialog,
            "encounter_sites": sites,
            "site_markers": markers,
            "pending_encounters": encounters,
            "encounter_window": window,
            "loading_card": card,
            "campaign_battle_resource": world.contains_resource::<CampaignBattle>(),
            "outcome_band": band,
            "last_result_title": flow.last_result_title,
            "last_aftermath": aftermath,
            "played_battles_overlays_opened": world
                .get_resource::<history::OverlayStats>()
                .map(|stats| stats.opened),
            "played_battles": world
                .get_resource::<history::PlayedBattles>()
                .map(|log| serde_json::to_value(&log.0).unwrap_or(Value::Null)),
            "journal_lines": flow.journal.len(),
            "journal_tail": flow.journal.iter().rev().take(3).collect::<Vec<_>>(),
            "errors": flow.errors,
            "stats": {
                "dialogs_opened": s.dialogs_opened, "auto_resolved": s.auto_resolved,
                "withdrawn": s.withdrawn, "launched": s.launched, "cards_closed": s.cards_closed,
                "left_battle": s.left_battle, "abandoned": s.abandoned,
                "result_refused": s.result_refused, "bands_shown": s.bands_shown,
                "encounter_windows": s.encounter_windows, "encounters_chosen": s.encounters_chosen,
                "encounters_postponed": s.encounters_postponed,
                "site_battles": s.site_battles, "settled_at_end": s.settled_at_end,
                "autosaves": s.autosaves, "naval_dialogs": s.naval_dialogs,
                "naval_resolved": s.naval_resolved, "naval_withdrawn": s.naval_withdrawn,
            },
    });
    if let (Some(base), Some(more)) = (value.as_object_mut(), extra.as_object()) {
        base.extend(more.clone());
    }
    ("flow".to_owned(), value)
}
