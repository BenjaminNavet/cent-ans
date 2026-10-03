//! Lot LR-15: per-province breakdown of the starting budget of the realms
//! still in structural deficit (receipts, city and secondary buildings,
//! garrisons). `LR15_FACTIONS=fac_a,fac_b cargo test -p sim-campaign --test
//! lr15_budget_breakdown -- --ignored --nocapture`.

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementKind};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
#[ignore = "diagnostic table of the starting budgets, province by province"]
fn starting_budget_breakdown() {
    use sim_campaign::buildings::province_building_upkeep;
    use sim_campaign::economy::{building_upkeep_percent, garrison_upkeep_percent, unit_upkeep};
    let data = data();
    let state =
        CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 1).expect("start");
    let watched = std::env::var("LR15_FACTIONS").unwrap_or_else(|_| {
        "fac_marinids,fac_hafsids,fac_lithuania,fac_serbia,fac_poland,fac_aragon".into()
    });
    for raw in watched.split(',') {
        let id = FactionId::new(raw).unwrap();
        let f = &state.factions[&id];
        let tech = sim_campaign::research::faction_province_tech_effects(&state, &data, &id);
        println!("== {id}");
        println!("province | gross | city bld | other bld | city garr | other garr | places");
        let mut totals = [0i64; 5];
        for p in state.controlled_provinces(&id) {
            let gross = state.province_gross_income(&data, &p, &id, f.tax_rate, &tech);
            let mut row = [gross, 0, 0, 0, 0];
            let mut places = 0;
            for (_, s) in state.settlements_of(&p).filter(|(_, s)| s.controller == id) {
                places += 1;
                let city = s.kind == SettlementKind::City;
                let bld = province_building_upkeep(&data, &s.buildings)
                    * building_upkeep_percent(&data, s.kind)
                    / 100;
                let garr: i64 = s
                    .garrison
                    .iter()
                    .map(|u| unit_upkeep(&data, u))
                    .sum::<i64>()
                    * garrison_upkeep_percent(&data, s.kind)
                    / 100;
                row[if city { 1 } else { 2 }] += bld;
                row[if city { 3 } else { 4 }] += garr;
            }
            for (t, v) in totals.iter_mut().zip(row) {
                *t += v;
            }
            println!(
                "{p} | {} | {} | {} | {} | {} | {places}",
                row[0], row[1], row[2], row[3], row[4]
            );
        }
        println!(
            "TOTAL | {} | {} | {} | {} | {}",
            totals[0], totals[1], totals[2], totals[3], totals[4]
        );
    }
}
