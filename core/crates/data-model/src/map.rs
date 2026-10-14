//! Map metadata and province geometry written by the `tools/geo` pipeline
//! into `data/map/` (see `docs/design/m1-campaign-map.md`, section 2).
//!
//! These files are produced by a tool rather than edited by hand, so unknown
//! keys are kept in `extra` instead of being rejected.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::ProvinceId;

/// Contents of `data/map/map.json`: projection and raster geometry.
///
/// Two coordinate systems exist (ADR 0355, `docs/design/2026-10-14-carte-composee.md`):
/// - `"EPSG:3035"`: the real map, pixels = LAEA metres / `meters_per_px`;
/// - `"composed"`: the composed (cartogram) map; pixels = real pixels carried through the warp
///   of `data/map/compose/warp.json` (see [`ComposedWarp`]). Load it with [`MapMeta::load_warp`].
///
/// This type is the single projection of the engine: every rule or bridge function that needs
/// `lon/lat <-> map pixel` goes through [`MapMeta::lonlat_to_px`] / [`MapMeta::px_to_lonlat`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapMeta {
    /// Coordinate reference system: `"EPSG:3035"` or `"composed"`.
    pub crs: String,
    /// `[minx, miny, maxx, maxy]` of the projected extent (the real grid when composed).
    #[serde(default)]
    pub bounds_projected: [f64; 4],
    /// `[width, height]` of the rasters in pixels.
    pub size_px: [u32; 2],
    /// Metres per game pixel (nominal when composed).
    pub meters_per_px: f64,
    /// Altitude encoded by a heightmap value of 0.
    pub height_min_m: f64,
    /// Altitude encoded by the maximum heightmap value.
    pub height_max_m: f64,
    /// Warp of a composed map, filled by [`MapMeta::load_warp`].
    #[serde(skip)]
    pub warp: Option<Arc<ComposedWarp>>,
    /// Any other key the pipeline may add.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// CRS value of the composed map.
pub const CRS_COMPOSED: &str = "composed";

/// Real (EPSG:3035) pixel grid the warp is defined on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RealGrid {
    pub minx: f64,
    pub maxy: f64,
    pub meters_per_px: f64,
}

impl RealGrid {
    /// The historical 7168 x 6144 map at 718.98 m/px (ADR 0115).
    pub const LEGACY: RealGrid = RealGrid {
        minx: 2_169_486.0,
        maxy: 5_193_076.0,
        meters_per_px: 718.976_562_5,
    };
}

/// Bilinear float32 grid of `(x, y)` pairs, row 0 = north.
#[derive(Debug, Clone, PartialEq)]
struct PairGrid {
    nx: usize,
    ny: usize,
    data: Vec<f32>,
}

impl PairGrid {
    fn from_bytes(bytes: &[u8], ny: usize) -> Option<PairGrid> {
        if ny == 0 || !bytes.len().is_multiple_of(8) {
            return None;
        }
        let data: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect();
        let nx = data.len() / 2 / ny;
        (nx > 0 && nx * ny * 2 == data.len()).then_some(PairGrid { nx, ny, data })
    }

    /// Bilinear value at node coordinates `(i, j)` (fractional column, row); beyond the last
    /// node centre the last cell is extended linearly (same as the Python reference). `None`
    /// when a contributing node is NaN. Extent checks belong to the caller.
    fn sample(&self, i: f64, j: f64) -> Option<[f64; 2]> {
        if self.nx < 2 || self.ny < 2 {
            return None;
        }
        let i0 = (i.floor().max(0.0) as usize).min(self.nx - 2);
        let j0 = (j.floor().max(0.0) as usize).min(self.ny - 2);
        let (i1, j1) = (i0 + 1, j0 + 1);
        let (fx, fy) = (i - i0 as f64, j - j0 as f64);
        let at = |c: usize, r: usize| {
            let k = (r * self.nx + c) * 2;
            [f64::from(self.data[k]), f64::from(self.data[k + 1])]
        };
        let mut out = [0.0; 2];
        for (c, r, w) in [
            (i0, j0, (1.0 - fx) * (1.0 - fy)),
            (i1, j0, fx * (1.0 - fy)),
            (i0, j1, (1.0 - fx) * fy),
            (i1, j1, fx * fy),
        ] {
            if w != 0.0 {
                let v = at(c, r);
                out[0] += w * v[0];
                out[1] += w * v[1];
            }
        }
        (out[0].is_finite() && out[1].is_finite()).then_some(out)
    }
}

/// Warp of the composed map (`data/map/compose/warp.json` + two float32 grids).
///
/// Conventions (shared contract, `docs/wip/cc-carte-composee.md`): both grids have shape
/// `(ny, nx, 2)`, little-endian, row 0 = north; node `(i, j)` sits at the centre of a block of
/// `factor` full-resolution pixels, so node coordinates are `(px - origin) / factor - 0.5`.
/// - forward: indexed by real pixels minus the crop origin `crop_box_px[0..2]`; value = composed
///   pixel (origin = crop corner);
/// - inverse: indexed by composed pixels; value = real pixel (absolute, legacy grid).
#[derive(Debug, Clone, PartialEq)]
pub struct ComposedWarp {
    /// `[x0, y0, x1, y1]` of the crop in real full-resolution pixels.
    pub crop_box_px: [f64; 4],
    pub factor: f64,
    /// Composed canvas `[width, height]` (full resolution).
    pub size_px: [u32; 2],
    pub real_grid: RealGrid,
    forward: PairGrid,
    inverse: PairGrid,
}

impl ComposedWarp {
    /// Builds a warp from the parsed `warp.json` and the raw bytes of the two grids.
    ///
    /// Keys read from `warp.json`: `crop_box_px` `[x0, y0, x1, y1]`, `factor`, `size_px`
    /// `[w, h]`, `real_grid` (`bounds_projected` `[minx, _, _, maxy]` or `origin` `[minx, maxy]`,
    /// `meters_per_px`; legacy grid when absent). Grid heights are derived from the box:
    /// forward `ceil(h_crop / factor)`, inverse `ceil(height / factor)`; the width follows from
    /// the byte length.
    pub fn from_parts(
        header: &Value,
        forward_bytes: &[u8],
        inverse_bytes: &[u8],
        fallback_real: Option<RealGrid>,
    ) -> Result<ComposedWarp, String> {
        let nums = |v: Option<&Value>, n: usize| -> Option<Vec<f64>> {
            let list: Vec<f64> = v?.as_array()?.iter().filter_map(Value::as_f64).collect();
            (list.len() >= n).then_some(list)
        };
        let crop = nums(header.get("crop_box_px"), 4).ok_or("warp: missing crop_box_px")?;
        let factor = header
            .get("factor")
            .and_then(Value::as_f64)
            .filter(|f| *f >= 1.0)
            .ok_or("warp: missing factor")?;
        let size = nums(header.get("size_px"), 2).ok_or("warp: missing size_px")?;
        let real_grid = header
            .get("real_grid")
            .and_then(|g| {
                let mpp = g.get("meters_per_px").and_then(Value::as_f64)?;
                let (minx, maxy) = if let Some(b) = nums(g.get("bounds_projected"), 4) {
                    (b[0], b[3])
                } else {
                    let o = nums(g.get("origin"), 2)?;
                    (o[0], o[1])
                };
                Some(RealGrid {
                    minx,
                    maxy,
                    meters_per_px: mpp,
                })
            })
            .or(fallback_real)
            .unwrap_or(RealGrid::LEGACY);
        let crop_h = crop[3] - crop[1];
        let rows = |key: &str, default: f64| -> usize {
            header
                .get(key)
                .and_then(|g| g.get("shape"))
                .and_then(|s| s.get(0))
                .and_then(Value::as_u64)
                .map_or((default / factor).ceil() as usize, |n| n as usize)
        };
        let forward = PairGrid::from_bytes(forward_bytes, rows("forward", crop_h))
            .ok_or("warp: bad forward grid")?;
        let inverse = PairGrid::from_bytes(inverse_bytes, rows("inverse", size[1]))
            .ok_or("warp: bad inverse grid")?;
        Ok(ComposedWarp {
            crop_box_px: [crop[0], crop[1], crop[2], crop[3]],
            factor,
            size_px: [size[0] as u32, size[1] as u32],
            real_grid,
            forward,
            inverse,
        })
    }

    /// Real (legacy grid) pixel -> composed pixel; `None` outside the crop.
    pub fn forward_px(&self, x: f64, y: f64) -> Option<[f64; 2]> {
        let [x0, y0, x1, y1] = self.crop_box_px;
        if !(x >= x0 && x < x1 && y >= y0 && y < y1) {
            return None;
        }
        let i = (x - self.crop_box_px[0]) / self.factor - 0.5;
        let j = (y - self.crop_box_px[1]) / self.factor - 0.5;
        self.forward.sample(i, j)
    }

    /// Composed pixel -> real (legacy grid) pixel; `None` outside the canvas.
    pub fn inverse_px(&self, x: f64, y: f64) -> Option<[f64; 2]> {
        let [w, h] = self.size_px.map(f64::from);
        if !(x >= 0.0 && x < w && y >= 0.0 && y < h) {
            return None;
        }
        self.inverse
            .sample(x / self.factor - 0.5, y / self.factor - 0.5)
    }

    /// Writes a synthetic warp to bytes (tests, tools): `(forward, inverse)` little-endian grids
    /// from two closures.
    pub fn encode_grid(
        nx: usize,
        ny: usize,
        mut value: impl FnMut(usize, usize) -> [f32; 2],
    ) -> Vec<u8> {
        let mut out = Vec::with_capacity(nx * ny * 8);
        for j in 0..ny {
            for i in 0..nx {
                for v in value(i, j) {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        out
    }
}

impl MapMeta {
    /// Whether the map uses the composed CRS.
    pub fn is_composed(&self) -> bool {
        self.crs == CRS_COMPOSED
    }

    /// Reads `map_dir/map.json` and, for the composed CRS, its warp. This is the way the
    /// bridge and tools obtain the engine projection.
    pub fn load(map_dir: &Path) -> Result<MapMeta, String> {
        let path = map_dir.join("map.json");
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut meta: MapMeta = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        meta.load_warp(map_dir)?;
        Ok(meta)
    }

    /// Reads the warp of a composed map (`composed.warp` of `map.json`, default
    /// `compose/warp.json`, relative to `map_dir`; grids named by `forward`/`inverse` keys of
    /// the warp file, default `warp_forward.f32` / `warp_inverse.f32` next to it). No-op for
    /// other CRS. Errors are returned as text; the map then stays without projection.
    pub fn load_warp(&mut self, map_dir: &Path) -> Result<(), String> {
        if !self.is_composed() {
            return Ok(());
        }
        let rel = self
            .extra
            .get("composed")
            .and_then(|c| c.get("warp"))
            .and_then(Value::as_str)
            .unwrap_or("compose/warp.json");
        let path = map_dir.join(rel);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let header: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let dir = path.parent().unwrap_or(map_dir);
        let file = |key: &str, default: &str| {
            let name = header
                .get(key)
                .and_then(|v| v.as_str().or_else(|| v.get("file").and_then(Value::as_str)))
                .unwrap_or(default);
            std::fs::read(dir.join(name)).map_err(|e| format!("{name}: {e}"))
        };
        let forward = file("forward", "warp_forward.f32")?;
        let inverse = file("inverse", "warp_inverse.f32")?;
        let fallback = self.extra.get("real_grid").and_then(|g| {
            let b = g.get("bounds_projected")?.as_array()?;
            Some(RealGrid {
                minx: b.first()?.as_f64()?,
                maxy: b.get(3)?.as_f64()?,
                meters_per_px: g.get("meters_per_px")?.as_f64()?,
            })
        });
        self.warp = Some(Arc::new(ComposedWarp::from_parts(
            &header, &forward, &inverse, fallback,
        )?));
        Ok(())
    }

    /// Legacy-grid pixel of EPSG:3035 metres.
    fn projected_to_real_px(&self, e: f64, n: f64) -> Option<[f64; 2]> {
        let grid = self.warp.as_ref().map_or_else(
            || RealGrid {
                minx: self.bounds_projected[0],
                maxy: self.bounds_projected[3],
                meters_per_px: self.meters_per_px,
            },
            |w| w.real_grid,
        );
        (grid.meters_per_px > 0.0).then(|| {
            [
                (e - grid.minx) / grid.meters_per_px,
                (grid.maxy - n) / grid.meters_per_px,
            ]
        })
    }

    /// Map pixel of EPSG:3035 metres `(E, N)` (landmark origins, `data/landmarks_v2`).
    pub fn projected_to_px(&self, e: f64, n: f64) -> Option<[f64; 2]> {
        let real = self.projected_to_real_px(e, n)?;
        if self.is_composed() {
            return self.warp.as_ref()?.forward_px(real[0], real[1]);
        }
        Some(real)
    }

    /// Map-pixel position of a longitude/latitude (degrees): EPSG:3035 projection
    /// ([`laea_3035`]) then `x = (E − minx) / m`, `y = (maxy − N) / m` (convention of `tools/geo`,
    /// ADR 0086), then, for the composed CRS, the forward warp. Another CRS falls back to a
    /// linear mapping of `extent_lonlat` over `size_px` (synthetic test maps); `None` without
    /// either, or outside the composed canvas.
    pub fn lonlat_to_px(&self, lon: f64, lat: f64) -> Option<[f64; 2]> {
        if (self.crs == "EPSG:3035" && self.meters_per_px > 0.0) || self.is_composed() {
            let (e, n) = laea_3035(lon, lat);
            return self.projected_to_px(e, n);
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

    /// Inverse of [`MapMeta::lonlat_to_px`]: real longitude/latitude (degrees) of a map pixel.
    pub fn px_to_lonlat(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        if self.crs == "EPSG:3035" || self.is_composed() {
            let [rx, ry] = if self.is_composed() {
                self.warp.as_ref()?.inverse_px(x, y)?
            } else {
                [x, y]
            };
            let grid = self.warp.as_ref().map_or_else(
                || RealGrid {
                    minx: self.bounds_projected[0],
                    maxy: self.bounds_projected[3],
                    meters_per_px: self.meters_per_px,
                },
                |w| w.real_grid,
            );
            let e = grid.minx + rx * grid.meters_per_px;
            let n = grid.maxy - ry * grid.meters_per_px;
            return Some(inverse_laea_3035(e, n));
        }
        let extent: Vec<f64> = self
            .extra
            .get("extent_lonlat")?
            .as_array()?
            .iter()
            .filter_map(Value::as_f64)
            .collect();
        let [w, s, e, n] = <[f64; 4]>::try_from(extent).ok()?;
        Some((
            w + x / f64::from(self.size_px[0]) * (e - w),
            n - y / f64::from(self.size_px[1]) * (n - s),
        ))
    }

    /// Distance (km) between two map pixels: game distance, pixels x `meters_per_px`.
    pub fn px_distance_km(&self, a: [f64; 2], b: [f64; 2]) -> f64 {
        (a[0] - b[0]).hypot(a[1] - b[1]) * self.meters_per_px / 1000.0
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

/// Longitude and latitude (degrees) of EPSG:3035 metres: inverse of [`laea_3035`] (Snyder
/// 24-28 to 24-31, authalic latitude series).
pub fn inverse_laea_3035(e: f64, n: f64) -> (f64, f64) {
    let e2 = GRS80_F * (2.0 - GRS80_F);
    let ec = e2.sqrt();
    let q = |phi: f64| {
        let s = phi.sin();
        (1.0 - e2)
            * (s / (1.0 - e2 * s * s) - (1.0 / (2.0 * ec)) * ((1.0 - ec * s) / (1.0 + ec * s)).ln())
    };
    let qp = q(std::f64::consts::FRAC_PI_2);
    let phi1 = LAEA_LAT0.to_radians();
    let beta1 = (q(phi1) / qp).asin();
    let rq = GRS80_A * (qp / 2.0).sqrt();
    let m1 = phi1.cos() / (1.0 - e2 * phi1.sin().powi(2)).sqrt();
    let d = GRS80_A * m1 / (rq * beta1.cos());
    let (x, y) = (e - LAEA_FALSE_EASTING, n - LAEA_FALSE_NORTHING);
    let rho = (x / d).hypot(d * y);
    if rho < 1e-9 {
        return (LAEA_LON0, LAEA_LAT0);
    }
    let ce = 2.0 * (rho / (2.0 * rq)).clamp(-1.0, 1.0).asin();
    let (sc, cc) = ce.sin_cos();
    let beta = (cc * beta1.sin() + d * y * sc * beta1.cos() / rho)
        .clamp(-1.0, 1.0)
        .asin();
    let lon = LAEA_LON0.to_radians()
        + (x * sc).atan2(d * rho * beta1.cos() * cc - d * d * y * beta1.sin() * sc);
    let (e4, e6) = (e2 * e2, e2 * e2 * e2);
    let phi = beta
        + (e2 / 3.0 + 31.0 * e4 / 180.0 + 517.0 * e6 / 5040.0) * (2.0 * beta).sin()
        + (23.0 * e4 / 360.0 + 251.0 * e6 / 3780.0) * (4.0 * beta).sin()
        + (761.0 * e6 / 45360.0) * (6.0 * beta).sin();
    (lon.to_degrees(), phi.to_degrees())
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
            warp: None,
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

    /// Affine synthetic warp: composed = (real - crop origin) * 1.5 + (10, 20).
    fn synthetic_composed() -> MapMeta {
        let header = serde_json::json!({
            "crop_box_px": [100, 200, 500, 600], "factor": 8, "size_px": [620, 620],
            "real_grid": {"origin": [2_169_486.0, 5_193_076.0], "meters_per_px": 718.976_562_5},
        });
        let forward = ComposedWarp::encode_grid(50, 50, |i, j| {
            let (rx, ry) = (
                100.0 + (i as f32 + 0.5) * 8.0,
                200.0 + (j as f32 + 0.5) * 8.0,
            );
            [(rx - 100.0) * 1.5 + 10.0, (ry - 200.0) * 1.5 + 20.0]
        });
        let inverse = ComposedWarp::encode_grid(78, 78, |i, j| {
            let (cx, cy) = ((i as f32 + 0.5) * 8.0, (j as f32 + 0.5) * 8.0);
            [(cx - 10.0) / 1.5 + 100.0, (cy - 20.0) / 1.5 + 200.0]
        });
        let warp = ComposedWarp::from_parts(&header, &forward, &inverse, None).unwrap();
        let mut m = meta([0.0; 4], [620, 620]);
        m.crs = CRS_COMPOSED.into();
        m.meters_per_px = 719.0;
        m.warp = Some(Arc::new(warp));
        m
    }

    #[test]
    fn composed_forward_inverse_round_trip() {
        let m = synthetic_composed();
        let w = m.warp.as_ref().unwrap();
        let f = w.forward_px(300.0, 400.0).unwrap();
        assert!(
            (f[0] - 310.0).abs() < 1e-3 && (f[1] - 320.0).abs() < 1e-3,
            "{f:?}"
        );
        let back = w.inverse_px(f[0], f[1]).unwrap();
        assert!((back[0] - 300.0).abs() < 1e-3 && (back[1] - 400.0).abs() < 1e-3);
    }

    #[test]
    fn composed_out_of_extent_is_none() {
        let m = synthetic_composed();
        let w = m.warp.as_ref().unwrap();
        assert!(w.forward_px(50.0, 300.0).is_none());
        assert!(w.forward_px(300.0, 700.0).is_none());
        assert!(w.inverse_px(-40.0, 10.0).is_none());
        assert!(w.inverse_px(10.0, 900.0).is_none());
    }

    #[test]
    fn composed_lonlat_round_trip() {
        // Paris on the synthetic canvas: real px (2213.3, 3204.0) is outside the crop -> None.
        let m = synthetic_composed();
        assert!(m.lonlat_to_px(2.3522, 48.8566).is_none());
        // A point inside the crop: invert a composed pixel, project it back.
        let (lon, lat) = m.px_to_lonlat(300.0, 300.0).unwrap();
        let px = m.lonlat_to_px(lon, lat).unwrap();
        assert!(
            (px[0] - 300.0).abs() < 0.01 && (px[1] - 300.0).abs() < 0.01,
            "{px:?}"
        );
    }

    #[test]
    fn nan_node_is_outside() {
        let bytes =
            ComposedWarp::encode_grid(2, 2, |i, _| [if i == 1 { f32::NAN } else { 1.0 }, 1.0]);
        let grid = PairGrid::from_bytes(&bytes, 2).unwrap();
        assert!(grid.sample(0.0, 0.0).is_some());
        assert!(grid.sample(0.5, 0.5).is_none());
    }

    #[test]
    fn inverse_laea_matches_forward() {
        for (lon, lat, _, _) in PYPROJ {
            let (e, n) = laea_3035(lon, lat);
            let (l, p) = inverse_laea_3035(e, n);
            assert!(
                (l - lon).abs() < 1e-7 && (p - lat).abs() < 1e-7,
                "{lon} {lat}: {l} {p}"
            );
        }
    }

    #[test]
    fn legacy_map_px_round_trip() {
        let wide = meta(
            [2_169_486.0, 775_684.0, 7_323_110.0, 5_193_076.0],
            [7168, 6144],
        );
        let px = wide.lonlat_to_px(2.3522, 48.8566).unwrap();
        let (lon, lat) = wide.px_to_lonlat(px[0], px[1]).unwrap();
        assert!((lon - 2.3522).abs() < 1e-7 && (lat - 48.8566).abs() < 1e-7);
    }

    #[test]
    fn composed_map_loads_from_files() {
        let dir = std::env::temp_dir().join(format!("cc5_warp_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("compose")).unwrap();
        let forward = ComposedWarp::encode_grid(50, 50, |i, j| {
            [(i as f32 + 0.5) * 8.0, (j as f32 + 0.5) * 8.0]
        });
        let inverse = ComposedWarp::encode_grid(50, 50, |i, j| {
            [
                100.0 + (i as f32 + 0.5) * 8.0,
                200.0 + (j as f32 + 0.5) * 8.0,
            ]
        });
        std::fs::write(dir.join("compose/warp_forward.f32"), forward).unwrap();
        std::fs::write(dir.join("compose/warp_inverse.f32"), inverse).unwrap();
        std::fs::write(
            dir.join("compose/warp.json"),
            r#"{"crop_box_px":[100,200,500,600],"factor":8,"size_px":[400,400]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("map.json"),
            r#"{"crs":"composed","size_px":[400,400],"meters_per_px":719,
                "height_min_m":-200,"height_max_m":4800,"composed":{"warp":"compose/warp.json"}}"#,
        )
        .unwrap();
        let meta = MapMeta::load(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert!(meta.is_composed());
        // Identity-like warp (pure shift): real px (110, 210) -> composed (10, 10).
        let w = meta.warp.as_ref().unwrap();
        let c = w.forward_px(110.0, 210.0).unwrap();
        assert!(
            (c[0] - 10.0).abs() < 1e-3 && (c[1] - 10.0).abs() < 1e-3,
            "{c:?}"
        );
        assert!(meta.px_distance_km([0.0, 0.0], [1000.0, 0.0]) > 700.0);
    }
}
