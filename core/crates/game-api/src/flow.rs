//! Campaign ↔ battle flow views: pending battles, the launch package of a 3D battle, the result
//! of a battle (class band, aftermath), map encounters.
//!
//! Replaces the state-reading half of `godot-bridge` `battle_sim_campaign.rs`
//! (`get_pending_battles`, `get_battle_setup`, `get_last_battle_outcome`),
//! `campaign_sim_encounters.rs` and `game/scripts/battle/battle_aftermath.gd`. No rule here: the
//! core computes, these functions only shape and compare.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_battle::BattleSetup;
use sim_campaign::battle_forecast::BattleForecast;
use sim_campaign::battle_request::AutoResolveReport;
use sim_campaign::{ArmyId, CampaignState, GameEvent, Order};

/// A pending battle of the campaign (`get_pending_battles`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PendingBattleView {
    pub index: usize,
    /// Army ids (the attacker of a sortie is the garrison; ids are those of the request).
    pub attacker: String,
    pub defender: String,
    pub province: String,
    pub province_name: String,
    pub attacker_name: String,
    pub defender_name: String,
    /// `attacker`, `defender` or empty (the player is not in the battle).
    pub player_side: String,
    pub attacker_strength: i64,
    pub defender_strength: i64,
    pub siege: bool,
    pub sortie: bool,
    pub fortification: i64,
    pub location: String,
    pub settlement_name: String,
    pub settlement_kind: String,
    pub breach: i64,
    pub seed: u64,
}

/// Pending battles in index order, as the Godot bridge lists them.
pub fn pending_battles(data: &GameData, state: &CampaignState) -> Vec<PendingBattleView> {
    state
        .pending_battle_views(data)
        .iter()
        .map(|view| {
            let strength =
                |id: &ArmyId| state.army(id).map_or(0, |a| i64::from(a.total_strength()));
            let settlement = state.settlement_state(&view.location);
            let garrison: i64 = settlement.map_or(0, |s| {
                s.garrison.iter().map(|u| i64::from(u.strength)).sum()
            });
            let seed =
                state.seed ^ (u64::from(state.turn()) << 20) ^ ((view.index as u64) << 8) ^ 0xBA77;
            PendingBattleView {
                index: view.index,
                attacker: view.attacker.to_string(),
                defender: view.defender.to_string(),
                province: view.province.to_string(),
                province_name: data.province_name(&view.province),
                attacker_name: view.attacker_name.clone(),
                defender_name: view.defender_name.clone(),
                player_side: view.player_side.map_or("", |s| s.key()).to_owned(),
                attacker_strength: if view.sortie {
                    garrison
                } else {
                    strength(&view.attacker)
                },
                defender_strength: if view.siege {
                    garrison
                } else {
                    strength(&view.defender)
                },
                siege: view.siege,
                sortie: view.sortie,
                fortification: i64::from(state.fortification_level(data, &view.location)),
                location: view.location.to_string(),
                settlement_name: data.settlement_name(&view.location),
                settlement_kind: state.settlement_kind(&view.location).key().to_owned(),
                breach: settlement
                    .and_then(|s| s.siege.as_ref())
                    .map_or(0, |s| i64::from(s.breach)),
                seed: seed & 0x7FFF_FFFF_FFFF,
            }
        })
        .collect()
}

/// Estimated balance of pending battle `index` (`get_battle_forecast`).
pub fn battle_forecast(
    data: &GameData,
    state: &CampaignState,
    index: usize,
) -> Result<BattleForecast, String> {
    state
        .battle_forecast(data, index)
        .map_err(|e| e.to_string())
}

/// What the 3D battle needs to be built from a pending battle (`get_battle_setup` + `seed`).
#[derive(Debug, Clone)]
pub struct BattleLaunch {
    pub setup: BattleSetup,
    pub seed: u64,
    /// Starting hour drawn by the campaign (EP8), no random stream consumed.
    pub hour: f64,
    pub index: usize,
}

/// Setup, seed and hour of pending battle `index`.
pub fn battle_launch(
    data: &GameData,
    state: &CampaignState,
    index: usize,
) -> Result<BattleLaunch, String> {
    let setup = state.battle_setup(data, index).map_err(|e| e.to_string())?;
    let seed = pending_battles(data, state)
        .into_iter()
        .find(|view| view.index == index)
        .map_or(state.seed, |view| view.seed);
    let key = sim_battle::time_of_day::campaign_battle_key(state.turn(), index, &setup.province);
    let hour = sim_battle::TimeOfDayRules::bundled().campaign_hour(key);
    Ok(BattleLaunch {
        setup,
        seed,
        hour,
        index,
    })
}

/// Class of the last field battle (`get_last_battle_outcome`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutcomeView {
    pub turn: u32,
    pub province: String,
    pub attacker_faction: String,
    pub defender_faction: String,
    pub attacker_class: String,
    pub attacker_label: String,
    pub defender_class: String,
    pub defender_label: String,
    pub player_class: Option<String>,
    pub player_label: Option<String>,
}

/// The last battle classification, `None` before any field battle.
pub fn last_battle_outcome(state: &CampaignState) -> Option<OutcomeView> {
    let report = state.last_battle_outcome.as_ref()?;
    let player = report.class_of(state.player_faction());
    Some(OutcomeView {
        turn: report.turn,
        province: report.province.to_string(),
        attacker_faction: report.attacker_faction.to_string(),
        defender_faction: report.defender_faction.to_string(),
        attacker_class: report.attacker.key.clone(),
        attacker_label: report.attacker.label.clone(),
        defender_class: report.defender.key.clone(),
        defender_label: report.defender.label.clone(),
        player_class: player.map(|v| v.key.clone()),
        player_label: player.map(|v| v.label.clone()),
    })
}

/// Result of an auto-resolution.
#[derive(Debug, Clone)]
pub struct AutoResolved {
    pub events: Vec<GameEvent>,
    /// `None` for an assault (the result is in the events).
    pub report: Option<AutoResolveReport>,
}

/// Auto-resolves pending battle `index` (`auto_resolve_battle_report`).
pub fn auto_resolve(
    data: &GameData,
    state: &mut CampaignState,
    index: usize,
) -> Result<AutoResolved, String> {
    state
        .auto_resolve_pending_report(data, index)
        .map(|(events, report)| AutoResolved { events, report })
        .map_err(|e| e.to_string())
}

/// The player calls off pending battle `index` (`withdraw_pending_battle`).
pub fn withdraw(
    data: &GameData,
    state: &mut CampaignState,
    index: usize,
) -> Result<Vec<GameEvent>, String> {
    state
        .withdraw_pending_battle(data, index)
        .map_err(|e| e.to_string())
}

/// Applies the outcome of a 3D battle (`resolve_battle`); the class changes with it.
pub fn resolve_battle(
    data: &GameData,
    state: &mut CampaignState,
    index: usize,
    outcome: &sim_battle::BattleOutcome,
) -> Result<Vec<GameEvent>, String> {
    state
        .resolve_pending_battle(data, index, outcome)
        .map_err(|e| e.to_string())
}

// --- Aftermath -----------------------------------------------------------------------------

/// A captive for the aftermath.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaptiveLine {
    pub name: String,
    /// Faction of the captive (`held`) or of the captor (`lost`).
    pub faction: String,
    pub ransom: i64,
    pub rank: String,
}

/// State read before and after a battle of the player (`BattleAftermath.snapshot`).
#[derive(Debug, Clone, Default)]
pub struct AftermathSnapshot {
    /// `(unit type, experience)` of the army's regiments.
    units: Vec<(String, u8)>,
    general: Option<GeneralReading>,
    /// Captives held by the player, by character id.
    held: BTreeMap<String, CaptiveLine>,
    /// The player's captives held by others.
    ours: BTreeMap<String, CaptiveLine>,
}

#[derive(Debug, Clone)]
struct GeneralReading {
    name: String,
    experience: u32,
    skill_points: u32,
    alive: bool,
    captive: bool,
}

/// Reads army, general and captives before / after `resolve`.
pub fn aftermath_snapshot(
    data: &GameData,
    state: &CampaignState,
    army: Option<&ArmyId>,
) -> AftermathSnapshot {
    let mut snap = AftermathSnapshot::default();
    let player: &FactionId = state.player_faction();
    if let Some(army) = army.and_then(|id| state.army(id)) {
        snap.units = army
            .units
            .iter()
            .map(|u| (u.unit_type.to_string(), u.experience))
            .collect();
        snap.general = army
            .general
            .as_ref()
            .and_then(|id| state.character(id).map(|c| (id, c)))
            .map(|(id, c)| GeneralReading {
                name: state.character_name(data, id),
                experience: c.experience,
                skill_points: c.skill_points,
                alive: c.alive,
                captive: c.captive,
            });
    }
    for (id, c) in &state.characters {
        if !c.alive || !c.captive {
            continue;
        }
        let line = |faction: &FactionId| CaptiveLine {
            name: state.character_name(data, id),
            faction: faction.to_string(),
            ransom: sim_campaign::ransom::ransom_amount(state, data, id),
            rank: sim_campaign::ransom::captive_rank(state, id)
                .label_fr()
                .to_owned(),
        };
        if &c.faction == player {
            if let Some(captor) = &c.captor {
                snap.ours.insert(id.to_string(), line(captor));
            }
        } else if c.captor.as_ref() == Some(player) {
            snap.held.insert(id.to_string(), line(&c.faction));
        }
    }
    snap
}

/// What changed for the player's army and captives (`BattleAftermath.diff`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Aftermath {
    pub general_name: String,
    pub general_xp: u32,
    pub skill_points: u32,
    /// `""`, `tombé au combat` or `prisonnier`.
    pub general_fate: String,
    pub veterans: u32,
    pub captives: Vec<CaptiveLine>,
    pub lost: Vec<CaptiveLine>,
    pub ransom_total: i64,
}

pub fn aftermath_diff(before: &AftermathSnapshot, after: &AftermathSnapshot) -> Aftermath {
    let mut result = Aftermath::default();
    if let Some(g0) = &before.general {
        result.general_name = g0.name.clone();
        if let Some(g1) = &after.general {
            result.general_xp = g1.experience.saturating_sub(g0.experience);
            result.skill_points = g1.skill_points.saturating_sub(g0.skill_points);
            if !g1.alive {
                result.general_fate = "tombé au combat".into();
            } else if g1.captive && !g0.captive {
                result.general_fate = "prisonnier".into();
            }
        }
    }
    result.veterans = veterans(&before.units, &after.units);
    for (id, captive) in &after.held {
        if !before.held.contains_key(id) {
            result.ransom_total += captive.ransom;
            result.captives.push(captive.clone());
        }
    }
    for (id, captive) in &after.ours {
        if !before.ours.contains_key(id) {
            result.lost.push(captive.clone());
        }
    }
    result
}

/// Regiments whose experience rose. Annihilated regiments disappear from the army: the lists
/// are paired in order by unit type, skipping the ones missing afterwards.
fn veterans(before: &[(String, u8)], after: &[(String, u8)]) -> u32 {
    let mut count = 0;
    let mut i = 0;
    for (unit_type, experience) in after {
        while i < before.len() && before[i].0 != *unit_type {
            i += 1;
        }
        let Some(old) = before.get(i) else { break };
        if *experience > old.1 {
            count += 1;
        }
        i += 1;
    }
    count
}

// --- Encounters ----------------------------------------------------------------------------

pub use sim_campaign::encounter::{EncounterOptionView, EncounterSiteView, PendingEncounterView};

/// Sites the player sees (`get_encounter_sites`).
pub fn encounter_sites(data: &GameData, state: &CampaignState) -> Vec<EncounterSiteView> {
    state.encounter_site_views(data, state.player_faction())
}

/// Encounters of the player awaiting a choice (`get_pending_encounters`).
pub fn pending_encounters(data: &GameData, state: &CampaignState) -> Vec<PendingEncounterView> {
    state.pending_encounter_views(data, state.player_faction())
}

/// Answers a pending encounter (`choose_encounter_option`).
pub fn choose_encounter_option(
    data: &GameData,
    state: &mut CampaignState,
    army: &str,
    site: u32,
    option: usize,
) -> Result<(), String> {
    let army = ArmyId::parse(army).ok_or_else(|| format!("armée inconnue : {army}"))?;
    state
        .submit_order(data, Order::ChooseEncounterOption { army, site, option })
        .map_err(|e| e.to_string())
}

/// Army and general of the player in pending battle `view` (the aftermath is read for them).
pub fn player_army_of(state: &CampaignState, view: &PendingBattleView) -> Option<ArmyId> {
    let id = if view.player_side == "defender" {
        &view.defender
    } else {
        &view.attacker
    };
    let id = ArmyId::parse(id)?;
    state
        .army(&id)
        .filter(|a| a.faction == *state.player_faction())?;
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn veterans_pair_in_order_skipping_lost_regiments() {
        let before = vec![
            ("a".to_owned(), 0),
            ("b".to_owned(), 1),
            ("c".to_owned(), 2),
        ];
        let after = vec![("a".to_owned(), 1), ("c".to_owned(), 2)];
        assert_eq!(veterans(&before, &after), 1);
        assert_eq!(veterans(&before, &[]), 0);
    }

    #[test]
    fn staged_battle_lists_launches_and_resolves() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        assert!(pending_battles(data, &state).is_empty());
        assert!(last_battle_outcome(&state).is_none());
        // Stage a battle between the player's army and any army at war with it.
        let player = state.player_faction().clone();
        let mine: Vec<ArmyId> = state
            .armies
            .iter()
            .filter(|(_, a)| a.faction == player)
            .map(|(id, _)| id.clone())
            .collect();
        let mut staged = None;
        'search: for a in &mine {
            for (d, army) in &state.armies {
                if army.faction != player && state.is_at_war(&player, &army.faction) {
                    let mut probe = state.clone();
                    if probe.debug_stage_battle(a, d).is_ok() {
                        staged = Some((a.clone(), d.clone()));
                        break 'search;
                    }
                }
            }
        }
        let (a, d) = staged.expect("an enemy army at war");
        let index = state.debug_stage_battle(&a, &d).expect("staged");
        let views = pending_battles(data, &state);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].index, index);
        assert!(views[0].attacker_strength > 0 && views[0].defender_strength > 0);
        let launch = battle_launch(data, &state, index).expect("setup");
        assert!(!launch.setup.attacker.units.is_empty());
        assert!((0.0..24.0).contains(&launch.hour));
        assert!(battle_forecast(data, &state, index).is_ok());
        let army = player_army_of(&state, &views[0]);
        let before = aftermath_snapshot(data, &state, army.as_ref());
        let resolved = auto_resolve(data, &mut state, index).expect("auto");
        assert!(resolved.report.is_some());
        assert!(!resolved.events.is_empty());
        assert!(pending_battles(data, &state).is_empty());
        let outcome = last_battle_outcome(&state).expect("class");
        assert!(outcome.player_class.is_some());
        let after = aftermath_snapshot(data, &state, army.as_ref());
        let _ = aftermath_diff(&before, &after);
    }
}
