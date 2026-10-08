//! R4: reverse slope against the longbow (direct and indirect shooting),
//! one score for a defensive position (cover x crest), an attacker above an
//! army of shooters that waits on its heights for a while.
//!
//! Surveys (ignored): `cargo test --release -p sim-battle --test r4 --
//! --ignored --nocapture`.

use crate::common;

use common::*;
use data_model::GameData;
use sim_battle::missile_arc::{arc_clears, MissileArcRules};
use sim_battle::position::{score_position, Front};
use sim_battle::relief_ai::ReliefMap;
use sim_battle::{BattleSim, Obstacle, ObstacleKind, SideId, UnitState, DEFENDER_LINE_Z};

const PI: f64 = std::f64::consts::PI;

/// Clears woods, mud, pools, river and hedges, then shapes the ground with
/// `height(z)`.
fn shape(sim: &mut BattleSim, height: impl Fn(f64) -> f64) {
    let field = sim.field_mut();
    field.forests.clear();
    field.forest_parts.clear();
    field.mud.clear();
    field.mud_parts.clear();
    field.pools.clear();
    field.obstacles.clear();
    field.river = None;
    let (nx, res) = (field.nx, field.resolution);
    for (k, h) in field.heights.iter_mut().enumerate() {
        *h = height((k / nx) as f64 * res);
    }
}

/// An east-west ridge, 20 m high, crest at `crest_z`.
fn ridge(crest_z: f64) -> impl Fn(f64) -> f64 {
    move |z| 20.0 * (-((z - crest_z) / 45.0).powi(2)).exp()
}

// ----- shooting over a crest ------------------------------------------------

const CREST: f64 = 400.0;

/// Two longbows (ids 0, 1) at z 240 facing a regiment of men-at-arms (id 3)
/// 200 m away; a knight regiment (id 2) watches from the crest's east end
/// when `spotter`, from far behind the bows otherwise. Both AIs off.
fn longbows_at_a_line(data: &GameData, ridged: bool, spotter: bool) -> BattleSim {
    let battle = setup(
        units(
            data,
            &["unit_longbowmen", "unit_longbowmen", "unit_knights"],
        ),
        units(data, &["unit_men_at_arms_foot"]),
        None,
    );
    let mut sim = BattleSim::new(battle, 3).unwrap();
    lab(&mut sim);
    if ridged {
        shape(&mut sim, ridge(CREST));
    } else {
        shape(&mut sim, |_| 0.0);
    }
    place(&mut sim, 0, 560.0, 240.0, 0.0);
    place(&mut sim, 1, 640.0, 240.0, 0.0);
    if spotter {
        place(&mut sim, 2, 800.0, CREST, PI);
    } else {
        place(&mut sim, 2, 600.0, 40.0, 0.0);
    }
    // The line stands on the reverse slope, 40 m behind the crest.
    place(&mut sim, 3, 600.0, CREST + 40.0, PI);
    sim
}

/// Men of the target line killed by the arrows in `seconds`.
fn arrow_losses(sim: &mut BattleSim, seconds: f64) -> f64 {
    let before = sim.units()[3].hp;
    run(sim, seconds);
    before - sim.units()[3].hp
}

#[test]
fn the_reverse_slope_shelters_a_line_from_longbows() {
    let data = data();
    let mut open = longbows_at_a_line(data, false, true);
    let mut sheltered = longbows_at_a_line(data, true, true);
    {
        let field = sheltered.field();
        let line = &sheltered.units()[3];
        assert!(!ReliefMap::sees(field, (600.0, 240.0), (line.x, line.z)));
        assert!(ReliefMap::sees(field, (800.0, CREST), (line.x, line.z)));
    }
    let in_the_open = arrow_losses(&mut open, 90.0);
    let behind_the_crest = arrow_losses(&mut sheltered, 90.0);
    assert!(in_the_open > 10.0, "open ground: {in_the_open:.1} killed");
    assert!(
        behind_the_crest < 0.45 * in_the_open,
        "reverse slope {behind_the_crest:.1} vs open {in_the_open:.1}"
    );
    // Still not useless: the lobbed volleys directed from the crest kill.
    assert!(behind_the_crest > 0.0);
}

#[test]
fn a_lobbed_volley_needs_a_friend_who_sees_the_target() {
    let data = data();
    let mut spotted = longbows_at_a_line(data, true, true);
    let killed = arrow_losses(&mut spotted, 60.0);
    let shots = spotted.take_shots();
    assert!(killed > 0.0);
    assert!(
        shots.iter().any(|s| s.target == Some(3) && s.indirect),
        "indirect volleys at the line"
    );
    let mut blind = longbows_at_a_line(data, true, false);
    assert_eq!(arrow_losses(&mut blind, 60.0), 0.0);
    assert!(blind.take_shots().iter().all(|s| s.target != Some(3)));
}

#[test]
fn a_target_seen_a_moment_ago_draws_a_few_scattered_volleys() {
    let data = data();
    let mut sim = longbows_at_a_line(data, true, false);
    // The line stands on the crest in sight, then steps back behind it.
    place(&mut sim, 3, 600.0, CREST - 10.0, PI);
    run(&mut sim, 8.0);
    assert!(sim.units()[3].seen_at > 0.0);
    place(&mut sim, 3, 600.0, CREST + 40.0, PI);
    sim.take_shots();
    run(&mut sim, 30.0);
    let shots = sim.take_shots();
    let lobbed: Vec<f64> = shots
        .iter()
        .filter(|s| s.target == Some(3))
        .map(|s| s.time)
        .collect();
    let memory = MissileArcRules::bundled().memory_s;
    assert!(!lobbed.is_empty(), "a few volleys at the remembered line");
    assert!(
        lobbed.iter().all(|&t| t <= 8.0 + memory + 0.2),
        "no volley once forgotten: {lobbed:?}"
    );
}

#[test]
fn a_crest_close_in_front_of_the_target_leaves_dead_ground() {
    let data = data();
    let battle = setup(
        units(data, &["unit_longbowmen"]),
        units(data, &["unit_men_at_arms_foot"]),
        None,
    );
    let mut sim = BattleSim::new(battle, 3).unwrap();
    // A steep bank 12 m high at z 400 (a scarp), flat beyond it.
    shape(&mut sim, |z| 12.0 / (1.0 + ((z - 400.0) / 4.0).exp()));
    let rules = MissileArcRules::bundled();
    let (angle, margin) = (rules.max_launch_angle_deg, rules.clearance_m);
    let field = sim.field();
    // From the top of the bank looking down: masked right at its foot...
    let hidden = (600.0, 408.0);
    assert!(!ReliefMap::sees(field, (600.0, 200.0), hidden));
    // ... and a shooter 190 m away cannot drop an arrow there over the
    // bank's lip only when the lip is close to the shooter; shooting down
    // the bank the target 8 m below the lip is reachable from far off.
    assert!(arc_clears(field, (600.0, 220.0), hidden, angle, margin));
    // A shooter at the foot of a scarp looking up cannot lob over a lip
    // right in front of it at a target on top.
    let mut sim2 = BattleSim::new(
        setup(
            units(data, &["unit_longbowmen"]),
            units(data, &["unit_men_at_arms_foot"]),
            None,
        ),
        3,
    )
    .unwrap();
    // A 25 m wall-like rise 10 m in front of the shooter at z 300.
    shape(&mut sim2, |z| 25.0 / (1.0 + ((310.0 - z) / 2.0).exp()));
    let field2 = sim2.field();
    assert!(!arc_clears(
        field2,
        (600.0, 300.0),
        (600.0, 330.0),
        angle,
        margin
    ));
    // Far back from the same rise the arrows clear it.
    assert!(arc_clears(
        field2,
        (600.0, 250.0),
        (600.0, 420.0),
        angle,
        margin
    ));
}

// ----- position score ---------------------------------------------------------

/// A ridge crest at z 470 in front of the defender's deployment line (z
/// 550), hedges optional.
fn crest_field(data: &GameData, crest_z: f64, hedges: &[Obstacle]) -> BattleSim {
    let french = [
        "unit_knights",
        "unit_knights",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_crossbowmen",
        "unit_crossbowmen",
    ];
    let english = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_longbowmen",
    ];
    let mut battle = setup(units(data, &french), units(data, &english), None);
    battle.village = Some(false);
    // The crest and its hedges are laid for the standard 300 m line gap (ADR 0184
    // widened the field battles' gap).
    let mut sim = BattleSim::new_scaled(battle, 5, sim_battle::BattleScale::default()).unwrap();
    sim.set_weather(sim_battle::Weather::Clear);
    shape(&mut sim, ridge(crest_z));
    sim.field_mut().obstacles.extend_from_slice(hedges);
    sim
}

fn hedge_at(z: f64, x0: f64, x1: f64) -> Obstacle {
    Obstacle {
        a: (x0, z),
        b: (x1, z),
        kind: ObstacleKind::Hedge,
    }
}

#[test]
fn the_score_prefers_a_hedge_on_a_crest() {
    let data = data();
    // Hedge on the crest (x 470-730) and in the hollow behind (x 470-730).
    let sim = crest_field(
        data,
        470.0,
        &[hedge_at(465.0, 470.0, 730.0), hedge_at(600.0, 470.0, 730.0)],
    );
    let (field, map) = (sim.field(), sim.relief_map());
    let base = field.height(600.0, DEFENDER_LINE_Z);
    let score = |x: f64, z: f64| {
        let front = Front {
            center: (x, z),
            forward: -1.0,
            width: 120.0,
        };
        score_position(field, map, front, base, true)
    };
    let hedge_on_crest = score(600.0, 472.0);
    let bare_crest = score(950.0, 472.0);
    let hedge_in_hollow = score(600.0, 607.0);
    assert!(hedge_on_crest.cover > 10.0 && bare_crest.cover == 0.0);
    assert!(hedge_in_hollow.cover > 10.0);
    assert!(
        hedge_on_crest.total() > bare_crest.total() + 5.0,
        "{hedge_on_crest:?} vs {bare_crest:?}"
    );
    assert!(
        hedge_on_crest.total() > hedge_in_hollow.total() + 5.0,
        "{hedge_on_crest:?} vs {hedge_in_hollow:?}"
    );
    assert!(bare_crest.glacis > 0.0 && hedge_in_hollow.glacis == 0.0);
}

#[test]
fn a_wing_on_a_wood_or_a_river_scores_its_flank() {
    let data = data();
    let mut sim = crest_field(data, 470.0, &[]);
    let front = Front {
        center: (600.0, 470.0),
        forward: -1.0,
        width: 120.0,
    };
    let bare = sim_battle::position::flank_points(sim.field(), front);
    sim.field_mut().forests.push(sim_battle::Zone {
        x: 700.0,
        z: 470.0,
        radius: 40.0,
    });
    let wooded = sim_battle::position::flank_points(sim.field(), front);
    assert_eq!(bare, 0.0);
    assert_eq!(wooded, sim_battle::position::FLANK_POINTS);
}

/// English archers of a defensive side take the hedge on the crest ahead
/// rather than the nearer hedge in the hollow B6 alone would pick.
#[test]
fn english_archers_take_the_hedge_on_the_crest() {
    let data = data();
    // The crest 45 m ahead of the deployment line (reachable before the
    // French foot), the hollow 30 m behind it.
    let crest_hedge = hedge_at(500.0, 470.0, 730.0);
    let hollow_hedge = hedge_at(580.0, 480.0, 720.0);
    let mut sim = crest_field(data, 505.0, &[crest_hedge, hollow_hedge]);
    let b6 = sim_battle::ai::defensive_cover(sim.field(), SideId::Defender).unwrap();
    assert!(b6.center.1 > 570.0, "B6 alone takes the hollow hedge");
    sim.set_ai(SideId::Attacker, false);
    // Uphill to the crest: a slow march (L13b, ADR 0184: approach pace x0.45).
    run(&mut sim, 400.0);
    for u in sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender && u.can_shoot())
    {
        assert!(
            u.z > 500.0 && u.z < 500.0 + sim_battle::site::HEDGE_COVER_REACH,
            "{} at ({:.0}, {:.0}), not behind the crest hedge",
            u.name,
            u.x,
            u.z
        );
    }
}

/// The attacker's line strikes the enemy regiment in the open rather than
/// the nearer one behind a hedge.
#[test]
fn the_attacker_strikes_the_weak_point() {
    let data = data();
    let battle = setup(
        units(data, &["unit_men_at_arms_foot"]),
        units(data, &["unit_urban_militia", "unit_urban_militia"]),
        None,
    );
    let mut sim = BattleSim::new(battle, 3).unwrap();
    lab(&mut sim);
    shape(&mut sim, |_| 0.0);
    sim.field_mut()
        .obstacles
        .push(hedge_at(385.0, 520.0, 610.0));
    place(&mut sim, 0, 600.0, 300.0, 0.0);
    place(&mut sim, 1, 570.0, 395.0, PI);
    place(&mut sim, 2, 640.0, 395.0, PI);
    // Weakened defenders: the attacker is clearly stronger, not waiting.
    for id in [1, 2] {
        sim.units_mut()[id].hp *= 0.4;
    }
    sim.set_ai(SideId::Attacker, true);
    run(&mut sim, 4.0);
    assert_eq!(
        sim.units()[0].target,
        Some(2),
        "the men-at-arms go for the militia in the open"
    );
}

// ----- attacker above an army of shooters -----------------------------------

/// Attacker deployed on a plateau (z < 300) above a defender made mostly of
/// longbows, which stays passive.
fn attacker_above_bows(data: &GameData) -> BattleSim {
    let battle = setup(
        units(
            data,
            &[
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_urban_militia",
                "unit_longbowmen",
            ],
        ),
        units(
            data,
            &[
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_men_at_arms_foot",
            ],
        ),
        None,
    );
    let mut sim = BattleSim::new(battle, 11).unwrap();
    sim.set_weather(sim_battle::Weather::Clear);
    shape(&mut sim, |z| 18.0 / (1.0 + ((z - 330.0) / 20.0).exp()));
    sim.set_ai(SideId::Defender, false);
    sim
}

#[test]
fn an_attacker_above_shooters_waits_then_attacks() {
    let data = data();
    let mut sim = attacker_above_bows(data);
    let foot_z = |sim: &BattleSim| {
        sim.units()
            .iter()
            .filter(|u| u.side == SideId::Attacker && !u.can_shoot())
            .map(|u| u.z)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    run(&mut sim, 90.0);
    assert!(
        foot_z(&sim) < 320.0,
        "the attacker waits on its plateau, foot at z {:.0}",
        foot_z(&sim)
    );
    // Then it attacks: no frozen battle.
    let mut engaged = false;
    // L13b (ADR 0184): approach pace x0.45 and patience clocks x1.9.
    while sim.elapsed() < 800.0 && !sim.is_finished() && !engaged {
        sim.step();
        engaged = sim.units().iter().any(|u| u.state == UnitState::Melee);
    }
    assert!(
        engaged || sim.is_finished(),
        "contact before 13 minutes (foot at z {:.0})",
        foot_z(&sim)
    );
}

/// On a bare rounded crest, the defending archers stand on its military
/// crest (down the forward slope), from which they see their glacis, not on
/// the topographic top with dead ground below them.
#[test]
fn archers_leave_no_dead_ground_below_a_rounded_crest() {
    use sim_battle::position::{dead_ground, military_crest, DEAD_TOLERANCE};
    let data = data();
    let mut sim = crest_field(data, 505.0, &[]);
    {
        // Just behind the top (where the height search lands), the ridge
        // hides the foot of its slope; the military crest does not.
        let field = sim.field();
        let top = Front {
            center: (600.0, 525.0),
            forward: -1.0,
            width: 60.0,
        };
        assert!(dead_ground(field, top) > DEAD_TOLERANCE);
        let post = military_crest(field, top);
        assert!(post.1 < 525.0, "towards the forward slope: z {:.0}", post.1);
        assert!(
            dead_ground(
                field,
                Front {
                    center: post,
                    ..top
                }
            ) <= DEAD_TOLERANCE
        );
    }
    sim.set_ai(SideId::Attacker, false);
    // L13b (ADR 0184): approach pace x0.45.
    run(&mut sim, 400.0);
    let field = sim.field();
    for u in sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender && u.can_shoot())
    {
        let front = Front {
            center: (u.x, u.z),
            forward: -1.0,
            width: u.extent().0,
        };
        let dead = dead_ground(field, front);
        assert!(
            dead <= DEAD_TOLERANCE,
            "{} at ({:.0}, {:.0}) leaves {:.0} % of its glacis in dead ground",
            u.name,
            u.x,
            u.z,
            dead * 100.0
        );
    }
}
