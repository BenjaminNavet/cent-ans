//! CV3-2 (spec campagne vivante § 1.3): a battle opening in ambush, a side
//! in forced march, an entrenched camp.

mod common;

use common::*;
use sim_battle::opening::{column_path, OpeningRules};
use sim_battle::{
    BattleOpening, BattleReplay, BattleSetup, BattleSim, CommandError, Formation, ObstacleKind,
    ReplayAction, ReplayPlayer, ReplayRecorder, ReplayStart, SideId,
};

const SEED: u64 = 4242;

/// Victim (defender) regiments, by setup index: 0 foot, 1 knights,
/// 2 longbowmen, 3 hobelars, 4 foot, 5 foot.
const VICTIM: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_knights",
    "unit_longbowmen",
    "unit_hobelars",
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
];

fn ambush_setup() -> BattleSetup {
    let data = data();
    let mut setup = setup(
        units(
            data,
            &[
                "unit_men_at_arms_foot",
                "unit_longbowmen",
                "unit_knights",
                "unit_men_at_arms_foot",
            ],
        ),
        units(data, &VICTIM),
        None,
    );
    setup.opening = BattleOpening::Ambush {
        victim: SideId::Defender,
    };
    setup
}

fn victims(sim: &BattleSim) -> Vec<&sim_battle::Unit> {
    sim.units()
        .iter()
        .filter(|u| u.side == SideId::Defender && !u.reserve)
        .collect()
}

/// Arc length of the point of `path` nearest to `p`.
fn arc_of(path: &[(f64, f64)], p: (f64, f64)) -> f64 {
    let mut best = (f64::MAX, 0.0);
    let mut start = 0.0;
    for w in path.windows(2) {
        let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let len = dx.hypot(dz);
        let t = (((p.0 - w[0].0) * dx + (p.1 - w[0].1) * dz) / (len * len)).clamp(0.0, 1.0);
        let q = (w[0].0 + dx * t, w[0].1 + dz * t);
        let d = (q.0 - p.0).hypot(q.1 - p.1);
        if d < best.0 {
            best = (d, start + t * len);
        }
        start += len;
    }
    best.1
}

fn distance_to(path: &[(f64, f64)], p: (f64, f64)) -> f64 {
    path.windows(2)
        .map(|w| {
            let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let len2 = (dx * dx + dz * dz).max(1e-9);
            let t = (((p.0 - w[0].0) * dx + (p.1 - w[0].1) * dz) / len2).clamp(0.0, 1.0);
            (w[0].0 + dx * t - p.0).hypot(w[0].1 + dz * t - p.1)
        })
        .fold(f64::MAX, f64::min)
}

#[test]
fn the_victim_marches_in_column_along_the_road() {
    let sim = BattleSim::new(ambush_setup(), SEED).unwrap();
    assert!(
        !sim.field().roads.is_empty(),
        "a field without river has a road"
    );
    let layout = sim.ambush_layout().expect("ambush layout");
    assert_eq!(layout.victim, SideId::Defender);
    assert!(layout.on_road, "the column follows the road");
    let column = victims(&sim);
    assert_eq!(column.len(), VICTIM.len());
    for unit in &column {
        assert_eq!(
            unit.formation,
            Formation::of("column"),
            "{} in column",
            unit.name
        );
        let d = distance_to(&layout.path, (unit.x, unit.z));
        assert!(d < 1.0, "{} stands on the road ({d:.1} m off)", unit.name);
    }
    // Marching order, head first: hobelars, vanguard foot, knights, main
    // foot, longbowmen, rearguard foot.
    let expected = [3, 0, 1, 4, 2, 5];
    let mut by_arc: Vec<(f64, usize)> = column
        .iter()
        .map(|u| (arc_of(&layout.path, (u.x, u.z)), u.setup_index))
        .collect();
    by_arc.sort_by(|a, b| b.0.total_cmp(&a.0));
    let order: Vec<usize> = by_arc.iter().map(|&(_, i)| i).collect();
    assert_eq!(order, expected, "vanguard → main body → rearguard");
    // Regiments do not overlap along the road.
    for pair in by_arc.windows(2) {
        assert!(
            pair[0].0 - pair[1].0 > 5.0,
            "gaps in the column: {by_arc:?}"
        );
    }
}

#[test]
fn without_a_road_the_column_follows_the_long_axis() {
    let sim = BattleSim::new(ambush_setup(), SEED).unwrap();
    let mut field = sim.field().clone();
    field.roads.clear();
    let rules = OpeningRules::bundled();
    let (path, on_road) = column_path(&field, rules);
    assert!(!on_road);
    assert_eq!(path.len(), 2);
    let (a, b) = (path[0], path[1]);
    if field.width >= field.depth {
        assert_eq!(a.1, b.1, "along x through the middle");
        assert!((b.0 - a.0).abs() >= field.width - 2.0 * rules.column.edge_margin_m - 1e-6);
    } else {
        assert_eq!(a.0, b.0, "along z through the middle");
    }
    // With the road the path is the road.
    let (_, on_road) = column_path(sim.field(), rules);
    assert!(on_road);
}

#[test]
fn the_ambusher_deploys_on_the_flanks_and_the_victim_not_at_all() {
    let mut setup = ambush_setup();
    setup.player_side = Some(SideId::Attacker);
    let mut sim = BattleSim::new(setup, SEED).unwrap();
    let layout = sim.ambush_layout().unwrap().clone();
    assert!(!sim.can_deploy(SideId::Defender));
    assert!(sim.can_deploy(SideId::Attacker));
    assert!(sim.deployment_zones(SideId::Defender).is_empty());
    let zones = sim.deployment_zones(SideId::Attacker);
    assert!((1..=2).contains(&zones.len()), "{zones:?}");
    assert_eq!(zones, layout.zones);
    let near = OpeningRules::bundled().flank.near_m;
    for unit in victims(&sim) {
        for zone in &zones {
            assert!(
                !zone.contains(unit.x, unit.z),
                "the column is out of the zone"
            );
            // Every point of the zone keeps `near` from the column axis.
            let across = if layout.along_z {
                (zone.x0 - unit.x).abs().min((zone.x1 - unit.x).abs())
            } else {
                (zone.z0 - unit.z).abs().min((zone.z1 - unit.z).abs())
            };
            assert!(
                across >= near - 1.0,
                "zone {zone:?} too close to {}",
                unit.name
            );
        }
    }
    for unit in sim.units().iter().filter(|u| u.side == SideId::Attacker) {
        assert!(
            zones.iter().any(|z| z.contains(unit.x, unit.z)),
            "{} starts in a flank zone",
            unit.name
        );
    }
    assert!(sim.begin_deployment(), "the ambusher keeps its deployment");
    let ambusher = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Attacker)
        .unwrap()
        .clone();
    let zone = zones[0];
    let (cx, cz) = ((zone.x0 + zone.x1) * 0.5, (zone.z0 + zone.z1) * 0.5);
    assert_eq!(sim.deploy_unit(ambusher.id, cx, cz, None), Ok(()));
    let target = victims(&sim)[0].clone();
    assert_eq!(
        sim.deploy_unit(ambusher.id, target.x, target.z, None),
        Err(CommandError::OutsideZone(ambusher.id)),
        "not onto the road"
    );
    assert!(
        sim.events()
            .iter()
            .any(|e| e.text_fr.starts_with("Embuscade !")),
        "journal: {:?}",
        sim.events()
    );
}

#[test]
fn a_player_caught_in_column_gets_no_deployment_phase() {
    let mut setup = ambush_setup();
    setup.player_side = Some(SideId::Defender);
    let mut sim = BattleSim::new(setup, SEED).unwrap();
    let before: Vec<(f64, f64)> = sim.units().iter().map(|u| (u.x, u.z)).collect();
    assert!(!sim.begin_deployment());
    assert!(!sim.is_deploying());
    let id = victims(&sim)[0].id;
    assert_eq!(
        sim.deploy_unit(id, 100.0, 100.0, None),
        Err(CommandError::NotDeploying)
    );
    let after: Vec<(f64, f64)> = sim.units().iter().map(|u| (u.x, u.z)).collect();
    assert_eq!(before, after, "nobody moved");
}

#[test]
fn the_ai_ambusher_keeps_its_flanks_and_the_column_its_road() {
    let mut sim = BattleSim::new(ambush_setup(), SEED).unwrap();
    let before: Vec<(f64, f64)> = sim.units().iter().map(|u| (u.x, u.z)).collect();
    assert!(sim.begin_deployment());
    let after: Vec<(f64, f64)> = sim.units().iter().map(|u| (u.x, u.z)).collect();
    assert_eq!(before, after, "no AI redeployment in an ambush");
}

#[test]
fn forced_march_starts_tired_without_deployment() {
    let data = data();
    let mut setup = setup(
        units(data, &["unit_men_at_arms_foot", "unit_longbowmen"]),
        units(data, &["unit_men_at_arms_foot", "unit_knights"]),
        None,
    );
    setup.defender.forced_march = true;
    setup.defender.start_fatigue = 60.0;
    setup.player_side = Some(SideId::Defender);
    let mut sim = BattleSim::new(setup, SEED).unwrap();
    for unit in sim.units() {
        let expected = if unit.side == SideId::Defender {
            60.0
        } else {
            0.0
        };
        assert_eq!(unit.fatigue, expected, "{}", unit.name);
    }
    assert!(!sim.can_deploy(SideId::Defender));
    assert!(sim.can_deploy(SideId::Attacker));
    assert!(
        !sim.begin_deployment(),
        "no deployment phase for the player"
    );
    assert!(sim.ambush_layout().is_none());
    assert!(sim
        .events()
        .iter()
        .any(|e| e.text_fr.starts_with("Marche forcée")));
}

fn entrenched_setup() -> BattleSetup {
    let data = data();
    let mut setup = setup(
        units(data, &["unit_men_at_arms_foot", "unit_men_at_arms_foot"]),
        units(
            data,
            &[
                "unit_men_at_arms_foot",
                "unit_longbowmen",
                "unit_longbowmen",
            ],
        ),
        None,
    );
    setup.defender.entrenched = true;
    setup
}

#[test]
fn an_entrenched_camp_has_its_stakes_and_palisade() {
    let sim = BattleSim::new(entrenched_setup(), SEED).unwrap();
    let archers: Vec<_> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender && u.unit_type == "unit_longbowmen")
        .collect();
    assert_eq!(archers.len(), 2);
    assert!(archers.iter().all(|u| u.stakes_planted), "stakes ready");
    let palisades = sim.palisades();
    assert!(!palisades.is_empty(), "a palisade stands");
    // In front of the defender (who faces -z), behind nobody of his.
    let front = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender)
        .map(|u| u.z)
        .fold(f64::MAX, f64::min);
    let attackers = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker)
        .map(|u| u.z)
        .fold(f64::MIN, f64::max);
    for p in &palisades {
        assert_eq!(p.kind, ObstacleKind::Palisade);
        assert!(p.a.1 < front && p.a.1 > attackers, "between the armies");
    }
    // The palisade slows whoever crosses it.
    let (mx, mz) = (
        (palisades[0].a.0 + palisades[0].b.0) * 0.5,
        palisades[0].a.1,
    );
    let open = sim
        .field()
        .site_speed_factor(mx, mz - 30.0, false, sim.weather());
    let at = sim.field().site_speed_factor(mx, mz, false, sim.weather());
    assert!(at < open, "slower at the palisade ({at} vs {open})");
    assert!(sim
        .events()
        .iter()
        .any(|e| e.text_fr.starts_with("Camp retranché")));
    // No stakes nor palisade without the stance.
    let mut plain = entrenched_setup();
    plain.defender.entrenched = false;
    let sim = BattleSim::new(plain, SEED).unwrap();
    assert!(sim.palisades().is_empty());
    assert!(sim.units().iter().all(|u| !u.stakes_planted));
}

/// Defender losses after `seconds` of melee across the palisade.
fn melee_losses(with_palisade: bool) -> f64 {
    let mut sim = BattleSim::new(entrenched_setup(), SEED).unwrap();
    lab(&mut sim);
    let palisade = sim.palisades()[0];
    if !with_palisade {
        sim.field_mut()
            .obstacles
            .retain(|o| o.kind != ObstacleKind::Palisade);
    }
    let (mx, mz) = ((palisade.a.0 + palisade.b.0) * 0.5, palisade.a.1);
    let defender = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender && u.unit_type == "unit_men_at_arms_foot")
        .unwrap()
        .clone();
    let attacker = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Attacker)
        .unwrap()
        .clone();
    let (_, dd) = defender.extent();
    let (_, ad) = attacker.extent();
    // Everybody else far away.
    for u in sim.units_mut() {
        u.x = 20.0;
        u.z = if u.side == SideId::Attacker {
            20.0
        } else {
            60.0
        };
    }
    place(
        &mut sim,
        defender.id,
        mx,
        mz + dd * 0.5 + 1.0,
        std::f64::consts::PI,
    );
    place(&mut sim, attacker.id, mx, mz - ad * 0.5 - 1.0, 0.0);
    run(&mut sim, 20.0);
    let d = sim.unit(defender.id).unwrap();
    defender.hp - d.hp
}

#[test]
fn the_palisade_shelters_its_defenders_in_melee() {
    let with = melee_losses(true);
    let without = melee_losses(false);
    assert!(without > 0.0, "the melee took place");
    assert!(
        with < without,
        "fewer losses behind the palisade ({with} vs {without})"
    );
}

#[test]
fn the_opening_is_deterministic() {
    let snapshot = |sim: &BattleSim| -> Vec<(u64, u64, u64, Formation)> {
        sim.units()
            .iter()
            .map(|u| {
                (
                    u.x.to_bits(),
                    u.z.to_bits(),
                    u.facing.to_bits(),
                    u.formation,
                )
            })
            .collect()
    };
    let a = BattleSim::new(ambush_setup(), SEED).unwrap();
    let b = BattleSim::new(ambush_setup(), SEED).unwrap();
    assert_eq!(snapshot(&a), snapshot(&b));
    assert_eq!(a.ambush_layout(), b.ambush_layout());
    let c = BattleSim::new(entrenched_setup(), SEED).unwrap();
    let d = BattleSim::new(entrenched_setup(), SEED).unwrap();
    assert_eq!(snapshot(&c), snapshot(&d));
    assert_eq!(c.palisades(), d.palisades());
}

#[test]
fn an_old_setup_without_the_fields_opens_as_before() {
    let text = include_str!("fixtures/demo_battle_1337.json");
    assert!(!text.contains("opening") && !text.contains("forced_march"));
    let setup: BattleSetup = serde_json::from_str(text).unwrap();
    assert_eq!(setup.opening, BattleOpening::Standard);
    for side in SideId::BOTH {
        let s = setup.side(side);
        assert!(!s.forced_march && !s.entrenched && s.start_fatigue == 0.0);
    }
    // Written back, the setup has none of the new keys.
    let written = serde_json::to_string(&setup).unwrap();
    assert!(!written.contains("opening") && !written.contains("entrenched"));
    let mut sim = BattleSim::new(setup, 1337).unwrap();
    assert!(sim.ambush_layout().is_none());
    assert!(sim.palisades().is_empty());
    assert!(sim
        .units()
        .iter()
        .all(|u| u.formation == Formation::of("line") && u.fatigue == 0.0 && !u.stakes_planted));
    for side in SideId::BOTH {
        assert!(sim.can_deploy(side));
        assert_eq!(sim.deployment_zones(side), vec![sim.deployment_zone(side)]);
    }
    assert!(!sim.events().iter().any(|e| e.text_fr.contains("Embuscade")));
    assert!(sim.begin_deployment());
}

#[test]
fn an_ambush_replays_to_the_same_battle() {
    let mut setup = ambush_setup();
    setup.player_side = Some(SideId::Attacker);
    let start = ReplayStart::plain(setup, SEED);
    let mut sim = start.build().unwrap();
    let mut recorder = ReplayRecorder::new(start, &sim);
    let drive = |sim: &mut BattleSim, recorder: &mut ReplayRecorder, action: ReplayAction| {
        recorder.record(sim, action.clone());
        action.apply(sim);
    };
    drive(&mut sim, &mut recorder, ReplayAction::BeginDeployment);
    drive(&mut sim, &mut recorder, ReplayAction::StartBattle);
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::SetAi {
            side: SideId::Attacker,
            enabled: true,
        },
    );
    while !sim.is_finished() && sim.elapsed() < 600.0 {
        sim.tick(0.8);
        recorder.observe(&sim);
    }
    let replay = recorder.finish("Embuscade", 1_790_000_000);
    let read = BattleReplay::from_json(&replay.to_json()).unwrap();
    assert_eq!(read, replay);
    let (mut player, mut replayed) = ReplayPlayer::load(read).unwrap();
    assert!(replayed.ambush_layout().is_some());
    while player.advance(&mut replayed, 1.0) > 0 {}
    assert!(player.divergence().is_none(), "{:?}", player.divergence());
    assert_eq!(replayed.events(), sim.events(), "same journal");
}
