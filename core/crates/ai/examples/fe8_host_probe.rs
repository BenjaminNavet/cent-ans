//! FE8 (`docs/wip/fe8-equilibre.md`): for a few suzerains, lists their direct
//! vassals with power, loyalty and distances (to the suzerain's lands and to
//! France's), at the start and after `turns` AI turns.
//!
//! Usage: `fe8_host_probe [turns] [seed]`.
use std::path::Path;

use data_model::{movement_graph::distance_km, FactionId, GameData};
use sim_campaign::CampaignState;

fn id(s: &str) -> FactionId {
    FactionId::new(s).expect("well-formed id")
}

/// Shortest distance between two factions' controlled settlements, in km.
fn gap_km(state: &CampaignState, data: &GameData, a: &FactionId, b: &FactionId) -> f64 {
    let points = |f: &FactionId| -> Vec<[f64; 2]> {
        state
            .settlements
            .iter()
            .filter(|(_, s)| &s.controller == f)
            .filter_map(|(id, _)| data.settlements.get(id).map(|d| d.lonlat))
            .collect()
    };
    let (pa, pb) = (points(a), points(b));
    pa.iter()
        .flat_map(|x| pb.iter().map(move |y| distance_km(*x, *y)))
        .fold(f64::INFINITY, f64::min)
}

fn dump(state: &CampaignState, data: &GameData) {
    let france = id("fac_france");
    let england = id("fac_england");
    println!(
        "felonies {:?}; commise ratio France/England {:.2}; France wars {:?}; commise order {:?}",
        state
            .feudal
            .felonies
            .iter()
            .map(|c| (c.vassal.as_str(), c.liege.as_str(), c.expires_turn))
            .collect::<Vec<_>>(),
        ai::feudal::commise_power_ratio(state, &france, &england),
        state.factions[&france].at_war_with,
        ai::feudal::plan_commise(state, data, &france)
    );
    for liege in [
        "fac_empire",
        "fac_france",
        "fac_england",
        "fac_aragon",
        "fac_castile",
    ] {
        let liege = id(liege);
        let power = state.faction_power(&liege);
        println!("{liege} power {power:.0}");
        for v in sim_campaign::feudal::direct_vassals(state, data, &liege) {
            let f = &state.factions[&v];
            println!(
                "   {:<18} power {:>6.0} ratio {:>5.2} loyalty {:>3} to liege {:>5.0} km to France {:>5.0} km allied {} serves vs France {}",
                v.as_str(),
                state.faction_power(&v),
                state.faction_power(&v) / power.max(1.0),
                f.loyalty,
                gap_km(state, data, &v, &liege),
                gap_km(state, data, &v, &france),
                state.is_allied(&v, &liege),
                sim_campaign::feudal::can_serve(state, data, &v, &liege, &france)
            );
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let mut state = CampaignState::new_1337(&data, id("fac_france"), seed).expect("state");
    dump(&state, &data);
    for _ in 0..turns {
        let orders = ai::plan_turn(&state, &data, &id("fac_france"));
        for order in orders {
            let _ = state.submit_order(&data, order);
        }
        state.end_turn_with(&data, ai::plan_turn);
    }
    if turns > 0 {
        println!("--- after {turns} turns");
        dump(&state, &data);
    }
}
