//! CB-M2: path preview (`preview_path`) and contextual cursor
//! (`hover_context`). The preview is the chain of `plan_route`, whose first
//! waypoint is what the regiment heads for each tick after the real order:
//! the two must agree, in the open field, across a river and in a siege.

mod common;

use common::*;
use sim_battle::replay::state_digest;
use sim_battle::{BattleSim, Command, HoverKind, PreviewError, SideId, SiegeSetup, Water, DT};

type P = (f64, f64);

fn close(a: P, b: P, tolerance: f64) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) <= tolerance
}

/// A regiment's walk after an order: every new waypoint it heads for (the
/// real `route()` of the step), tick after tick, and where it stood.
struct Walk {
    heads: Vec<P>,
    trail: Vec<P>,
}

impl Walk {
    /// The waypoints in the order they were first headed for (a regiment
    /// hesitating at a crossing heads back and forth between two of them).
    fn first_visits(&self) -> Vec<P> {
        let mut out: Vec<P> = Vec::new();
        for &h in &self.heads {
            if !out.contains(&h) {
                out.push(h);
            }
        }
        out
    }
}

/// Orders regiment `id` to (x, z) and records its walk until it arrives or
/// `seconds` pass.
fn followed(sim: &mut BattleSim, id: u32, to: P, seconds: f64) -> Walk {
    sim.apply_command(
        Command::Move {
            units: vec![id],
            x: to.0,
            z: to.1,
            run: false,
            facing: None,
        },
        None,
    )
    .unwrap();
    let mut walk = Walk {
        heads: Vec::new(),
        trail: Vec::new(),
    };
    for _ in 0..(seconds / DT) as usize {
        let Some(h) = sim.heading_of(id) else {
            break;
        };
        if walk.heads.last() != Some(&h) {
            walk.heads.push(h);
        }
        sim.step();
        let u = &sim.units()[id as usize];
        walk.trail.push((u.x, u.z));
        if u.destination.is_none() {
            break;
        }
    }
    walk
}

/// Distance from `p` to the polyline `from` + `path`.
fn off_path(p: P, from: P, path: &[P]) -> f64 {
    let mut best = f64::MAX;
    let mut a = from;
    for &b in path {
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len2 = (dx * dx + dz * dz).max(1e-9);
        let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0);
        best = best.min((a.0 + dx * t - p.0).hypot(a.1 + dz * t - p.1));
        a = b;
    }
    best
}

/// A plains field with a river, both AIs off.
fn river_lab(seed: u64) -> BattleSim {
    let data = data();
    let mut battle = setup(
        units(&data, &["unit_knights", "unit_men_at_arms_foot"]),
        units(&data, &["unit_urban_militia"]),
        None,
    );
    battle.river = true;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    lab(&mut sim);
    sim
}

/// A seed whose river has a bridge and no ford near it.
fn bridged_seed() -> (u64, f64) {
    for seed in 0..60 {
        let sim = river_lab(seed);
        let f = sim.field();
        if let Some(b) = f.bridges.iter().find(|b| b.stream.is_none()) {
            let r = f.river.as_ref().unwrap();
            if r.fords.iter().all(|fd| (fd.x - b.x).abs() > 250.0) && (300.0..900.0).contains(&b.x)
            {
                return (seed, b.x);
            }
        }
    }
    panic!("no bridged field");
}

/// A seed whose river has a ford and no bridge near it.
fn forded_seed() -> (u64, f64) {
    for seed in 0..80 {
        let sim = river_lab(seed);
        let f = sim.field();
        let r = f.river.as_ref().unwrap();
        for fd in &r.fords {
            let lonely = f
                .bridges
                .iter()
                .filter(|b| b.stream.is_none())
                .all(|b| (b.x - fd.x).abs() > 300.0);
            if lonely && (300.0..900.0).contains(&fd.x) {
                return (seed, fd.x);
            }
        }
    }
    panic!("no forded field");
}

/// South and north banks, 40 m off the water, at `x`.
fn banks(sim: &BattleSim, x: f64) -> (P, P) {
    let r = sim.field().river.clone().unwrap();
    let half = r.width_at(x) * 0.5 + 40.0;
    ((x, r.center_z(x) - half), (x, r.center_z(x) + half))
}

#[test]
fn straight_line_preview_is_the_order() {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(&data, &["unit_knights"]),
            units(&data, &["unit_urban_militia"]),
            None,
        ),
        3,
    )
    .unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 400.0, 300.0, 0.0);
    place(&mut sim, 1, 1100.0, 780.0, 0.0);
    let to = (620.0, 480.0);
    let preview = sim.preview_path(0, to.0, to.1).unwrap();
    assert_eq!(preview, vec![to]);
    let walk = followed(&mut sim, 0, to, 200.0);
    assert_eq!(walk.heads, preview);
}

#[test]
fn bridge_preview_is_the_way_the_knights_ride() {
    let (seed, bx) = bridged_seed();
    let mut sim = river_lab(seed);
    let (south, north) = banks(&sim, bx - 120.0);
    place(&mut sim, 0, south.0, south.1, 0.0);
    let preview = sim.preview_path(0, north.0, north.1).unwrap();
    assert!(preview.len() >= 3, "by the bridge: {preview:?}");
    assert_eq!(*preview.last().unwrap(), north);
    let from = (sim.units()[0].x, sim.units()[0].z);
    let walk = followed(&mut sim, 0, north, 300.0);
    // Exactly the same waypoints, in the same order (the knights hesitate
    // between the two ends of the bridge a few ticks: first visits).
    assert_eq!(walk.first_visits(), preview, "preview and ride differ");
    assert!(sim.units()[0].destination.is_none(), "arrived");
    let off = walk
        .trail
        .iter()
        .map(|&p| off_path(p, from, &preview))
        .fold(0.0, f64::max);
    assert!(off < 5.0, "strayed {off:.1} m from the preview");
}

#[test]
fn ford_preview_is_the_way_the_knights_ride() {
    let (seed, fx) = forded_seed();
    let mut sim = river_lab(seed);
    let (south, _) = banks(&sim, fx - 150.0);
    let (_, north) = banks(&sim, fx - 100.0);
    place(&mut sim, 0, south.0, south.1, 0.0);
    let preview = sim.preview_path(0, north.0, north.1).unwrap();
    assert!(preview.len() >= 3, "by the ford: {preview:?}");
    let f = sim.field();
    assert!(preview[..preview.len() - 1]
        .iter()
        .all(|p| f.water_kind(p.0, p.1) != Some(Water::Deep)));
    let from = (sim.units()[0].x, sim.units()[0].z);
    let walk = followed(&mut sim, 0, north, 300.0);
    assert!(sim.units()[0].destination.is_none(), "arrived");
    // The same waypoints in the same order; in the ford itself the riders
    // make for the bank at their own x, a few metres from the far end the
    // preview (taken from the near end) gives.
    let visits = walk.first_visits();
    let mut k = 0;
    for v in &visits {
        let j = preview
            .iter()
            .position(|p| close(*v, *p, 3.0))
            .unwrap_or_else(|| panic!("headed for {v:?} off the preview {preview:?}"));
        assert!(j >= k, "{v:?} out of the preview's order");
        k = j;
    }
    assert_eq!(k, preview.len() - 1);
    assert_eq!(visits.first(), preview.first());
    assert_eq!(visits.last(), preview.last());
    let off = walk
        .trail
        .iter()
        .map(|&p| off_path(p, from, &preview))
        .fold(0.0, f64::max);
    assert!(off < 5.0, "strayed {off:.1} m from the preview");
}

#[test]
fn deep_water_without_crossing_is_unreachable_for_horse() {
    let (seed, bx) = bridged_seed();
    let sim = river_lab(seed);
    let r = sim.field().river.clone().unwrap();
    // The middle of the river, off the bridge.
    let x = bx - 150.0;
    let z = r.center_z(x);
    assert_eq!(sim.field().water_kind(x, z), Some(Water::Deep));
    assert_eq!(sim.preview_path(0, x, z), Err(PreviewError::Unreachable));
    assert_eq!(
        sim.preview_path(0, -50.0, 100.0),
        Err(PreviewError::OutsideField)
    );
}

fn siege_lab(seed: u64) -> BattleSim {
    let data = data();
    let attacker = units(&data, &["unit_mounted_sergeants", "unit_men_at_arms_foot"]);
    let defender = units(&data, &["unit_urban_militia"]);
    let siege = SiegeSetup {
        fortification: 2,
        breach: 0,
    };
    let mut sim = BattleSim::new(setup(attacker, defender, Some(siege)), seed).unwrap();
    lab(&mut sim);
    sim
}

#[test]
fn siege_preview_is_the_way_through_the_breach() {
    let mut sim = siege_lab(11);
    let works = sim.siege().unwrap().clone();
    let front = works.front_walls();
    let breach = *front.last().unwrap();
    sim.siege_mut().unwrap().pieces[breach].hp = 0.0;
    let works = sim.siege().unwrap().clone();
    let (bx, _) = works.pieces[breach].midpoint();
    let (gx, gz) = works.pieces[works.gate].midpoint();
    place(&mut sim, 0, gx + (gx - bx) * 1.5, gz - 60.0, 0.0);
    place(&mut sim, 2, 1100.0, 780.0, 0.0);
    let to = works.center;
    let from = (sim.units()[0].x, sim.units()[0].z);
    let preview = sim.preview_path(0, to.0, to.1).unwrap();
    assert!(preview.len() >= 3, "round the walls: {preview:?}");
    assert_eq!(*preview.last().unwrap(), to);
    // The preview strings the A* path of the order: its waypoints are cells
    // of that path, in order.
    let cells = sim.siege_route(SideId::Attacker, from, to).unwrap();
    let mut at = 0;
    for p in &preview[..preview.len() - 1] {
        let k = cells[at..]
            .iter()
            .position(|c| c == p)
            .unwrap_or_else(|| panic!("{p:?} is not a cell of the way after {at}"));
        at += k;
    }
    let walk = followed(&mut sim, 0, to, 300.0);
    assert!(sim.units()[0].destination.is_none(), "arrived");
    // The order heads first for the first waypoint of the preview, then only
    // for cells of the same path (pulled from where the regiment stands),
    // for the destination, or off a wall face it was pressed against.
    assert_eq!(walk.heads[0], preview[0]);
    for h in walk.first_visits() {
        let off_wall = works
            .nearest_intact(h.0, h.1)
            .is_some_and(|(_, d)| d <= works.band() + 3.0 + 1e-6);
        assert!(
            cells.contains(&h) || h == to || off_wall,
            "headed for {h:?}, off the previewed way"
        );
    }
    // Pushed by the houses and the walls the regiment strays a little from
    // the strung line of the preview, never by more than three cells.
    let off = walk
        .trail
        .iter()
        .map(|&p| off_path(p, from, &preview))
        .fold(0.0, f64::max);
    assert!(off < 3.0 * 4.0 + 3.0, "strayed {off:.1} m from the preview");
}

#[test]
fn previews_leave_the_battle_untouched() {
    let mut a = siege_lab(11);
    let mut b = siege_lab(11);
    for sim in [&mut a, &mut b] {
        sim.issue_command(Command::Move {
            units: vec![1],
            x: 600.0,
            z: 300.0,
            run: false,
            facing: None,
        })
        .unwrap();
        run(sim, 5.0);
    }
    let center = a.siege().unwrap().center;
    let before = state_digest(&a);
    for k in 0..100 {
        let _ = a.preview_path(0, center.0 + k as f64, center.1);
        let _ = a.preview_path(1, 300.0 + k as f64 * 3.0, 200.0);
        let _ = a.preview_group(&[0, 1], center.0, center.1 - k as f64, None);
        let _ = a.hover_context(center.0, center.1 + k as f64, &[0, 1], SideId::Attacker);
    }
    assert_eq!(state_digest(&a), before);
    for sim in [&mut a, &mut b] {
        sim.issue_command(Command::Move {
            units: vec![0, 1],
            x: center.0,
            z: center.1,
            run: false,
            facing: None,
        })
        .unwrap();
        run(sim, 60.0);
    }
    assert_eq!(
        state_digest(&a),
        state_digest(&b),
        "the previews changed the battle"
    );
}

#[test]
#[ignore = "CB-M2: hover table, filled in the next step"]
fn hover_context_table() {
    let sim = siege_lab(11);
    let h = sim.hover_context(0.0, 0.0, &[], SideId::Attacker);
    assert_eq!(h.context, HoverKind::None);
}
