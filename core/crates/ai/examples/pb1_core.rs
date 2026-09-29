//! FE5: Rust share of the PB1 bench (`game/tests/pb1_turns.gd`): complete
//! ends of turn (`end_turn_with(ai::plan_turn)`, every AI faction plus the
//! end-of-turn phases) from spring 1337, France played by the same AI.
//!
//! Usage: `pb1_core [turns] [seed...]` (default 12 turns, seeds 1 2 3).
//! Prints the mean and median milliseconds per end of turn.
use std::path::Path;
use std::time::Instant;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(12);
    let mut seeds: Vec<u64> = args.iter().skip(1).filter_map(|a| a.parse().ok()).collect();
    if seeds.is_empty() {
        seeds = vec![1, 2, 3];
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("game data loads").0;
    let _ = data.navgrid().component(0, 0);
    let france = FactionId::new("fac_france").expect("id");
    let mut times: Vec<f64> = Vec::new();
    for seed in &seeds {
        let mut state = CampaignState::new_1337(&data, france.clone(), *seed).expect("state");
        state.interactive_battles = false;
        for _ in 0..turns {
            let started = Instant::now();
            for order in ai::plan_turn(&state, &data, &france) {
                let _ = state.submit_order(&data, order);
            }
            state.end_turn_with(&data, ai::plan_turn);
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let mut sorted = times.clone();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    println!(
        "PB1_CORE turns {} seeds {:?}: mean {mean:.1} ms, median {median:.1} ms, max {:.1} ms",
        times.len(),
        seeds,
        sorted[sorted.len() - 1]
    );
}
