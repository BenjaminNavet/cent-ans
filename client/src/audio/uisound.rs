//! Interface sounds of `UiSounds` (`game/scripts/audio/ui_sounds.gd`) that no module fires by
//! itself: order given / refused, army selected, recruit, build, letter received. Godot calls
//! `UiSounds.play(...)` at each trigger site; here the audio folder watches the state those sites
//! already publish (order logs, selection, chronicle), so no campaign or battle module is edited.
//!
//! | Event            | Godot trigger                                | Watched here                        |
//! |------------------|----------------------------------------------|-------------------------------------|
//! | `ui_order`       | `play_order_result` ok (map, battle)         | campaign `OrderLog`, battle `OrderLog`, army panel |
//! | `ui_order_refused` | same, refused                              | same                                |
//! | `ui_army_select` | `select_army` to another army                | `SelectedArmy`                      |
//! | `ui_recruit`     | `_submit` of a `recruit` order               | army panel `PanelState.last`        |
//! | `ui_build`       | `_submit` of a `build` order                 | province panel (hook, `build:`)     |
//! | `ui_letter`      | `push_news_batch` at the end of a turn       | `ChronicleLog.letters`              |
//! | `ui_order` (panels) | diplomacy / feudal / court / tech screens | `panel_sound` at each press |
//! | `ui_card`        | unit card / ability click                    | `button_event` (mod.rs)             |
//!
//! Dump `ui_sounds`: how many times each event was asked for by this module.

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde_json::{Value, json};

use super::PlaySfx;
use crate::agent::dump::DumpAppExt;
use crate::battle::orders::input::OrderLog as BattleOrderLog;
use crate::campaign::armies::{OrderLog as CampaignOrderLog, SelectedArmy};
use crate::campaign::panels::army::PanelState as ArmyPanelState;
use crate::campaign::panels::chronicle::log::ChronicleLog;

/// Event of an accepted (`ok`) or refused order of the army panel (`kind` is the panel's own
/// order name: `recruit`, `recruit_into`, `disband_unit`).
pub fn panel_order_event(kind: &str, ok: bool) -> &'static str {
    if !ok {
        "ui_order_refused"
    } else if kind.starts_with("recruit") {
        "ui_recruit"
    } else {
        "ui_order"
    }
}

/// Event of a province-panel action: `build:<id>` launches a construction.
pub fn province_action_event(action: &str, ok: bool) -> &'static str {
    if !ok {
        "ui_order_refused"
    } else if action.starts_with("build:") {
        "ui_build"
    } else {
        "ui_order"
    }
}

/// Events for a counter pair that moved: `accepted` then `refused` increments.
pub fn order_events(accepted: u32, refused: u32) -> Vec<&'static str> {
    let mut out = Vec::new();
    if accepted > 0 {
        out.push("ui_order");
    }
    if refused > 0 {
        out.push("ui_order_refused");
    }
    out
}

/// Runs `act` on a panel state and tells whether it produced a fresh order outcome: the result
/// slot is emptied first, so an identical outcome (same text twice) still counts, and the
/// previous outcome is put back when `act` set none. Returns the sound to play: `ui_order` for an
/// accepted order, `ui_order_refused` otherwise, nothing when no order was run (diplomacy,
/// feudal, court screens: Godot plays nothing there, the Bevy port adds one line per screen).
pub fn panel_sound<U, R: Clone>(
    ui: &mut U,
    slot: fn(&mut U) -> &mut Option<R>,
    ok: fn(&R) -> bool,
    act: impl FnOnce(&mut U),
) -> Option<&'static str> {
    let previous = slot(ui).take();
    act(ui);
    match slot(ui) {
        Some(result) => Some(if ok(result) {
            "ui_order"
        } else {
            "ui_order_refused"
        }),
        empty => {
            *empty = previous;
            None
        }
    }
}

/// Counters of the last values seen (a watcher fires on the difference only).
#[derive(Resource, Default, Debug)]
pub struct UiSoundState {
    campaign_orders: Option<u32>,
    campaign_accepted: u32,
    panel_issued: Option<u32>,
    battle: Option<(u32, u32)>,
    selected: Option<String>,
    letters: Option<(u32, String)>,
    /// Requests sent by this module, per event (dump).
    pub sent: BTreeMap<String, u32>,
}

impl UiSoundState {
    fn send(&mut self, sfx: &mut MessageWriter<PlaySfx>, event: &str) {
        *self.sent.entry(event.to_owned()).or_default() += 1;
        sfx.write(PlaySfx::new(event));
    }
}

pub struct UiSoundPlugin;

impl Plugin for UiSoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiSoundState>()
            .add_systems(
                Update,
                (
                    campaign_order_sounds,
                    panel_order_sounds,
                    army_select_sound,
                    letter_sound,
                    battle_order_sounds,
                ),
            )
            .register_dump(dump_ui_sounds);
    }
}

/// Move / march orders of the map (`OrderLog.issued`).
fn campaign_order_sounds(
    log: Option<Res<CampaignOrderLog>>,
    mut state: ResMut<UiSoundState>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some(log) = log else { return };
    let Some(seen) = state.campaign_orders.replace(log.issued) else {
        state.campaign_accepted = log.accepted;
        return;
    };
    if log.issued > seen {
        let ok = log.accepted > state.campaign_accepted;
        state.campaign_accepted = log.accepted;
        state.send(&mut sfx, if ok { "ui_order" } else { "ui_order_refused" });
    }
}

/// Recruitment and the other orders of the army panels (`PanelState.issued`).
fn panel_order_sounds(
    panel: Option<Res<ArmyPanelState>>,
    mut state: ResMut<UiSoundState>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some(panel) = panel else { return };
    let Some(seen) = state.panel_issued.replace(panel.issued) else {
        return;
    };
    if panel.issued > seen
        && let Some(last) = &panel.last
    {
        state.send(&mut sfx, panel_order_event(&last.kind, last.ok));
    }
}

/// Selecting another army of the map.
fn army_select_sound(
    selected: Option<Res<SelectedArmy>>,
    mut state: ResMut<UiSoundState>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some(selected) = selected else { return };
    let now = selected.0.as_ref().map(|id| id.to_string());
    if now != state.selected {
        if now.is_some() && state.selected != now {
            state.send(&mut sfx, "ui_army_select");
        }
        state.selected = now;
    }
}

/// New sealed letters after a turn: one sound per batch (`push_news_batch`).
fn letter_sound(
    log: Option<Res<ChronicleLog>>,
    mut state: ResMut<UiSoundState>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some(log) = log else { return };
    let top = log
        .letters
        .first()
        .map(|letter| format!("{}|{}|{}", letter.kind, letter.title, letter.text))
        .unwrap_or_default();
    let Some((turns, previous_top)) = state.letters.replace((log.turns, top.clone())) else {
        return;
    };
    // `record` puts the turn's letters on top: a new batch moves the top letter.
    if log.turns > turns && top != previous_top {
        state.send(&mut sfx, "ui_letter");
    }
}

/// Orders of the battle, accepted or refused by the core (`battle OrderLog`).
fn battle_order_sounds(
    log: Option<Res<BattleOrderLog>>,
    mut state: ResMut<UiSoundState>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some(log) = log else { return };
    let now = (log.accepted, log.refused);
    let Some((accepted, refused)) = state.battle.replace(now) else {
        return;
    };
    // A fresh battle restarts the counters.
    let (da, dr) = if now.0 < accepted || now.1 < refused {
        (now.0, now.1)
    } else {
        (now.0 - accepted, now.1 - refused)
    };
    for event in order_events(da, dr) {
        state.send(&mut sfx, event);
    }
}

fn dump_ui_sounds(world: &World) -> (String, Value) {
    let Some(state) = world.get_resource::<UiSoundState>() else {
        return ("ui_sounds".into(), Value::Null);
    };
    ("ui_sounds".into(), json!({ "sent": state.sent }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_orders_pick_the_event() {
        assert_eq!(panel_order_event("recruit", true), "ui_recruit");
        assert_eq!(panel_order_event("recruit_into", true), "ui_recruit");
        assert_eq!(panel_order_event("disband_unit", true), "ui_order");
        assert_eq!(panel_order_event("recruit", false), "ui_order_refused");
    }

    #[test]
    fn province_actions_pick_the_event() {
        assert_eq!(province_action_event("build:mill", true), "ui_build");
        assert_eq!(province_action_event("tax:low", true), "ui_order");
        assert_eq!(
            province_action_event("build:mill", false),
            "ui_order_refused"
        );
    }

    #[test]
    fn panel_sound_reports_fresh_outcomes_only() {
        struct Ui {
            result: Option<bool>,
        }
        let slot: fn(&mut Ui) -> &mut Option<bool> = |ui| &mut ui.result;
        let mut ui = Ui { result: Some(true) };
        // No order run: nothing played, the old outcome stays on screen.
        assert_eq!(panel_sound(&mut ui, slot, |r| *r, |_| {}), None);
        assert_eq!(ui.result, Some(true));
        // Same outcome again still plays.
        let again = panel_sound(&mut ui, slot, |r| *r, |ui| ui.result = Some(true));
        assert_eq!(again, Some("ui_order"));
        let refused = panel_sound(&mut ui, slot, |r| *r, |ui| ui.result = Some(false));
        assert_eq!(refused, Some("ui_order_refused"));
    }

    #[test]
    fn counters_become_events() {
        assert_eq!(order_events(0, 0), Vec::<&str>::new());
        assert_eq!(order_events(2, 0), vec!["ui_order"]);
        assert_eq!(order_events(1, 1), vec!["ui_order", "ui_order_refused"]);
    }

    fn sent_after(app: &mut App) -> Vec<String> {
        app.update();
        app.world_mut()
            .resource_mut::<Messages<PlaySfx>>()
            .drain()
            .map(|m| m.event)
            .collect()
    }

    #[test]
    fn battle_log_counters_trigger_order_sounds() {
        let mut app = App::new();
        app.add_message::<PlaySfx>()
            .init_resource::<UiSoundState>()
            .init_resource::<BattleOrderLog>()
            .add_systems(Update, battle_order_sounds);
        assert!(sent_after(&mut app).is_empty());
        app.world_mut().resource_mut::<BattleOrderLog>().accepted = 1;
        assert_eq!(sent_after(&mut app), vec!["ui_order"]);
        app.world_mut().resource_mut::<BattleOrderLog>().refused = 1;
        assert_eq!(sent_after(&mut app), vec!["ui_order_refused"]);
        assert_eq!(app.world().resource::<UiSoundState>().sent.len(), 2);
    }

    #[test]
    fn selecting_another_army_triggers_the_stamp() {
        let mut app = App::new();
        app.add_message::<PlaySfx>()
            .init_resource::<UiSoundState>()
            .init_resource::<SelectedArmy>()
            .add_systems(Update, army_select_sound);
        assert!(sent_after(&mut app).is_empty());
        let id = sim_campaign::ArmyId::from_index(1);
        app.world_mut().resource_mut::<SelectedArmy>().0 = Some(id);
        assert_eq!(sent_after(&mut app), vec!["ui_army_select"]);
        app.world_mut().resource_mut::<SelectedArmy>().0 = None;
        assert!(sent_after(&mut app).is_empty());
    }

    #[test]
    fn unit_cards_and_abilities_use_the_card_sound() {
        assert_eq!(crate::audio::button_event("unitcard:3"), "ui_card");
        assert_eq!(crate::audio::button_event("ability:3:0"), "ui_card");
        assert_eq!(crate::audio::button_event("hud:end_turn"), "ui_click");
    }
}
