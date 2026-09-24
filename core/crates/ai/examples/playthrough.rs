//! F9 acceptance: a full campaign played to its end year for each playable
//! faction, the "player" driven by the strategic AI (answering offers as the
//! AI would). Checks that nothing panics, orders are accepted and the
//! campaign reaches an outcome.
//!
//! `cargo run --release -p ai --example playthrough [seed]`

use std::time::Instant;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, Order};

const PLAYABLE: &[&str] = &["fac_france", "fac_england", "fac_burgundy"];

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1337);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    println!("faction        turns  end date          outcome   score  provinces  orders refused  seconds");
    for raw in PLAYABLE {
        let player = FactionId::new(*raw).expect("id");
        play(&data, player, seed);
    }
}

fn play(data: &GameData, player: FactionId, seed: u64) {
    let started = Instant::now();
    let mut state = CampaignState::new_1337(data, player.clone(), seed).expect("state");
    let (mut issued, mut refused) = (0u32, 0u32);
    // Burgundy's deadline is 1477: 560 seasons; stop at the outcome or 600.
    for _ in 0..600 {
        for offer in state.factions[&player].offers.clone() {
            let accept = sim_campaign::diplomacy::evaluate(
                &state,
                data,
                &offer.from,
                &player,
                &offer.proposal,
            )
            .accept;
            let _ = state.submit_order(
                data,
                Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
        for order in ai::plan_turn(&state, data, &player) {
            issued += 1;
            if state.submit_order(data, order).is_err() {
                refused += 1;
            }
        }
        state.end_turn_with(data, ai::plan_turn);
        if state.outcome.is_some() {
            break;
        }
    }
    let provinces = state
        .provinces
        .values()
        .filter(|p| p.owner == player)
        .count();
    let (kind, score) = state.outcome.as_ref().map_or(
        ("aucune".to_owned(), state.campaign_score(data, &player)),
        |o| (format!("{:?}", o.kind), o.score),
    );
    println!(
        "{:<14} {:>5}  {:<16}  {:<8} {:>6}  {:>9}  {:>6} {:>7}  {:>7.1}",
        player.as_str(),
        state.turn(),
        state.date_label(),
        kind,
        score,
        provinces,
        issued,
        refused,
        started.elapsed().as_secs_f64()
    );
}
