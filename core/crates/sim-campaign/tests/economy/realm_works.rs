//! Realm-wide construction overview: the query and the batch of works.

use data_model::test_support::game_data;
use data_model::{BuildingId, FactionId, GameData, SettlementId};
use sim_campaign::realm_works::{apply_works_batch, plan_works_batch, realm_works};
use sim_campaign::CampaignState;

fn france() -> FactionId {
    FactionId::new("fac_france").unwrap()
}

fn start(data: &GameData, treasury: i64) -> CampaignState {
    let mut state = CampaignState::new_1337(data, france(), 7).expect("1337 start");
    for place in state.settlements.values_mut() {
        if place.owner == france() {
            place.construction = None;
            place.build_queue.clear();
        }
    }
    state.factions.get_mut(&france()).unwrap().treasury = treasury;
    state
}

/// Up to `count` `(settlement, building)` pairs the overview calls available.
fn first_entries(
    state: &CampaignState,
    data: &GameData,
    count: usize,
) -> Vec<(SettlementId, BuildingId)> {
    realm_works(state, data, &france())
        .rows
        .iter()
        .filter_map(|row| {
            row.options
                .iter()
                .find(|o| o.available && o.cost > 0)
                .map(|o| (row.settlement.clone(), o.building.clone()))
        })
        .take(count)
        .collect()
}

#[test]
fn overview_lists_the_colonies_with_free_slots() {
    let data = game_data();
    let state = start(data, 1_000_000);
    let works = realm_works(&state, data, &france());
    assert!(works.rows.len() > 5, "France holds many localités");
    assert_eq!(works.treasury, 1_000_000);
    assert_eq!(works.works_in_progress, 0);
    assert!(works.free_slots > 0);
    assert!(works.rows.iter().any(|r| r.can_start()));
    assert!(works.rows.iter().all(|r| r.queue_size >= 1));
}

#[test]
fn overview_of_an_unknown_faction_is_empty() {
    let data = game_data();
    let state = start(data, 0);
    let works = realm_works(&state, data, &FactionId::new("fac_nowhere").unwrap());
    assert!(works.rows.is_empty());
}

#[test]
fn batch_starts_every_entry_and_charges_the_plan_cost() {
    let data = game_data();
    let mut state = start(data, 1_000_000);
    let entries = first_entries(&state, data, 6);
    assert!(entries.len() >= 3);
    let plan = plan_works_batch(&state, data, &france(), &entries);
    assert_eq!(plan.started(), entries.len());
    // The plan runs on a copy: the real state is untouched.
    assert_eq!(state.factions[&france()].treasury, 1_000_000);

    let report = apply_works_batch(&mut state, data, &france(), &entries);
    assert_eq!(report, plan);
    assert_eq!(
        state.factions[&france()].treasury,
        1_000_000 - plan.total_cost()
    );
    assert_eq!(
        realm_works(&state, data, &france()).works_in_progress,
        entries.len()
    );
}

#[test]
fn batch_runs_in_order_until_the_money_runs_out() {
    let data = game_data();
    let state = start(data, 1_000_000);
    let entries = first_entries(&state, data, 6);
    let full = plan_works_batch(&state, data, &france(), &entries);
    let first_cost = full.entries[0].outcome.clone().unwrap();
    let limited = start(data, i64::from(first_cost));
    let plan = plan_works_batch(&limited, data, &france(), &entries);
    assert!(plan.entries[0].outcome.is_ok());
    assert!(plan.entries[1..].iter().all(|e| e.outcome.is_err()));
    assert!(plan.started() < entries.len());
}

#[test]
fn batch_refuses_a_full_queue_without_stopping_the_others() {
    let data = game_data();
    let mut state = start(data, 10_000_000);
    let entries = first_entries(&state, data, 2);
    let (town, building) = entries[0].clone();
    let size = data.economy_rules.construction_queue_size as usize;
    let one_town: Vec<_> = (0..=size)
        .map(|_| (town.clone(), building.clone()))
        .collect();
    let mut all = one_town;
    all.push(entries[1].clone());
    let report = apply_works_batch(&mut state, data, &france(), &all);
    // The same building cannot be queued twice, and the last entry still runs.
    assert!(report.entries[0].outcome.is_ok());
    assert!(report.entries[1].outcome.is_err());
    assert!(report.entries.last().unwrap().outcome.is_ok());
}
