//! LR-11 diagnostic: why long wars against a realm down to its last
//! bastions (F4: never besieged without a claim) never end in peace.
//! Plays `turns` AI turns (seed), then prints, for every war of an AI faction
//! whose enemy holds at most `ai::campaign::LAST_BASTIONS` provinces, both
//! sides' view of a white peace and what `plan_peace` would send.
//!
//! Usage: `lr11_peace_probe [turns] [seed]` (default 24, 1).
use std::path::Path;

use data_model::{FactionId, GameData};
use sim_campaign::negotiation::{evaluate_treaty, plan_peace, Article};
use sim_campaign::{CampaignState, Order};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let turns: u32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(24);
    let seed: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(1);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = FactionId::new("fac_france").expect("id");
    let mut state = CampaignState::new_1337(&data, france.clone(), seed).expect("state");
    state.interactive_battles = false;
    for _ in 0..turns {
        for offer in state.factions[&france].offers.clone() {
            let accept = sim_campaign::diplomacy::evaluate(
                &state,
                &data,
                &offer.from,
                &france,
                &offer.proposal,
            )
            .accept;
            let _ = state.submit_order(
                &data,
                Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
        for order in ai::plan_turn(&state, &data, &france) {
            let _ = state.submit_order(&data, order);
        }
        state.end_turn_with(&data, ai::plan_turn);
    }
    println!("turn {}", state.turn);
    let held = |f: &FactionId| {
        state
            .provinces
            .keys()
            .filter(|p| state.holds_province(f, p))
            .count()
    };
    let white = vec![Article::Peace];
    for (id, f) in &state.factions {
        if !f.alive || id.as_str() == "fac_rebels" {
            continue;
        }
        for enemy in f.at_war_with.iter().filter(|e| e.as_str() != "fac_rebels") {
            if held(enemy) > ai::campaign::LAST_BASTIONS {
                continue;
            }
            let ours = evaluate_treaty(&state, &data, enemy, id, &white);
            let theirs = evaluate_treaty(&state, &data, id, enemy, &white);
            println!(
                "{id} ({} prov, power {:.0}) vs {enemy} ({} prov, power {:.0}) since t{} score {} weariness {}/{}",
                held(id),
                state.faction_power(id),
                held(enemy),
                state.faction_power(enemy),
                f.war_started.get(enemy).copied().unwrap_or(0),
                state.war_score(&data, id, enemy),
                f.ledger.weariness,
                state.factions[enemy].ledger.weariness,
            );
            println!(
                "  we accept white: {} {:?} blocked {:?}",
                ours.chance, ours.context, ours.blocked
            );
            println!(
                "  they accept white: {} {:?} blocked {:?}",
                theirs.chance, theirs.context, theirs.blocked
            );
            println!(
                "  plan_peace us: {:?}",
                plan_peace(&state, &data, id).map(|o| format!("{o:?}"))
            );
            println!(
                "  plan_peace them: {:?}",
                plan_peace(&state, &data, enemy).map(|o| format!("{o:?}"))
            );
        }
    }
}
