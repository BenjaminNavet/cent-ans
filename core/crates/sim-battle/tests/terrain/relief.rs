//! Realistic relief of the battlefields (lot R2): compatibility of the draws,
//! determinism, bounded slopes on the deployment lines, amplitude per
//! terrain, defender's high ground, irregular woods and mud along the relief.

use data_model::Terrain;
use sim_battle::{
    BattleRng, Battlefield, ReliefStyle, Weather, ATTACKER_LINE_Z, DEFENDER_LINE_Z, FIELD_DEPTH,
    FIELD_WIDTH,
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

fn field(terrain: Terrain, river: bool, seed: u64) -> Battlefield {
    let weather = if river { Weather::Rain } else { Weather::Clear };
    Battlefield::generate(terrain, river, weather, &mut BattleRng::from_seed(seed))
}

/// Next draw of the battle stream after the field, recorded before R2: the
/// relief must not shift the units, the siege works or any later draw.
const PRE_R2_NEXT_DRAW: [(Terrain, bool, u64, u64); 28] = [
    (Terrain::Plains, false, 7, 0x7aeda4257c2a641b),
    (Terrain::Plains, false, 1234, 0xb9ec39e4d3549dbc),
    (Terrain::Plains, true, 7, 0xe052fb15908b74ae),
    (Terrain::Plains, true, 1234, 0xb5d32a27f0d90a4a),
    (Terrain::Hills, false, 7, 0x154edac2e5d52452),
    (Terrain::Hills, false, 1234, 0x83a25b8fd85a278f),
    (Terrain::Hills, true, 7, 0x2c527d03f91b76a),
    (Terrain::Hills, true, 1234, 0xa4b969f354c3cf3b),
    (Terrain::Mountains, false, 7, 0x5ba1c1d335e210df),
    (Terrain::Mountains, false, 1234, 0x3cd6e06869101795),
    (Terrain::Mountains, true, 7, 0x83591e27c5b8f825),
    (Terrain::Mountains, true, 1234, 0x502cd4ffa8abc28d),
    (Terrain::Forest, false, 7, 0xdcff66df0615afee),
    (Terrain::Forest, false, 1234, 0x502cd4ffa8abc28d),
    (Terrain::Forest, true, 7, 0x94c7cebca64ca239),
    (Terrain::Forest, true, 1234, 0x1c808e6e1070fad7),
    (Terrain::Marsh, false, 7, 0x5bcaf0b48a42e342),
    (Terrain::Marsh, false, 1234, 0x17c14bd117a48cae),
    (Terrain::Marsh, true, 7, 0xe15317191e60271b),
    (Terrain::Marsh, true, 1234, 0x33d1dc2a3296446e),
    (Terrain::Heath, false, 7, 0xb73c6171def32801),
    (Terrain::Heath, false, 1234, 0x402345996c9721fa),
    (Terrain::Heath, true, 7, 0x3b2562767cf97fce),
    (Terrain::Heath, true, 1234, 0x35c53e73b74715ae),
    (Terrain::Bocage, false, 7, 0x83591e27c5b8f825),
    (Terrain::Bocage, false, 1234, 0xf8188c98f0ecd42d),
    (Terrain::Bocage, true, 7, 0xafde0f2b0ebb8880),
    (Terrain::Bocage, true, 1234, 0x834975d49e48a1a0),
];

#[test]
fn relief_does_not_shift_the_later_draws() {
    for (terrain, river, seed, next) in PRE_R2_NEXT_DRAW {
        let weather = if river { Weather::Rain } else { Weather::Clear };
        let mut rng = BattleRng::from_seed(seed);
        let f = Battlefield::generate(terrain, river, weather, &mut rng);
        assert_eq!(
            rng.next_u64(),
            next,
            "{terrain:?} river {river} seed {seed}"
        );
        // The anchors keep their number (pools and site features hang on them).
        let base = match terrain {
            Terrain::Plains => (2, 1),
            Terrain::Heath => (1, 1),
            Terrain::Bocage => (7, 1),
            Terrain::Forest => (9, 1),
            Terrain::Hills | Terrain::Mountains => (3, 0),
            Terrain::Marsh => (1, 8),
            Terrain::Steppe | Terrain::Desert => (0, 0),
        };
        assert_eq!(f.forests.len(), base.0);
        assert_eq!(f.mud.len(), base.1 + if river { 2 } else { 0 });
    }
}

#[test]
fn same_seed_same_relief() {
    for terrain in TERRAINS {
        for river in [false, true] {
            let a = field(terrain, river, 99);
            assert_eq!(a, field(terrain, river, 99));
            assert_ne!(a.heights, field(terrain, river, 100).heights);
        }
    }
}

/// Steepest slope between neighbouring grid points on the centre of both
/// deployment lines (|z - line| <= 40 m, |x - 600| <= 360 m).
fn line_slope(f: &Battlefield) -> f64 {
    let mut steepest: f64 = 0.0;
    for line in [ATTACKER_LINE_Z, DEFENDER_LINE_Z] {
        let mut z = line - 40.0;
        while z <= line + 40.0 {
            let mut x = FIELD_WIDTH * 0.5 - 360.0;
            while x <= FIELD_WIDTH * 0.5 + 360.0 {
                let h = f.height(x, z);
                steepest = steepest
                    .max((f.height(x + 10.0, z) - h).abs() / 10.0)
                    .max((f.height(x, z + 10.0) - h).abs() / 10.0);
                x += 10.0;
            }
            z += 10.0;
        }
    }
    steepest
}

#[test]
fn deployment_lines_stay_playable() {
    for terrain in TERRAINS {
        let bound = ReliefStyle::of(terrain).line_slope;
        for seed in 0..16 {
            for river in [false, true] {
                let f = field(terrain, river, seed);
                let slope = line_slope(&f);
                assert!(
                    slope <= bound + 0.011,
                    "{terrain:?} seed {seed} river {river}: slope {slope:.3} > {bound}"
                );
                // Woods and mud stay off the centre of the lines.
                for line in [ATTACKER_LINE_Z, DEFENDER_LINE_Z] {
                    for i in 0..=24 {
                        let x = FIELD_WIDTH * 0.5 - 300.0 + 25.0 * f64::from(i);
                        assert!(!f.in_forest(x, line) && !f.in_mud(x, line));
                    }
                }
            }
        }
    }
}

fn range(f: &Battlefield) -> f64 {
    let max = f.heights.iter().cloned().fold(f64::MIN, f64::max);
    let min = f.heights.iter().cloned().fold(f64::MAX, f64::min);
    max - min
}

/// Local roughness: mean absolute difference between a grid point and the
/// mean of its neighbours at 30 m (the micro-relief, not the landforms).
fn roughness(f: &Battlefield) -> f64 {
    let (mut sum, mut n) = (0.0, 0.0);
    let mut z = 40.0;
    while z < FIELD_DEPTH - 40.0 {
        let mut x = 40.0;
        while x < FIELD_WIDTH - 40.0 {
            let ring = (f.height(x + 30.0, z)
                + f.height(x - 30.0, z)
                + f.height(x, z + 30.0)
                + f.height(x, z - 30.0))
                * 0.25;
            sum += (ring - f.height(x, z)).abs();
            n += 1.0;
            x += 20.0;
        }
        z += 20.0;
    }
    sum / n
}

fn mean(terrain: Terrain, measure: &dyn Fn(&Battlefield) -> f64) -> f64 {
    (0..16)
        .map(|seed| measure(&field(terrain, false, seed)))
        .sum::<f64>()
        / 16.0
}

#[test]
fn amplitude_follows_the_terrain() {
    let plains = mean(Terrain::Plains, &range);
    let hills = mean(Terrain::Hills, &range);
    let mountains = mean(Terrain::Mountains, &range);
    let marsh = mean(Terrain::Marsh, &range);
    let bocage = mean(Terrain::Bocage, &range);
    assert!(mountains > 1.5 * hills, "{mountains} vs {hills}");
    assert!(hills > 2.0 * plains, "{hills} vs {plains}");
    assert!(bocage > plains, "{bocage} vs {plains}");
    assert!(plains > marsh, "{plains} vs {marsh}");
    assert!((8.0..30.0).contains(&plains), "plains {plains}");
    assert!((3.0..15.0).contains(&marsh), "marsh {marsh}");
    assert!((30.0..90.0).contains(&hills), "hills {hills}");
    assert!((60.0..180.0).contains(&mountains), "mountains {mountains}");
    // The plains are not flat: undulations of a metre or so everywhere.
    let micro = mean(Terrain::Plains, &roughness);
    assert!(micro > 0.12, "plains roughness {micro}");
    assert!(mean(Terrain::Mountains, &roughness) > 3.0 * micro);
}

fn line_mean(f: &Battlefield, line: f64) -> f64 {
    (0..=36)
        .map(|i| f.height(FIELD_WIDTH * 0.5 - 360.0 + 20.0 * f64::from(i), line))
        .sum::<f64>()
        / 37.0
}

#[test]
fn defender_holds_the_high_ground_in_hills_and_mountains() {
    for terrain in [Terrain::Hills, Terrain::Mountains] {
        let rise = ReliefStyle::of(terrain).defender_rise;
        for seed in 0..16 {
            for river in [false, true] {
                let f = field(terrain, river, seed);
                let gap = line_mean(&f, DEFENDER_LINE_Z) - line_mean(&f, ATTACKER_LINE_Z);
                assert!(
                    gap >= rise * 0.8,
                    "{terrain:?} seed {seed} river {river}: {gap:.1} m"
                );
            }
        }
    }
}

/// Hollowness at (x, z): ring at 40 m minus the centre.
fn hollow(f: &Battlefield, x: f64, z: f64) -> f64 {
    (f.height(x + 40.0, z) + f.height(x - 40.0, z) + f.height(x, z + 40.0) + f.height(x, z - 40.0))
        * 0.25
        - f.height(x, z)
}

fn slope(f: &Battlefield, x: f64, z: f64) -> f64 {
    let gx = (f.height(x + 10.0, z) - f.height(x - 10.0, z)) / 20.0;
    let gz = (f.height(x, z + 10.0) - f.height(x, z - 10.0)) / 20.0;
    (gx * gx + gz * gz).sqrt()
}

#[test]
fn woods_and_mud_are_irregular_and_follow_the_relief() {
    let (mut mud_hollow, mut any_hollow, mut wood_slope, mut any_slope) = (0.0, 0.0, 0.0, 0.0);
    let (mut muds, mut woods, mut samples) = (0.0, 0.0, 0.0);
    for seed in 0..12 {
        for terrain in [
            Terrain::Hills,
            Terrain::Forest,
            Terrain::Marsh,
            Terrain::Bocage,
        ] {
            let f = field(terrain, false, seed);
            // Every wood is a massif of several lobes.
            assert!(f.forest_parts.len() >= 2 * f.forests.len());
            assert!(f.mud_parts.len() >= f.mud.len());
            for part in &f.forest_parts {
                assert!(f.in_forest(part.x, part.z));
            }
            for m in &f.mud {
                mud_hollow += hollow(&f, m.x, m.z);
                muds += 1.0;
            }
            for w in &f.forests {
                wood_slope += slope(&f, w.x, w.z);
                woods += 1.0;
            }
            for i in 0..40 {
                let x = 60.0 + 27.0 * f64::from(i);
                let z = 60.0 + (f64::from(i) * 97.0) % 680.0;
                any_hollow += hollow(&f, x, z);
                any_slope += slope(&f, x, z);
                samples += 1.0;
            }
        }
    }
    let (mud_hollow, any_hollow) = (mud_hollow / muds, any_hollow / samples);
    let (wood_slope, any_slope) = (wood_slope / woods, any_slope / samples);
    assert!(
        mud_hollow > any_hollow + 0.2,
        "mud {mud_hollow} vs {any_hollow}"
    );
    assert!(
        wood_slope > any_slope * 1.1,
        "woods {wood_slope} vs {any_slope}"
    );
}

/// Diagnostic (`cargo test -p sim-battle --test relief -- --ignored --nocapture`).
#[test]
#[ignore]
fn print_relief_stats() {
    for terrain in TERRAINS {
        let r = mean(terrain, &range);
        let micro = mean(terrain, &roughness);
        let slope = (0..16)
            .map(|s| line_slope(&field(terrain, true, s)))
            .fold(0.0, f64::max);
        println!("{terrain:?}: range {r:.1} m, roughness {micro:.2} m, max line slope {slope:.3}");
    }
}
