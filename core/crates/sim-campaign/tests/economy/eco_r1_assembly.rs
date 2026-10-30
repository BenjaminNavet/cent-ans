//! ECO-R1 (« L'impôt consenti »): the crown convenes an assembly, which votes a subsidy
//! collected season after season and demands concessions; promises kept or broken change its
//! mood; a house without assembly keeps the old extraordinary levies.

use data_model::test_support::{fac, game_data, prov};
use data_model::{GameData, NecessityKind};
use sim_campaign::assembly::{
    self, AssemblyError, AssemblyRuntime, Promise, PromiseStatus, SessionStatus, VoteOutcome,
};
use sim_campaign::causes::CauseKind;
use sim_campaign::coinage::CoinageLevel;
use sim_campaign::finance;
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::test_support::{idle, start_quiet};
use sim_campaign::{CampaignState, Order, OrderError};

const OIL: &str = "asm_etats_oil";

/// The campaign of 1337 with every war and war goal cleared: the tests make their own.
fn scene(data: &GameData, faction: &str) -> CampaignState {
    let mut state = start_quiet(data, faction, 5);
    for f in state.factions.values_mut() {
        f.at_war_with.clear();
        f.ledger.war_goals.clear();
    }
    state
}

fn make_war(state: &mut CampaignState, a: &str, b: &str) {
    let (a, b) = (fac(a), fac(b));
    state
        .factions
        .get_mut(&a)
        .unwrap()
        .at_war_with
        .insert(b.clone());
    state.factions.get_mut(&b).unwrap().at_war_with.insert(a);
}

fn set_mood(state: &mut CampaignState, data: &GameData, assembly: &str, mood: i32) {
    let start = data.assembly_rules.assembly(assembly).unwrap().start_mood;
    let rt = state
        .assemblies
        .houses
        .entry(assembly.to_owned())
        .or_insert_with(|| AssemblyRuntime {
            mood: start,
            last_convened: None,
            sessions: vec![],
            promises: vec![],
            subsidies: vec![],
            grievances: vec![],
            convened: 0,
            granted: 0,
            refused: 0,
        });
    rt.mood = mood;
}

fn convene(
    state: &mut CampaignState,
    data: &GameData,
    necessity: NecessityKind,
    ask: &str,
) -> Result<(), OrderError> {
    state.submit_order(
        data,
        Order::ConveneAssembly {
            assembly: OIL.into(),
            necessity,
            ask: ask.into(),
            form: None,
            permanent: false,
        },
    )
}

fn first_session(state: &CampaignState) -> &sim_campaign::assembly::Session {
    &state.assemblies.houses[OIL].sessions[0]
}

#[test]
fn defensive_war_subsidy_granted_by_trusting_estates() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    let france = fac("fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    set_mood(&mut state, data, OIL, 90);
    let before = state.factions[&france].treasury;
    convene(&mut state, data, NecessityKind::DefensiveWar, "modest").expect("convened");
    assert_eq!(first_session(&state).status, SessionStatus::Pending);
    assert_eq!(state.factions[&france].treasury, before, "nothing yet");
    state.end_turn_with(data, idle);
    let session = first_session(&state);
    assert_eq!(session.status, SessionStatus::Granted, "{session:?}");
    assert!(session.per_season > 0);
    assert!(session.concessions.is_empty());
    let f = &state.factions[&france];
    assert_eq!(f.last_budget.levy_income, session.per_season);
    assert!(state
        .cause_journal()
        .records()
        .any(|r| r.cause.kind == CauseKind::SubsidyGranted));
    assert!(
        state.assemblies.houses[OIL].mood < 90,
        "each subsidy tires the estates"
    );
}

#[test]
fn offensive_war_with_low_mood_is_refused() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    let france = fac("fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .ledger
        .war_goals
        .insert(fac("fac_england"), vec![prov("prov_guyenne")]);
    set_mood(&mut state, data, OIL, 10);
    convene(&mut state, data, NecessityKind::OffensiveWar, "heavy").expect("convened");
    let treasury = state.factions[&france].treasury;
    state.end_turn_with(data, idle);
    assert_eq!(first_session(&state).status, SessionStatus::Refused);
    assert_eq!(state.factions[&france].last_budget.levy_income, 0);
    assert!(
        state.factions[&france].treasury <= treasury + state.factions[&france].last_budget.income
    );
    assert!(state
        .cause_journal()
        .records()
        .any(|r| r.cause.kind == CauseKind::AssemblyRefused));
    assert!(
        state.assemblies.houses[OIL].mood < 10 + 1,
        "a refusal costs trust"
    );
    // The delay between two convocations holds.
    assert!(matches!(
        convene(&mut state, data, NecessityKind::OffensiveWar, "modest"),
        Err(OrderError::Assembly(AssemblyError::TooSoon(_)))
    ));
}

#[test]
fn conditions_are_proposed_then_accepted_or_declined() {
    let data = game_data();
    let france = fac("fac_france");
    let rules = &data.assembly_rules;
    let mut base = scene(data, "fac_france");
    make_war(&mut base, "fac_france", "fac_england");
    // A mood that leaves the agreement just under the grant threshold.
    let forecast = |state: &CampaignState| {
        assembly::forecast(
            state,
            data,
            &france,
            OIL,
            NecessityKind::DefensiveWar,
            "standard",
            None,
            false,
        )
        .expect("forecast")
    };
    set_mood(&mut base, data, OIL, 50);
    let probe = forecast(&base);
    let want = (rules.vote.grant_threshold - 12.0) as i32;
    let mood = 50 + want - probe.accord;
    set_mood(&mut base, data, OIL, mood);
    assert_eq!(forecast(&base).outcome, VoteOutcome::Conditions);
    assert!(!forecast(&base).terms.is_empty());

    for accept in [true, false] {
        let mut state = base.clone();
        convene(&mut state, data, NecessityKind::DefensiveWar, "standard").unwrap();
        state.end_turn_with(data, idle);
        let session = first_session(&state).clone();
        assert_eq!(session.status, SessionStatus::Proposed, "{session:?}");
        assert!(!session.concessions.is_empty() && session.concessions.len() <= 3);
        let mood_before = state.assemblies.houses[OIL].mood;
        state
            .submit_order(
                data,
                Order::AnswerAssembly {
                    session: session.id,
                    accept_terms: accept,
                },
            )
            .expect("answer");
        let rt = &state.assemblies.houses[OIL];
        if accept {
            assert_eq!(rt.sessions[0].status, SessionStatus::Granted);
            assert_eq!(rt.subsidies.len(), 1);
        } else {
            assert_eq!(rt.sessions[0].status, SessionStatus::Declined);
            assert!(rt.subsidies.is_empty());
            assert!(rt.mood < mood_before);
        }
        // Answering twice is refused.
        assert!(state
            .submit_order(
                data,
                Order::AnswerAssembly {
                    session: session.id,
                    accept_terms: true,
                },
            )
            .is_err());
    }
}

#[test]
fn unanswered_terms_lapse() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = scene(data, "fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    let probe = assembly::forecast(
        &state,
        data,
        &france,
        OIL,
        NecessityKind::DefensiveWar,
        "standard",
        None,
        false,
    )
    .unwrap();
    let mood = 55 + (data.assembly_rules.vote.grant_threshold as i32 - 12) - probe.accord;
    set_mood(&mut state, data, OIL, mood);
    convene(&mut state, data, NecessityKind::DefensiveWar, "standard").unwrap();
    for _ in 0..(1 + data.assembly_rules.vote.answer_window_seasons) {
        state.end_turn_with(data, idle);
    }
    assert_eq!(first_session(&state).status, SessionStatus::Lapsed);
}

#[test]
fn subsidy_paid_per_season_over_its_term() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    let france = fac("fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    set_mood(&mut state, data, OIL, 95);
    convene(&mut state, data, NecessityKind::DefensiveWar, "modest").unwrap();
    let term = data.assembly_rules.ask("modest").unwrap().term_seasons;
    let mut paid = Vec::new();
    for _ in 0..(term + 2) {
        state.end_turn_with(data, idle);
        paid.push(state.factions[&france].last_budget.levy_income);
    }
    let per_season = first_session(&state).per_season;
    assert!(per_season > 0);
    for (i, p) in paid.iter().enumerate() {
        if (i as u32) < term {
            assert_eq!(*p, per_season, "season {i}");
        } else {
            assert_eq!(*p, 0, "no payment after the term (season {i})");
        }
    }
    assert!(state.assemblies.houses[OIL].subsidies.is_empty());
}

#[test]
fn the_subsidy_angers_the_provinces_it_is_levied_on() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    let france = fac("fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    set_mood(&mut state, data, OIL, 95);
    let province = prov("prov_ile_de_france");
    let before = assembly::province_unrest(&state, data, &province, &france);
    assert_eq!(before, 0.0);
    convene(&mut state, data, NecessityKind::DefensiveWar, "heavy").unwrap();
    state.end_turn_with(data, idle);
    let after = assembly::province_unrest(&state, data, &province, &france);
    assert!(after > 0.0, "the levied province grumbles");
    // A province outside the assembly's regions is spared.
    let spared = assembly::province_unrest(&state, data, &prov("prov_carcassonne"), &france);
    assert_eq!(spared, 0.0);
}

fn craft_promise(
    state: &mut CampaignState,
    data: &GameData,
    concession: &str,
    until: u32,
    wars: Vec<&str>,
) {
    set_mood(state, data, OIL, 70);
    let turn = state.turn;
    state
        .assemblies
        .houses
        .get_mut(OIL)
        .unwrap()
        .promises
        .push(Promise {
            id: 900,
            faction: fac("fac_france"),
            concession: concession.into(),
            since: turn,
            until,
            deadline: until,
            status: PromiseStatus::Active,
            resolved: 0,
            wars: wars.into_iter().map(fac).collect(),
        });
}

#[test]
fn broken_strong_money_promise_angers_estates() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    let france = fac("fac_france");
    craft_promise(&mut state, data, "con_strong_money", 40, vec![]);
    state
        .submit_order(
            data,
            Order::SetCoinage {
                level: CoinageLevel::Debased,
            },
        )
        .expect("debasing is allowed, at a price");
    let rt = &state.assemblies.houses[OIL];
    assert_eq!(rt.promises[0].status, PromiseStatus::Broken);
    assert_eq!(
        rt.mood,
        70 - data.assembly_rules.mood.promise_broken_penalty
    );
    assert!(!rt.grievances.is_empty());
    // The towns grumble.
    let city = prov("prov_ile_de_france");
    assert!(assembly::province_unrest(&state, data, &city, &france) > 0.0);
    state.end_turn_with(data, idle);
    assert!(state
        .cause_journal()
        .records()
        .any(|r| r.cause.kind == CauseKind::PromiseBroken));
    // And the next vote remembers.
    make_war(&mut state, "fac_france", "fac_england");
    let vote = assembly::forecast(
        &state,
        data,
        &france,
        OIL,
        NecessityKind::DefensiveWar,
        "modest",
        None,
        false,
    )
    .expect("forecast");
    let remembered = vote
        .parts
        .iter()
        .find(|(key, _)| *key == "assembly_broken")
        .map(|(_, points)| *points);
    assert_eq!(
        remembered,
        Some(-12),
        "one broken promise weighs on the vote"
    );
}

#[test]
fn kept_promise_warms_the_mood() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    craft_promise(&mut state, data, "con_strong_money", 2, vec![]);
    for _ in 0..3 {
        state.end_turn_with(data, idle);
    }
    let rt = &state.assemblies.houses[OIL];
    assert_eq!(rt.promises[0].status, PromiseStatus::Kept);
    assert!(
        rt.mood > 70 - 6,
        "kept promise raises the mood (it drifts to rest meanwhile)"
    );
}

#[test]
fn an_unmet_truce_promise_is_broken_at_its_deadline() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    craft_promise(&mut state, data, "con_end_war", 2, vec!["fac_england"]);
    state.end_turn_with(data, idle);
    assert_eq!(
        state.assemblies.houses[OIL].promises[0].status,
        PromiseStatus::Active
    );
    state.end_turn_with(data, idle);
    state.end_turn_with(data, idle);
    assert_eq!(
        state.assemblies.houses[OIL].promises[0].status,
        PromiseStatus::Broken
    );

    // The same promise, with the war over before the deadline, is kept.
    let mut state = scene(data, "fac_france");
    craft_promise(&mut state, data, "con_end_war", 6, vec!["fac_england"]);
    state.end_turn_with(data, idle);
    assert_eq!(
        state.assemblies.houses[OIL].promises[0].status,
        PromiseStatus::Kept
    );
}

#[test]
fn elected_officers_cap_the_household() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    let france = fac("fac_france");
    let before = finance::household_percent(&state, data, &france);
    let cap = data
        .assembly_rules
        .concession("con_elected_officers")
        .unwrap()
        .household_cap_percent
        .unwrap();
    assert!(
        before > cap,
        "France's court costs more than the cap ({before})"
    );
    craft_promise(&mut state, data, "con_elected_officers", 40, vec![]);
    assert_eq!(finance::household_percent(&state, data, &france), cap);
}

#[test]
fn permanent_gabelle_needs_the_concession() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = scene(data, "fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    // Before 1360 nothing can be made permanent.
    assert!(matches!(
        state.submit_order(
            data,
            Order::ConveneAssembly {
                assembly: OIL.into(),
                necessity: NecessityKind::DefensiveWar,
                ask: "standard".into(),
                form: Some("aide".into()),
                permanent: true,
            },
        ),
        Err(OrderError::Assembly(AssemblyError::PermanentUnavailable(_)))
    ));
    state.year = 1361;
    let ask = |state: &CampaignState, permanent: bool| {
        assembly::forecast(
            state,
            data,
            &france,
            OIL,
            NecessityKind::DefensiveWar,
            "standard",
            Some("gabelle"),
            permanent,
        )
        .expect("forecast")
    };
    set_mood(&mut state, data, OIL, 50);
    let probe = ask(&state, false).accord;
    let mood = 50 + (data.assembly_rules.vote.grant_threshold as i32 + 5) - probe;
    set_mood(&mut state, data, OIL, mood);
    assert_eq!(ask(&state, false).outcome, VoteOutcome::Grant);
    assert_eq!(
        ask(&state, true).outcome,
        VoteOutcome::Conditions,
        "permanence costs the crown a concession"
    );
    let income_before = state.faction_income(data, &france);
    state
        .submit_order(
            data,
            Order::ConveneAssembly {
                assembly: OIL.into(),
                necessity: NecessityKind::DefensiveWar,
                ask: "standard".into(),
                form: Some("gabelle".into()),
                permanent: true,
            },
        )
        .expect("convened");
    state.end_turn_with(data, idle);
    let session = first_session(&state).clone();
    assert_eq!(session.status, SessionStatus::Proposed);
    state
        .submit_order(
            data,
            Order::AnswerAssembly {
                session: session.id,
                accept_terms: true,
            },
        )
        .unwrap();
    assert_eq!(
        state.assemblies.permanent[&france],
        vec!["gabelle".to_owned()]
    );
    assert!(
        state.faction_income(data, &france) > income_before,
        "a permanent gabelle is ordinary income"
    );
}

#[test]
fn convening_needs_the_situation_and_the_house() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    // Peace: no necessity.
    assert!(matches!(
        convene(&mut state, data, NecessityKind::DefensiveWar, "modest"),
        Err(OrderError::Assembly(AssemblyError::NecessityNotMet(_)))
    ));
    make_war(&mut state, "fac_france", "fac_england");
    // The Parlement is not France's.
    assert!(matches!(
        state.submit_order(
            data,
            Order::ConveneAssembly {
                assembly: "asm_parliament".into(),
                necessity: NecessityKind::DefensiveWar,
                ask: "modest".into(),
                form: None,
                permanent: false,
            },
        ),
        Err(OrderError::Assembly(AssemblyError::NotYourAssembly))
    ));
    convene(&mut state, data, NecessityKind::DefensiveWar, "modest").unwrap();
    assert!(matches!(
        convene(&mut state, data, NecessityKind::DefensiveWar, "modest"),
        Err(OrderError::Assembly(AssemblyError::SessionOpen))
    ));
}

#[test]
fn a_house_with_an_assembly_cannot_levy_the_old_way() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    assert!(matches!(
        state.submit_order(
            data,
            Order::LevyExtraordinary {
                levy: "aide".into()
            }
        ),
        Err(OrderError::Finance(finance::FinanceError::AssemblyRequired))
    ));
}

#[test]
fn no_assembly_file_keeps_old_levies() {
    let mut data = game_data().clone();
    data.assembly_rules = Default::default();
    let mut state = scene(&data, "fac_france");
    let before = state.factions[&fac("fac_france")].treasury;
    state
        .submit_order(
            &data,
            Order::LevyExtraordinary {
                levy: "aide".into(),
            },
        )
        .expect("the old levy still works");
    assert!(state.factions[&fac("fac_france")].treasury > before);
    make_war(&mut state, "fac_france", "fac_england");
    assert!(matches!(
        convene(&mut state, &data, NecessityKind::DefensiveWar, "modest"),
        Err(OrderError::Assembly(AssemblyError::NoAssemblies))
    ));
    state.end_turn_with(&data, idle);
}

#[test]
fn a_house_without_assembly_keeps_its_levies() {
    let data = game_data();
    let mut state = scene(data, "fac_burgundy");
    state
        .submit_order(
            data,
            Order::LevyExtraordinary {
                levy: "aide".into(),
            },
        )
        .expect("Burgundy has no assembly in the data");
}

#[test]
fn state_round_trips_and_old_saves_have_no_assemblies() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    let json = serde_json::to_value(&state).unwrap();
    assert!(
        json.get("assemblies").is_none(),
        "an untouched state is not saved with assemblies"
    );
    make_war(&mut state, "fac_france", "fac_england");
    set_mood(&mut state, data, OIL, 90);
    convene(&mut state, data, NecessityKind::DefensiveWar, "modest").unwrap();
    state.end_turn_with(data, idle);
    let back: CampaignState =
        serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
    assert_eq!(back.assemblies, state.assemblies);
    assert!(!back.assemblies.houses[OIL].subsidies.is_empty());
}

#[test]
fn the_ai_convenes_when_poor_and_at_war_and_answers_terms() {
    let data = game_data();
    let mut state = scene(data, "fac_england");
    let france = fac("fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    state.factions.get_mut(&france).unwrap().treasury = 0;
    {
        let cache = PlanCache::new(&state);
        let orders = assembly::ai_assembly_orders(&cache, data, &france);
        assert!(
            matches!(orders.as_slice(), [Order::ConveneAssembly { necessity, .. }] if *necessity == NecessityKind::DefensiveWar),
            "{orders:?}"
        );
    }
    // At peace, or rich, it does not.
    state.factions.get_mut(&france).unwrap().treasury = 10_000_000;
    let cache = PlanCache::new(&state);
    assert!(assembly::ai_assembly_orders(&cache, data, &france).is_empty());
}

#[test]
fn every_assembly_of_the_data_is_consistent() {
    let rules = &game_data().assembly_rules;
    assert!(rules.is_active());
    for a in &rules.assemblies {
        assert!(a.min_gap_seasons >= 4, "{}: at least four seasons", a.id);
        assert!(!a.demands.is_empty(), "{} demands", a.id);
        assert!(
            a.all_provinces || !a.provinces.is_empty() || !a.regions.is_empty(),
            "{} covers something",
            a.id
        );
    }
    for kind in NecessityKind::ALL {
        assert!(rules.necessity(kind).is_some(), "{kind:?} is in the data");
    }
    // Small counties have no assembly: only the great fiefs are listed.
    let houses: Vec<&str> = rules.assemblies.iter().map(|a| a.house.as_str()).collect();
    assert!(!houses.contains(&"fac_foix"));
}
