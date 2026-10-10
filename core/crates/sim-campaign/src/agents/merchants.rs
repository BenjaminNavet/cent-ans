//! Trading posts of the merchants (ADR 0332): seasonal income, expulsion in
//! wartime. Rules in `data/rules/agents.json` (`merchant`).

use super::*;

/// Income of one merchant for a season, before the host's share (0 when the
/// place takes no post).
pub(super) fn post_income(
    state: &CampaignState,
    data: &GameData,
    agent: &Agent,
    place: &SettlementId,
    rivals: usize,
) -> i64 {
    let rules = &rules(data).merchant;
    let Some(settlement) = state.settlements.get(place) else {
        return 0;
    };
    let Some(base) = rules.income.get(&settlement.kind) else {
        return 0;
    };
    let resources = data
        .provinces
        .get(&settlement.province)
        .map_or(0, |p| p.resources.len() as i64);
    let gross = base + rules.resource_income * resources;
    let skilled = gross * (100 + rules.per_level_percent * (i64::from(agent.level) - 1)) / 100;
    let shared = if rivals > 0 {
        skilled * rules.rival_share_percent / 100
    } else {
        skilled
    };
    crate::coinage::priced(state, &agent.faction, shared)
}

/// Seasonal phase: drops stale posts, expels merchants from places held by an
/// enemy, then pays the posted merchants.
pub(super) fn resolve_merchants(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let player = state.player_faction.clone();
    let ids: Vec<AgentId> = state
        .agents
        .agents
        .iter()
        .filter(|(_, a)| a.kind == AgentKind::Merchant && a.post.is_some())
        .map(|(id, _)| id.clone())
        .collect();
    // Stale posts and posts in enemy hands.
    for id in &ids {
        let agent = &state.agents.agents[id];
        let place = agent.post.clone().expect("filtered");
        let enemy_held = state
            .settlements
            .get(&place)
            .is_none_or(|s| state.is_at_war(&agent.faction, &s.controller));
        if agent.location != place || enemy_held {
            if enemy_held && agent.location == place && agent.faction == player {
                events.push(
                    GameEvent::new(
                        EventKind::Agent,
                        format!(
                            "{} est chassé de {} : le comptoir est fermé.",
                            agent.name,
                            data.settlement_name(&place)
                        ),
                    )
                    .faction(&player),
                );
            }
            state.agents.agents.get_mut(id).expect("exists").post = None;
        }
    }
    let mut paid: BTreeMap<FactionId, i64> = BTreeMap::new();
    for id in &ids {
        let agent = &state.agents.agents[id];
        let Some(place) = agent.post.clone() else {
            continue;
        };
        let rivals = state
            .agents
            .agents
            .iter()
            .filter(|(other, a)| *other != id && a.kind == AgentKind::Merchant)
            .filter(|(_, a)| a.post.as_ref() == Some(&place))
            .count();
        let income = post_income(state, data, agent, &place, rivals);
        if income <= 0 {
            continue;
        }
        let host = state
            .settlements
            .get(&place)
            .map(|s| s.controller.clone())
            .filter(|c| c != &agent.faction && c.as_str() != REBELS_FACTION);
        let host_cut = host.as_ref().map_or(0, |_| {
            income * rules(data).merchant.host_share_percent.clamp(0, 100) / 100
        });
        let owner = agent.faction.clone();
        *paid.entry(owner).or_default() += income - host_cut;
        if let Some(host) = host.filter(|_| host_cut > 0) {
            *paid.entry(host).or_default() += host_cut;
        }
    }
    for (faction, amount) in &paid {
        if let Some(f) = state.factions.get_mut(faction) {
            f.treasury += amount;
        }
    }
    state.agents.merchant_income_last_turn = paid;
}
