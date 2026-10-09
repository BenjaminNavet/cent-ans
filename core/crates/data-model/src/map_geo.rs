//! Parsing of the vector map data (lot SC DT4, ADR 0206) : provinces, rivers, coastline, roads and
//! rendered rivers, from the GeoJSON / JSON files of `data/map/`. Pure Rust (no Godot type) so it
//! can be unit-tested; the GDExtension converts the result into packed arrays.
//!
//! Points are map pixels, kept as `[f32; 2]` (parsed as `f64` then cast, like Godot's `Vector2`).

use serde::Deserialize;
use serde_json::Value;

/// A polyline in map pixels.
pub type Line = Vec<[f32; 2]>;

/// A province as read from `provinces.geojson` (properties with `MapData`'s defaults applied).
#[derive(Debug, Clone, PartialEq)]
pub struct ProvinceFeature {
    pub index: i64,
    pub id: String,
    pub name: String,
    pub owner: String,
    pub terrain: String,
    pub capital_name: String,
    pub centroid: [f32; 2],
    /// `capital_px` when present, else the centroid.
    pub capital_px: [f32; 2],
    /// Whether `capital_px` or `centroid` was present (used by the faction picker's seat).
    pub has_seat: bool,
    pub neighbors: Vec<String>,
    pub area_px: f64,
    /// Outer rings of every polygon.
    pub rings: Vec<Line>,
}

/// A river line from `rivers.geojson`.
#[derive(Debug, Clone, PartialEq)]
pub struct RiverLine {
    pub name: String,
    pub importance: i64,
    pub points: Line,
}

/// A road line from `roads.geojson` (at least two points).
#[derive(Debug, Clone, PartialEq)]
pub struct RoadLine {
    pub kind: String,
    pub points: Line,
}

/// A river of `rivers_render.json`, with one width per point.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderRiver {
    pub name: String,
    pub importance: i64,
    pub points: Line,
    pub widths: Vec<f32>,
}

/// A custom zone (emblematic town) of `rivers_render.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderZone {
    pub id: String,
    pub name: String,
    pub px: [f32; 2],
    pub radius_px: f32,
    pub boundary_bridges: bool,
}

/// Content of `rivers_render.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedRivers {
    pub bank_px: f32,
    pub zones: Vec<RenderZone>,
    pub rivers: Vec<RenderRiver>,
}

/// Error of a map vector file.
#[derive(Debug, thiserror::Error)]
pub enum MapGeoError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Deserialize)]
struct Collection {
    #[serde(default)]
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    #[serde(default)]
    id: Value,
    #[serde(default)]
    properties: serde_json::Map<String, Value>,
    #[serde(default)]
    geometry: Geometry,
}

#[derive(Deserialize, Default)]
struct Geometry {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    coordinates: Value,
}

fn point(value: &Value) -> [f32; 2] {
    let coord = |i: usize| value.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32;
    [coord(0), coord(1)]
}

fn line(value: &Value) -> Line {
    value
        .as_array()
        .map(|points| points.iter().map(point).collect())
        .unwrap_or_default()
}

fn lines(geometry: &Geometry) -> Vec<Line> {
    match (geometry.kind.as_str(), geometry.coordinates.as_array()) {
        ("LineString", Some(_)) => vec![line(&geometry.coordinates)],
        ("MultiLineString", Some(parts)) => parts.iter().map(line).collect(),
        _ => Vec::new(),
    }
}

fn outer_rings(geometry: &Geometry) -> Vec<Line> {
    let Some(coords) = geometry.coordinates.as_array() else {
        return Vec::new();
    };
    let first_ring = |polygon: &Value| polygon.get(0).map(line);
    match geometry.kind.as_str() {
        "Polygon" => coords.first().map(line).into_iter().collect(),
        "MultiPolygon" => coords.iter().filter_map(first_ring).collect(),
        _ => Vec::new(),
    }
}

fn text(props: &serde_json::Map<String, Value>, key: &str, default: &str) -> String {
    match props.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => default.to_string(),
        Some(other) => other.to_string(),
    }
}

fn integer(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_f64().map(|f| f as i64))
}

/// Parses `provinces.geojson`. Same defaults as the historical `MapData._load_provinces`.
pub fn parse_provinces(bytes: &[u8]) -> Result<Vec<ProvinceFeature>, MapGeoError> {
    let collection: Collection = serde_json::from_slice(bytes)?;
    let mut provinces = Vec::with_capacity(collection.features.len());
    for (position, feature) in collection.features.iter().enumerate() {
        let props = &feature.properties;
        let index = props
            .get("index")
            .and_then(integer)
            .unwrap_or(position as i64 + 1);
        let default_id = match &feature.id {
            Value::String(s) => s.clone(),
            _ => format!("prov_{index}"),
        };
        let id = text(props, "id", &default_id);
        let centroid_value = props.get("centroid");
        let centroid = centroid_value.map_or([0.0, 0.0], point);
        let capital_value = props.get("capital_px");
        let capital_px = capital_value.map_or(centroid, point);
        provinces.push(ProvinceFeature {
            index,
            name: text(props, "name", &id),
            owner: text(props, "owner", ""),
            terrain: text(props, "terrain", ""),
            capital_name: text(props, "capital_name", ""),
            centroid,
            capital_px,
            has_seat: capital_value.is_some_and(Value::is_array)
                || centroid_value.is_some_and(Value::is_array),
            neighbors: props
                .get("neighbors")
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default(),
            area_px: props.get("area_px").and_then(Value::as_f64).unwrap_or(0.0),
            rings: outer_rings(&feature.geometry),
            id,
        });
    }
    Ok(provinces)
}

/// Parses `rivers.geojson` (importance from `strahler`, else from `scalerank`).
pub fn parse_rivers(bytes: &[u8]) -> Result<Vec<RiverLine>, MapGeoError> {
    let collection: Collection = serde_json::from_slice(bytes)?;
    let mut rivers = Vec::new();
    for feature in &collection.features {
        let props = &feature.properties;
        let importance = match props.get("strahler").and_then(integer) {
            Some(strahler) => strahler.clamp(1, 6),
            None => (12 - props.get("scalerank").and_then(integer).unwrap_or(10)).clamp(0, 6),
        };
        let name = text(props, "name", "");
        for points in lines(&feature.geometry) {
            rivers.push(RiverLine {
                name: name.clone(),
                importance,
                points,
            });
        }
    }
    Ok(rivers)
}

/// Parses `coastline.geojson`.
pub fn parse_coastline(bytes: &[u8]) -> Result<Vec<Line>, MapGeoError> {
    let collection: Collection = serde_json::from_slice(bytes)?;
    Ok(collection
        .features
        .iter()
        .flat_map(|feature| lines(&feature.geometry))
        .collect())
}

/// Parses `roads.geojson` (lines under two points are dropped, `type` defaults to `secondary`).
pub fn parse_roads(bytes: &[u8]) -> Result<Vec<RoadLine>, MapGeoError> {
    let collection: Collection = serde_json::from_slice(bytes)?;
    let mut roads = Vec::new();
    for feature in &collection.features {
        let kind = text(&feature.properties, "type", "secondary");
        for points in lines(&feature.geometry) {
            if points.len() >= 2 {
                roads.push(RoadLine {
                    kind: kind.clone(),
                    points,
                });
            }
        }
    }
    Ok(roads)
}

#[derive(Deserialize)]
struct RawRendered {
    #[serde(default = "default_bank")]
    bank_px: f64,
    #[serde(default)]
    custom_zones: Vec<RawZone>,
    #[serde(default)]
    rivers: Vec<RawRiver>,
}

fn default_bank() -> f64 {
    1.1
}

#[derive(Deserialize)]
struct RawZone {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    px: Vec<f64>,
    #[serde(default)]
    radius_px: f64,
    #[serde(default = "default_true")]
    boundary_bridges: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct RawRiver {
    #[serde(default)]
    name: String,
    #[serde(default)]
    importance: f64,
    #[serde(default)]
    points: Vec<Vec<f64>>,
    #[serde(default)]
    widths: Vec<f64>,
}

/// Width of a rendered river point without an entry in `widths`.
const DEFAULT_WIDTH: f32 = 0.3;

/// Parses `rivers_render.json`.
pub fn parse_rendered_rivers(bytes: &[u8]) -> Result<RenderedRivers, MapGeoError> {
    let raw: RawRendered = serde_json::from_slice(bytes)?;
    let coord = |v: &[f64], i: usize| v.get(i).copied().unwrap_or(0.0) as f32;
    Ok(RenderedRivers {
        bank_px: raw.bank_px as f32,
        zones: raw
            .custom_zones
            .iter()
            .map(|z| RenderZone {
                id: z.id.clone(),
                name: z.name.clone(),
                px: [coord(&z.px, 0), coord(&z.px, 1)],
                radius_px: z.radius_px as f32,
                boundary_bridges: z.boundary_bridges,
            })
            .collect(),
        rivers: raw
            .rivers
            .into_iter()
            .map(|r| {
                let widths = (0..r.points.len())
                    .map(|i| r.widths.get(i).map_or(DEFAULT_WIDTH, |w| *w as f32))
                    .collect();
                RenderRiver {
                    name: r.name,
                    importance: r.importance as i64,
                    points: r
                        .points
                        .iter()
                        .map(|p| [coord(p, 0), coord(p, 1)])
                        .collect(),
                    widths,
                }
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn province_defaults_and_rings() {
        let json = br#"{"features":[
          {"properties":{"id":"p_a","index":7,"centroid":[1,2],"neighbors":["p_b"],"area_px":3},
           "geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1]],[[0,0],[9,9],[0,9]]]}},
          {"id":"feat","properties":{},"geometry":{"type":"MultiPolygon","coordinates":[[[[0,0],[2,0],[2,2]]],[[[5,5],[6,5],[6,6]]]]}}]}"#;
        let provinces = parse_provinces(json).unwrap();
        assert_eq!(provinces[0].index, 7);
        assert_eq!(provinces[0].capital_px, [1.0, 2.0]);
        assert!(provinces[0].has_seat);
        assert_eq!(provinces[0].rings.len(), 1);
        assert_eq!(provinces[0].rings[0].len(), 3);
        assert_eq!(provinces[1].index, 2);
        assert_eq!(provinces[1].id, "feat");
        assert_eq!(provinces[1].name, "feat");
        assert!(!provinces[1].has_seat);
        assert_eq!(provinces[1].rings.len(), 2);
    }

    #[test]
    fn river_importance_rules() {
        let json = br#"{"features":[
          {"properties":{"strahler":9,"name":"A"},"geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]}},
          {"properties":{"scalerank":8},"geometry":{"type":"MultiLineString","coordinates":[[[0,0],[1,1]],[[2,2],[3,3]]]}}]}"#;
        let rivers = parse_rivers(json).unwrap();
        assert_eq!(rivers.len(), 3);
        assert_eq!(rivers[0].importance, 6);
        assert_eq!(rivers[1].importance, 4);
    }

    #[test]
    fn roads_drop_short_lines_and_default_type() {
        let json = br#"{"features":[
          {"properties":{},"geometry":{"type":"LineString","coordinates":[[0,0]]}},
          {"properties":{"type":"main"},"geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]}},
          {"properties":{},"geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]}}]}"#;
        let roads = parse_roads(json).unwrap();
        assert_eq!(roads.len(), 2);
        assert_eq!(roads[0].kind, "main");
        assert_eq!(roads[1].kind, "secondary");
    }

    #[test]
    fn rendered_rivers_pad_widths() {
        let json = br#"{"bank_px":0.7,"custom_zones":[{"id":"paris","px":[1,2],"radius_px":6.8}],
          "rivers":[{"name":"Seine","importance":6,"points":[[0,0],[1,1],[2,2]],"widths":[1.0]}]}"#;
        let rendered = parse_rendered_rivers(json).unwrap();
        assert_eq!(rendered.zones[0].px, [1.0, 2.0]);
        assert!(rendered.zones[0].boundary_bridges);
        assert_eq!(rendered.rivers[0].widths, vec![1.0, 0.3, 0.3]);
    }
}
