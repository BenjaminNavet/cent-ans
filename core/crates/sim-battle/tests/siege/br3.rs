//! BR3 (ADR 0047): dense besieged town and solid street furniture.

use crate::common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleSim, FireState, PropKind, SideId, SiegeLayout, SiegeSetup, TownRules};

fn siege(data: &GameData, town: Option<&str>, seed: u64) -> BattleSim {
    let mut setup = setup(
        units(data, &["unit_men_at_arms_foot", "unit_urban_militia"]),
        units(data, &["unit_urban_militia", "unit_crossbowmen"]),
        Some(SiegeSetup {
            fortification: 2,
            breach: 0,
            ..Default::default()
        }),
    );
    setup.siege_layout =
        town.map(|id| SiegeLayout::from_landmark(&data.landmarks[id]).expect("siege.battle"));
    BattleSim::new(setup, seed).unwrap()
}

fn towns(data: &GameData) -> Vec<Option<String>> {
    let mut out: Vec<Option<String>> = vec![None];
    out.extend(data.landmarks.keys().map(|k| Some(k.clone())));
    out
}

#[test]
fn every_opening_reaches_the_square() {
    let data = data();
    for town in towns(data) {
        for seed in [1, 2, 3] {
            let mut sim = siege(data, town.as_deref(), seed);
            let count = sim.siege().unwrap().pieces.len();
            for p in 0..count {
                sim.siege_mut().unwrap().pieces[p].hp = 0.0;
            }
            let works = sim.siege().unwrap().clone();
            for (p, piece) in works.pieces.iter().enumerate() {
                let (mx, mz) = piece.midpoint();
                let (nx, nz) = piece.outward();
                let from = (mx + nx * 25.0, mz + nz * 25.0);
                let route = sim.siege_route(SideId::Attacker, from, works.center);
                assert!(
                    route.is_some(),
                    "{town:?} seed {seed}: no way from piece {p} to the square"
                );
            }
        }
    }
}

#[test]
fn the_towns_are_dense() {
    let data = data();
    for town in towns(data) {
        let sim = siege(data, town.as_deref(), 1);
        let works = sim.siege().unwrap();
        let blocks = works.houses.iter().filter(|h| !h.suburb).count();
        println!("{town:?}: {blocks} blocks, {} props", works.props.len());
        let least = if town.is_none() { 45 } else { 30 };
        assert!(blocks >= least, "{town:?}: {blocks} blocks");
        assert_eq!(
            works.houses.iter().filter(|h| h.church).count(),
            1,
            "{town:?}: one church"
        );
    }
}

#[test]
fn props_are_deterministic_and_stand_against_their_houses() {
    let data = data();
    let clearance = TownRules::bundled().block.clearance_m;
    for town in towns(data) {
        let a = siege(data, town.as_deref(), 4);
        let b = siege(data, town.as_deref(), 4);
        let works = a.siege().unwrap();
        assert_eq!(works.props, b.siege().unwrap().props, "{town:?}");
        assert!(
            works.props.iter().any(|p| p.kind == PropKind::Well),
            "{town:?}: a well on the square"
        );
        assert!(
            works
                .props
                .iter()
                .filter(|p| p.kind == PropKind::Stall)
                .count()
                >= 6,
            "{town:?}: a market"
        );
        assert!(
            works.props.iter().filter(|p| p.house.is_some()).count() >= 20,
            "{town:?}: props along the streets"
        );
        for prop in &works.props {
            let f = prop.footprint();
            for (i, h) in works.houses.iter().enumerate() {
                assert!(
                    h.footprint().distance_to(&f) > 0.1,
                    "{town:?}: {prop:?} in house {i}"
                );
            }
            match prop.house {
                // Against its house, within the clearance kept by the
                // regiments: it never narrows a street.
                Some(h) => assert!(
                    works.houses[h].footprint().distance_to(&f) < 1.0
                        && f.bounding_radius() * 2.0 < clearance + works.houses[h].depth,
                    "{town:?}: {prop:?}"
                ),
                None => assert!(works.in_square(prop.x, prop.z), "{town:?}: {prop:?}"),
            }
        }
    }
}

#[test]
fn the_market_leaves_the_streets_open() {
    let data = data();
    let sim = siege(data, None, 2);
    let works = sim.siege().unwrap();
    let passage = TownRules::bundled().props.market.passage_m;
    for prop in works.props.iter().filter(|p| p.house.is_none()) {
        for piece in &works.pieces {
            let f = prop.footprint();
            let d = f.distance_to_segment(works.center, piece.midpoint());
            assert!(
                prop.kind == PropKind::Well || d >= passage * 0.5 - 1.0,
                "{prop:?} in the street to {:?}: {d:.1}",
                piece.midpoint()
            );
        }
    }
}

#[test]
fn burnt_houses_lose_their_props() {
    let data = data();
    let mut sim = siege(data, None, 3);
    let works = sim.siege().unwrap();
    let house = works.props.iter().find_map(|p| p.house).unwrap();
    let before = works.standing_props().count();
    let own = works
        .props
        .iter()
        .filter(|p| p.house == Some(house))
        .count();
    sim.siege_mut().unwrap().houses[house].fire.state = FireState::Burnt;
    let works = sim.siege().unwrap();
    assert_eq!(works.standing_props().count(), before - own);
    assert!(works.standing_props().all(|p| p.house != Some(house)));
}

/// Every figure of every regiment is outside every footprint near it.
fn assert_figures_clear(sim: &BattleSim, what: &str) {
    for unit in sim.units() {
        let poses = sim.soldier_poses(unit, 1.0);
        let near = sim.obstacles_near(unit.x, unit.z, 200.0);
        for p in &poses {
            for o in &near {
                assert!(
                    !o.contains(p[0], p[2], -0.01),
                    "{what}: {} figure at ({:.1}, {:.1}) in {o:?}",
                    unit.name,
                    p[0],
                    p[2]
                );
            }
        }
    }
}

#[test]
fn no_figure_stands_in_a_house_or_a_prop_during_a_siege() {
    let data = data();
    for town in [None, Some("paris")] {
        let mut sim = siege(data, town, 6);
        lab(&mut sim);
        let works = sim.siege().unwrap().clone();
        // Every regiment dropped on a block, then on a prop of the market.
        let ids: Vec<u32> = sim.units().iter().map(|u| u.id).collect();
        let blocks: Vec<_> = works.houses.iter().filter(|h| !h.suburb).collect();
        for (k, &id) in ids.iter().enumerate() {
            let h = blocks[(k * 7) % blocks.len()];
            place(&mut sim, id, h.x, h.z, 0.3 * k as f64);
            sim.units_mut()[id as usize].on_wall = false;
            sim.units_mut()[id as usize].climbing = None;
        }
        assert_figures_clear(&sim, "blocks");
        let stall = works
            .props
            .iter()
            .find(|p| p.kind == PropKind::Stall)
            .unwrap();
        for &id in &ids {
            place(&mut sim, id, stall.x, stall.z, 1.0);
        }
        assert_figures_clear(&sim, "market");
        // And while the battle runs.
        let mut sim = siege(data, town, 6);
        sim.set_ai(SideId::Attacker, true);
        sim.set_ai(SideId::Defender, true);
        if sim.is_deploying() {
            sim.start_battle().unwrap();
        }
        for _ in 0..6 {
            run(&mut sim, 30.0);
            assert_figures_clear(&sim, "battle");
        }
    }
}

#[test]
fn no_figure_stands_in_a_village_house() {
    let data = data();
    let mut s = setup(
        units(data, &["unit_men_at_arms_foot", "unit_longbowmen"]),
        units(data, &["unit_urban_militia", "unit_knights"]),
        None,
    );
    s.village = Some(true);
    let mut sim = BattleSim::new(s, 5).unwrap();
    lab(&mut sim);
    let village = sim.field().village.clone().expect("a village");
    assert!(!sim.village_props().is_empty(), "props before the houses");
    assert_eq!(
        sim.village_props(),
        BattleSim::new(sim_setup_village(data), 5)
            .unwrap()
            .village_props()
    );
    let ids: Vec<u32> = sim.units().iter().map(|u| u.id).collect();
    for (k, h) in village.houses.iter().enumerate() {
        let id = ids[k % ids.len()];
        place(&mut sim, id, h.x, h.z, h.yaw);
        assert_figures_clear(&sim, "village");
    }
}

fn sim_setup_village(data: &GameData) -> sim_battle::BattleSetup {
    let mut s = setup(
        units(data, &["unit_men_at_arms_foot", "unit_longbowmen"]),
        units(data, &["unit_urban_militia", "unit_knights"]),
        None,
    );
    s.village = Some(true);
    s
}

/// Plan of each town as SVG in `$BR3_SVG_DIR` (layout review).
#[test]
#[ignore = "debug: writes the town plans as SVG"]
fn dump_town_plans() {
    let data = data();
    let dir = std::env::var("BR3_SVG_DIR").unwrap_or_else(|_| ".".into());
    for town in towns(data) {
        let sim = siege(data, town.as_deref(), 1);
        let works = sim.siege().unwrap();
        let (cx, cz) = works.center;
        let mut svg = String::from(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='-420 -420 840 840' width='840' height='840'><rect x='-420' y='-420' width='840' height='840' fill='#eee'/>",
        );
        let pt = |x: f64, z: f64| format!("{:.1},{:.1}", x - cx, cz - z);
        for p in &works.pieces {
            let colour = if p.kind == sim_battle::PieceKind::Gate {
                "red"
            } else {
                "#555"
            };
            svg += &format!(
                "<polyline points='{} {}' stroke='{colour}' stroke-width='4'/>",
                pt(p.a.0, p.a.1),
                pt(p.b.0, p.b.1),
            );
        }
        svg += &format!(
            "<circle cx='0' cy='0' r='{}' fill='#ccb'/>",
            works.square_radius
        );
        for line in works.landmark.iter().flat_map(|l| l.streets.iter()) {
            let pts: Vec<String> = line.iter().map(|&(x, z)| pt(x, z)).collect();
            svg += &format!(
                "<polyline points='{}' stroke='#69c' stroke-width='2' fill='none'/>",
                pts.join(" ")
            );
        }
        for h in &works.houses {
            let pts: Vec<String> = h
                .footprint()
                .corners()
                .iter()
                .map(|&(x, z)| pt(x, z))
                .collect();
            let fill = if h.church {
                "#a66"
            } else if h.suburb {
                "#9a7"
            } else {
                "#b98"
            };
            svg += &format!(
                "<polygon points='{}' fill='{fill}' stroke='#333' stroke-width='0.3'/>",
                pts.join(" ")
            );
        }
        for p in &works.props {
            let pts: Vec<String> = p
                .footprint()
                .corners()
                .iter()
                .map(|&(x, z)| pt(x, z))
                .collect();
            svg += &format!("<polygon points='{}' fill='#236'/>", pts.join(" "));
        }
        svg += "</svg>";
        let name = town.clone().unwrap_or_else(|| "generic".into());
        std::fs::write(format!("{dir}/{name}.svg"), svg).unwrap();
    }
}
