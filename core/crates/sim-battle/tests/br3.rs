//! BR3 (ADR 0047): dense besieged town and solid street furniture.

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleSim, SideId, SiegeLayout, SiegeSetup};

fn siege(data: &GameData, town: Option<&str>, seed: u64) -> BattleSim {
    let mut setup = setup(
        units(data, &["unit_men_at_arms_foot", "unit_urban_militia"]),
        units(data, &["unit_urban_militia", "unit_crossbowmen"]),
        Some(SiegeSetup {
            fortification: 2,
            breach: 0,
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
    for town in towns(&data) {
        for seed in [1, 2, 3] {
            let mut sim = siege(&data, town.as_deref(), seed);
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
    for town in towns(&data) {
        let sim = siege(&data, town.as_deref(), 1);
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

/// Plan of each town as SVG in `$BR3_SVG_DIR` (layout review).
#[test]
#[ignore = "debug: writes the town plans as SVG"]
fn dump_town_plans() {
    let data = data();
    let dir = std::env::var("BR3_SVG_DIR").unwrap_or_else(|_| ".".into());
    for town in towns(&data) {
        let sim = siege(&data, town.as_deref(), 1);
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
