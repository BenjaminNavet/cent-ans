//! Forest stands (« peuplements »): what a whole massif is made of (lot DN-FORET, ADR 0221).
//!
//! The species table of `species.rs` draws a species per biome, role and altitude; two forests of
//! the same biome therefore looked alike. A stand type adds a regional identity: a multiplier per
//! species (oak-hornbeam, beech, Scots pine, maritime pine of the Landes, fir-spruce of the
//! mountains, holm oak, taiga...), a tint, a density and a height factor. A stand is chosen by:
//!
//! 1. a named region (ellipse of `data/art/forest_stands.json`, e.g. the Landes), highest
//!    priority first; else
//! 2. an ecoregion cell (jittered Voronoi cells of `cell_px`) whose roll picks among the stand
//!    types eligible at the point (biome, altitude, latitude, longitude, conifer share).
//!
//! Rendering support only, no game rule. Optional: without a stand table the scatter is the HB4 one.

use crate::hash01;

/// Mean Earth radius of the authalic sphere (m), for the latitude of a map pixel.
const EARTH_R: f64 = 6_371_007.0;
const LAEA_LAT0: f64 = 52.0;
const LAEA_LON0: f64 = 10.0;
const LAEA_FALSE_E: f64 = 4_321_000.0;
const LAEA_FALSE_N: f64 = 3_210_000.0;

/// Named massif (ellipse in map pixels) with a fixed stand type.
#[derive(Clone, Debug)]
pub struct StandRegion {
    pub cx: f64,
    pub cy: f64,
    /// Semi-axes (px) along the ellipse angle and perpendicular to it.
    pub rx: f64,
    pub ry: f64,
    pub angle: f64,
    pub stand: usize,
    pub priority: f64,
}

/// Stand types, flattened from `data/art/forest_stands.json` by `ForestStands.table()`.
#[derive(Clone, Debug, Default)]
pub struct StandTable {
    pub count: usize,
    pub species_count: usize,
    /// `mult[stand * species_count + s]`: weight multiplier of a species in a stand.
    pub mult: Vec<f32>,
    /// `tint[3 * stand ..]` RGB multiplier of the instance tint.
    pub tint: Vec<f32>,
    pub density: Vec<f32>,
    pub height: Vec<f32>,
    /// Eligibility: bit `b` set = biome `b` allowed.
    pub biome_mask: Vec<i32>,
    /// `(min, max)` pairs.
    pub altitude: Vec<f32>,
    pub lat: Vec<f32>,
    pub lon: Vec<f32>,
    pub conifer: Vec<f32>,
    pub weight: Vec<f32>,
    pub regions: Vec<StandRegion>,
    /// Side (px) of the ecoregion cells and jitter of their seed points (0..1).
    pub cell_px: f64,
    pub jitter: f64,
    /// Map frame: projected `minx`, `maxy` (m) and metres per pixel.
    pub min_x_m: f64,
    pub max_y_m: f64,
    pub m_per_px: f64,
}

impl StandTable {
    pub fn is_valid(&self, species_count: usize) -> bool {
        let n = self.count;
        n > 0
            && self.species_count == species_count
            && self.mult.len() == n * species_count
            && self.tint.len() == 3 * n
            && [&self.density, &self.height, &self.weight]
                .iter()
                .all(|a| a.len() == n)
            && self.biome_mask.len() == n
            && [&self.altitude, &self.lat, &self.lon, &self.conifer]
                .iter()
                .all(|a| a.len() == 2 * n)
            && self.regions.iter().all(|r| r.stand < n)
            && self.m_per_px > 0.0
    }

    /// Longitude, latitude (degrees) of a map pixel (spherical inverse LAEA, ~0.1 degree).
    pub fn lonlat(&self, x: f64, y: f64) -> (f64, f64) {
        let e = self.min_x_m + x * self.m_per_px - LAEA_FALSE_E;
        let n = self.max_y_m - y * self.m_per_px - LAEA_FALSE_N;
        let rho = e.hypot(n);
        if rho < 1.0 {
            return (LAEA_LON0, LAEA_LAT0);
        }
        let c = 2.0 * (rho / (2.0 * EARTH_R)).clamp(-1.0, 1.0).asin();
        let (phi1_s, phi1_c) = LAEA_LAT0.to_radians().sin_cos();
        let (sc, cc) = c.sin_cos();
        let lat = (cc * phi1_s + n * sc * phi1_c / rho)
            .clamp(-1.0, 1.0)
            .asin();
        let lon = LAEA_LON0.to_radians() + (e * sc).atan2(rho * phi1_c * cc - n * phi1_s * sc);
        (lon.to_degrees(), lat.to_degrees())
    }

    fn eligible(&self, s: usize, biome: usize, alt: f64, conifer: f64, lonlat: (f64, f64)) -> bool {
        if self.biome_mask[s] & (1 << biome.min(30)) == 0 {
            return false;
        }
        let within = |r: &[f32], v: f64| v >= r[2 * s] as f64 && v <= r[2 * s + 1] as f64;
        within(&self.altitude, alt)
            && within(&self.conifer, conifer)
            && within(&self.lat, lonlat.1)
            && within(&self.lon, lonlat.0)
    }

    /// Stand of a point; `None` when the table has no stand eligible there.
    pub fn stand_at(&self, x: f64, y: f64, biome: usize, alt: f64, conifer: f64) -> Option<usize> {
        let mut best: Option<(f64, usize)> = None;
        for r in &self.regions {
            let (dx, dy) = (x - r.cx, y - r.cy);
            let (s, c) = r.angle.sin_cos();
            let u = (dx * c + dy * s) / r.rx;
            let v = (-dx * s + dy * c) / r.ry;
            if u * u + v * v <= 1.0 && best.is_none_or(|(p, _)| r.priority > p) {
                best = Some((r.priority, r.stand));
            }
        }
        if let Some((_, stand)) = best {
            return Some(stand);
        }
        let lonlat = self.lonlat(x, y);
        let roll = self.cell_roll(x, y);
        let mut total = 0.0;
        for s in 0..self.count {
            if self.eligible(s, biome, alt, conifer, lonlat) {
                total += self.weight[s] as f64;
            }
        }
        if total <= 0.0 {
            return None;
        }
        let target = roll * total;
        let mut acc = 0.0;
        let mut last = None;
        for s in 0..self.count {
            if !self.eligible(s, biome, alt, conifer, lonlat) {
                continue;
            }
            acc += self.weight[s] as f64;
            last = Some(s);
            if target < acc {
                return Some(s);
            }
        }
        last
    }

    /// Roll of the jittered Voronoi cell containing `(x, y)`: constant over an ecoregion.
    fn cell_roll(&self, x: f64, y: f64) -> f64 {
        let side = self.cell_px.max(1.0);
        let (cx, cy) = ((x / side).floor() as i64, (y / side).floor() as i64);
        let mut best = (f64::MAX, 0i64);
        for iy in cy - 1..=cy + 1 {
            for ix in cx - 1..=cx + 1 {
                let id = ix * 9176 + iy * 31337 + 101;
                let px = (ix as f64 + 0.5 + (hash01(id) - 0.5) * self.jitter) * side;
                let py = (iy as f64 + 0.5 + (hash01(id + 7) - 0.5) * self.jitter) * side;
                let d = (px - x).powi(2) + (py - y).powi(2);
                if d < best.0 {
                    best = (d, id);
                }
            }
        }
        hash01(best.1 + 13)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn table() -> StandTable {
        // Two stands over 3 species: 0 = oak stand, 1 = pine stand (low altitude only).
        StandTable {
            count: 2,
            species_count: 3,
            mult: vec![3.0, 0.1, 0.1, 0.1, 3.0, 0.1],
            tint: vec![1.0, 1.0, 1.0, 0.8, 0.9, 1.1],
            density: vec![1.0, 0.8],
            height: vec![1.0, 1.1],
            biome_mask: vec![0b110, 0b110],
            altitude: vec![0.0, 3000.0, 0.0, 3000.0],
            lat: vec![-90.0, 90.0, -90.0, 90.0],
            lon: vec![-180.0, 180.0, -180.0, 180.0],
            conifer: vec![0.0, 0.4, 0.4, 1.0],
            weight: vec![1.0, 1.0],
            regions: vec![StandRegion {
                cx: 100.0,
                cy: 100.0,
                rx: 20.0,
                ry: 10.0,
                angle: 0.0,
                stand: 1,
                priority: 1.0,
            }],
            cell_px: 40.0,
            jitter: 0.8,
            min_x_m: 2_169_486.0,
            max_y_m: 5_193_076.0,
            m_per_px: 718.976_562_5,
        }
    }

    #[test]
    fn region_wins_and_conifer_share_selects() {
        let t = table();
        assert!(t.is_valid(3));
        assert_eq!(t.stand_at(110.0, 100.0, 2, 100.0, 0.1), Some(1));
        assert_eq!(t.stand_at(500.0, 500.0, 2, 100.0, 0.1), Some(0));
        assert_eq!(t.stand_at(500.0, 500.0, 2, 100.0, 0.9), Some(1));
        assert_eq!(t.stand_at(500.0, 500.0, 0, 100.0, 0.1), None);
    }

    #[test]
    fn lonlat_of_paris_is_close() {
        let t = table();
        // Paris (2.35E, 48.86N) is at about px (2213, 3204) of the survey views.
        let (lon, lat) = t.lonlat(2213.0, 3204.0);
        assert!(
            (lon - 2.35).abs() < 0.6 && (lat - 48.86).abs() < 0.4,
            "{lon} {lat}"
        );
    }

    #[test]
    fn cells_are_stable_and_vary() {
        let mut t = table();
        t.regions.clear();
        t.conifer = vec![0.0, 1.0, 0.0, 1.0];
        let a = t.stand_at(1000.0, 1000.0, 2, 10.0, 0.5);
        assert_eq!(a, t.stand_at(1000.5, 1000.5, 2, 10.0, 0.5));
        let kinds: std::collections::HashSet<_> = (0..40)
            .map(|k| t.stand_at(k as f64 * 90.0, 300.0, 2, 10.0, 0.5))
            .collect();
        assert_eq!(kinds.len(), 2);
    }
}
