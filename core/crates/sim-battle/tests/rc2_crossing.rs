//! RC2 (ADR 0141): a campaign battle fought at a river crossing — the
//! tactical field gets a wide river between the battle lines with the
//! crossing's single passage, the attacker on the far bank.

mod common;

use common::*;
use data_model::Terrain;
use sim_battle::{
    BattleCrossing, BattleRng, BattleSetup, BattleSim, Battlefield, CrossingStructure, FieldSize,
    RoadKind, SideId, Water, WaterRules, Weather,
};

const STRUCTURES: [CrossingStructure; 5] = [
    CrossingStructure::StoneBridge,
    CrossingStructure::WoodBridge,
    CrossingStructure::BoatBridge,
    CrossingStructure::Ford,
    CrossingStructure::Ferry,
];

/// A stronger attacker (AI) against a weaker defender (AI) at `structure`.
fn crossing_setup(structure: CrossingStructure, terrain: Terrain) -> BattleSetup {
    let data = data();
    let mut battle = setup(
        units(
            &data,
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
            &data,
            &[
                "unit_men_at_arms_foot",
                "unit_longbowmen",
                "unit_urban_militia",
            ],
        ),
        None,
    );
    battle.terrain = terrain;
    battle.crossing = Some(BattleCrossing {
        structure,
        name: "Pont des Tourelles".to_owned(),
        river: "Loire".to_owned(),
    });
    battle
}

fn zone_centre(sim: &BattleSim, side: SideId) -> (f64, f64) {
    let z = sim.deployment_zone(side);
    ((z.x0 + z.x1) * 0.5, (z.z0 + z.z1) * 0.5)
}

#[test]
fn one_passage_of_the_right_kind_between_the_lines() {
    let rules = WaterRules::bundled();
    for structure in STRUCTURES {
        for seed in 0..6u64 {
            for terrain in [Terrain::Plains, Terrain::Hills, Terrain::Forest] {
                let sim = BattleSim::new(crossing_setup(structure, terrain), seed).unwrap();
                let f = sim.field();
                let tag = format!("{structure:?} {terrain:?} seed {seed}");
                assert_eq!(f.crossing, Some(structure), "{tag}");
                let r = f.river.as_ref().expect("a river at a crossing");
                let crossings = f.crossings();
                assert_eq!(crossings.len(), 1, "{tag}: {crossings:?}");
                let c = crossings[0];
                let main_bridges: Vec<_> =
                    f.bridges.iter().filter(|b| b.stream.is_none()).collect();
                match structure {
                    CrossingStructure::StoneBridge
                    | CrossingStructure::WoodBridge
                    | CrossingStructure::BoatBridge => {
                        assert!(r.fords.is_empty(), "{tag}");
                        assert_eq!(main_bridges.len(), 1, "{tag}");
                        let b = main_bridges[0];
                        assert_eq!(c.bridge.map(|i| &f.bridges[i]), Some(b), "{tag}");
                        assert_eq!(b.stone, structure == CrossingStructure::StoneBridge);
                        if structure == CrossingStructure::BoatBridge {
                            assert_eq!(b.width, rules.river.bridge_width_m.wood[0], "{tag}");
                        }
                        let [lo, _] = rules.crossing.bridge_river_width_m;
                        assert!(r.width >= lo, "{tag}: river {:.0} m", r.width);
                    }
                    CrossingStructure::Ford | CrossingStructure::Ferry => {
                        assert!(main_bridges.is_empty(), "{tag}");
                        assert_eq!(r.fords.len(), 1, "{tag}");
                        assert!(c.ford.is_some(), "{tag}");
                        if structure == CrossingStructure::Ferry {
                            assert_eq!(
                                r.fords[0].half_width, rules.crossing.ferry_half_width_m,
                                "{tag}"
                            );
                        }
                    }
                }
                // Near the centre of the field.
                let (mx, mz) = c.middle();
                assert!(
                    (mx - f.width * 0.5).abs() <= rules.crossing.passage_jitter_m + 5.0,
                    "{tag}: passage at x {mx:.0}"
                );
                // Deep water everywhere else along the river.
                let mut x = 20.0;
                while x < f.width - 20.0 {
                    if (x - mx).abs() > 60.0 {
                        assert_eq!(
                            f.water_kind(x, r.center_z(x)),
                            Some(Water::Deep),
                            "{tag}: not deep at x {x:.0}"
                        );
                    }
                    x += 10.0;
                }
                // A main road through the passage.
                assert_eq!(f.road_at(mx, mz), Some(RoadKind::Main), "{tag}");
                // Opposite banks: the attacker on the far (south) bank, the
                // defender on the passage's bank it holds.
                let (ax, az) = zone_centre(&sim, SideId::Attacker);
                let (dx, dz) = zone_centre(&sim, SideId::Defender);
                assert!(!r.north_of(ax, az), "{tag}");
                assert!(r.north_of(dx, dz), "{tag}");
                for u in sim.units() {
                    assert_eq!(
                        r.north_of(u.x, u.z),
                        u.side == SideId::Defender,
                        "{tag}: {} deployed on the wrong bank",
                        u.unit_type
                    );
                    assert_ne!(f.water_kind(u.x, u.z), Some(Water::Deep), "{tag}");
                }
            }
        }
    }
}

#[test]
fn the_defender_holds_the_bridgehead() {
    for structure in STRUCTURES {
        let mut held = 0;
        for seed in 0..8u64 {
            let sim = BattleSim::new(crossing_setup(structure, Terrain::Plains), seed).unwrap();
            let f = sim.field();
            let attacker = zone_centre(&sim, SideId::Attacker);
            let Some(cover) = sim_battle::ai::river_hold(f, SideId::Defender, attacker) else {
                continue;
            };
            let r = f.river.as_ref().unwrap();
            assert!(r.north_of(cover.center.0, cover.center.1));
            let (mx, _) = f.crossings()[0].middle();
            assert!(
                (cover.center.0 - mx).abs() < 10.0,
                "{structure:?} seed {seed}: holds x {:.0}, passage at {mx:.0}",
                cover.center.0
            );
            held += 1;
        }
        assert!(held >= 6, "{structure:?}: held on {held}/8 fields");
    }
}

#[test]
fn the_attacker_forces_the_passage() {
    for structure in STRUCTURES {
        let mut used = 0;
        for seed in [1u64, 4] {
            let mut sim = BattleSim::new(crossing_setup(structure, Terrain::Plains), seed).unwrap();
            let count = sim.units().len();
            let mut on_passage = vec![false; count];
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
                    if f.bridge_at(u.x, u.z).is_some_and(|b| b.stream.is_none())
                        || f.water_kind(u.x, u.z) == Some(Water::Ford)
                    {
                        on_passage[k] = true;
                    }
                }
            }
            used += on_passage.iter().filter(|&&b| b).count();
        }
        assert!(used > 0, "{structure:?}: no attacker reached the passage");
    }
}

/// Without a crossing the field is the plain site's, draw for draw, and a
/// crossing leaves the battle stream where the plain site leaves it.
#[test]
fn no_crossing_keeps_the_field_and_the_draws() {
    let setup = crossing_setup(CrossingStructure::StoneBridge, Terrain::Plains);
    for seed in 0..8u64 {
        for river in [false, true] {
            let mut plain_setup = setup.clone();
            plain_setup.river = river;
            plain_setup.crossing = None;
            let site = plain_setup.field_site();
            let size = FieldSize::STANDARD;
            let mut a = BattleRng::from_seed(seed);
            let mut b = BattleRng::from_seed(seed);
            let mut c = BattleRng::from_seed(seed);
            let plain = Battlefield::generate_site_sized(&site, size, Weather::Clear, &mut a);
            let none =
                Battlefield::generate_site_crossing(&site, None, size, Weather::Clear, &mut b);
            let bridged = Battlefield::generate_site_crossing(
                &site,
                Some(CrossingStructure::StoneBridge),
                size,
                Weather::Clear,
                &mut c,
            );
            assert_eq!(plain, none, "seed {seed}");
            assert_eq!(plain.crossing, None);
            assert_ne!(plain, bridged);
            assert_eq!(a, b);
            assert_eq!(a, c, "seed {seed}: the crossing consumed battle draws");
            // The setup without a crossing builds the plain battle.
            let sim = BattleSim::new(plain_setup, seed).unwrap();
            assert_eq!(sim.field().crossing, None);
            assert_eq!(sim.setup().field_crossing(), None);
        }
    }
}

#[test]
fn a_siege_ignores_the_crossing() {
    let mut battle = crossing_setup(CrossingStructure::StoneBridge, Terrain::Plains);
    battle.siege = Some(sim_battle::SiegeSetup {
        fortification: 1,
        breach: 0,
        place: Default::default(),
        engines: None,
    });
    assert_eq!(battle.field_crossing(), None);
    let sim = BattleSim::new(battle, 3).unwrap();
    assert!(sim.field().river.is_none());
    assert!(sim.field().bridges.is_empty());
}
