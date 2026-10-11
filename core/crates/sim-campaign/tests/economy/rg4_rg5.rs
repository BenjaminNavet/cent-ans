//! Lots RG4 (prorated construction cancellation) and RG5 (`can_build`).

use data_model::test_support::game_data;
use data_model::{BuildingId, FactionId, GameData, SettlementId, SettlementKind};
use sim_campaign::{CampaignState, Order, Place};

fn france() -> FactionId {
    FactionId::new("fac_france").unwrap()
}

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

/// Buildings buildable now taking at least 4 turns, longest first.
fn options_with(
    state: &CampaignState,
    data: &GameData,
    town: &SettlementId,
    min_turns: u32,
) -> Vec<BuildingId> {
    let mut options: Vec<_> = state
        .buildable(data, town)
        .into_iter()
        .filter(|o| o.available && o.cost > 0 && o.turns >= min_turns)
        .collect();
    options.sort_by_key(|o| std::cmp::Reverse(o.turns));
    options.into_iter().map(|o| o.building).collect()
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

fn cancel(state: &mut CampaignState, data: &GameData, town: &SettlementId) {
    state
        .submit_order(
            data,
            Order::CancelBuild {
                settlement: Place::Settlement(town.clone()),
            },
        )
        .expect("cancel order");
}

#[test]
fn same_turn_cancel_refunds_everything() {
    let data = game_data();
    let (mut state, town) = setup(data);
    let b = options_with(&state, data, &town, 4)[0].clone();
    let before = state.factions[&france()].treasury;
    build(&mut state, data, &town, &b);
    let quote = state.cancel_build_quote(data, &town).unwrap();
    assert_eq!(quote.elapsed, 0);
    assert_eq!(quote.refund, quote.paid);
    cancel(&mut state, data, &town);
    assert_eq!(state.factions[&france()].treasury, before);
}

#[test]
fn half_way_cancel_refunds_the_remaining_share() {
    let data = game_data();
    let (mut state, town) = setup(data);
    let b = options_with(&state, data, &town, 4)[0].clone();
    build(&mut state, data, &town, &b);
    let work = state.settlements[&town].construction.clone().unwrap();
    assert!(work.total_turns >= 4);
    let half = work.total_turns / 2;
    for _ in 0..half {
        state.end_turn(data);
    }
    let work = state.settlements[&town].construction.clone().unwrap();
    let quote = state.cancel_build_quote(data, &town).unwrap();
    assert_eq!(quote.elapsed, half);
    assert_eq!(quote.total, work.total_turns);
    let expected = (u64::from(work.paid) * u64::from(work.total_turns - half)
        / u64::from(work.total_turns)) as u32;
    assert_eq!(quote.refund, expected);
    assert!(quote.refund < quote.paid);
    let before = state.factions[&france()].treasury;
    cancel(&mut state, data, &town);
    assert_eq!(
        state.factions[&france()].treasury,
        before + i64::from(expected)
    );
}

#[test]
fn queued_cancel_refunds_everything() {
    let data = game_data();
    let (mut state, town) = setup(data);
    let options = options_with(&state, data, &town, 1);
    build(&mut state, data, &town, &options[0]);
    let before = state.factions[&france()].treasury;
    build(&mut state, data, &town, &options[1]);
    let paid = state.settlements[&town].build_queue[0].paid;
    let quote = state.cancel_queued_build_quote(data, &town, 0).unwrap();
    assert_eq!((quote.paid, quote.refund, quote.elapsed), (paid, paid, 0));
    state
        .submit_order(
            data,
            Order::CancelQueuedBuild {
                settlement: Place::Settlement(town.clone()),
                index: 0,
            },
        )
        .unwrap();
    assert_eq!(state.factions[&france()].treasury, before);
}

#[test]
fn can_build_ignores_money_and_reports_a_full_queue() {
    let data = game_data();
    let (mut state, town) = setup(data);
    assert!(state.can_build(data, &town).ok);
    state.factions.get_mut(&france()).unwrap().treasury = 0;
    assert!(state.can_build(data, &town).ok);
    state.factions.get_mut(&france()).unwrap().treasury = 1_000_000;
    let size = data.economy_rules.construction_queue_size as usize;
    let options = options_with(&state, data, &town, 1);
    assert!(options.len() >= size, "enough buildings to fill the queue");
    // Keep the first accepted options: some (e.g. RG9 roads) take a slot and
    // can make the following ones refused before the queue is full.
    let mut queued = 0;
    for b in &options {
        if queued == size {
            break;
        }
        let order = Order::Build {
            settlement: Place::Settlement(town.clone()),
            building: b.clone(),
        };
        if state.submit_order(data, order).is_ok() {
            queued += 1;
        }
    }
    assert_eq!(queued, size, "queue filled");
    let verdict = state.can_build(data, &town);
    assert!(!verdict.ok);
    assert_eq!(verdict.reason.as_deref(), Some("File de chantiers pleine"));
}

#[test]
fn holdings_row_carries_can_build() {
    let data = game_data();
    let (state, town) = setup(data);
    let verdict = state.can_build(data, &town);
    let overview = sim_campaign::holdings::holdings_overview(&state, data, &france());
    let province = overview
        .provinces
        .iter()
        .find(|p| p.settlements.iter().any(|s| s.id == town))
        .unwrap();
    let row = province.settlements.iter().find(|s| s.id == town).unwrap();
    assert_eq!(row.can_build, verdict.ok);
    assert_eq!(row.can_build_reason, verdict.reason);
    assert_eq!(
        province.buildable_count,
        province.settlements.iter().filter(|s| s.can_build).count()
    );
}
