//! M9 strategic campaign AI tests (spec `docs/design/m9-ai.md` § 1).
use data_model::test_support::{fac, game_data};

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::{CampaignState, Order, Place};

fn data() -> &'static GameData {
    ai::feudal::install();
    game_data()
}

fn start(data: &GameData, player: &str, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac(player), seed).expect("1337 start")
}

/// Applies the AI's orders for `faction` and returns (issued, refused).
fn apply(state: &mut CampaignState, data: &GameData, faction: &FactionId) -> (usize, usize) {
    let orders = ai::plan_turn(state, data, faction);
    let issued = orders.len();
    let refused = orders
        .into_iter()
        .filter(|o| state.apply_order(data, faction, o.clone()).is_err())
        .count();
    (issued, refused)
}

#[test]
fn planner_is_pure_and_deterministic() {
    let data = data();
    let state = start(data, "fac_england", 1);
    let a = ai::plan_turn(&state, data, &fac("fac_france"));
    let b = ai::plan_turn(&state, data, &fac("fac_france"));
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

#[test]
fn a_stronger_faction_at_war_besieges() {
    let data = data();
    let mut state = start(data, "fac_england", 2);
    let france = fac("fac_france");
    let mut besieged = false;
    for _ in 0..12 {
        apply(&mut state, data, &france);
        state.end_turn_with(data, ai::plan_turn);
        besieged |= state
            .settlements
            .values()
            .any(|s| s.siege.as_ref().is_some_and(|s| s.attacker == france));
        besieged |= state
            .settlements
            .values()
            .any(|s| s.owner != france && s.controller == france);
    }
    assert!(
        besieged,
        "France should besiege or take an English settlement"
    );
}

#[test]
fn a_threatened_province_is_defended() {
    let data = data();
    let mut state = start(data, "fac_england", 3);
    let france = fac("fac_france");
    // Move the English main army next to Picardie.
    let english = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_england"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let ponthieu = ProvinceId::new("prov_ponthieu").unwrap();
    let city = state.province_city_id(&ponthieu).unwrap().clone();
    state.armies.get_mut(&english).unwrap().position = sim_campaign::ArmyPosition::Settlement(city);
    let orders = ai::plan_turn(&state, data, &france);
    let near = [
        "prov_picardie",
        "prov_ponthieu",
        "prov_artois",
        "prov_normandie",
    ];
    let moves_towards_threat = orders.iter().any(|o| match o {
        Order::MoveArmy {
            target: sim_campaign::MoveOrderTarget::Place(p),
            ..
        } => match p {
            Place::Settlement(s) => state
                .settlement_province(s)
                .is_some_and(|p| near.contains(&p.as_str())),
            Place::Province(p) => near.contains(&p.as_str()),
        },
        _ => false,
    });
    let stays = orders.iter().all(|o| !matches!(o, Order::MoveArmy { .. }));
    assert!(moves_towards_threat || stays, "{orders:?}");
}

#[test]
fn recruitment_respects_the_budget() {
    let data = data();
    let mut state = start(data, "fac_england", 4);
    let navarre = fac("fac_navarre");
    state.factions.get_mut(&navarre).unwrap().treasury = 300;
    let orders = ai::plan_turn(&state, data, &navarre);
    assert!(
        !orders
            .iter()
            .any(|o| matches!(o, Order::Recruit { .. } | Order::Build { .. })),
        "{orders:?}"
    );
}

#[test]
fn indebted_factions_dismiss_troops() {
    let data = data();
    let mut state = start(data, "fac_england", 5);
    let navarre = fac("fac_navarre");
    state.factions.get_mut(&navarre).unwrap().treasury = -5000;
    let orders = ai::plan_turn(&state, data, &navarre);
    assert!(
        orders
            .iter()
            .any(|o| matches!(o, Order::DisbandUnit { .. })),
        "{orders:?}"
    );
}

#[test]
fn governors_and_generals_are_appointed() {
    let data = data();
    let mut state = start(data, "fac_england", 6);
    let france = fac("fac_france");
    apply(&mut state, data, &france);
    let governors = state
        .characters
        .values()
        .filter(|c| c.faction == france && c.governor_of.is_some())
        .count();
    assert!(governors >= 1, "at least one French governor");
    let led = state
        .armies
        .values()
        .filter(|a| a.faction == france)
        .all(|a| {
            a.general.is_some()
                || state.characters.values().all(|c| {
                    c.location.as_ref() != a.settlement().and_then(|s| state.settlement_province(s))
                        || c.faction != france
                        || c.army.is_some()
                        || c.governor_of.is_some()
                })
        });
    assert!(led);
}

#[test]
fn few_orders_are_refused_over_forty_turns() {
    let data = data();
    let mut state = start(data, "fac_england", 7);
    let (mut issued, mut refused) = (0, 0);
    for _ in 0..40 {
        for faction in ["fac_france", "fac_castile", "fac_scotland"] {
            let (i, r) = apply(&mut state, data, &fac(faction));
            issued += i;
            refused += r;
        }
        state.end_turn_with(data, ai::plan_turn);
    }
    assert!(issued > 100);
    assert!(refused * 5 < issued, "{refused} refused of {issued}");
}

#[test]
fn twenty_turns_are_deterministic() {
    let data = data();
    let run = || {
        let mut state = start(data, "fac_england", 8);
        for _ in 0..20 {
            state.end_turn_with(data, ai::plan_turn);
        }
        state.save_json()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_century_without_panic() {
    let data = data();
    let mut state = start(data, "fac_burgundy", 9);
    for _ in 0..100 {
        state.end_turn_with(data, ai::plan_turn);
    }
    let alive = state.factions.values().filter(|f| f.alive).count();
    assert!(alive >= 8, "{alive} factions alive after 25 years");
}
