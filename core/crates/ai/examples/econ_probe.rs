//! Lot C7a: economy of France and England by settlement kind at the 1337
//! start (places, garrison units, garrison and building upkeep), then every
//! 5 turns of AI play: income, upkeep, field army, buildings, administration.
//!
//! Usage: `cargo run --release -p ai --example econ_probe`
use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    state.interactive_battles = false;
    for f in ["fac_france", "fac_england"] {
        let f = FactionId::new(f).unwrap();
        let mut by_kind: std::collections::BTreeMap<String, (usize, usize, i64, i64)> =
            Default::default();
        for s in state.settlements.values().filter(|s| s.controller == f) {
            let e = by_kind.entry(format!("{:?}", s.kind)).or_default();
            e.0 += 1;
            e.1 += s.garrison.len();
            e.2 += s
                .garrison
                .iter()
                .map(|u| sim_campaign::economy::unit_upkeep(&data, u))
                .sum::<i64>()
                * sim_campaign::economy::garrison_upkeep_percent(&data, s.kind)
                / 100;
            e.3 += sim_campaign::buildings::province_building_upkeep(&data, &s.buildings)
                * sim_campaign::economy::building_upkeep_percent(&data, s.kind)
                / 100;
        }
        println!(
            "{} (places, units, garrison upkeep, building upkeep) {by_kind:?}",
            f.as_str()
        );
    }
    for t in 0..=20 {
        if t % 5 == 0 {
            for f in ["fac_france", "fac_england"] {
                let f = FactionId::new(f).unwrap();
                let army: i64 = state
                    .armies
                    .values()
                    .filter(|a| a.faction == f)
                    .flat_map(|a| a.units.iter())
                    .map(|u| sim_campaign::economy::unit_upkeep(&data, u))
                    .sum();
                let gar_units: usize = state
                    .settlements
                    .values()
                    .filter(|s| s.controller == f)
                    .map(|s| s.garrison.len())
                    .sum();
                let army_units: usize = state
                    .armies
                    .values()
                    .filter(|a| a.faction == f)
                    .map(|a| a.units.len())
                    .sum();
                println!(
                    "t{t} {} income {} upkeep(all) {} field-raw {} field-units {} gar-units {} buildings {} admin {} treasury {}",
                    f.as_str(),
                    state.faction_income_effective(&data, &f),
                    state.faction_upkeep(&data, &f),
                    army,
                    army_units,
                    gar_units,
                    state.faction_building_upkeep(&data, &f),
                    state.faction_administration_upkeep(&data, &f),
                    state.factions[&f].treasury
                );
            }
        }
        for o in ai::plan_turn(&state, &data, &france) {
            let _ = state.submit_order(&data, o);
        }
        state.end_turn_with(&data, ai::plan_turn);
    }
}
