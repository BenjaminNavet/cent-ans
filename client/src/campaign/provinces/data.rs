//! Province data of the campaign map: `provinces.geojson` (via `data_model::map_geo`) and the
//! province-id raster `province_ids.png` (RGB8, index = `R | G << 8`, 0 = sea / off map), same
//! contract as `game/scripts/map/map_data.gd`. Index = position (1-based) of the feature, unless
//! the feature carries an explicit `index`.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use data_model::map_geo::{self, ProvinceFeature};

use super::projection::MapProjection;

/// Provinces, raster and projection, loaded once when the campaign screen opens.
#[derive(Resource)]
pub struct ProvinceMap {
    pub width: u32,
    pub height: u32,
    pub projection: MapProjection,
    ids: Vec<u16>,
    provinces: Vec<ProvinceFeature>,
    slot_by_index: HashMap<u16, usize>,
    index_by_id: HashMap<String, u16>,
}

impl ProvinceMap {
    /// Loads `<map_dir>/{map.json, provinces.geojson, province_ids.png}`.
    pub fn load(map_dir: &Path) -> Result<Self, String> {
        let projection = MapProjection::load(map_dir)?;
        let bytes = std::fs::read(map_dir.join("provinces.geojson"))
            .map_err(|e| format!("provinces.geojson: {e}"))?;
        let provinces = map_geo::parse_provinces(&bytes).map_err(|e| e.to_string())?;
        let (width, height, ids) = load_ids_png(&map_dir.join("province_ids.png"))?;
        if [width, height] != projection.size_px() {
            return Err(format!(
                "province_ids.png is {width}x{height}, map.json says {:?}",
                projection.size_px()
            ));
        }
        Ok(Self::from_parts(width, height, projection, ids, provinces))
    }

    /// Assembles a map from decoded parts (tests build tiny ones).
    pub fn from_parts(
        width: u32,
        height: u32,
        projection: MapProjection,
        ids: Vec<u16>,
        provinces: Vec<ProvinceFeature>,
    ) -> Self {
        let slot_by_index = provinces
            .iter()
            .enumerate()
            .map(|(slot, p)| (p.index as u16, slot))
            .collect();
        let index_by_id = provinces
            .iter()
            .map(|p| (p.id.clone(), p.index as u16))
            .collect();
        Self {
            width,
            height,
            projection,
            ids,
            provinces,
            slot_by_index,
            index_by_id,
        }
    }

    pub fn provinces(&self) -> &[ProvinceFeature] {
        &self.provinces
    }

    pub fn province(&self, index: u16) -> Option<&ProvinceFeature> {
        self.slot_by_index.get(&index).map(|s| &self.provinces[*s])
    }

    /// Raster index of a province id (0 if unknown).
    pub fn index_of_id(&self, id: &str) -> u16 {
        self.index_by_id.get(id).copied().unwrap_or(0)
    }

    /// Highest raster index (size of the per-province tables, which are `1..=max_index`).
    pub fn max_index(&self) -> u16 {
        self.provinces
            .iter()
            .map(|p| p.index as u16)
            .max()
            .unwrap_or(0)
    }

    /// Province index at a map position in pixels (0 = sea / outside).
    pub fn index_at(&self, x: f32, y: f32) -> u16 {
        if x < 0.0 || y < 0.0 {
            return 0;
        }
        let (px, py) = (x as u32, y as u32);
        if px >= self.width || py >= self.height {
            return 0;
        }
        self.ids[(py * self.width + px) as usize]
    }

    /// Province index at a world position (`X = x`, `Z = y`, see `projection.rs`).
    pub fn index_at_world(&self, world: Vec3) -> u16 {
        self.index_at(world.x, world.z)
    }

    /// Raw raster, row-major, for the fill texture.
    pub fn raster(&self) -> &[u16] {
        &self.ids
    }
}

/// Decodes `province_ids.png` into `(width, height, index per pixel)`.
pub fn load_ids_png(path: &Path) -> Result<(u32, u32, Vec<u16>), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err("province_ids.png must be 8-bit".into());
    }
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => {
            return Err(format!(
                "province_ids.png: unsupported colour type {other:?}"
            ));
        }
    };
    let ids = buffer[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|px| u16::from(px[0]) | (u16::from(px[1]) << 8))
        .collect();
    Ok((info.width, info.height, ids))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::OnceLock;

    /// The real map, loaded once for all tests (the raster is 43 M pixels).
    pub(crate) fn real_map() -> &'static ProvinceMap {
        static MAP: OnceLock<ProvinceMap> = OnceLock::new();
        MAP.get_or_init(|| {
            let root = crate::paths::find_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
                .expect("repository root");
            ProvinceMap::load(&root.join("data/map")).expect("map data")
        })
    }

    #[test]
    fn loads_all_provinces_and_raster() {
        let map = real_map();
        assert_eq!(map.provinces().len(), 213);
        assert_eq!((map.width, map.height), (7080, 6104));
        let paris = map.index_of_id("prov_ile_de_france");
        assert_eq!(paris, 79);
        assert_eq!(map.province(paris).unwrap().name, "Île-de-France");
    }

    #[test]
    fn centroid_pixels_belong_to_their_province_or_neighbour() {
        let map = real_map();
        let hits = map
            .provinces()
            .iter()
            .filter(|p| map.index_at(p.capital_px[0], p.capital_px[1]) == p.index as u16)
            .count();
        assert!(
            hits * 10 >= map.provinces().len() * 9,
            "{hits} capitals inside their province"
        );
    }

    #[test]
    fn paris_lonlat_lands_in_ile_de_france() {
        let map = real_map();
        let px = map
            .projection
            .lonlat_to_px(2.35, 48.85)
            .expect("on the map");
        assert_eq!(
            map.index_at(px.x, px.y),
            map.index_of_id("prov_ile_de_france")
        );
    }

    #[test]
    fn out_of_map_is_sea() {
        let map = real_map();
        assert_eq!(map.index_at(-1.0, 5.0), 0);
        assert_eq!(map.index_at(1e9, 5.0), 0);
    }
}
