//! Lot BV2: charge impacts (men knocked down, pikes and stakes stopping the
//! horses) and causes of death reported to the renderer.

mod common;

use common::*;
use sim_battle::impact::{self, KNOCKDOWN_TIME};
use sim_battle::{BattleSim, Command, ImpactEvent, ImpactKind, LossCause};

/// Lab battle between two lists of regiments (AIs off, clear weather).
fn arena(attacker: Vec<sim_battle::UnitSetup>, defender: Vec<sim_battle::UnitSetup>) -> BattleSim {
    let mut sim = BattleSim::new(setup(attacker, defender, None), 7).unwrap();
    lab(&mut sim);
    sim
}

/// Knights (unit 0) charge `target` (unit 1) head on over 150 m; returns the
/// simulation after `seconds` and every impact read on the way.
fn charge(
    target: &str,
    seconds: f64,
    prepare: impl Fn(&mut BattleSim),
) -> (BattleSim, Vec<ImpactEvent>) {
    let data = data();
    let mut sim = arena(vec![unit(data, "unit_knights")], vec![unit(data, target)]);
    place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
    place(&mut sim, 0, 600.0, 250.0, 0.0);
    sim.units_mut()[1].ammo = 0;
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

fn no_stakes(sim: &mut BattleSim) {
    sim.units_mut()[1].remove_ability(data_model::Ability::Stakes);
}

#[test]
fn a_charge_home_knocks_men_down_and_is_reported_once() {
    let (sim, impacts) = charge("unit_longbowmen", 30.0, no_stakes);
    assert_eq!(impacts.len(), 1, "{impacts:?}");
    let hit = &impacts[0];
    assert_eq!(hit.kind, ImpactKind::Shock);
    assert_eq!((hit.attacker, hit.defender), (0, 1));
    assert!(hit.knocked > 0, "{hit:?}");
    assert!(hit.depth > 0.0 && hit.depth <= 6.0, "{hit:?}");
    assert_eq!(hit.cohesion, impact::shock_morale(0));
    assert!(hit.mass > 0.5, "{hit:?}");
    // The charge runs along +z: heading ~ 0, contact point on the archers' face.
    assert!(hit.heading.abs() < 0.3, "{hit:?}");
    assert!(hit.point.1 < 400.0 && hit.point.1 > 330.0, "{hit:?}");
    // The archers die under the lances: cause charge, by the knights.
    let archers = &sim.units()[1];
    assert!(archers.soldiers() < archers.initial_soldiers);
    assert!(
        matches!(archers.loss_cause, LossCause::Charge | LossCause::Melee),
        "{:?}",
        archers.loss_cause
    );
    assert_eq!(archers.loss_by, Some(0));
    // Knocked-down men are back on their feet after the knock-down time.
    assert_eq!(archers.knocked_timer.max(0.0), 0.0);
    assert!(30.0 - hit.time > KNOCKDOWN_TIME);
}

#[test]
fn knocked_down_men_stop_fighting_for_a_while() {
    let data = data();
    let mut sim = arena(
        vec![unit(data, "unit_knights")],
        vec![unit(data, "unit_men_at_arms_foot")],
    );
    let before = sim.units()[1].fighting_soldiers();
    sim.units_mut()[1].knocked = 10.0;
    sim.units_mut()[1].knocked_timer = KNOCKDOWN_TIME;
    let during = sim.units()[1].fighting_soldiers();
    assert!((before - during - 10.0).abs() < 1e-9, "{before} {during}");
    sim.units_mut()[1].knocked_timer = 0.0;
    assert_eq!(sim.units()[1].fighting_soldiers(), before);
}

#[test]
fn impacts_are_deterministic() {
    let (a_sim, a) = charge("unit_urban_militia", 40.0, |_| {});
    let (b_sim, b) = charge("unit_urban_militia", 40.0, |_| {});
    assert_eq!(a, b);
    assert_eq!(a_sim.units()[1].hp, b_sim.units()[1].hp);
}

#[test]
fn heavier_and_flank_charges_knock_down_more() {
    let data = data();
    let knights = BattleSim::new(
        setup(
            vec![unit(data, "unit_knights")],
            vec![unit(data, "unit_urban_militia")],
            None,
        ),
        1,
    )
    .unwrap();
    let archers = knights.units()[0].clone();
    let militia = &knights.units()[1];
    let front = impact::knocked_count(&archers, militia, 0);
    let flank = impact::knocked_count(&archers, militia, 1);
    let rear = impact::knocked_count(&archers, militia, 2);
    assert!(
        front > 0 && flank > front && rear > flank,
        "{front} {flank} {rear}"
    );
    let mut on_foot = archers.clone();
    on_foot.mounted = false;
    assert!(impact::charge_mass(&on_foot) < impact::charge_mass(&archers));
    // Men-at-arms in plate stand better than townsmen.
    let men_at_arms = BattleSim::new(
        setup(
            vec![unit(data, "unit_knights")],
            vec![unit(data, "unit_men_at_arms_foot")],
            None,
        ),
        1,
    )
    .unwrap();
    let mass = impact::charge_mass(&archers);
    assert!(
        impact::knocked_share(mass, &men_at_arms.units()[1], 0)
            < impact::knocked_share(mass, militia, 0)
    );
}

#[test]
fn levelled_pikes_stop_the_charge_and_unhorse_riders() {
    let (sim, impacts) = charge("unit_flemish_pikemen", 40.0, |_| {});
    assert_eq!(impacts.len(), 1, "{impacts:?}");
    let hit = &impacts[0];
    assert_eq!(hit.kind, ImpactKind::Pikes);
    assert_eq!(hit.knocked, 0);
    assert!(hit.unhorsed > 0, "{hit:?}");
    assert!(has_event(&sim, "piques"));
    let knights = &sim.units()[0];
    assert!(knights.soldiers() < knights.initial_soldiers);
}

#[test]
fn stakes_impale_the_horses() {
    let (sim, impacts) = charge("unit_longbowmen", 30.0, |sim| {
        run(sim, 16.0);
        assert!(sim.units()[1].stakes_planted);
    });
    let hit = impacts
        .iter()
        .find(|i| i.kind == ImpactKind::Stakes)
        .expect("stakes impact");
    assert!(hit.unhorsed > 0);
    assert_eq!(hit.knocked, 0);
    // The riders who died on the stakes carry the stakes as their cause
    // unless the melee that follows has overwritten it.
    let knights = &sim.units()[0];
    assert!(matches!(
        knights.loss_cause,
        LossCause::Stakes | LossCause::Melee
    ));
}

#[test]
fn missiles_record_their_cause() {
    let data = data();
    let mut sim = arena(
        vec![unit(data, "unit_longbowmen")],
        vec![unit(data, "unit_urban_militia")],
    );
    place(&mut sim, 0, 600.0, 250.0, 0.0);
    place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
    run(&mut sim, 20.0);
    let militia = &sim.units()[1];
    assert!(militia.soldiers() < militia.initial_soldiers);
    assert_eq!(militia.loss_cause, LossCause::Arrow);
    assert_eq!(militia.loss_by, Some(0));
}
