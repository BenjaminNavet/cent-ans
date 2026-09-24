//! Plays N turns (default 100) with the strategic AI for every faction and
//! reports wars, captures, treasuries and the order refusal rate (M9 probe).
//! Usage: `ai_probe [seed] [turns]`.
use std::collections::BTreeMap;
use std::path::Path;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, EventKind};

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(7);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(100);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    // The player faction also plays with the AI here.
    let player = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(&data, player.clone(), seed).expect("state");
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let (mut issued, mut refused) = (0u32, 0u32);
    let mut refusals: BTreeMap<String, u32> = BTreeMap::new();
    for turn in 0..turns {
        if std::env::var("TRACE").is_ok() && turn % 24 == 0 {
            for id in ["fac_england", "fac_france"] {
                let f = &state.factions[&data_model::FactionId::new(id).unwrap()];
                let garrison: usize = state
                    .settlements
                    .values()
                    .filter(|s| s.controller.as_str() == id)
                    .map(|s| s.garrison.len())
                    .sum();
                let field: usize = state
                    .armies
                    .values()
                    .filter(|a| a.faction.as_str() == id)
                    .map(|a| a.units.len())
                    .sum();
                println!("TRACE {} {id} treasury {} income {} upkeep {} army {} build {} garrison {garrison} field {field} tax {:?} suzerain {:?} war {:?}", state.date_label(), f.treasury, f.income_last_turn, f.upkeep_last_turn, f.army_upkeep_last_turn, f.building_upkeep_last_turn, f.tax_rate, f.suzerain, f.at_war_with);
            }
        }
        for order in ai::plan_turn(&state, &data, &player) {
            issued += 1;
            let label = format!("{order:?}")
                .split([' ', '{'])
                .next()
                .unwrap_or("?")
                .to_owned();
            if let Err(error) = state.submit_order(&data, order) {
                refused += 1;
                *refusals.entry(format!("{label}: {error}")).or_default() += 1;
            }
        }
        for event in state.end_turn_with(&data, ai::plan_turn) {
            let key = if event.kind == EventKind::Bankruptcy {
                format!("Bankruptcy {:?}", event.faction)
            } else {
                format!("{:?}", event.kind)
            };
            *counts.entry(key).or_default() += 1;
            if matches!(
                event.kind,
                EventKind::ProvinceCaptured | EventKind::WarDeclared | EventKind::PeaceSigned
            ) {
                println!(
                    "{:>16} {:?}: {}",
                    state.date_label(),
                    event.kind,
                    event.text_fr
                );
            }
        }
    }
    println!("\n{} — events {counts:?}", state.date_label());
    println!(
        "France orders: {issued} issued, {refused} refused ({:.1} %)",
        100.0 * f64::from(refused) / f64::from(issued.max(1))
    );
    let mut top: Vec<_> = refusals.into_iter().collect();
    top.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    for (reason, n) in top.iter().take(8) {
        println!("  {n:4} × {reason}");
    }
    for (id, f) in &state.factions {
        if f.alive {
            let provinces = state.controlled_provinces(id).len();
            let armies = state.armies.values().filter(|a| &a.faction == id).count();
            println!(
                "{id:14} treasury {:>8} income {:>6} upkeep {:>6} (army {:>6}, build {:>5}) provinces {provinces:3} armies {armies:2} power {:>6.0} war {:?}",
                f.treasury, f.income_last_turn, f.upkeep_last_turn, f.army_upkeep_last_turn, f.building_upkeep_last_turn, state.faction_power(id), f.at_war_with
            );
        }
    }
}
