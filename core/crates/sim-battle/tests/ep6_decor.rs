//! EP6: villages and decor of the battlefield — regional composition,
//! placement constraints (roads, water, bridges, field size), terrain
//! effects of the decor, camps and the looting of the baggage, hand
//! placement for the historical maps.

mod common;

use common::*;
use data_model::Terrain;
use sim_battle::town::Footprint;
use sim_battle::{
    AreaKind, BattleRng, BattleScale, BattleSeason, BattleSim, Battlefield, DecorPlan,
    DecorPropKind, FieldSite, FieldSize, HamletLayout, HouseKind, ObstacleKind, SideId, Weather,
};

fn site(terrain: Terrain, season: BattleSeason) -> FieldSite {
    FieldSite {
        terrain,
        river: true,
        coastal: false,
        season,
        village: None,
    }
}

/// A campaign field of `size` with the decor of `province`.
fn decor_field(
    province: &str,
    terrain: Terrain,
    season: BattleSeason,
    size: FieldSize,
    seed: u64,
) -> Battlefield {
    let mut rng = BattleRng::from_seed(seed);
    let mut field =
        Battlefield::generate_site_sized(&site(terrain, season), size, Weather::Clear, &mut rng);
    field.lay_decor(province, &rng);
    field
}

fn epic() -> FieldSize {
    BattleScale::named("epic").unwrap().field
}

/// Every solid or visible footprint of the decor (buildings, props, camp
/// items and baggage).
fn footprints(field: &Battlefield) -> Vec<(String, Footprint)> {
    let d = &field.decor;
    let mut out: Vec<(String, Footprint)> = d
        .buildings
        .iter()
        .map(|b| {
            (
                b.kind.key().to_owned(),
                Footprint::new(b.x, b.z, b.length, b.width, b.yaw),
            )
        })
        .collect();
    out.extend(
        d.props
            .iter()
            .map(|p| (p.kind.key().to_owned(), p.footprint())),
    );
    for c in &d.camps {
        out.extend(
            c.items
                .iter()
                .map(|p| (p.kind.key().to_owned(), p.footprint())),
        );
        out.extend(
            c.convoy
                .iter()
                .map(|p| ("convoy".to_owned(), p.footprint())),
        );
    }
    out
}

#[test]
fn decor_is_deterministic_and_follows_the_field_size() {
    let a = decor_field(
        "prov_guyenne",
        Terrain::Plains,
        BattleSeason::Summer,
        FieldSize::STANDARD,
        3,
    );
    let b = decor_field(
        "prov_guyenne",
        Terrain::Plains,
        BattleSeason::Summer,
        FieldSize::STANDARD,
        3,
    );
    assert_eq!(a.decor, b.decor);
    assert_eq!(a.decor.profile, "vignoble");
    let (mut small, mut large) = (0, 0);
    for seed in 0..12 {
        for (size, total) in [(FieldSize::STANDARD, &mut small), (epic(), &mut large)] {
            let f = decor_field(
                "prov_picardie",
                Terrain::Plains,
                BattleSeason::Summer,
                size,
                seed,
            );
            assert_eq!(f.decor.camps.len(), 2, "seed {seed}: camps");
            *total += f.decor.areas.len() + f.decor.buildings.len();
            for (kind, fp) in footprints(&f) {
                for (x, z) in fp.corners() {
                    assert!(
                        f.inside(x, z),
                        "seed {seed} {}×{}: {kind} at ({x:.0}, {z:.0}) outside",
                        f.width,
                        f.depth
                    );
                }
            }
            for c in &f.decor.camps {
                // Behind its own battle line.
                match c.side {
                    SideId::Attacker => assert!(c.area.z < f.attacker_line_z() - 60.0),
                    SideId::Defender => assert!(c.area.z > f.defender_line_z() + 60.0),
                }
                assert!(
                    c.items.len() >= 10,
                    "seed {seed}: {} camp items",
                    c.items.len()
                );
            }
        }
    }
    // The epic field is 4 times the standard one.
    assert!(
        large > small * 2,
        "decor {small} (standard) vs {large} (epic)"
    );
}

#[test]
fn decor_depends_on_the_region_and_the_season() {
    let count = |province: &str, terrain: Terrain, season: BattleSeason, kind: AreaKind| {
        (0..10)
            .map(|seed| {
                decor_field(province, terrain, season, FieldSize::STANDARD, seed)
                    .decor
                    .areas
                    .iter()
                    .filter(|a| a.kind == kind)
                    .count()
            })
            .sum::<usize>()
    };
    // Vineyards in Guyenne, Burgundy and Champagne, none in Picardy.
    for province in ["prov_guyenne", "prov_bourgogne", "prov_champagne"] {
        let v = count(
            province,
            Terrain::Plains,
            BattleSeason::Summer,
            AreaKind::Vineyard,
        );
        assert!(v >= 20, "{province}: {v} vineyards over 10 fields");
    }
    assert_eq!(
        count(
            "prov_picardie",
            Terrain::Plains,
            BattleSeason::Summer,
            AreaKind::Vineyard
        ),
        0
    );
    // Bocage of Brittany: hedges round the orchards and meadows, even on a
    // field that is not bocage terrain.
    let hedges = |province: &str| {
        (0..6)
            .map(|seed| {
                decor_field(
                    province,
                    Terrain::Plains,
                    BattleSeason::Summer,
                    FieldSize::STANDARD,
                    seed,
                )
                .obstacles
                .iter()
                .filter(|o| o.kind == ObstacleKind::Hedge)
                .count()
            })
            .sum::<usize>()
    };
    assert!(hedges("prov_bretagne") > hedges("prov_picardie") + 10);
    // Haystacks in summer, none in spring; leafy vines except in winter.
    let stacks = |season| {
        (0..10)
            .map(|seed| {
                let f = decor_field(
                    "prov_normandie_ouest",
                    Terrain::Bocage,
                    season,
                    FieldSize::STANDARD,
                    seed,
                );
                f.decor
                    .props
                    .iter()
                    .filter(|p| p.kind == DecorPropKind::Haystack)
                    .count()
            })
            .sum::<usize>()
    };
    assert!(stacks(BattleSeason::Summer) > stacks(BattleSeason::Spring) + 10);
    let winter = decor_field(
        "prov_guyenne",
        Terrain::Plains,
        BattleSeason::Winter,
        FieldSize::STANDARD,
        1,
    );
    assert!(!winter.decor.vines_leafy);
    let spring = decor_field(
        "prov_guyenne",
        Terrain::Plains,
        BattleSeason::Spring,
        FieldSize::STANDARD,
        1,
    );
    assert!(spring.decor.vines_leafy && spring.decor.orchard_blossom);
    // Ripe corn only in summer.
    let crop = |season| {
        (0..6)
            .flat_map(|seed| {
                decor_field(
                    "prov_picardie",
                    Terrain::Plains,
                    season,
                    FieldSize::STANDARD,
                    seed,
                )
                .decor
                .areas
            })
            .filter(|a| a.state == Some(sim_battle::FieldState::Crop))
            .count()
    };
    assert!(crop(BattleSeason::Summer) > 5);
    assert_eq!(crop(BattleSeason::Winter), 0);
}

#[test]
fn nothing_stands_in_water_on_bridges_fords_or_roads() {
    let mut mills = 0;
    let mut windmills = 0;
    for seed in 0..24 {
        let (province, terrain) = [
            ("prov_guyenne", Terrain::Plains),
            ("prov_bretagne", Terrain::Bocage),
            ("prov_kent", Terrain::Hills),
            ("prov_holland", Terrain::Marsh),
        ][seed as usize % 4];
        let size = if seed % 3 == 0 {
            epic()
        } else {
            FieldSize::STANDARD
        };
        let f = decor_field(province, terrain, BattleSeason::Summer, size, seed);
        mills += f
            .decor
            .buildings
            .iter()
            .filter(|b| b.kind == HouseKind::Watermill)
            .count();
        windmills += f
            .decor
            .buildings
            .iter()
            .filter(|b| b.kind == HouseKind::Windmill)
            .count();
        for (kind, fp) in footprints(&f) {
            let water_ok = kind == "watermill";
            // No part in deep water (a water mill's wheel may overhang it,
            // not the mill house).
            let step = 2.0;
            let nu = (fp.half_length / step).ceil() as i32;
            let nv = (fp.half_depth / step).ceil() as i32;
            for i in -nu..=nu {
                for j in -nv..=nv {
                    let (x, z) = fp.world(
                        f64::from(i) * fp.half_length / f64::from(nu.max(1)),
                        f64::from(j) * fp.half_depth / f64::from(nv.max(1)),
                    );
                    if !water_ok {
                        assert!(
                            f.water_kind(x, z).is_none(),
                            "seed {seed}: {kind} in the water at ({x:.0}, {z:.0})"
                        );
                    }
                    assert!(
                        f.bridge_at(x, z).is_none(),
                        "seed {seed}: {kind} on a bridge"
                    );
                }
            }
            for b in &f.bridges {
                assert!(
                    fp.signed_distance(b.x, b.z) > b.length * 0.5,
                    "seed {seed}: {kind} at a bridge"
                );
            }
            if let Some(r) = &f.river {
                for ford in &r.fords {
                    assert!(
                        fp.signed_distance(ford.x, r.center_z(ford.x)) > ford.half_width,
                        "seed {seed}: {kind} in a ford"
                    );
                }
            }
            for road in &f.roads {
                for w in road.points.windows(2) {
                    assert!(
                        fp.distance_to_segment(w[0], w[1]) > road.width * 0.5,
                        "seed {seed}: {kind} on a {} road at ({:.0}, {:.0})",
                        road.kind.key(),
                        fp.x,
                        fp.z
                    );
                }
            }
        }
        // Plots stay off the roads too.
        for a in &f.decor.areas {
            if !matches!(
                a.kind,
                AreaKind::Vineyard | AreaKind::Orchard | AreaKind::Meadow
            ) {
                continue;
            }
            let fp = a.footprint();
            for road in &f.roads {
                for w in road.points.windows(2) {
                    assert!(
                        fp.distance_to_segment(w[0], w[1]) > road.width * 0.5,
                        "seed {seed}: {} on a road",
                        a.kind.key()
                    );
                }
            }
        }
    }
    assert!(mills >= 6, "{mills} water mills over 24 fields");
    assert!(windmills >= 6, "{windmills} windmills over 24 fields");
}

#[test]
fn roads_run_through_the_street_hamlets() {
    let mut street = 0;
    let mut green = 0;
    for seed in 0..20 {
        let f = decor_field(
            "prov_toulousain",
            Terrain::Plains,
            BattleSeason::Summer,
            FieldSize::STANDARD,
            seed,
        );
        for h in &f.decor.hamlets {
            // Distance from the roads to the hamlet's houses and crofts.
            let area = f
                .decor
                .areas
                .iter()
                .find(|a| {
                    matches!(a.kind, AreaKind::Hamlet | AreaKind::Farmstead) && a.contains(h.x, h.z)
                })
                .expect("hamlet area")
                .footprint();
            let near = f
                .roads
                .iter()
                .flat_map(|r| {
                    r.points
                        .windows(2)
                        .map(|w| area.distance_to_segment(w[0], w[1]))
                })
                .fold(f64::INFINITY, f64::min);
            match h.layout {
                HamletLayout::Street => {
                    street += 1;
                    assert!(
                        near == 0.0,
                        "seed {seed}: the road misses a street hamlet by {near:.0} m"
                    );
                    // Houses on both sides of the road.
                    assert!(h.buildings.len() >= 4);
                }
                HamletLayout::Green => {
                    green += 1;
                    assert!(
                        near < 30.0,
                        "seed {seed}: green hamlet {near:.0} m off the road"
                    );
                }
                HamletLayout::Farmstead => {}
            }
        }
    }
    assert!(street >= 8, "{street} street hamlets");
    assert!(green >= 1, "{green} green hamlets");
}

#[test]
fn decor_areas_give_cover_slow_and_defend() {
    let mut f = decor_field(
        "prov_test",
        Terrain::Plains,
        BattleSeason::Summer,
        FieldSize::STANDARD,
        1,
    );
    f.clear_decor();
    f.place_plot(AreaKind::Vineyard, 300.0, 400.0, 80.0, 50.0, 0.0, None);
    f.place_manor(900.0, 400.0, 0.0, true);
    assert!(f.decor_speed_factor(300.0, 400.0, true, false) < 0.6);
    assert!(f.decor_speed_factor(300.0, 400.0, false, false) < 0.9);
    assert_eq!(f.decor_speed_factor(600.0, 400.0, true, false), 1.0);
    assert!(f.decor_breaks_charge(300.0, 400.0).is_some());
    assert!(f.decor_cover(900.0, 400.0) < 0.5);
    assert!(f.decor_defense(900.0, 400.0) > 1.3);
    assert!(!f.decor.moats.is_empty());
    // Muddy furrows in the rain.
    f.place_plot(AreaKind::Ploughland, 600.0, 200.0, 120.0, 60.0, 0.0, None);
    assert!(
        f.decor_speed_factor(600.0, 200.0, false, true)
            < f.decor_speed_factor(600.0, 200.0, false, false)
    );

    // In battle: the same regiment loses fewer men holding the manor.
    let losses = |in_manor: bool| {
        let data = data();
        let mut battle = setup(
            units(data, &["unit_men_at_arms_foot"]),
            units(data, &["unit_men_at_arms_foot"]),
            None,
        );
        battle.village = Some(false);
        let mut sim = BattleSim::new(battle, 4).unwrap();
        lab(&mut sim);
        sim.field_mut().clear_decor();
        if in_manor {
            sim.field_mut().place_manor(600.0, 400.0, 0.0, false);
        }
        place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
        place(&mut sim, 0, 600.0, 370.0, 0.0);
        sim.apply_command(
            sim_battle::Command::Attack {
                units: vec![0],
                target: 1,
                run: false,
                queue: false,
            },
            None,
        )
        .unwrap();
        run(&mut sim, 40.0);
        let u = &sim.units()[1];
        f64::from(u.max_soldiers) - u.hp
    };
    let (open, manor) = (losses(false), losses(true));
    assert!(
        manor < open * 0.85,
        "losses {open:.0} in the open, {manor:.0} in the manor"
    );
}

/// Knights of the attacker and a lone regiment of foot of the defender far
/// from the attacker's camp.
fn raid(guarded: bool) -> BattleSim {
    let data = data();
    let mut battle = setup(
        units(data, &["unit_men_at_arms_foot", "unit_men_at_arms_foot"]),
        units(data, &["unit_knights"]),
        None,
    );
    battle.village = Some(false);
    let mut sim = BattleSim::new(battle, 11).unwrap();
    lab(&mut sim);
    let camp = sim
        .field()
        .decor
        .camp(SideId::Attacker)
        .expect("attacker camp")
        .area;
    place(&mut sim, 2, camp.x, camp.z, 0.0);
    let far = if guarded {
        (camp.x + 30.0, camp.z + 40.0)
    } else {
        (camp.x, 700.0)
    };
    place(&mut sim, 0, far.0, far.1, 0.0);
    place(&mut sim, 1, camp.x + 400.0, 700.0, 0.0);
    hold_fire(&mut sim, SideId::Attacker);
    sim
}

#[test]
fn a_manor_before_the_line_is_a_defensive_cover() {
    let mut f = decor_field(
        "prov_test",
        Terrain::Plains,
        BattleSeason::Summer,
        FieldSize::STANDARD,
        2,
    );
    f.clear_decor();
    f.village = None;
    f.obstacles.clear();
    assert!(sim_battle::ai::defensive_cover(&f, SideId::Defender).is_none());
    f.place_manor(600.0, f.defender_line_z() - 30.0, 0.0, false);
    let cover =
        sim_battle::ai::defensive_cover(&f, SideId::Defender).expect("the manor is a cover");
    assert_eq!(cover.kind, sim_battle::ai::CoverKind::Village);
    assert_eq!(
        f.decor_area_at(cover.center.0, cover.center.1)
            .map(|a| a.kind),
        Some(AreaKind::Manor)
    );
}

#[test]
fn looting_an_unguarded_camp_costs_morale() {
    let mut sim = raid(false);
    let rules = &sim_battle::DecorRules::bundled().camp;
    run(&mut sim, rules.loot_seconds * 0.5);
    let state = sim.camp_state(SideId::Attacker).unwrap();
    assert!(state.alarmed && !state.looted && state.looters == 1);
    assert!(has_event(&sim, "entré dans le camp"));
    // Step until the camp falls; the morale blow lands on that step.
    let mut before: Vec<f64> = Vec::new();
    for _ in 0..(rules.loot_seconds / sim_battle::DT) as usize * 2 {
        before = sim.units().iter().map(|u| u.morale).collect();
        sim.step();
        if sim.camp_state(SideId::Attacker).unwrap().looted {
            break;
        }
    }
    let state = sim.camp_state(SideId::Attacker).unwrap();
    assert!(state.looted, "progress {:.2}", state.progress);
    assert!(has_event(&sim, "sont pillés"));
    for (i, morale) in before.iter().enumerate().take(2) {
        let lost = morale - sim.units()[i].morale;
        assert!(
            lost >= rules.looted_morale * 0.8,
            "regiment {i} lost {lost:.1} morale"
        );
    }
    sim.set_end_conditions(true);
    sim.units_mut()[2].hp = 0.0;
    run(&mut sim, 5.0);
    let outcome = sim.outcome().expect("battle over");
    assert!(outcome.attacker.baggage_lost);
    assert!(!outcome.defender.baggage_lost);
}

#[test]
fn a_guarded_camp_is_not_looted() {
    let mut sim = raid(true);
    let rules = &sim_battle::DecorRules::bundled().camp;
    run(&mut sim, rules.loot_seconds * 1.5);
    let state = sim.camp_state(SideId::Attacker).unwrap();
    assert!(!state.looted && state.guards >= 1, "{state:?}");
    assert!(!has_event(&sim, "sont pillés"));
}

#[test]
fn hand_placed_decor_plan_is_applied() {
    let plan: DecorPlan = serde_json::from_str(include_str!(
        "../../../../data/battle_maps/decor_plan_example.json"
    ))
    .expect("the example plan parses");
    let data = data();
    let mut battle = setup(
        units(data, &["unit_men_at_arms_foot"]),
        units(data, &["unit_longbowmen"]),
        None,
    );
    battle.province = "prov_ponthieu".to_owned();
    battle.decor_plan = Some(plan);
    let before = {
        let mut b = battle.clone();
        b.decor_plan = None;
        BattleSim::new(b, 5).unwrap().field().height(640.0, 700.0)
    };
    let sim = BattleSim::new(battle, 5).unwrap();
    let d = &sim.field().decor;
    assert!(d.buildings.iter().any(|b| b.kind == HouseKind::Windmill
        && (b.x - 640.0).abs() < 1e-6
        && (b.z - 700.0).abs() < 1e-6));
    // The windmill's mound.
    assert!(sim.field().height(640.0, 700.0) > before + 2.0);
    assert!(d.buildings.iter().any(|b| b.kind == HouseKind::Church));
    assert!(d.buildings.iter().any(|b| b.kind == HouseKind::Manor));
    assert_eq!(d.moats.len(), 1);
    assert!(d.areas.iter().any(|a| a.kind == AreaKind::Orchard));
    assert!(d.areas.iter().any(|a| a.kind == AreaKind::Vineyard));
    let camp = d.camp(SideId::Defender).unwrap();
    assert!((camp.area.x - 620.0).abs() < 1e-6 && (camp.area.z - 760.0).abs() < 1e-6);
    assert!(
        d.props
            .iter()
            .filter(|p| p.kind == DecorPropKind::Haystack)
            .count()
            >= 2
    );
    // The hand-placed hedge is a rule obstacle.
    assert!(sim
        .field()
        .obstacles
        .iter()
        .any(|o| o.kind == ObstacleKind::Hedge && o.a == (300.0, 300.0)));
}
