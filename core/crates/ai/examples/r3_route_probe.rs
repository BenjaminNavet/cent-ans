//! Lot R3: grid paths of a few known marches, to compare the navigation grid
//! before and after the historical forests of R1 entered it.
//!
//! Every faction is at peace (enemy places would block the path); the French
//! main army is teleported to each origin. Per route: straight-line distance,
//! path cost in plain kilometres, path length in cells, share of the path in
//! slow cells (cost above a plain, roads excluded), turns in summer and in
//! winter (`ceil(cost / allowance)`).
//!
//! Usage: `cargo run --release -p ai --example r3_route_probe`
use std::path::Path;

use data_model::{FactionId, GameData, SettlementId, PLAIN_COST};
use sim_campaign::march::px_per_km;
use sim_campaign::{ArmyPosition, CampaignState, Season};

const ROUTES: &[(&str, &str)] = &[
    ("set_paris", "set_orleans"),
    ("set_rouen", "set_paris"),
    ("set_calais", "set_paris"),
    ("set_orleans", "set_bourges"),
    ("set_blois", "set_bourges"),
    ("set_londres", "set_dover"),
    ("set_paris", "set_reims"),
    ("set_paris", "set_bordeaux"),
];

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = FactionId::new("fac_france").unwrap();
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
    let grid = data.navgrid();
    let km_per_point = grid.cell_km / f64::from(PLAIN_COST);
    state.season = Season::Summer;
    let summer = state.army_grid_allowance(&data, &state.armies[&army_id]);
    state.season = Season::Winter;
    let winter = state.army_grid_allowance(&data, &state.armies[&army_id]);
    state.season = Season::Summer;
    println!("Allocation : été {summer}, hiver {winter}");
    println!(
        "| trajet | vol d'oiseau (km) | chemin (km de plaine) | cases | cases lentes | tours été | tours hiver |"
    );
    println!("|---|---|---|---|---|---|---|");
    for (from_id, to_id) in ROUTES {
        let from_sid = SettlementId::new(*from_id).unwrap();
        let to_sid = SettlementId::new(*to_id).unwrap();
        let from = data.settlement_point(&from_sid).unwrap();
        let to = data.settlement_point(&to_sid).unwrap();
        state.armies.get_mut(&army_id).unwrap().position = ArmyPosition::Settlement(from_sid);
        let straight =
            ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt() / px_per_km(&data);
        let name = format!("{} → {}", &from_id[4..], &to_id[4..]);
        let Some(path) = state.find_path(&data, &army_id, to) else {
            println!("| {name} | {straight:.0} | inatteignable | - | - | - | - |");
            continue;
        };
        let slow = path
            .cells
            .iter()
            .filter(|c| grid.cost(i64::from(c.x), i64::from(c.y)) > PLAIN_COST)
            .count();
        println!(
            "| {name} | {straight:.0} | {:.0} | {} | {:.0} % | {} | {} |",
            f64::from(path.cost) * km_per_point,
            path.cells.len(),
            100.0 * slow as f64 / path.cells.len().max(1) as f64,
            path.cost.div_ceil(summer.max(1)),
            path.cost.div_ceil(winter.max(1)),
        );
    }
}
