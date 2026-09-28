//! FE deductions (spec § 3.3), obligations and loyalty (§ 4.1-4.2): deductions (F0), suzerain view, maxim, tribute and loyalty (F1).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), 7).expect("1337 start")
}

use sim_campaign::feudal;

#[test]
fn guyenne_owes_allegiance_to_england_then_france() {
    let data = data();
    let state = start(&data);
    assert_eq!(
        feudal::province_lieges(&state, &data, &prov("prov_guyenne")),
        vec![fac("fac_england"), fac("fac_france")]
    );
    assert_eq!(feudal::liege_of(&state, &data, &fac("fac_england")), None);
    assert!(feudal::title_vassals(&state, &data, &fac("fac_france")).contains(&fac("fac_england")));
}

#[test]
fn deduced_liege_matches_the_1337_suzerains() {
    let data = data();
    let state = start(&data);
    for (id, faction) in &data.factions {
        if let Some(suzerain) = &faction.suzerain {
            assert_eq!(
                feudal::liege_of(&state, &data, id).as_ref(),
                Some(suzerain),
                "{id}"
            );
        }
    }
    let vassals = feudal::direct_vassals(&state, &data, &fac("fac_france"));
    for v in ["fac_brittany", "fac_burgundy", "fac_flanders"] {
        assert!(vassals.contains(&fac(v)), "{v}");
    }
}

/// Navarre pays homage to Burgundy: a count (Angoumois' holder) below a
/// duke below the king.
fn with_rear_vassal(data: &GameData) -> CampaignState {
    let mut state = start(data);
    assert!(feudal::pay_homage(
        &mut state,
        data,
        &fac("fac_navarre"),
        &fac("fac_burgundy")
    ));
    state
}

#[test]
fn state_suzerain_is_a_view_of_the_titles() {
    let data = data();
    let mut state = start(&data);
    for id in state.factions.keys() {
        assert_eq!(
            state.factions[id].suzerain,
            feudal::liege_of(&state, &data, id),
            "{id}"
        );
    }
    // Princes of the Empire are deduced vassals of the Emperor.
    for p in ["fac_austria", "fac_bohemia", "fac_brabant", "fac_verona"] {
        assert_eq!(
            state.factions[&fac(p)].suzerain,
            Some(fac("fac_empire")),
            "{p}"
        );
    }
    // Homage changes the effective liege of the primary title, not the data.
    let scotland = fac("fac_scotland");
    assert!(feudal::pay_homage(
        &mut state,
        &data,
        &scotland,
        &fac("fac_france")
    ));
    assert_eq!(state.factions[&scotland].suzerain, Some(fac("fac_france")));
    let tit_scotland = data.factions[&scotland].primary_title.clone().unwrap();
    assert_eq!(data.titles[&tit_scotland].de_jure_liege, None);
    assert!(state.feudal.liege_overrides.contains_key(&tit_scotland));
    // Survives a save.
    let loaded = CampaignState::load_json(&state.save_json()).expect("round trip");
    assert_eq!(
        feudal::liege_of(&loaded, &data, &scotland),
        Some(fac("fac_france"))
    );
    // Release: sovereign again.
    state
        .release_vassal(&data, &fac("fac_france"), &fac("fac_brittany"))
        .expect("release");
    assert_eq!(feudal::liege_of(&state, &data, &fac("fac_brittany")), None);
    assert_eq!(state.factions[&fac("fac_brittany")].suzerain, None);
    // A pre-F1 save: its stored suzerain is adopted as an override.
    let mut legacy = start(&data);
    legacy.feudal.derived = false;
    legacy.factions.get_mut(&scotland).unwrap().suzerain = Some(fac("fac_france"));
    feudal::sync_suzerains(&mut legacy, &data);
    assert!(legacy.feudal.derived);
    assert_eq!(
        feudal::liege_of(&legacy, &data, &scotland),
        Some(fac("fac_france"))
    );
    // No cycle: a liege paying homage to its own vassal is freed first.
    let mut cycle = start(&data);
    assert!(feudal::pay_homage(
        &mut cycle,
        &data,
        &fac("fac_france"),
        &fac("fac_brittany")
    ));
    assert_eq!(feudal::liege_of(&cycle, &data, &fac("fac_brittany")), None);
    assert_eq!(
        feudal::liege_of(&cycle, &data, &fac("fac_france")),
        Some(fac("fac_brittany"))
    );
}

#[test]
fn king_does_not_levy_his_dukes_counts() {
    let data = data();
    let mut state = with_rear_vassal(&data);
    let (france, burgundy, navarre) = (fac("fac_france"), fac("fac_burgundy"), fac("fac_navarre"));
    assert_eq!(
        feudal::liege_chain(&state, &data, &navarre),
        vec![burgundy.clone(), france.clone()]
    );
    assert!(!feudal::direct_vassals(&state, &data, &france).contains(&navarre));
    let tree = feudal::feudal_tree(&state, &data, &france);
    let burgundy_node = tree
        .vassals
        .iter()
        .find(|n| n.faction == burgundy)
        .expect("duke");
    assert!(burgundy_node.vassals.iter().any(|n| n.faction == navarre));
    // France goes to war: its loyal duke follows, the duke's count does not.
    let target = fac("fac_holstein");
    for f in [&burgundy, &navarre] {
        let s = state.factions.get_mut(f).unwrap();
        s.loyalty = 100;
        s.allies.clear();
    }
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .allies
        .remove(&navarre);
    state.declare_war(&data, &france, &target).expect("war");
    assert!(state.is_at_war(&burgundy, &target));
    assert!(!state.is_at_war(&navarre, &target));
}

#[test]
fn tribute_goes_to_the_direct_liege_only() {
    let data = data();
    let mut state = with_rear_vassal(&data);
    let navarre = fac("fac_navarre");
    state.factions.get_mut(&navarre).unwrap().income_last_turn = 1000;
    let expected = 1000 * data.feudal_rules.vassal_tribute_percent / 100;
    assert_eq!(
        feudal::tribute_due(&state, &data, &navarre),
        Some((fac("fac_burgundy"), expected))
    );
    // Sovereigns owe nothing.
    assert_eq!(feudal::tribute_due(&state, &data, &fac("fac_france")), None);
    assert_eq!(
        feudal::tribute_due(&state, &data, &fac("fac_england")),
        None
    );
}

#[test]
fn loyalty_thresholds_come_from_the_rules() {
    let mut data = data();
    let mut state = start(&data);
    let (france, burgundy, brittany) =
        (fac("fac_france"), fac("fac_burgundy"), fac("fac_brittany"));
    let target = |state: &CampaignState, data: &GameData| {
        i32::from(sim_campaign::diplomacy::loyalty_target(
            state, data, &burgundy, &france,
        ))
    };
    // Keep away from the 0-100 clamp.
    data.feudal_rules.loyalty.base = 40;
    let before = target(&state, &data);
    data.feudal_rules.loyalty.base = 30;
    assert_eq!(target(&state, &data), before - 10);
    // Shared culture (Burgundy and France are French) is weighed from the rules.
    let culture = data.feudal_rules.loyalty.shared_culture;
    data.feudal_rules.loyalty.shared_culture = culture + 7;
    assert_eq!(target(&state, &data), before - 10 + 7);
    data.feudal_rules.loyalty.shared_culture = culture;
    // Remembered events weigh until `memory_turns`.
    let base = target(&state, &data);
    let w = data.feudal_rules.loyalty.clone();
    feudal::record_title_grant(&mut state, &data, &france, &burgundy);
    assert_eq!(target(&state, &data), base + w.title_granted);
    feudal::record_liege_defeat(&mut state, &data, &france);
    assert_eq!(
        target(&state, &data),
        base + w.title_granted + w.liege_defeat
    );
    // A peer's forfeiture weighs on the others, not on the felon.
    feudal::record_peer_forfeiture(&mut state, &data, &france, &brittany);
    let events = &state.feudal.loyalty_events;
    assert!(events
        .iter()
        .any(|e| e.vassal == burgundy && e.kind == feudal::LoyaltyEventKind::PeerForfeiture));
    assert!(!events
        .iter()
        .any(|e| e.vassal == brittany && e.kind == feudal::LoyaltyEventKind::PeerForfeiture));
    state.turn += w.memory_turns;
    feudal::forget_expired(&mut state);
    assert!(state.feudal.loyalty_events.is_empty());
    assert_eq!(target(&state, &data), base);
    // The call-to-arms threshold comes from the rules too.
    let holstein = fac("fac_holstein");
    let threshold = data.feudal_rules.call_to_arms_loyalty;
    for (loyalty, follows) in [(threshold - 1, false), (threshold, true)] {
        let mut war = start(&data);
        war.factions.get_mut(&burgundy).unwrap().loyalty = loyalty;
        war.declare_war(&data, &france, &holstein).expect("war");
        assert_eq!(war.is_at_war(&burgundy, &holstein), follows, "{loyalty}");
    }
}

#[test]
fn title_holdings_survive_a_save_and_older_saves_are_refused() {
    let data = data();
    let state = start(&data);
    assert!(!state.feudal.holders.is_empty());
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).expect("round trip");
    assert_eq!(loaded.feudal, state.feudal);
    let old = json.replacen("\"state_version\":8", "\"state_version\":6", 1);
    assert!(matches!(
        CampaignState::load_json(&old),
        Err(sim_campaign::save::CampaignError::PreFeudalSave { found: 6, .. })
    ));
    // Lot OM1 (ADR 0115): a save of the old 4096² map is refused with a clear message.
    let before_om = json.replacen("\"state_version\":8", "\"state_version\":7", 1);
    let err = CampaignState::load_json(&before_om).unwrap_err();
    assert!(matches!(
        err,
        sim_campaign::save::CampaignError::PreWideMapSave { found: 7, .. }
    ));
    assert!(err.to_string().contains("Oural"));
}
