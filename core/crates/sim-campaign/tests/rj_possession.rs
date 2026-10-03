//! Lot RJ-c (ADR 0175): possession versus occupation for a viewer — the
//! four statuses the UI explains, the settlement count and the full-province
//! holder.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::possession::PossessionStatus;
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

#[test]
fn status_covers_the_five_cases() {
    let (me, them, third) = (fac("fac_france"), fac("fac_england"), fac("fac_burgundy"));
    assert_eq!(PossessionStatus::of(&me, &me, &me), PossessionStatus::Own);
    assert_eq!(
        PossessionStatus::of(&me, &me, &them),
        PossessionStatus::OwnOccupied
    );
    assert_eq!(
        PossessionStatus::of(&me, &them, &me),
        PossessionStatus::OccupiedByViewer
    );
    assert_eq!(
        PossessionStatus::of(&me, &them, &them),
        PossessionStatus::Foreign
    );
    assert_eq!(
        PossessionStatus::of(&me, &them, &third),
        PossessionStatus::ForeignOccupied
    );
    assert!(!PossessionStatus::Own.is_occupied());
    assert!(!PossessionStatus::Foreign.is_occupied());
    assert!(PossessionStatus::OwnOccupied.is_occupied());
    assert!(PossessionStatus::OccupiedByViewer.is_occupied());
    assert!(PossessionStatus::ForeignOccupied.is_occupied());
    assert_eq!(
        PossessionStatus::OccupiedByViewer.key(),
        "occupied_by_viewer"
    );
}

#[test]
fn province_possession_follows_the_city_and_counts_places() {
    let data = data();
    let player = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, player.clone(), 7).expect("1337 start");
    let province = ProvinceId::new("prov_ile_de_france").unwrap();
    let start = state
        .province_possession(&player, &province)
        .expect("known province");
    assert_eq!(start.status, PossessionStatus::Own);
    assert!(start.settlements_total >= 2, "several places");
    assert_eq!(start.held_by_viewer, start.settlements_total);
    assert_eq!(start.whole_province_holder.as_ref(), Some(&player));

    // An English force takes a non-city place: the province stays French,
    // the full-province bonus is lost.
    let enemy = fac("fac_england");
    let other = state.provinces[&province]
        .settlements
        .iter()
        .find(|id| **id != start.city)
        .cloned()
        .expect("a second place");
    state.settlements.get_mut(&other).unwrap().controller = enemy.clone();
    let after = state.province_possession(&player, &province).unwrap();
    assert_eq!(after.status, PossessionStatus::Own);
    assert_eq!(after.held_by_viewer, start.settlements_total - 1);
    assert_eq!(after.whole_province_holder, None);
    assert_eq!(
        state.settlement_possession(&player, &other),
        Some(PossessionStatus::OwnOccupied)
    );
    assert_eq!(
        state.settlement_possession(&enemy, &other),
        Some(PossessionStatus::OccupiedByViewer)
    );

    // Taking the city gives the province's control, not its possession.
    state.settlements.get_mut(&start.city).unwrap().controller = enemy.clone();
    let taken = state.province_possession(&player, &province).unwrap();
    assert_eq!(taken.status, PossessionStatus::OwnOccupied);
    assert_eq!(taken.owner, player);
    assert_eq!(taken.controller, enemy);
    let seen_by_enemy = state.province_possession(&enemy, &province).unwrap();
    assert_eq!(seen_by_enemy.status, PossessionStatus::OccupiedByViewer);
    let seen_by_third = state
        .province_possession(&fac("fac_burgundy"), &province)
        .unwrap();
    assert_eq!(seen_by_third.status, PossessionStatus::ForeignOccupied);
}
