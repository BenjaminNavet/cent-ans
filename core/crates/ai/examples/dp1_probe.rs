//! Lot DP1 probe: follows the France-England war of one campaign (seed,
//! turns) and prints, while they are at peace, why England does not
//! declare war (weariness, treasury, fronts, power ratio, attitude, truce),
//! plus every treaty France or England signs.
//!
//! Usage: `cargo run --release -p ai --example dp1_probe -- [turns] [seed]`.
use std::path::Path;

use data_model::{FactionId, GameData};
use sim_campaign::diplomacy::{claim_stakes, war_ready};
use sim_campaign::{CampaignState, EventKind};

fn id(s: &str) -> FactionId {
    FactionId::new(s).expect("id")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(200);
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(1);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = id("fac_france");
    let england = id("fac_england");
    let mut state = CampaignState::new_1337(&data, france.clone(), seed).expect("state");
    let mut war_turns = 0;
    let mut changes = 0;
    let owners = |s: &CampaignState| -> Vec<Option<FactionId>> {
        s.provinces
            .keys()
            .map(|p| s.province_owner(p).cloned())
            .collect()
    };
    let mut before = owners(&state);
    for _ in 0..turns {
        let offers = state.factions[&france].offers.clone();
        for offer in offers {
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
                sim_campaign::Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
        for order in ai::plan_turn(&state, &data, &france) {
            let _ = state.submit_order(&data, order);
        }
        let events = state.end_turn_with(&data, ai::plan_turn);
        for event in &events {
            let text = &event.text_fr;
            let ours = text.contains("France") && text.contains("Angleterre");
            let treaty = event.kind == EventKind::Diplomacy && text.starts_with("Traité");
            if ours && (treaty || matches!(event.kind, EventKind::WarDeclared)) {
                println!("{:>16} {}", state.date_label(), text);
            }
        }
        let now = owners(&state);
        changes += now.iter().zip(&before).filter(|(a, b)| a != b).count();
        before = now;
        if state.is_at_war(&france, &england) {
            war_turns += 1;
        } else if state.turn.is_multiple_of(8) {
            let en = &state.factions[&england];
            let enemies: f64 = en
                .at_war_with
                .iter()
                .filter(|e| e.as_str() != "fac_rebels")
                .filter(|e| state.are_neighbors(&data, &england, e))
                .map(|e| state.faction_power(e))
                .sum();
            println!(
                "{:>16}   EN treasury {} income {} upkeep {} regency {} weary {} ready {} front {:.2} ratio {:.2} att {} truce {} throne {} wars {:?}",
                state.date_label(),
                en.treasury,
                en.income_last_turn,
                en.upkeep_last_turn,
                en.regency,
                en.ledger.weariness,
                war_ready(&state, &england),
                enemies / state.faction_power(&england).max(1.0),
                state.coalition_power(&england) / state.faction_power(&france).max(1.0),
                state.attitude(&data, &england, &france).0,
                state.has_truce(&england, &france),
                claim_stakes(&state, &england, &france).throne,
                en.at_war_with.iter().map(|e| e.as_str()).collect::<Vec<_>>(),
            );
        }
    }
    println!(
        "war FR-EN {} % of {turns} turns, {changes} province owner changes",
        war_turns * 100 / turns.max(1)
    );
}
