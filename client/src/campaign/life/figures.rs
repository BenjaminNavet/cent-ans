//! Figures of the living map: the villagers, workers, merchants, guards and monks are the figure
//! meshes of the battle (`models/battle_skinned|battle_fine/*.mesh.bin`, `CAM1`) baked into static
//! meshes of the rest pose, with the colours of their role (`data/rules/folk_props.json`).
//!
//! Differences with Godot (`FolkModels.ROLES`, `BattleSkinned` states): the figures are not
//! animated (no bone texture: the walk is a step bounce of the root, the work poses are the rest
//! pose), held tools and shields are dropped, and the livery of a role replaces the painted atlas.
//! A role without any readable figure falls back to the capsule stand-in of `folk.rs`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::data::LifeData;
use super::props::PropTable;
use crate::battle::units::figures::{FigureArrays, Manifest, decode_mesh, relative_asset_path};

/// Material codes of the baked vertex colour (`C_*` of `skinned.wesl`).
mod code {
    pub const LIVERY: i32 = 0;
    pub const CLOTH: i32 = 4;
    pub const ARMS: i32 = 6;
    pub const SKIN: i32 = 7;
    pub const HAIR: i32 = 9;
    pub const COAT: i32 = 10;
    pub const QUILT: i32 = 12;
}

/// Skin tones (`skin_colour` of the shader) and hair tones, picked per figure.
const SKIN: [[f32; 3]; 5] = [
    [1.0, 0.93, 0.85],
    [0.9, 0.78, 0.66],
    [0.78, 0.6, 0.48],
    [0.66, 0.5, 0.4],
    [0.55, 0.4, 0.32],
];
const HAIR: [[f32; 3]; 5] = [
    [0.16, 0.07, 0.03],
    [0.26, 0.1, 0.03],
    [0.03, 0.025, 0.022],
    [0.34, 0.33, 0.31],
    [0.36, 0.26, 0.13],
];

/// Number of colour variants baked per role (drab wool x skin).
pub const TINTS: usize = 4;

/// Look picked for one figure: variant of the file (parts shown) and tint index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Look {
    pub variant: u32,
    pub tint: usize,
}

impl Look {
    /// From a fixed random number of the figure.
    pub fn of(h: f32, h2: f32, variants: u32) -> Look {
        Look {
            variant: ((h * variants.max(1) as f32) as u32).min(variants.max(1) - 1),
            tint: ((h2 * TINTS as f32) as usize).min(TINTS - 1),
        }
    }
}

fn mul(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * b[0] * k, a[1] * b[1] * k, a[2] * b[2] * k]
}

/// Static mesh of the rest pose with the parts of `look.variant` and the colours of the role.
pub fn bake(arrays: &FigureArrays, look: Look, livery: [f32; 3], drab: [f32; 3]) -> Mesh {
    let skin = SKIN[(look.tint + look.variant as usize) % SKIN.len()];
    let hair = HAIR[(look.tint * 2 + look.variant as usize) % HAIR.len()];
    let lum = livery[0] * 0.3 + livery[1] * 0.59 + livery[2] * 0.11;
    let dyed = [
        livery[0] * 0.7 + lum * 0.3,
        livery[1] * 0.7 + lum * 0.3,
        livery[2] * 0.7 + lum * 0.3,
    ];
    let shown = |mask: f32| -> bool {
        let m = (mask + 0.5) as i32;
        let bits = m & 63;
        // Bits 6 and 7 are the held weapon and the pavise: never shown here.
        if m & 192 != 0 && bits == 0 {
            return false;
        }
        bits == 0 || (bits >> look.variant.min(5)) & 1 == 1
    };
    let colour = |c: [f32; 4]| -> [f32; 4] {
        let base = [c[0], c[1], c[2]];
        let rgb = match (c[3] + 0.5) as i32 {
            code::LIVERY => mul(dyed, base, 0.78),
            code::ARMS => mul(dyed, [1.0; 3], 0.78),
            code::CLOTH | code::QUILT | code::COAT => mul(drab, base, 1.2),
            code::SKIN => mul(skin, base, 1.0),
            code::HAIR => hair,
            _ => base,
        };
        [rgb[0], rgb[1], rgb[2], 1.0]
    };
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut remap: HashMap<u32, u32> = HashMap::new();
    for tri in arrays.indices.as_chunks::<3>().0 {
        if !tri.iter().all(|&i| shown(arrays.masks[i as usize])) {
            continue;
        }
        for &i in tri {
            let next = positions.len() as u32;
            let new = *remap.entry(i).or_insert_with(|| {
                positions.push(arrays.positions[i as usize]);
                normals.push(arrays.normals[i as usize]);
                colors.push(colour(arrays.colors[i as usize]));
                next
            });
            indices.push(new);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Cache of the decoded figure files and of the baked meshes.
#[derive(Resource, Default)]
pub struct FigureLibrary {
    manifest: Option<Manifest>,
    root: PathBuf,
    raw: HashMap<(String, u32, u8), Option<Arc<FigureArrays>>>,
    meshes: HashMap<(String, u32, u8, String, Look), Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
    /// Resolved (family, variant) per role.
    figure_of_role: HashMap<String, Option<(String, u32)>>,
    capsule: Option<(Handle<Mesh>, Handle<Mesh>)>,
    role_materials: HashMap<String, Handle<StandardMaterial>>,
}

impl FigureLibrary {
    pub fn new(asset_root: PathBuf) -> FigureLibrary {
        let manifest = Manifest::load(&asset_root).ok();
        FigureLibrary {
            manifest,
            root: asset_root,
            ..default()
        }
    }

    pub fn available(&self) -> bool {
        self.manifest.is_some()
    }

    fn lod_file(&self, kind: &str, variant: u32, lod: u8) -> Option<PathBuf> {
        let entry = self.manifest.as_ref()?.figure(kind, variant)?;
        let file = entry.lods.get(lod as usize).or_else(|| entry.lods.last())?;
        let path = self.root.join(relative_asset_path(file));
        path.is_file().then_some(path)
    }

    fn present(&self, kind: &str, variant: u32) -> bool {
        self.lod_file(kind, variant, 2).is_some()
    }

    /// Number of variants of the file of a figure.
    pub fn variants_of(&self, kind: &str, variant: u32) -> u32 {
        self.manifest
            .as_ref()
            .and_then(|m| m.figure(kind, variant))
            .map_or(1, |f| f.variants.max(1))
    }

    fn arrays(&mut self, kind: &str, variant: u32, lod: u8) -> Option<Arc<FigureArrays>> {
        let key = (kind.to_string(), variant, lod);
        if let Some(found) = self.raw.get(&key) {
            return found.clone();
        }
        let decoded = self
            .lod_file(kind, variant, lod)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| decode_mesh(&b).ok())
            .map(Arc::new);
        self.raw.insert(key, decoded.clone());
        decoded
    }

    /// Figure (family, variant) of a role: first candidate whose file is readable.
    pub fn figure_of(&mut self, table: &PropTable, role: &str) -> Option<(String, u32)> {
        if let Some(found) = self.figure_of_role.get(role) {
            return found.clone();
        }
        let found = table.figure_of(role, &|k, v| self.present(k, v));
        self.figure_of_role.insert(role.to_string(), found.clone());
        found
    }

    pub fn material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        self.material
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.92,
                    ..default()
                })
            })
            .clone()
    }

    /// Mesh of a role for a look, at a LOD (0 fine, 2 distant), cached.
    pub fn mesh_for(
        &mut self,
        table: &PropTable,
        meshes: &mut Assets<Mesh>,
        role: &str,
        lod: u8,
        h: f32,
        h2: f32,
    ) -> Option<Handle<Mesh>> {
        let (kind, variant) = self.figure_of(table, role)?;
        let look = Look::of(h, h2, self.variants_of(&kind, variant));
        let key = (kind.clone(), variant, lod, role.to_string(), look);
        if let Some(handle) = self.meshes.get(&key) {
            return Some(handle.clone());
        }
        let arrays = self.arrays(&kind, variant, lod)?;
        let drab = table
            .drab
            .get(look.tint % table.drab.len().max(1))
            .copied()
            .unwrap_or([0.5, 0.45, 0.35]);
        let handle = meshes.add(bake(&arrays, look, table.livery_of(role), drab));
        self.meshes.insert(key, handle.clone());
        Some(handle)
    }

    pub fn baked(&self) -> usize {
        self.meshes.len()
    }
}

/// A figure root (counted by the dump); `fallback` when the capsule stand-in is used.
#[derive(Component, Debug, Clone)]
pub struct Figure {
    pub role: String,
    pub fallback: bool,
}

/// Spawns figures: baked battle mesh of the role, else the capsule stand-in.
#[derive(SystemParam)]
pub struct Figures<'w> {
    pub library: ResMut<'w, FigureLibrary>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub data: Res<'w, LifeData>,
}

impl Figures<'_> {
    /// Spawns a figure of `role` at the local position `at` (map units) with its own fixed random
    /// numbers `h` (variant) and `h2` (tint), at the world scale `ws` (map units per metre) and
    /// the heading axis +X. `lod` 0 is fine, 2 distant.
    pub fn spawn(
        &mut self,
        commands: &mut Commands,
        role: &str,
        lod: u8,
        h: f32,
        h2: f32,
        ws: f32,
        at: Vec3,
    ) -> Entity {
        let table = &self.data.table;
        let baked = self
            .library
            .mesh_for(table, &mut self.meshes, role, lod, h, h2);
        let livery = table.livery_of(role);
        match baked {
            Some(mesh) => {
                let material = self.library.material(&mut self.materials);
                commands
                    .spawn((
                        Name::new("life:figure"),
                        Figure {
                            role: role.to_string(),
                            fallback: false,
                        },
                        // The mesh looks along +Z, the heading axis of the instance is +X.
                        Transform::from_translation(at)
                            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
                            .with_scale(Vec3::splat(ws)),
                        Visibility::Inherited,
                    ))
                    .with_child((Mesh3d(mesh), MeshMaterial3d(material)))
                    .id()
            }
            None => {
                let (body, head) = self
                    .library
                    .capsule
                    .get_or_insert_with(|| {
                        (
                            self.meshes.add(Capsule3d::new(0.2, 1.1)),
                            self.meshes.add(
                                Sphere::new(0.14)
                                    .mesh()
                                    .ico(1)
                                    .unwrap_or_else(|_| Sphere::new(0.14).mesh().into()),
                            ),
                        )
                    })
                    .clone();
                let material = self
                    .library
                    .role_materials
                    .entry(role.to_string())
                    .or_insert_with(|| {
                        self.materials.add(StandardMaterial {
                            base_color: Color::srgb(livery[0], livery[1], livery[2]),
                            perceptual_roughness: 0.95,
                            ..default()
                        })
                    })
                    .clone();
                commands
                    .spawn((
                        Name::new("life:figure"),
                        Figure {
                            role: role.to_string(),
                            fallback: true,
                        },
                        Transform::from_translation(at).with_scale(Vec3::splat(ws)),
                        Visibility::Inherited,
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Mesh3d(body),
                            MeshMaterial3d(material.clone()),
                            Transform::from_xyz(0.0, 0.85, 0.0),
                        ));
                        p.spawn((
                            Mesh3d(head),
                            MeshMaterial3d(material),
                            Transform::from_xyz(0.0, 1.62, 0.0),
                        ));
                    })
                    .id()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arrays_of_two_parts() -> FigureArrays {
        // Triangle 0: always shown (mask 0); triangle 1: only variant 1 (bit 1); triangle 2: weapon.
        let p = |x: f32| [x, 0.0, 0.0];
        FigureArrays {
            positions: vec![
                p(0.0),
                p(1.0),
                p(2.0),
                p(3.0),
                p(4.0),
                p(5.0),
                p(6.0),
                p(7.0),
                p(8.0),
            ],
            normals: vec![[0.0, 1.0, 0.0]; 9],
            colors: (0..9)
                .map(|i| match i / 3 {
                    0 => [0.5, 0.5, 0.5, code::SKIN as f32],
                    1 => [0.5, 0.5, 0.5, code::CLOTH as f32],
                    _ => [0.5, 0.5, 0.5, code::LIVERY as f32],
                })
                .collect(),
            uvs: vec![[0.0, 0.0]; 9],
            bones: vec![[0.0; 4]; 9],
            weights: vec![[0.0; 4]; 9],
            masks: vec![0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 64.0, 64.0, 64.0],
            indices: (0..9).collect(),
        }
    }

    fn count(mesh: &Mesh) -> usize {
        mesh.indices().map_or(0, |i| i.len() / 3)
    }

    #[test]
    fn bake_keeps_parts_of_the_variant_and_drops_weapons() {
        let a = arrays_of_two_parts();
        let v0 = bake(
            &a,
            Look {
                variant: 0,
                tint: 0,
            },
            [0.4, 0.3, 0.2],
            [0.5; 3],
        );
        let v1 = bake(
            &a,
            Look {
                variant: 1,
                tint: 0,
            },
            [0.4, 0.3, 0.2],
            [0.5; 3],
        );
        assert_eq!(
            count(&v0),
            1,
            "variant 0 hides the part of variant 1 and the weapon"
        );
        assert_eq!(count(&v1), 2);
    }

    #[test]
    fn livery_changes_colours_by_role() {
        let a = arrays_of_two_parts();
        let first = |m: &Mesh| match m.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(bevy::mesh::VertexAttributeValues::Float32x4(v)) => v[v.len() - 1],
            _ => panic!("colours"),
        };
        let red = bake(
            &a,
            Look {
                variant: 1,
                tint: 0,
            },
            [0.5, 0.1, 0.1],
            [0.5; 3],
        );
        let blue = bake(
            &a,
            Look {
                variant: 1,
                tint: 0,
            },
            [0.1, 0.1, 0.5],
            [0.5; 3],
        );
        // The last vertex is cloth (drab), the skin part comes first: compare the cloth drab.
        let drab_a = bake(
            &a,
            Look {
                variant: 1,
                tint: 0,
            },
            [0.5; 3],
            [0.2; 3],
        );
        let drab_b = bake(
            &a,
            Look {
                variant: 1,
                tint: 0,
            },
            [0.5; 3],
            [0.6; 3],
        );
        assert_eq!(first(&red), first(&blue), "cloth ignores the livery");
        assert_ne!(first(&drab_a), first(&drab_b));
    }

    #[test]
    fn look_stays_in_range() {
        for i in 0..100 {
            let l = Look::of(i as f32 / 100.0, (99 - i) as f32 / 100.0, 3);
            assert!(l.variant < 3 && l.tint < TINTS);
        }
        assert_eq!(Look::of(0.999, 0.999, 1).variant, 0);
    }

    #[test]
    fn real_villager_is_a_man_sized_mesh() {
        let root = crate::paths::find_root().expect("root");
        let mut lib = FigureLibrary::new(crate::paths::asset_root(&root));
        if !lib.available() {
            return;
        }
        let table = PropTable::load(&crate::paths::data_dir(&root));
        let mut meshes = Assets::<Mesh>::default();
        let Some(handle) = lib.mesh_for(&table, &mut meshes, "peasant", 2, 0.3, 0.6) else {
            return;
        };
        let mesh = meshes.get(&handle).unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions")
        };
        let top = p.iter().map(|v| v[1]).fold(f32::MIN, f32::max);
        let bottom = p.iter().map(|v| v[1]).fold(f32::MAX, f32::min);
        assert!(
            (1.3..2.2).contains(&(top - bottom)),
            "height {}",
            top - bottom
        );
        assert!(bottom.abs() < 0.15, "feet at the ground: {bottom}");
    }
}
