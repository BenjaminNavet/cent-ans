//! EP8 tests: time of day on the battlefield (ADR 0052). Dawn and dusk
//! shorten the shooters' effective range; the day moves on with the battle.

mod common;

use common::*;
use sim_battle::{BattleSim, SideId, TimeOfDayRules, Weather};

/// Longbowmen facing foot men-at-arms at `gap` metres, both AIs off.
fn range_duel(gap: f64) -> BattleSim {
    let data = data();
    let battle = setup(
        units(&data, &["unit_longbowmen"]),
        units(&data, &["unit_men_at_arms_foot"]),
        None,
    );
    let mut sim = BattleSim::new(battle, 5).unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 600.0, 300.0, 0.0);
    place(&mut sim, 1, 600.0, 300.0 + gap, std::f64::consts::PI);
    sim
}

fn hour_of(key: &str) -> f64 {
    TimeOfDayRules::bundled().start_hour_of(key).unwrap()
}

#[test]
fn default_start_is_midday_in_full_light() {
    let sim = range_duel(120.0);
    assert_eq!(sim.day_phase().key, "midday");
    assert_eq!(sim.visibility(), 1.0);
    assert_eq!(sim.range_factor(), 1.0);
}

#[test]
fn dusk_and_dawn_shorten_the_effective_range() {
    let mut sim = range_duel(120.0);
    let bow = sim.units()[0].clone();
    let noon = sim.effective_range(&bow, 600.0, 420.0);
    for key in ["dawn", "dusk"] {
        sim.set_start_hour(hour_of(key));
        let low_light = sim.effective_range(&bow, 600.0, 420.0);
        let visibility = TimeOfDayRules::bundled().phase(key).unwrap().visibility;
        assert!(low_light < noon, "{key}: {low_light} >= {noon}");
        assert!((low_light - noon * visibility).abs() < 1e-9);
    }
    sim.set_start_hour(hour_of("morning"));
    assert_eq!(sim.effective_range(&bow, 600.0, 420.0), noon);
}

#[test]
fn fog_and_dusk_add_up() {
    let mut sim = range_duel(120.0);
    sim.set_weather(Weather::Fog);
    sim.set_start_hour(hour_of("dusk"));
    let expected = Weather::Fog.range_factor() * sim.visibility();
    assert!((sim.range_factor() - expected).abs() < 1e-12);
    assert!(sim.range_factor() < Weather::Fog.range_factor());
}

#[test]
fn archers_out_of_reach_at_dusk_shoot_at_noon() {
    let probe = range_duel(120.0);
    let full = f64::from(probe.units()[0].stats.range);
    // Between the dusk range and the full range: in reach at noon only.
    let visibility = TimeOfDayRules::bundled().phase("dusk").unwrap().visibility;
    let gap = full * (1.0 + visibility) * 0.5;
    let mut noon = range_duel(gap);
    let mut dusk = range_duel(gap);
    dusk.set_start_hour(hour_of("dusk"));
    run(&mut noon, 20.0);
    run(&mut dusk, 20.0);
    let noon_shots = noon.take_shots().iter().filter(|s| s.shooter == 0).count();
    let dusk_shots = dusk.take_shots().iter().filter(|s| s.shooter == 0).count();
    assert!(noon_shots > 0, "the longbows shoot at {gap:.0} m at noon");
    assert_eq!(dusk_shots, 0, "no volley at {gap:.0} m at dusk");
    assert!(noon.strength(SideId::Defender) < dusk.strength(SideId::Defender));
}

#[test]
fn the_day_moves_on_and_dusk_is_announced() {
    let mut sim = range_duel(400.0);
    let rules = TimeOfDayRules::bundled();
    // A few battle minutes before dusk.
    let dusk = rules.phase("dusk").unwrap().from_hour;
    sim.set_start_hour(dusk - 0.05);
    assert_eq!(sim.day_phase().key, "afternoon");
    run(&mut sim, 30.0);
    assert_eq!(sim.day_phase().key, "dusk");
    assert!(sim.hour() > dusk);
    assert!(has_event(&sim, "crépuscule"));
    assert!(sim.visibility() < 1.0);
}

#[test]
fn the_starting_phase_is_not_announced() {
    let mut sim = range_duel(400.0);
    sim.set_start_hour(hour_of("dawn"));
    run(&mut sim, 5.0);
    assert!(!has_event(&sim, "jour se lève"));
}
