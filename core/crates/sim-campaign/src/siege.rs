//! Sieges, captures and chevauchées (spec § 1.3 steps 3 and 4).

use data_model::{FactionId, GameData, ProvinceId};

use crate::economy::province_income;
use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState, SiegeState, Stance};

/// Base siege duration in turns, added to the fortification level.
pub const SIEGE_BASE_TURNS: u32 = 2;
/// Devastation added by one turn of chevauchée.
pub const RAID_DEVASTATION: u8 = 30;
/// Unrest added to a province by a chevauchée.
pub const RAID_UNREST: u8 = 10;
/// Share of the province's seasonal tax base taken as loot.
pub const RAID_LOOT_SHARE: f64 = 0.5;
/// Unrest added to a province when it changes hands.
pub const CAPTURE_UNREST: u8 = 20;

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

/// Armies in `province` besieging its controller, sorted by id.
fn besiegers(state: &CampaignState, province: &ProvinceId, controller: &FactionId) -> Vec<ArmyId> {
    state
        .armies
        .iter()
        .filter(|(_, army)| {
            &army.location == province
                && army.stance == Stance::Siege
                && state.is_at_war(&army.faction, controller)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Phase 3: progress, start or lift sieges; capture provinces.
pub(crate) fn resolve_sieges(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ProvinceId> = state.provinces.keys().cloned().collect();
    for province_id in ids {
        let controller = state.provinces[&province_id].controller.clone();
        let besiegers = besiegers(state, &province_id, &controller);
        let defenders = state.friendly_armies_in(&controller, &province_id);
        if besiegers.is_empty() || !defenders.is_empty() {
            if state
                .provinces
                .get_mut(&province_id)
                .and_then(|p| p.siege.take())
                .is_some()
            {
                events.push(
                    GameEvent::new(
                        EventKind::SiegeLifted,
                        format!(
                            "Le siège de {} est levé.",
                            province_name(data, &province_id)
                        ),
                    )
                    .province(&province_id)
                    .faction(&controller),
                );
            }
            continue;
        }
        let attacker = state.armies[&besiegers[0]].faction.clone();
        let garrison_empty = state.provinces[&province_id].garrison.is_empty();
        if garrison_empty {
            capture(state, data, &province_id, &attacker, events);
            continue;
        }
        let fortification = state.fortification_level(data, &province_id);
        let province = state.provinces.get_mut(&province_id).expect("exists");
        match &mut province.siege {
            Some(siege) if siege.attacker == attacker => {
                siege.turns_left = siege.turns_left.saturating_sub(1);
                if siege.turns_left == 0 {
                    province.garrison.clear();
                    capture(state, data, &province_id, &attacker, events);
                }
            }
            _ => {
                province.siege = Some(SiegeState {
                    attacker: attacker.clone(),
                    turns_left: SIEGE_BASE_TURNS + fortification,
                });
                events.push(
                    GameEvent::new(
                        EventKind::SiegeStarted,
                        format!(
                            "{} met le siège devant {}.",
                            faction_name(data, &attacker),
                            province_name(data, &province_id)
                        ),
                    )
                    .province(&province_id)
                    .army(&besiegers[0])
                    .faction(&attacker),
                );
            }
        }
    }
}

/// Hands `province` to `new_controller` (occupation: the de jure owner is kept).
pub(crate) fn capture(
    state: &mut CampaignState,
    data: &GameData,
    province_id: &ProvinceId,
    new_controller: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let province = state.provinces.get_mut(province_id).expect("exists");
    let previous = std::mem::replace(&mut province.controller, new_controller.clone());
    province.siege = None;
    province.garrison.clear();
    province.recruit_queue.clear();
    province.unrest = province.unrest.saturating_add(CAPTURE_UNREST).min(100);
    events.push(
        GameEvent::new(
            EventKind::ProvinceCaptured,
            format!(
                "{} tombe aux mains de {} (auparavant {}).",
                province_name(data, province_id),
                faction_name(data, new_controller),
                faction_name(data, &previous)
            ),
        )
        .province(province_id)
        .faction(new_controller),
    );
}

/// Phase 4: armies in `Raid` stance devastate hostile provinces for loot.
pub(crate) fn resolve_raids(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ArmyId> = state.armies.keys().cloned().collect();
    for army_id in ids {
        let army = &state.armies[&army_id];
        if army.stance != Stance::Raid {
            continue;
        }
        let faction = army.faction.clone();
        let province_id = army.location.clone();
        if !state.is_hostile_territory(&faction, &province_id) {
            continue;
        }
        let province = state.provinces.get_mut(&province_id).expect("exists");
        let loot = (province_income(province) * RAID_LOOT_SHARE).round() as i64;
        province.devastation = province
            .devastation
            .saturating_add(RAID_DEVASTATION)
            .min(100);
        province.unrest = province.unrest.saturating_add(RAID_UNREST).min(100);
        if let Some(faction_state) = state.factions.get_mut(&faction) {
            faction_state.treasury += loot;
        }
        events.push(
            GameEvent::new(
                EventKind::Raid,
                format!(
                    "Chevauchée de {} en {} : {loot} livres de butin.",
                    faction_name(data, &faction),
                    province_name(data, &province_id)
                ),
            )
            .province(&province_id)
            .army(&army_id)
            .faction(&faction),
        );
    }
}
