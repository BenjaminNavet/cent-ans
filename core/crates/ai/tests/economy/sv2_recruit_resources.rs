//! Lot SV2: the campaign AI prices its recruits of one turn against the
//! resource supply its earlier recruits of the same turn drew.

use std::collections::BTreeMap;

use data_model::{BuildingId, FactionId, ProvinceId, ResourceId, UnitTypeId};
use sim_campaign::CampaignState;

use data_model::test_support::game_data;

#[test]
fn later_recruits_of_a_turn_pay_the_import_of_what_earlier_ones_drew() {
    let data = game_data();
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(data, france.clone(), 3).unwrap();
    let paris = state
        .province_city_id(&ProvinceId::new("prov_ile_de_france").unwrap())
        .unwrap()
        .clone();
    let trebuchet = UnitTypeId::new("unit_trebuchet").unwrap();
    let wood = ResourceId::new("res_wood").unwrap();
    let unit = &data.unit_types[&trebuchet];
    {
        let f = state.factions.get_mut(&france).unwrap();
        if let Some(tech) = &unit.required_technology {
            f.technologies.insert(tech.clone());
        }
        f.treasury = 1_000_000;
        let settlement = state.settlements.get_mut(&paris).unwrap();
        settlement.recruit_queue.clear();
        settlement
            .buildings
            .push(BuildingId::new("bld_siege_workshop").unwrap());
    }
    let options = state.recruitable(data, &paris);
    let first = options.iter().find(|o| o.unit_type == trebuchet).unwrap();
    assert_eq!(first.import_cost, 0);

    // The planned recruits drew all the wood but one unit.
    let need = unit.cost.resources[&wood];
    let mut supply = state.free_supply(data, &france);
    let owned = supply[&wood];
    ai::campaign::draw_supply(&mut supply, &BTreeMap::from([(wood.clone(), owned - 1)]));
    let repriced = ai::campaign::reprice_recruits(&state, data, &france, &paris, &options, &supply);
    let next = repriced.iter().find(|o| o.unit_type == trebuchet).unwrap();
    assert_eq!(next.imported.get(&wood), Some(&(need - 1)));
    assert!(next.import_cost > 0);
    assert_eq!(next.cost, first.cost + next.import_cost);

    // Units without resources keep their price.
    for (before, after) in options.iter().zip(&repriced) {
        if before.resources.is_empty() {
            assert_eq!(before.cost, after.cost);
        }
    }
}
