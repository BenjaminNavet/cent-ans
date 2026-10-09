//! Black Death wave (M10 `plague_wave` effect), split from `chronicle`.

use data_model::{GameData, ProvinceId, SceneKind};

use crate::effects::add_clamped;

use crate::chronicle::{RecentScene, PLAGUE_HEALTH_LOSS, PLAGUE_POPULATION_LOSS, PLAGUE_UNREST};
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// Provinces sorted south to north (latitude of the capital; unknown last).
fn provinces_by_latitude(state: &CampaignState, data: &GameData) -> Vec<ProvinceId> {
    let mut provinces: Vec<(f64, ProvinceId)> = state
        .provinces
        .keys()
        .map(|id| {
            let latitude = data
                .provinces
                .get(id)
                .and_then(|p| p.geo.as_ref())
                .map_or(f64::MAX, |geo| geo.capital_lonlat[1]);
            (latitude, id.clone())
        })
        .collect();
    provinces.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    provinces.into_iter().map(|(_, id)| id).collect()
}

/// Provinces the wave strikes at step `step` (0-based) out of `duration`.
pub fn plague_slice(
    state: &CampaignState,
    data: &GameData,
    step: u32,
    duration: u32,
) -> Vec<ProvinceId> {
    let ordered = provinces_by_latitude(state, data);
    let count = ordered.len() as u64;
    ordered
        .into_iter()
        .enumerate()
        .filter(|(index, _)| (*index as u64 * u64::from(duration) / count.max(1)) as u32 == step)
        .map(|(_, id)| id)
        .collect()
}

pub(crate) fn resolve_plague_wave(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let Some(wave) = state.chronicle.plague_wave.clone() else {
        return;
    };
    let step = state.turn.saturating_sub(wave.start_turn);
    if step >= wave.duration {
        state.chronicle.plague_wave = None;
        return;
    }
    let reached = plague_slice(state, data, step, wave.duration);
    let (low, high) = PLAGUE_POPULATION_LOSS;
    let mut struck = Vec::new();
    let mut spared = Vec::new();
    for id in reached {
        let loss = low + state.rng.below(high - low + 1);
        // H4: plague resistance may spare the province (the roll only
        // happens with some resistance) and softens the blow.
        let resistance = crate::population::plague_resistance(state, data, &id);
        if resistance > 0.0
            && state.rng.unit_f64()
                < resistance * crate::population::BLACK_DEATH_SPARE_PERCENT / 100.0
        {
            spared.push(id);
            continue;
        }
        let factor = 1.0 - f64::from(loss) / 100.0 * (1.0 - resistance);
        let health = crate::population::mitigated(-PLAGUE_HEALTH_LOSS, resistance);
        let unrest = -crate::population::mitigated(-PLAGUE_UNREST, resistance);
        if let Some(p) = state.provinces.get_mut(&id) {
            for c in p.population.iter_mut() {
                c.health = add_clamped(c.health, health);
                c.unrest = add_clamped(c.unrest, unrest);
                c.count = (c.count as f64 * factor).round() as u64;
            }
        }
        struck.push(id);
    }
    // FK1: every province struck shows its plague scene (visual only).
    let turn = state.turn;
    state
        .chronicle
        .recent_scenes
        .extend(struck.iter().map(|id| RecentScene {
            kind: SceneKind::Plague,
            province: id.clone(),
            turn,
            event: None,
        }));
    let player = state.player_faction.clone();
    for id in &spared {
        if state.controls_province(&player, id) {
            events.push(
                GameEvent::new(
                    EventKind::Medicine,
                    format!(
                        "La Grande Mortalité épargne {} : quarantaine, fumigations et apothicaires ont tenu.",
                        data.province_name(id)
                    ),
                )
                .province(id)
                .faction(&player),
            );
        }
    }
    if !struck.is_empty() {
        let names: Vec<String> = struck.iter().map(|p| data.province_name(p)).collect();
        let mut entry = GameEvent::new(
            EventKind::Plague,
            format!("La Grande Mortalité frappe : {}.", names.join(", ")),
        );
        if let Some(own) = struck.iter().find(|p| state.controls_province(&player, p)) {
            entry = entry.province(own).faction(&player);
        }
        events.push(entry);
    }
    if step + 1 >= wave.duration {
        state.chronicle.plague_wave = None;
    }
}
