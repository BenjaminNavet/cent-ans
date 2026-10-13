//! `scene.json` loading, `--set` overrides, binary data files and camera path interpolation.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Directory holding `scene.json`, `assets/` and this crate (`tools/proto_bevy`).
pub fn proto_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate lives in tools/proto_bevy/bevy")
        .to_path_buf()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Placement {
    pub asset: String,
    #[serde(default)]
    pub node: String,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CameraSpec {
    pub pos: [f32; 3],
    pub look_at: [f32; 3],
    pub fov_y: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SunSpec {
    pub elevation: f32,
    pub azimuth: f32,
    pub color: [f32; 3],
    pub lux: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FogSpec {
    pub density: f32,
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeightmapSpec {
    pub file: String,
    pub grid: usize,
    pub size_m: f32,
    pub texture: String,
    pub uv_tile_m: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WalkSpec {
    pub bob_m: f32,
    pub sway_deg: f32,
    pub hz: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArmySpec {
    pub file: String,
    pub count: usize,
    pub models: Vec<String>,
    pub walk: WalkSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VegetationSpec {
    pub file: String,
    pub asset: String,
    pub count: usize,
    pub fraction: f32,
    pub node: String,
}

impl VegetationSpec {
    /// Number of instances actually drawn: `floor(count * fraction)`.
    pub fn drawn(&self) -> usize {
        let wanted = (self.count as f64 * f64::from(self.fraction)).floor() as usize;
        wanted.min(self.count)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Vegetation {
    pub grass: VegetationSpec,
    pub firs: VegetationSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathKey {
    pub t: f32,
    pub pos: [f32; 3],
    pub look_at: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UiPanel {
    pub title: String,
    pub body: String,
    pub button: String,
    pub tooltip: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Bench {
    pub warmup_frames: u32,
    pub duration_s: f32,
    pub resolution: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Comparable,
    Best,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scene {
    pub assets: std::collections::BTreeMap<String, String>,
    pub placements: Vec<Placement>,
    pub camera: CameraSpec,
    pub sun: SunSpec,
    pub fog: FogSpec,
    pub heightmap: HeightmapSpec,
    pub army: ArmySpec,
    pub vegetation: Vegetation,
    pub camera_path: Vec<PathKey>,
    pub ui_panel: UiPanel,
    pub bench: Bench,
    pub quality: Quality,
}

impl Scene {
    /// Path of a glTF asset relative to the Bevy asset root (`tools/proto_bevy`).
    pub fn asset_path(&self, name: &str) -> Result<String, String> {
        let raw = self
            .assets
            .get(name)
            .ok_or_else(|| format!("asset {name} missing from scene.json"))?;
        Ok(raw.strip_prefix("tools/proto_bevy/").unwrap_or(raw).to_owned())
    }
}

/// Sets `dotted.key` in `root` (array elements by numeric segment); `raw` is parsed as JSON,
/// else kept as a string.
pub fn set_dotted(root: &mut Value, dotted: &str, raw: &str) -> Result<(), String> {
    let new_value = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_owned()));
    let mut cursor = root;
    let segments: Vec<&str> = dotted.split('.').collect();
    for (position, segment) in segments.iter().enumerate() {
        let last = position + 1 == segments.len();
        let slot = match cursor {
            Value::Object(map) => {
                if last {
                    map.insert((*segment).to_owned(), new_value);
                    return Ok(());
                }
                map.get_mut(*segment)
                    .ok_or_else(|| format!("--set {dotted}: no key {segment}"))?
            }
            Value::Array(items) => {
                let index: usize = segment
                    .parse()
                    .map_err(|_| format!("--set {dotted}: {segment} is not an index"))?;
                let len = items.len();
                let item = items
                    .get_mut(index)
                    .ok_or_else(|| format!("--set {dotted}: index {index} out of {len}"))?;
                if last {
                    *item = new_value;
                    return Ok(());
                }
                item
            }
            _ => return Err(format!("--set {dotted}: {segment} is below a scalar")),
        };
        cursor = slot;
    }
    Err(format!("--set {dotted}: empty key"))
}

/// Reads `scene_path`, applies the overrides, returns the typed scene and the effective JSON.
pub fn load_scene(
    scene_path: &Path,
    overrides: &[(String, String)],
) -> Result<(Scene, Value), String> {
    let text = std::fs::read_to_string(scene_path)
        .map_err(|error| format!("{}: {error}", scene_path.display()))?;
    let mut value: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    for (key, raw) in overrides {
        set_dotted(&mut value, key, raw)?;
    }
    let scene: Scene = serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    Ok((scene, value))
}

/// Height field, `grid × grid` metres, row-major, row = z.
pub struct HeightField {
    pub grid: usize,
    pub size_m: f32,
    pub heights: Vec<f32>,
}

impl HeightField {
    pub fn from_bytes(bytes: &[u8], grid: usize, size_m: f32) -> Result<Self, String> {
        if bytes.len() != grid * grid * 4 {
            return Err(format!(
                "heightmap has {} bytes, expected {}",
                bytes.len(),
                grid * grid * 4
            ));
        }
        let heights = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        Ok(Self { grid, size_m, heights })
    }

    pub fn step(&self) -> f32 {
        self.size_m / (self.grid - 1) as f32
    }

    /// World x (or z) of sample index `index`.
    pub fn coord(&self, index: usize) -> f32 {
        -self.size_m / 2.0 + index as f32 * self.step()
    }

    pub fn at(&self, row: usize, col: usize) -> f32 {
        self.heights[row * self.grid + col]
    }

    /// Bilinear height at world `(x, z)`, clamped to the field.
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let last = (self.grid - 1) as f32;
        let fx = ((x + self.size_m / 2.0) / self.step()).clamp(0.0, last);
        let fz = ((z + self.size_m / 2.0) / self.step()).clamp(0.0, last);
        let (col, row) = (fx.floor() as usize, fz.floor() as usize);
        let (col1, row1) = ((col + 1).min(self.grid - 1), (row + 1).min(self.grid - 1));
        let (tx, tz) = (fx - col as f32, fz - row as f32);
        let top = self.at(row, col) * (1.0 - tx) + self.at(row, col1) * tx;
        let bottom = self.at(row1, col) * (1.0 - tx) + self.at(row1, col1) * tx;
        top * (1.0 - tz) + bottom * tz
    }
}

/// One instance record: `x, y, z, yaw_deg, scale, model_index`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    pub pos: [f32; 3],
    pub yaw_deg: f32,
    pub scale: f32,
    pub model: usize,
}

pub fn parse_records(bytes: &[u8]) -> Result<Vec<Record>, String> {
    if bytes.len() % 24 != 0 {
        return Err(format!("record file has {} bytes, not a multiple of 24", bytes.len()));
    }
    Ok(bytes
        .chunks_exact(24)
        .map(|chunk| {
            let f = |i: usize| {
                f32::from_le_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]])
            };
            Record {
                pos: [f(0), f(1), f(2)],
                yaw_deg: f(3),
                scale: f(4),
                model: f(5) as usize,
            }
        })
        .collect())
}

/// Linear camera path: returns `(pos, look_at)` at `t` seconds, clamped to the end keys.
pub fn camera_at(keys: &[PathKey], t: f32) -> ([f32; 3], [f32; 3]) {
    let first = &keys[0];
    if t <= first.t {
        return (first.pos, first.look_at);
    }
    for pair in keys.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if t <= b.t {
            let k = (t - a.t) / (b.t - a.t);
            let mix = |x: [f32; 3], y: [f32; 3]| [0, 1, 2].map(|i| x[i] + (y[i] - x[i]) * k);
            return (mix(a.pos, b.pos), mix(a.look_at, b.look_at));
        }
    }
    let last = keys.last().expect("camera_path is not empty");
    (last.pos, last.look_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_path() -> PathBuf {
        proto_root().join("scene.json")
    }

    #[test]
    fn loads_scene_json() {
        let (scene, _) = load_scene(&scene_path(), &[]).unwrap();
        assert_eq!(scene.army.count, 5000);
        assert_eq!(scene.vegetation.grass.count, 200_000);
        assert_eq!(scene.placements.len(), 6);
        assert_eq!(scene.quality, Quality::Comparable);
        assert_eq!(scene.asset_path("barn_0").unwrap(), "assets/barn_0.glb");
    }

    #[test]
    fn set_overrides_nested_values() {
        let overrides = vec![
            ("vegetation.grass.fraction".to_owned(), "0.5".to_owned()),
            ("quality".to_owned(), "best".to_owned()),
            ("camera_path.1.t".to_owned(), "7".to_owned()),
        ];
        let (scene, effective) = load_scene(&scene_path(), &overrides).unwrap();
        assert_eq!(scene.vegetation.grass.drawn(), 100_000);
        assert_eq!(scene.quality, Quality::Best);
        assert_eq!(scene.camera_path[1].t, 7.0);
        assert_eq!(effective["quality"], "best");
    }

    #[test]
    fn set_rejects_unknown_path() {
        let mut value = serde_json::json!({"a": {"b": 1}});
        assert!(set_dotted(&mut value, "a.c.d", "1").is_err());
        set_dotted(&mut value, "a.b", "[1,2]").unwrap();
        assert_eq!(value["a"]["b"][1], 2);
    }

    #[test]
    fn heightmap_sampling_is_bilinear() {
        let field = HeightField { grid: 2, size_m: 10.0, heights: vec![0.0, 2.0, 4.0, 6.0] };
        assert_eq!(field.sample(-5.0, -5.0), 0.0);
        assert_eq!(field.sample(5.0, 5.0), 6.0);
        assert!((field.sample(0.0, 0.0) - 3.0).abs() < 1e-6);
        assert_eq!(field.sample(100.0, -100.0), 2.0);
        assert!(HeightField::from_bytes(&[0; 12], 2, 1.0).is_err());
    }

    #[test]
    fn real_heightmap_and_records_parse() {
        let root = proto_root();
        let (scene, _) = load_scene(&scene_path(), &[]).unwrap();
        let bytes = std::fs::read(root.join(&scene.heightmap.file)).unwrap();
        let field =
            HeightField::from_bytes(&bytes, scene.heightmap.grid, scene.heightmap.size_m).unwrap();
        assert!(field.sample(0.0, 0.0).is_finite());
        let army = parse_records(&std::fs::read(root.join(&scene.army.file)).unwrap()).unwrap();
        assert_eq!(army.len(), scene.army.count);
        assert!(army.iter().all(|r| r.model < scene.army.models.len()));
    }

    #[test]
    fn records_decode_little_endian() {
        let mut bytes = Vec::new();
        for value in [1.0f32, 2.0, 3.0, 90.0, 0.5, 2.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let records = parse_records(&bytes).unwrap();
        assert_eq!(
            records,
            vec![Record { pos: [1.0, 2.0, 3.0], yaw_deg: 90.0, scale: 0.5, model: 2 }]
        );
        assert!(parse_records(&bytes[..23]).is_err());
    }

    #[test]
    fn camera_path_interpolates_and_clamps() {
        let key = |t: f32, x: f32| PathKey { t, pos: [x, 0.0, 0.0], look_at: [0.0, x, 0.0] };
        let keys = vec![key(0.0, 0.0), key(10.0, 10.0), key(20.0, 0.0)];
        assert_eq!(camera_at(&keys, -1.0).0, [0.0, 0.0, 0.0]);
        assert_eq!(camera_at(&keys, 5.0).0, [5.0, 0.0, 0.0]);
        assert_eq!(camera_at(&keys, 15.0).1, [0.0, 5.0, 0.0]);
        assert_eq!(camera_at(&keys, 99.0).0, [0.0, 0.0, 0.0]);
    }
}
