//! IA night: replays the campaign like `turn_perf` up to `turn`, then plays
//! `faction`'s turn `reps` times on copies (planning alone, then planning and
//! execution), to profile one slow faction turn (`sample <pid>` on macOS).
//!
//! Usage: `turn_hotspot <seed> <turn> <faction> [reps]`.
use std::path::Path;
use std::time::Instant;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = args[0].parse().expect("seed");
    let target_turn: u32 = args[1].parse().expect("turn");
    let target = FactionId::new(&args[2]).expect("faction id");
    let reps: u32 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(20);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("game data loads").0;
    let _ = data.navgrid().component(0, 0);
    let france = FactionId::new("fac_france").expect("id");
    let mut state = CampaignState::new_1337(&data, france.clone(), seed).expect("state");
    state.interactive_battles = false;
    let planner = ai::plan_turn_sequential;
    for turn in 0..=target_turn {
        for order in ai::plan_turn(&state, &data, &france) {
            let _ = state.submit_order(&data, order);
        }
        let mut events = Vec::new();
        let factions: Vec<FactionId> = state
            .factions
            .iter()
            .filter(|(id, f)| f.alive && **id != france)
            .map(|(id, _)| id.clone())
            .collect();
        for faction in factions {
            if turn == target_turn && faction == target {
                let started = Instant::now();
                for _ in 0..reps {
                    let one = Instant::now();
                    std::hint::black_box(planner(&state, &data, &faction));
                    println!("  plan {:.2} ms", one.elapsed().as_secs_f64() * 1000.0);
                }
                let plan_ms = started.elapsed().as_secs_f64() * 1000.0 / reps as f64;
                let started = Instant::now();
                for _ in 0..reps {
                    let mut copy = state.clone();
                    let mut scratch = Vec::new();
                    copy.play_ai_turn(&data, &faction, &planner, &mut scratch);
                }
                let play_ms = started.elapsed().as_secs_f64() * 1000.0 / reps as f64;
                let orders = planner(&state, &data, &faction);
                println!(
                    "{faction} seed {seed} turn {turn}: plan {plan_ms:.2} ms, plan+play {play_ms:.2} ms (incl. clone), {} orders",
                    orders.len()
                );
                return;
            }
            state.play_ai_turn(&data, &faction, &planner, &mut events);
        }
        state.resolve_end_of_turn(&data, &mut events);
    }
}
