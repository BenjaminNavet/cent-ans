//! Tree species and their distribution by biome (lot HB4, ADR 0143).
//!
//! Mirror of `TreeSpecies` (`game/scripts/map/tree_species.gd`): the table compiled from
//! `data/art/tree_species.yaml` arrives flattened from GDScript (`VegetationScatter.set_species`)
//! and every scattered tree draws its species here. Weight of a species =
//! `base[biome][role] × altitude window × (1 + river affinity × river proximity) × conifer share`.
//! Rendering support only, no game rule.

/// Roles of a tree (order shared with `TreeSpecies.Role`).
pub const ROLE_MASSIF: usize = 0;
pub const ROLE_EDGE: usize = 1;
pub const ROLE_ISOLATED: usize = 2;
pub const ROLE_ORCHARD: usize = 3;
pub const ROLE_RIPARIAN: usize = 4;
pub const ROLE_SCRUB: usize = 5;
pub const ROLE_COUNT: usize = 6;
/// Biome indices 0..14 of `data/map/biomes.png` (0 = sea, 1-7 base classes, 8-14 regional
/// sub-classes that fall back to their parent, ADR 0237).
pub const BIOME_COUNT: usize = 15;
/// Per-biome parameters (`TreeSpecies.BIOME_KEYS`).
pub const B_FOREST: usize = 0;
pub const B_ISOLATED: usize = 1;
pub const B_GROVE: usize = 2;
pub const B_ORCHARD: usize = 3;
pub const B_ORCHARD_RING: usize = 4;
pub const B_ORCHARD_OPEN: usize = 5;
pub const B_RIPARIAN: usize = 6;
pub const B_RIPARIAN_PX: usize = 7;
pub const B_SCRUB: usize = 8;
pub const BIOME_STRIDE: usize = 9;
/// Instance custom data encoding: `r = tint + CUSTOM_STRIDE × (row + 1)`,
/// `g = tint + CUSTOM_STRIDE × (season class + 1)` (`foliage_common.gdshaderinc`).
pub const CUSTOM_STRIDE: f32 = 4.0;
/// Most species drawn from one table (atlas rows).
pub const MAX_SPECIES: usize = 64;
/// `VegetationTileJob.RIVER_CLEARANCE`.
const RIVER_CLEARANCE: f64 = 0.3;

/// Global distribution parameters (`distribution` of the catalogue).
#[derive(Clone, Debug)]
pub struct Distribution {
    pub massif_core: f64,
    pub massif_fill: f64,
    pub edge_fill: f64,
    pub stand_px: f64,
    pub stand_share: f64,
    pub altitude_fade_m: f64,
    pub river_reach_px: f64,
    pub conifer_raster: f64,
    pub orchard_parcel_px: f64,
    pub orchard_fill: f64,
    pub hedge_boost: f64,
    pub grove_core: f64,
    pub hedge_tree: f64,
    pub village_boost: f64,
    pub default_biome: usize,
}

impl Default for Distribution {
    fn default() -> Self {
        Distribution {
            massif_core: 0.6,
            massif_fill: 0.95,
            edge_fill: 0.8,
            stand_px: 3.0,
            stand_share: 0.7,
            altitude_fade_m: 150.0,
            river_reach_px: 3.0,
            conifer_raster: 0.6,
            orchard_parcel_px: 2.2,
            orchard_fill: 0.75,
            hedge_boost: 3.0,
            grove_core: 0.55,
            hedge_tree: 0.025,
            village_boost: 4.0,
            default_biome: 2,
        }
    }
}

/// Flattened species table (`TreeSpecies.table()`).
#[derive(Clone, Debug, Default)]
pub struct SpeciesTable {
    pub count: usize,
    /// `base[(biome * ROLE_COUNT + role) * count + s]`.
    pub base: Vec<f32>,
    pub alt_lo: Vec<f32>,
    pub alt_hi: Vec<f32>,
    pub river: Vec<f32>,
    /// 1 for conifers (conifer mesh), 0 for broadleaves.
    pub conifer: Vec<f32>,
    /// Fallback mesh kind (`KIND_OAK`, `KIND_BEECH`, `KIND_CONIFER`).
    pub kind: Vec<i32>,
    pub season: Vec<i32>,
    /// `(min, max)` pairs, map units.
    pub height: Vec<f32>,
    /// `(min, max)` width / height ratios.
    pub width: Vec<f32>,
    /// `BIOME_COUNT × BIOME_STRIDE`.
    pub biome_params: Vec<f32>,
    /// Parent of every biome (`data/map/biome_parents.json`, `BiomeParents.parents()`); empty =
    /// no fallback. A biome whose rows are all zero takes its parent's.
    pub biome_parent: Vec<i32>,
    pub dist: Distribution,
    /// Forest stand types (lot DN-FORET); `count == 0` = none.
    pub stands: crate::stands::StandTable,
}

impl SpeciesTable {
    /// True when every array has the size announced by `count`.
    pub fn is_valid(&self) -> bool {
        let n = self.count;
        n >= 3
            && self.base.len() == BIOME_COUNT * ROLE_COUNT * n
            && [&self.alt_lo, &self.alt_hi, &self.river, &self.conifer]
                .iter()
                .all(|a| a.len() == n)
            && self.kind.len() == n
            && self.season.len() == n
            && self.height.len() == 2 * n
            && self.width.len() == 2 * n
            && self.biome_params.len() == BIOME_COUNT * BIOME_STRIDE
    }

    /// True when the data defines something for biome `b` (a species weight or a parameter).
    fn biome_defined(&self, b: usize) -> bool {
        let n = self.count;
        let weights = &self.base[b * ROLE_COUNT * n..(b + 1) * ROLE_COUNT * n];
        let params = &self.biome_params[b * BIOME_STRIDE..(b + 1) * BIOME_STRIDE];
        weights.iter().chain(params).any(|&v| v != 0.0)
    }

    /// Biome whose rows are used for `b`: `b` itself, or the nearest ancestor when the data
    /// defines nothing for it (ADR 0237). The sea (0) and base classes never move.
    pub fn resolve_biome(&self, b: usize) -> usize {
        let mut b = b.min(BIOME_COUNT - 1);
        for _ in 0..BIOME_COUNT {
            let Some(&parent) = self.biome_parent.get(b) else {
                break;
            };
            let parent = parent.max(0) as usize;
            if parent == b || parent >= BIOME_COUNT || self.biome_defined(b) {
                break;
            }
            b = parent;
        }
        b
    }

    /// Parameter `key` (`B_*`) of biome `b`.
    pub fn biome(&self, b: usize, key: usize) -> f64 {
        self.biome_params[self.resolve_biome(b) * BIOME_STRIDE + key] as f64
    }

    /// Species drawn for a candidate, `None` when no species fits (`TreeSpecies.pick`).
    #[allow(clippy::too_many_arguments)]
    pub fn pick(
        &self,
        role: usize,
        biome: usize,
        altitude_m: f64,
        river_sd: f64,
        conifer_share: f64,
        roll: f64,
        stand: Option<usize>,
    ) -> Option<usize> {
        let n = self.count;
        let offset = (self.resolve_biome(biome) * ROLE_COUNT + role) * n;
        let fade = self.dist.altitude_fade_m.max(1.0);
        let reach = self.dist.river_reach_px.max(0.01);
        let near = 1.0 - smoothstep(RIVER_CLEARANCE, RIVER_CLEARANCE + reach, river_sd);
        let k = self.dist.conifer_raster;
        let mut weights = [0.0f64; MAX_SPECIES];
        let mut total = 0.0;
        for (s, weight) in weights.iter_mut().enumerate().take(n.min(MAX_SPECIES)) {
            let mut w = self.base[offset + s] as f64;
            if w <= 0.0 {
                continue;
            }
            let out = (self.alt_lo[s] as f64 - altitude_m)
                .max(altitude_m - self.alt_hi[s] as f64)
                .max(0.0);
            w *= (1.0 - out / fade).clamp(0.0, 1.0);
            w *= (1.0 + self.river[s] as f64 * near).max(0.0);
            let share = if self.conifer[s] > 0.5 {
                conifer_share
            } else {
                1.0 - conifer_share
            };
            w *= 1.0 + (0.25 + 1.5 * share - 1.0) * k;
            if let Some(st) = stand {
                w *= self.stands.mult[st * n + s] as f64;
            }
            *weight = w;
            total += w;
        }
        if total <= 0.0 {
            return None;
        }
        let target = roll * total;
        let mut acc = 0.0;
        let mut last = None;
        for (s, &w) in weights.iter().enumerate().take(n.min(MAX_SPECIES)) {
            if w <= 0.0 {
                continue;
            }
            acc += w;
            last = Some(s);
            if target < acc {
                return Some(s);
            }
        }
        last
    }

    /// Same value for every tree of a stand cell (`TreeSpecies.stand_roll`).
    pub fn stand_roll(&self, x: f64, y: f64) -> f64 {
        let side = self.dist.stand_px.max(0.1);
        let ix = (x / side).floor() as i64;
        let iy = (y / side).floor() as i64;
        crate::hash01(ix * 7919 + iy * 104729 + 17)
    }

    /// Orchard parcel draw (`TreeSpecies.parcel_roll`).
    pub fn parcel_roll(&self, x: f64, y: f64) -> f64 {
        let side = self.dist.orchard_parcel_px.max(0.1);
        let ix = (x / side).floor() as i64;
        let iy = (y / side).floor() as i64;
        crate::hash01(ix * 15731 + iy * 789221 + 3)
    }
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Three species: oak (broadleaf, low), fir (conifer, high), poplar (riparian).
    pub(crate) fn table() -> SpeciesTable {
        let n = 3;
        let mut base = vec![0.0; BIOME_COUNT * ROLE_COUNT * n];
        for b in 1..BIOME_COUNT {
            for (s, roles) in [
                (0usize, [1.0f32, 1.0, 1.0, 1.0, 0.0, 0.5]),
                (1, [1.0, 0.5, 0.1, 0.0, 0.0, 0.0]),
                (2, [0.0, 0.1, 0.1, 0.0, 1.0, 0.0]),
            ] {
                for (r, w) in roles.iter().enumerate() {
                    base[(b * ROLE_COUNT + r) * n + s] = *w;
                }
            }
        }
        let mut biome_params = vec![0.0; BIOME_COUNT * BIOME_STRIDE];
        for b in 1..BIOME_COUNT {
            let p = &mut biome_params[b * BIOME_STRIDE..(b + 1) * BIOME_STRIDE];
            p.copy_from_slice(&[1.0, 0.002, 0.3, 0.4, 5.0, 0.0, 0.4, 1.5, 0.0]);
        }
        // steppe: almost bare
        biome_params[4 * BIOME_STRIDE + B_FOREST] = 0.1;
        SpeciesTable {
            count: n,
            base,
            alt_lo: vec![0.0, 500.0, 0.0],
            alt_hi: vec![900.0, 1800.0, 1200.0],
            river: vec![0.0, 0.0, 1.5],
            conifer: vec![0.0, 1.0, 0.0],
            kind: vec![0, 2, 1],
            season: vec![0, 2, 4],
            height: vec![1.1, 1.7, 1.3, 2.1, 1.6, 2.3],
            width: vec![0.95, 1.25, 0.85, 1.05, 0.65, 0.8],
            biome_params,
            biome_parent: Vec::new(),
            dist: Distribution::default(),
            stands: Default::default(),
        }
    }

    #[test]
    fn pick_follows_altitude_role_and_river() {
        let t = table();
        assert!(t.is_valid());
        // low altitude, massif: only oak (fir out of its window)
        for k in 0..20 {
            let roll = k as f64 / 20.0;
            assert_eq!(t.pick(ROLE_MASSIF, 2, 100.0, 8.0, 0.5, roll, None), Some(0));
        }
        // high altitude: only fir
        assert_eq!(t.pick(ROLE_MASSIF, 2, 1500.0, 8.0, 0.5, 0.3, None), Some(1));
        // riparian role: only poplar
        assert_eq!(
            t.pick(ROLE_RIPARIAN, 2, 100.0, 0.5, 0.5, 0.9, None),
            Some(2)
        );
        // sea biome: nothing
        assert_eq!(t.pick(ROLE_MASSIF, 0, 100.0, 8.0, 0.5, 0.3, None), None);
    }

    /// Parents of `data/map/biome_parents.json` (ADR 0237).
    pub(crate) const PARENTS: [i32; BIOME_COUNT] = [0, 1, 2, 3, 4, 5, 6, 7, 7, 5, 2, 1, 2, 5, 3];

    /// A table where biomes 8-14 are empty (data written before the regional biomes).
    fn table_without_regional() -> SpeciesTable {
        let mut t = table();
        let n = t.count;
        for b in 8..BIOME_COUNT {
            t.base[b * ROLE_COUNT * n..(b + 1) * ROLE_COUNT * n].fill(0.0);
            t.biome_params[b * BIOME_STRIDE..(b + 1) * BIOME_STRIDE].fill(0.0);
        }
        t
    }

    #[test]
    fn empty_regional_biome_falls_back_to_its_parent() {
        let mut t = table_without_regional();
        // No parent table: an empty biome stays empty.
        assert_eq!(t.pick(ROLE_MASSIF, 10, 100.0, 8.0, 0.5, 0.3, None), None);
        t.biome_parent = PARENTS.to_vec();
        assert_eq!(t.resolve_biome(10), 2);
        assert_eq!(t.resolve_biome(8), 7);
        assert_eq!(t.resolve_biome(14), 3);
        assert_eq!(t.resolve_biome(0), 0);
        assert_eq!(
            t.pick(ROLE_MASSIF, 10, 100.0, 8.0, 0.5, 0.3, None),
            t.pick(ROLE_MASSIF, 2, 100.0, 8.0, 0.5, 0.3, None)
        );
        assert_eq!(t.biome(9, B_FOREST), t.biome(5, B_FOREST));
    }

    #[test]
    fn defined_regional_biome_keeps_its_own_rows() {
        let mut t = table_without_regional();
        t.biome_parent = PARENTS.to_vec();
        // Tundra: no tree weight at all, one parameter: it is defined, so no fallback.
        t.biome_params[9 * BIOME_STRIDE + B_FOREST] = 0.05;
        assert_eq!(t.resolve_biome(9), 9);
        assert_eq!(t.pick(ROLE_MASSIF, 9, 100.0, 8.0, 0.5, 0.3, None), None);
        assert!((t.biome(9, B_FOREST) - 0.05).abs() < 1e-6);
    }

    #[test]
    fn conifer_share_tilts_the_mix() {
        let t = table();
        let count = |share: f64| {
            (0..1000)
                .filter(|k| {
                    t.pick(ROLE_MASSIF, 6, 700.0, 8.0, share, *k as f64 / 1000.0, None) == Some(1)
                })
                .count()
        };
        assert!(count(0.9) > count(0.1) + 200);
    }
}
