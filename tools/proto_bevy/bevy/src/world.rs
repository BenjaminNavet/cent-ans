//! World construction: terrain, glTF placements, GPU-batched army and vegetation.

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    gltf::{Gltf, GltfAssetLabel, GltfMaterial, GltfMesh, GltfNode},
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    mesh::{Indices, MeshTag, PrimitiveTopology},
    pbr::gltf::standard_material_from_gltf_material,
    prelude::*,
    world_serialization::WorldAssetRoot,
};

use std::sync::Arc;

use crate::instancing::{InstancedBatch, MaterialParams};
use crate::scene::{HeightField, Instancing, Record, Scene, VegetationSpec, parse_records};
use crate::walk::{WalkExt, WalkMaterial, WalkParams};

/// Terrain is cut into `CHUNKS × CHUNKS` meshes so shadow cascades and the frustum can cull it.
pub const CHUNKS: usize = 8;

/// Handles that must be loaded before the world can be built.
#[derive(Resource)]
pub struct Pending {
    pub gltfs: HashMap<String, Handle<Gltf>>,
    pub textures: [Handle<Image>; 3],
}

/// What was spawned, for `--dump-state` and the walk-time updates.
#[derive(Resource, Default)]
pub struct Counts {
    pub placements: usize,
    pub army: usize,
    pub grass: usize,
    pub firs: usize,
    pub walk_materials: Vec<Handle<WalkMaterial>>,
}

fn mipped_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    })
}

/// Same-length fix of `"texCoord":-1` (invalid for the `gltf` crate, tolerated by Godot).
pub fn patch_tex_coord(bytes: &[u8]) -> Option<Vec<u8>> {
    const BAD: &[u8] = b"\"texCoord\":-1";
    const GOOD: &[u8] = b"\"texCoord\": 0";
    let mut out = bytes.to_vec();
    let mut changed = false;
    let mut at = 0;
    while at + BAD.len() <= out.len() {
        if &out[at..at + BAD.len()] == BAD {
            out[at..at + BAD.len()].copy_from_slice(GOOD);
            changed = true;
            at += BAD.len();
        } else {
            at += 1;
        }
    }
    changed.then_some(out)
}

/// Returns the asset path to load: the original, or a patched copy under `bevy/target/patched`.
fn loadable_path(root: &std::path::Path, path: String) -> String {
    let Ok(bytes) = std::fs::read(root.join(&path)) else {
        return path;
    };
    let Some(patched) = patch_tex_coord(&bytes) else {
        return path;
    };
    let name = std::path::Path::new(&path).file_name().unwrap_or_default();
    let relative = std::path::Path::new("bevy/target/patched").join(name);
    let target = root.join(&relative);
    if let Some(dir) = target.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::write(&target, patched).is_ok() {
        return relative.to_string_lossy().into_owned();
    }
    path
}

/// Starts loading every glTF the scene needs plus the terrain textures.
pub fn start_loading(scene: &Scene, asset_server: &AssetServer) -> Result<Pending, String> {
    let mut names: Vec<&str> = scene.placements.iter().map(|p| p.asset.as_str()).collect();
    names.extend(scene.army.models.iter().map(String::as_str));
    names.push(&scene.vegetation.grass.asset);
    names.push(&scene.vegetation.firs.asset);
    let mut gltfs = HashMap::new();
    for name in names {
        if !gltfs.contains_key(name) {
            let path = loadable_path(&crate::scene::proto_root(), scene.asset_path(name)?);
            gltfs.insert(name.to_owned(), asset_server.load::<Gltf>(path));
        }
    }
    let texture = |suffix: &str, srgb: bool| {
        let path = format!("assets/{}_{suffix}.jpg", scene.heightmap.texture);
        asset_server
            .load_builder()
            .with_settings(move |settings: &mut ImageLoaderSettings| {
                settings.is_srgb = srgb;
                settings.sampler = mipped_sampler();
            })
            .load::<Image>(path)
    };
    let textures = [
        texture("Diffuse", true),
        texture("nor_gl", false),
        texture("Rough", false),
    ];
    Ok(Pending { gltfs, textures })
}

/// True when any pending asset failed to load.
pub fn any_failed(pending: &Pending, asset_server: &AssetServer) -> bool {
    pending
        .gltfs
        .values()
        .map(|handle| handle.id().untyped())
        .chain(pending.textures.iter().map(|handle| handle.id().untyped()))
        .any(|id| asset_server.load_state(id).is_failed())
}

/// True once every pending asset and its dependencies are loaded.
pub fn all_loaded(pending: &Pending, asset_server: &AssetServer) -> bool {
    pending
        .gltfs
        .values()
        .map(|handle| handle.id().untyped())
        .chain(pending.textures.iter().map(|handle| handle.id().untyped()))
        .all(|id| asset_server.is_loaded_with_dependencies(id))
}

/// Builds the mesh of terrain chunk `(chunk_row, chunk_col)` with normals, tangents and UVs.
pub fn terrain_chunk(
    field: &HeightField,
    uv_tile_m: f32,
    chunk_row: usize,
    chunk_col: usize,
) -> Mesh {
    let quads = (field.grid - 1) / CHUNKS;
    let side = quads + 1;
    let step = field.step();
    let last = field.grid - 1;
    let mut positions = Vec::with_capacity(side * side);
    let mut normals = Vec::with_capacity(side * side);
    let mut tangents = Vec::with_capacity(side * side);
    let mut uvs = Vec::with_capacity(side * side);
    for local_row in 0..side {
        let row = chunk_row * quads + local_row;
        for local_col in 0..side {
            let col = chunk_col * quads + local_col;
            let (x, z) = (field.coord(col), field.coord(row));
            let y = field.at(row, col);
            let dx = (field.at(row, (col + 1).min(last)) - field.at(row, col.saturating_sub(1)))
                / ((col + 1).min(last) - col.saturating_sub(1)) as f32
                / step;
            let dz = (field.at((row + 1).min(last), col) - field.at(row.saturating_sub(1), col))
                / ((row + 1).min(last) - row.saturating_sub(1)) as f32
                / step;
            let normal = Vec3::new(-dx, 1.0, -dz).normalize();
            let tangent = Vec3::new(1.0, dx, 0.0).normalize();
            positions.push([x, y, z]);
            normals.push(normal.to_array());
            // Normal map is OpenGL style: bitangent points to -Z (up in the image).
            tangents.push([tangent.x, tangent.y, tangent.z, 1.0]);
            uvs.push([x / uv_tile_m, z / uv_tile_m]);
        }
    }
    let mut indices = Vec::with_capacity(quads * quads * 6);
    for row in 0..quads {
        for col in 0..quads {
            let a = (row * side + col) as u32;
            let b = a + 1;
            let c = a + side as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, tangents)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Meshes and glTF materials of the given nodes (`node` empty = every node of the file).
fn primitives(
    gltf: &Gltf,
    node: &str,
    nodes: &Assets<GltfNode>,
    gltf_meshes: &Assets<GltfMesh>,
) -> Result<Primitives, String> {
    let node_handles: Vec<&Handle<GltfNode>> = if node.is_empty() {
        gltf.nodes.iter().collect()
    } else {
        vec![
            gltf.named_nodes
                .get(node)
                .ok_or_else(|| format!("node {node} missing from glTF"))?,
        ]
    };
    let mut found = Vec::new();
    for handle in node_handles {
        let node = nodes.get(handle).ok_or("glTF node not loaded")?;
        if let Some(mesh_handle) = &node.mesh {
            let mesh = gltf_meshes.get(mesh_handle).ok_or("glTF mesh not loaded")?;
            for primitive in &mesh.primitives {
                found.push((primitive.mesh.clone(), primitive.material.clone()));
            }
        }
    }
    Ok(found)
}

fn instance_transform(record: &Record) -> Transform {
    Transform {
        translation: Vec3::from_array(record.pos),
        rotation: Quat::from_rotation_y(record.yaw_deg.to_radians()),
        scale: Vec3::splat(record.scale),
    }
}

fn read_records(root: &std::path::Path, file: &str) -> Result<Vec<Record>, String> {
    let bytes = std::fs::read(root.join(file)).map_err(|error| format!("{file}: {error}"))?;
    parse_records(&bytes)
}

/// Everything `build_world` needs from the ECS.
#[derive(bevy::ecs::system::SystemParam)]
pub struct BuildParams<'w> {
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub standard: ResMut<'w, Assets<StandardMaterial>>,
    pub walk: ResMut<'w, Assets<WalkMaterial>>,
    pub gltfs: Res<'w, Assets<Gltf>>,
    pub nodes: Res<'w, Assets<GltfNode>>,
    pub gltf_meshes: Res<'w, Assets<GltfMesh>>,
    pub gltf_materials: Res<'w, Assets<GltfMaterial>>,
    pub asset_server: Res<'w, AssetServer>,
    pub images: ResMut<'w, Assets<Image>>,
}

fn standard_from(
    handle: &Option<Handle<GltfMaterial>>,
    materials: &Assets<GltfMaterial>,
) -> StandardMaterial {
    handle
        .as_ref()
        .and_then(|handle| materials.get(handle))
        .map(standard_material_from_gltf_material)
        .unwrap_or_default()
}

type Primitives = Vec<(Handle<Mesh>, Option<Handle<GltfMaterial>>)>;

/// The instanced shader reads position, normal and UV: add zero UVs / flat normals if absent.
fn ensure_instanced_attributes(meshes: &mut Assets<Mesh>, handle: &Handle<Mesh>) {
    let Some(mut mesh) = meshes.get_mut(handle) else {
        return;
    };
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_0) {
        warn!("mesh {:?} has no UV_0: using zeros", handle.id());
        let zeros = vec![[0.0f32, 0.0]; mesh.count_vertices()];
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, zeros);
    }
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_NORMAL) {
        warn!("mesh {:?} has no normals: computing flat ones", handle.id());
        mesh.compute_flat_normals();
    }
}

fn spawn_vegetation(
    commands: &mut Commands,
    prims: Primitives,
    standard: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    gltf_materials: &Assets<GltfMaterial>,
    spec: &VegetationSpec,
    instancing: Instancing,
    raw_records: &Arc<Vec<u8>>,
) -> Result<usize, String> {
    let records = parse_records(raw_records)?;
    let drawn = spec.drawn().min(records.len());
    for (mesh, material) in prims {
        let material = standard_from(&material, gltf_materials);
        if instancing == Instancing::Custom {
            ensure_instanced_attributes(meshes, &mesh);
            let color = material.base_color.to_linear();
            commands.spawn(InstancedBatch {
                mesh: mesh.id(),
                records: raw_records.clone(),
                count: drawn as u32,
                texture: material.base_color_texture.as_ref().map(Handle::id),
                params: MaterialParams {
                    base_color: [color.red, color.green, color.blue, color.alpha],
                    alpha_cutoff: match material.alpha_mode {
                        AlphaMode::Mask(cutoff) => cutoff,
                        _ => 0.0,
                    },
                    roughness: material.perceptual_roughness,
                    metallic: material.metallic,
                    double_sided: material.double_sided,
                },
            });
            continue;
        }
        let material = standard.add(material);
        let batch: Vec<_> = records[..drawn]
            .iter()
            .map(|record| {
                (
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    instance_transform(record),
                )
            })
            .collect();
        if !spec.cast_shadows {
            commands.spawn_batch(
                batch
                    .into_iter()
                    .map(|bundle| (bundle, bevy::light::NotShadowCaster)),
            );
        } else {
            commands.spawn_batch(batch);
        }
    }
    Ok(drawn)
}

/// Spawns terrain, placements, army, grass and firs. Returns the counts.
pub fn build_world(
    commands: &mut Commands,
    params: &mut BuildParams,
    scene: &Scene,
    root: &std::path::Path,
    pending: &Pending,
) -> Result<Counts, String> {
    let mut counts = Counts::default();

    // Terrain.
    let spec = &scene.heightmap;
    let bytes = std::fs::read(root.join(&spec.file)).map_err(|error| error.to_string())?;
    let field = HeightField::from_bytes(&bytes, spec.grid, spec.size_m)?;
    let [diffuse, normal, rough] = pending.textures.clone();
    let terrain_material = params.standard.add(StandardMaterial {
        base_color_texture: Some(diffuse),
        normal_map_texture: Some(normal),
        metallic_roughness_texture: Some(rough),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.25,
        ..default()
    });
    for chunk_row in 0..CHUNKS {
        for chunk_col in 0..CHUNKS {
            let mesh = terrain_chunk(&field, spec.uv_tile_m, chunk_row, chunk_col);
            commands.spawn((
                Mesh3d(params.meshes.add(mesh)),
                MeshMaterial3d(terrain_material.clone()),
                Transform::IDENTITY,
            ));
        }
    }

    // Placements.
    for placement in &scene.placements {
        let transform = Transform {
            translation: Vec3::from_array(placement.pos),
            rotation: Quat::from_rotation_y(placement.yaw.to_radians()),
            scale: Vec3::splat(placement.scale),
        };
        if placement.node.is_empty() {
            let path = loadable_path(root, scene.asset_path(&placement.asset)?);
            let world = params
                .asset_server
                .load(GltfAssetLabel::Scene(0).from_asset(path));
            commands.spawn((WorldAssetRoot(world), transform));
        } else {
            let gltf = params
                .gltfs
                .get(&pending.gltfs[&placement.asset])
                .ok_or("placement glTF not loaded")?;
            let prims = primitives(gltf, &placement.node, &params.nodes, &params.gltf_meshes)?;
            for (mesh, material) in prims {
                let material = params
                    .standard
                    .add(standard_from(&material, &params.gltf_materials));
                commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), transform));
            }
        }
        counts.placements += 1;
    }

    // Army: one walk material per (model, primitive); every soldier shares them, so Bevy's
    // automatic batching merges the 5000 instances of a primitive into one indirect draw.
    let army = read_records(root, &scene.army.file)?;
    let walk = &scene.army.walk;
    let walk_params = WalkParams {
        t0: 1.0e9,
        hz: walk.hz,
        bob_m: walk.bob_m,
        sway_rad: walk.sway_deg.to_radians(),
    };
    let mut per_model = Vec::new();
    for model in &scene.army.models {
        let gltf = params
            .gltfs
            .get(&pending.gltfs[model])
            .ok_or("army glTF not loaded")?;
        let mut parts = Vec::new();
        for (mesh, material) in primitives(gltf, "", &params.nodes, &params.gltf_meshes)? {
            let base = standard_from(&material, &params.gltf_materials);
            let handle = params.walk.add(WalkMaterial {
                base,
                extension: WalkExt {
                    params: walk_params,
                },
            });
            counts.walk_materials.push(handle.clone());
            parts.push((mesh, handle));
        }
        per_model.push(parts);
    }
    let army = &army[..scene.army.count.min(army.len())];
    for (model_index, parts) in per_model.iter().enumerate() {
        for (mesh, material) in parts {
            let batch: Vec<_> = army
                .iter()
                .enumerate()
                .filter(|(_, record)| record.model == model_index)
                .map(|(index, record)| {
                    (
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        instance_transform(record),
                        MeshTag::new(index as u32),
                    )
                })
                .collect();
            commands.spawn_batch(batch);
        }
    }
    counts.army = army.len();

    // Vegetation.
    let veg = &scene.vegetation;
    for (spec, slot) in [
        (&veg.grass, &mut counts.grass),
        (&veg.firs, &mut counts.firs),
    ] {
        let gltf = params
            .gltfs
            .get(&pending.gltfs[&spec.asset])
            .ok_or("vegetation glTF not loaded")?;
        let prims = primitives(gltf, &spec.node, &params.nodes, &params.gltf_meshes)?;
        let raw = Arc::new(std::fs::read(root.join(&spec.file)).map_err(|e| e.to_string())?);
        *slot = spawn_vegetation(
            commands,
            prims,
            &mut params.standard,
            &mut params.meshes,
            &params.gltf_materials,
            spec,
            scene.instancing,
            &raw,
        )?;
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tex_coord_patch_keeps_length() {
        let input = br#"{"normalTexture":{"index":0,"texCoord":-1},"x":1}"#;
        let patched = patch_tex_coord(input).unwrap();
        assert_eq!(patched.len(), input.len());
        assert!(serde_json::from_slice::<serde_json::Value>(&patched).is_ok());
        assert!(patch_tex_coord(b"{}").is_none());
    }

    #[test]
    fn terrain_chunk_has_expected_shape() {
        let grid = 17;
        let heights: Vec<f32> = (0..grid * grid).map(|i| (i % grid) as f32 * 0.5).collect();
        let field = HeightField {
            grid,
            size_m: 16.0,
            heights,
        };
        let mesh = terrain_chunk(&field, 3.0, 1, 1);
        let quads = (grid - 1) / CHUNKS;
        assert_eq!(mesh.count_vertices(), (quads + 1) * (quads + 1));
        assert_eq!(mesh.indices().unwrap().len(), quads * quads * 6);
        // Slope of 0.5 m per metre along x tilts the normal toward -x.
        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .and_then(|values| values.as_float3())
            .unwrap();
        assert!(normals[0][0] < 0.0 && normals[0][1] > 0.0);
    }
}
