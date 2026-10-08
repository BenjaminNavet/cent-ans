//! LR-17: province and title sales with a price (`transfer_province`,
//! `transfer_title`), a throne taken by a living rival (`set_ruler`), and the
//! three Baltic events they made possible: the sale of Danish Estonia (1346),
//! Algirdas grand duke (1345) and the Ösel-Wiek election (1338). See
//! `docs/wip/lr-17.md`.

use data_model::{CharacterId, CharacterRef, EventEffect, EventId, FactionId, GameData, TitleId};
use sim_campaign::{CampaignState, EventContext, Order, Season};

use data_model::test_support::{fac, game_data};

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

fn evt(id: &str) -> EventId {
    EventId::new(id).unwrap()
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn apply(state: &mut CampaignState, data: &GameData, faction: &str, effect: EventEffect) {
    let ctx = EventContext {
        faction: Some(fac(faction)),
        province: None,
    };
    sim_campaign::chronicle::apply_effect(state, data, &effect, &ctx, &mut Vec::new());
}

/// England plays (none of the Baltic realms), chronicle on.
fn start(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_england"), seed).expect("1337 start")
}

fn treasury(state: &CampaignState, faction: &str) -> i64 {
    state.factions[&fac(faction)].treasury
}

#[test]
fn a_province_sale_pays_the_former_owner() {
    let data = game_data();
    let mut state = start(data, 1);
    state.chronicle.disabled = true;
    let dauphine = data_model::ProvinceId::new("prov_dauphine").unwrap();
    let (france, empire) = (
        treasury(&state, "fac_france"),
        treasury(&state, "fac_empire"),
    );
    let sale = EventEffect::TransferProvince {
        province: dauphine.clone(),
        faction: None,
        from: Some(fac("fac_empire")),
        price: 5000,
        payer: None,
    };
    apply(&mut state, data, "fac_france", sale.clone());
    assert_eq!(state.province_owner(&dauphine), Some(&fac("fac_france")));
    assert_eq!(treasury(&state, "fac_france"), france - 5000);
    assert_eq!(treasury(&state, "fac_empire"), empire + 5000);
    // Already sold: nothing changes hands, nothing is paid again.
    apply(&mut state, data, "fac_france", sale);
    assert_eq!(treasury(&state, "fac_france"), france - 5000);
}

#[test]
fn a_title_sale_takes_the_capital_and_absorbs_the_seller() {
    let data = game_data();
    let mut state = start(data, 2);
    state.chronicle.disabled = true;
    let reval = data_model::ProvinceId::new("prov_harrien_wierland").unwrap();
    let teutonic = treasury(&state, "fac_teutonic");
    // A plain province transfer refuses the capital of the seller.
    apply(
        &mut state,
        data,
        "fac_livonian_order",
        EventEffect::TransferProvince {
            province: reval.clone(),
            faction: None,
            from: None,
            price: 0,
            payer: None,
        },
    );
    assert_eq!(
        state.province_owner(&reval),
        Some(&fac("fac_danish_estonia"))
    );
    let sale = EventEffect::TransferTitle {
        title: TitleId::new("tit_estonia").unwrap(),
        faction: Some(fac("fac_livonian_order")),
        from: Some(fac("fac_danish_estonia")),
        price: 6000,
        payer: Some(fac("fac_teutonic")),
    };
    apply(&mut state, data, "fac_danish_estonia", sale.clone());
    assert_eq!(
        state.province_owner(&reval),
        Some(&fac("fac_livonian_order"))
    );
    assert!(!state.factions[&fac("fac_danish_estonia")].alive);
    // The seller is gone: the price leaves the map (the Danish crown).
    assert_eq!(treasury(&state, "fac_teutonic"), teutonic - 6000);
    // The Livonian Order stays the Teutonic Order's branch.
    assert_eq!(
        state.factions[&fac("fac_livonian_order")].suzerain,
        Some(fac("fac_teutonic"))
    );
    // `from` no longer holds it: a second sale is ignored.
    apply(&mut state, data, "fac_danish_estonia", sale);
    assert_eq!(treasury(&state, "fac_teutonic"), teutonic - 6000);
}

#[test]
fn set_ruler_deposes_without_killing() {
    let data = game_data();
    let mut state = start(data, 3);
    state.chronicle.disabled = true;
    let lithuania = fac("fac_lithuania");
    // A rival of another faction cannot take the throne.
    apply(
        &mut state,
        data,
        "fac_lithuania",
        EventEffect::SetRuler {
            character: chr("chr_jakob_ii_d_osel_wiek"),
            faction: None,
        },
    );
    assert_eq!(state.factions[&lithuania].ruler, Some(chr("chr_gediminas")));
    apply(
        &mut state,
        data,
        "fac_lithuania",
        EventEffect::SetRuler {
            character: chr("chr_algirdas"),
            faction: None,
        },
    );
    assert_eq!(state.factions[&lithuania].ruler, Some(chr("chr_algirdas")));
    assert!(state.characters[&chr("chr_gediminas")].alive);
    assert_ne!(state.factions[&lithuania].heir, Some(chr("chr_algirdas")));
}

#[test]
fn danish_estonia_is_sold_to_the_order_in_1346() {
    let data = game_data();
    let mut state = start(data, 4);
    state.year = 1346;
    state.season = Season::Summer;
    state.end_turn_with(data, idle);
    assert!(state
        .chronicle
        .fired_events
        .contains(&evt("evt_vente_de_l_estonie")));
    let reval = data_model::ProvinceId::new("prov_harrien_wierland").unwrap();
    assert_eq!(
        state.province_owner(&reval),
        Some(&fac("fac_livonian_order"))
    );
    assert!(!state.factions[&fac("fac_danish_estonia")].alive);
}

#[test]
fn algirdas_takes_vilnius_in_1345() {
    let data = game_data();
    let mut state = start(data, 5);
    // Gediminas died in 1341: Jaunutis rules.
    apply(
        &mut state,
        data,
        "fac_lithuania",
        EventEffect::KillCharacter {
            id: CharacterRef::Id(chr("chr_gediminas")),
            faction: None,
        },
    );
    let lithuania = fac("fac_lithuania");
    assert_eq!(state.factions[&lithuania].ruler, Some(chr("chr_jaunutis")));
    state.year = 1345;
    state.season = Season::Spring;
    state.end_turn_with(data, idle);
    assert!(state
        .chronicle
        .fired_events
        .contains(&evt("evt_algirdas_grand_duc")));
    assert_eq!(state.factions[&lithuania].ruler, Some(chr("chr_algirdas")));
    assert!(state.characters[&chr("chr_jaunutis")].alive);
}

#[test]
fn the_osel_wiek_chapter_elects_hermann_in_1338() {
    let data = game_data();
    let mut state = start(data, 6);
    let osel = fac("fac_osel_wiek");
    assert_eq!(
        state.factions[&osel].ruler,
        Some(chr("chr_jakob_ii_d_osel_wiek"))
    );
    state.year = 1338;
    state.season = Season::Spring;
    state.end_turn_with(data, idle);
    assert!(state
        .chronicle
        .fired_events
        .contains(&evt("evt_election_d_osel_wiek")));
    assert_eq!(
        state.factions[&osel].ruler,
        Some(chr("chr_hermann_ii_osenbrugge"))
    );
}
