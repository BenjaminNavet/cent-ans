//! ECO-R1c (« L'impôt consenti : IA et Angleterre »): the wool custom and the poll tax of the
//! English Parliament, the AI that convenes on the forecast of its war, and the AI that keeps
//! its word to the estates.

use data_model::test_support::{fac, game_data};
use data_model::{GameData, NecessityKind};
use sim_campaign::assembly::{
    self, AssemblyRuntime, Promise, PromiseStatus, Session, SessionStatus, VoteOutcome,
};
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::test_support::start_quiet;
use sim_campaign::{CampaignState, Order};

const PARLIAMENT: &str = "asm_parliament";
const OIL: &str = "asm_etats_oil";

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

fn runtime<'a>(
    state: &'a mut CampaignState,
    data: &GameData,
    assembly: &str,
) -> &'a mut AssemblyRuntime {
    let start = data.assembly_rules.assembly(assembly).unwrap().start_mood;
    state
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
        })
}

fn promise(state: &mut CampaignState, data: &GameData, faction: &str, concession: &str) {
    let turn = state.turn;
    runtime(state, data, OIL).promises.push(Promise {
        id: 901,
        faction: fac(faction),
        concession: concession.into(),
        since: turn,
        until: turn + 12,
        deadline: turn + 12,
        status: PromiseStatus::Active,
        resolved: 0,
        wars: vec![fac("fac_england")],
    });
}

fn forecast(
    state: &CampaignState,
    data: &GameData,
    form: &str,
) -> Result<assembly::Vote, assembly::AssemblyError> {
    assembly::forecast(
        state,
        data,
        &fac("fac_england"),
        PARLIAMENT,
        NecessityKind::DefensiveWar,
        "standard",
        Some(form),
        false,
    )
}

#[test]
fn english_wool_custom_falls_under_flemish_embargo() {
    let data = game_data();
    let mut state = scene(data, "fac_england");
    let england = fac("fac_england");
    make_war(&mut state, "fac_england", "fac_france");
    // 1337: Flanders embargoes the English wool (the Flemish embargo of 1336); lifted, it sails.
    assert_eq!(
        sim_campaign::trade::export_openness(&state, data, &england, "res_wool"),
        0.0,
        "closed at the start of the campaign"
    );
    for f in state.factions.values_mut() {
        f.embargoes.clear();
    }
    let open = sim_campaign::trade::export_openness(&state, data, &england, "res_wool");
    assert!(open > 0.5, "the wool sails at the start: {open}");
    let before = forecast(&state, data, "wool_custom").expect("wool custom");
    assert!(before.per_season > 0, "{before:?}");
    // Flanders embargoes the English wool: the custom dries up and the Parliament refuses it.
    state
        .factions
        .get_mut(&fac("fac_flanders"))
        .unwrap()
        .embargoes
        .insert(england.clone());
    let closed = sim_campaign::trade::export_openness(&state, data, &england, "res_wool");
    assert!(closed < open, "{closed} < {open}");
    let after = forecast(&state, data, "wool_custom").expect("wool custom");
    assert!(after.per_season < before.per_season, "{after:?}");
    // At war with Flanders no wool is left to levy.
    make_war(&mut state, "fac_england", "fac_flanders");
    assert_eq!(
        sim_campaign::trade::export_openness(&state, data, &england, "res_wool"),
        0.0
    );
    let dead = forecast(&state, data, "wool_custom").expect("wool custom");
    assert_eq!(dead.per_season, 0);
    assert_eq!(dead.outcome, VoteOutcome::Refuse);
}

#[test]
fn wool_custom_is_levied_on_the_wool_provinces_only() {
    let data = game_data();
    let mut state = scene(data, "fac_england");
    make_war(&mut state, "fac_england", "fac_france");
    let wool = forecast(&state, data, "wool_custom").expect("wool custom");
    let lay = forecast(&state, data, "lay_subsidy").expect("lay subsidy");
    assert!(!wool.shares.is_empty());
    assert!(wool.shares.len() < lay.shares.len());
    for share in &wool.shares {
        let province = &data.provinces[&share.province];
        assert!(province.resources.iter().any(|r| r.as_str() == "res_wool"));
    }
}

#[test]
fn the_poll_tax_comes_in_1377_and_weighs_on_the_peasants() {
    let data = game_data();
    let mut state = scene(data, "fac_england");
    make_war(&mut state, "fac_england", "fac_france");
    state.year = 1370;
    assert!(
        forecast(&state, data, "poll_tax").is_err(),
        "not before 1377"
    );
    state.year = 1378;
    let poll = forecast(&state, data, "poll_tax").expect("poll tax");
    let lay = forecast(&state, data, "lay_subsidy").expect("lay subsidy");
    assert!(poll.per_season > lay.per_season, "strong yield");
    let rules = &data.assembly_rules;
    let (p, l) = (
        rules.form("poll_tax").unwrap(),
        rules.form("lay_subsidy").unwrap(),
    );
    assert!(p.rural_weight > l.rural_weight, "heavier on the villages");
}

#[test]
fn ai_convenes_before_a_campaign() {
    let data = game_data();
    let mut state = scene(data, "fac_france");
    let england = fac("fac_england");
    make_war(&mut state, "fac_england", "fac_france");
    // A treasury that would pass for comfortable in peace, but not for a war: the forecast of
    // the war, not an empty chest, calls the Parliament.
    let gross = state.faction_income(data, &england).max(1);
    state.factions.get_mut(&england).unwrap().treasury = 5 * gross;
    let cache = PlanCache::new(&state);
    let orders = assembly::ai_assembly_orders(&cache, data, &england);
    assert!(
        matches!(orders.as_slice(), [Order::ConveneAssembly { .. }]),
        "{orders:?}"
    );
    // The same treasury in peace calls nobody.
    let mut peace = scene(data, "fac_france");
    peace.factions.get_mut(&england).unwrap().treasury = 5 * gross;
    let cache = PlanCache::new(&peace);
    assert!(assembly::ai_assembly_orders(&cache, data, &england).is_empty());
}

#[test]
fn ai_keeps_promises_when_honourable() {
    let data = game_data();
    let mut state = scene(data, "fac_england");
    let france = fac("fac_france");
    make_war(&mut state, "fac_england", "fac_france");
    promise(&mut state, data, "fac_france", "con_strong_money");
    let gross = state.faction_income(data, &france).max(1);
    // A hollow chest is not a numbered need: the money stays strong.
    state.factions.get_mut(&france).unwrap().treasury = -gross / 2;
    {
        let cache = PlanCache::new(&state);
        assert!(!assembly::ai_may_debase(&cache, data, &france));
        let orders = sim_campaign::coinage::ai_choose_coinage(&cache, data, &france);
        assert!(
            !orders.iter().any(|o| matches!(
                o,
                Order::SetCoinage { level } if (*level as u8) >= (sim_campaign::coinage::CoinageLevel::Debased as u8)
            )),
            "{orders:?}"
        );
    }
    // A debt of several seasons is: the prince breaks his word.
    state.factions.get_mut(&france).unwrap().treasury = -10 * gross;
    let cache = PlanCache::new(&state);
    assert!(assembly::ai_may_debase(&cache, data, &france));
    // Nobody is held to a promise he never made.
    let mut free = scene(data, "fac_england");
    free.factions.get_mut(&france).unwrap().treasury = -gross / 2;
    let cache = PlanCache::new(&free);
    assert!(assembly::ai_may_debase(&cache, data, &france));
}

#[test]
fn ai_sues_for_the_peace_it_promised() {
    let data = game_data();
    let mut state = scene(data, "fac_england");
    make_war(&mut state, "fac_england", "fac_france");
    assert!(!assembly::peace_promised(&state, data, &fac("fac_france")));
    promise(&mut state, data, "fac_france", "con_end_war");
    assert!(assembly::peace_promised(&state, data, &fac("fac_france")));
}

#[test]
fn ai_declines_a_truce_it_cannot_keep_unless_in_need() {
    let data = game_data();
    let mut state = scene(data, "fac_england");
    let france = fac("fac_france");
    make_war(&mut state, "fac_france", "fac_england");
    // France fights a war of conquest (it has war goals): it cannot promise the end of it.
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .ledger
        .war_goals
        .insert(
            fac("fac_england"),
            vec![data_model::ProvinceId::new("prov_kent").unwrap()],
        );
    let gross = state.faction_income(data, &france).max(1);
    state.factions.get_mut(&france).unwrap().treasury = 10 * gross;
    runtime(&mut state, data, OIL).sessions.push(Session {
        id: 77,
        faction: france.clone(),
        necessity: NecessityKind::OffensiveWar,
        ask: "standard".into(),
        form: "aide".into(),
        permanent: false,
        convened_turn: 0,
        status: SessionStatus::Proposed,
        accord: 60,
        per_season: 1000,
        term: 8,
        shares: vec![],
        concessions: vec!["con_end_war".into()],
        answered_turn: Some(0),
    });
    let answer = |state: &CampaignState| {
        let cache = PlanCache::new(state);
        assembly::ai_assembly_orders(&cache, data, &france)
            .into_iter()
            .find_map(|o| match o {
                Order::AnswerAssembly { accept_terms, .. } => Some(accept_terms),
                _ => None,
            })
            .expect("an answer")
    };
    assert!(!answer(&state), "an honourable prince does not promise it");
    state.factions.get_mut(&france).unwrap().treasury = 0;
    assert!(answer(&state), "a numbered need accepts and will break");
}
