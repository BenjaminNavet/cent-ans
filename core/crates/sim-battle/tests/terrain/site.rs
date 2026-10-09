//! Battlefields drawn from the campaign site (lot B5): one field per terrain,
//! determinism, compatibility with the pre-B5 draws, and the game effects of
//! hedges, pools, beaches and snow.

use crate::common;

use common::{data, lab, setup, unit};
use data_model::Terrain;
use sim_battle::{
    BattleRng, BattleSeason, BattleSim, Battlefield, Command, FieldSite, Ground, Obstacle,
    ObstacleKind, Weather, ATTACKER_LINE_Z, DEFENDER_LINE_Z, DT, FIELD_WIDTH,
};

const TERRAINS: [Terrain; 7] = [
    Terrain::Plains,
    Terrain::Hills,
    Terrain::Mountains,
    Terrain::Forest,
    Terrain::Marsh,
    Terrain::Heath,
    Terrain::Bocage,
];

fn site(terrain: Terrain) -> FieldSite {
    FieldSite {
        terrain,
        river: false,
        coastal: false,
        season: BattleSeason::Summer,
    }
}

fn field(site: &FieldSite, weather: Weather, seed: u64) -> Battlefield {
    Battlefield::generate_site(site, weather, &mut BattleRng::from_seed(seed))
}

fn relief(field: &Battlefield) -> f64 {
    let max = field.heights.iter().cloned().fold(f64::MIN, f64::max);
    let min = field.heights.iter().cloned().fold(f64::MAX, f64::min);
    max - min
}

/// The centres of the two deployment lines stay free of every feature.
fn lines_are_clear(field: &Battlefield) -> bool {
    [ATTACKER_LINE_Z, DEFENDER_LINE_Z].iter().all(|&z| {
        (0..=24).all(|i| {
            let x = FIELD_WIDTH * 0.5 - 300.0 + 25.0 * f64::from(i);
            field.obstacles.iter().all(|o| o.distance(x, z) > 30.0)
                && field.water_at(x, z).is_none()
        })
    })
}

#[test]
fn one_field_per_terrain() {
    for terrain in TERRAINS {
        for seed in 0..12 {
            let f = field(&site(terrain), Weather::Clear, seed);
            assert_eq!(f.terrain, terrain);
            assert_eq!(f.heights.len(), f.nx * f.nz);
            assert!(f.heights.iter().all(|h| h.is_finite()));
            assert!(
                lines_are_clear(&f),
                "{terrain:?} seed {seed}: lines blocked"
            );
        }
    }
}

#[test]
fn terrains_look_like_their_province() {
    let mean = |terrain: Terrain, measure: &dyn Fn(&Battlefield) -> f64| {
        (0..16)
            .map(|seed| measure(&field(&site(terrain), Weather::Clear, seed)))
            .sum::<f64>()
            / 16.0
    };
    let hills = |f: &Battlefield| relief(f);
    assert!(mean(Terrain::Mountains, &hills) > mean(Terrain::Hills, &hills));
    assert!(mean(Terrain::Hills, &hills) > 2.0 * mean(Terrain::Plains, &hills));
    let woods = |f: &Battlefield| f.forests.len() as f64;
    assert!(mean(Terrain::Forest, &woods) > 3.0 * mean(Terrain::Plains, &woods));
    let hedges = |f: &Battlefield| {
        f.obstacles
            .iter()
            .filter(|o| o.kind == ObstacleKind::Hedge)
            .map(|o| o.length())
            .sum::<f64>()
    };
    assert!(mean(Terrain::Bocage, &hedges) > 3.0 * mean(Terrain::Heath, &hedges).max(1.0));
    let pools = |f: &Battlefield| f.pools.len() as f64;
    assert!(mean(Terrain::Marsh, &pools) >= 2.0);
    assert_eq!(mean(Terrain::Plains, &pools), 0.0);
    let ditches = |f: &Battlefield| {
        f.obstacles
            .iter()
            .filter(|o| o.kind == ObstacleKind::Ditch)
            .count() as f64
    };
    assert!(mean(Terrain::Marsh, &ditches) >= 1.0);
    assert_eq!(
        field(&site(Terrain::Forest), Weather::Clear, 1).woodland,
        1.0
    );
}

#[test]
fn same_seed_same_field_other_seed_other_field() {
    for terrain in TERRAINS {
        let mut s = site(terrain);
        s.coastal = true;
        s.river = true;
        let a = field(&s, Weather::Rain, 42);
        let b = field(&s, Weather::Rain, 42);
        assert_eq!(a, b);
        let c = field(&s, Weather::Rain, 43);
        assert_ne!(a, c);
    }
}

#[test]
fn site_features_do_not_shift_the_older_draws() {
    for terrain in TERRAINS {
        for seed in 0..6 {
            let mut plain_rng = BattleRng::from_seed(seed);
            let plain = Battlefield::generate(terrain, true, Weather::Rain, &mut plain_rng);
            let mut s = site(terrain);
            s.river = true;
            s.coastal = true;
            let mut rng = BattleRng::from_seed(seed);
            let rich = Battlefield::generate_site(&s, Weather::Rain, &mut rng);
            // Same RNG state after the field: every later draw of the
            // battle is unchanged.
            assert_eq!(plain_rng, rng);
            assert_eq!(plain.forests, rich.forests);
            assert_eq!(plain.river, rich.river);
            assert_eq!(&plain.mud[..], &rich.mud[..plain.mud.len()]);
            if rich.coast.is_none() {
                assert_eq!(plain.heights, rich.heights);
            }
            assert!(plain.obstacles.is_empty());
        }
    }
}

#[test]
fn ground_follows_season_and_weather() {
    let mut s = site(Terrain::Plains);
    s.season = BattleSeason::Winter;
    assert_eq!(field(&s, Weather::Snow, 1).ground, Ground::Snowy);
    let winter: Vec<Ground> = (0..40)
        .map(|seed| field(&s, Weather::Clear, seed).ground)
        .collect();
    assert!(winter.contains(&Ground::Snowy));
    assert!(winter.contains(&Ground::Muddy));
    s.season = BattleSeason::Summer;
    assert_eq!(field(&s, Weather::Clear, 1).ground, Ground::Dry);
    assert_eq!(field(&s, Weather::Rain, 1).ground, Ground::Muddy);
    // Snow on the ground slows the march unless it is already snowing.
    let snowy = (0..40)
        .map(|seed| {
            let mut w = site(Terrain::Plains);
            w.season = BattleSeason::Winter;
            field(&w, Weather::Clear, seed)
        })
        .find(|f| f.ground == Ground::Snowy)
        .unwrap();
    assert!(snowy.site_speed_factor(600.0, 250.0, false, Weather::Clear) < 1.0);
    assert_eq!(
        snowy.site_speed_factor(600.0, 250.0, false, Weather::Snow),
        1.0
    );
}

#[test]
fn coastal_provinces_get_a_beach_on_a_flank() {
    let mut s = site(Terrain::Plains);
    s.coastal = true;
    let fields: Vec<Battlefield> = (0..20)
        .map(|seed| field(&s, Weather::Clear, seed))
        .collect();
    let coast_count = fields.iter().filter(|f| f.coast.is_some()).count();
    assert!(coast_count >= 8, "{coast_count} coasts out of 20");
    let f = fields.iter().find(|f| f.coast.is_some()).unwrap();
    let coast = f.coast.unwrap();
    let x_beach = match coast.flank {
        sim_battle::Flank::West => 10.0,
        sim_battle::Flank::East => FIELD_WIDTH - 10.0,
    };
    assert!(coast.on_beach(x_beach));
    assert!(f.height(x_beach, 400.0) < 3.0);
    assert!(f.site_speed_factor(x_beach, 400.0, false, Weather::Clear) < 1.0);
    assert!(!coast.on_beach(FIELD_WIDTH * 0.5));
    // Inland provinces never see the sea.
    assert!(
        (0..20).all(|seed| field(&site(Terrain::Plains), Weather::Clear, seed)
            .coast
            .is_none())
    );
}

#[test]
fn battles_carry_the_site_and_sieges_drop_it() {
    let data = data();
    let army = || vec![unit(data, "unit_longbowmen"), unit(data, "unit_knights")];
    let mut s = setup(army(), army(), None);
    s.terrain = Terrain::Marsh;
    s.coastal = true;
    s.season = BattleSeason::Winter;
    let sim = BattleSim::new(s.clone(), 3).unwrap();
    assert_eq!(sim.field().terrain, Terrain::Marsh);
    assert_eq!(sim.field().season, BattleSeason::Winter);
    // The setup crosses the bridge as JSON: the new keys are optional.
    let json = serde_json::to_value(&s).unwrap();
    let mut legacy = json.clone();
    legacy.as_object_mut().unwrap().remove("coastal");
    let old: sim_battle::BattleSetup = serde_json::from_value(legacy).unwrap();
    assert!(!old.coastal);
    s.siege = Some(sim_battle::SiegeSetup {
        fortification: 1,
        breach: 0,
        ..Default::default()
    });
    let siege = BattleSim::new(s, 3).unwrap();
    assert!(siege.field().coast.is_none());
}

fn run(sim: &mut BattleSim, seconds: f64) {
    for _ in 0..(seconds / DT).round() as u64 {
        sim.step();
    }
}

fn losses(sim: &BattleSim, id: u32) -> f64 {
    let u = &sim.units()[id as usize];
    f64::from(u.initial_soldiers) - u.hp
}

fn place(sim: &mut BattleSim, id: u32, x: f64, z: f64, facing: f64) {
    let unit = &mut sim.units_mut()[id as usize];
    unit.x = x;
    unit.z = z;
    unit.facing = facing;
}

/// A hedge across the field at z = 390, from x = 450 to 750.
fn hedge(kind: ObstacleKind) -> Obstacle {
    Obstacle {
        a: (450.0, 390.0),
        b: (750.0, 390.0),
        kind,
    }
}

fn clear_site(sim: &mut BattleSim) {
    let field = sim.field_mut();
    field.obstacles.clear();
    field.pools.clear();
    field.coast = None;
    field.ground = Ground::Dry;
}

#[test]
fn archers_behind_a_hedge_suffer_less_from_arrows() {
    let data = data();
    let volley = |with_hedge: bool| {
        let mut sim = lab_sim(
            vec![unit(data, "unit_longbowmen")],
            vec![unit(data, "unit_longbowmen")],
        );
        if with_hedge {
            sim.field_mut().obstacles.push(hedge(ObstacleKind::Hedge));
        }
        place(&mut sim, 0, 600.0, 230.0, 0.0);
        place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
        sim.units_mut()[1].ammo = 0;
        sim.issue_command(Command::Attack {
            units: vec![0],
            target: 1,
            run: false,
            queue: false,
        })
        .unwrap();
        run(&mut sim, 40.0);
        losses(&sim, 1)
    };
    let open = volley(false);
    let covered = volley(true);
    assert!(open > 0.0);
    assert!(covered < open * 0.8, "covered {covered} vs open {open}");
}

#[test]
fn a_hedge_breaks_a_cavalry_charge() {
    let data = data();
    let charge = |with_hedge: bool| {
        let mut sim = lab_sim(
            vec![unit(data, "unit_knights")],
            vec![unit(data, "unit_longbowmen")],
        );
        if with_hedge {
            sim.field_mut().obstacles.push(hedge(ObstacleKind::Hedge));
        }
        place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
        place(&mut sim, 0, 600.0, 250.0, 0.0);
        sim.units_mut()[1].ammo = 0;
        sim.units_mut()[1].remove_ability(data_model::Ability::Stakes);
        sim.issue_command(Command::Attack {
            units: vec![0],
            target: 1,
            run: true,
            queue: false,
        })
        .unwrap();
        run(&mut sim, 30.0);
        let broken = sim
            .events()
            .iter()
            .any(|e| e.text_fr.contains("se brise sur la haie"));
        (broken, losses(&sim, 1))
    };
    let (broken_open, archers_open) = charge(false);
    let (broken_hedge, archers_hedge) = charge(true);
    assert!(!broken_open);
    assert!(broken_hedge);
    assert!(
        archers_hedge < archers_open,
        "archers {archers_hedge} vs {archers_open}"
    );
}

#[test]
fn obstacles_and_pools_slow_the_march() {
    let data = data();
    let mut sim = lab_sim(
        vec![unit(data, "unit_knights")],
        vec![unit(data, "unit_longbowmen")],
    );
    let field = sim.field_mut();
    field.obstacles.push(hedge(ObstacleKind::Hedge));
    let on_hedge = field.site_speed_factor(600.0, 390.0, true, Weather::Clear);
    let on_foot = field.site_speed_factor(600.0, 390.0, false, Weather::Clear);
    assert!(on_hedge < on_foot && on_foot < 1.0);
    assert_eq!(
        field.site_speed_factor(600.0, 300.0, true, Weather::Clear),
        1.0
    );
    field.pools.push(sim_battle::Zone {
        x: 300.0,
        z: 300.0,
        radius: 20.0,
    });
    assert_eq!(field.water_at(300.0, 300.0), Some(true));
    assert_eq!(field.water_at(300.0, 340.0), None);
}

/// Laboratory sim with no site features at all.
fn lab_sim(
    attacker: Vec<sim_battle::UnitSetup>,
    defender: Vec<sim_battle::UnitSetup>,
) -> BattleSim {
    let mut s = setup(attacker, defender, None);
    s.bare_field = true;
    let mut sim = BattleSim::new(s, 7).unwrap();
    lab(&mut sim);
    clear_site(&mut sim);
    sim
}
