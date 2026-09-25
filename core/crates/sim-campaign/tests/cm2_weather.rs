//! Lot CM2: campaign map weather — deterministic per seed, date and province,
//! regional fronts, chances of `data/rules/campaign_weather.json`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::{Climate, FactionId, GameData, ProvinceId};
use sim_campaign::state::Season;
use sim_campaign::weather::{compute_weather, weather_map, MapWeather};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
fn rules_file_is_loaded() {
    let data = data();
    assert!((data.campaign_weather.front_scale_deg - 3.5).abs() < 1e-9);
    assert_eq!(data.campaign_weather.climates.mountain.winter.snow, 50);
}

#[test]
fn weather_is_deterministic_and_changes_with_the_turn() {
    let data = data();
    let a = compute_weather(
        &data.campaign_weather,
        &data.provinces,
        42,
        3,
        Season::Autumn,
    );
    let b = compute_weather(
        &data.campaign_weather,
        &data.provinces,
        42,
        3,
        Season::Autumn,
    );
    assert_eq!(a, b);
    assert_eq!(a.len(), data.provinces.len());
    let next = compute_weather(
        &data.campaign_weather,
        &data.provinces,
        42,
        4,
        Season::Winter,
    );
    let other_seed = compute_weather(
        &data.campaign_weather,
        &data.provinces,
        43,
        3,
        Season::Autumn,
    );
    let changed = |m: &BTreeMap<ProvinceId, _>| {
        a.iter()
            .filter(|(id, w)| {
                m.get(*id).map(|x: &sim_campaign::ProvinceWeather| x.kind) != Some(w.kind)
            })
            .count()
    };
    assert!(
        changed(&next) > 10,
        "the sky changes from one turn to the next"
    );
    assert!(changed(&other_seed) > 10, "another game has another sky");
}

#[test]
fn chances_hold_over_many_turns() {
    let data = data();
    let mut counts: BTreeMap<MapWeather, u32> = BTreeMap::new();
    let mut total = 0u32;
    for turn in 0..60u32 {
        let map = compute_weather(
            &data.campaign_weather,
            &data.provinces,
            7,
            turn * 4 + 3,
            Season::Winter,
        );
        for (id, w) in &map {
            if data.provinces[id].climate == Some(Climate::Mountain) {
                *counts.entry(w.kind).or_default() += 1;
                total += 1;
            }
            // No summer thunder in winter tables with 0 %; no snow in a
            // Mediterranean summer (checked below).
            assert!((0.0..=1.0).contains(&w.intensity));
        }
    }
    let snow = f64::from(*counts.get(&MapWeather::Snow).unwrap_or(&0)) / f64::from(total);
    assert!(
        (0.3..0.7).contains(&snow),
        "mountain winter snow share {snow}"
    );
    assert_eq!(
        counts.get(&MapWeather::Storm),
        None,
        "no storm at 0 % in mountain winters"
    );

    for turn in 0..40u32 {
        let map = compute_weather(
            &data.campaign_weather,
            &data.provinces,
            9,
            turn * 4 + 1,
            Season::Summer,
        );
        for (id, w) in &map {
            if data.provinces[id].climate == Some(Climate::Mediterranean) {
                assert_ne!(w.kind, MapWeather::Snow, "{id:?}");
            }
        }
    }
}

#[test]
fn neighbours_share_the_sky_more_often_than_distant_provinces() {
    let data = data();
    let ids: Vec<&ProvinceId> = data.provinces.keys().collect();
    let (mut near_same, mut near_total, mut far_same, mut far_total) = (0u32, 0u32, 0u32, 0u32);
    for turn in 0..40u32 {
        let map = compute_weather(
            &data.campaign_weather,
            &data.provinces,
            5,
            turn,
            Season::Autumn,
        );
        let wet = |id: &ProvinceId| map[id].kind >= MapWeather::Rain;
        for (id, province) in &data.provinces {
            for n in &province.neighbors {
                if map.contains_key(n) {
                    near_total += 1;
                    near_same += u32::from(wet(id) == wet(n));
                }
            }
        }
        for (i, id) in ids.iter().enumerate() {
            let other = ids[(i * 37 + 11) % ids.len()];
            far_total += 1;
            far_same += u32::from(wet(id) == wet(other));
        }
    }
    let near = f64::from(near_same) / f64::from(near_total);
    let far = f64::from(far_same) / f64::from(far_total);
    assert!(
        near > far + 0.08,
        "fronts: neighbours {near:.2} vs random pairs {far:.2}"
    );
}

#[test]
fn weather_map_reads_the_state_date_and_seed() {
    let data = data();
    let france = FactionId::new("fac_france").unwrap();
    let state = CampaignState::new_1337(&data, france, 11).unwrap();
    let map = weather_map(&data, &state);
    let direct = compute_weather(
        &data.campaign_weather,
        &data.provinces,
        11,
        state.turn(),
        state.season(),
    );
    assert_eq!(map, direct);
    // The weather lives outside the state: reading it does not touch the RNG.
    let before = state.rng.clone();
    let _ = weather_map(&data, &state);
    assert_eq!(before, state.rng);
}
