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
pub mod launch;
pub mod loading;
pub mod notice;

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
}

#[derive(Resource, Default)]
pub struct FlowState {
    pub stage: Stage,
    pub active: Option<ActiveBattle>,
    pub stats: FlowStats,
    pub card: Option<Entity>,
    pub cards_opened: u32,
    pub(crate) launched: bool,
    pub(crate) primed_outcome: bool,
    pub(crate) seen_outcome: Option<OutcomeView>,
    /// Encounters put off with « Plus tard » (`army:site`).
    pub dismissed: HashSet<String>,
    pub last_aftermath: Option<Aftermath>,
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
        app.add_message::<LeaveBattle>()
            .add_message::<FlowCommand>()
            .add_message::<BattleResolved>()
            .init_resource::<FlowState>()
            .register_dump(dump_flow)
            .add_systems(Startup, loading::load_screens)
            .add_systems(
                OnEnter(AppState::Campaign),
                (
                    encounters::spawn_layer,
                    battles::show_return_notice,
                    reset_for_campaign,
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
                    battles::drive_launch,
                    battles::close_card,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                battles::leave_battle.run_if(in_state(AppState::Battle)),
            )
            .add_plugins(dev::FlowDevPlugin);
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
    (
        "flow".to_owned(),
        json!({
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
            },
        }),
    )
}
