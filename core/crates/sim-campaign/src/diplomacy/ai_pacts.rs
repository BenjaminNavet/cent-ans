//! The AI's use of « Rejoindre la guerre » and of the non-aggression pact
//! (WR `ai-diplo`, ADR 0302). Pure; hooked into [`super::plan_diplomacy`].
//!
//! Proposals to the player go through the usual offer channel
//! (`Order::ProposeTreaty` → `CampaignState::propose` → `create_offer`, with
//! its cooldown); proposals between AIs are checked against the recipient's
//! own answer first, so a refused request is never sent.

use super::*;
use crate::negotiation::{check_treaty, evaluate_treaty_with, Party};

/// Requests and pacts the AI `faction` sends this turn.
pub fn plan_pacts(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    slot: u32,
) -> Vec<Order> {
    let rules = &data.ai_diplomacy.ai_pacts;
    let state = cache.state();
    let mut orders = Vec::new();
    if !rules.enabled || state.turn < 4 {
        return orders;
    }
    let turn = state.turn.wrapping_add(slot);
    if rules.join_war_period > 0 && turn % rules.join_war_period == 2 % rules.join_war_period {
        orders.extend(join_war_requests(cache, data, faction));
    }
    if rules.pact_period > 0 && turn % rules.pact_period == 3 % rules.pact_period {
        orders.extend(pact_proposal(cache, data, faction));
    }
    orders
}

/// `true` when `article` from `faction` to `target` is valid and the
/// recipient would sign it (the player is only asked: it answers itself).
fn would_be_signed(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    target: &FactionId,
    article: &Article,
) -> bool {
    let state = cache.state();
    let articles = std::slice::from_ref(article);
    if check_treaty(state, data, faction, target, articles).is_err() {
        return false;
    }
    target == &state.player_faction
        || evaluate_treaty_with(cache, data, faction, target, articles).accept
}

/// A faction at war asks its military allies out of it to join, when it is
/// losing or the ally has a grudge against the enemy too.
fn join_war_requests(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let state = cache.state();
    let rules = &data.ai_diplomacy.ai_pacts;
    let Some(me) = state.factions.get(faction) else {
        return Vec::new();
    };
    let mut orders = Vec::new();
    for enemy in me.at_war_with.iter().filter(|e| {
        !e.is_rebels()
            && e.as_str() != PAPACY_FACTION
            && state.factions.get(*e).is_some_and(|f| f.alive)
    }) {
        let ratio = cache.coalition_power(faction) / cache.coalition_power(enemy).max(1.0);
        let losing = ratio < rules.join_war_losing_ratio;
        for ally in &me.allies {
            if orders.len() >= rules.join_war_per_turn {
                return orders;
            }
            let Some(ally_state) = state.factions.get(ally) else {
                continue;
            };
            if !ally_state.alive
                || ally == enemy
                || state.relation(faction, ally) != RelationKind::Alliance
                || state.is_defensive_alliance(faction, ally)
                || ally_state.at_war_with.contains(enemy)
                || cache.attitude(data, ally, faction).0 < rules.join_war_min_attitude
            {
                continue;
            }
            let common = cache.rivals(ally).contains(enemy)
                || cache.attitude(data, ally, enemy).0 < 0
                || cache.neighbour_factions(data, ally).contains(enemy);
            if !(losing || common) {
                continue;
            }
            let article = Article::JoinWar {
                giver: Party::Recipient,
                target: enemy.clone(),
            };
            if would_be_signed(cache, data, faction, ally, &article) {
                orders.push(Order::ProposeTreaty {
                    target: ally.clone(),
                    articles: vec![article],
                });
            }
        }
    }
    orders
}

/// A faction busy elsewhere, or far weaker than a neighbour, offers that
/// neighbour a non-aggression pact when neither bears the other a grudge.
fn pact_proposal(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Option<Order> {
    let state = cache.state();
    let rules = &data.ai_diplomacy.ai_pacts;
    let me = state.factions.get(faction)?;
    let active = me
        .ledger
        .non_aggression
        .values()
        .filter(|until| **until > state.turn)
        .count();
    if active >= rules.pact_max_active {
        return None;
    }
    let busy = me
        .at_war_with
        .iter()
        .any(|e| !e.is_rebels() && state.factions.get(e).is_some_and(|f| f.alive));
    let my_rivals = cache.rivals(faction);
    let my_power = cache.faction_power(faction);
    let article = Article::NonAggression {
        turns: rules.pact_turns,
    };
    cache
        .neighbour_factions(data, faction)
        .into_iter()
        .filter(|n| {
            let Some(f) = state.factions.get(n) else {
                return false;
            };
            f.alive
                && !n.is_rebels()
                && n.as_str() != PAPACY_FACTION
                && !state.is_allied(faction, n)
                && !state.is_at_war(faction, n)
                && !state.has_truce(faction, n)
                && !state.has_non_aggression(faction, n)
                && !my_rivals.contains(n)
                && !cache.rivals(n).contains(faction)
                && !claim_stakes(state, faction, n).any()
                && !claim_stakes(state, n, faction).any()
                && cache.attitude(data, faction, n).0 >= rules.pact_min_attitude
                && cache.attitude(data, n, faction).0 >= rules.pact_min_attitude
                && (busy || my_power < rules.pact_weak_ratio * cache.faction_power(n))
        })
        // The most dangerous neighbour is the one worth a pact.
        .max_by(|a, b| {
            cache
                .faction_power(a)
                .total_cmp(&cache.faction_power(b))
                .then_with(|| b.cmp(a))
        })
        .filter(|n| would_be_signed(cache, data, faction, n, &article))
        .map(|target| Order::ProposeTreaty {
            target,
            articles: vec![article],
        })
}
