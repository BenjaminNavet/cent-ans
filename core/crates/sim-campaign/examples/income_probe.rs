//! Prints France's projected income over 24 turns (balance probe).
use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;
use std::path::Path;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(&data, france.clone(), 7).expect("state");
    for turn in 0..24 {
        let eco = state.faction_economy(&data, &france).unwrap();
        let city = state
            .province_city(
                &data,
                &data_model::ProvinceId::new("prov_ile_de_france").unwrap(),
            )
            .unwrap();
        let peasants = &city.classes.peasants;
        if turn % 4 == 0 {
            println!(
                "turn {turn:2} {} income={} projected={} treasury={} | paris peasants wealth={} unrest={} health={}",
                state.date_label(), eco.income, eco.projected_income, eco.treasury,
                peasants.wealth, peasants.unrest, peasants.health
            );
        }
        state.end_turn(&data);
    }
}
