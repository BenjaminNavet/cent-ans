//! Map metadata and province geometry written by the `tools/geo` pipeline
//! into `data/map/` (see `docs/design/m1-campaign-map.md`, section 2).
//!
//! These files are produced by a tool rather than edited by hand, so unknown
//! keys are kept in `extra` instead of being rejected.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::ProvinceId;

/// Contents of `data/map/map.json`: projection and raster geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapMeta {
    /// Coordinate reference system, e.g. `"EPSG:3035"`.
    pub crs: String,
    /// `[minx, miny, maxx, maxy]` of the projected extent.
    pub bounds_projected: [f64; 4],
    /// `[width, height]` of the rasters in pixels.
    pub size_px: [u32; 2],
    pub meters_per_px: f64,
    /// Altitude encoded by a heightmap value of 0.
    pub height_min_m: f64,
    /// Altitude encoded by the maximum heightmap value.
    pub height_max_m: f64,
    /// Any other key the pipeline may add.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl MapMeta {
    /// Map-pixel position of a longitude/latitude (degrees): EPSG:3035 projection
    /// ([`laea_3035`]) then `x = (E − minx) / m`, `y = (maxy − N) / m` (convention of `tools/geo`,
    /// ADR 0086). Another CRS falls back to a linear mapping of `extent_lonlat` over `size_px`
    /// (synthetic test maps); `None` without either.
    pub fn lonlat_to_px(&self, lon: f64, lat: f64) -> Option<[f64; 2]> {
        if self.crs == "EPSG:3035" && self.meters_per_px > 0.0 {
            let (e, n) = laea_3035(lon, lat);
            let [minx, _, _, maxy] = self.bounds_projected;
            return Some([
                (e - minx) / self.meters_per_px,
                (maxy - n) / self.meters_per_px,
            ]);
        }
        let extent: Vec<f64> = self
            .extra
            .get("extent_lonlat")?
            .as_array()?
            .iter()
            .filter_map(Value::as_f64)
            .collect();
        let [w, s, e, n] = <[f64; 4]>::try_from(extent).ok()?;
        Some([
            (lon - w) / (e - w) * f64::from(self.size_px[0]),
            (n - lat) / (n - s) * f64::from(self.size_px[1]),
        ])
    }
}

/// GRS80 semi-major axis and flattening (ETRS89).
const GRS80_A: f64 = 6_378_137.0;
const GRS80_F: f64 = 1.0 / 298.257_222_101;
/// EPSG:3035 (ETRS89-extended / LAEA Europe): centre 52° N 10° E, false easting and northing.
const LAEA_LAT0: f64 = 52.0;
const LAEA_LON0: f64 = 10.0;
const LAEA_FALSE_EASTING: f64 = 4_321_000.0;
const LAEA_FALSE_NORTHING: f64 = 3_210_000.0;

/// EPSG:3035 easting and northing (metres) of a longitude/latitude (degrees): ellipsoidal
/// Lambert azimuthal equal-area, oblique aspect (Snyder, *Map Projections — A Working Manual*,
/// eq. 3-12, 24-17 to 24-19); agrees with PROJ to a few millimetres.
pub fn laea_3035(lon: f64, lat: f64) -> (f64, f64) {
    let e2 = GRS80_F * (2.0 - GRS80_F);
    let e = e2.sqrt();
    let q = |phi: f64| {
        let s = phi.sin();
        (1.0 - e2)
            * (s / (1.0 - e2 * s * s) - (1.0 / (2.0 * e)) * ((1.0 - e * s) / (1.0 + e * s)).ln())
    };
    let qp = q(std::f64::consts::FRAC_PI_2);
    let phi1 = LAEA_LAT0.to_radians();
    let beta1 = (q(phi1) / qp).asin();
    let beta = (q(lat.to_radians()) / qp).clamp(-1.0, 1.0).asin();
    let rq = GRS80_A * (qp / 2.0).sqrt();
    let m1 = phi1.cos() / (1.0 - e2 * phi1.sin().powi(2)).sqrt();
    let d = GRS80_A * m1 / (rq * beta1.cos());
    let dlon = (lon - LAEA_LON0).to_radians();
    let b = rq
        * (2.0 / (1.0 + beta1.sin() * beta.sin() + beta1.cos() * beta.cos() * dlon.cos())).sqrt();
    let x = b * d * beta.cos() * dlon.sin();
    let y = (b / d) * (beta1.cos() * beta.sin() - beta1.sin() * beta.cos() * dlon.cos());
    (LAEA_FALSE_EASTING + x, LAEA_FALSE_NORTHING + y)
}

/// One feature of `data/map/provinces.geojson`: the polygon of a province in
/// map coordinates (pixels) plus precomputed properties.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvinceGeometry {
    pub id: ProvinceId,
    /// `[x, y]` centroid in map pixels.
    pub centroid: [f64; 2],
    /// Adjacent provinces as computed from shared edges and sea links.
    pub neighbors: Vec<ProvinceId>,
    /// `[x, y]` position of the capital marker in map pixels.
    pub capital_px: [f64; 2],
    /// Raw GeoJSON `geometry` member (Polygon or MultiPolygon), kept as-is.
    pub geometry: Value,
    /// Any other property the pipeline may add.
    pub extra: BTreeMap<String, Value>,
}

/// Serialized shape of a GeoJSON `FeatureCollection` of provinces.
#[derive(Debug, Deserialize)]
pub(crate) struct ProvinceFeatureCollection {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) features: Vec<ProvinceFeature>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProvinceFeature {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) properties: ProvinceProperties,
    pub(crate) geometry: Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProvinceProperties {
    pub(crate) id: ProvinceId,
    pub(crate) centroid: [f64; 2],
    #[serde(default)]
    pub(crate) neighbors: Vec<ProvinceId>,
    pub(crate) capital_px: [f64; 2],
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, Value>,
}

impl From<ProvinceFeature> for ProvinceGeometry {
    fn from(feature: ProvinceFeature) -> Self {
        ProvinceGeometry {
            id: feature.properties.id,
            centroid: feature.properties.centroid,
            neighbors: feature.properties.neighbors,
            capital_px: feature.properties.capital_px,
            geometry: feature.geometry,
            extra: feature.properties.extra,
        }
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    /// (lon, lat, E, N) computed with pyproj (EPSG:4326 → EPSG:3035, `always_xy`).
    const PYPROJ: [(f64, f64, f64, f64); 6] = [
        (2.3522, 48.8566, 3_760_771.864_838_01, 2_889_484.801_900_801),
        (
            -0.1276,
            51.5072,
            3_620_448.669_410_665,
            3_203_879.367_017_084,
        ),
        (
            37.6173,
            55.7558,
            6_006_447.700_439_354,
            3_957_761.972_348_331,
        ),
        (
            31.2357,
            30.0444,
            6_378_369.921_873_99,
            1_066_036.704_414_431,
        ),
        (
            49.1221,
            55.7887,
            6_636_009.230_902_53,
            4_286_624.910_937_207,
        ),
        (
            -9.1393,
            38.7223,
            2_665_519.214_312_482,
            1_946_764.044_817_952,
        ),
    ];

    fn meta(bounds: [f64; 4], size: [u32; 2]) -> MapMeta {
        MapMeta {
            crs: "EPSG:3035".into(),
            bounds_projected: bounds,
            size_px: size,
            meters_per_px: 718.976_562_5,
            height_min_m: -200.0,
            height_max_m: 4800.0,
            extra: BTreeMap::new(),
        }
    }

    #[test]
    fn laea_matches_pyproj() {
        for (lon, lat, e, n) in PYPROJ {
            let (x, y) = laea_3035(lon, lat);
            assert!(
                (x - e).abs() < 0.05 && (y - n).abs() < 0.05,
                "{lon} {lat}: {x} {y}"
            );
        }
    }

    #[test]
    fn lonlat_to_px_on_the_wide_world_within_a_pixel() {
        // ADR 0115: 7168 × 6144 units, same west edge, top raised by 1280 units.
        let wide = meta(
            [2_169_486.0, 775_684.0, 7_323_110.0, 5_193_076.0],
            [7168, 6144],
        );
        let old = meta(
            [2_169_486.0, 1_327_858.0, 5_114_414.0, 4_272_786.0],
            [4096, 4096],
        );
        for (lon, lat, e, n) in PYPROJ {
            let expected = [
                (e - 2_169_486.0) / 718.976_562_5,
                (5_193_076.0 - n) / 718.976_562_5,
            ];
            let px = wide.lonlat_to_px(lon, lat).unwrap();
            assert!((px[0] - expected[0]).abs() < 1.0 && (px[1] - expected[1]).abs() < 1.0);
            let before = old.lonlat_to_px(lon, lat).unwrap();
            assert!((px[0] - before[0]).abs() < 1e-6 && (px[1] - before[1] - 1280.0).abs() < 1e-6);
        }
        // Paris, Moscow: inside the wide world.
        let paris = wide.lonlat_to_px(2.3522, 48.8566).unwrap();
        assert!((paris[0] - 2213.265).abs() < 0.01 && (paris[1] - 3203.986).abs() < 0.01);
        let moscow = wide.lonlat_to_px(37.6173, 55.7558).unwrap();
        assert!(moscow[0] > 4096.0 && moscow[0] < 7168.0 && moscow[1] > 0.0);
    }
}
