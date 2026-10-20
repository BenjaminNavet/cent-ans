//! Loading of the vegetation inputs, off the main thread: map rasters (heightmap bytes for the
//! scatter, river bed, splat forest / crops, conifer share, biomes, land mask, wetlands), the
//! species table (`data/art/tree_species.json`, mirror of `tree_species.gd`), the forest stands
//! (`forest_stands.json`, mirror of `forest_stands.gd`) and the clearings of the settlements.
//!
//! Rendering support only, no game rule.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;
use vegetation::species::{BIOME_COUNT, BIOME_STRIDE, ROLE_COUNT};
use vegetation::stands::{StandRegion, StandTable};
use vegetation::{Distribution, MapRasters, SpeciesTable};

use crate::campaign::terrain::relief_floor::HeightGrid;

/// Role names, in the order of `ROLE_*` of the crate.
const ROLE_NAMES: [&str; ROLE_COUNT] = [
    "massif",
    "lisiere",
    "isole",
    "verger",
    "ripisylve",
    "garrigue",
];
/// `TreeSpecies.BIOME_KEYS`.
const BIOME_KEYS: [&str; BIOME_STRIDE] = [
    "forest",
    "isolated",
    "grove",
    "orchard",
    "orchard_ring_px",
    "orchard_open",
    "riparian",
    "riparian_px",
    "scrub",
];

/// Generalised tree style constants of `MapPropScale` (`tree_style: generalised`, ADR 0161).
pub mod style {
    pub const TREE_HEIGHT: f64 = 1.5;
    pub const REFERENCE_HEIGHT: f64 = 1.4;
    pub const CROWN_RATIO: f64 = 0.6;
    pub const CROWN_CLEARANCE: f64 = 1.0;
    pub const SPACING: f64 = 1.3;
    pub const HEDGE_BOOST: f64 = 6.0;
    /// Widening of the grown crowns (`generalised_crown_widen`).
    pub const CROWN_WIDEN: f32 = 1.25;
    /// Height of the edge trees (`generalised_edge_height`) and share removed (`edge_thin`).
    pub const EDGE_HEIGHT: f32 = 0.85;
    pub const GROVE_LOW: f32 = 0.28;
    pub const GROVE_HIGH: f32 = 0.42;
    pub const GROVE_CORE: f64 = 0.55;
    /// Clearing radius of a settlement (map px) before the crown radius is added.
    pub const SETTLEMENT_CLEARING: f64 = 6.0;
    /// Clearing radius of a province capital (`Vegetation.build`).
    pub const CAPITAL_CLEARING: f64 = 11.0;
    /// Clearing radius of a hamlet (`ModelLibrary.HAMLET_SCALE` 3.6 x 0.6).
    pub const HAMLET_CLEARING: f64 = 2.16;

    pub fn scale() -> f64 {
        (TREE_HEIGHT / REFERENCE_HEIGHT).clamp(1e-4, 4.0)
    }
    pub fn crown_radius() -> f64 {
        TREE_HEIGHT * CROWN_RATIO * CROWN_CLEARANCE
    }
}

/// An 8-bit raster, `ch` interleaved channels.
#[derive(Default)]
pub struct Plane {
    pub w: usize,
    pub h: usize,
    pub ch: usize,
    pub data: Vec<u8>,
}

impl Plane {
    pub fn empty() -> Plane {
        Plane {
            w: 0,
            h: 0,
            ch: 1,
            data: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Nearest texel of channel `c` at the map point (x, y), the raster covering `map` px.
    pub fn texel(&self, map: (f64, f64), x: f64, y: f64, c: usize) -> f32 {
        if self.data.is_empty() {
            return 0.0;
        }
        let px = ((x * self.w as f64 / map.0) as i64).clamp(0, self.w as i64 - 1) as usize;
        let py = ((y * self.h as f64 / map.1) as i64).clamp(0, self.h as i64 - 1) as usize;
        self.data[(py * self.w + px) * self.ch + c.min(self.ch - 1)] as f32 / 255.0
    }
}

/// Reads a PNG as 8 bits per channel (palette expanded, 16 bits stripped).
pub fn read_png(path: &Path) -> Result<Plane, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let (w, h) = (reader.info().width as usize, reader.info().height as usize);
    let mut data = vec![0u8; reader.output_buffer_size().unwrap_or(w * h * 4)];
    let frame = reader
        .next_frame(&mut data)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    data.truncate(frame.buffer_size());
    let ch = frame.color_type.samples();
    Ok(Plane { w, h, ch, data })
}

/// A wetland site (`wetland_sites_px.json`).
#[derive(Clone, Debug)]
pub struct WetSite {
    pub id: String,
    pub kind: String,
    pub px: (f64, f64),
    pub radius_px: f64,
}

/// Everything the scatter workers need, shared by `Arc`.
pub struct VegetationWorld {
    pub heights: Arc<HeightGrid>,
    pub map: Arc<MapRasters>,
    pub species: Arc<SpeciesTable>,
    pub species_ids: Vec<String>,
    pub dn_ids: Vec<String>,
    /// Mean width / height ratio of every species (mesh normalisation).
    pub width_ratio: Vec<f32>,
    pub size: (f64, f64),
    pub meters_per_px: f32,
    pub splat_forest: Plane,
    pub splat_crops: Plane,
    pub conifer: Plane,
    pub biomes: Plane,
    pub land: Plane,
    /// R marsh, G ponds, B wet meadows (`wetlands.png`).
    pub wet: Plane,
    pub exclusions: Vec<(f64, f64, f64)>,
    /// Lakes, sea and main roads (`clearance.rs`).
    pub clearance: super::clearance::Clearance,
    /// Reed and pond decor of the wetland sites (`reeds.rs`).
    pub wetlands: super::reeds::Wetlands,
    pub wet_sites: Vec<WetSite>,
    pub cover: (f32, f32),
    pub beech: BeechParams,
    pub default_biome: usize,
    pub stand_count: usize,
    /// Forest look (`campaign_forest_look.json`, lot TS-T2).
    pub look: super::look::ForestLook,
    pub load_ms: f32,
}

/// `forest_cover.json` `beech` block.
#[derive(Clone, Copy, Debug)]
pub struct BeechParams {
    pub altitude_low_m: f32,
    pub altitude_high_m: f32,
    pub patch_px: f32,
    pub south_oak: f32,
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn f(v: &Value, default: f64) -> f64 {
    v.as_f64().unwrap_or(default)
}

fn pair(v: &Value, default: (f64, f64)) -> (f64, f64) {
    match v.as_array() {
        Some(a) if a.len() >= 2 => (f(&a[0], default.0), f(&a[1], default.1)),
        _ => default,
    }
}

/// Value of `table[b]`, else that of the nearest ancestor that has one (`TreeSpecies._inherited`).
fn inherited<'a>(table: &'a Value, b: usize, parents: &[i32]) -> Option<&'a Value> {
    let mut current = b;
    for _ in 0..3 {
        if let Some(v) = table.get(current.to_string()) {
            return Some(v);
        }
        let parent = *parents.get(current)? as usize;
        if parent == current {
            break;
        }
        current = parent;
    }
    None
}

/// Species table, species ids, glb ids and mean width ratios.
pub type SpeciesData = (SpeciesTable, Vec<String>, Vec<String>, Vec<f32>);

/// `TreeSpecies.load_dict` with the generalised-style gains of `Vegetation._generalised_species`
/// (all 1 except `hedge_boost` and `grove_core`).
pub fn species_table(data: &Value, parents: &[i32]) -> Result<SpeciesData, String> {
    let list = data["species"].as_array().ok_or("species list missing")?;
    let count = list.len();
    if count < 3 {
        return Err("fewer than 3 species".into());
    }
    let mut table = SpeciesTable {
        count,
        base: vec![0.0; BIOME_COUNT * ROLE_COUNT * count],
        alt_lo: vec![0.0; count],
        alt_hi: vec![0.0; count],
        river: vec![0.0; count],
        conifer: vec![0.0; count],
        kind: vec![0; count],
        season: vec![0; count],
        height: vec![0.0; 2 * count],
        width: vec![0.0; 2 * count],
        biome_params: vec![0.0; BIOME_COUNT * BIOME_STRIDE],
        biome_parent: parents.to_vec(),
        ..Default::default()
    };
    let mut ids = Vec::with_capacity(count);
    let mut dn_ids = Vec::with_capacity(count);
    let mut ratios = Vec::with_capacity(count);
    for (s, sp) in list.iter().enumerate() {
        ids.push(sp["id"].as_str().unwrap_or("").to_string());
        dn_ids.push(sp["dn_id"].as_str().unwrap_or("").to_string());
        let alt = pair(&sp["altitude_m"], (0.0, 9000.0));
        table.alt_lo[s] = alt.0 as f32;
        table.alt_hi[s] = alt.1 as f32;
        table.river[s] = f(&sp["river"], 0.0) as f32;
        table.kind[s] = match sp["mesh"].as_str().unwrap_or("oak") {
            "beech" => 1,
            "conifer" => 2,
            _ => 0,
        };
        table.conifer[s] = if table.kind[s] == 2 { 1.0 } else { 0.0 };
        table.season[s] = f(&sp["season_class"], 0.0) as i32;
        let h = pair(&sp["height"], (1.1, 1.7));
        let w = pair(&sp["width"], (1.0, 1.1));
        table.height[2 * s] = h.0 as f32;
        table.height[2 * s + 1] = h.1 as f32;
        table.width[2 * s] = w.0 as f32;
        table.width[2 * s + 1] = w.1 as f32;
        ratios.push(((w.0 + w.1) * 0.5) as f32);
        for b in 0..BIOME_COUNT {
            let wb = inherited(&sp["biomes"], b, parents).map_or(0.0, |v| f(v, 0.0));
            if wb <= 0.0 {
                continue;
            }
            for (r, role) in ROLE_NAMES.iter().enumerate() {
                table.base[(b * ROLE_COUNT + r) * count + s] =
                    (wb * f(&sp["roles"][*role], 0.0)) as f32;
            }
        }
    }
    for b in 1..BIOME_COUNT {
        let Some(cfg) = inherited(&data["biomes"], b, parents) else {
            continue;
        };
        for (k, key) in BIOME_KEYS.iter().enumerate() {
            table.biome_params[b * BIOME_STRIDE + k] = f(&cfg[*key], 0.0) as f32;
        }
    }
    let d = &data["distribution"];
    let dflt = Distribution::default();
    table.dist = Distribution {
        massif_core: f(&d["massif_core"], dflt.massif_core),
        massif_fill: f(&d["massif_fill"], dflt.massif_fill),
        edge_fill: f(&d["edge_fill"], dflt.edge_fill),
        stand_px: f(&d["stand_px"], dflt.stand_px),
        stand_share: f(&d["stand_share"], dflt.stand_share),
        altitude_fade_m: f(&d["altitude_fade_m"], dflt.altitude_fade_m),
        river_reach_px: f(&d["river_reach_px"], dflt.river_reach_px),
        conifer_raster: f(&d["conifer_raster"], dflt.conifer_raster),
        orchard_parcel_px: f(&d["orchard_parcel_px"], dflt.orchard_parcel_px),
        orchard_fill: f(&d["orchard_fill"], dflt.orchard_fill),
        hedge_boost: style::HEDGE_BOOST,
        grove_core: style::GROVE_CORE,
        hedge_tree: f(&d["hedge_tree"], dflt.hedge_tree),
        village_boost: f(&d["village_boost"], dflt.village_boost),
        default_biome: f(&d["default_biome"], dflt.default_biome as f64) as usize,
    };
    if !table.is_valid() {
        return Err("species table invalid".into());
    }
    Ok((table, ids, dn_ids, ratios))
}

/// `ForestStands.table`: the stand types and the named massifs (ellipses in map px).
pub fn stand_table(
    stands: &Value,
    forests: &Value,
    species_ids: &[String],
    meta: Arc<data_model::MapMeta>,
    parents: &[i32],
) -> Option<StandTable> {
    let list = stands["stands"].as_array()?;
    if list.is_empty() {
        return None;
    }
    let n = list.len();
    let count = species_ids.len();
    let dist = &stands["distribution"];
    let other = f(&dist["other_mult"], 0.05);
    let mut table = StandTable {
        count: n,
        species_count: count,
        biome_parent: parents.to_vec(),
        cell_px: f(&dist["cell_px"], 36.0),
        jitter: f(&dist["jitter"], 0.8),
        projection: Some(meta.clone()),
        ..Default::default()
    };
    let mut index = std::collections::HashMap::new();
    for (i, st) in list.iter().enumerate() {
        index.insert(st["id"].as_str().unwrap_or("").to_string(), i);
        for id in species_ids {
            table.mult.push(f(&st["species"][id], other) as f32);
        }
        let t = st["tint"].as_array();
        for k in 0..3 {
            table
                .tint
                .push(t.and_then(|a| a.get(k)).map_or(1.0, |v| f(v, 1.0)) as f32);
        }
        table.density.push(f(&st["density"], 1.0) as f32);
        table.height.push(f(&st["height"], 1.0) as f32);
        let mut mask = 0i32;
        for b in st["biomes"].as_array().into_iter().flatten() {
            mask |= 1 << (f(b, 0.0) as i32).clamp(0, 30);
        }
        table.biome_mask.push(mask);
        let ranges = [
            ("altitude_m", (0.0, 9000.0)),
            ("lat", (-90.0, 90.0)),
            ("lon", (-180.0, 180.0)),
            ("conifer", (0.0, 1.0)),
        ];
        for (key, dflt) in ranges {
            let (a, b) = pair(&st[key], dflt);
            let target = match key {
                "altitude_m" => &mut table.altitude,
                "lat" => &mut table.lat,
                "lon" => &mut table.lon,
                _ => &mut table.conifer,
            };
            target.push(a as f32);
            target.push(b as f32);
        }
        table.weight.push(f(&st["weight"], 1.0) as f32);
    }
    let mut areas = std::collections::HashMap::new();
    for a in forests["areas"].as_array().into_iter().flatten() {
        areas.insert(
            a["id"].as_str().unwrap_or("").to_string(),
            a["ellipse"].clone(),
        );
    }
    let mpp = meta.meters_per_px;
    for r in stands["regions"].as_array().into_iter().flatten() {
        let Some(&stand) = index.get(r["stand"].as_str().unwrap_or("")) else {
            continue;
        };
        let ellipse = match r["area"].as_str() {
            Some(area) => areas.get(area).cloned().unwrap_or(Value::Null),
            None => r["ellipse"].clone(),
        };
        if ellipse.is_null() {
            continue;
        }
        let c = pair(&ellipse["center"], (0.0, 0.0));
        let radii = pair(&ellipse["radii_km"], (1.0, 1.0));
        let Some(centre) = meta.lonlat_to_px(c.0, c.1) else {
            continue;
        };
        let rx = radii.0 * 1000.0 / mpp;
        let ry = radii.1 * 1000.0 / mpp;
        let angle = -f(&ellipse["angle_deg"], 0.0).to_radians();
        table.regions.push(StandRegion {
            cx: centre[0],
            cy: centre[1],
            rx,
            ry,
            angle,
            stand,
            priority: f(&r["priority"], 0.0) + 1000.0 / (rx * ry).max(1.0),
        });
    }
    table.is_valid(count).then_some(table)
}

/// Loads every vegetation input. `heights` is the world heightmap of the terrain lot.
pub fn load(root: &Path, heights: Arc<HeightGrid>) -> Result<VegetationWorld, String> {
    let started = std::time::Instant::now();
    let map_dir = root.join("data/map");
    let art = root.join("data/art");
    let meta = Arc::new(data_model::MapMeta::load(&map_dir)?);
    let size = (heights.width as f64, heights.height as f64);

    let parents: Vec<i32> = read_json(&map_dir.join("biome_parents.json"))
        .ok()
        .and_then(|v| {
            v["parents"]
                .as_array()
                .map(|a| a.iter().map(|x| x.as_i64().unwrap_or(0) as i32).collect())
        })
        .unwrap_or_default();
    let species_json = read_json(&art.join("tree_species.json"))?;
    let (mut species, species_ids, dn_ids, width_ratio) = species_table(&species_json, &parents)?;
    let default_biome = species.dist.default_biome;
    if let (Ok(stands), Ok(forests)) = (
        read_json(&art.join("forest_stands.json")),
        read_json(&map_dir.join("historical_forests.json")),
    ) && let Some(table) = stand_table(&stands, &forests, &species_ids, meta.clone(), &parents)
    {
        species.stands = table;
    }
    let stand_count = species.stands.count;

    // Heightmap bytes for the scatter (little-endian 16 bits).
    let mut bytes = Vec::with_capacity(heights.data.len() * 2);
    for v in &heights.data {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    let river = read_png(&map_dir.join("river_bed.png")).unwrap_or_else(|_| Plane::empty());
    let river_plane = Plane {
        w: river.w,
        h: river.h,
        ch: river.ch,
        data: river.data.clone(),
    };
    let map = MapRasters {
        height: bytes,
        width: heights.width,
        height_px: heights.height,
        bpp: 2,
        little_endian: true,
        height_min_m: heights.h_min_m as f64,
        height_max_m: (heights.h_min_m + heights.h_range_m) as f64,
        river_width: river.w,
        river_height: river.h,
        river: if river.ch == 1 {
            river.data
        } else {
            Vec::new()
        },
    };

    // Cover and conifer configuration (`forest_cover.json`).
    let cover_cfg = read_json(&map_dir.join("forest_cover.json")).unwrap_or(Value::Null);
    let cover = (
        f(&cover_cfg["cover"]["low"], 0.22) as f32,
        f(&cover_cfg["cover"]["high"], 0.68) as f32,
    );
    let channel = match cover_cfg["cover"]["channel"].as_str().unwrap_or("b") {
        "r" => 0,
        "g" => 1,
        "a" => 3,
        _ => 2,
    };
    let beech = BeechParams {
        altitude_low_m: f(&cover_cfg["beech"]["altitude_low_m"], 150.0) as f32,
        altitude_high_m: f(&cover_cfg["beech"]["altitude_high_m"], 750.0) as f32,
        patch_px: f(&cover_cfg["beech"]["patch_px"], 22.0) as f32,
        south_oak: f(&cover_cfg["beech"]["south_oak"], 0.35) as f32,
    };
    let splat = read_png(&map_dir.join("splat.png"))?;
    if splat.ch < 3 {
        return Err("splat.png has fewer than 3 channels".into());
    }
    // Keep only forest (cover channel) and crops (G + 0.6 R) planes: 2 bytes per pixel.
    let mut forest = Vec::with_capacity(splat.w * splat.h);
    let mut crops = Vec::with_capacity(splat.w * splat.h);
    for px in splat.data.chunks_exact(splat.ch) {
        forest.push(px[channel.min(splat.ch - 1)]);
        crops.push((px[1] as f32 + 0.6 * px[0] as f32).min(255.0) as u8);
    }
    let (sw, sh) = (splat.w, splat.h);
    drop(splat);
    let plane = |data| Plane {
        w: sw,
        h: sh,
        ch: 1,
        data,
    };
    let conifer = match cover_cfg["conifer"]["file"].as_str() {
        Some(file) => read_png(&map_dir.join(file)).unwrap_or_else(|_| Plane::empty()),
        None => Plane::empty(),
    };
    let biomes = read_png(&map_dir.join("biomes.png")).unwrap_or_else(|_| Plane::empty());
    let land = read_png(&map_dir.join("land_mask.png")).unwrap_or_else(|_| Plane::empty());
    let wet = read_png(&map_dir.join("wetlands.png")).unwrap_or_else(|_| Plane::empty());

    // Clearings: settlements (+ crown radius, as `Vegetation.build`).
    let crown = style::crown_radius();
    let mut exclusions = Vec::new();
    if let Ok(Value::Object(places)) = read_json(&map_dir.join("settlements_px.json")) {
        for (id, v) in &places {
            if id.starts_with("set_") {
                let (x, y) = pair(v, (-1.0, -1.0));
                if x >= 0.0 {
                    exclusions.push((x, y, style::SETTLEMENT_CLEARING + crown));
                }
            }
        }
    }
    // Hamlets (`SettlementLayer.vegetation_exclusions`: HAMLET_SCALE x 0.6) and the clearing
    // around the capital of every province (`Vegetation.build`: 11 px).
    if let Ok(Value::Array(hamlets)) = read_json(&map_dir.join("hamlets.json")) {
        for hamlet in &hamlets {
            let (x, y) = pair(&hamlet["px"], (-1.0, -1.0));
            if x >= 0.0 {
                exclusions.push((x, y, style::HAMLET_CLEARING + crown));
            }
        }
    }
    if let Ok(bytes) = std::fs::read(map_dir.join("provinces.geojson"))
        && let Ok(provinces) = data_model::map_geo::parse_provinces(&bytes)
    {
        for province in &provinces {
            exclusions.push((
                province.capital_px[0] as f64,
                province.capital_px[1] as f64,
                style::CAPITAL_CLEARING + crown,
            ));
        }
    }
    let clearance = super::clearance::Clearance::load(
        &map_dir,
        size,
        crown as f32,
        read_png(&map_dir.join("coast_dist.png")).unwrap_or_else(|_| Plane::empty()),
        if river_plane.ch == 1 {
            river_plane
        } else {
            Plane::empty()
        },
    );
    let mut wet_sites = Vec::new();
    if let Ok(v) = read_json(&map_dir.join("wetland_sites_px.json"))
        && let Some(sites) = v["sites"].as_object()
    {
        for (id, s) in sites {
            let (x, y) = pair(&s["px"], (0.0, 0.0));
            wet_sites.push(WetSite {
                id: id.clone(),
                kind: s["kind"].as_str().unwrap_or("").to_string(),
                px: (x, y),
                radius_px: f(&s["radius_px"], 0.0),
            });
        }
    }

    Ok(VegetationWorld {
        heights,
        map: Arc::new(map),
        species: Arc::new(species),
        species_ids,
        dn_ids,
        width_ratio,
        size,
        meters_per_px: meta.meters_per_px as f32,
        splat_forest: plane(forest),
        splat_crops: plane(crops),
        conifer,
        biomes,
        land,
        wet,
        exclusions,
        clearance,
        wetlands: super::reeds::Wetlands::load(&map_dir),
        wet_sites,
        cover,
        beech,
        default_biome,
        stand_count,
        look: super::look::ForestLook::load(),
        load_ms: started.elapsed().as_secs_f32() * 1000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        crate::paths::find_root().expect("repository root")
    }

    #[test]
    fn species_table_of_the_repository_is_valid() {
        let root = repo_root();
        let json = read_json(&root.join("data/art/tree_species.json")).unwrap();
        let parents: Vec<i32> =
            read_json(&root.join("data/map/biome_parents.json")).unwrap()["parents"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as i32)
                .collect();
        let (table, ids, dn_ids, ratios) = species_table(&json, &parents).unwrap();
        assert!(table.is_valid());
        assert_eq!(ids.len(), table.count);
        assert_eq!(dn_ids.len(), table.count);
        assert_eq!(ratios.len(), table.count);
        assert_eq!(ids[0], "oak");
        assert_eq!(table.dist.hedge_boost, style::HEDGE_BOOST);
        // Oak is drawn in the oceanic biome as a massif tree.
        assert!(table.base[ROLE_COUNT * table.count] > 0.0);
    }

    #[test]
    fn plane_texel_maps_the_map_onto_the_raster() {
        let plane = Plane {
            w: 2,
            h: 2,
            ch: 1,
            data: vec![0, 255, 0, 0],
        };
        assert_eq!(plane.texel((10.0, 10.0), 9.0, 1.0, 0), 1.0);
        assert_eq!(plane.texel((10.0, 10.0), 1.0, 1.0, 0), 0.0);
    }
}
