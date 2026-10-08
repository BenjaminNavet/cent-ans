//! Lot HL1 (settlements quick-access list, touche B): `holdings_overview`.
//! See `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` § 1.

use data_model::{FactionId, GameData, SettlementId, SettlementKind};
use sim_campaign::holdings::{holdings_overview, DangerReason, SettlementRow};
use sim_campaign::{CampaignState, Order, Place};

use data_model::test_support::game_data;

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 7).expect("1337 start")
}

fn france() -> FactionId {
    FactionId::new("fac_france").unwrap()
}

/// First settlement of `kind` owned and controlled by France, by id.
fn french_settlement(state: &CampaignState, kind: SettlementKind) -> SettlementId {
    let france = france();
    state
        .settlements
        .iter()
        .find(|(_, s)| s.kind == kind && s.owner == france && s.controller == france)
        .map(|(id, _)| id.clone())
        .expect("France holds a settlement of this kind")
}

/// The row of `id` in the overview of `faction`.
fn row<'a>(
    rows: &'a [sim_campaign::holdings::ProvinceRow],
    id: &SettlementId,
) -> &'a SettlementRow {
    rows.iter()
        .flat_map(|p| &p.settlements)
        .find(|s| &s.id == id)
        .expect("settlement is listed")
}

/// Gives France's treasury a very large amount so every construction is
/// affordable, or empties it so none is.
fn set_treasury(state: &mut CampaignState, amount: i64) {
    state.factions.get_mut(&france()).unwrap().treasury = amount;
}

#[test]
fn idle_settlement_with_enough_treasury_can_upgrade() {
    let data = game_data();
    let mut state = start(data);
    set_treasury(&mut state, 10_000_000);

    // Find a settlement with at least one upgrade among its build options
    // once money is not a limit.
    let settlement = state
        .settlements
        .iter()
        .filter(|(_, s)| {
            s.owner == france() && s.controller == france() && s.construction.is_none()
        })
        .map(|(id, _)| id.clone())
        .find(|id| {
            let live = &state.settlements[id];
            state.buildable(data, id).iter().any(|option| {
                option.available
                    && data
                        .buildings
                        .get(&option.building)
                        .and_then(|b| b.upgrades_from.as_ref())
                        .is_some_and(|base| live.buildings.contains(base))
            })
        })
        .expect("at least one French settlement has an upgrade available");

    let overview = holdings_overview(&state, data, &france());
    let r = row(&overview.provinces, &settlement);
    assert!(r.idle, "no construction under way: the settlement is idle");
    assert!(r.upgrade_available, "an upgrade is affordable");
    assert!(
        !r.options_available.is_empty(),
        "at least the upgrade is offered"
    );
    assert_eq!(overview.count_idle, count_matching(&overview, |s| s.idle));
    assert_eq!(
        overview.count_upgrade,
        count_matching(&overview, |s| !s.options_available.is_empty())
    );
}

#[test]
fn idle_settlement_without_enough_treasury_has_no_options() {
    let data = game_data();
    let mut state = start(data);
    set_treasury(&mut state, 0);

    let village = french_settlement(&state, SettlementKind::Village);
    let overview = holdings_overview(&state, data, &france());
    let r = row(&overview.provinces, &village);
    assert!(r.idle);
    assert!(
        r.options_available.is_empty(),
        "no treasury: nothing is affordable"
    );
    assert!(!r.upgrade_available);
}

#[test]
fn settlement_under_construction_is_not_idle() {
    let data = game_data();
    let mut state = start(data);
    set_treasury(&mut state, 10_000_000);
    let town = french_settlement(&state, SettlementKind::Town);
    let build = state
        .buildable(data, &town)
        .into_iter()
        .find(|option| option.available)
        .expect("a town can build at least one building");
    state
        .submit_order(
            data,
            Order::Build {
                settlement: Place::Settlement(town.clone()),
                building: build.building.clone(),
            },
        )
        .expect("build in the town");

    let overview = holdings_overview(&state, data, &france());
    let r = row(&overview.provinces, &town);
    assert!(!r.idle, "a construction is under way");
    let construction = r.construction.as_ref().expect("construction is filled");
    assert_eq!(construction.building, build.building);
    assert!(
        r.options_available.is_empty(),
        "already building: no other option offered"
    );
}

#[test]
fn besieged_settlement_has_no_income_and_is_endangered() {
    let data = game_data();
    let mut state = start(data);
    let town = french_settlement(&state, SettlementKind::Town);
    state.settlements.get_mut(&town).unwrap().siege = Some(sim_campaign::SiegeState {
        attacker: FactionId::new("fac_england").unwrap(),
        turns_left: 4,
        turns_elapsed: 0,
        supplies: 100,
        breach: 0,
        started_turn: 0,
        engine_work: 0,
    });

    let overview = holdings_overview(&state, data, &france());
    let r = row(&overview.provinces, &town);
    assert_eq!(r.income, 0);
    assert!(r.endangered);
    assert!(r.danger_reasons.contains(&DangerReason::Siege));
    assert!(overview.count_endangered >= 1);
}

#[test]
fn occupied_settlement_is_listed_and_excluded_from_slots_total() {
    let data = game_data();
    let mut state = start(data);
    let town = french_settlement(&state, SettlementKind::Town);
    let england = FactionId::new("fac_england").unwrap();
    state.settlements.get_mut(&town).unwrap().controller = england;

    let overview = holdings_overview(&state, data, &france());
    let r = row(&overview.provinces, &town);
    assert!(r.occupied);
    assert!(r.danger_reasons.contains(&DangerReason::Occupied));
    assert_eq!(r.income, 0);

    let province_id = state.settlements[&town].province.clone();
    let province_row = overview
        .provinces
        .iter()
        .find(|p| p.province == province_id)
        .expect("the province is still listed");
    let non_occupied_settlements = province_row
        .settlements
        .iter()
        .filter(|s| !s.occupied)
        .count();
    assert_eq!(province_row.slots_total, non_occupied_settlements);
}

#[test]
fn province_income_is_the_sum_of_its_settlements() {
    let data = game_data();
    let state = start(data);
    let overview = holdings_overview(&state, data, &france());
    assert!(!overview.provinces.is_empty());
    for province in &overview.provinces {
        let sum: i64 = province.settlements.iter().map(|s| s.income).sum();
        assert_eq!(province.income, sum);
    }
    let total: i64 = overview.provinces.iter().map(|p| p.income).sum();
    assert_eq!(overview.settlement_income_total, total);
}

#[test]
fn unknown_faction_has_an_empty_overview() {
    let data = game_data();
    let state = start(data);
    let unknown = FactionId::new("fac_does_not_exist").unwrap();
    let overview = holdings_overview(&state, data, &unknown);
    assert!(overview.provinces.is_empty());
    assert_eq!(overview.treasury, 0);
}

fn count_matching(
    overview: &sim_campaign::holdings::HoldingsOverview,
    pred: impl Fn(&SettlementRow) -> bool,
) -> usize {
    overview
        .provinces
        .iter()
        .flat_map(|p| &p.settlements)
        .filter(|s| pred(s))
        .count()
}
