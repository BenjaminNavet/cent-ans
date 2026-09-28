//! CB6: group formations (attack / defence presets, proposed placement).
//!
//! `data/rules/group_formations.json` holds six presets; the pure
//! `BattleSim::formation_slots` turns one into a place per regiment. The
//! initial deployment and the AI deployment go through « Ligne de
//! bataille » with the very placement they had before CB6 (golden file
//! `fixtures/cb6_deploy_golden.json`, written by the ignored
//! `write_deploy_golden` test on the code before CB6).

mod common;

use std::path::PathBuf;

use common::*;
use sim_battle::{BattleSetup, BattleSim, HistoricalMap, SideId};

// ----- scenarios of the non-regression golden --------------------------

fn demo_setup() -> BattleSetup {
    serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap()
}

fn mixed_setup() -> BattleSetup {
    let data = data();
    let french = [
        "unit_knights",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_crossbowmen",
        "unit_crossbowmen",
        "unit_knights",
    ];
    let english = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_knights",
    ];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    battle
}

/// A big army whose rows wrap (wider than the field).
fn big_setup() -> BattleSetup {
    let data = data();
    let mut french: Vec<&str> = vec!["unit_men_at_arms_foot"; 18];
    french.extend(["unit_crossbowmen"; 10]);
    french.extend(["unit_knights"; 9]);
    french.extend(["unit_trebuchet"; 3]);
    let english = ["unit_longbowmen", "unit_men_at_arms_foot", "unit_knights"];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    battle
}

/// No infantry: the shooters take the line.
fn shooters_setup() -> BattleSetup {
    let data = data();
    let a = ["unit_crossbowmen", "unit_knights", "unit_longbowmen"];
    let d = [
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_knights",
        "unit_knights",
    ];
    let mut battle = setup(units(&data, &a), units(&data, &d), None);
    battle.village = Some(false);
    battle
}

fn historical(id: &str, seed: u64) -> BattleSim {
    let data = data();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    let map = HistoricalMap::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    let setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .unwrap();
    map.start(setup, seed).unwrap()
}

/// Every golden scenario: its name and the battle just after set-up (and,
/// for the `*_deploy` ones, after the AI deployment of the other side).
fn golden_scenarios() -> Vec<(String, BattleSim)> {
    let mut out = Vec::new();
    for (name, setup) in [
        ("demo", demo_setup()),
        ("mixed", mixed_setup()),
        ("big", big_setup()),
        ("shooters", shooters_setup()),
    ] {
        out.push((name.to_owned(), BattleSim::new(setup.clone(), 7).unwrap()));
        for player in SideId::BOTH {
            let mut s = setup.clone();
            s.player_side = Some(player);
            let mut sim = BattleSim::new(s, 7).unwrap();
            sim.set_ai(player, false);
            sim.set_ai(player.other(), true);
            sim.begin_deployment();
            out.push((format!("{name}_deploy_{player:?}"), sim));
        }
    }
    for id in ["crecy", "azincourt", "poitiers"] {
        out.push((id.to_owned(), historical(id, 3)));
    }
    out
}

/// Exact positions (bit patterns) of every regiment.
fn positions(sim: &BattleSim) -> Vec<[u64; 3]> {
    sim.units()
        .iter()
        .map(|u| [u.x.to_bits(), u.z.to_bits(), u.facing.to_bits()])
        .collect()
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cb6_deploy_golden.json")
}

/// Writes the golden file (run once on the code before CB6).
#[test]
#[ignore]
fn write_deploy_golden() {
    let map: std::collections::BTreeMap<String, Vec<[u64; 3]>> = golden_scenarios()
        .into_iter()
        .map(|(name, sim)| (name, positions(&sim)))
        .collect();
    std::fs::write(golden_path(), serde_json::to_string_pretty(&map).unwrap()).unwrap();
}

#[test]
fn battle_line_is_the_placement_before_cb6() {
    let text = std::fs::read_to_string(golden_path()).expect("golden file");
    let golden: std::collections::BTreeMap<String, Vec<[u64; 3]>> =
        serde_json::from_str(&text).unwrap();
    let scenarios = golden_scenarios();
    assert_eq!(scenarios.len(), golden.len());
    for (name, sim) in scenarios {
        let now = positions(&sim);
        let before = &golden[&name];
        assert_eq!(now.len(), before.len(), "{name}");
        for (k, (a, b)) in now.iter().zip(before).enumerate() {
            assert_eq!(
                a,
                b,
                "{name}: regiment {k} moved: now ({}, {}), before ({}, {})",
                f64::from_bits(a[0]),
                f64::from_bits(a[1]),
                f64::from_bits(b[0]),
                f64::from_bits(b[1])
            );
        }
    }
}

// ----- presets -----------------------------------------------------------

use sim_battle::{Command, FormationSlot, GroupFormationRules, Role, Unit};

/// 0 knights (the general), 1-3 men-at-arms, 4-5 longbowmen, 6 crossbowmen,
/// 7 knights, 8 mounted archers, 9 trebuchet against militia far away (10).
fn army_setup() -> BattleSetup {
    let data = data();
    let a = [
        "unit_knights",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_crossbowmen",
        "unit_knights",
        "unit_mounted_archers",
        "unit_trebuchet",
    ];
    let mut battle = setup(
        units(&data, &a),
        units(&data, &["unit_urban_militia"]),
        None,
    );
    battle.village = Some(false);
    battle.attacker.general = Some(
        serde_json::from_value(serde_json::json!({
            "character": "chr_test",
            "name": "Test",
            "command": 5,
            "unit_index": 0
        }))
        .unwrap(),
    );
    battle
}

const ARMY: [u32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
const ORIGIN: (f64, f64) = (600.0, 420.0);

fn army_sim() -> BattleSim {
    let mut sim = BattleSim::new(army_setup(), 5).unwrap();
    lab(&mut sim);
    // Scattered in a known left-to-right order (not the id order).
    let xs = [
        520.0, 480.0, 600.0, 540.0, 450.0, 700.0, 640.0, 760.0, 400.0, 580.0,
    ];
    for (id, x) in xs.iter().enumerate() {
        place(&mut sim, id as u32, *x, 250.0, 0.0);
    }
    place(&mut sim, 10, 600.0, 1000.0, std::f64::consts::PI);
    sim
}

fn preset_ids() -> Vec<String> {
    GroupFormationRules::bundled()
        .presets
        .iter()
        .map(|p| p.id.clone())
        .collect()
}

/// Local coordinates of a slot: `s` to the right, `t` towards the enemy.
fn local(slot: &FormationSlot, facing: f64) -> (f64, f64) {
    let right = (facing.cos(), -facing.sin());
    let forward = (facing.sin(), facing.cos());
    let (dx, dz) = (slot.x - ORIGIN.0, slot.z - ORIGIN.1);
    (dx * right.0 + dz * right.1, dx * forward.0 + dz * forward.1)
}

fn role(sim: &BattleSim, id: u32) -> Role {
    let u: &Unit = &sim.units()[id as usize];
    if u.is_general {
        Role::General
    } else {
        Role::of(u)
    }
}

/// `(s, t)` of the slots of `r`.
fn of_role(sim: &BattleSim, slots: &[FormationSlot], facing: f64, r: Role) -> Vec<(f64, f64)> {
    slots
        .iter()
        .filter(|s| role(sim, s.id) == r)
        .map(|s| local(s, facing))
        .collect()
}

fn max_abs_s(v: &[(f64, f64)]) -> f64 {
    v.iter().map(|p| p.0.abs()).fold(0.0, f64::max)
}

#[test]
fn six_presets_one_place_per_regiment_deterministic() {
    let sim = army_sim();
    assert_eq!(preset_ids().len(), 6);
    for facing in [0.0, 0.7, std::f64::consts::PI] {
        for id in preset_ids() {
            let slots = sim.formation_slots(&id, &ARMY, ORIGIN.0, ORIGIN.1, facing);
            let mut got: Vec<u32> = slots.iter().map(|s| s.id).collect();
            got.sort_unstable();
            assert_eq!(got, ARMY.to_vec(), "{id}");
            for s in &slots {
                assert!(s.x.is_finite() && s.z.is_finite() && s.facing.is_finite());
                assert!(sim.field().inside(s.x, s.z), "{id}: ({}, {})", s.x, s.z);
            }
            let again = sim.formation_slots(&id, &ARMY, ORIGIN.0, ORIGIN.1, facing);
            assert_eq!(slots, again, "{id} is deterministic");
            // Same answer from a fresh battle with the same seed.
            let other = army_sim().formation_slots(&id, &ARMY, ORIGIN.0, ORIGIN.1, facing);
            assert_eq!(slots, other, "{id}");
        }
    }
    assert!(sim
        .formation_slots("nope", &ARMY, 600.0, 420.0, 0.0)
        .is_empty());
    // Duplicates and unknown ids are left out.
    let slots = sim.formation_slots("battle_line", &[1, 1, 99, 2], 600.0, 420.0, 0.0);
    assert_eq!(slots.len(), 2);
}

#[test]
fn roles_take_their_places() {
    let sim = army_sim();
    for facing in [0.0, 1.1] {
        let get = |id: &str| sim.formation_slots(id, &ARMY, ORIGIN.0, ORIGIN.1, facing);
        let tol = 1e-6;

        // Ligne de bataille: foot on the line, shooters behind, horse on the wings.
        let slots = get("battle_line");
        let foot = of_role(&sim, &slots, facing, Role::Infantry);
        let shooters = of_role(&sim, &slots, facing, Role::FootRanged);
        let horse = of_role(&sim, &slots, facing, Role::Cavalry);
        let engines = of_role(&sim, &slots, facing, Role::Siege);
        let general = of_role(&sim, &slots, facing, Role::General);
        assert!(foot.iter().all(|p| p.1.abs() < tol));
        assert!(shooters.iter().all(|p| p.1 < -40.0));
        assert!(horse.iter().all(|p| p.0.abs() > max_abs_s(&foot)));
        assert!(horse.iter().any(|p| p.0 > 0.0) && horse.iter().any(|p| p.0 < 0.0));
        assert!(engines.iter().all(|p| p.1 < -90.0));
        assert_eq!(general.len(), 1);
        assert!(general[0].1 < 0.0 && general[0].0.abs() < 1.0);

        // La herse: archers ahead on both wings, turned inwards; horse in reserve.
        let slots = get("harrow");
        let foot = of_role(&sim, &slots, facing, Role::Infantry);
        let shooters = of_role(&sim, &slots, facing, Role::FootRanged);
        let horse = of_role(&sim, &slots, facing, Role::Cavalry);
        assert!(foot.iter().all(|p| p.1.abs() < tol));
        assert!(shooters.iter().all(|p| p.1 > 20.0));
        assert!(shooters.iter().all(|p| p.0.abs() > max_abs_s(&foot)));
        assert!(shooters.iter().any(|p| p.0 > 0.0) && shooters.iter().any(|p| p.0 < 0.0));
        for s in slots
            .iter()
            .filter(|s| role(&sim, s.id) == Role::FootRanged)
        {
            let (ls, _) = local(s, facing);
            // Turned towards the centre: the facing's lateral component points inwards.
            let dir = (s.facing.sin(), s.facing.cos());
            let right = (facing.cos(), -facing.sin());
            let lateral = dir.0 * right.0 + dir.1 * right.1;
            assert!(lateral * ls < 0.0, "archer at s {ls:.0} faces outwards");
        }
        assert!(horse.iter().all(|p| p.1 < -80.0));

        // Trois batailles: the foot in three successive lines.
        let slots = get("three_battles");
        let foot = of_role(&sim, &slots, facing, Role::Infantry);
        let mut lines: Vec<i64> = foot.iter().map(|p| p.1.round() as i64).collect();
        lines.sort_unstable();
        lines.dedup();
        assert_eq!(lines, vec![-200, -100, 0]);
        let shooters = of_role(&sim, &slots, facing, Role::FootRanged);
        assert!(shooters.iter().all(|p| p.1 > 30.0));
        let general = of_role(&sim, &slots, facing, Role::General);
        assert!(general[0].1 < -100.0 && general[0].1 > -200.0);

        // Charge de la chevalerie: horse first, foot following, shooters ahead on the wings.
        let slots = get("knight_charge");
        let foot = of_role(&sim, &slots, facing, Role::Infantry);
        let horse = of_role(&sim, &slots, facing, Role::Cavalry);
        let shooters = of_role(&sim, &slots, facing, Role::FootRanged);
        let last_horse = horse.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        assert!(horse.iter().any(|p| p.1.abs() < tol));
        assert!(foot.iter().all(|p| p.1 < last_horse - 30.0));
        assert!(shooters.iter().all(|p| p.1 > 0.0));

        // Bataille à pied: the mounted reserve apart on one flank, well back.
        let slots = get("foot_battle");
        let foot = of_role(&sim, &slots, facing, Role::Infantry);
        let horse = of_role(&sim, &slots, facing, Role::Cavalry);
        let right_edge = foot.iter().map(|p| p.0).fold(f64::MIN, f64::max);
        assert!(horse
            .iter()
            .all(|p| p.0 > right_edge + 40.0 && p.1 < -100.0));

        // Ordre de marche: one file, scouts first, heavy horse last.
        let slots = get("march");
        let mut column: Vec<(f64, u32)> = slots
            .iter()
            .map(|s| {
                let (ls, lt) = local(s, facing);
                assert!(ls.abs() < 1e-6, "off the file");
                (lt, s.id)
            })
            .collect();
        column.sort_by(|a, b| b.0.total_cmp(&a.0));
        let order: Vec<u32> = column.iter().map(|c| c.1).collect();
        assert_eq!(order[0], 8, "mounted archers lead");
        assert_eq!(*order.last().unwrap(), 7, "the knights close the column");
        let pos = |id: u32| order.iter().position(|&o| o == id).unwrap();
        assert!(
            pos(0) > pos(4) && pos(0) < pos(9),
            "the general before the engines"
        );
        for w in column.windows(2) {
            assert!(w[0].0 - w[1].0 > 10.0, "regiments overlap in the column");
        }
        assert!(slots
            .iter()
            .filter(|s| role(&sim, s.id) != Role::Siege)
            .all(|s| s.width.is_some()));
    }
}

#[test]
fn left_to_right_order_is_kept() {
    let sim = army_sim();
    // Infantry stands at 480 (id 1), 540 (id 3), 600 (id 2).
    for id in ["battle_line", "harrow", "foot_battle"] {
        let slots = sim.formation_slots(id, &ARMY, ORIGIN.0, ORIGIN.1, 0.0);
        let mut foot: Vec<(f64, u32)> = slots
            .iter()
            .filter(|s| role(&sim, s.id) == Role::Infantry)
            .map(|s| (s.x, s.id))
            .collect();
        foot.sort_by(|a, b| a.0.total_cmp(&b.0));
        let order: Vec<u32> = foot.iter().map(|f| f.1).collect();
        assert_eq!(order, vec![1, 3, 2], "{id}");
        // Shooters keep their order too: 4 (450), 6 (640), 5 (700).
        let mut shooters: Vec<(f64, u32)> = slots
            .iter()
            .filter(|s| role(&sim, s.id) == Role::FootRanged)
            .map(|s| (s.x, s.id))
            .collect();
        shooters.sort_by(|a, b| a.0.total_cmp(&b.0));
        let order: Vec<u32> = shooters.iter().map(|f| f.1).collect();
        assert_eq!(order, vec![4, 6, 5], "{id}");
    }
    // Facing the other way the order is read from the regiments' own right,
    // so the world order is the same.
    let slots = sim.formation_slots(
        "battle_line",
        &ARMY,
        ORIGIN.0,
        ORIGIN.1,
        std::f64::consts::PI,
    );
    let mut foot: Vec<(f64, u32)> = slots
        .iter()
        .filter(|s| role(&sim, s.id) == Role::Infantry)
        .map(|s| (s.x, s.id))
        .collect();
    foot.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(foot.iter().map(|f| f.1).collect::<Vec<_>>(), vec![1, 3, 2]);
}

#[test]
fn deployment_places_stay_in_the_zone_and_can_be_taken() {
    for id in preset_ids() {
        let mut setup = army_setup();
        setup.player_side = Some(SideId::Attacker);
        let mut sim = BattleSim::new(setup, 5).unwrap();
        sim.set_ai(SideId::Attacker, false);
        assert!(sim.begin_deployment());
        let zone = sim.deployment_zone(SideId::Attacker);
        let cx = (zone.x0 + zone.x1) * 0.5;
        // Front a little behind the zone's front edge; the deep presets overflow the back.
        let slots = sim.formation_slots(&id, &ARMY, cx, zone.z1 - 10.0, 0.0);
        assert_eq!(slots.len(), ARMY.len(), "{id}");
        for s in &slots {
            assert!(
                zone.contains(s.x, s.z),
                "{id}: ({:.0}, {:.0}) out of the zone",
                s.x,
                s.z
            );
            sim.deploy_unit_width(s.id, s.x, s.z, Some(s.facing), s.width)
                .unwrap_or_else(|e| panic!("{id}: {e}"));
        }
        // A far-off anchor is brought back into the zone as well.
        for s in sim.formation_slots(&id, &ARMY, cx, 1100.0, 0.0) {
            assert!(zone.contains(s.x, s.z), "{id}");
        }
    }
}

#[test]
fn never_in_deep_water() {
    use sim_battle::Water;
    let data = data();
    let a = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_knights",
        "unit_knights",
        "unit_trebuchet",
    ];
    let mut battle = setup(
        units(&data, &a),
        units(&data, &["unit_urban_militia"]),
        None,
    );
    battle.river = true;
    let ids: Vec<u32> = (0..7).collect();
    let mut checked = 0;
    for seed in 1..6 {
        let mut sim = BattleSim::new(battle.clone(), seed).unwrap();
        lab(&mut sim);
        let Some(river) = sim.field().river.clone() else {
            continue;
        };
        for x in [300.0, 600.0, 900.0] {
            if river.in_ford(x) {
                continue;
            }
            let z = river.center_z(x);
            for id in preset_ids() {
                for facing in [0.0, std::f64::consts::FRAC_PI_2] {
                    for s in sim.formation_slots(&id, &ids, x, z, facing) {
                        assert_ne!(
                            sim.field().water_kind(s.x, s.z),
                            Some(Water::Deep),
                            "seed {seed} {id}: ({:.0}, {:.0}) in deep water",
                            s.x,
                            s.z
                        );
                        checked += 1;
                    }
                }
            }
        }
    }
    assert!(checked > 0);
}

#[test]
fn places_become_individual_group_moves() {
    let mut sim = army_sim();
    let slots = sim.formation_slots("harrow", &ARMY, ORIGIN.0, ORIGIN.1, 0.0);
    for s in &slots {
        sim.apply_command(
            Command::Move {
                units: vec![s.id],
                x: s.x,
                z: s.z,
                run: false,
                facing: Some(s.facing),
                queue: false,
                width: s.width,
                match_speed: true,
                group_tag: Some(77),
            },
            None,
        )
        .unwrap();
    }
    run(&mut sim, 240.0);
    for s in &slots {
        let u = &sim.units()[s.id as usize];
        let d = ((u.x - s.x).powi(2) + (u.z - s.z).powi(2)).sqrt();
        assert!(d < 8.0, "regiment {} {d:.0} m from its place", s.id);
    }
}
