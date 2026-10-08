//! F4 « guerre de Cent Ans vivante » AI tests (`docs/design/m9-ai.md` § F4).
use data_model::test_support::{fac, game_data};

use data_model::{GameData, UnitTypeId};
use sim_campaign::{CampaignState, Order, Season, Unit};

fn data() -> &'static GameData {
    ai::feudal::install();
    game_data()
}

/// 1345, France and England at peace, their truce just expired.
fn peace_after_truce(data: &GameData) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), 3).unwrap();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    // England's other wars of 1337 (Scotland, Burgundy) are over too.
    for other in ["fac_scotland", "fac_burgundy"] {
        let other = fac(other);
        state
            .factions
            .get_mut(&other)
            .unwrap()
            .at_war_with
            .remove(&england);
        state
            .factions
            .get_mut(&england)
            .unwrap()
            .at_war_with
            .remove(&other);
    }
    for (a, b) in [(&france, &england), (&england, &france)] {
        let f = state.factions.get_mut(a).unwrap();
        f.at_war_with.remove(b);
        f.truces.remove(b);
        f.war_scores.remove(b);
        f.war_started.remove(b);
    }
    // Planning parity differs per faction: try on both parities.
    state.turn = 32;
    state
}

fn declares_on(state: &mut CampaignState, data: &GameData, who: &str, target: &str) -> bool {
    (0..2).any(|offset| {
        state.turn = 32 + offset;
        sim_campaign::diplomacy::plan_diplomacy(state, data, &fac(who))
            .iter()
            .any(|o| matches!(o, Order::DeclareWar { target: t } if t == &fac(target)))
    })
}

#[test]
fn england_presses_its_claim_on_france_without_superiority() {
    let data = data();
    let mut state = peace_after_truce(data);
    // The pre-F4 rule (coalition ratio >= 1.5) would never declare here.
    let ratio =
        state.coalition_power(&fac("fac_england")) / state.coalition_power(&fac("fac_france"));
    assert!(
        ratio < sim_campaign::diplomacy::OPPORTUNIST_RATIO,
        "ratio {ratio}"
    );
    let wars = state.factions[&fac("fac_england")].at_war_with.clone();
    let pretender =
        state.coalition_power(&fac("fac_england")) / state.faction_power(&fac("fac_france"));
    assert!(
        declares_on(&mut state, data, "fac_england", "fac_france"),
        "ratio vs crown {pretender}, ready {}, wars {:?}, attitude {}",
        sim_campaign::diplomacy::war_ready(&state, &fac("fac_england")),
        wars,
        state
            .attitude(data, &fac("fac_england"), &fac("fac_france"))
            .0
    );
}

#[test]
fn no_claim_war_during_a_truce_a_regency_or_a_bankruptcy() {
    let data = data();
    let mut truce = peace_after_truce(data);
    let until = truce.turn + 10;
    truce
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .truces
        .insert(fac("fac_france"), until);
    assert!(!declares_on(&mut truce, data, "fac_england", "fac_france"));

    let mut regency = peace_after_truce(data);
    regency
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .regency = true;
    assert!(!declares_on(
        &mut regency,
        data,
        "fac_england",
        "fac_france"
    ));

    let mut broke = peace_after_truce(data);
    broke
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = -500;
    assert!(!declares_on(&mut broke, data, "fac_england", "fac_france"));
}

#[test]
fn claims_follow_the_crown() {
    let data = data();
    let state = CampaignState::new_1337(data, fac("fac_papacy"), 1).unwrap();
    let claimed = sim_campaign::diplomacy::claimed_provinces(&state, &fac("fac_england"));
    let paris = state.factions[&fac("fac_france")].capital.clone();
    assert!(claimed.contains(&paris), "the throne claim covers Paris");
    let stakes =
        sim_campaign::diplomacy::claim_stakes(&state, &fac("fac_france"), &fac("fac_england"));
    assert!(stakes.provinces >= 2, "France claims Guyenne and Gascony");
}

#[test]
fn the_auld_alliance_answers_the_call_to_arms() {
    let data = data();
    let state = CampaignState::new_1337(data, fac("fac_papacy"), 1).unwrap();
    assert!(sim_campaign::diplomacy::answers_call_to_arms(
        &state,
        data,
        &fac("fac_scotland"),
        &fac("fac_france"),
        &fac("fac_england"),
    ));
}

#[test]
fn a_hoarding_realm_spends() {
    let data = data();
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), 1).unwrap();
    let empire = fac("fac_france");
    // Lot C4: 300 000 (was 400 000). The settlements' buildings add upkeep,
    // and at 400 000 the court's opulence (20 % of the excess) made the war
    // runway rule dismiss troops before the hoard could be spent.
    state.factions.get_mut(&empire).unwrap().treasury = 300_000;
    state.season = Season::Autumn;
    let orders = ai::plan_turn(&state, data, &empire);
    assert!(orders.iter().any(|o| matches!(o, Order::Recruit { .. })));
    assert!(
        orders.iter().any(|o| matches!(o, Order::Build { .. })),
        "{orders:?}"
    );
    assert!(orders
        .iter()
        .any(|o| matches!(o, Order::DonateToChurch { .. })));
    assert!(
        !orders.iter().any(
            |o| matches!(o, Order::SetTaxRate { rate } if *rate == sim_campaign::TaxRate::Low)
        ),
        "a rich treasury is spent, not untaxed"
    );
}

#[test]
fn a_small_realm_dismisses_troops_before_bankruptcy() {
    let data = data();
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), 1).unwrap();
    let swiss = fac("fac_swiss");
    let capital = state.factions[&swiss].capital.clone();
    let unit_type = data
        .unit_types
        .get(&UnitTypeId::new("unit_pikemen").unwrap())
        .or_else(|| data.unit_types.values().next())
        .unwrap();
    for _ in 0..12 {
        let unit = Unit::fresh(unit_type);
        state.city_state_mut(&capital).unwrap().garrison.push(unit);
    }
    // Positive but short treasury: not bankrupt yet.
    state.factions.get_mut(&swiss).unwrap().treasury = 300;
    let orders = ai::plan_turn(&state, data, &swiss);
    assert!(orders
        .iter()
        .any(|o| matches!(o, Order::DisbandUnit { .. })));
    assert!(!orders.iter().any(|o| matches!(o, Order::Recruit { .. })));
}

#[test]
fn rulers_and_heirs_are_married_by_the_ai() {
    let data = data();
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), 1).unwrap();
    state.season = Season::Spring;
    let proposals: usize = state
        .factions
        .keys()
        .map(|f| {
            ai::plan_turn(&state, data, f)
                .iter()
                .filter(|o| {
                    matches!(
                        o,
                        Order::ProposeMarriage { .. } | Order::ProposeFactionMarriage { .. }
                    )
                })
                .count()
        })
        .sum();
    assert!(proposals >= 3, "only {proposals} marriage proposals");
    let _ = &mut state;
}
