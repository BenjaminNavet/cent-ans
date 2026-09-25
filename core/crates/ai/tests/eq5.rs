//! Lot EQ5: the campaign AI's budget counts its commitments (tributes of a
//! lost war), and its armies do not camp in the lands of a realm at peace
//! without right of passage.

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId, SettlementKind};
use sim_campaign::movement::edges;
use sim_campaign::negotiation::TributeDue;
use sim_campaign::passage::trespassed_owner;
use sim_campaign::{ArmyId, CampaignState, Order, Place};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn main_army(state: &CampaignState, faction: &FactionId) -> ArmyId {
    state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

fn destination(orders: &[Order], army: &ArmyId) -> Option<SettlementId> {
    orders.iter().rev().find_map(|o| match o {
        Order::MoveArmy { army: a, target } if a == army => match target {
            sim_campaign::MoveOrderTarget::Place(Place::Settlement(s)) => Some(s.clone()),
            sim_campaign::MoveOrderTarget::Path(path) => match path.last() {
                Some(Place::Settlement(s)) => Some(s.clone()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    })
}

fn at_peace(state: &mut CampaignState) {
    for f in state.factions.values_mut() {
        f.at_war_with.clear();
    }
}

/// A place in the lands of a realm at peace with `faction` whose roads all
/// lead to other places of those lands: an army there is surrounded by
/// lands closed without right of passage.
fn deep_foreign_place(state: &CampaignState, data: &GameData, faction: &FactionId) -> SettlementId {
    state
        .settlements
        .keys()
        .find(|id| {
            let Some(owner) = state
                .settlement_province(id)
                .and_then(|p| trespassed_owner(state, faction, p))
            else {
                return false;
            };
            let roads = edges(data, id);
            !roads.is_empty()
                && roads.iter().all(|(next, _)| {
                    state
                        .settlement_province(next)
                        .and_then(|p| trespassed_owner(state, faction, p))
                        .as_ref()
                        == Some(&owner)
                })
        })
        .cloned()
        .expect("a place deep in foreign lands")
}

#[test]
fn an_army_caught_deep_in_foreign_lands_marches_out() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 1).unwrap();
    at_peace(&mut state);
    let france = fac("fac_france");
    let army = main_army(&state, &france);
    let place = deep_foreign_place(&state, &data, &france);
    state.armies.get_mut(&army).unwrap().position = sim_campaign::ArmyPosition::Settlement(place);
    let orders = ai::plan_turn(&state, &data, &france);
    assert!(
        destination(&orders, &army).is_some(),
        "the army leaves the lands it may not stay in: {orders:?}"
    );
}

#[test]
fn at_peace_an_army_in_its_own_castle_abroad_joins_the_garrison() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 1).unwrap();
    at_peace(&mut state);
    let france = fac("fac_france");
    // A castle of a foreign province, held by France.
    let place = state
        .settlements
        .iter()
        .filter(|(_, s)| s.kind == SettlementKind::Castle)
        .map(|(id, _)| id.clone())
        .find(|id| {
            state
                .settlement_province(id)
                .and_then(|p| trespassed_owner(&state, &france, p))
                .is_some()
        })
        .expect("a foreign castle");
    {
        let s = state.settlements.get_mut(&place).unwrap();
        s.controller = france.clone();
        s.owner = france.clone();
        s.garrison.clear();
        s.siege = None;
    }
    let army = main_army(&state, &france);
    {
        let a = state.armies.get_mut(&army).unwrap();
        a.units.truncate(2);
        a.position = sim_campaign::ArmyPosition::Settlement(place);
    }
    let orders = ai::plan_turn(&state, &data, &france);
    assert!(
        orders.iter().any(|o| matches!(
            o,
            Order::GarrisonUnits { army: a, unit_indices } if a == &army && unit_indices == &vec![0, 1]
        )),
        "the whole army joins the garrison: {orders:?}"
    );
}

#[test]
fn a_tribute_the_treasury_cannot_bear_is_budgeted() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 1).unwrap();
    at_peace(&mut state);
    let france = fac("fac_france");
    let income = state.faction_income_effective(&data, &france);
    {
        let f = state.factions.get_mut(&france).unwrap();
        f.treasury = 0;
        f.ledger.tributes.push(TributeDue {
            to: fac("fac_england"),
            per_season: income,
            until_turn: state.turn + 8,
        });
    }
    let orders = ai::plan_turn(&state, &data, &france);
    assert!(
        orders
            .iter()
            .any(|o| matches!(o, Order::DisbandUnit { .. })),
        "units are dismissed to pay the tribute"
    );
    assert!(
        !orders.iter().any(|o| matches!(o, Order::Recruit { .. } | Order::Build { .. })),
        "nothing is bought"
    );
}
