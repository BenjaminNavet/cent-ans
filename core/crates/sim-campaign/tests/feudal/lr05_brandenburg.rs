//! Lot LR-05: every playable faction of 1337 holds at least one province at
//! turn 0 (the March of Brandenburg was stillborn from FE0 until HV8 gave its
//! capital back; these tests keep it that way).

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::CampaignState;

use data_model::test_support::game_data;

#[test]
fn brandenburg_holds_the_march_at_turn_0() {
    let data = game_data();
    let state =
        CampaignState::new_1337(data, FactionId::new("fac_papacy").unwrap(), 1).expect("setup");
    let march = ProvinceId::new("prov_brandenburg").unwrap();
    assert_eq!(
        state.province_owner(&march).map(|f| f.as_str()),
        Some("fac_brandenburg")
    );
}

#[test]
fn every_playable_faction_holds_a_province_at_turn_0() {
    let data = game_data();
    let state =
        CampaignState::new_1337(data, FactionId::new("fac_papacy").unwrap(), 1).expect("setup");
    let landless: Vec<&str> = data
        .factions
        .values()
        .filter(|f| f.playable)
        // The crusade starts landless by design (ADR 0165).
        .filter(|f| f.id.as_str() != "fac_crusaders")
        .filter(|f| {
            !state
                .provinces
                .keys()
                .any(|p| state.province_owner(p) == Some(&f.id))
        })
        .map(|f| f.id.as_str())
        .collect();
    assert!(landless.is_empty(), "factions sans province : {landless:?}");
}

fn kill_last_of_line(state: &mut CampaignState, data: &GameData, faction: &str) -> Vec<String> {
    let faction = FactionId::new(faction).unwrap();
    let ruler = state.factions[&faction].ruler.clone().expect("ruler");
    for (id, c) in state.characters.iter_mut() {
        if *id != ruler && c.faction == faction {
            c.alive = false;
        }
    }
    state.factions.get_mut(&faction).unwrap().heir = None;
    let mut events = Vec::new();
    sim_campaign::characters::kill(state, data, &ruler, &mut events);
    events.into_iter().map(|e| e.text_fr).collect()
}

/// LR-05: a see, an order or a republic elects a new head; it never
/// escheats to its liege as an extinct dynasty would.
#[test]
fn an_elective_realm_elects_instead_of_escheating() {
    let mut data = game_data().clone();
    data.feudal_rules.collateral_line_percent = 0;
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_papacy").unwrap(), 1).expect("setup");
    let riga = FactionId::new("fac_riga_archbishopric").unwrap();
    assert_eq!(
        data.factions[&riga].succession_law,
        data_model::SuccessionLaw::Elective
    );
    let provinces = state.owned_provinces(&riga);
    assert!(!provinces.is_empty());
    let events = kill_last_of_line(&mut state, &data, riga.as_str());
    assert!(state.factions[&riga].alive, "{events:?}");
    assert!(state.factions[&riga].ruler.is_some());
    assert_eq!(state.owned_provinces(&riga), provinces);
    assert!(events.iter().any(|e| e.contains("est élu")), "{events:?}");
}

/// LR-05: with `collateral_line_percent` at 100, a dynasty whose direct line
/// dies out without kin abroad passes to a cadet branch of the same house.
#[test]
fn a_cadet_branch_keeps_the_fief() {
    let mut data = game_data().clone();
    data.feudal_rules.collateral_line_percent = 100;
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_papacy").unwrap(), 1).expect("setup");
    let berg = FactionId::new("fac_berg").unwrap();
    let old = state.factions[&berg].ruler.clone().unwrap();
    let house = state.characters[&old].house.clone();
    let provinces = state.owned_provinces(&berg);
    let events = kill_last_of_line(&mut state, &data, berg.as_str());
    assert!(state.factions[&berg].alive, "{events:?}");
    let ruler = state.factions[&berg].ruler.clone().unwrap();
    assert_ne!(ruler, old);
    assert_eq!(state.characters[&ruler].house, house);
    assert_eq!(state.owned_provinces(&berg), provinces);
    assert!(
        events.iter().any(|e| e.contains("branche cadette")),
        "{events:?}"
    );
}

/// LR-05: at 0 the line escheats to the liege as before (FE F3).
#[test]
fn without_cadet_branch_the_fief_escheats() {
    let mut data = game_data().clone();
    data.feudal_rules.collateral_line_percent = 0;
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_papacy").unwrap(), 1).expect("setup");
    let events = kill_last_of_line(&mut state, &data, "fac_berg");
    assert!(
        !state.factions[&FactionId::new("fac_berg").unwrap()].alive,
        "{events:?}"
    );
}
