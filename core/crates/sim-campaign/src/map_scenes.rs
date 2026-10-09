//! Lot FK1: province scenes shown on the close campaign view
//! (`docs/design/2026-09-29-carte-vivante-folk.md` § 2.1.1).
//!
//! Purely visual, like the weather (ADR 0027): a pure function of the state
//! and the data, no RNG read, no rule effect.
//!
//! Sources:
//! - the current state: unrest above the revolt threshold or a province held
//!   by the rebels (`revolt`), devastation from
//!   [`data_model::MapSceneRules::devastation_threshold`] (`devastation`),
//!   winter famine risk (`famine`, the famine rule of `population`: winter
//!   and devastation above [`FAMINE_DEVASTATION_THRESHOLD`]), sickly
//!   population (`plague`, below [`PLAGUE_HEALTH_THRESHOLD`]), sieges,
//!   constructions and recruit queues of each settlement (`siege`,
//!   `construction`, `muster`). A cause still active does not fade: the
//!   intensity follows its value.
//! - recent chronicle events carrying a `map_scene`
//!   ([`crate::chronicle::RecentScene`], plus the Black Death wave): they
//!   last [`data_model::MapSceneRules::duration`] turns, the intensity
//!   decreasing linearly with age (1 on the first turn shown).
//!
//! One scene per (province, settlement, kind): the most intense wins.

pub use data_model::SceneKind;
use data_model::{GameData, ProvinceId, SettlementId};
use serde::{Deserialize, Serialize};

use crate::population::{weighted_unrest, FAMINE_DEVASTATION_THRESHOLD, PLAGUE_HEALTH_THRESHOLD};
use crate::state::Season;
use crate::CampaignState;

/// Recruits in the queue for a full-intensity `muster` scene.
const MUSTER_FULL_QUEUE: f32 = 4.0;

/// One scene to stage near a settlement (or the province seat).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapScene {
    pub province: ProvinceId,
    pub settlement: Option<SettlementId>,
    pub kind: SceneKind,
    pub since_turn: u32,
    /// In [0, 1]: from the source value, decreasing with the scene's age.
    pub intensity: f32,
}

/// Scenes of the current state, sorted by province, kind and settlement.
pub fn map_scenes(state: &CampaignState, data: &GameData) -> Vec<MapScene> {
    let turn = state.turn();
    let mut scenes = Vec::new();
    province_scenes(state, data, turn, &mut scenes);
    settlement_scenes(state, data, turn, &mut scenes);
    event_scenes(state, data, turn, &mut scenes);
    for scene in &mut scenes {
        scene.intensity = scene.intensity.clamp(0.0, 1.0);
    }
    scenes.sort_by(|a, b| {
        (&a.province, a.kind, &a.settlement)
            .cmp(&(&b.province, b.kind, &b.settlement))
            .then(b.intensity.total_cmp(&a.intensity))
    });
    scenes.dedup_by(|later, kept| {
        later.province == kept.province
            && later.kind == kept.kind
            && later.settlement == kept.settlement
    });
    scenes
}

/// Revolt, devastation, famine and sickness of each province.
fn province_scenes(state: &CampaignState, data: &GameData, turn: u32, out: &mut Vec<MapScene>) {
    let rules = &data.map_scene_rules;
    let threshold = data.population_rules.revolt_unrest_threshold;
    let winter = state.season() == Season::Winter;
    for (id, province) in &state.provinces {
        let scene = |kind, settlement: Option<SettlementId>, since_turn, intensity| MapScene {
            province: id.clone(),
            settlement,
            kind,
            since_turn,
            intensity,
        };
        let rebels_hold = state
            .province_controller(id)
            .is_some_and(|faction| faction.is_rebels());
        let unrest = weighted_unrest(&province.population);
        if rebels_hold {
            out.push(scene(
                SceneKind::Revolt,
                Some(province.city.clone()),
                turn,
                1.0,
            ));
        } else if unrest > threshold {
            let span = (100.0 - threshold).max(1.0);
            let intensity = 0.5 + 0.5 * ((unrest - threshold) / span) as f32;
            let since = turn.saturating_sub(province.revolt_seasons);
            out.push(scene(
                SceneKind::Revolt,
                Some(province.city.clone()),
                since,
                intensity,
            ));
        }
        if province.devastation >= rules.devastation_threshold.max(1) {
            let intensity = f32::from(province.devastation) / 100.0;
            out.push(scene(SceneKind::Devastation, None, turn, intensity));
        }
        if winter && province.devastation > FAMINE_DEVASTATION_THRESHOLD {
            let intensity = f32::from(province.devastation) / 100.0;
            out.push(scene(SceneKind::Famine, None, turn, intensity));
        }
        let health = province
            .population
            .iter()
            .map(|(_, class)| f32::from(class.health))
            .sum::<f32>()
            / 4.0;
        let sick_below = f32::from(PLAGUE_HEALTH_THRESHOLD);
        if province.population.total() > 0 && health < sick_below {
            let intensity = 0.4 + 0.6 * (1.0 - health / sick_below);
            out.push(scene(
                SceneKind::Plague,
                Some(province.city.clone()),
                turn,
                intensity,
            ));
        }
    }
}

/// Sieges, constructions and recruit queues of each settlement.
fn settlement_scenes(state: &CampaignState, data: &GameData, turn: u32, out: &mut Vec<MapScene>) {
    for (id, settlement) in &state.settlements {
        let scene = |kind, since_turn, intensity| MapScene {
            province: settlement.province.clone(),
            settlement: Some(id.clone()),
            kind,
            since_turn,
            intensity,
        };
        if let Some(siege) = &settlement.siege {
            let hardship = settlement_hardship(siege.breach, siege.supplies);
            out.push(scene(
                SceneKind::Siege,
                siege.started_turn,
                0.4 + 0.6 * hardship,
            ));
        }
        if let Some(work) = &settlement.construction {
            let total = data
                .buildings
                .get(&work.building)
                .map_or(work.turns_left, |b| b.build_time_turns)
                .max(work.turns_left)
                .max(1);
            let done = total - work.turns_left;
            let progress = done as f32 / total as f32;
            out.push(scene(
                SceneKind::Construction,
                turn.saturating_sub(done),
                0.5 + 0.5 * progress,
            ));
        }
        if !settlement.recruit_queue.is_empty() {
            let since = settlement
                .recruit_queue
                .iter()
                .map(|r| r.ordered_turn)
                .min()
                .unwrap_or(turn);
            let intensity =
                (settlement.recruit_queue.len() as f32 / MUSTER_FULL_QUEUE).clamp(0.25, 1.0);
            out.push(scene(SceneKind::Muster, since, intensity));
        }
    }
}

/// 0-1: how far a siege has gone (walls breached or food running out).
fn settlement_hardship(breach: u8, supplies: u8) -> f32 {
    f32::from(breach.max(100u8.saturating_sub(supplies))) / 100.0
}

/// Scenes of recent chronicle events, fading with age.
fn event_scenes(state: &CampaignState, data: &GameData, turn: u32, out: &mut Vec<MapScene>) {
    let rules = &data.map_scene_rules;
    for recent in &state.chronicle.recent_scenes {
        let duration = rules.duration(recent.kind);
        // Fired at the end of `recent.turn`: shown from the next turn.
        let since = recent.turn + 1;
        if turn < since || turn >= since + duration {
            continue;
        }
        let age = turn - since;
        let Some(city) = state
            .provinces
            .get(&recent.province)
            .map(|p| p.city.clone())
        else {
            continue;
        };
        out.push(MapScene {
            province: recent.province.clone(),
            settlement: Some(city),
            kind: recent.kind,
            since_turn: since,
            intensity: 1.0 - age as f32 / duration as f32,
        });
    }
}
