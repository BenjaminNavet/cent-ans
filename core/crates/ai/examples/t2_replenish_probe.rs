//! Lot TW2-T2 (ADR 0102): army sizes of the AI over N turns with and without
//! the replenishment and recruitment pools. Usage:
//! `t2_replenish_probe [seed] [turns] [on|off]` — `off` zeroes the
//! replenishment rates and makes the pools bottomless (the rules before T2).
use std::path::Path;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(7);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(20);
    let on = args.next().is_none_or(|s| s != "off");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (mut data, _) = GameData::load(&root).expect("data");
    if !on {
        let rules = &mut data.replenishment_rules;
        let t = &mut rules.replenishment.territory_percent;
        t.own = 0;
        t.ally = 0;
        t.neutral = 0;
        let cap = &mut rules.recruit_pool.base_cap;
        cap.city = 1000;
        cap.town = 1000;
        cap.castle = 1000;
        cap.abbey = 1000;
        cap.village = 1000;
    }
    let majors =
        ["fac_france", "fac_england", "fac_burgundy"].map(|id| FactionId::new(id).expect("id"));
    // Every major is played by the AI: the Papacy watches.
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_papacy").unwrap(), seed).unwrap();
    println!(
        "seed {seed}, {turns} turns, T2 {}",
        if on { "on" } else { "off" }
    );
    let mut totals = [0u64; 3];
    for turn in 0..=turns {
        if turn > 0 {
            state.end_turn_with(&data, ai::plan_turn);
        }
        let mut line = format!("{:>3} {:>16}", turn, state.date_label());
        for (index, faction) in majors.iter().enumerate() {
            let armies: Vec<_> = state
                .armies
                .values()
                .filter(|a| &a.faction == faction)
                .collect();
            let men: u32 = armies.iter().map(|a| a.total_strength()).sum();
            let max: u32 = armies
                .iter()
                .flat_map(|a| &a.units)
                .map(|u| u.max_strength)
                .sum();
            let units: usize = armies.iter().map(|a| a.units.len()).sum();
            let treasury = state.factions.get(faction).map_or(0, |f| f.treasury);
            totals[index] += u64::from(men);
            line.push_str(&format!(
                " | {} {:>2}a {:>3}u {:>6}/{:<6} {:>7}£",
                &faction.as_str()[4..7],
                armies.len(),
                units,
                men,
                max,
                treasury
            ));
        }
        if turn % 4 == 0 || turn == turns {
            println!("{line}");
        }
    }
    let n = u64::from(turns + 1);
    println!(
        "mean field men: FRA {} ENG {} BUR {}",
        totals[0] / n,
        totals[1] / n,
        totals[2] / n
    );
}
