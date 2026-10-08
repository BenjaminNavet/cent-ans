//! MF1 (campaign map filters) integration tests: `map_lens` values on the
//! 1337 start.

use data_model::GameData;
use sim_campaign::map_lens::{map_lens, ClaimStance};
use sim_campaign::{CampaignState, Season};

use data_model::test_support::{fac, game_data, prov};

fn start(data: &GameData, player: &str) -> CampaignState {
    CampaignState::new_1337(data, fac(player), 7).expect("1337 start")
}

#[test]
fn every_province_has_values_in_range() {
    let data = game_data();
    let state = start(data, "fac_france");
    let lens = map_lens(&state, data, &fac("fac_france"));
    assert_eq!(lens.len(), state.provinces.len());
    for (id, values) in &lens {
        assert!(
            (0.0..=100.0).contains(&values.unrest),
            "{id}: {}",
            values.unrest
        );
        assert!(values.income >= 0.0, "{id}");
    }
    let paris = &lens[&prov("prov_ile_de_france")];
    assert!(paris.income > 0.0 && paris.population > 0);
}

#[test]
fn vassal_provinces_carry_their_lord_and_loyalty() {
    let data = game_data();
    let state = start(data, "fac_france");
    let flanders = fac("fac_flanders");
    let loyalty = state.faction_state(&flanders).unwrap().loyalty;
    let lens = map_lens(&state, data, &fac("fac_france"));
    let flemish: Vec<_> = lens
        .iter()
        .filter(|(id, _)| state.province_owner(id) == Some(&flanders))
        .map(|(_, values)| values)
        .collect();
    assert!(!flemish.is_empty());
    for values in flemish {
        assert_eq!(values.vassal_loyalty, Some(loyalty));
        assert_eq!(values.suzerain, Some(fac("fac_france")));
    }
    // The crown itself is nobody's vassal.
    assert_eq!(lens[&prov("prov_ile_de_france")].vassal_loyalty, None);
}

#[test]
fn claims_are_seen_from_both_sides() {
    let data = game_data();
    let state = start(data, "fac_england");
    // Edward III claims the French crown, hence every French province.
    let english = map_lens(&state, data, &fac("fac_england"));
    assert!(matches!(
        english[&prov("prov_ile_de_france")].claim,
        ClaimStance::Ours | ClaimStance::Contested
    ));
    let french = map_lens(&state, data, &fac("fac_france"));
    assert_eq!(
        french[&prov("prov_ile_de_france")].claim,
        ClaimStance::AgainstUs
    );
}

#[test]
fn supply_recovers_at_home_and_drains_abroad_worse_in_winter() {
    let data = game_data();
    let mut state = start(data, "fac_france");
    let france = fac("fac_france");
    let lens = map_lens(&state, data, &france);
    assert!(lens[&prov("prov_ile_de_france")].supply_change > 0);
    let abroad = lens[&prov("prov_kent")].supply_change;
    assert!(abroad < 0, "Kent: {abroad}");
    state.season = Season::Winter;
    let winter = map_lens(&state, data, &france)[&prov("prov_kent")].supply_change;
    assert!(winter < abroad, "winter {winter} vs {abroad}");
}
