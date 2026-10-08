//! LR-13: the pre-battle forecast agrees with the auto-resolver.
//!
//! Q8: the screen read « Défaite presque certaine, 0 % » for battles the
//! player then won hands down: the forecast ran the pre-N1 formula, not the
//! phased resolver. These tests compare the forecast with the frequency of
//! wins of the auto-resolver on seeds the forecast never uses.

use data_model::{FactionId, GameData, ProvinceId, Terrain, UnitCategory, UnitTypeId};
use sim_campaign::battle_forecast::forecast_sides;
use sim_campaign::rng::CampaignRng;
use sim_campaign::state::Season;
use sim_campaign::test_support::first_army;
use sim_campaign::{
    resolve_with_crossings, BattleContext, BattleUnit, CampaignState, FieldConditions, Side,
    UnitProfile, Winner,
};

/// Largest gap allowed between the forecast and the observed frequency
/// (sampling noise of 200 + 400 runs is ~4 points at 50 %).
const MARGIN: f64 = 0.12;

use data_model::test_support::{fac, game_data};

fn army(data: &GameData, spec: &[(&str, usize)]) -> (Side, Vec<UnitProfile>) {
    let mut side = Side::default();
    let mut profiles = Vec::new();
    for (id, n) in spec {
        let t = &data.unit_types[&UnitTypeId::new(*id).unwrap()];
        for _ in 0..*n {
            side.units.push(BattleUnit {
                strength: t.soldiers,
                max_strength: t.soldiers,
                experience: 0,
                morale: t.stats.morale,
                melee: t.stats.melee,
                ranged: t.stats.ranged,
                armor: t.stats.armor,
                is_ranged: matches!(t.category, UnitCategory::Ranged | UnitCategory::Siege),
            });
            profiles.push(UnitProfile::of(t));
        }
    }
    (side, profiles)
}

#[test]
fn forecast_matches_the_auto_resolver_on_a_sample_of_compositions() {
    const MAA: &str = "unit_men_at_arms_foot";
    const LB: &str = "unit_longbowmen";
    const KN: &str = "unit_knights";
    const MIL: &str = "unit_urban_militia";
    const XB: &str = "unit_crossbowmen";
    const SP: &str = "unit_welsh_spearmen";
    const HB: &str = "unit_hobelars";
    type Spec<'a> = &'a [(&'a str, usize)];
    let scenarios: [(Spec, Spec, Terrain, Season); 8] = [
        (
            &[(MAA, 3), (LB, 3), (KN, 2)],
            &[(MAA, 3), (LB, 3), (KN, 2)],
            Terrain::Plains,
            Season::Summer,
        ),
        (
            &[(KN, 6), (XB, 3), (MIL, 3)],
            &[(MAA, 4), (LB, 6)],
            Terrain::Plains,
            Season::Autumn,
        ),
        // The pre-N1 formula said 0 % here; the resolver wins every time.
        (
            &[(KN, 3), (MAA, 3)],
            &[(MIL, 6), (SP, 4), (XB, 2)],
            Terrain::Plains,
            Season::Summer,
        ),
        (
            &[(KN, 4), (MAA, 4)],
            &[(LB, 5), (MAA, 3)],
            Terrain::Hills,
            Season::Summer,
        ),
        (
            &[(HB, 6), (LB, 2)],
            &[(MIL, 6), (XB, 2)],
            Terrain::Forest,
            Season::Winter,
        ),
        (
            &[(MAA, 3), (LB, 3), (KN, 2)],
            &[(MAA, 3), (XB, 4), (KN, 2)],
            Terrain::Plains,
            Season::Spring,
        ),
        (
            &[(MAA, 4), (LB, 2), (KN, 2)],
            &[(MAA, 3), (LB, 3), (KN, 2)],
            Terrain::Plains,
            Season::Summer,
        ),
        (
            &[(MIL, 8), (KN, 2)],
            &[(MAA, 3), (LB, 3)],
            Terrain::Plains,
            Season::Summer,
        ),
    ];
    let data = game_data();
    for (index, (a, d, terrain, season)) in scenarios.iter().enumerate() {
        let attacker = army(data, a);
        let defender = army(data, d);
        let conditions = FieldConditions {
            terrain: Some(*terrain),
            season: Some(*season),
            weather: None,
        };
        let forecast = forecast_sides(
            (&attacker.0, &attacker.1),
            (&defender.0, &defender.1),
            &BattleContext::default(),
            &conditions,
            &data.auto_resolve,
            &data.river_crossing_rules,
            0,
        );
        let runs = 400;
        let wins = (0..runs)
            .filter(|seed| {
                let mut rng = CampaignRng::from_seed(5_000_011 + seed);
                resolve_with_crossings(
                    &attacker.0,
                    &attacker.1,
                    &defender.0,
                    &defender.1,
                    &BattleContext::default(),
                    &conditions,
                    &data.auto_resolve,
                    &data.river_crossing_rules,
                    &mut rng,
                )
                .winner
                    == Winner::Attacker
            })
            .count();
        let observed = wins as f64 / runs as f64;
        assert!(
            (forecast.win_chance - observed).abs() <= MARGIN,
            "scenario {index}: forecast {:.2}, auto-resolver {observed:.2}",
            forecast.win_chance
        );
        assert!(forecast.attacker_power > 0.0 && forecast.defender_power > 0.0);
    }
}

/// Share of `attacker`'s wins against `defender` in pending battle 0, over
/// `runs` reseeded copies of `state` (a win raises its war score).
fn observed(
    state: &CampaignState,
    data: &GameData,
    attacker: &FactionId,
    defender: &FactionId,
    runs: u64,
) -> f64 {
    let score = |s: &CampaignState| {
        s.factions[attacker]
            .war_scores
            .get(defender)
            .copied()
            .unwrap_or(0)
    };
    let wins = (0..runs)
        .filter(|seed| {
            let mut copy = state.clone();
            copy.rng = CampaignRng::from_seed(9_000_001 + seed);
            copy.auto_resolve_pending(data, 0).expect("resolves");
            score(&copy) > score(state)
        })
        .count();
    wins as f64 / runs as f64
}

fn at_war(state: &mut CampaignState) {
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
}

#[test]
fn campaign_forecast_matches_the_campaign_auto_resolution() {
    let data = game_data();
    for (attacker_faction, defender_faction) in
        [("fac_france", "fac_england"), ("fac_england", "fac_france")]
    {
        let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).expect("1337 start");
        state.chronicle.disabled = true;
        at_war(&mut state);
        let attacker = first_army(&state, attacker_faction);
        let defender = first_army(&state, defender_faction);
        state.debug_stage_battle(&attacker, &defender).unwrap();
        let forecast = state.battle_forecast(data, 0).unwrap().attacker_win_chance;
        let seen = observed(
            &state,
            data,
            &fac(attacker_faction),
            &fac(defender_faction),
            120,
        );
        assert!(
            (forecast - seen).abs() <= MARGIN + 0.03,
            "{attacker_faction} attacks: forecast {forecast:.2}, auto-resolution {seen:.2}"
        );
    }
}

#[test]
fn assault_forecast_matches_the_auto_assault() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 9).expect("1337 start");
    state.chronicle.disabled = true;
    let lead = first_army(&state, "fac_france");
    let guyenne = ProvinceId::new("prov_guyenne").unwrap();
    let index = state.debug_stage_siege(data, &lead, &guyenne).unwrap();
    assert_eq!(index, 0, "the assault is the only pending battle");
    let forecast = state.battle_forecast(data, 0).unwrap().attacker_win_chance;
    assert_eq!(
        Some(forecast),
        state.assault_win_chance(data, &lead),
        "the army bar shows the screen's number"
    );
    let place = state.armies[&lead].settlement().cloned().unwrap();
    let garrison = state.settlements[&place].controller.clone();
    let seen = observed(&state, data, &fac("fac_france"), &garrison, 120);
    assert!(
        (forecast - seen).abs() <= MARGIN + 0.03,
        "assault: forecast {forecast:.2}, auto-assault {seen:.2}"
    );
}
