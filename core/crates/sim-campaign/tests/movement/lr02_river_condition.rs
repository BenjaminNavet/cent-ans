//! LR-02: the `province_on_river` event condition (boatmen's strike only
//! fires in a province crossed by a river).

use data_model::{Condition, EventId, FactionId, ProvinceId};
use sim_campaign::{CampaignState, EventContext};

use data_model::test_support::game_data;

#[test]
fn river_condition_follows_the_province_river_list() {
    let data = game_data();
    let state = CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 1).unwrap();
    let with_river = data
        .provinces
        .values()
        .find(|p| !p.rivers.is_empty())
        .unwrap();
    let without_river = data
        .provinces
        .values()
        .find(|p| p.rivers.is_empty())
        .unwrap();
    let holds = |id: &ProvinceId| {
        let ctx = EventContext {
            faction: None,
            province: Some(id.clone()),
        };
        state.condition_holds(data, &Condition::ProvinceOnRiver { province: None }, &ctx)
    };
    assert!(holds(&with_river.id));
    assert!(!holds(&without_river.id));
    let event = data
        .events
        .get(&EventId::new("evt_bateliers_en_greve").unwrap())
        .unwrap();
    assert!(event
        .trigger
        .conditions
        .iter()
        .any(|c| matches!(c, Condition::ProvinceOnRiver { .. })));
}
