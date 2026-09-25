//! Lot DP1 (ADR 0025): treaties of friendship planned by the AI, beside the
//! war and peace of `sim_campaign::diplomacy::plan_diplomacy` (whose treaty
//! peace lives in `sim_campaign::negotiation::plan_peace`).
//!
//! - **Trade agreements** with friendly partners that are not rivals.
//! - **Military access** swapped between co-belligerents.
//!
//! Every treaty is judged by the recipient through
//! `sim_campaign::negotiation::evaluate_treaty`; the AI only sends what the
//! other side would likely sign (the player receives it as an offer).

use data_model::{FactionId, GameData};
use sim_campaign::diplomacy::{rivals, PAPACY_FACTION, REBELS_FACTION};
use sim_campaign::negotiation::{evaluate_treaty, friendly, Article, Party};
use sim_campaign::{CampaignState, Order};

/// Seasons between two treaty plans of one faction.
const PLAN_PERIOD: u32 = 6;
/// Attitude of the proposer towards a partner worth a trade agreement.
const TRADE_MIN_ATTITUDE: i32 = 10;
/// Attitude a player partner must show to receive a trade offer.
const PLAYER_TRADE_MIN_ATTITUDE: i32 = 30;

fn slot(faction: &FactionId) -> u32 {
    faction
        .as_str()
        .bytes()
        .fold(0u32, |a, b| a.wrapping_add(u32::from(b)))
}

fn negotiable(state: &CampaignState, id: &FactionId) -> bool {
    id.as_str() != REBELS_FACTION
        && id.as_str() != PAPACY_FACTION
        && state.factions.get(id).is_some_and(|f| f.alive)
}

/// Treaties `faction` sends this turn (at most one).
pub fn plan_treaties(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let rules = &data.ai_diplomacy.negotiation;
    if !rules.enabled || !negotiable(state, faction) {
        return Vec::new();
    }
    let phase = (state.turn + slot(faction)) % PLAN_PERIOD;
    let order = match phase {
        0 => plan_trade(state, data, faction),
        3 => plan_access(state, data, faction),
        _ => None,
    };
    order.into_iter().collect()
}

fn would_sign(
    state: &CampaignState,
    data: &GameData,
    from: &FactionId,
    to: &FactionId,
    articles: &[Article],
) -> bool {
    to == &state.player_faction
        || evaluate_treaty(state, data, from, to, articles).chance
            >= data.ai_diplomacy.negotiation.ai_min_chance
}

/// A trade agreement with the friendliest partner that is not a rival.
fn plan_trade(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<Order> {
    let me = state.factions.get(faction)?;
    let my_rivals = rivals(state, faction);
    let treaty = [Article::TradeAgreement];
    state
        .factions
        .keys()
        .filter(|id| *id != faction && negotiable(state, id))
        .filter(|id| !my_rivals.contains(*id) && friendly(state, faction, id))
        .filter(|id| !me.ledger.trade_agreements.contains(*id))
        .filter(|id| !me.embargoes.contains(*id))
        .filter(|id| state.are_neighbors(data, faction, id) || state.is_allied(faction, id))
        .map(|id| (id.clone(), state.attitude(data, faction, id).0))
        .filter(|(id, attitude)| {
            let needed = if id == &state.player_faction {
                PLAYER_TRADE_MIN_ATTITUDE
            } else {
                TRADE_MIN_ATTITUDE
            };
            *attitude >= needed
        })
        .filter(|(id, _)| would_sign(state, data, faction, id, &treaty))
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(target, _)| Order::ProposeTreaty {
            target,
            articles: treaty.to_vec(),
        })
}

/// Mutual military access with a friendly neighbour of one of our enemies
/// (allies already supply each other's armies).
fn plan_access(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<Order> {
    let me = state.factions.get(faction)?;
    let enemies: Vec<&FactionId> = me
        .at_war_with
        .iter()
        .filter(|e| e.as_str() != REBELS_FACTION)
        .collect();
    if enemies.is_empty() {
        return None;
    }
    let my_rivals = rivals(state, faction);
    let mut candidates: Vec<&FactionId> = state
        .factions
        .keys()
        .filter(|id| *id != faction && negotiable(state, id) && *id != &state.player_faction)
        .filter(|id| !state.is_allied(faction, id) && friendly(state, faction, id))
        .filter(|id| !my_rivals.contains(*id))
        .filter(|id| !state.factions[*id].ledger.military_access.contains(faction))
        .filter(|id| enemies.iter().any(|e| state.are_neighbors(data, id, e)))
        .collect();
    candidates.sort();
    candidates.into_iter().find_map(|id| {
        let mut treaty = vec![Article::MilitaryAccess {
            giver: Party::Recipient,
        }];
        if !me.ledger.military_access.contains(id) {
            treaty.push(Article::MilitaryAccess {
                giver: Party::Proposer,
            });
        }
        would_sign(state, data, faction, id, &treaty).then(|| Order::ProposeTreaty {
            target: id.clone(),
            articles: treaty,
        })
    })
}
