//! Lot M8: a settlement queues up to `construction_queue_size` buildings,
//! paid when queued, started one after the other, cancellable with a refund.

use data_model::{BuildingId, FactionId, GameData, SettlementId, SettlementKind};
use sim_campaign::{CampaignState, Order, Place};

use data_model::test_support::game_data;

fn france() -> FactionId {
    FactionId::new("fac_france").unwrap()
}

/// A French town with no work under way and a rich treasury.
fn setup(data: &GameData) -> (CampaignState, SettlementId) {
    let mut state = CampaignState::new_1337(data, france(), 7).expect("1337 start");
    let town = state
        .settlements
        .iter()
        .find(|(_, s)| {
            s.kind == SettlementKind::Town && s.owner == france() && s.controller == france()
        })
        .map(|(id, _)| id.clone())
        .expect("a French town");
    let place = state.settlements.get_mut(&town).unwrap();
    place.construction = None;
    place.build_queue.clear();
    state.factions.get_mut(&france()).unwrap().treasury = 1_000_000;
    (state, town)
}

fn available(state: &CampaignState, data: &GameData, town: &SettlementId) -> Vec<BuildingId> {
    state
        .buildable(data, town)
        .into_iter()
        .filter(|o| o.available && o.cost > 0)
        .map(|o| o.building)
        .collect()
}

fn build(state: &mut CampaignState, data: &GameData, town: &SettlementId, b: &BuildingId) {
    state
        .submit_order(
            data,
            Order::Build {
                settlement: Place::Settlement(town.clone()),
                building: b.clone(),
            },
        )
        .expect("build order");
}

#[test]
fn queue_is_paid_up_front_and_limited() {
    let data = game_data();
    let (mut state, town) = setup(data);
    let size = data.economy_rules.construction_queue_size as usize;
    assert_eq!(size, 3);
    let options = available(&state, data, &town);
    assert!(
        options.len() > size,
        "enough distinct buildings to fill the queue"
    );
    let before = state.factions[&france()].treasury;
    for b in options.iter().take(size) {
        build(&mut state, data, &town, b);
    }
    let place = &state.settlements[&town];
    assert!(place.construction.is_some());
    assert_eq!(place.build_queue.len(), size - 1);
    assert!(state.factions[&france()].treasury < before);
    // Queue full: further orders are refused with a reason.
    let extra = state
        .buildable(data, &town)
        .into_iter()
        .find(|o| {
            !place
                .construction
                .iter()
                .chain(place.build_queue.iter())
                .any(|c| c.building == o.building)
        })
        .unwrap();
    assert!(!extra.available);
    assert!(extra.reason.unwrap().contains("file"));
}

#[test]
fn queued_builds_start_in_turn() {
    let data = game_data();
    let (mut state, town) = setup(data);
    let options = available(&state, data, &town);
    let (a, b) = (options[0].clone(), options[1].clone());
    build(&mut state, data, &town, &a);
    build(&mut state, data, &town, &b);
    let mut guard = 0;
    while !state.settlements[&town].buildings.contains(&a) {
        state.end_turn(data);
        guard += 1;
        assert!(guard < 50, "first construction finishes");
    }
    let place = &state.settlements[&town];
    assert_eq!(place.construction.as_ref().map(|c| &c.building), Some(&b));
    assert!(place.build_queue.is_empty());
}

#[test]
fn cancelling_refunds_half_and_promotes() {
    let data = game_data();
    let (mut state, town) = setup(data);
    let options = available(&state, data, &town);
    let (a, b) = (options[0].clone(), options[1].clone());
    build(&mut state, data, &town, &a);
    let before_b = state.factions[&france()].treasury;
    build(&mut state, data, &town, &b);
    let paid_b = state.settlements[&town].build_queue[0].paid;
    assert_eq!(
        state.factions[&france()].treasury,
        before_b - i64::from(paid_b)
    );
    state
        .submit_order(
            data,
            Order::CancelQueuedBuild {
                settlement: Place::Settlement(town.clone()),
                index: 0,
            },
        )
        .unwrap();
    assert!(state.settlements[&town].build_queue.is_empty());
    assert_eq!(
        state.factions[&france()].treasury,
        before_b - i64::from(paid_b) + i64::from(paid_b * 50 / 100)
    );
    // Queue again, then cancel the active one: the queued one starts.
    build(&mut state, data, &town, &b);
    state
        .submit_order(
            data,
            Order::CancelBuild {
                settlement: Place::Settlement(town.clone()),
            },
        )
        .unwrap();
    let place = &state.settlements[&town];
    assert_eq!(place.construction.as_ref().map(|c| &c.building), Some(&b));
    assert!(place.build_queue.is_empty());
}

#[test]
fn build_queue_survives_a_save() {
    let data = game_data();
    let (mut state, town) = setup(data);
    let options = available(&state, data, &town);
    build(&mut state, data, &town, &options[0]);
    let json = state.save_json();
    assert!(!json.contains("build_queue"), "empty queue is not written");
    let loaded = CampaignState::load_json(&json).unwrap();
    assert!(loaded.settlements[&town].build_queue.is_empty());
    build(&mut state, data, &town, &options[1]);
    let loaded = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(loaded.settlements[&town].build_queue.len(), 1);
}

#[test]
fn recruit_lines_fall_in_u13_groups() {
    use sim_campaign::RecruitGroup;
    let data = game_data();
    let (state, town) = setup(data);
    let options = state.recruitable(data, &town);
    let groups: Vec<RecruitGroup> = options.iter().map(|o| o.group()).collect();
    assert!(groups.contains(&RecruitGroup::Ready));
    for option in &options {
        let reason = option.reason.clone().unwrap_or_default();
        match option.group() {
            RecruitGroup::Ready => assert!(option.available),
            RecruitGroup::Elsewhere => assert!(
                reason.starts_with("réservé") || reason.starts_with("culture"),
                "{reason}"
            ),
            RecruitGroup::Soon => assert!(
                reason.contains("requis") || reason.starts_with("disponible"),
                "{reason}"
            ),
            RecruitGroup::Blocked => assert!(!option.available),
        }
    }
}
