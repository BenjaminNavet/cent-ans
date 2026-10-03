//! Lot LR-07: plays `turns` AI turns then prints, per faction size, the
//! weighted garrison of each province, its population and its weighted
//! unrest (how much the garrison soothes, who would revolt).
//!
//! Usage: `lr07_garrison_probe [turns] [seed]`.
use std::collections::BTreeMap;

use data_model::{FactionId, GameData, SocialClass};
use sim_campaign::CampaignState;

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(40);
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(1);
    let idle = FactionId::new("fac_papacy").expect("papacy");
    let mut state = CampaignState::new_1337(&data, idle, seed).expect("setup");
    state.interactive_battles = false;
    for _ in 0..turns {
        state.end_turn_with(&data, ai::plan_turn);
    }
    let mut size: BTreeMap<FactionId, usize> = BTreeMap::new();
    for p in state.provinces.values() {
        *size
            .entry(state.settlements[&p.city].controller.clone())
            .or_default() += 1;
    }
    // bucket -> (count, men, pop, unrest, provinces above 60)
    let mut buckets: BTreeMap<&str, (usize, f64, f64, f64, usize, Vec<u32>)> = BTreeMap::new();
    for (id, p) in &state.provinces {
        let ctrl = &state.settlements[&p.city].controller;
        let n = size.get(ctrl).copied().unwrap_or(0);
        let bucket = match n {
            0..=2 => "a 1-2",
            3..=6 => "b 3-6",
            7..=15 => "c 7-15",
            _ => "d 16+",
        };
        let pop = &p.population;
        let total = pop.total().max(1) as f64;
        let weighted = SocialClass::ALL
            .iter()
            .map(|c| f64::from(pop.get(*c).unrest) * pop.get(*c).count as f64)
            .sum::<f64>()
            / total;
        let men = state.weighted_garrison_strength(&data, id);
        let e = buckets.entry(bucket).or_default();
        e.0 += 1;
        e.1 += f64::from(men);
        e.2 += total;
        e.3 += weighted;
        if weighted > 60.0 {
            e.4 += 1;
        }
        e.5.push(men);
    }
    println!("turn {} seed {seed}", state.turn);
    println!("| taille | provinces | garnison moy. (h.) | médiane | p90 | pop. moy. | trouble moy. | > 60 |");
    for (b, (n, men, pop, unrest, hot, mut list)) in buckets {
        list.sort_unstable();
        let nf = n as f64;
        println!(
            "| {b} | {n} | {:.0} | {} | {} | {:.0} | {:.1} | {hot} |",
            men / nf,
            list[list.len() / 2],
            list[list.len() * 9 / 10],
            pop / nf,
            unrest / nf
        );
    }
}
