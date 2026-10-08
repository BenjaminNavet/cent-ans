//! M5 (diplomacy & religion) integration tests. See
//! `docs/design/m5-diplomacy-religion.md` § 2.5.

use data_model::{FactionId, GameData, ReligionId};
use sim_campaign::diplomacy::{evaluate, Proposal, RelationKind};
use sim_campaign::{CampaignState, EventKind, Order, OrderError, Season, SettlementState};

use data_model::test_support::{fac, game_data, prov};

/// Mutable state of the city of a province (lot C4: it carries control).
fn city_mut<'a>(state: &'a mut CampaignState, province: &str) -> &'a mut SettlementState {
    let id = state.province_city_id(&prov(province)).unwrap().clone();
    state.settlements.get_mut(&id).unwrap()
}

fn start(data: &GameData, player: &str, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac(player), seed).expect("1337 start")
}

/// A planner that does nothing: only the orders the test submits apply.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn events_of(state: &mut CampaignState, data: &GameData) -> Vec<sim_campaign::GameEvent> {
    state.end_turn_with(data, idle)
}

// ----- attitude & casus belli ------------------------------------------------

#[test]
fn attitude_has_reasons() {
    let data = game_data();
    let state = start(data, "fac_france", 1);
    let (value, reasons) = state.attitude(data, &fac("fac_england"), &fac("fac_france"));
    assert!(value < 0, "England hates France in 1337: {value}");
    assert!(reasons.iter().any(|(t, _)| t == "En guerre"));
    let (ally, reasons) = state.attitude(data, &fac("fac_scotland"), &fac("fac_france"));
    assert!(ally > 0, "the Auld Alliance: {ally}");
    assert!(reasons.iter().any(|(t, _)| t == "Alliés"));
    assert!(reasons.iter().any(|(t, _)| t == "Ennemi commun"));
}

#[test]
fn claims_give_casus_belli() {
    let data = game_data();
    let state = start(data, "fac_france", 2);
    let cb = state.casus_belli(data, &fac("fac_england"), &fac("fac_france"));
    assert!(cb.unwrap().contains("trône"));
    let cb = state.casus_belli(data, &fac("fac_france"), &fac("fac_england"));
    assert!(cb.unwrap().contains("prétention"));
    assert!(state
        .casus_belli(data, &fac("fac_aragon"), &fac("fac_navarre"))
        .is_none());
}

// ----- war & peace --------------------------------------------------------------

#[test]
fn war_without_casus_belli_costs_reputation() {
    let data = game_data();
    let mut state = start(data, "fac_aragon", 3);
    let before = state
        .attitude(data, &fac("fac_castile"), &fac("fac_aragon"))
        .0;
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_navarre"),
            },
        )
        .unwrap();
    assert!(state.is_at_war(&fac("fac_aragon"), &fac("fac_navarre")));
    let after = state
        .attitude(data, &fac("fac_castile"), &fac("fac_aragon"))
        .0;
    assert!(after < before, "{before} -> {after}");
    let events = events_of(&mut state, data);
    assert!(events.iter().any(|e| e.kind == EventKind::WarDeclared));
}

#[test]
fn allies_are_called_to_arms() {
    let data = game_data();
    let mut state = start(data, "fac_england", 4);
    // England attacks Castile, France's ally: France, already at war with
    // England, stays at war; Castile's other ties decide for themselves.
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_castile"),
            },
        )
        .unwrap();
    assert!(state.is_at_war(&fac("fac_england"), &fac("fac_castile")));
    // Every ally of Castile either joined or broke the alliance.
    let castile_allies = state
        .faction_state(&fac("fac_castile"))
        .unwrap()
        .allies
        .clone();
    for ally in castile_allies {
        assert!(state.is_at_war(&ally, &fac("fac_england")), "{ally} joined");
    }
}

#[test]
fn allied_factions_cannot_declare_war_on_each_other() {
    let data = game_data();
    let mut state = start(data, "fac_france", 5);
    let result = state.submit_order(
        data,
        Order::DeclareWar {
            target: fac("fac_scotland"),
        },
    );
    assert!(matches!(result, Err(OrderError::Diplomacy(_))));
}

#[test]
fn peace_depends_on_war_score() {
    let data = game_data();
    let mut state = start(data, "fac_france", 6);
    let white = Proposal::Peace {
        provinces: Vec::new(),
        tribute: 0,
    };
    // Even war: England has no reason to accept a peace that cedes Guyenne.
    let greedy = Proposal::Peace {
        provinces: vec![prov("prov_guyenne"), prov("prov_gascogne")],
        tribute: 0,
    };
    let verdict = evaluate(
        &state,
        data,
        &fac("fac_france"),
        &fac("fac_england"),
        &greedy,
    );
    assert!(!verdict.accept, "{verdict:?}");
    // France occupies English lands and wins battles: now it accepts.
    for province in [
        "prov_guyenne",
        "prov_gascogne",
        "prov_ponthieu",
        "prov_kent",
    ] {
        city_mut(&mut state, province).controller = fac("fac_france");
    }
    for _ in 0..6 {
        state
            .factions
            .get_mut(&fac("fac_france"))
            .unwrap()
            .war_scores
            .insert(fac("fac_england"), 50);
        state
            .factions
            .get_mut(&fac("fac_england"))
            .unwrap()
            .war_scores
            .insert(fac("fac_france"), -50);
    }
    let verdict = evaluate(
        &state,
        data,
        &fac("fac_france"),
        &fac("fac_england"),
        &white,
    );
    assert!(verdict.accept, "{verdict:?}");
    assert!(verdict.reasons.iter().any(|(t, _)| t == "Score de guerre"));
}

#[test]
fn peace_cedes_provinces_and_starts_a_truce() {
    let data = game_data();
    let mut state = start(data, "fac_france", 7);
    city_mut(&mut state, "prov_guyenne").controller = fac("fac_france");
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .war_scores
        .insert(fac("fac_england"), 100);
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .war_scores
        .insert(fac("fac_france"), -100);
    state
        .submit_order(
            data,
            Order::ProposePeace {
                target: fac("fac_england"),
                provinces: vec![prov("prov_guyenne")],
                tribute: 0,
            },
        )
        .unwrap();
    assert!(!state.is_at_war(&fac("fac_france"), &fac("fac_england")));
    assert_eq!(
        state.province_owner(&prov("prov_guyenne")),
        Some(&fac("fac_france"))
    );
    assert_eq!(
        state.relation(&fac("fac_france"), &fac("fac_england")),
        RelationKind::Truce
    );
    // England now claims Guyenne back.
    assert!(state
        .faction_state(&fac("fac_england"))
        .unwrap()
        .claims
        .iter()
        .any(|c| c.province == Some(prov("prov_guyenne"))));
    // Breaking the truce is perjury.
    let before = state
        .attitude(data, &fac("fac_castile"), &fac("fac_france"))
        .0;
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_england"),
            },
        )
        .unwrap();
    let after = state
        .attitude(data, &fac("fac_castile"), &fac("fac_france"))
        .0;
    assert!(after <= before - 30, "{before} -> {after}");
}

// ----- alliances, embargo, vassals --------------------------------------------

#[test]
fn alliance_needs_a_good_attitude() {
    let data = game_data();
    let mut state = start(data, "fac_england", 8);
    let refused = state.submit_order(
        data,
        Order::ProposeAlliance {
            target: fac("fac_scotland"),
        },
    );
    assert!(matches!(refused, Err(OrderError::Diplomacy(_))), "at war");
    // Portugal (at war with Castile, France's ally) is a natural partner.
    let verdict = evaluate(
        &state,
        data,
        &fac("fac_england"),
        &fac("fac_portugal"),
        &Proposal::Alliance,
    );
    assert!(verdict.reasons.iter().any(|(t, _)| t == "Attitude"));
}

#[test]
fn embargo_cuts_income() {
    let data = game_data();
    let mut state = start(data, "fac_france", 9);
    let flanders = fac("fac_flanders");
    let before = state.faction_income_effective(data, &flanders);
    // Remove England's embargo, then compare.
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .embargoes
        .remove(&flanders);
    let without = state.faction_income_effective(data, &flanders);
    assert!(
        without > before,
        "embargo costs Flanders: {before} vs {without}"
    );
}

#[test]
fn vassals_pay_tribute_and_can_rebel() {
    let data = game_data();
    let mut state = start(data, "fac_france", 10);
    let flanders = fac("fac_flanders");
    assert_eq!(
        state.faction_state(&flanders).unwrap().suzerain,
        Some(fac("fac_france"))
    );
    // English embargo squeezes Flanders: low loyalty (1337 historical tension).
    let loyalty = state.faction_state(&flanders).unwrap().loyalty;
    assert!(loyalty < 60, "Flemish loyalty {loyalty}");
    state.factions.get_mut(&flanders).unwrap().loyalty = 0;
    let mut rebelled = false;
    for _ in 0..40 {
        state.factions.get_mut(&flanders).unwrap().loyalty = 0;
        let events = events_of(&mut state, data);
        if events.iter().any(|e| e.kind == EventKind::VassalRebellion) {
            rebelled = true;
            break;
        }
    }
    assert!(rebelled);
    assert!(state.is_at_war(&flanders, &fac("fac_france")));
    assert_eq!(state.faction_state(&flanders).unwrap().suzerain, None);
}

#[test]
fn release_vassal_and_demand_vassalage() {
    let data = game_data();
    let mut state = start(data, "fac_france", 11);
    state
        .submit_order(
            data,
            Order::ReleaseVassal {
                target: fac("fac_brittany"),
            },
        )
        .unwrap();
    assert_eq!(
        state.faction_state(&fac("fac_brittany")).unwrap().suzerain,
        None
    );
    // France is far stronger than Navarre but Navarre refuses without cause.
    let verdict = evaluate(
        &state,
        data,
        &fac("fac_france"),
        &fac("fac_navarre"),
        &Proposal::Vassalage,
    );
    assert!(verdict
        .reasons
        .iter()
        .any(|(t, _)| t == "Perte d'indépendance"));
}

// ----- offers ----------------------------------------------------------------

#[test]
fn ai_proposals_to_the_player_become_offers() {
    let data = game_data();
    let mut state = start(data, "fac_france", 12);
    state
        .propose(
            data,
            &fac("fac_england"),
            &fac("fac_france"),
            Proposal::Peace {
                provinces: Vec::new(),
                tribute: 0,
            },
        )
        .unwrap();
    let offers = state
        .faction_state(&fac("fac_france"))
        .unwrap()
        .offers
        .clone();
    assert_eq!(offers.len(), 1);
    assert!(offers[0].text_fr.contains("paix"));
    state
        .submit_order(
            data,
            Order::AnswerOffer {
                offer: offers[0].id,
                accept: true,
            },
        )
        .unwrap();
    assert!(!state.is_at_war(&fac("fac_france"), &fac("fac_england")));
    // Offers expire.
    state
        .propose(
            data,
            &fac("fac_castile"),
            &fac("fac_france"),
            Proposal::Alliance,
        )
        .ok();
    for _ in 0..4 {
        events_of(&mut state, data);
    }
    assert!(state
        .faction_state(&fac("fac_france"))
        .unwrap()
        .offers
        .is_empty());
}

// ----- religion ------------------------------------------------------------------

#[test]
fn papal_mediation_and_donations() {
    let data = game_data();
    let mut state = start(data, "fac_france", 13);
    let favor = state.faction_state(&fac("fac_france")).unwrap().papal_favor;
    state
        .submit_order(data, Order::DonateToChurch { amount: 2000 })
        .unwrap();
    assert!(state.faction_state(&fac("fac_france")).unwrap().papal_favor >= favor + 10);
    let treasury = state.faction_state(&fac("fac_france")).unwrap().treasury;
    // Scotland is at war with England; Scotland asks the pope to mediate
    // would be the AI's call; here France asks for a truce with England.
    let result = state.submit_order(
        data,
        Order::RequestPapalMediation {
            target: fac("fac_england"),
        },
    );
    match result {
        Ok(()) => {
            assert!(!state.is_at_war(&fac("fac_france"), &fac("fac_england")));
            assert_eq!(
                state.faction_state(&fac("fac_france")).unwrap().treasury,
                treasury - sim_campaign::diplomacy::MEDIATION_COST
            );
        }
        Err(_) => {
            // Refused: nothing was paid.
            assert_eq!(
                state.faction_state(&fac("fac_france")).unwrap().treasury,
                treasury
            );
        }
    }
}

#[test]
fn excommunication_after_aggression_with_no_favor() {
    let data = game_data();
    let mut state = start(data, "fac_aragon", 14);
    state
        .factions
        .get_mut(&fac("fac_aragon"))
        .unwrap()
        .papal_favor = 5;
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_navarre"),
            },
        )
        .unwrap();
    assert!(sim_campaign::religion::is_excommunicated(
        &state,
        &fac("fac_aragon")
    ));
    let (value, reasons) = state.attitude(data, &fac("fac_castile"), &fac("fac_aragon"));
    assert!(reasons.iter().any(|(t, _)| t == "Excommunié"), "{value}");
}

#[test]
fn great_schism_1378_and_council_of_constance() {
    let data = game_data();
    let mut state = start(data, "fac_france", 15);
    state.year = 1378;
    state.season = Season::Autumn;
    let events = events_of(&mut state, data);
    assert!(state.schism);
    assert!(events.iter().any(|e| e.kind == EventKind::Schism));
    let rome = ReligionId::new("rel_catholic_rome").unwrap();
    let avignon = ReligionId::new("rel_catholic").unwrap();
    assert_eq!(
        state.faction_state(&fac("fac_england")).unwrap().religion,
        Some(rome.clone())
    );
    assert_eq!(
        state.faction_state(&fac("fac_france")).unwrap().religion,
        Some(avignon.clone())
    );
    let (_, reasons) = state.attitude(data, &fac("fac_england"), &fac("fac_france"));
    assert!(reasons.iter().any(|(t, _)| t == "Obédience rivale"));
    // The player is asked to choose.
    let offer = state
        .faction_state(&fac("fac_france"))
        .unwrap()
        .offers
        .iter()
        .find(|o| matches!(o.proposal, Proposal::Obedience { .. }))
        .cloned();
    assert!(offer.is_some());
    // 1417: the Church is reunited.
    state.year = 1417;
    state.season = Season::Autumn;
    events_of(&mut state, data);
    assert!(!state.schism);
    assert_eq!(
        state.faction_state(&fac("fac_england")).unwrap().religion,
        Some(avignon)
    );
}

#[test]
fn lollards_appear_and_spread_with_unrest() {
    let data = game_data();
    let mut state = start(data, "fac_england", 16);
    state.year = 1381;
    let oxford = prov("prov_oxford");
    events_of(&mut state, data);
    let p = state.province_state(&oxford).unwrap();
    assert!(p.heresy > 0, "Lollards in Oxford");
    assert_eq!(
        p.heresy_religion,
        Some(ReligionId::new("rel_lollard").unwrap())
    );
    // Unhappy clergy make it grow.
    let before = p.heresy;
    for _ in 0..4 {
        let p = state.provinces.get_mut(&oxford).unwrap();
        p.population.clergy.unrest = 100;
        p.population.clergy.goods_satisfaction = 0;
        events_of(&mut state, data);
    }
    assert!(state.province_state(&oxford).unwrap().heresy > before);
    assert!(state.political_unrest(&oxford) > 0.0);
}

// ----- save & determinism ----------------------------------------------------

#[test]
fn save_round_trip_keeps_diplomacy() {
    let data = game_data();
    let mut state = start(data, "fac_france", 17);
    state
        .submit_order(
            data,
            Order::SetEmbargo {
                target: fac("fac_england"),
                active: true,
            },
        )
        .unwrap();
    for _ in 0..4 {
        state.end_turn(data);
    }
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded, state);
    assert!(loaded
        .faction_state(&fac("fac_france"))
        .unwrap()
        .embargoes
        .contains(&fac("fac_england")));
}

#[test]
fn forty_turns_are_deterministic_with_diplomacy() {
    let data = game_data();
    let run = || {
        let mut state = start(data, "fac_france", 18);
        for _ in 0..40 {
            state.end_turn(data);
        }
        state.save_json()
    };
    assert_eq!(run(), run());
}
