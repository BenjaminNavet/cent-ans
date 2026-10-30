//! Lot CAMP-GEN (ADR 0831): the offers of the eight playable houses at the
//! 1337 start are proportionate to their might.
use data_model::test_support::game_data;
use sim_campaign::mission_scale::{house_might, probe_offers};
use sim_campaign::test_support::start;

const HOUSES: [&str; 8] = [
    "fac_france",
    "fac_england",
    "fac_burgundy",
    "fac_navarre",
    "fac_brittany",
    "fac_flanders",
    "fac_scotland",
    "fac_castile",
];

/// Conquests need a target force within the margin of the men available; levies
/// must be payable from the treasury.
#[test]
fn offers_of_the_eight_houses_are_proportionate_at_1337() {
    let data = game_data();
    let margin = data.mission_rules.scale.max_target_force_ratio;
    let share = data.mission_rules.scale.levy_treasury_share;
    for house in HOUSES {
        let state = start(&data, house, 1);
        let might = house_might(&state, &state.player_faction);
        let before = probe_offers(&state, &data, false);
        let after = probe_offers(&state, &data, true);
        let too_big = |o: &sim_campaign::mission_scale::OfferProbe| {
            f64::from(o.required_men) > f64::from(might.men) * margin
        };
        let disproportionate_before = before.iter().filter(|o| too_big(o)).count();
        eprintln!(
            "{house}: men {} gold {} offers {} -> {} (conquests too big before: {disproportionate_before})",
            might.men,
            might.gold,
            before.len(),
            after.len()
        );
        for offer in &after {
            assert!(
                !too_big(offer),
                "{house}: {} asks {} men against {} (margin {margin})",
                offer.template,
                offer.required_men,
                might.men
            );
            assert!(
                f64::from(offer.levy_units * offer.levy_unit_cost)
                    <= might.gold as f64 * share + 1.0,
                "{house}: levy {} unaffordable",
                offer.template
            );
        }
    }
}

/// No house, left without orders, loses its field army before season 4
/// (Navarre's 100 men used to vanish in the summer of 1337).
#[test]
fn no_house_loses_its_field_army_before_season_4() {
    let data = game_data();
    for house in HOUSES {
        let mut state = start(&data, house, 1);
        let mut series = Vec::new();
        for _ in 0..4 {
            series.push(house_might(&state, &state.player_faction).men);
            state.end_turn(&data);
        }
        eprintln!("{house}: men by season {series:?}");
        assert!(
            series.iter().all(|men| *men >= 100),
            "{house} drops under 100 men: {series:?}"
        );
    }
}
