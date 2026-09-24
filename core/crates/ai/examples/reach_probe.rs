//! Lot C7a: how far an army marches in one season on the settlement graph.
//!
//! For a few cities (Paris with the French main army, London and Bordeaux
//! with the English one), at the 1337 start and in winter: settlements and
//! provinces reachable in one season (the game's Dijkstra: enemy places
//! stop the march), the median and maximum number of edges of those paths,
//! and the farthest settlement. `NEUTRAL=1` ignores enemies (a faction at
//! peace with everyone), to measure the raw reach of the graph.
//!
//! Usage: `cargo run --release -p ai --example reach_probe`
use std::collections::BTreeSet;
use std::path::Path;

use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::movement::{dijkstra, path_to};
use sim_campaign::{CampaignState, Season};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let neutral = std::env::var("NEUTRAL").is_ok();
    println!("| départ | armée | saison | points | colonies | provinces | étapes méd. | étapes max | plus loin |");
    println!("|---|---|---|---|---|---|---|---|---|");
    for (city, faction) in [
        ("set_paris", "fac_france"),
        ("set_londres", "fac_england"),
        ("set_bordeaux", "fac_england"),
    ] {
        for winter in [false, true] {
            let faction = FactionId::new(faction).unwrap();
            let mut state =
                CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 1).unwrap();
            if winter {
                state.season = Season::Winter;
            }
            if neutral {
                for f in state.factions.values_mut() {
                    f.at_war_with.clear();
                }
            }
            let city = SettlementId::new(city).unwrap();
            let army_id = state
                .armies
                .iter()
                .filter(|(_, a)| a.faction == faction)
                .max_by_key(|(_, a)| a.units.len())
                .map(|(id, _)| id.clone())
                .expect("an army");
            state.armies.get_mut(&army_id).unwrap().location = city.clone();
            let allowance = state.army_movement_allowance(&data, &state.armies[&army_id]);
            let table = dijkstra(
                &state,
                &data,
                &faction,
                &city,
                Some(allowance),
                Some(allowance),
            );
            let mut steps: Vec<(usize, SettlementId)> = table
                .keys()
                .filter(|id| **id != city)
                .map(|id| (path_to(&table, id).map_or(0, |p| p.len()), id.clone()))
                .collect();
            steps.sort();
            let provinces: BTreeSet<_> = table
                .keys()
                .filter_map(|id| state.settlement_province(id))
                .collect();
            let median = steps.get(steps.len() / 2).map_or(0, |s| s.0);
            let farthest = steps
                .iter()
                .max_by_key(|(_, id)| table[id].cost)
                .map_or("-".to_owned(), |(n, id)| {
                    format!("{} ({} pts, {n} étapes)", id.as_str(), table[id].cost)
                });
            println!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                city.as_str(),
                faction.as_str(),
                if winter { "hiver" } else { "été" },
                allowance,
                steps.len(),
                provinces.len().saturating_sub(1),
                median,
                steps.last().map_or(0, |s| s.0),
                farthest
            );
        }
    }
}
