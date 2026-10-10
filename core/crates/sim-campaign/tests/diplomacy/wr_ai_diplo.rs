//! Lot WR `ai-diplo` (ADR 0302): the AI asks allies to join its wars and
//! proposes non-aggression pacts; its answers to the player's.

use data_model::test_support::{fac, game_data};
use data_model::{FactionId, GameData};
use sim_campaign::diplomacy::{claim_stakes, plan_pacts};
use sim_campaign::negotiation::{evaluate_treaty, Article, Party, Treaty};
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::{CampaignState, Order};

fn start(seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(game_data(), fac("fac_france"), seed).expect("start");
    state.chronicle.disabled = true;
    state.turn = 8;
    state
}

fn goodwill(state: &mut CampaignState, holder: &str, with: &str, value: i32) {
    let expires_turn = state.turn + 40;
    state
        .factions
        .get_mut(&fac(holder))
        .unwrap()
        .modifiers
        .push(sim_campaign::OpinionModifier {
            with: fac(with),
            value,
            reason_fr: "Bonne volonté".to_owned(),
            expires_turn,
        });
}

/// Every order of `plan_pacts` for `faction`, over all the planning slots.
fn all_orders(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let cache = PlanCache::new(state);
    (0..12)
        .flat_map(|slot| plan_pacts(&cache, data, faction, slot))
        .collect()
}

fn join_war_scene() -> (CampaignState, FactionId, FactionId, FactionId) {
    let mut state = start(31);
    let (pt, ar, ca) = (fac("fac_portugal"), fac("fac_aragon"), fac("fac_castile"));
    for (a, b) in [(&pt, &ar), (&pt, &ca), (&ar, &ca)] {
        for (x, y) in [(a, b), (b, a)] {
            let f = state.factions.get_mut(x).unwrap();
            f.at_war_with.remove(y);
            f.truces.remove(y);
            f.allies.remove(y);
        }
    }
    for (x, y) in [(&pt, &ar), (&ar, &pt)] {
        state.factions.get_mut(x).unwrap().allies.insert(y.clone());
    }
    for (x, y) in [(&pt, &ca), (&ca, &pt)] {
        state.factions.get_mut(x).unwrap().at_war_with.insert(y.clone());
    }
    goodwill(&mut state, "fac_aragon", "fac_portugal", 60);
    goodwill(&mut state, "fac_aragon", "fac_castile", -60);
    state.factions.get_mut(&ar).unwrap().treasury = 50_000;
    state.factions.get_mut(&ar).unwrap().ledger.weariness = 0;
    (state, pt, ar, ca)
}

fn join_requests(orders: &[Order], ally: &FactionId, enemy: &FactionId) -> usize {
    orders
        .iter()
        .filter(|o| {
            matches!(o, Order::ProposeTreaty { target, articles }
                if target == ally
                    && matches!(articles.as_slice(),
                        [Article::JoinWar { giver: Party::Recipient, target: t }] if t == enemy))
        })
        .count()
}

#[test]
fn a_losing_faction_asks_its_idle_ally_to_join_once_per_period() {
    let (state, pt, ar, ca) = join_war_scene();
    let mut data = game_data().clone();
    data.ai_diplomacy.ai_pacts.join_war_losing_ratio = 1e9;
    let cache = PlanCache::new(&state);
    let period = data.ai_diplomacy.ai_pacts.join_war_period;
    let asked: Vec<u32> = (0..period)
        .filter(|slot| join_requests(&plan_pacts(&cache, &data, &pt, *slot), &ar, &ca) > 0)
        .collect();
    assert_eq!(asked.len(), 1, "one slot in {period}: {asked:?}");
    // The request is signed by the ally: the war is joined.
    let mut played = state.clone();
    played
        .propose(
            &data,
            &pt,
            &ar,
            Treaty::single(Article::JoinWar {
                giver: Party::Recipient,
                target: ca.clone(),
            }),
        )
        .unwrap();
    assert!(played.is_at_war(&ar, &ca));
}

#[test]
fn nobody_asks_an_ally_who_would_refuse_or_when_disabled() {
    let (mut state, pt, ar, ca) = join_war_scene();
    let mut data = game_data().clone();
    data.ai_diplomacy.ai_pacts.join_war_losing_ratio = 1e9;
    assert!(join_requests(&all_orders(&state, &data, &pt), &ar, &ca) > 0);
    // A broke ally cannot join: not asked.
    state.factions.get_mut(&ar).unwrap().treasury = -10;
    assert_eq!(join_requests(&all_orders(&state, &data, &pt), &ar, &ca), 0);
    state.factions.get_mut(&ar).unwrap().treasury = 50_000;
    // Off by data.
    data.ai_diplomacy.ai_pacts.enabled = false;
    assert!(all_orders(&state, &data, &pt).is_empty());
}

#[test]
fn a_winning_faction_without_common_enemy_asks_nobody() {
    let (mut state, pt, ar, ca) = join_war_scene();
    let data = game_data();
    // Aragon bears no grudge, Portugal wins.
    state.factions.get_mut(&ar).unwrap().modifiers.clear();
    let mut calm_data = data.clone();
    calm_data.ai_diplomacy.ai_pacts.join_war_losing_ratio = 0.0;
    let orders = all_orders(&state, &calm_data, &pt);
    let asked = join_requests(&orders, &ar, &ca);
    let cache = PlanCache::new(&state);
    let common = cache.rivals(&ar).contains(&ca)
        || cache.attitude(&calm_data, &ar, &ca).0 < 0
        || cache.neighbour_factions(&calm_data, &ar).contains(&ca);
    assert_eq!(asked > 0, common);
}

#[test]
fn pact_proposals_respect_grudges_and_the_cap() {
    let mut data = game_data().clone();
    data.ai_diplomacy.ai_pacts.pact_weak_ratio = 1e9; // everyone counts as weak
    let state = start(32);
    let player = state.player_faction.clone();
    let mut proposals = 0;
    for faction in state.factions.keys() {
        if faction.is_rebels() || faction == &player {
            continue;
        }
        let cache = PlanCache::new(&state);
        for order in all_orders(&state, &data, faction) {
            let Order::ProposeTreaty { target, articles } = order else {
                continue;
            };
            let [Article::NonAggression { turns }] = articles.as_slice() else {
                continue;
            };
            proposals += 1;
            assert_eq!(*turns, data.ai_diplomacy.ai_pacts.pact_turns);
            assert!(cache.are_neighbors(&data, faction, &target));
            assert!(!state.is_allied(faction, &target) && !state.is_at_war(faction, &target));
            assert!(!claim_stakes(&state, faction, &target).any());
            assert!(!claim_stakes(&state, &target, faction).any());
            assert!(!cache.rivals(faction).contains(&target));
        }
    }
    assert!(proposals > 0, "someone proposes a pact");
    // At the cap, nobody proposes.
    data.ai_diplomacy.ai_pacts.pact_max_active = 0;
    let none = state
        .factions
        .keys()
        .filter(|f| !f.is_rebels())
        .flat_map(|f| all_orders(&state, &data, f))
        .filter(|o| matches!(o, Order::ProposeTreaty { articles, .. } if matches!(articles.as_slice(), [Article::NonAggression { .. }])))
        .count();
    assert_eq!(none, 0);
}

#[test]
fn the_ai_answers_the_players_pact_by_its_grudges() {
    let data = game_data();
    let mut state = start(33);
    let (fr, ar) = (fac("fac_france"), fac("fac_aragon"));
    let pact = vec![Article::NonAggression { turns: 12 }];
    goodwill(&mut state, "fac_aragon", "fac_france", 40);
    assert!(evaluate_treaty(&state, data, &fr, &ar, &pact).accept);
    goodwill(&mut state, "fac_aragon", "fac_france", -120);
    assert!(!evaluate_treaty(&state, data, &fr, &ar, &pact).accept);
}

#[test]
fn the_ai_answers_the_players_join_war_by_the_odds() {
    let data = game_data();
    let (mut state, pt, _ar, ca) = join_war_scene();
    // France (the player) asks Portugal to join its war on Castile.
    let fr = fac("fac_france");
    state.factions.get_mut(&fr).unwrap().at_war_with.insert(ca.clone());
    state.factions.get_mut(&ca).unwrap().at_war_with.insert(fr.clone());
    state.factions.get_mut(&pt).unwrap().at_war_with.remove(&ca);
    state.factions.get_mut(&ca).unwrap().at_war_with.remove(&pt);
    let join = vec![Article::JoinWar {
        giver: Party::Recipient,
        target: ca.clone(),
    }];
    state.factions.get_mut(&pt).unwrap().treasury = 50_000;
    state.factions.get_mut(&pt).unwrap().ledger.weariness = 0;
    goodwill(&mut state, "fac_portugal", "fac_france", 60);
    goodwill(&mut state, "fac_portugal", "fac_castile", -60);
    let eager = evaluate_treaty(&state, data, &fr, &pt, &join);
    goodwill(&mut state, "fac_portugal", "fac_france", -200);
    goodwill(&mut state, "fac_portugal", "fac_castile", 200);
    let cold = evaluate_treaty(&state, data, &fr, &pt, &join);
    assert!(eager.score > cold.score);
    assert!(!cold.accept);
}
