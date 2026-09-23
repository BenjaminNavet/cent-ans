//! Battlefield (height, forests, mud, river) and weather, generated from the
//! seed, the province terrain and the season (spec § 1).

use data_model::Terrain;
use serde::{Deserialize, Serialize};

use crate::rng::BattleRng;
use crate::setup::BattleSeason;

/// Width of the field along x, in metres.
pub const FIELD_WIDTH: f64 = 1200.0;
/// Depth of the field along z, in metres.
pub const FIELD_DEPTH: f64 = 800.0;
/// Spacing of the height grid, in metres.
pub const GRID_RESOLUTION: f64 = 10.0;

/// A circular zone (forest or mud).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub x: f64,
    pub z: f64,
    pub radius: f64,
}

impl Zone {
    pub fn contains(&self, x: f64, z: f64) -> bool {
        let (dx, dz) = (x - self.x, z - self.z);
        dx * dx + dz * dz <= self.radius * self.radius
    }
}

/// A ford across the river.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ford {
    pub x: f64,
    pub half_width: f64,
}

/// A river crossing the field from west to east between the two armies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct River {
    pub z0: f64,
    pub amplitude: f64,
    pub wavelength: f64,
    pub phase: f64,
    pub width: f64,
    pub fords: Vec<Ford>,
}

impl River {
    /// z of the river centre line at `x`.
    pub fn center_z(&self, x: f64) -> f64 {
        self.z0 + self.amplitude * (x / self.wavelength * std::f64::consts::TAU + self.phase).sin()
    }

    pub fn in_water(&self, x: f64, z: f64) -> bool {
        (z - self.center_z(x)).abs() <= self.width * 0.5
    }

    pub fn in_ford(&self, x: f64) -> bool {
        self.fords.iter().any(|f| (x - f.x).abs() <= f.half_width)
    }

    /// Centre line sampled every `step` metres (for rendering).
    pub fn polyline(&self, step: f64) -> Vec<(f64, f64)> {
        let count = (FIELD_WIDTH / step).ceil() as usize;
        (0..=count)
            .map(|i| {
                let x = (i as f64 * step).min(FIELD_WIDTH);
                (x, self.center_z(x))
            })
            .collect()
    }
}

/// Weather of the battle (spec § 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weather {
    Clear,
    /// Longbows and crossbows shoot 40 % worse; mud is heavier.
    Rain,
    /// Ranges are 30 % shorter.
    Fog,
    /// Slower and more tiring marches.
    Snow,
}

impl Weather {
    pub fn key(self) -> &'static str {
        match self {
            Weather::Clear => "clear",
            Weather::Rain => "rain",
            Weather::Fog => "fog",
            Weather::Snow => "snow",
        }
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            Weather::Clear => "Temps clair",
            Weather::Rain => "Pluie",
            Weather::Fog => "Brouillard",
            Weather::Snow => "Neige",
        }
    }

    /// Draw by season (percent chances clear / rain / fog / snow).
    pub fn draw(season: BattleSeason, rng: &mut BattleRng) -> Weather {
        let table: [u32; 4] = match season {
            BattleSeason::Spring => [60, 30, 10, 0],
            BattleSeason::Summer => [80, 15, 5, 0],
            BattleSeason::Autumn => [45, 35, 20, 0],
            BattleSeason::Winter => [35, 20, 15, 30],
        };
        let mut roll = rng.below(100);
        for (weather, chance) in [Weather::Clear, Weather::Rain, Weather::Fog, Weather::Snow]
            .into_iter()
            .zip(table)
        {
            if roll < chance {
                return weather;
            }
            roll -= chance;
        }
        Weather::Clear
    }

    /// Multiplier on the range of every shooter.
    pub fn range_factor(self) -> f64 {
        if self == Weather::Fog {
            0.7
        } else {
            1.0
        }
    }

    /// Multiplier on the accuracy of bows and crossbows (`rain_penalty`).
    pub fn bow_factor(self) -> f64 {
        match self {
            Weather::Rain => 0.6,
            Weather::Snow => 0.8,
            _ => 1.0,
        }
    }
}

/// The generated battlefield.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Battlefield {
    pub width: f64,
    pub depth: f64,
    pub resolution: f64,
    /// Grid points along x.
    pub nx: usize,
    /// Grid points along z.
    pub nz: usize,
    /// Row-major heights (z rows of `nx` values), in metres.
    pub heights: Vec<f64>,
    pub forests: Vec<Zone>,
    pub mud: Vec<Zone>,
    pub river: Option<River>,
}

struct Hill {
    x: f64,
    z: f64,
    radius: f64,
    height: f64,
}

/// z of the attacker's and defender's battle lines at deployment.
pub const ATTACKER_LINE_Z: f64 = 250.0;
pub const DEFENDER_LINE_Z: f64 = 550.0;

impl Battlefield {
    /// Builds the field for `terrain`, adding a river when `river` is set and
    /// extra mud in the rain or snow.
    pub fn generate(terrain: Terrain, river: bool, weather: Weather, rng: &mut BattleRng) -> Self {
        let (hill_count, hill_height, forest_count, mud_count) = match terrain {
            Terrain::Plains => (4, 5.0, 2, 1),
            Terrain::Heath => (5, 7.0, 1, 1),
            Terrain::Bocage => (5, 7.0, 7, 1),
            Terrain::Forest => (5, 9.0, 9, 1),
            Terrain::Hills => (7, 22.0, 3, 0),
            Terrain::Mountains => (8, 45.0, 3, 0),
            Terrain::Marsh => (3, 2.0, 1, 8),
        };
        let mut hills: Vec<Hill> = (0..hill_count)
            .map(|_| Hill {
                x: rng.range(0.0, FIELD_WIDTH),
                z: rng.range(0.0, FIELD_DEPTH),
                radius: rng.range(80.0, 260.0),
                height: hill_height * rng.range(0.4, 1.0),
            })
            .collect();
        // Hills and mountains: the defender holds the high ground.
        if matches!(terrain, Terrain::Hills | Terrain::Mountains) {
            hills.push(Hill {
                x: FIELD_WIDTH * 0.5,
                z: DEFENDER_LINE_Z + 60.0,
                radius: 320.0,
                height: hill_height * 0.8,
            });
        }
        let nx = (FIELD_WIDTH / GRID_RESOLUTION) as usize + 1;
        let nz = (FIELD_DEPTH / GRID_RESOLUTION) as usize + 1;
        let tilt_x = rng.range(-0.004, 0.004);
        let tilt_z = rng.range(-0.004, 0.004);
        let river_def = river.then(|| {
            let fords = (0..2)
                .map(|i| Ford {
                    x: FIELD_WIDTH * (0.3 + 0.4 * i as f64) + rng.range(-80.0, 80.0),
                    half_width: 35.0,
                })
                .collect();
            River {
                z0: (ATTACKER_LINE_Z + DEFENDER_LINE_Z) * 0.5 + rng.range(-30.0, 30.0),
                amplitude: rng.range(10.0, 35.0),
                wavelength: rng.range(500.0, 900.0),
                phase: rng.range(0.0, std::f64::consts::TAU),
                width: 18.0,
                fords,
            }
        });
        let mut heights = Vec::with_capacity(nx * nz);
        for iz in 0..nz {
            for ix in 0..nx {
                let x = ix as f64 * GRID_RESOLUTION;
                let z = iz as f64 * GRID_RESOLUTION;
                let mut h =
                    3.0 + tilt_x * (x - FIELD_WIDTH * 0.5) + tilt_z * (z - FIELD_DEPTH * 0.5);
                for hill in &hills {
                    let d2 = (x - hill.x).powi(2) + (z - hill.z).powi(2);
                    h += hill.height * (-d2 / (hill.radius * hill.radius)).exp();
                }
                if let Some(r) = &river_def {
                    let d = (z - r.center_z(x)).abs();
                    if d < r.width * 1.5 {
                        let depth = if r.in_ford(x) { 0.6 } else { 1.6 };
                        h -= depth * (1.0 - d / (r.width * 1.5));
                    }
                }
                heights.push(h);
            }
        }
        // Forests and mud stay off the centre of the deployment lines.
        let zone = |radius_low: f64, radius_high: f64, rng: &mut BattleRng| loop {
            let candidate = Zone {
                x: rng.range(0.0, FIELD_WIDTH),
                z: rng.range(60.0, FIELD_DEPTH - 60.0),
                radius: rng.range(radius_low, radius_high),
            };
            let near_line = |line: f64| {
                (candidate.z - line).abs() < candidate.radius + 40.0
                    && (candidate.x - FIELD_WIDTH * 0.5).abs() < 360.0 + candidate.radius
            };
            if !near_line(ATTACKER_LINE_Z) && !near_line(DEFENDER_LINE_Z) {
                break candidate;
            }
        };
        let forests = (0..forest_count).map(|_| zone(35.0, 90.0, rng)).collect();
        let wet = matches!(weather, Weather::Rain | Weather::Snow);
        let mud = (0..mud_count + if wet { 2 } else { 0 })
            .map(|_| zone(30.0, 70.0, rng))
            .collect();
        Battlefield {
            width: FIELD_WIDTH,
            depth: FIELD_DEPTH,
            resolution: GRID_RESOLUTION,
            nx,
            nz,
            heights,
            forests,
            mud,
            river: river_def,
        }
    }

    /// Bilinear height at (x, z), clamped to the field.
    pub fn height(&self, x: f64, z: f64) -> f64 {
        let fx = (x / self.resolution).clamp(0.0, (self.nx - 1) as f64);
        let fz = (z / self.resolution).clamp(0.0, (self.nz - 1) as f64);
        let (ix, iz) = (fx.floor() as usize, fz.floor() as usize);
        let (ix1, iz1) = ((ix + 1).min(self.nx - 1), (iz + 1).min(self.nz - 1));
        let (tx, tz) = (fx - ix as f64, fz - iz as f64);
        let at = |i: usize, j: usize| self.heights[j * self.nx + i];
        let top = at(ix, iz) * (1.0 - tx) + at(ix1, iz) * tx;
        let bottom = at(ix, iz1) * (1.0 - tx) + at(ix1, iz1) * tx;
        top * (1.0 - tz) + bottom * tz
    }

    pub fn in_forest(&self, x: f64, z: f64) -> bool {
        self.forests.iter().any(|f| f.contains(x, z))
    }

    pub fn in_mud(&self, x: f64, z: f64) -> bool {
        self.mud.iter().any(|m| m.contains(x, z))
    }

    /// `Some(true)` in a ford, `Some(false)` in deep water, `None` on land.
    pub fn water_at(&self, x: f64, z: f64) -> Option<bool> {
        let river = self.river.as_ref()?;
        river.in_water(x, z).then(|| river.in_ford(x))
    }

    pub fn inside(&self, x: f64, z: f64) -> bool {
        (0.0..=self.width).contains(&x) && (0.0..=self.depth).contains(&z)
    }

    /// `true` when the ground between the two points rises above the line of
    /// sight (eyes 2 m above the ground).
    pub fn blocks_sight(&self, from: (f64, f64), to: (f64, f64)) -> bool {
        let h0 = self.height(from.0, from.1) + 2.0;
        let h1 = self.height(to.0, to.1) + 2.0;
        (1..8).any(|i| {
            let t = f64::from(i) / 8.0;
            let x = from.0 + (to.0 - from.0) * t;
            let z = from.1 + (to.1 - from.1) * t;
            self.height(x, z) > h0 + (h1 - h0) * t + 1.0
        })
    }
}
