//! Free characters on the road (WR armies, ADR 0305): `Order::SendCharacter`
//! sends a character without army or governorship to another province of its
//! faction; it arrives after `ceil(distance / km_per_turn)` turns (at least
//! one) and meanwhile cannot be named general (`journey`).

use data_model::{CharacterId, FactionId, GameData, ProvinceId};

use crate::march::px_per_km;
use crate::orders::OrderError;
use crate::state::{CampaignState, Journey};

/// Turns a character needs to go from `from` to `to`.
pub fn travel_turns(data: &GameData, from: &ProvinceId, to: &ProvinceId) -> u32 {
    let point = |p: &ProvinceId| data.province_geometry.get(p).map(|g| g.centroid);
    let (Some(a), Some(b)) = (point(from), point(to)) else {
        return 1;
    };
    let px = f64::from(px_per_km(data)).max(1e-6);
    let km = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() / px;
    let per_turn = data.army_rules.character_travel.km_per_turn.max(1.0);
    ((km / per_turn).ceil() as u32).max(1)
}

pub(crate) fn order_send_character(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    character: &CharacterId,
    to: &ProvinceId,
) -> Result<(), OrderError> {
    let year = state.year;
    if !state.provinces.contains_key(to) {
        return Err(OrderError::UnknownProvince(to.clone()));
    }
    if !state.holds_province(faction, to) {
        return Err(OrderError::NotYourProvince(faction.clone()));
    }
    let c = state
        .characters
        .get(character)
        .ok_or_else(|| OrderError::UnknownCharacter(character.clone()))?;
    if !c.alive || c.captive || &c.faction != faction || !c.is_major(year) {
        return Err(OrderError::CharacterUnavailable);
    }
    if c.army.is_some() || c.governor_of.is_some() || c.journey.is_some() {
        return Err(OrderError::CharacterBusy);
    }
    if c.location.as_ref() == Some(to) {
        return Ok(());
    }
    let turns_left = c
        .location
        .as_ref()
        .map_or(1, |from| travel_turns(data, from, to));
    if let Some(c) = state.characters.get_mut(character) {
        c.journey = Some(Journey {
            to: to.clone(),
            turns_left,
        });
    }
    Ok(())
}

/// Start of the characters phase: one turn on the road; arrivals settle in.
pub(crate) fn advance_journeys(state: &mut CampaignState) {
    for c in state.characters.values_mut() {
        let Some(journey) = c.journey.as_mut() else {
            continue;
        };
        if !c.alive || c.captive {
            c.journey = None;
            continue;
        }
        journey.turns_left = journey.turns_left.saturating_sub(1);
        if journey.turns_left == 0 {
            c.location = Some(journey.to.clone());
            c.journey = None;
        }
    }
}
