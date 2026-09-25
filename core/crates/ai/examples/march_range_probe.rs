//! Lot M5b: how many turns the French main army needs, on the navigation
//! grid, to march from Paris to a few cities (free movement, spec
//! 2026-09-24-mouvement-libre § 3.2).
//!
//! Every faction is at peace (enemy places would block the path), the army
//! starts in Paris with its full allowance. Per destination: straight-line
//! distance, grid path cost in plain kilometres, and turns in summer and in
//! winter (`ceil(cost / allowance)`).
//!
//! Usage: `cargo run --release -p ai --example march_range_probe`
use std::path::Path;

use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::march::px_per_km;
use sim_campaign::{ArmyPosition, CampaignState, Season};

const DESTINATIONS: &[&str] = &[
    "set_orleans",
    "set_reims",
    "set_rouen",
    "set_calais",
    "set_tours",
    "set_dijon",
    "set_poitiers",
    "set_lyon",
    "set_bordeaux",
    "set_toulouse",
    "set_bayonne",
];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = FactionId::new("fac_france").unwrap();
    let paris = SettlementId::new("set_paris").unwrap();
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    for faction in state.factions.values_mut() {
        faction.at_war_with.clear();
    }
    let army_id = state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == france)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("French army");
    state.armies.get_mut(&army_id).unwrap().position = ArmyPosition::Settlement(paris.clone());
    let km_per_point = data.navgrid().cell_km / f64::from(data_model::PLAIN_COST);
    let allowance = |state: &mut CampaignState, season: Season| {
        state.season = season;
        let army = &state.armies[&army_id];
        state.army_grid_allowance(&data, army)
    };
    let summer = allowance(&mut state, Season::Summer);
    let winter = allowance(&mut state, Season::Winter);
    state.season = Season::Summer;
    println!(
        "Allocation : été {summer} ({:.0} km de plaine), hiver {winter} ({:.0} km)",
        f64::from(summer) * km_per_point,
        f64::from(winter) * km_per_point
    );
    println!(
        "| destination | à vol d'oiseau (km) | chemin (km de plaine) | tours été | tours hiver |"
    );
    println!("|---|---|---|---|---|");
    let from = data.settlement_point(&paris).unwrap();
    for destination in DESTINATIONS {
        let id = SettlementId::new(*destination).unwrap();
        let to = data.settlement_point(&id).unwrap();
        let straight =
            ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt() / px_per_km(&data);
        let Some(path) = state.find_path(&data, &army_id, to) else {
            println!("| {destination} | {straight:.0} | inatteignable | - | - |");
            continue;
        };
        println!(
            "| {destination} | {straight:.0} | {:.0} | {} | {} |",
            f64::from(path.cost) * km_per_point,
            path.cost.div_ceil(summer.max(1)),
            path.cost.div_ceil(winter.max(1)),
        );
    }
}
