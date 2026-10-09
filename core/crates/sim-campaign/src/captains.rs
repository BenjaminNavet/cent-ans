//! Hiring captains (WH chars, ADR 0277): an army without a general can be
//! given a *chevalier banneret* recruited in a settlement of the faction,
//! as Total War: Warhammer III lets one recruit lords.
//!
//! - data: `data/rules/captains.json` ([`CaptainRules`]): cost (scaled by the
//!   price level, `coinage::priced`), cap (`base_cap` + ruler prestige /
//!   `prestige_per_slot`), command range, ages, cooldown between two hirings;
//! - the captain is a generated [`crate::state::CharacterState`] (name drawn
//!   from `data/names` for the faction's culture, house = the province);
//! - AI: [`ai_hire_captain`], when an army has no general and nobody free to
//!   lead it.

use data_model::{CaptainRules, CharacterId, FactionId, GameData, SettlementId, Sex, Skills};

use crate::dynasty::{generated_full_name, next_generated_id, pick_name};
use crate::events::{EventKind, GameEvent};
use crate::orders::{Order, Place};
use crate::state::{CampaignState, CharacterState};

/// Why a `HireCaptain` order was refused (French messages for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptainError {
    #[error("cette place n'est pas tenue par votre faction")]
    NotYours,
    #[error("la place est assiégée ou en ruine : on n'y recrute pas de capitaine")]
    Unavailable,
    #[error("plafond atteint : {cap} capitaine(s) au plus (le prestige du souverain l'élève)")]
    CapReached { cap: u32 },
    #[error("un capitaine a été engagé il y a peu : encore {0} tour(s)")]
    Cooldown(u32),
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    NotEnoughFunds { needed: i64, available: i64 },
}

fn rules() -> &'static CaptainRules {
    CaptainRules::bundled()
}

/// Living captains hired by `faction`.
pub fn captain_count(state: &CampaignState, faction: &FactionId) -> u32 {
    state
        .characters
        .values()
        .filter(|c| c.alive && c.captain && &c.faction == faction)
        .count() as u32
}

/// Captains `faction` may keep: the base plus one per `prestige_per_slot` of
/// its ruler's prestige.
pub fn captain_cap(state: &CampaignState, faction: &FactionId) -> u32 {
    let prestige = state
        .factions
        .get(faction)
        .and_then(|f| f.ruler.as_ref())
        .and_then(|r| state.characters.get(r))
        .map_or(0, |c| c.prestige.max(0));
    rules().base_cap + (prestige / rules().prestige_per_slot.max(1)) as u32
}

/// Price of a captain for `faction` (livres, at the current price level).
pub fn captain_cost(state: &CampaignState, faction: &FactionId) -> i64 {
    crate::coinage::priced(state, faction, rules().base_cost)
}

/// Turns before `faction` may hire again (0: ready).
pub fn cooldown_left(state: &CampaignState, faction: &FactionId) -> u32 {
    state
        .factions
        .get(faction)
        .and_then(|f| f.last_captain_turn)
        .map_or(0, |t| {
            (t + rules().cooldown_turns).saturating_sub(state.turn)
        })
}

/// Why `faction` cannot hire a captain in `settlement` now, if it cannot.
pub fn hire_refusal(
    state: &CampaignState,
    faction: &FactionId,
    settlement: &SettlementId,
) -> Option<CaptainError> {
    let Some(s) = state.settlements.get(settlement) else {
        return Some(CaptainError::NotYours);
    };
    if &s.controller != faction {
        return Some(CaptainError::NotYours);
    }
    if s.siege.is_some() || crate::capture::is_ruined(state, settlement) {
        return Some(CaptainError::Unavailable);
    }
    let cap = captain_cap(state, faction);
    if captain_count(state, faction) >= cap {
        return Some(CaptainError::CapReached { cap });
    }
    let cooldown = cooldown_left(state, faction);
    if cooldown > 0 {
        return Some(CaptainError::Cooldown(cooldown));
    }
    let needed = captain_cost(state, faction);
    let available = state.factions.get(faction).map_or(0, |f| f.treasury);
    (available < needed).then_some(CaptainError::NotEnoughFunds { needed, available })
}

/// Validates and applies `HireCaptain { settlement }` for `faction`; returns
/// the new captain.
pub fn hire_captain(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    settlement: &SettlementId,
) -> Result<CharacterId, CaptainError> {
    if let Some(error) = hire_refusal(state, faction, settlement) {
        return Err(error);
    }
    let province = state.settlements[settlement].province.clone();
    let cost = captain_cost(state, faction);
    let r = rules();
    let first_name = pick_name(data, faction, Sex::Male, &mut state.rng);
    let house = data.province_name(&province);
    let age = r.age_min + state.rng.below((r.age_max - r.age_min + 1).max(1) as u32) as i32;
    let spread = u32::from(r.command_max.saturating_sub(r.command_min)) + 1;
    let skills = Skills {
        command: r.command_min + state.rng.below(spread) as u8,
        governance: state.rng.below(u32::from(r.other_skill_max) + 1) as u8,
        court: state.rng.below(u32::from(r.other_skill_max) + 1) as u8,
    };
    let id = next_generated_id(state);
    let name = generated_full_name(&first_name, &house);
    state.characters.insert(
        id.clone(),
        CharacterState {
            name: Some(name.clone()),
            location: Some(province.clone()),
            title: Some("Chevalier banneret".to_owned()),
            captain: true,
            ..CharacterState::new(faction.clone(), state.year - age, Sex::Male, house, skills)
        },
    );
    let turn = state.turn;
    let f = state.factions.get_mut(faction).expect("faction exists");
    f.treasury -= cost;
    f.last_captain_turn = Some(turn);
    state.pending_events.push(
        GameEvent::new(
            EventKind::Captain,
            format!(
                "{name} entre au service de {}.",
                data.faction_label(faction)
            ),
        )
        .faction(faction)
        .province(&province),
    );
    Ok(id)
}

/// AI: hires one captain when an army has no general and no free adult can
/// lead it; the capital (or the first held settlement) hosts the hiring.
pub fn ai_hire_captain(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let year = state.year;
    let leaderless = state
        .armies
        .values()
        .filter(|a| &a.faction == faction && a.general.is_none())
        .count();
    if leaderless == 0 {
        return Vec::new();
    }
    let free = state
        .characters
        .values()
        .filter(|c| {
            c.alive
                && !c.captive
                && &c.faction == faction
                && c.army.is_none()
                && c.governor_of.is_none()
                && c.is_major(year)
        })
        .count();
    if free > 0 {
        return Vec::new();
    }
    let Some(f) = state.factions.get(faction) else {
        return Vec::new();
    };
    // Keep a reserve: the captain must cost at most a third of the purse.
    if f.treasury < captain_cost(state, faction) * 3 {
        return Vec::new();
    }
    let capital = state
        .province_city_id(&f.capital)
        .filter(|s| hire_refusal(state, faction, s).is_none())
        .cloned()
        .or_else(|| {
            state
                .settlements
                .iter()
                .filter(|(id, _)| hire_refusal(state, faction, id).is_none())
                .map(|(id, _)| id.clone())
                .next()
        });
    let _ = data;
    capital
        .map(|settlement| Order::HireCaptain {
            settlement: Place::Settlement(settlement),
        })
        .into_iter()
        .collect()
}
