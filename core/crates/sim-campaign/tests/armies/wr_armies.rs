//! Lot WR armies (ADR 0305): committed share of far reinforcements (curve,
//! forecast) and free characters sent to another province.
use data_model::test_support::{fac, game_data, prov};
use sim_campaign::test_support::main_army;
use sim_campaign::{movement, CampaignState, Order, OrderError, Season};

fn start() -> CampaignState {
    let mut state = CampaignState::new_1337(game_data(), fac("fac_france"), 5).unwrap();
    state.chronicle.disabled = true;
    state.season = Season::Summer;
    state
}

#[test]
fn committed_share_is_full_then_falls_to_the_floor_then_stops() {
    let data = game_data();
    let rules = data.free_movement_rules();
    assert!(rules.reinforce_full_radius_km > rules.engage_radius_km);
    assert_eq!(movement::committed_percent_at(data, 0.0), 100);
    assert_eq!(
        movement::committed_percent_at(data, rules.reinforce_full_radius_km),
        100
    );
    let mid = (rules.reinforce_full_radius_km + rules.reinforce_radius_km) / 2.0;
    let share = movement::committed_percent_at(data, mid);
    assert!(f64::from(share) < 100.0 && f64::from(share) > rules.reinforce_min_percent);
    assert_eq!(
        f64::from(movement::committed_percent_at(
            data,
            rules.reinforce_radius_km
        )),
        rules.reinforce_min_percent
    );
}

/// A free adult commander of France and a French province other than his.
fn free_frenchman(state: &mut CampaignState) -> (data_model::CharacterId, data_model::ProvinceId) {
    let year = state.year();
    let id = state
        .characters
        .iter()
        .filter(|(_, c)| c.faction == fac("fac_france") && c.alive && c.is_major(year))
        .map(|(id, _)| id.clone())
        .next()
        .expect("a french adult");
    let c = state.characters.get_mut(&id).unwrap();
    c.army = None;
    c.governor_of = None;
    c.journey = None;
    let from = c.location.clone().expect("located");
    let to = state
        .provinces
        .keys()
        .find(|p| state.holds_province(&fac("fac_france"), p) && **p != from)
        .cloned()
        .unwrap();
    (id, to)
}

#[test]
fn a_sent_character_arrives_after_the_trip_and_cannot_lead_meanwhile() {
    let data = game_data();
    let mut state = start();
    let (id, to) = free_frenchman(&mut state);
    let france = fac("fac_france");
    state
        .apply_order(
            data,
            &france,
            Order::SendCharacter {
                character: id.clone(),
                to: to.clone(),
            },
        )
        .unwrap();
    let journey = state.characters[&id].journey.clone().expect("on the road");
    assert!(journey.turns_left >= 1);
    // En route: cannot be named general, nor sent again.
    let army = main_army(&state, "fac_france");
    assert_eq!(
        state.apply_order(
            data,
            &france,
            Order::AssignGeneral {
                army,
                character: id.clone()
            }
        ),
        Err(OrderError::CharacterBusy)
    );
    assert!(matches!(
        state.apply_order(
            data,
            &france,
            Order::SendCharacter {
                character: id.clone(),
                to: to.clone()
            }
        ),
        Err(OrderError::CharacterBusy)
    ));
    for _ in 0..journey.turns_left {
        state.end_turn(data);
    }
    let c = &state.characters[&id];
    if c.alive {
        assert!(c.journey.is_none());
        assert_eq!(c.location.as_ref(), Some(&to));
    }
}

#[test]
fn a_far_province_takes_longer_and_a_foreign_one_is_refused() {
    let data = game_data();
    let mut state = start();
    let (id, _) = free_frenchman(&mut state);
    let from = state.characters[&id].location.clone().unwrap();
    let near = sim_campaign::char_travel::travel_turns(data, &from, &from);
    assert_eq!(near, 1);
    let england = prov("prov_kent");
    if !state.holds_province(&fac("fac_france"), &england) {
        assert_eq!(
            state.apply_order(
                data,
                &fac("fac_france"),
                Order::SendCharacter {
                    character: id,
                    to: england
                }
            ),
            Err(OrderError::NotYourProvince(fac("fac_france")))
        );
    }
}
