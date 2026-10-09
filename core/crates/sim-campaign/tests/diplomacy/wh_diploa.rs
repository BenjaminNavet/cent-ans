//! Lot WH `diploa` (ADR 0278): readable diplomacy — war preview forecast,
//! attitude band and timed reasons, faction sheet links, revoking a military
//! access, breaches in the treaty history.

use data_model::test_support::{fac, game_data};
use sim_campaign::diplomacy::{call_to_arms_forecast, CallForecast};
use sim_campaign::negotiation::{has_military_access, Rupture};
use sim_campaign::{CampaignState, OpinionModifier};

fn start(seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(game_data(), fac("fac_france"), seed).expect("start");
    state.chronicle.disabled = true;
    state
}

fn ally(state: &mut CampaignState, a: &str, b: &str) {
    for (x, y) in [(a, b), (b, a)] {
        let f = state.factions.get_mut(&fac(x)).unwrap();
        f.at_war_with.remove(&fac(y));
        f.allies.insert(fac(y));
    }
}

fn goodwill(state: &mut CampaignState, holder: &str, with: &str, value: i32, turns: u32) {
    let expires_turn = state.turn + turns;
    state
        .factions
        .get_mut(&fac(holder))
        .unwrap()
        .modifiers
        .push(OpinionModifier {
            with: fac(with),
            value,
            reason_fr: "Bonne volonté".to_owned(),
            expires_turn,
        });
}

#[test]
fn attitude_bands_come_from_the_data() {
    let rules = &game_data().diplomacy_rules;
    assert_eq!(rules.attitude_band(-90), "Hostile");
    assert_eq!(rules.attitude_band(-30), "Défiant");
    assert_eq!(rules.attitude_band(0), "Réservé");
    assert_eq!(rules.attitude_band(25), "Cordial");
    assert_eq!(rules.attitude_band(55), "Amical");
    assert_eq!(rules.attitude_band(100), "Dévoué");
}

#[test]
fn forecast_tells_joins_hesitates_refuses() {
    let data = game_data();
    let mut state = start(5);
    let (england, portugal, castile) =
        (fac("fac_england"), fac("fac_portugal"), fac("fac_castile"));
    state.factions.get_mut(&england).unwrap().treasury = 100_000;
    state.factions.get_mut(&england).unwrap().ledger.weariness = 0;
    goodwill(&mut state, "fac_england", "fac_portugal", 80, 40);
    assert_eq!(
        call_to_arms_forecast(&state, data, &england, &portugal, &castile),
        CallForecast::Joins
    );
    // Attitude pushed to -10: no longer marches, still not hostile.
    let now = state.attitude(data, &england, &portugal).0;
    goodwill(&mut state, "fac_england", "fac_portugal", -10 - now, 40);
    assert_eq!(
        call_to_arms_forecast(&state, data, &england, &portugal, &castile),
        CallForecast::Hesitates
    );
    // Hostile: refuses.
    goodwill(&mut state, "fac_england", "fac_portugal", -90, 40);
    assert!(matches!(
        call_to_arms_forecast(&state, data, &england, &portugal, &castile),
        CallForecast::Refuses(_)
    ));
    // Broke: refuses whatever the attitude, with the reason.
    goodwill(&mut state, "fac_england", "fac_portugal", 200, 40);
    state.factions.get_mut(&england).unwrap().treasury = -1;
    assert_eq!(
        call_to_arms_forecast(&state, data, &england, &portugal, &castile),
        CallForecast::Refuses("trésorerie vide")
    );
}

#[test]
fn modifier_turns_left_count_down() {
    let data = game_data();
    let mut state = start(6);
    goodwill(&mut state, "fac_england", "fac_france", 20, 40);
    let left = |s: &CampaignState| {
        s.diplomacy_view(data, &fac("fac_france"))
            .into_iter()
            .find(|e| e.faction == fac("fac_england"))
            .unwrap()
            .reason_turns
            .get("Bonne volonté")
            .copied()
    };
    assert_eq!(left(&state), Some(40));
    state.turn += 1;
    assert_eq!(left(&state), Some(39));
    state.turn += 39;
    assert_eq!(left(&state), None);
}

#[test]
fn sheet_lists_allies_enemies_and_vassals() {
    let data = game_data();
    let mut state = start(7);
    for f in ["fac_scotland", "fac_navarre", "fac_hainaut"] {
        state
            .factions
            .get_mut(&fac("fac_england"))
            .unwrap()
            .allies
            .remove(&fac(f));
    }
    ally(&mut state, "fac_england", "fac_portugal");
    ally(&mut state, "fac_england", "fac_hainaut");
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .at_war_with
        .insert(fac("fac_castile"));
    state
        .factions
        .get_mut(&fac("fac_navarre"))
        .unwrap()
        .suzerain = Some(fac("fac_england"));
    let entry = state
        .diplomacy_view(data, &fac("fac_france"))
        .into_iter()
        .find(|e| e.faction == fac("fac_england"))
        .unwrap();
    assert!(entry.allies.contains(&fac("fac_hainaut")));
    assert!(entry.allies.contains(&fac("fac_portugal")));
    let mut sorted = entry.allies.clone();
    sorted.sort();
    assert_eq!(entry.allies, sorted, "stable order");
    assert!(entry.enemies.contains(&fac("fac_castile")));
    assert!(entry.vassals.contains(&fac("fac_navarre")));
    assert!(!entry.attitude_band.is_empty());
}

#[test]
fn revoking_access_removes_passage_costs_attitude_and_is_recorded() {
    let data = game_data();
    let mut state = start(8);
    let (host, guest) = (fac("fac_france"), fac("fac_england"));
    assert!(state.revoke_military_access(data, &host, &guest).is_err());
    state
        .factions
        .get_mut(&host)
        .unwrap()
        .ledger
        .military_access
        .insert(guest.clone());
    assert!(has_military_access(&state, &host, &guest));
    let before = state.attitude(data, &guest, &host).0;
    state.revoke_military_access(data, &host, &guest).unwrap();
    assert!(!has_military_access(&state, &host, &guest));
    let rules = &data.diplomacy_rules.revoke_access;
    assert_eq!(
        state.attitude(data, &guest, &host).0,
        before + rules.attitude
    );
    for me in [&host, &guest] {
        let record = state.factions[me].ledger.history.last().unwrap();
        assert_eq!(record.rupture, Some(Rupture::Broken));
        assert!(!record.accepted);
    }
    // A second revocation has nothing to revoke.
    assert!(state.revoke_military_access(data, &host, &guest).is_err());
}

#[test]
fn breaches_enter_the_history() {
    let data = game_data();
    let mut state = start(9);
    let (fr, en, sc) = (fac("fac_france"), fac("fac_england"), fac("fac_scotland"));
    // Broken alliance.
    ally(&mut state, "fac_france", "fac_castile");
    state
        .break_alliance(data, &fr, &fac("fac_castile"))
        .unwrap();
    let last = state.factions[&fr].ledger.history.last().unwrap();
    assert_eq!(last.rupture, Some(Rupture::Broken));
    assert_eq!(last.articles, vec!["alliance".to_owned()]);
    // Perjury: war in the middle of a truce.
    for (a, b) in [(&fr, &en), (&en, &fr)] {
        let until = state.turn + 10;
        let f = state.factions.get_mut(a).unwrap();
        f.at_war_with.remove(b);
        f.allies.remove(b);
        f.truces.insert(b.clone(), until);
    }
    state.declare_war(data, &fr, &en).unwrap();
    let perjury = state.factions[&en]
        .ledger
        .history
        .iter()
        .find(|r| r.rupture == Some(Rupture::Perjury))
        .expect("perjury recorded");
    assert_eq!(perjury.with, fr);
    // A refused call to arms: the ally of the defender is broke.
    ally(&mut state, "fac_scotland", "fac_england");
    {
        let f = state.factions.get_mut(&sc).unwrap();
        f.treasury = -1;
        f.at_war_with.remove(&fr);
        f.allies.remove(&fr);
    }
    state.factions.get_mut(&fr).unwrap().allies.remove(&sc);
    state.factions.get_mut(&en).unwrap().at_war_with.remove(&fr);
    state.factions.get_mut(&fr).unwrap().at_war_with.remove(&en);
    state.factions.get_mut(&fr).unwrap().truces.remove(&en);
    state.factions.get_mut(&en).unwrap().truces.remove(&fr);
    state.declare_war(data, &fr, &en).unwrap();
    assert!(state.factions[&en]
        .ledger
        .history
        .iter()
        .any(|r| r.rupture == Some(Rupture::RefusedCall) && r.with == sc));
}

#[test]
fn old_treaty_records_load_without_rupture() {
    let json = r#"{"turn":3,"with":"fac_france","proposed":true,"accepted":true,"articles":["peace"],"text_fr":"x"}"#;
    let record: sim_campaign::negotiation::TreatyRecord = serde_json::from_str(json).unwrap();
    assert_eq!(record.rupture, None);
}
