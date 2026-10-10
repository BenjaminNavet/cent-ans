//! Lot TW bsim (ADR 0320): spear wall, morale by type, speech morale,
//! per-type reload and wavering regiments.

use crate::common;

use common::*;
use data_model::Ability;
use sim_battle::{BattleSim, Command, ImpactEvent};

fn arena(attacker: Vec<sim_battle::UnitSetup>, defender: Vec<sim_battle::UnitSetup>) -> BattleSim {
    let mut sim = BattleSim::new(setup(attacker, defender, None), 7).unwrap();
    lab(&mut sim);
    sim
}

/// Knights (unit 0) charge `target` (unit 1) over 150 m, the target facing
/// `facing` (PI: the knights come at its front). Returns the sim after
/// `seconds` and the impacts.
fn charge(
    target: &str,
    facing: f64,
    seconds: f64,
    prepare: impl Fn(&mut BattleSim),
) -> (BattleSim, Vec<ImpactEvent>) {
    let data = data();
    let mut sim = arena(vec![unit(data, "unit_knights")], vec![unit(data, target)]);
    place(&mut sim, 1, 600.0, 400.0, facing);
    place(&mut sim, 0, 600.0, 250.0, 0.0);
    prepare(&mut sim);
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: true,
        queue: false,
    })
    .unwrap();
    let mut impacts = Vec::new();
    let mut t = 0.0;
    while t < seconds {
        sim.tick(0.5);
        impacts.extend(sim.take_impacts());
        t += 0.5;
    }
    (sim, impacts)
}

fn without_wall(sim: &mut BattleSim) {
    sim.units_mut()[1].remove_ability(Ability::SpearWall);
}

#[test]
fn spear_wall_is_given_to_the_spear_units_in_data() {
    let data = data();
    for id in [
        "unit_welsh_spearmen",
        "unit_goedendag_militia",
        "unit_coutiliers",
        "unit_urban_militia",
    ] {
        assert!(unit_has(data, id, Ability::SpearWall), "{id}");
    }
    assert!(!unit_has(data, "unit_knights", Ability::SpearWall));
}

fn unit_has(data: &data_model::GameData, id: &str, ability: Ability) -> bool {
    data.unit_types[&data_model::UnitTypeId::new(id).unwrap()]
        .abilities
        .contains(&ability)
}

#[test]
fn a_frontal_charge_breaks_in_part_on_a_spear_wall() {
    let front = std::f64::consts::PI;
    let (walled, hit_w) = charge("unit_urban_militia", front, 30.0, |_| {});
    let (bare, hit_b) = charge("unit_urban_militia", front, 30.0, without_wall);
    assert_eq!((hit_w.len(), hit_b.len()), (1, 1));
    // The horse loses men and morale at the impact, the shock is dulled.
    let (kw, kb) = (&walled.units()[0], &bare.units()[0]);
    assert!(kw.hp < kb.hp, "{} vs {}", kw.hp, kb.hp);
    assert!(hit_w[0].knocked < hit_b[0].knocked, "{hit_w:?} {hit_b:?}");
    assert!(hit_w[0].cohesion < hit_b[0].cohesion);
    assert!(has_event(&walled, "lances"));
    assert!(!has_event(&bare, "lances"));
    // Weaker than levelled pikes: the charge is not stopped dead.
    assert!(hit_w[0].knocked > 0, "{hit_w:?}");
}

#[test]
fn a_flank_charge_ignores_the_spear_wall() {
    let flank = std::f64::consts::FRAC_PI_2;
    let (walled, hit_w) = charge("unit_urban_militia", flank, 30.0, |_| {});
    let (bare, hit_b) = charge("unit_urban_militia", flank, 30.0, without_wall);
    assert!(!has_event(&walled, "lances"));
    assert_eq!(hit_w[0].knocked, hit_b[0].knocked);
    assert_eq!(walled.units()[0].hp, bare.units()[0].hp);
}

/// Knights walk into the militia (no charge) and fight for `seconds`;
/// returns the knights' hp left.
fn melee_hp(facing: f64, seconds: f64, wall: bool) -> f64 {
    let data = data();
    let mut sim = arena(
        vec![unit(data, "unit_knights")],
        vec![unit(data, "unit_urban_militia")],
    );
    place(&mut sim, 1, 600.0, 400.0, facing);
    place(&mut sim, 0, 600.0, 385.0, 0.0);
    if !wall {
        without_wall(&mut sim);
    }
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: false,
        queue: false,
    })
    .unwrap();
    run(&mut sim, seconds);
    sim.units()[0].hp
}

#[test]
fn spear_wall_hits_horsemen_in_front_harder_but_not_on_the_flank() {
    let front = std::f64::consts::PI;
    assert!(melee_hp(front, 8.0, true) < melee_hp(front, 8.0, false));
    let flank = std::f64::consts::FRAC_PI_2;
    assert_eq!(melee_hp(flank, 8.0, true), melee_hp(flank, 8.0, false));
}

/// A longbow volley at point-blank range on `target`; returns
/// (morale lost) / (share of men lost).
fn morale_lost_per_casualty_share(target: &str) -> f64 {
    let data = data();
    let mut sim = arena(
        vec![unit(data, "unit_longbowmen")],
        vec![unit(data, target)],
    );
    sim.units_mut()[0].remove_ability(Ability::Stakes);
    place(&mut sim, 0, 600.0, 300.0, 0.0);
    place(&mut sim, 1, 600.0, 380.0, std::f64::consts::PI);
    sim.units_mut()[1].ammo = 0;
    let (cap, hp0) = (sim.units()[1].morale_cap, sim.units()[1].hp);
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: false,
        queue: false,
    })
    .unwrap();
    run(&mut sim, 7.0);
    let target = &sim.units()[1];
    let share = (hp0 - target.hp) / hp0;
    assert!(share > 0.0, "no casualties");
    (cap - target.morale) / share
}

#[test]
fn levies_lose_more_morale_per_casualty_than_men_at_arms() {
    let militia = morale_lost_per_casualty_share("unit_urban_militia");
    let men_at_arms = morale_lost_per_casualty_share("unit_men_at_arms_foot");
    assert!(
        militia > men_at_arms * 1.3,
        "militia {militia:.0} vs men_at_arms {men_at_arms:.0}"
    );
}

#[test]
fn speech_gives_the_weaker_side_more_morale_for_a_while() {
    let data = data();
    let mut sim = arena(
        vec![unit(data, "unit_knights")],
        vec![
            unit(data, "unit_knights"),
            unit(data, "unit_knights"),
            unit(data, "unit_knights"),
        ],
    );
    assert!(sim.begin_deployment());
    let before: Vec<f64> = sim.units().iter().map(|u| u.morale_cap).collect();
    sim.start_battle().unwrap();
    let gain = |sim: &BattleSim, i: usize| sim.units()[i].morale_cap - before[i];
    // Attacker (1 against 3) is the weak side: the full bonus; the strong
    // defender gets the smaller one.
    assert!(
        gain(&sim, 0) > gain(&sim, 1),
        "{} {}",
        gain(&sim, 0),
        gain(&sim, 1)
    );
    assert!(gain(&sim, 1) > 0.0);
    // It wears off.
    run(&mut sim, 100.0);
    assert_eq!(sim.units()[0].morale_cap, before[0]);
}

#[test]
fn reload_follows_the_type_then_the_default() {
    let data = data();
    let period = |id: &str| {
        let sim = arena(vec![unit(data, id)], vec![unit(data, "unit_knights")]);
        sim.units()[0].reload_period()
    };
    assert_eq!(period("unit_longbowmen"), 4.0);
    assert_eq!(period("unit_crossbowmen"), 9.0);
    assert!(period("unit_longbowmen") < period("unit_crossbowmen"));
    // Without the field: the former constants.
    assert_eq!(period("unit_francs_archers"), 6.0);
    assert_eq!(period("unit_trebuchet"), sim_battle::shot::ENGINE_RELOAD);
}

#[test]
fn faster_reload_means_more_arrows_in_the_same_time() {
    let spent = |reload: Option<u8>| {
        let data = data();
        let mut sim = arena(
            vec![unit(data, "unit_longbowmen")],
            vec![unit(data, "unit_urban_militia")],
        );
        sim.units_mut()[0].remove_ability(Ability::Stakes);
        sim.units_mut()[0].stats.reload_s = reload;
        place(&mut sim, 0, 600.0, 300.0, 0.0);
        place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
        sim.units_mut()[1].ammo = 0;
        let ammo = sim.units()[0].ammo;
        sim.issue_command(Command::Attack {
            units: vec![0],
            target: 1,
            run: false,
            queue: false,
        })
        .unwrap();
        run(&mut sim, 30.0);
        ammo - sim.units()[0].ammo
    };
    let (fast, slow) = (spent(Some(4)), spent(Some(8)));
    assert!(fast > slow, "{fast} vs {slow}");
    assert_eq!(spent(None), spent(Some(6)));
}

/// Knights at `morale` charge militia 150 m away; did they ever charge?
fn charged(morale: f64) -> bool {
    let (_, impacts) = charge("unit_urban_militia", std::f64::consts::PI, 30.0, |sim| {
        let u = &mut sim.units_mut()[0];
        u.morale = morale;
        u.morale_cap = morale;
        sim.units_mut()[1].remove_ability(Ability::SpearWall);
    });
    !impacts.is_empty()
}

#[test]
fn a_wavering_regiment_no_longer_charges() {
    assert!(charged(80.0));
    assert!(!charged(30.0));
}

/// Damage the militia takes in 3 s of melee from knights at `morale`.
fn melee_damage_at(morale: f64) -> f64 {
    let data = data();
    let mut sim = arena(
        vec![unit(data, "unit_knights")],
        vec![unit(data, "unit_urban_militia")],
    );
    place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
    place(&mut sim, 0, 600.0, 385.0, 0.0);
    without_wall(&mut sim);
    {
        let u = &mut sim.units_mut()[0];
        u.morale = morale;
        u.morale_cap = morale;
    }
    let hp0 = sim.units()[1].hp;
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: false,
        queue: false,
    })
    .unwrap();
    run(&mut sim, 3.0);
    hp0 - sim.units()[1].hp
}

#[test]
fn a_wavering_regiment_hits_softer() {
    let (steady, shaken) = (melee_damage_at(36.0), melee_damage_at(34.0));
    let ratio = shaken / steady;
    assert!((0.7..0.9).contains(&ratio), "ratio {ratio}");
}
