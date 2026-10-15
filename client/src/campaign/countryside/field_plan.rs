//! Placement of the generated 3D field models on the campaign map (`field_plan.gd`, ADR 0222)
//! and the shared parcel layout of the terrain shader / hedge scatter (`vegetation_fields.gd`).
//! Pure computation, deterministic (integer hashing), no ECS.
//!
//! Data: crop weights per biome (`ground_biome_mix.json`) and per regional landscape
//! (`agri_landscapes.json` + mask), splat (crops, forest); the material -> models table lives in
//! `data/art/dn_fields.json`.
//! - Dominant-crop regions (wide Voronoi): one common draw for 70 % of the parcels, like the
//!   shader (`hb_region_cells`); the others draw alone.
//! - Jittered-site Voronoi parcels (`parcel_px`); only those whose centre is farmed (splat crop
//!   share, no forest, no town, no river) carry a model.
//! - "One parcel = one model": one instance per farmed parcel, centred on the site, footprint =
//!   parcel - `headland_px`, rows along the region frame.
//!
//! Output of [`FieldPlan::plan_cell`]: model id -> flat `(x, z, yaw, size_m)` records.
//! The crop order of a weight table is the file order (draws are cumulative), kept by [`Ordered`].

use std::collections::{BTreeMap, HashMap};
use std::f32::consts::PI;

use bevy::math::{IVec2, Vec2};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::placement::Terrain;
use crate::campaign::vegetation::detail_profile::smoothstep;

pub const STRIDE: usize = 4;
pub const SALT_REGION: u32 = 11;
pub const SALT_PARCEL: u32 = 23;
/// First row of the regional landscapes in the ground table.
pub const FIRST_LANDSCAPE_ROW: i32 = 8;
/// Metres per map pixel when no map is given.
pub const DEFAULT_METERS_PER_PX: f32 = 718.976_56;

// --- Ordered maps ------------------------------------------------------------------------

/// A JSON object read as an ordered list (draws are cumulative in file order).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ordered<T>(pub Vec<(String, T)>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Ordered<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = Ordered<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Ordered<T>, A::Error> {
                let mut out = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, T>()? {
                    out.push((k, v));
                }
                Ok(Ordered(out))
            }
        }
        deserializer.deserialize_map(V(std::marker::PhantomData))
    }
}

// --- Configuration -----------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct PlanConfig {
    pub parcel_px: f32,
    pub region_px: f32,
    pub dominant_share: f32,
    pub dominant_jitter: f32,
    pub cross_share: f32,
    pub forest_max: f32,
    pub farm_gain: f32,
    pub headland_px: f32,
    pub town_clear_px: f32,
    pub river_clear_px: f32,
    pub footprint_fill: f32,
    pub yaw_jitter: f32,
    pub size_jitter: f32,
}

impl Default for PlanConfig {
    fn default() -> Self {
        PlanConfig {
            parcel_px: 0.8,
            region_px: 16.0,
            dominant_share: 0.7,
            dominant_jitter: 0.25,
            cross_share: 0.15,
            forest_max: 0.4,
            farm_gain: 1.0,
            headland_px: 0.1,
            town_clear_px: 0.4,
            river_clear_px: 0.12,
            footprint_fill: 0.9,
            yaw_jitter: 0.08,
            size_jitter: 0.06,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Variant {
    pub id: String,
    pub weight: f32,
    #[serde(default = "one")]
    pub tile: i32,
}

fn one() -> i32 {
    1
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Crop {
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct RenderConfig {
    pub cell_px: f32,
    pub max_instances: u32,
}

/// `data/art/dn_fields.json`.
#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default)]
pub struct FieldsConfig {
    pub enabled: bool,
    pub render: RenderConfig,
    pub plan: PlanConfig,
    pub crops: HashMap<String, Crop>,
    pub crops_closeup: HashMap<String, Crop>,
}

/// One row of the ground table (biome or regional landscape).
#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default)]
pub struct MixEntry {
    pub crops: Ordered<f32>,
    pub farm: Option<f32>,
    pub open_to_farm: f32,
}

#[derive(Deserialize, Default)]
struct MixFile {
    #[serde(default)]
    biomes: HashMap<String, MixEntry>,
}

#[derive(Deserialize, Default)]
struct LandscapesFile {
    #[serde(default)]
    landscapes: Ordered<MixEntry>,
}

/// Ground table rows: biome keys "1".."N" and landscape rows (`FIRST_LANDSCAPE_ROW + index`).
pub fn build_mix(
    mix_json: &str,
    landscapes_json: &str,
) -> Result<HashMap<i32, MixEntry>, serde_json::Error> {
    let mix: MixFile = serde_json::from_str(mix_json)?;
    let landscapes: LandscapesFile = serde_json::from_str(landscapes_json)?;
    let mut rows = HashMap::new();
    for (key, entry) in mix.biomes {
        if let Ok(row) = key.parse::<i32>() {
            rows.insert(row, entry);
        }
    }
    for (n, (_, entry)) in landscapes.landscapes.0.into_iter().enumerate() {
        rows.insert(FIRST_LANDSCAPE_ROW + n as i32, entry);
    }
    Ok(rows)
}

// --- Hash --------------------------------------------------------------------------------

/// `FieldPlan.hash01`: 32-bit integer hash in [0, 1).
pub fn hash01(a: i32, b: i32, salt: u32) -> f32 {
    let mut h = (a as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((b as u32).wrapping_mul(668_265_263))
        .wrapping_add(salt.wrapping_mul(2_147_483_629));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    h = (h ^ (h >> 15)).wrapping_mul(2_246_822_519);
    ((h ^ (h >> 13)) & 0xFF_FFFF) as f32 / 16_777_216.0
}

// --- Plan --------------------------------------------------------------------------------

/// Dominant crop region of a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub id: IVec2,
    pub roll: f32,
    pub angle: f32,
}

/// A farmed parcel.
#[derive(Clone, Debug, PartialEq)]
pub struct Parcel {
    pub site: Vec2,
    pub material: String,
    pub variant: Variant,
    pub angle: f32,
}

pub struct FieldPlan<'a, T: Terrain> {
    pub config: &'a FieldsConfig,
    pub mix: &'a HashMap<i32, MixEntry>,
    pub terrain: &'a T,
    pub meters_per_px: f32,
}

impl<T: Terrain> FieldPlan<'_, T> {
    fn plan(&self) -> &PlanConfig {
        &self.config.plan
    }

    /// Whether the plan can produce something (table present, `enabled`).
    pub fn is_ready(&self) -> bool {
        self.config.enabled && !self.config.crops.is_empty()
    }

    /// Row of the ground table at the point: regional landscape if any, else the biome (1..=7),
    /// 0 for none.
    pub fn ground_row(&self, p: Vec2) -> i32 {
        let row = self.terrain.landscape_row(p);
        if row >= 0 && self.mix.contains_key(&row) && row >= FIRST_LANDSCAPE_ROW {
            return row;
        }
        let biome = self.terrain.biome(p);
        if biome > 0 { biome.clamp(1, 7) } else { 0 }
    }

    /// Crop share of the splat with the meadow -> crops conversion of the regional landscape
    /// (`open_to_farm`, ME8); 0 in dense forest.
    pub fn farm_share(&self, p: Vec2, row: i32) -> f32 {
        let splat = self.terrain.splat(p);
        if splat[2] > self.plan().forest_max {
            return 0.0;
        }
        let entry = self.mix.get(&row);
        let open_to_farm = entry.map_or(0.0, |e| e.open_to_farm);
        let farm = entry.and_then(|e| e.farm).unwrap_or(1.0);
        ((splat[1] + splat[0] * open_to_farm) * farm * self.plan().farm_gain).clamp(0.0, 1.0)
    }

    /// Dominant region of the point.
    pub fn region_at(&self, p: Vec2) -> Region {
        let u = p / self.plan().region_px;
        let cell = IVec2::new(u.x.floor() as i32, u.y.floor() as i32);
        let mut best = IVec2::ZERO;
        let mut best_d = f32::INFINITY;
        for j in -1..=1 {
            for i in -1..=1 {
                let id = cell + IVec2::new(i, j);
                let site = Vec2::new(id.x as f32, id.y as f32)
                    + Vec2::new(
                        0.1 + 0.8 * hash01(id.x, id.y, 3),
                        0.1 + 0.8 * hash01(id.x, id.y, 5),
                    );
                let d = u.distance(site) - (hash01(id.x, id.y, 7) - 0.5) * 0.7;
                if d < best_d {
                    best_d = d;
                    best = id;
                }
            }
        }
        Region {
            id: best,
            roll: hash01(best.x, best.y, SALT_REGION),
            angle: hash01(best.x, best.y, SALT_REGION + 1) * PI,
        }
    }

    /// Site (map px) of the parcel of cell `id`.
    pub fn parcel_site(&self, id: IVec2) -> Vec2 {
        let size = self.plan().parcel_px;
        (Vec2::new(id.x as f32, id.y as f32)
            + Vec2::new(
                0.1 + 0.8 * hash01(id.x, id.y, 31),
                0.1 + 0.8 * hash01(id.x, id.y, 37),
            ))
            * size
    }

    /// Material drawn in the weights of row `row` for the roll in [0, 1]; empty without crops.
    pub fn pick_material(&self, row: i32, roll: f32) -> &str {
        let Some(entry) = self.mix.get(&row) else {
            return "";
        };
        let total: f32 = entry.crops.0.iter().map(|(_, w)| w).sum();
        if total <= 0.0 {
            return "";
        }
        let target = roll.clamp(0.0, 0.9999) * total;
        let mut cumulative = 0.0;
        for (id, weight) in &entry.crops.0 {
            cumulative += weight;
            if target <= cumulative {
                return id;
            }
        }
        ""
    }

    /// Roll of the parcel `id`: the region's for `dominant_share` of the parcels (spread
    /// `dominant_jitter`), its own for the others.
    pub fn parcel_roll(&self, id: IVec2, region_roll: f32) -> f32 {
        let own = hash01(id.x, id.y, 43);
        if hash01(id.x, id.y, 47) < self.plan().dominant_share {
            (region_roll + (own - 0.5) * self.plan().dominant_jitter * 2.0).clamp(0.0, 1.0)
        } else {
            own
        }
    }

    /// Dominant crop material of the region at the point (tests, comparison with the ground).
    pub fn dominant_material(&self, p: Vec2) -> &str {
        let row = self.ground_row(p);
        if row > 0 {
            self.pick_material(row, self.region_at(p).roll)
        } else {
            ""
        }
    }

    /// Farmed parcel of cell `id`; `None` when not farmed.
    pub fn parcel(&self, id: IVec2) -> Option<Parcel> {
        let site = self.parcel_site(id);
        let size = self.terrain.size();
        if site.x < 0.0 || site.y < 0.0 || site.x >= size.x || site.y >= size.y {
            return None;
        }
        let row = self.ground_row(site);
        if row <= 0 {
            return None;
        }
        if hash01(id.x, id.y, 53) >= self.farm_share(site, row) {
            return None;
        }
        if !self.terrain.is_land(site) || self.terrain.river_sd(site) < self.plan().river_clear_px {
            return None;
        }
        if self.terrain.near_town(site, self.plan().town_clear_px) {
            return None;
        }
        let region = self.region_at(site);
        let material = self.pick_material(row, self.parcel_roll(id, region.roll));
        let crop = self.config.crops.get(material)?;
        let mut angle = region.angle;
        if hash01(id.x, id.y, 59) < self.plan().cross_share {
            angle += PI * 0.5;
        }
        Some(Parcel {
            site,
            material: material.to_string(),
            variant: pick_variant(&crop.variants, hash01(id.x, id.y, 61))?.clone(),
            angle,
        })
    }

    /// Footprint (m) of a parcel model: the parcel minus the headland strip.
    pub fn footprint_m(&self) -> f32 {
        let side = self.plan().parcel_px - self.plan().headland_px;
        side.max(0.01) * self.plan().footprint_fill * self.meters_per_px
    }

    /// Instances of the parcels whose site falls in cell `cell` (`render.cell_px` side): one per
    /// farmed parcel. Model id -> flat `(x, z, yaw, size_m)`.
    pub fn plan_cell(&self, cell: IVec2) -> BTreeMap<String, Vec<f32>> {
        let mut out = BTreeMap::new();
        let cell_px = if self.config.render.cell_px > 0.0 {
            self.config.render.cell_px
        } else {
            2.0
        };
        let size = self.plan().parcel_px;
        let low = Vec2::new(cell.x as f32, cell.y as f32) * cell_px;
        let first = IVec2::new(
            (low.x / size).floor() as i32 - 1,
            (low.y / size).floor() as i32 - 1,
        );
        let last = IVec2::new(
            ((low.x + cell_px) / size).floor() as i32 + 1,
            ((low.y + cell_px) / size).floor() as i32 + 1,
        );
        for gy in first.y..=last.y {
            for gx in first.x..=last.x {
                let id = IVec2::new(gx, gy);
                let site = self.parcel_site(id);
                if site.x < low.x
                    || site.y < low.y
                    || site.x >= low.x + cell_px
                    || site.y >= low.y + cell_px
                {
                    continue;
                }
                if let Some(parcel) = self.parcel(id) {
                    self.place_parcel(id, &parcel, &mut out);
                }
            }
        }
        out
    }

    /// Places the single model of the parcel: centred on its site, longest side = footprint, rows
    /// (model X axis) on the region frame, small yaw and size variation. A variant with `tile > 1`
    /// (close-up) lays `tile x tile` aligned squares over the footprint.
    fn place_parcel(&self, id: IVec2, parcel: &Parcel, out: &mut BTreeMap<String, Vec<f32>>) {
        let site = parcel.site;
        let plan = self.plan();
        let yaw = -parcel.angle + (hash01(id.x, id.y, 67) - 0.5) * 2.0 * plan.yaw_jitter;
        let grow = 1.0 + (hash01(id.x, id.y, 71) - 0.5) * 2.0 * plan.size_jitter;
        let tiles = parcel.variant.tile.max(1);
        let key = &parcel.variant.id;
        if tiles == 1 {
            append(out, key, site.x, site.y, yaw, self.footprint_m() * grow);
            return;
        }
        let step_m = self.footprint_m() * grow / tiles as f32;
        let step_px = step_m / self.meters_per_px;
        let axis_u = Vec2::new(yaw.cos(), -yaw.sin()) * step_px;
        let axis_v = Vec2::new(yaw.sin(), yaw.cos()) * step_px;
        for i in 0..tiles {
            for j in 0..tiles {
                let at = site
                    + axis_u * (i as f32 + 0.5 - 0.5 * tiles as f32)
                    + axis_v * (j as f32 + 0.5 - 0.5 * tiles as f32);
                append(out, key, at.x, at.y, yaw, step_m);
            }
        }
    }
}

fn pick_variant(variants: &[Variant], roll: f32) -> Option<&Variant> {
    let total: f32 = variants.iter().map(|v| v.weight).sum();
    let target = roll * total;
    let mut cumulative = 0.0;
    for variant in variants {
        cumulative += variant.weight;
        if target <= cumulative {
            return Some(variant);
        }
    }
    variants.last()
}

fn append(out: &mut BTreeMap<String, Vec<f32>>, key: &str, x: f32, z: f32, yaw: f32, size_m: f32) {
    out.entry(key.to_string())
        .or_default()
        .extend_from_slice(&[x, z, yaw, size_m]);
}

// --- Shared layout (vegetation_fields.gd) --------------------------------------------------

/// Parcel frames `[k (skew), fu, fv (enclosure size, map px), warp phase]`.
pub const LAYOUTS: [[f32; 4]; 2] = [[0.35, 5.0, 4.2, 0.0], [-0.8, 4.6, 3.8, 2.1]];
pub const LANDUSE_SIZE: usize = 128;
pub const LANDUSE_SAMPLES: usize = 192;

/// Slow region function: > 0 -> frame 1, otherwise frame 0.
pub fn region_value(x: f32, y: f32) -> f32 {
    (0.0131 * x + 1.3 * (0.0093 * y).sin()).sin() + (0.0117 * y + 1.1 * (0.0171 * x).sin()).sin()
        - 0.25
}

pub fn layout_at(x: f32, y: f32) -> usize {
    usize::from(region_value(x, y) > 0.0)
}

/// Soft displacement of the parcel frame (winding edges, enclosures of varied sizes).
pub fn warp(x0: f32, y0: f32, phase: f32) -> Vec2 {
    let ox = 1.7 * (0.071 * y0 + 1.9 * (0.027 * x0).sin() + phase).sin()
        + 0.55 * (0.23 * y0 + 1.3 * (0.061 * x0).sin()).sin();
    let oy = 1.7 * (0.063 * x0 + 1.7 * (0.023 * y0).sin() + phase * 1.3).sin()
        + 0.55 * (0.19 * x0 + 1.1 * (0.047 * y0).sin()).sin();
    Vec2::new(ox, oy)
}

/// Map point of the frame coordinates (u, v) of `layout`.
pub fn to_map(layout: usize, u: f32, v: f32) -> Vec2 {
    let [k, fu, fv, phase] = LAYOUTS[layout.min(1)];
    let (a, b) = (u * fu, v * fv);
    let det = 1.0 + k * k;
    let x0 = (a - k * b) / det;
    let y0 = (b + k * a) / det;
    Vec2::new(x0, y0) + warp(x0, y0, phase)
}

/// Land use of a province (`data/provinces/<id>.json`): `[vine, dryness, bocage, 1]`.
/// Vine: wine resource, press building, rarer northwards and in mountains; dryness: Mediterranean
/// climate, Iberian interior, latitude gradient; bocage: terrain "bocage".
pub fn landuse_of(province: &Value) -> [f32; 4] {
    let text = |key: &str| province.get(key).and_then(Value::as_str).unwrap_or("");
    let has = |key: &str, id: &str| {
        province
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(id)))
    };
    let climate = text("climate");
    let lat = province
        .pointer("/geo/capital_lonlat/1")
        .and_then(Value::as_f64)
        .unwrap_or(46.0) as f32;
    let mut vine = 0.0;
    if has("resources", "res_wine") {
        vine += 0.45;
    }
    if has("buildings", "bld_vineyard_press") {
        vine += 0.35;
    }
    let climate_factor = match climate {
        "mediterranean" => 1.0,
        "continental" => 0.8,
        "oceanic" => 0.75,
        "mountain" => 0.35,
        _ => 0.6,
    };
    vine *= climate_factor * (1.0 - smoothstep(47.5, 50.5, lat));
    let mut dry = if climate == "mediterranean" {
        0.85
    } else if lat < 43.5 {
        0.6
    } else {
        0.0
    };
    dry = f32::max(dry, 0.5 * (1.0 - smoothstep(42.5, 45.5, lat)));
    let bocage = f32::from(text("terrain") == "bocage");
    [vine.clamp(0.0, 1.0), dry.clamp(0.0, 1.0), bocage, 1.0]
}

/// Land-use raster (`LANDUSE_SIZE`²) of the map frame, RGBA in [0, 1]. `value_at(i, j, n)` gives
/// the land use of the province under sample (i, j) of an n x n grid (`None` at sea). Sea cells
/// take the average of neighbouring land (8 dilation passes) so that coasts are not diluted by the
/// blur, then the grid is resampled bilinearly to `LANDUSE_SIZE` (Godot: trilinear reduction).
pub fn landuse_image(value_at: impl Fn(usize, usize, usize) -> Option<[f32; 4]>) -> Vec<[f32; 4]> {
    let n = LANDUSE_SAMPLES;
    let mut cells = vec![[0.0f32; 4]; n * n];
    let mut known = vec![false; n * n];
    for j in 0..n {
        for i in 0..n {
            if let Some(v) = value_at(i, j, n) {
                cells[j * n + i] = v;
                known[j * n + i] = true;
            }
        }
    }
    for _ in 0..8 {
        let mut next = known.clone();
        for j in 0..n {
            for i in 0..n {
                if known[j * n + i] {
                    continue;
                }
                let mut sum = [0.0f32; 4];
                let mut count = 0.0;
                for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let (a, b) = (i as i32 + dx, j as i32 + dy);
                    if a >= 0 && b >= 0 && (a as usize) < n && (b as usize) < n {
                        let at = b as usize * n + a as usize;
                        if known[at] {
                            for (s, c) in sum.iter_mut().zip(cells[at]) {
                                *s += c;
                            }
                            count += 1.0;
                        }
                    }
                }
                if count > 0.0 {
                    cells[j * n + i] = sum.map(|s| s / count);
                    next[j * n + i] = true;
                }
            }
        }
        known = next;
    }
    for (cell, k) in cells.iter_mut().zip(&known) {
        if !k {
            *cell = [0.0, 0.0, 0.0, 1.0];
        }
        cell[3] = 1.0;
    }
    let m = LANDUSE_SIZE;
    let mut out = vec![[0.0f32; 4]; m * m];
    for y in 0..m {
        for x in 0..m {
            let fx = ((x as f32 + 0.5) * n as f32 / m as f32 - 0.5).clamp(0.0, (n - 1) as f32);
            let fy = ((y as f32 + 0.5) * n as f32 / m as f32 - 0.5).clamp(0.0, (n - 1) as f32);
            let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
            let (x1, y1) = ((x0 + 1).min(n - 1), (y0 + 1).min(n - 1));
            let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
            let mut px = [0.0; 4];
            for (c, slot) in px.iter_mut().enumerate() {
                let top = cells[y0 * n + x0][c] * (1.0 - tx) + cells[y0 * n + x1][c] * tx;
                let bottom = cells[y1 * n + x0][c] * (1.0 - tx) + cells[y1 * n + x1][c] * tx;
                *slot = top * (1.0 - ty) + bottom * ty;
            }
            out[y * m + x] = px;
        }
    }
    out
}

// --- Dump --------------------------------------------------------------------------------

/// Dump section `field_plan`: parcels planned around the view focus (or the map centre) and
/// land use of all provinces, from the real data.
pub fn dump(world: &bevy::ecs::world::World) -> (String, serde_json::Value) {
    use serde_json::json;
    let value = (|| {
        let loaded = world.get_resource::<super::LoadedCountryside>()?;
        let root = crate::paths::find_root().ok()?;
        let data = crate::paths::data_dir(&root);
        let read = |file: &str| std::fs::read_to_string(data.join(file)).ok();
        let config: FieldsConfig = serde_json::from_str(&read("art/dn_fields.json")?).ok()?;
        let mix = build_mix(
            &read("art/ground_biome_mix.json")?,
            &read("map/agri_landscapes.json")?,
        )
        .ok()?;
        let terrain = loaded.terrain.as_ref();
        let plan = FieldPlan {
            config: &config,
            mix: &mix,
            terrain,
            meters_per_px: DEFAULT_METERS_PER_PX,
        };
        let size = terrain.size();
        let view = world.get_resource::<super::layer::PropView>();
        let focus = view.filter(|v| v.active).map_or(size * 0.45, |v| v.focus);
        let cell_px = config.render.cell_px.max(0.5);
        let center = IVec2::new((focus.x / cell_px) as i32, (focus.y / cell_px) as i32);
        let started = std::time::Instant::now();
        let mut by_model: BTreeMap<String, usize> = BTreeMap::new();
        let mut cells = 0usize;
        for dy in -10..10 {
            for dx in -10..10 {
                cells += 1;
                for (id, records) in plan.plan_cell(center + IVec2::new(dx, dy)) {
                    *by_model.entry(id).or_default() += records.len() / STRIDE;
                }
            }
        }
        let plan_ms = started.elapsed().as_secs_f32() * 1000.0;
        let (mut provinces, mut vine, mut dry, mut bocage) = (0, 0, 0, 0);
        if let Ok(dir) = std::fs::read_dir(data.join("provinces")) {
            for entry in dir.flatten() {
                let Ok(text) = std::fs::read_to_string(entry.path()) else {
                    continue;
                };
                let Ok(province) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                let [v, d, b, _] = landuse_of(&province);
                provinces += 1;
                vine += usize::from(v > 0.0);
                dry += usize::from(d > 0.5);
                bocage += usize::from(b > 0.0);
            }
        }
        Some(json!({
            "enabled": config.enabled, "ready": plan.is_ready(),
            "crops": config.crops.len(), "mix_rows": mix.len(),
            "focus": [focus.x, focus.y], "cells_planned": cells, "plan_ms": plan_ms,
            "instances": by_model.values().sum::<usize>(), "by_model": by_model,
            "footprint_m": plan.footprint_m(),
            "dominant_at_focus": plan.dominant_material(focus),
            "landuse": {"provinces": provinces, "with_vine": vine, "dry": dry, "bocage": bocage},
        }))
    })();
    (
        "field_plan".into(),
        value.unwrap_or(serde_json::Value::Null),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Flat;

    impl Terrain for Flat {
        fn size(&self) -> Vec2 {
            Vec2::splat(200.0)
        }
        fn is_land(&self, p: Vec2) -> bool {
            p.x > 10.0
        }
        fn river_sd(&self, _: Vec2) -> f32 {
            8.0
        }
        fn in_lake(&self, _: Vec2) -> bool {
            false
        }
        fn height_m(&self, _: Vec2) -> f32 {
            100.0
        }
        fn splat(&self, _: Vec2) -> [f32; 4] {
            [0.2, 0.7, 0.05, 0.0]
        }
        fn wet(&self, _: Vec2) -> [f32; 2] {
            [0.0; 2]
        }
        fn biome(&self, _: Vec2) -> i32 {
            1
        }
        fn biome_allows(&self, _: u32, _: i32) -> bool {
            true
        }
        fn landscape_row(&self, _: Vec2) -> i32 {
            -1
        }
        fn landscape_id_row(&self, _: &str) -> i32 {
            -2
        }
        fn near_town(&self, p: Vec2, clearance: f32) -> bool {
            p.distance(Vec2::new(100.0, 100.0)) < clearance
        }
        fn villages_in(&self, _: IVec2) -> &[super::super::placement::Village] {
            &[]
        }
        fn roads_in(&self, _: IVec2) -> &[super::super::placement::RoadSeg] {
            &[]
        }
    }

    fn config() -> FieldsConfig {
        serde_json::from_value(json!({
            "enabled": true,
            "render": {"cell_px": 2.0},
            "crops": {
                "wheat_ripe": {"variants": [{"id": "props/field_wheat", "weight": 1.0}]},
                "vineyard_rows": {"variants": [{"id": "props/vine", "weight": 1.0, "tile": 2}]}
            }
        }))
        .unwrap()
    }

    fn mix() -> HashMap<i32, MixEntry> {
        build_mix(
            r#"{"biomes": {"1": {"farm": 1.0, "crops": {"wheat_ripe": 2.0, "vineyard_rows": 1.0}}}}"#,
            r#"{"landscapes": {"a": {"crops": {"x": 1}}, "b": {"crops": {"y": 1}}}}"#,
        )
        .unwrap()
    }

    #[test]
    fn hash_is_stable_unit_and_signed_safe() {
        assert_eq!(hash01(3, 5, 7), hash01(3, 5, 7));
        for (a, b) in [(-5, 9), (0, 0), (123456, -98765), (i32::MIN, i32::MAX)] {
            let h = hash01(a, b, 41);
            assert!((0.0..1.0).contains(&h), "{h}");
        }
        assert_ne!(hash01(1, 2, 3), hash01(2, 1, 3));
    }

    #[test]
    fn mix_rows_keep_file_order_and_landscape_rows() {
        let rows = mix();
        assert_eq!(rows[&1].crops.0[0].0, "wheat_ripe");
        assert_eq!(rows[&1].crops.0[1].0, "vineyard_rows");
        assert!(rows.contains_key(&FIRST_LANDSCAPE_ROW));
        assert!(rows.contains_key(&(FIRST_LANDSCAPE_ROW + 1)));
    }

    #[test]
    fn material_draw_follows_cumulative_weights() {
        let (config, mix) = (config(), mix());
        let plan = FieldPlan {
            config: &config,
            mix: &mix,
            terrain: &Flat,
            meters_per_px: DEFAULT_METERS_PER_PX,
        };
        assert_eq!(plan.pick_material(1, 0.1), "wheat_ripe");
        assert_eq!(plan.pick_material(1, 0.9), "vineyard_rows");
        assert_eq!(plan.pick_material(99, 0.5), "");
    }

    #[test]
    fn cells_are_deterministic_and_cover_each_parcel_once() {
        let (config, mix) = (config(), mix());
        let plan = FieldPlan {
            config: &config,
            mix: &mix,
            terrain: &Flat,
            meters_per_px: DEFAULT_METERS_PER_PX,
        };
        let mut total = 0usize;
        let mut ids = std::collections::HashSet::new();
        for cy in 0..20 {
            for cx in 0..20 {
                let cell = IVec2::new(cx, cy);
                let first = plan.plan_cell(cell);
                assert_eq!(first, plan.plan_cell(cell));
                total += first.values().map(|v| v.len() / STRIDE).sum::<usize>();
                ids.extend(first.keys().cloned());
                for records in first.values() {
                    for r in records.chunks(STRIDE) {
                        assert!(r[3] > 0.0);
                    }
                }
            }
        }
        // 40 x 40 px at 0.8 px/parcel: 2500 parcels, ~65 % farmed, tiled vines count 4 each.
        assert!(total > 1000, "{total}");
        assert!(ids.contains("props/field_wheat"));
        assert!(ids.contains("props/vine"));
    }

    #[test]
    fn no_parcel_on_sea_or_in_town() {
        let (config, mix) = (config(), mix());
        let plan = FieldPlan {
            config: &config,
            mix: &mix,
            terrain: &Flat,
            meters_per_px: DEFAULT_METERS_PER_PX,
        };
        for gy in -2..60 {
            for gx in -2..60 {
                if let Some(parcel) = plan.parcel(IVec2::new(gx, gy)) {
                    assert!(parcel.site.x > 10.0);
                    assert!(parcel.site.distance(Vec2::new(100.0, 100.0)) >= 0.4);
                }
            }
        }
    }

    #[test]
    fn layout_is_two_regions_and_to_map_is_near_identity_scale() {
        let mut seen = [false; 2];
        for i in 0..80 {
            for j in 0..80 {
                seen[layout_at(i as f32 * 90.0, j as f32 * 90.0)] = true;
            }
        }
        assert_eq!(seen, [true, true]);
        let p = to_map(0, 0.0, 0.0);
        assert!(p.length() < 4.0, "{p}");
    }

    #[test]
    fn landuse_follows_province_data() {
        let vine = landuse_of(&json!({
            "climate": "mediterranean", "terrain": "plain",
            "resources": ["res_wine"], "buildings": ["bld_vineyard_press"],
            "geo": {"capital_lonlat": [4.8, 43.0]}}));
        assert!((vine[0] - 0.8).abs() < 1e-5);
        assert!(vine[1] >= 0.85);
        let bocage = landuse_of(&json!({"climate": "oceanic", "terrain": "bocage",
            "geo": {"capital_lonlat": [-1.0, 48.5]}}));
        assert_eq!(bocage[2], 1.0);
        assert_eq!(bocage[0], 0.0);
        let north = landuse_of(&json!({"resources": ["res_wine"],
            "geo": {"capital_lonlat": [0.0, 52.0]}}));
        assert_eq!(north[0], 0.0);
    }

    #[test]
    fn landuse_raster_dilates_into_the_sea() {
        let image = landuse_image(|i, _j, n| (i < n / 2).then_some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(image.len(), LANDUSE_SIZE * LANDUSE_SIZE);
        assert!(image[10 * LANDUSE_SIZE + 5][0] > 0.99);
        // Just past the coast: still land colour after dilation; far sea: black.
        assert!(image[10 * LANDUSE_SIZE + LANDUSE_SIZE / 2 + 3][0] > 0.9);
        assert!(image[10 * LANDUSE_SIZE + LANDUSE_SIZE - 2][0] < 0.01);
        assert!(image.iter().all(|p| p[3] == 1.0));
    }
}
