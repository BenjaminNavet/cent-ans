//! Aftermath of a 3D field battle on the campaign (lot TW pursuit, ADR
//! 0321): experience of the regiments, plunder of standards and baggage,
//! and the chronicle of the pursuit (men cut down, men taken alive).
//! Rates in `data/rules/battle_outcome.json`.

use data_model::{FactionId, GameData, ProvinceId};
use sim_battle::SideResult;

use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState};
use crate::traditions::add_experience_milli;

/// Gives each regiment of `armies` (in coalition order) the experience its
/// 3D battle earned (`xp_milli` follows the same order). Call before the
/// losses are applied, while the regiments are all still listed.
pub(crate) fn apply_unit_xp(state: &mut CampaignState, armies: &[ArmyId], xp_milli: &[u32]) {
    let mut offset = 0;
    for id in armies {
        let Some(army) = state.armies.get_mut(id) else {
            continue;
        };
        for (i, unit) in army.units.iter_mut().enumerate() {
            if let Some(&milli) = xp_milli.get(offset + i) {
                add_experience_milli(unit, milli);
            }
        }
        offset += army.units.len();
    }
}

/// Trophies and plunder: `side` took `taken` standards from `enemy` and
/// perhaps looted its camp. The gold comes out of the enemy treasury.
#[allow(clippy::too_many_arguments)]
fn pay_spoils(
    state: &mut CampaignState,
    data: &GameData,
    side: &SideResult,
    enemy: &SideResult,
    faction: &FactionId,
    enemy_faction: &FactionId,
    province: &ProvinceId,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.battle_outcome_rules.spoils;
    let taken = side.standards_taken.len() as i64;
    let generals = side.standards_taken.iter().filter(|t| t.general).count() as i32;
    let looted = enemy.baggage_lost;
    let wanted = rules.standard_gold * taken + if looted { rules.baggage_gold } else { 0 };
    let available = state
        .factions
        .get(enemy_faction)
        .map_or(0, |f| f.treasury.max(0));
    let gold = wanted.min(available);
    if gold > 0 {
        if let Some(f) = state.factions.get_mut(enemy_faction) {
            f.treasury -= gold;
        }
        if let Some(f) = state.factions.get_mut(faction) {
            f.treasury += gold;
        }
    }
    let prestige = rules.standard_prestige * taken as i32 + rules.general_standard_prestige * generals;
    if prestige != 0 {
        state.change_ruler_prestige(faction, prestige);
    }
    let lost_prestige = rules.standard_lost_prestige * enemy.standards_lost as i32;
    if lost_prestige != 0 {
        state.change_ruler_prestige(enemy_faction, -lost_prestige);
    }
    if gold > 0 || prestige != 0 {
        let name = sim_battle::sim::of_faction(&data.faction_name(faction));
        let mut parts = Vec::new();
        if taken > 0 {
            parts.push(format!("{taken} étendard(s) pris"));
        }
        if looted {
            parts.push("camp et bagages pillés".to_owned());
        }
        events.push(
            GameEvent::new(
                EventKind::Battle,
                format!(
                    "Butin de l'ost {name} : {} ({gold} livres, {prestige} de prestige).",
                    parts.join(", ")
                ),
            )
            .province(province)
            .faction(faction),
        );
    }
}

/// Plunder for both sides of a 3D battle.
pub(crate) fn apply_spoils(
    state: &mut CampaignState,
    data: &GameData,
    attacker: (&SideResult, &FactionId),
    defender: (&SideResult, &FactionId),
    province: &ProvinceId,
    events: &mut Vec<GameEvent>,
) {
    pay_spoils(state, data, attacker.0, defender.0, attacker.1, defender.1, province, events);
    pay_spoils(state, data, defender.0, attacker.0, defender.1, attacker.1, province, events);
}

/// Chronicle line of the pursuit of the beaten side, if it cost anything.
pub(crate) fn pursuit_event(
    data: &GameData,
    winner: &FactionId,
    loser: &FactionId,
    result: &SideResult,
    province: &ProvinceId,
) -> Option<GameEvent> {
    let killed: u32 = result.pursuit_losses.iter().sum();
    if killed == 0 && result.captured == 0 {
        return None;
    }
    let name = |f: &FactionId| sim_battle::sim::of_faction(&data.faction_name(f));
    let text = format!(
        "Poursuite : l'ost {} rattrape les fuyards de l'ost {} ({killed} tués, {} prisonniers).",
        name(winner),
        name(loser),
        result.captured
    );
    Some(
        GameEvent::new(EventKind::Battle, text)
            .province(province)
            .faction(winner),
    )
}
