//! Figure assets: merged manifest, `CAM1`/`CAM2` skinned meshes and `CAB1` bone textures baked
//! by tools/blender_scripts/battle_skinned.py. Port of the loading half of
//! game/scripts/battle/battle_skinned.gd (`manifest`, `_load_mesh`, `bone_texture`).
//!
//! `CAM1`: `"CAM1"`, u32 vertices `n`, u32 indices `m`, u32 raw size, then a zlib stream of
//! `n` x 21 f32 (positions 3, normals 3, colour rgb + material code 4, uv 2, bone ids 4,
//! weights 4, variant mask 1) followed by `m` u32 indices. `CAM2` adds one packed atlas uv
//! float per vertex (ignored here). `CAB1`: `"CAB1"`, u32 bones, u32 frames, a zlib stream of
//! 3 RGBA32F texels (a 3x4 skinning matrix) per bone and frame.

use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, VertexFormat};
use flate2::read::{DeflateDecoder, ZlibDecoder};
use serde::Deserialize;

use super::anim::{ClipInfo, RigClips};

/// Bone indices of a vertex (four floats, as baked).
pub const ATTRIBUTE_BONES: MeshVertexAttribute =
    MeshVertexAttribute::new("FigureBones", 0x4255_0001, VertexFormat::Float32x4);
/// Bone weights of a vertex.
pub const ATTRIBUTE_WEIGHTS: MeshVertexAttribute =
    MeshVertexAttribute::new("FigureWeights", 0x4255_0002, VertexFormat::Float32x4);
/// Variant mask (bits 0-5 variants that show the part, 6 held weapon, 7 pavise).
pub const ATTRIBUTE_MASK: MeshVertexAttribute =
    MeshVertexAttribute::new("FigureMask", 0x4255_0003, VertexFormat::Float32);

/// Figure of the merged manifest.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FigureEntry {
    #[serde(default)]
    pub lods: Vec<String>,
    #[serde(default)]
    pub rig: String,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub noble: Option<bool>,
    #[serde(default = "one")]
    pub variants: u32,
    #[serde(default)]
    pub ga3_albedo: Option<String>,
    #[serde(default)]
    pub ga3_lum: Option<[f32; 2]>,
    #[serde(default)]
    pub ga3_head_y: Option<f32>,
    #[serde(default)]
    pub tris: Vec<u32>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
struct ClipEntry {
    start: u32,
    frames: u32,
    #[serde(default, rename = "loop")]
    looped: bool,
}

/// Rig of the merged manifest.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RigEntry {
    #[serde(default)]
    pub bones: u32,
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default)]
    pub texture: String,
    #[serde(default)]
    pub mocap_textures: Vec<String>,
    #[serde(default)]
    clips: BTreeMap<String, ClipEntry>,
}

fn default_fps() -> f32 {
    24.0
}

impl RigEntry {
    pub fn clips(&self) -> RigClips {
        let map: BTreeMap<String, ClipInfo> = self
            .clips
            .iter()
            .map(|(name, c)| {
                (
                    name.clone(),
                    ClipInfo {
                        start: c.start,
                        frames: c.frames,
                        looped: c.looped,
                    },
                )
            })
            .collect();
        RigClips::new(self.fps, &map)
    }

    /// Number of clips (the shader table holds [`MAX_CLIPS`]).
    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }
}

/// Clips the shader table can hold.
pub const MAX_CLIPS: usize = 96;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub figures: HashMap<String, FigureEntry>,
    #[serde(default)]
    pub rigs: HashMap<String, RigEntry>,
}

/// Directory of the skinned figures under the asset root.
pub const SKINNED_DIR: &str = "models/battle_skinned";

impl Manifest {
    pub fn load(asset_root: &Path) -> Result<Self, String> {
        let path = asset_root.join(SKINNED_DIR).join("manifest_merged.json");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn figure(&self, kind: &str, variant: u32) -> Option<&FigureEntry> {
        self.figures.get(&format!("{kind}_{variant}"))
    }

    /// Figure of `kind`/`variant`, else the variant 0 of the family.
    pub fn figure_or_default(&self, kind: &str, variant: u32) -> Option<(u32, &FigureEntry)> {
        self.figure(kind, variant)
            .map(|f| (variant, f))
            .or_else(|| self.figure(kind, 0).map(|f| (0, f)))
    }

    pub fn rig(&self, name: &str) -> Option<&RigEntry> {
        self.rigs.get(name)
    }

    /// Figure is noble (manifest, else variant 0 of infantry and cavalry).
    pub fn is_noble(&self, kind: &str, variant: u32) -> bool {
        self.figure(kind, variant)
            .and_then(|f| f.noble)
            .unwrap_or(variant == 0 && (kind == "infantry" || kind == "cavalry"))
    }
}

/// Path of a manifest file under the asset root: `res://assets/x` or a name relative to the
/// skinned directory.
pub fn relative_asset_path(file: &str) -> String {
    match file.strip_prefix("res://assets/") {
        Some(rest) => rest.to_owned(),
        None => match file.strip_prefix("res://") {
            Some(rest) => rest.to_owned(),
            None => format!("{SKINNED_DIR}/{file}"),
        },
    }
}

fn inflate(bytes: &[u8], expected: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(expected);
    if ZlibDecoder::new(bytes).read_to_end(&mut out).is_ok() && out.len() == expected {
        return Ok(out);
    }
    out.clear();
    DeflateDecoder::new(bytes)
        .read_to_end(&mut out)
        .map_err(|e| format!("inflate: {e}"))?;
    if out.len() != expected {
        return Err(format!("inflated {} bytes, expected {expected}", out.len()));
    }
    Ok(out)
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"))
}

fn floats(raw: &[u8], first: usize, count: usize) -> Vec<f32> {
    raw[first * 4..(first + count) * 4]
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().expect("4 bytes")))
        .collect()
}

/// Decoded `CAM1`/`CAM2` mesh arrays (Godot clockwise winding already flipped for Bevy).
pub struct FigureArrays {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// rgb and the material code (integer valued, not divided).
    pub colors: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub bones: Vec<[f32; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub masks: Vec<f32>,
    pub indices: Vec<u32>,
}

pub fn decode_mesh(bytes: &[u8]) -> Result<FigureArrays, String> {
    if bytes.len() < 16 {
        return Err("mesh file too short".into());
    }
    let stride = match &bytes[..4] {
        b"CAM1" => 21,
        b"CAM2" => 22,
        _ => return Err("bad mesh magic".into()),
    };
    let n = u32_at(bytes, 4) as usize;
    let m = u32_at(bytes, 8) as usize;
    let raw_len = u32_at(bytes, 12) as usize;
    let raw = inflate(&bytes[16..], raw_len)?;
    if raw.len() < (n * stride + m) * 4 {
        return Err("mesh payload too short".into());
    }
    let all = floats(&raw, 0, n * stride);
    let chunk = |first: usize, width: usize| -> Vec<Vec<f32>> {
        (0..n)
            .map(|i| all[first + i * width..first + (i + 1) * width].to_vec())
            .collect()
    };
    let v3 = |first: usize| -> Vec<[f32; 3]> {
        chunk(first, 3).iter().map(|v| [v[0], v[1], v[2]]).collect()
    };
    let v4 = |first: usize| -> Vec<[f32; 4]> {
        chunk(first, 4)
            .iter()
            .map(|v| [v[0], v[1], v[2], v[3]])
            .collect()
    };
    let mut indices: Vec<u32> = raw[n * stride * 4..(n * stride + m) * 4]
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().expect("4 bytes")))
        .collect();
    // Godot front faces are clockwise, Bevy's counter-clockwise.
    for tri in indices.chunks_exact_mut(3) {
        tri.swap(1, 2);
    }
    Ok(FigureArrays {
        positions: v3(0),
        normals: v3(n * 3),
        colors: v4(n * 6),
        uvs: chunk(n * 10, 2).iter().map(|v| [v[0], v[1]]).collect(),
        bones: v4(n * 12),
        weights: v4(n * 16),
        masks: all[n * 20..n * 21].to_vec(),
        indices,
    })
}

pub fn mesh_from_arrays(arrays: FigureArrays) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, arrays.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, arrays.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, arrays.uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, arrays.colors);
    mesh.insert_attribute(ATTRIBUTE_BONES, VertexAttributeValues::Float32x4(arrays.bones));
    mesh.insert_attribute(
        ATTRIBUTE_WEIGHTS,
        VertexAttributeValues::Float32x4(arrays.weights),
    );
    mesh.insert_attribute(ATTRIBUTE_MASK, VertexAttributeValues::Float32(arrays.masks));
    mesh.insert_indices(Indices::U32(arrays.indices));
    mesh
}

pub fn load_mesh(path: &Path) -> Result<Mesh, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_mesh(&bytes)
        .map(mesh_from_arrays)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Decoded `CAB1`: bones, frames, raw RGBA32F texels.
pub fn decode_bones(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if bytes.len() <= 12 || &bytes[..4] != b"CAB1" {
        return Err("bad bone texture".into());
    }
    let bones = u32_at(bytes, 4);
    let frames = u32_at(bytes, 8);
    let data = inflate(&bytes[12..], bones as usize * 3 * frames as usize * 16)?;
    Ok((bones, frames, data))
}

/// Bone texture of a rig (its own frames, then the baked mocap layers in merge order):
/// `bones * 3` texels wide, one row per frame.
pub fn load_bone_texture(asset_root: &Path, rig: &RigEntry) -> Result<Image, String> {
    let read = |file: &str| -> Result<(u32, u32, Vec<u8>), String> {
        let path: PathBuf = asset_root.join(relative_asset_path(file));
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        decode_bones(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    };
    let (bones, mut frames, mut data) = read(&rig.texture)?;
    for layer in &rig.mocap_textures {
        match read(layer) {
            Ok((b, f, extra)) if b == bones => {
                data.extend_from_slice(&extra);
                frames += f;
            }
            Ok(_) => warn!("bone texture {layer}: bone count differs, skipped"),
            Err(error) => warn!("bone texture layer skipped: {error}"),
        }
    }
    Ok(Image::new(
        Extent3d {
            width: bones * 3,
            height: frames,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        crate::paths::asset_root(&crate::paths::find_root().expect("repo root"))
    }

    #[test]
    fn manifest_lists_figures_and_rigs() {
        let manifest = Manifest::load(&root()).expect("manifest");
        assert!(manifest.figures.len() >= 30);
        let (variant, infantry) = manifest.figure_or_default("infantry", 0).expect("infantry_0");
        assert_eq!(variant, 0);
        assert_eq!(infantry.lods.len(), 3);
        let rig = manifest.rig(&infantry.rig).expect("rig");
        assert!(rig.clip_count() <= MAX_CLIPS, "{}", rig.clip_count());
        assert!(rig.clips().get("walk").is_some());
        assert!(manifest.is_noble("infantry", 0));
        assert!(manifest.figure_or_default("infantry", 99).is_some());
    }

    #[test]
    fn meshes_decode_with_consistent_counts() {
        let manifest = Manifest::load(&root()).expect("manifest");
        for (kind, level) in [("infantry", 2), ("cavalry", 2), ("archer", 1)] {
            let fig = manifest.figure(kind, 0).expect("figure");
            let path = root().join(relative_asset_path(&fig.lods[level]));
            let bytes = std::fs::read(path).unwrap();
            let arrays = decode_mesh(&bytes).expect("decode");
            let n = arrays.positions.len();
            assert!(n > 100);
            assert_eq!(arrays.bones.len(), n);
            assert_eq!(arrays.masks.len(), n);
            assert_eq!(arrays.indices.len() % 3, 0);
            assert!(arrays.indices.iter().all(|i| (*i as usize) < n));
            // Weights sum to ~1 and bone ids are small integers.
            let sum: f32 = arrays.weights[n / 2].iter().sum();
            assert!((sum - 1.0).abs() < 0.05, "{sum}");
            assert!(arrays.bones[n / 2][0].fract() == 0.0);
        }
    }

    #[test]
    fn bone_textures_have_one_row_per_frame() {
        let manifest = Manifest::load(&root()).expect("manifest");
        let fig = manifest.figure("infantry", 0).unwrap();
        let rig = manifest.rig(&fig.rig).unwrap();
        let image = load_bone_texture(&root(), rig).expect("texture");
        assert_eq!(image.width(), rig.bones * 3);
        let clips = rig.clips();
        let last = clips
            .names
            .iter()
            .filter_map(|n| clips.get(n))
            .map(|c| c.start + c.frames)
            .max()
            .unwrap();
        assert!(image.height() >= last, "{} < {last}", image.height());
    }
}
