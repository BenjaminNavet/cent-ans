//! R2b: the battle AI reads the relief (crests, reverse slopes, valleys,
//! lines of sight). Win rate of an active AI against a passive side, mirrored
//! armies, on each relief style.
//!
//! `cargo test --release -p sim-battle --test ai_relief -- --ignored --nocapture`
//! prints the survey table (32 battles per terrain).

mod common;

use common::*;
use data_model::{GameData, Terrain};
use sim_battle::relief_ai::ReliefMap;
use sim_battle::{BattleSim, SideId};

const ARMY: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
];

/// Did the active AI beat a passive `passive` side on `terrain`?
fn active_wins(data: &GameData, terrain: Terrain, seed: u64, passive: SideId) -> bool {
    let mut battle = setup(units(data, &ARMY), units(data, &ARMY), None);
    battle.terrain = terrain;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(passive, false);
    run_to_end(&mut sim);
    if std::env::var("R2B_TIMES").is_ok() {
        println!(
            "{} {seed} {} {:.0}",
            terrain.key(),
            passive.other().key(),
            sim.elapsed()
        );
    }
    sim.winner() == Some(passive.other())
}

/// Victories of the active AI over `seeds` x both passive sides, with the
/// lost battles as `seed/active side`.
fn wins(data: &GameData, terrain: Terrain, seeds: std::ops::Range<u64>) -> (usize, Vec<String>) {
    let mut won = 0;
    let mut lost = Vec::new();
    for seed in seeds {
        for passive in SideId::BOTH {
            if active_wins(data, terrain, seed, passive) {
                won += 1;
            } else {
                lost.push(format!("{seed}/{}", passive.other().key()));
            }
        }
    }
    (won, lost)
}

/// R2b: on a fixed sample (seeds 0-15, both sides passive in turn) the
/// active AI beats a passive side at least as often as before R2 (21/32 in
/// the plains). Measured after R2b: plains 29, bocage 22, hills 22,
/// mountains 21 (main after R2: 17, 9, 16, 18; before R2: 21, 11, 14, 14).
#[test]
fn ai_beats_a_passive_side_on_every_relief() {
    let data = data();
    for (terrain, least) in [
        (Terrain::Plains, 25),
        (Terrain::Bocage, 18),
        (Terrain::Hills, 18),
        (Terrain::Mountains, 18),
    ] {
        let (won, lost) = wins(&data, terrain, 0..16);
        assert!(
            won >= least,
            "{}: the active AI won {won}/32 (at least {least} expected), lost {}",
            terrain.key(),
            lost.join(" ")
        );
    }
}

/// A plain field with an east-west ridge (crest at `crest_z`, 20 m high),
/// no wood, mud or river.
fn ridge_sim(data: &GameData, attacker: &[&str], defender: &[&str], crest_z: f64) -> BattleSim {
    let mut battle = setup(units(data, attacker), units(data, defender), None);
    battle.village = Some(false);
    // The ridge is laid for the standard 300 m line gap (ADR 0180 widened the
    // field battles' gap).
    let scale = sim_battle::BattleScale::default();
    let mut sim = BattleSim::new_scaled(battle, 7, scale).unwrap();
    let field = sim.field_mut();
    field.forests.clear();
    field.forest_parts.clear();
    field.mud.clear();
    field.mud_parts.clear();
    field.pools.clear();
    field.river = None;
    let (nx, res) = (field.nx, field.resolution);
    for (k, h) in field.heights.iter_mut().enumerate() {
        let z = (k / nx) as f64 * res;
        *h = 20.0 * (-((z - crest_z) / 45.0).powi(2)).exp();
    }
    sim
}

#[test]
fn the_relief_map_reads_crests_hollows_and_slopes() {
    let data = data();
    let sim = ridge_sim(&data, &ARMY, &ARMY, 400.0);
    let (field, map) = (sim.field(), sim.relief_map());
    assert!(
        map.prominence(600.0, 400.0) > 5.0,
        "crest above its surroundings"
    );
    assert!(
        map.prominence(600.0, 520.0) < 0.0,
        "foot of the ridge below them"
    );
    assert!(ReliefMap::slope(field, 600.0, 400.0) < 0.05, "flat top");
    assert!(ReliefMap::slope(field, 600.0, 440.0) > 0.2, "steep flank");
    // Up the flank: the climb is steep and the march costs more than its
    // length; along the ridge it costs its length.
    assert!(ReliefMap::climb(field, (600.0, 300.0), (600.0, 400.0)) > 0.2);
    assert_eq!(ReliefMap::climb(field, (600.0, 400.0), (600.0, 300.0)), 0.0);
    assert!(ReliefMap::march_cost(field, (600.0, 300.0), (600.0, 400.0)) > 150.0);
    let along = ReliefMap::march_cost(field, (500.0, 400.0), (600.0, 400.0));
    assert!((along - 100.0).abs() < 1.0);
    // Lines of sight over the crest; the reverse slope behind it.
    assert!(!ReliefMap::sees(field, (600.0, 250.0), (600.0, 520.0)));
    assert!(ReliefMap::sees(field, (600.0, 250.0), (600.0, 400.0)));
    let hidden = ReliefMap::reverse_slope(field, (600.0, 400.0), -1.0, 150.0, 60.0)
        .expect("a reverse slope behind the crest");
    assert!(hidden.1 > 400.0 && hidden.1 <= 460.0);
    assert!(!ReliefMap::sees(field, (600.0, 250.0), hidden));
    // A flat field has neither.
    assert!(ReliefMap::reverse_slope(field, (600.0, 700.0), -1.0, 150.0, 60.0).is_none());
}

/// A weaker defender facing crossbows (which must see their target) holds
/// the crest in front of its deployment with its longbows and keeps its
/// foot on the reverse slope, out of sight.
#[test]
fn a_weaker_defender_holds_the_crest_and_hides_its_line_behind_it() {
    let data = data();
    let attacker = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_crossbowmen",
        "unit_crossbowmen",
        "unit_crossbowmen",
    ];
    let defender = ["unit_men_at_arms_foot", "unit_longbowmen"];
    let mut sim = ridge_sim(&data, &attacker, &defender, 525.0);
    sim.set_ai(SideId::Attacker, false);
    run(&mut sim, 120.0);
    let field = sim.field();
    let of = |kind: &str| {
        sim.units()
            .iter()
            .find(|u| u.side == SideId::Defender && u.unit_type == kind)
            .unwrap()
    };
    let (line, bows) = (of("unit_men_at_arms_foot"), of("unit_longbowmen"));
    assert!(
        bows.stakes_planted,
        "the longbows stand and plant their stakes"
    );
    assert!(
        (bows.z - 525.0).abs() < 35.0,
        "longbows on the crest, z {:.0}",
        bows.z
    );
    assert!(line.z > 530.0, "line behind the crest, z {:.0}", line.z);
    let crossbows = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker && u.unit_type == "unit_crossbowmen");
    for c in crossbows {
        assert!(
            !ReliefMap::sees(field, (c.x, c.z), (line.x, line.z)),
            "the crossbows at ({:.0}, {:.0}) cannot see the line",
            c.x,
            c.z
        );
    }
}

/// On a field with a clear height edge, an evenly matched defender keeps its
/// heights instead of walking down to meet the attacker.
#[test]
fn an_even_defender_keeps_its_heights() {
    let data = data();
    let mut sim = ridge_sim(&data, &ARMY, &ARMY, 560.0);
    sim.set_ai(SideId::Attacker, false);
    let start: Vec<f64> = sim.units().iter().map(|u| u.z).collect();
    run(&mut sim, 150.0);
    let field = sim.field();
    for u in sim.units().iter().filter(|u| u.side == SideId::Defender) {
        // The shooters stand on the forward slope in front of the line.
        assert!(
            field.height(u.x, u.z) > 8.0,
            "{} {} left the heights: z {:.0} -> {:.0}",
            u.id,
            u.unit_type,
            start[u.id as usize],
            u.z
        );
        if !u.can_shoot() {
            assert!(
                (u.z - start[u.id as usize]).abs() < 40.0,
                "{} {} moved from z {:.0} to {:.0}",
                u.id,
                u.unit_type,
                start[u.id as usize],
                u.z
            );
        }
    }
}

#[test]
#[ignore = "survey: run in release with --nocapture"]
fn survey_active_against_passive() {
    let data = data();
    for terrain in [
        Terrain::Plains,
        Terrain::Bocage,
        Terrain::Hills,
        Terrain::Mountains,
    ] {
        // R2B_SEEDS=16..48 checks another sample.
        let seeds = std::env::var("R2B_SEEDS")
            .ok()
            .and_then(|s| {
                let (a, b) = s.split_once("..")?;
                Some(a.parse().ok()?..b.parse().ok()?)
            })
            .unwrap_or(0..16);
        let n = 2 * (seeds.end - seeds.start);
        let (won, lost) = wins(&data, terrain, seeds);
        println!("{:<10} {won}/{n}  lost: {}", terrain.key(), lost.join(" "));
    }
}

#[test]
#[ignore = "debug trace: R2B_TRACE=terrain,seed,passive"]
fn trace_one_battle() {
    let data = data();
    let spec = std::env::var("R2B_TRACE").unwrap_or_else(|_| "plains,0,defender".into());
    let parts: Vec<&str> = spec.split(',').collect();
    let terrain = match parts[0] {
        "bocage" => Terrain::Bocage,
        "hills" => Terrain::Hills,
        "mountains" => Terrain::Mountains,
        _ => Terrain::Plains,
    };
    let seed: u64 = parts[1].parse().unwrap();
    let passive = if parts[2] == "attacker" {
        SideId::Attacker
    } else {
        SideId::Defender
    };
    let mut battle = setup(units(&data, &ARMY), units(&data, &ARMY), None);
    battle.terrain = terrain;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(passive, false);
    let mut t = 0.0;
    while !sim.is_finished() {
        if sim.elapsed() >= t {
            println!("--- t={:.0}", sim.elapsed());
            for u in sim.units() {
                if !u.present() {
                    continue;
                }
                println!(
                    "{:>2} {:<3} {:<22} x{:>5.0} z{:>5.0} h{:>5.1} hp{:>4.0} m{:>3.0} {:?} amm{} stk{} tgt{:?} dst{:?}",
                    u.id,
                    &u.side.key()[..3],
                    u.unit_type,
                    u.x,
                    u.z,
                    sim.field().height(u.x, u.z),
                    u.hp,
                    u.morale,
                    u.state,
                    u.ammo,
                    u.stakes_planted as u8, u.target, u.destination.map(|(a, b)| (a as i32, b as i32))
                );
            }
            t += if sim.elapsed() > 380.0 { 5.0 } else { 30.0 };
        }
        sim.step();
    }
    println!("winner {:?} at {:.0}", sim.winner(), sim.elapsed());
    for e in sim.events() {
        println!("{:>5.0} {}", e.time, e.text_fr);
    }
}
