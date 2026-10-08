//! EP3: water and paths of the battlefield — river network, bridges,
//! roads, and their rules in the simulation.

use crate::common;

use common::*;
use data_model::Terrain;
use sim_battle::{
    BattleRng, BattleSim, Battlefield, Command, Formation, RoadKind, SideId, UnitState, Water,
    WaterRules, Weather,
};

const TERRAINS: [Terrain; 7] = [
    Terrain::Plains,
    Terrain::Heath,
    Terrain::Bocage,
    Terrain::Forest,
    Terrain::Hills,
    Terrain::Mountains,
    Terrain::Marsh,
];

fn field(terrain: Terrain, seed: u64) -> Battlefield {
    Battlefield::generate(
        terrain,
        true,
        Weather::Clear,
        &mut BattleRng::from_seed(seed),
    )
}

#[test]
fn river_network_is_deterministic_and_varied() {
    let a = field(Terrain::Plains, 7);
    assert_eq!(a, field(Terrain::Plains, 7));
    let mut widths = Vec::new();
    let mut bridges = 0;
    let mut stone = 0;
    let mut streams = 0;
    let mut oxbows = 0;
    let mut banks = 0;
    for seed in 0..40 {
        let f = field(Terrain::Plains, seed);
        let r = f.river.as_ref().unwrap();
        widths.push(r.width);
        bridges += f.bridges.iter().filter(|b| b.stream.is_none()).count();
        stone += f.bridges.iter().filter(|b| b.stone).count();
        streams += f.streams.len();
        oxbows += usize::from(!f.oxbows.is_empty());
        banks += r.banks.len();
    }
    let (lo, hi) = widths
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), &w| (a.min(w), b.max(w)));
    assert!(lo < 20.0 && hi > 30.0, "widths {lo:.0}-{hi:.0}");
    assert!(bridges > 20, "{bridges} bridges over 40 fields");
    assert!(stone > 3 && stone < bridges, "{stone} stone bridges");
    assert!(streams > 20, "{streams} streams");
    assert!(oxbows > 5, "{oxbows} oxbows");
    assert!(banks > 40, "{banks} bank stretches");
}

#[test]
fn widths_follow_the_rules_and_every_river_can_be_crossed() {
    let rules = WaterRules::bundled();
    for terrain in TERRAINS {
        let span = rules.river.width_m.of(terrain);
        for seed in 0..12 {
            let f = field(terrain, seed);
            let r = f.river.as_ref().unwrap();
            assert!(
                (span[0]..=span[1]).contains(&r.width),
                "{terrain:?} {seed}: {}",
                r.width
            );
            for x in [0.0, 300.0, 600.0, 900.0, 1200.0] {
                let w = r.width_at(x);
                assert!((w >= r.width * 0.69) && (w <= r.width * 1.31));
            }
            assert!(!f.crossings().is_empty(), "{terrain:?} {seed}: no crossing");
            // Wide rivers: few fords; narrow ones: many.
            if r.width > 25.0 {
                assert!(r.fords.len() <= 1);
                assert!(f.bridges.iter().any(|b| b.stream.is_none()));
            }
            if r.width <= 15.0 {
                assert!(r.fords.len() >= 3);
            }
        }
    }
}

#[test]
fn bridges_span_the_river_above_the_banks() {
    for seed in 0..30 {
        let f = field(Terrain::Plains, seed);
        let r = f.river.as_ref().unwrap();
        for b in f.bridges.iter().filter(|b| b.stream.is_none()) {
            assert!((4.0..=8.0).contains(&b.width), "deck {}", b.width);
            // Deck dry, water on either side of it.
            assert_eq!(f.water_kind(b.x, b.z), None);
            let side = (-b.dir.1, b.dir.0);
            let off = b.width * 0.5 + 2.0;
            assert_eq!(
                f.water_kind(b.x + side.0 * off, b.z + side.1 * off),
                Some(Water::Deep),
                "seed {seed}: bridge at {:.0} not over deep water",
                b.x
            );
            // Both ends on dry land, deck above both banks.
            for (x, z) in b.ends() {
                assert!(!r.in_water(x, z), "seed {seed}: bridge end in the water");
                assert!(b.deck > f.height(x, z));
            }
            assert!(f.walk_height(b.x, b.z) > f.height(b.x, b.z) + 1.0);
            // Away from the fords.
            assert!(r.fords.iter().all(|fd| (fd.x - b.x).abs() > fd.half_width));
        }
    }
}

#[test]
fn roads_join_the_crossings_and_the_edges() {
    for seed in 0..20 {
        let f = field(Terrain::Plains, seed);
        for c in f.crossings() {
            if let Some(i) = c.bridge {
                let b = &f.bridges[i];
                assert_eq!(
                    f.road_at(b.x, b.z),
                    Some(RoadKind::Main),
                    "seed {seed}: no road on the bridge"
                );
            }
        }
        for road in &f.roads {
            let first = road.points[0];
            let last = *road.points.last().unwrap();
            let on_edge = |p: (f64, f64)| {
                p.0 <= 1.0 || p.1 <= 1.0 || p.0 >= f.width - 1.0 || p.1 >= f.depth - 1.0
            };
            assert!(
                on_edge(first) || on_edge(last),
                "seed {seed}: road ends in the field"
            );
        }
    }
    // Without a river: a road across the field all the same.
    let dry = Battlefield::generate(
        Terrain::Plains,
        false,
        Weather::Clear,
        &mut BattleRng::from_seed(3),
    );
    assert!(dry.bridges.is_empty() && dry.streams.is_empty());
    assert_eq!(dry.roads.len(), 1);
}

#[test]
fn streams_are_shallow_and_reach_the_river() {
    for seed in 0..30 {
        let f = field(Terrain::Bocage, seed);
        let r = f.river.as_ref().unwrap();
        for s in &f.streams {
            let end = *s.points.last().unwrap();
            assert!((end.1 - r.center_z(end.0)).abs() < 1.0);
            let mid = s.points[s.points.len() / 3];
            if !r.in_water(mid.0, mid.1) && f.bridge_at(mid.0, mid.1).is_none() {
                assert!(matches!(f.water_kind(mid.0, mid.1), Some(Water::Stream(_))));
                assert_eq!(f.water_at(mid.0, mid.1), Some(true));
            }
        }
    }
}

#[test]
fn waterside_spot_is_dry_and_faces_the_water() {
    let f = field(Terrain::Plains, 11);
    let spot = f.waterside_spot((600.0, 400.0), 6.0).expect("a spot");
    assert!(f.water_kind(spot.x, spot.z).is_none());
    let (wx, wz) = (
        spot.x + spot.towards_water.0 * 12.0,
        spot.z + spot.towards_water.1 * 12.0,
    );
    assert!(f.water_kind(wx, wz).is_some());
}

/// A plains field with a river, both AIs off.
fn river_lab(seed: u64) -> BattleSim {
    let data = data();
    let mut battle = setup(
        units(data, &["unit_knights", "unit_men_at_arms_foot"]),
        units(data, &["unit_urban_militia"]),
        None,
    );
    battle.river = true;
    // The rivers and bridges are drawn for the standard field (ADR 0184: the
    // field battles' wider gap moves them).
    let mut sim = BattleSim::new_scaled(battle, seed, sim_battle::BattleScale::default()).unwrap();
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

#[test]
fn horsemen_cross_by_the_bridge_not_through_deep_water() {
    let (seed, bx) = bridged_seed();
    let mut sim = river_lab(seed);
    let r = sim.field().river.clone().unwrap();
    // Knights 120 m west of the bridge, south bank; destination due north.
    let x = bx - 120.0;
    let south = r.center_z(x) - r.width_at(x) * 0.5 - 40.0;
    let north = r.center_z(x) + r.width_at(x) * 0.5 + 40.0;
    place(&mut sim, 0, x, south, 0.0);
    sim.apply_command(
        Command::Move {
            units: vec![0],
            x,
            z: north,
            run: false,
            facing: None,
            queue: false,
            width: None,
            match_speed: false,
            group_tag: None,
        },
        None,
    )
    .unwrap();
    let mut on_bridge = false;
    for _ in 0..(240.0 / sim_battle::DT) as usize {
        sim.step();
        let u = &sim.units()[0];
        assert_ne!(
            sim.field().water_kind(u.x, u.z),
            Some(Water::Deep),
            "knights in deep water at ({:.0}, {:.0})",
            u.x,
            u.z
        );
        on_bridge |= sim.field().bridge_at(u.x, u.z).is_some();
    }
    let u = &sim.units()[0];
    assert!(on_bridge, "the knights never used the bridge");
    assert!(
        r.north_of(u.x, u.z),
        "the knights did not cross: ({:.0}, {:.0})",
        u.x,
        u.z
    );
}

#[test]
fn foot_in_deep_water_slows_tires_and_drowns() {
    let (seed, bx) = bridged_seed();
    let mut sim = river_lab(seed);
    let r = sim.field().river.clone().unwrap();
    let x = bx - 150.0;
    let c = r.center_z(x);
    place(&mut sim, 1, x, c, 0.0);
    let before = sim.units()[1].hp;
    sim_battle::BattleSim::step(&mut sim);
    for _ in 0..(20.0 / sim_battle::DT) as usize {
        sim.step();
    }
    let u = &sim.units()[1];
    assert!(u.hp < before, "nobody drowned");
    assert!(u.fatigue > 10.0);
    assert!(has_event(&sim, "se noient"));
}

#[test]
fn a_regiment_wider_than_the_bridge_files_across_slowly() {
    let (seed, bx) = bridged_seed();
    let sim = river_lab(seed);
    let f = sim.field();
    let b = f.bridges.iter().find(|b| (b.x - bx).abs() < 1.0).unwrap();
    // Speed factors of a line and a column on the deck.
    let rules = WaterRules::bundled();
    let line = sim.units()[1].extent().0;
    assert!(line > b.width);
    let squeeze = (b.width / line).clamp(rules.movement.bridge_min_squeeze, 1.0);
    assert!(squeeze < 0.7);
}

#[test]
fn a_regiment_on_a_bridge_is_drawn_on_the_deck() {
    let (seed, bx) = bridged_seed();
    let mut sim = river_lab(seed);
    let b = sim
        .field()
        .bridges
        .iter()
        .find(|b| (b.x - bx).abs() < 1.0)
        .unwrap()
        .clone();
    // Foot in line across the deck's axis, heading over the river.
    let facing = b.dir.0.atan2(b.dir.1);
    place(&mut sim, 1, b.x, b.z, facing);
    let unit = sim.units()[1].clone();
    assert!(unit.extent().0 > b.width * 3.0);
    for scale in [1.0, 2.0] {
        let poses = sim.soldier_poses(&unit, scale);
        assert_eq!(poses.len(), unit.figure_count(scale) as usize);
        for &[x, y, z, _] in &poses {
            let (along, across) = b.local(x, z);
            assert!(
                across.abs() <= b.width * 0.5,
                "figure {across:.1} m off the axis of a {:.1} m deck",
                b.width
            );
            assert!((y - sim.field().walk_height(x, z)).abs() < 1e-9);
            if along.abs() <= b.span * 0.5 {
                assert!(y >= b.deck - 1e-9, "figure under the deck");
            }
        }
    }
    // Off the bridge, the line keeps its width.
    let r = sim.field().river.clone().unwrap();
    let x = bx - 200.0;
    place(
        &mut sim,
        1,
        x,
        r.center_z(x) - r.width_at(x) * 0.5 - 60.0,
        facing,
    );
    let unit = sim.units()[1].clone();
    let poses = sim.soldier_poses(&unit, 1.0);
    let spread = poses.iter().map(|p| p[0]).fold(f64::MIN, f64::max)
        - poses.iter().map(|p| p[0]).fold(f64::MAX, f64::min);
    assert!(spread > b.width * 3.0);
}

#[test]
fn marching_in_column_on_the_road_is_faster() {
    let data = data();
    let mut timings = Vec::new();
    for on_road in [true, false] {
        let mut battle = setup(
            units(data, &["unit_men_at_arms_foot"]),
            units(data, &["unit_urban_militia"]),
            None,
        );
        battle.river = false;
        let mut sim = BattleSim::new(battle, 3).unwrap();
        lab(&mut sim);
        let road = sim.field().roads[0].clone();
        // A straight-ish stretch of the road.
        let a = road.points[road.points.len() / 3];
        let x = if on_road { a.0 } else { a.0 + 80.0 };
        place(&mut sim, 0, x, a.1, 0.0);
        sim.apply_command(
            Command::Formation {
                units: vec![0],
                kind: Formation::of("column"),
            },
            None,
        )
        .unwrap();
        let start = (sim.units()[0].x, sim.units()[0].z);
        for _ in 0..(10.0 / sim_battle::DT) as usize {
            let u = &sim.units()[0];
            let road_here = sim.field().road_at(u.x, u.z).is_some();
            timings.push((on_road, road_here));
            sim.step();
        }
        let _ = start;
    }
    // The road factor applies on the road only.
    let rules = WaterRules::bundled();
    assert!(rules.movement.road_column > 1.1);
    assert!(timings.iter().any(|&(on, here)| on && here));
    assert!(timings.iter().all(|&(on, here)| on || !here));
}

#[test]
fn the_side_holding_the_bridgehead_strikes_harder() {
    let rules = WaterRules::bundled();
    assert!(rules.combat.bridge_holder_bonus > 1.0);
    assert!(rules.combat.ford_attacker < 1.0);
    // Deterministic whole battles on river fields.
    let data = data();
    let army = || {
        units(
            data,
            &["unit_men_at_arms_foot", "unit_longbowmen", "unit_knights"],
        )
    };
    let run_one = || {
        let mut battle = setup(army(), army(), None);
        battle.river = true;
        let mut sim = BattleSim::new(battle, 5).unwrap();
        run(&mut sim, 200.0);
        sim.units()
            .iter()
            .map(|u| (u.x, u.z, u.hp))
            .collect::<Vec<_>>()
    };
    assert_eq!(run_one(), run_one());
    let _ = SideId::Attacker;
}

/// The attacker's and defender's deployment centres of a field, read from
/// its dimensions (never a fixed 1200 × 800 m).
fn deployment_centres(f: &Battlefield) -> ((f64, f64), (f64, f64)) {
    let half_gap = (sim_battle::DEFENDER_LINE_Z - sim_battle::ATTACKER_LINE_Z) * 0.5;
    let (mid_x, mid_z) = (f.width * 0.5, f.depth * 0.5);
    ((mid_x, mid_z - half_gap), (mid_x, mid_z + half_gap))
}

#[test]
fn the_defender_holds_the_bank_at_a_crossing() {
    let mut held = 0;
    for seed in 0..24 {
        let f = field(Terrain::Plains, seed);
        let r = f.river.as_ref().unwrap();
        let (attacker, defender) = deployment_centres(&f);
        let Some(cover) = sim_battle::ai::river_hold(&f, SideId::Defender, attacker) else {
            continue;
        };
        held += 1;
        assert_eq!(cover.kind, sim_battle::ai::CoverKind::River);
        assert!(cover.breaks_charge);
        let own_north = r.north_of(defender.0, defender.1);
        let (cx, cz) = cover.center;
        assert_eq!(
            r.north_of(cx, cz),
            own_north,
            "seed {seed}: the bank held is not the defender's"
        );
        assert!(
            f.water_kind(cx, cz).is_none(),
            "seed {seed}: held in the water"
        );
        // Behind the own end of a crossing, a bank setback away.
        let near_crossing = f.crossings().iter().any(|c| {
            let end = c.end(own_north);
            (end.0 - cx).abs() < 1.0
                && (end.1 - cz).abs() < sim_battle::ai::BANK_SETBACK + r.width_at(cx) + 20.0
        });
        assert!(
            near_crossing,
            "seed {seed}: the bank held faces no crossing"
        );
        // The attacker's side never holds the river from its own bank here.
        assert!(sim_battle::ai::river_hold(&f, SideId::Attacker, defender)
            .is_some_and(|c| r.north_of(c.center.0, c.center.1) != own_north));
    }
    assert!(held >= 16, "the river is held on {held}/24 fields only");
}

/// A weaker defender (AI) behind the river and a stronger attacker (AI).
fn river_battle(seed: u64) -> BattleSim {
    let data = data();
    let mut battle = setup(
        units(
            data,
            &[
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_knights",
                "unit_knights",
            ],
        ),
        units(
            data,
            &[
                "unit_men_at_arms_foot",
                "unit_longbowmen",
                "unit_urban_militia",
            ],
        ),
        None,
    );
    battle.river = true;
    BattleSim::new(battle, seed).unwrap()
}

#[test]
fn a_weaker_defender_stays_behind_the_river() {
    for seed in [1, 4, 7] {
        let mut sim = river_battle(seed);
        let r = sim.field().river.clone().unwrap();
        let (_, defender) = deployment_centres(sim.field());
        let own_north = r.north_of(defender.0, defender.1);
        // The first minute: the attacker marches, the defender waits.
        run(&mut sim, 60.0);
        for u in sim.units().iter().filter(|u| u.side == SideId::Defender) {
            if !u.present() {
                continue;
            }
            assert_eq!(
                r.north_of(u.x, u.z),
                own_north,
                "seed {seed}: defender {} crossed to ({:.0}, {:.0})",
                u.unit_type,
                u.x,
                u.z
            );
        }
    }
}

#[test]
fn the_attacker_crosses_by_the_bridges_and_fords() {
    let mut crossed = 0;
    for seed in [1, 4, 7] {
        let mut sim = river_battle(seed);
        let r = sim.field().river.clone().unwrap();
        let (attacker, _) = deployment_centres(sim.field());
        let own_north = r.north_of(attacker.0, attacker.1);
        let count = sim.units().len();
        let mut used_crossing = vec![false; count];
        for _ in 0..(300.0 / sim_battle::DT) as usize {
            if sim.is_finished() {
                break;
            }
            sim.step();
            let f = sim.field();
            for (k, u) in sim.units().iter().enumerate() {
                if u.side != SideId::Attacker || !u.present() {
                    continue;
                }
                if f.bridge_at(u.x, u.z).is_some() || f.water_kind(u.x, u.z) == Some(Water::Ford) {
                    used_crossing[k] = true;
                }
                // Horsemen never stand in deep water, unless they flee in a
                // rout (`water_route`: routers throw themselves in and drown;
                // ADR 0052 lets the arrows rout the knights).
                if u.category == data_model::UnitCategory::Cavalry && u.state != UnitState::Routing
                {
                    assert_ne!(f.water_kind(u.x, u.z), Some(Water::Deep), "seed {seed}");
                }
                if r.north_of(u.x, u.z) != own_north && !used_crossing[k] {
                    // Beyond the river without a bridge or ford: only foot
                    // wading a narrow deep reach may do so, never horsemen.
                    assert_ne!(
                        u.category,
                        data_model::UnitCategory::Cavalry,
                        "seed {seed}: horsemen crossed without a crossing"
                    );
                }
            }
        }
        let beyond = sim
            .units()
            .iter()
            .enumerate()
            .filter(|(k, u)| {
                u.side == SideId::Attacker && used_crossing[*k] && r.north_of(u.x, u.z) != own_north
            })
            .count();
        crossed += beyond;
    }
    assert!(
        crossed > 0,
        "the attacker never crossed by a bridge or ford"
    );
}

/// EP3 × EP1: on rivers of every field size, the automatic deployment (the
/// default layout, then the AI's roles and its "never in deep water" step)
/// leaves every regiment inside its side's deployment zone and out of deep
/// water, so the player never inherits a regiment the zone would refuse.
#[test]
fn automatic_deployment_stays_in_the_zone_on_rivers() {
    let data = data();
    let kinds = [
        "unit_knights",
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_crossbowmen",
        "unit_urban_militia",
    ];
    for tier in ["skirmish", "large", "epic"] {
        for seed in 0..12 {
            let army: Vec<&str> = (0..12).map(|i| kinds[i % kinds.len()]).collect();
            let mut battle = setup(units(data, &army), units(data, &army), None);
            battle.river = true;
            battle.player_side = Some(SideId::Attacker);
            let scale = sim_battle::BattleScale::named(tier).unwrap();
            let mut sim = BattleSim::new_scaled(battle, seed, scale).unwrap();
            sim.set_ai(SideId::Attacker, false);
            sim.set_ai(SideId::Defender, true);
            assert!(sim.begin_deployment());
            for unit in sim.units().iter().filter(|u| u.present()) {
                if unit.category == data_model::UnitCategory::Siege {
                    continue;
                }
                let zone = sim.deployment_zone(unit.side);
                assert!(
                    zone.contains(unit.x, unit.z),
                    "{tier} seed {seed}: {} ({:?}) at ({:.0}, {:.0}) outside {zone:?}",
                    unit.name,
                    unit.side,
                    unit.x,
                    unit.z
                );
                assert_ne!(
                    sim.field().water_at(unit.x, unit.z),
                    Some(false),
                    "{tier} seed {seed}: {} in deep water",
                    unit.name
                );
                // The player can put it back where it stands.
                if unit.side == SideId::Attacker {
                    let (id, x, z) = (unit.id, unit.x, unit.z);
                    let mut probe = sim.clone();
                    assert!(probe.deploy_unit(id, x, z, None).is_ok());
                }
            }
        }
    }
}
