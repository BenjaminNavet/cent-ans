//! FG3 fine maps of the figurines (port of `BattleSkinned.fine_maps` in
//! game/scripts/battle/battle_skinned.gd): six PNG files of `assets/models/battle_fine/textures/`.
//!
//! - `fine_atlas_lod0.png` / `fine_atlas_lod1.png`: one 512² / 256² layer per figurine
//!   (`atlas_layer` of the manifest): RG shape normal, B baked occlusion, A mask by material.
//!   The level of detail picks the atlas: LOD0 meshes carry source 1 in their packed atlas uv,
//!   LOD1 meshes source 3, the horse coat 2 (`ATTRIBUTE_ATLAS`); LOD2 and impostors read nothing.
//! - `fine_detail_ga1.png` (12 layers, RG normal, B relief, A roughness) and
//!   `fine_detail_albedo.png` (12 layers, centred albedo): tiles shared by material. As in Godot
//!   the legacy `fine_detail.png` (8 layers) is only the fallback when the GA1 pair is missing.
//! - `fine_horse.png`: 1024² reduced CC0 coat (RG normal, B relief, A occlusion), repeated.
//!
//! The PNG strips are stacks of square layers; the decode and the mip chains run on a thread and
//! the images are added to the asset store when ready. The render world binds them at
//! bindings 6-10 of `skinned.rs` (fallbacks and `layer = -1` until then).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor},
};
use serde_json::json;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::decor::ground::{mip_chain, mip_count};

/// Distance (m) beyond which no map is read (`fine_distance` of the Godot shader).
pub const FINE_DISTANCE: f32 = 80.0;

/// One map: file, layer count (1 = plain 2D texture), side of a layer, repeats.
struct MapSpec {
    key: &'static str,
    file: &'static str,
    layers: u32,
    repeat: bool,
}

const ATLAS0: MapSpec = MapSpec {
    key: "atlas_lod0",
    file: "fine_atlas_lod0.png",
    layers: 32,
    repeat: false,
};
const ATLAS1: MapSpec = MapSpec {
    key: "atlas_lod1",
    file: "fine_atlas_lod1.png",
    layers: 32,
    repeat: false,
};
const DETAIL_GA1: MapSpec = MapSpec {
    key: "detail",
    file: "fine_detail_ga1.png",
    layers: 12,
    repeat: true,
};
const DETAIL_LEGACY: MapSpec = MapSpec {
    key: "detail",
    file: "fine_detail.png",
    layers: 8,
    repeat: true,
};
const DETAIL_ALBEDO: MapSpec = MapSpec {
    key: "detail_albedo",
    file: "fine_detail_albedo.png",
    layers: 12,
    repeat: true,
};
const HORSE: MapSpec = MapSpec {
    key: "horse",
    file: "fine_horse.png",
    layers: 1,
    repeat: true,
};

/// Decoded strip: all layers with their mip chains, layer after layer.
struct Decoded {
    spec: &'static MapSpec,
    width: u32,
    layers: u32,
    data: Vec<u8>,
}

/// Handles of the loaded maps (render world copy: `ExtractResource`).
#[derive(Resource, Default, Clone)]
pub struct FineMaps {
    pub atlas0: Option<Handle<Image>>,
    pub atlas1: Option<Handle<Image>>,
    pub detail: Option<Handle<Image>>,
    pub detail_albedo: Option<Handle<Image>>,
    pub horse: Option<Handle<Image>>,
    /// Layer counts / side, for the dump.
    pub info: Vec<(String, u32, u32, u32)>,
    /// Which detail file is in use (`ga1` or `legacy`).
    pub detail_kind: &'static str,
    pub error: Option<String>,
}

impl FineMaps {
    /// All maps are bound (the shader may read them).
    pub fn ready(&self) -> bool {
        self.atlas0.is_some()
            && self.atlas1.is_some()
            && self.detail.is_some()
            && self.detail_albedo.is_some()
            && self.horse.is_some()
    }
}

impl ExtractResource<bevy::render::RenderApp> for FineMaps {
    type Source = Self;

    fn extract_resource(source: &Self) -> Self {
        source.clone()
    }
}

#[derive(Resource)]
struct Loading(Mutex<mpsc::Receiver<Result<Vec<Decoded>, String>>>);

/// Decodes one PNG strip into RGBA8 layers with mipmaps.
fn decode(dir: &Path, spec: &'static MapSpec) -> Result<Decoded, String> {
    let path = dir.join(spec.file);
    let file = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{}: {e}", spec.file))?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("png too large")?];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{}: {e}", spec.file))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = info.color_type.samples();
    if w * spec.layers as usize != h {
        return Err(format!(
            "{}: {w}x{h} is not {} square layers",
            spec.file, spec.layers
        ));
    }
    let rgba: Vec<u8> = if channels == 4 {
        buf[..w * h * 4].to_vec()
    } else {
        (0..w * h)
            .flat_map(|i| {
                let p = &buf[i * channels..(i + 1) * channels];
                match channels {
                    1 => [p[0], p[0], p[0], 255],
                    2 => [p[0], p[0], p[0], p[1]],
                    _ => [p[0], p[1], p[2], 255],
                }
            })
            .collect()
    };
    let layer_bytes = w * w * 4;
    let mut data = Vec::with_capacity(rgba.len() * 4 / 3 + 64);
    for layer in 0..spec.layers as usize {
        data.extend_from_slice(&mip_chain(
            &rgba[layer * layer_bytes..(layer + 1) * layer_bytes],
            w,
        ));
    }
    Ok(Decoded {
        spec,
        width: w as u32,
        layers: spec.layers,
        data,
    })
}

fn decode_all(dir: PathBuf) -> Result<Vec<Decoded>, String> {
    let detail = if dir.join(DETAIL_GA1.file).exists() && dir.join(DETAIL_ALBEDO.file).exists() {
        &DETAIL_GA1
    } else {
        &DETAIL_LEGACY
    };
    let mut out = Vec::new();
    for spec in [&ATLAS0, &ATLAS1, detail, &DETAIL_ALBEDO, &HORSE] {
        // Without the GA1 pair the albedo layers are absent: fall back to a neutral strip below.
        if std::ptr::eq(spec, &DETAIL_ALBEDO) && std::ptr::eq(detail, &DETAIL_LEGACY) {
            continue;
        }
        out.push(decode(&dir, spec)?);
    }
    Ok(out)
}

fn image_of(decoded: Decoded) -> Image {
    let size = decoded.width;
    let array = decoded.layers > 1;
    let mut image = Image::new_uninit(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: decoded.layers,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(decoded.data);
    image.texture_descriptor.mip_level_count = mip_count(size);
    if array {
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(bevy::render::render_resource::TextureViewDimension::D2Array),
            ..default()
        });
    }
    let mode = if decoded.spec.repeat {
        ImageAddressMode::Repeat
    } else {
        ImageAddressMode::ClampToEdge
    };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
    image
}

pub struct FineMapsPlugin;

impl Plugin for FineMapsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FineMaps>()
            .add_plugins(ExtractResourcePlugin::<FineMaps>::default())
            .register_dump(dump_fine_maps)
            .add_systems(OnEnter(AppState::Battle), start_loading)
            .add_systems(Update, receive.run_if(resource_exists::<Loading>));
    }
}

/// Starts the decode thread once (the maps stay for the next battles).
fn start_loading(mut commands: Commands, maps: Res<FineMaps>, loading: Option<Res<Loading>>) {
    if maps.ready() || loading.is_some() || maps.error.is_some() {
        return;
    }
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let dir = crate::paths::asset_root(&root).join("models/battle_fine/textures");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(decode_all(dir));
    });
    commands.insert_resource(Loading(Mutex::new(receiver)));
}

fn receive(
    mut commands: Commands,
    loading: Res<Loading>,
    mut maps: ResMut<FineMaps>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(result) = loading.0.lock().expect("fine maps channel").try_recv() else {
        return;
    };
    commands.remove_resource::<Loading>();
    match result {
        Err(error) => {
            warn!("fine maps (FG3) not loaded: {error}");
            maps.error = Some(error);
        }
        Ok(decoded) => {
            maps.detail_kind = "ga1";
            let mut has_albedo = false;
            for item in decoded {
                let (key, size, layers) = (item.spec.key, item.width, item.layers);
                maps.info
                    .push((item.spec.file.to_string(), size, layers, mip_count(size)));
                if std::ptr::eq(item.spec, &DETAIL_LEGACY) {
                    maps.detail_kind = "legacy";
                }
                has_albedo |= key == "detail_albedo";
                let handle = images.add(image_of(item));
                match key {
                    "atlas_lod0" => maps.atlas0 = Some(handle),
                    "atlas_lod1" => maps.atlas1 = Some(handle),
                    "detail" => maps.detail = Some(handle),
                    "detail_albedo" => maps.detail_albedo = Some(handle),
                    _ => maps.horse = Some(handle),
                }
            }
            if !has_albedo {
                // Legacy detail tiles: a neutral albedo strip (the shader mixes it by 0).
                let neutral = Decoded {
                    spec: &DETAIL_ALBEDO,
                    width: 4,
                    layers: 12,
                    data: vec![128; (4 * 4 + 2 * 2 + 1) * 4 * 12],
                };
                maps.detail_albedo = Some(images.add(image_of(neutral)));
            }
        }
    }
}

fn dump_fine_maps(world: &World) -> (String, serde_json::Value) {
    let Some(maps) = world.get_resource::<FineMaps>() else {
        return ("battle_fine_maps".into(), serde_json::Value::Null);
    };
    let files: Vec<serde_json::Value> = maps
        .info
        .iter()
        .map(|(file, size, layers, mips)| json!({"file": file, "side": size, "layers": layers, "mips": mips}))
        .collect();
    // Textures bound per figure batch (what `skinned.rs` binds to a draw) and the atlas
    // source of the batch's current mesh (1 LOD0 atlas, 3 LOD1 atlas, 2 horse coat).
    let meshes = world.get_resource::<Assets<Mesh>>();
    let mut batches = Vec::new();
    let mut by_layer = 0;
    let mut by_source: std::collections::BTreeMap<u32, usize> = Default::default();
    for entity in world.iter_entities() {
        let Some(batch) = entity.get::<super::skinned::SkinnedBatch>() else {
            continue;
        };
        let layer = super::skinned::fine_layer_of(&batch.params);
        by_layer += usize::from(layer >= 0);
        let sources: Vec<u32> = meshes
            .and_then(|m| m.get(&batch.mesh_handle))
            .and_then(|mesh| mesh.attribute(super::figures::ATTRIBUTE_ATLAS))
            .and_then(|a| match a {
                bevy::mesh::VertexAttributeValues::Float32(v) => Some(v),
                _ => None,
            })
            .map(|v| {
                let mut set: Vec<u32> = v
                    .iter()
                    .filter(|p| **p > 0.0)
                    .map(|p| (*p as u32 >> 22) & 3)
                    .collect();
                set.sort_unstable();
                set.dedup();
                set
            })
            .unwrap_or_default();
        for source in &sources {
            *by_source.entry(*source).or_default() += 1;
        }
        let name = entity
            .get::<Name>()
            .map(|n| n.as_str().to_owned())
            .unwrap_or_default();
        batches.push(json!({
            "batch": name,
            "fine_layer": layer,
            "mesh_atlas_sources": sources,
            "ga3_albedo": batch.albedo.is_some(),
            "bound": if layer >= 0 && maps.ready() {
                json!(["atlas_lod0", "atlas_lod1", "detail", "detail_albedo", "horse"])
            } else {
                json!([])
            },
        }));
    }
    let listed: Vec<_> = batches.iter().take(40).cloned().collect();
    (
        "battle_fine_maps".into(),
        json!({
            "ready": maps.ready(),
            "error": maps.error,
            "detail": maps.detail_kind,
            "files": files,
            "distance": FINE_DISTANCE,
            "batches": batches.len(),
            "batches_with_fine_layer": by_layer,
            "batches_by_atlas_source": by_source,
            "per_batch": listed,
        }),
    )
}
