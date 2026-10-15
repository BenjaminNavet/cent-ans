//! Static decor of the council room (ports `_build_environment`, `_build_shell`, `_build_table`,
//! `_build_candles`, `_build_window`, `_build_tapestries` of `game/scripts/ui/council_room.gd`):
//! a dark intimate stone chamber, a long oak table, iron candelabra, a cold shaft of daylight from
//! a tall narrow window, tapestries in the faction colours. Coordinates are Godot's (right-handed,
//! Y up, the camera looks toward -Z), so every number is the original one.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::light::NotShadowCaster;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;

use super::pick::ROW_CENTER_X;
use super::{CandleFlame, CouncilModel, CouncilProps, Style};

pub const STONE_WALL: &str = "textures/buildings/stone_wall_diff.jpg";
pub const FLOOR_TEX: &str = "textures/battle/cobblestone_floor_01_diff.jpg";
pub const WOOD_TEX: &str = "textures/buildings/weathered_brown_planks_diff.jpg";
pub const HANDMADE_DIR: &str = "models/council/handmade/";
pub const MODEL_DIR: &str = "models/council/";
pub const MULTIVIEW_DIR: &str = "models/council/multiview/";
pub const TABLE_LENGTH: f32 = 4.6;
pub const TABLE_WIDTH: f32 = 1.1;
pub const TABLE_HEIGHT: f32 = 0.85;

/// A box mesh whose UVs are box-mapped in metres times `uv_scale` (Godot's triplanar `uv1_scale`).
fn box_mesh(size: Vec3, uv_scale: f32) -> Mesh {
    let mut mesh = Mesh::from(Cuboid::new(size.x, size.y, size.z));
    let positions: Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(values)) => values.clone(),
        _ => Vec::new(),
    };
    let normals: Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(values)) => values.clone(),
        _ => Vec::new(),
    };
    let uvs: Vec<[f32; 2]> = positions
        .iter()
        .zip(&normals)
        .map(|(p, n)| {
            let (u, v) = if n[0].abs() > 0.5 {
                (p[2], p[1])
            } else if n[1].abs() > 0.5 {
                (p[0], p[2])
            } else {
                (p[0], p[1])
            };
            [u * uv_scale, v * uv_scale]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

fn repeat_sampler(settings: &mut bevy::image::ImageLoaderSettings) {
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
}

fn textured(assets: &AssetServer, path: &str, tint: Color, roughness: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: tint,
        base_color_texture: Some(
            assets
                .load_builder()
                .with_settings(repeat_sampler)
                .load(path.to_owned()),
        ),
        perceptual_roughness: roughness,
        ..default()
    }
}

fn flat(color: Color, roughness: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        perceptual_roughness: roughness,
        ..default()
    }
}

/// Everything the builder needs.
pub struct Builder<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
    pub assets: &'a AssetServer,
    pub root: Entity,
    pub asset_root: Option<std::path::PathBuf>,
    pub style: Style,
    pub props: &'a mut CouncilProps,
}

impl Builder<'_, '_, '_> {
    fn boxed(
        &mut self,
        name: &str,
        size: Vec3,
        position: Vec3,
        material: Handle<StandardMaterial>,
        uv_scale: f32,
    ) -> Entity {
        let mesh = self.meshes.add(box_mesh(size, uv_scale));
        let entity = self
            .commands
            .spawn((
                Name::new(name.to_owned()),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(position),
                ChildOf(self.root),
            ))
            .id();
        self.props.meshes += 1;
        entity
    }

    /// Spawns a glb furniture model under `parent`, recorded in the dump list.
    pub fn model(&mut self, file: &str, parent: Entity, transform: Transform, role: &str) -> Option<Entity> {
        spawn_model(
            self.commands,
            self.assets,
            self.asset_root.as_deref(),
            &furniture_dirs(self.style),
            file,
            parent,
            transform,
            role,
        )
    }

    pub fn build(&mut self, faction_color: Color, faction: &str) {
        self.shell();
        self.table();
        self.candles();
        self.window();
        self.tapestries(faction_color, faction);
    }

    fn shell(&mut self) {
        let stone = self
            .materials
            .add(textured(self.assets, STONE_WALL, Color::srgb(0.55, 0.52, 0.5), 0.9));
        let floor = self
            .materials
            .add(textured(self.assets, FLOOR_TEX, Color::srgb(0.5, 0.47, 0.45), 0.9));
        self.boxed("Floor", Vec3::new(8.0, 0.2, 14.0), Vec3::new(0.0, -0.1, -0.5), floor, 0.3);
        for (name, size, position) in [
            ("WallLeft", Vec3::new(0.3, 5.0, 14.0), Vec3::new(-4.0, 2.4, -0.5)),
            ("WallRight", Vec3::new(0.3, 5.0, 14.0), Vec3::new(4.0, 2.4, -0.5)),
            ("WallBack", Vec3::new(8.3, 5.0, 0.3), Vec3::new(0.0, 2.4, -4.6)),
            ("WallFront", Vec3::new(8.3, 5.0, 0.3), Vec3::new(0.0, 2.4, 6.7)),
            ("Ceiling", Vec3::new(8.0, 0.3, 14.0), Vec3::new(0.0, 5.0, -0.5)),
        ] {
            self.boxed(name, size, position, stone.clone(), 0.35);
        }
        // Engaged columns and arch ribs (a vault suggested by half circles of beams).
        let column_mesh = self.meshes.add(ConicalFrustum {
            radius_top: 0.28,
            radius_bottom: 0.32,
            height: 3.4,
        });
        for z in [-3.5_f32, -0.5, 2.5] {
            for side in [-1.0_f32, 1.0] {
                self.commands.spawn((
                    Name::new("Column"),
                    Mesh3d(column_mesh.clone()),
                    MeshMaterial3d(stone.clone()),
                    Transform::from_xyz(side * 3.6, 1.7, z),
                    ChildOf(self.root),
                ));
                self.props.meshes += 1;
            }
            self.arch(z, stone.clone());
        }
        // Hearth at the back, with a discreet ember.
        let hearth = self
            .materials
            .add(textured(self.assets, STONE_WALL, Color::srgb(0.35, 0.33, 0.32), 0.9));
        self.boxed("Hearth", Vec3::new(2.2, 2.0, 0.5), Vec3::new(2.4, 1.0, -4.3), hearth, 0.5);
        self.commands.spawn((
            Name::new("Ember"),
            PointLight {
                color: Color::srgb(1.0, 0.45, 0.15),
                intensity: super::EMBER_LUMENS,
                range: 3.5,
                ..default()
            },
            Transform::from_xyz(2.4, 0.5, -3.8),
            ChildOf(self.root),
        ));
        self.props.lights += 1;
    }

    fn arch(&mut self, z: f32, material: Handle<StandardMaterial>) {
        let segments = 9;
        let radius = 3.6;
        for index in 0..segments {
            let angle = PI * (index as f32 + 0.5) / segments as f32;
            let piece = self.boxed(
                "ArchRib",
                Vec3::new(0.45, 0.45, 0.5),
                Vec3::new(angle.cos() * radius, 3.4 + angle.sin() * 1.35, z),
                material.clone(),
                0.35,
            );
            self.commands
                .entity(piece)
                .insert(Transform::from_xyz(angle.cos() * radius, 3.4 + angle.sin() * 1.35, z).with_rotation(
                    Quat::from_rotation_z(angle + FRAC_PI_2),
                ));
        }
    }

    fn table(&mut self) {
        let wood = self
            .materials
            .add(textured(self.assets, WOOD_TEX, Color::srgb(0.32, 0.22, 0.16), 0.55));
        let table = self
            .commands
            .spawn((
                Name::new("Table"),
                Transform::from_xyz(ROW_CENTER_X, 0.0, 0.0).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
                Visibility::default(),
                ChildOf(self.root),
            ))
            .id();
        // Models are 3.2 m long on Z: stretched to the table length, laid across X.
        let model = self.model(
            "council_table.glb",
            table,
            Transform::from_scale(Vec3::new(1.0, 1.0, TABLE_LENGTH / 3.2)),
            "table",
        );
        if model.is_none() {
            self.boxed(
                "TableTop",
                Vec3::new(TABLE_LENGTH, 0.08, TABLE_WIDTH),
                Vec3::new(ROW_CENTER_X, TABLE_HEIGHT - 0.04, 0.0),
                wood.clone(),
                0.6,
            );
            for x in [-TABLE_LENGTH * 0.45, TABLE_LENGTH * 0.45] {
                for z in [-0.48, 0.48] {
                    self.boxed(
                        "TableLeg",
                        Vec3::new(0.12, TABLE_HEIGHT, 0.12),
                        Vec3::new(ROW_CENTER_X + x, TABLE_HEIGHT * 0.5, z),
                        wood.clone(),
                        0.6,
                    );
                }
            }
        }
        // The kingdom map and scattered scrolls.
        let parchment = self.materials.add(flat(Color::srgb(0.72, 0.62, 0.45), 0.95));
        let ink = self.materials.add(flat(Color::srgb(0.25, 0.12, 0.08), 0.9));
        let y_top = TABLE_HEIGHT + 0.005;
        self.boxed("Map", Vec3::new(1.3, 0.01, 0.75), Vec3::new(ROW_CENTER_X + 0.4, y_top, 0.05), parchment.clone(), 1.0);
        self.boxed("MapLine", Vec3::new(0.5, 0.012, 0.02), Vec3::new(0.1, y_top + 0.006, 0.0), ink.clone(), 1.0);
        self.boxed("MapLine", Vec3::new(0.02, 0.012, 0.45), Vec3::new(-0.05, y_top + 0.006, 0.05), ink, 1.0);
        for scrap in [
            Vec3::new(-2.2, 0.0, 0.2),
            Vec3::new(1.6, 0.0, 0.25),
            Vec3::new(-1.3, 0.0, 0.3),
            Vec3::new(-2.6, 0.0, -0.15),
        ] {
            let sheet = self.boxed("Scroll", Vec3::new(0.22, 0.008, 0.3), Vec3::new(scrap.x, y_top, scrap.z), parchment.clone(), 1.0);
            self.commands.entity(sheet).insert(
                Transform::from_xyz(scrap.x, y_top, scrap.z)
                    .with_rotation(Quat::from_rotation_y(scrap.x * 1.7 + scrap.z)),
            );
        }
    }

    fn candles(&mut self) {
        let iron = self.materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.08, 0.09),
            metallic: 0.7,
            perceptual_roughness: 0.5,
            ..default()
        });
        let flame = self.materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.7, 0.3),
            emissive: LinearRgba::rgb(1.0, 0.6, 0.2) * 3.0,
            unlit: true,
            ..default()
        });
        let sphere = self.meshes.add(Sphere::new(0.025));
        for (index, spot) in [Vec2::new(-2.1, 0.3), Vec2::new(-0.42, 0.3), Vec2::new(1.27, 0.3)]
            .into_iter()
            .enumerate()
        {
            let stand = self.boxed("Candelabrum", Vec3::new(0.04, 0.24, 0.04), Vec3::new(spot.x, 0.97, spot.y), iron.clone(), 1.0);
            let arm = self.meshes.add(box_mesh(Vec3::new(0.3, 0.025, 0.04), 1.0));
            self.commands.spawn((
                Name::new("CandelabrumArm"),
                Mesh3d(arm),
                MeshMaterial3d(iron.clone()),
                Transform::from_xyz(0.0, 0.12, 0.0),
                ChildOf(stand),
            ));
            for dx in [-0.18_f32, 0.0, 0.18] {
                self.commands.spawn((
                    Name::new("Flame"),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(flame.clone()),
                    NotShadowCaster,
                    Transform::from_xyz(dx * 0.75, 0.17, 0.0),
                    ChildOf(stand),
                ));
            }
            self.commands.spawn((
                Name::new(format!("Candle{index}")),
                PointLight {
                    color: Color::srgb(1.0, 0.62, 0.32), // ~2400 K
                    intensity: super::CANDLE_LUMENS,
                    range: 5.0,
                    // Soft shadows on two flames only.
                    shadow_maps_enabled: index != 1,
                    ..default()
                },
                CandleFlame { index },
                Transform::from_xyz(spot.x, 1.3, spot.y + 0.2),
                ChildOf(self.root),
            ));
            self.props.lights += 1;
            self.props.candles += 1;
        }
    }

    fn window(&mut self) {
        let glass = self.materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.42, 0.55),
            emissive: LinearRgba::rgb(0.55, 0.7, 1.0) * 0.5,
            unlit: true,
            ..default()
        });
        self.boxed("Window", Vec3::new(0.05, 2.6, 0.5), Vec3::new(-3.83, 2.6, -2.2), glass, 1.0);
        let position = Vec3::new(-3.6, 3.4, -2.2);
        self.commands.spawn((
            Name::new("WindowLight"),
            SpotLight {
                color: Color::srgb(0.6, 0.72, 1.0), // cold daylight
                intensity: super::WINDOW_LUMENS,
                range: 9.0,
                outer_angle: 24.0_f32.to_radians(),
                inner_angle: 14.0_f32.to_radians(),
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_translation(position).looking_at(Vec3::new(0.4, 0.9, -1.0), Vec3::Y),
            ChildOf(self.root),
        ));
        self.props.lights += 1;
    }

    fn tapestries(&mut self, faction_color: Color, faction: &str) {
        let darkened = {
            let c = faction_color.to_srgba();
            Color::srgb(c.red * 0.55, c.green * 0.55, c.blue * 0.55)
        };
        let cloth = self.materials.add(StandardMaterial {
            base_color: darkened,
            perceptual_roughness: 1.0,
            cull_mode: None,
            double_sided: true,
            ..default()
        });
        let emblem_texture: Option<Handle<Image>> = self.asset_root.as_ref().and_then(|root| {
            let path = format!("heraldry/{faction}.png");
            root.join(&path).is_file().then(|| self.assets.load(path))
        });
        let emblem_material = emblem_texture.map(|texture| {
            self.materials.add(StandardMaterial {
                base_color: Color::srgb(0.75, 0.75, 0.75),
                base_color_texture: Some(texture),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 1.0,
                cull_mode: None,
                double_sided: true,
                ..default()
            })
        });
        self.props.emblem = emblem_material.is_some();
        let emblem_mesh = self.meshes.add(Rectangle::new(0.8, 0.8));
        for (position, yaw, size) in [
            (Vec3::new(0.0, 2.6, -4.43), 0.0, Vec2::new(1.3, 2.4)),
            (Vec3::new(3.82, 2.5, -2.5), -FRAC_PI_2, Vec2::new(1.1, 2.0)),
            (Vec3::new(3.82, 2.5, 1.5), -FRAC_PI_2, Vec2::new(1.1, 2.0)),
            (Vec3::new(-3.82, 2.5, 3.0), FRAC_PI_2, Vec2::new(1.1, 2.0)),
        ] {
            let mesh = self.meshes.add(Rectangle::new(size.x, size.y));
            let quad = self
                .commands
                .spawn((
                    Name::new("Tapestry"),
                    Mesh3d(mesh),
                    MeshMaterial3d(cloth.clone()),
                    Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
                    ChildOf(self.root),
                ))
                .id();
            self.props.tapestries += 1;
            if let Some(material) = &emblem_material {
                self.commands.spawn((
                    Name::new("Emblem"),
                    Mesh3d(emblem_mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    NotShadowCaster,
                    Transform::from_xyz(0.0, 0.1, 0.01),
                    ChildOf(quad),
                ));
            }
        }
    }
}

/// Directories searched for the furniture of a style, by priority (`_furniture_dirs`).
pub fn furniture_dirs(style: Style) -> Vec<&'static str> {
    match style {
        Style::Painted => vec![HANDMADE_DIR, MODEL_DIR],
        Style::MeshMultiview => vec![MULTIVIEW_DIR, MODEL_DIR],
        Style::Mesh => vec![MODEL_DIR],
    }
}

/// Directories searched for the figure glb of a style (`_figure_glb_path`).
pub fn figure_dirs(style: Style) -> Vec<&'static str> {
    match style {
        Style::MeshMultiview => vec![MULTIVIEW_DIR, MODEL_DIR],
        _ => vec![MODEL_DIR],
    }
}

/// Spawns the first existing glb `dir/file` among `dirs` under `parent`; `None` when none exists
/// (loading a missing glb would only log errors).
#[allow(clippy::too_many_arguments)]
pub fn spawn_model(
    commands: &mut Commands,
    assets: &AssetServer,
    asset_root: Option<&std::path::Path>,
    dirs: &[&str],
    file: &str,
    parent: Entity,
    transform: Transform,
    role: &str,
) -> Option<Entity> {
    let root = asset_root?;
    let path = dirs
        .iter()
        .map(|dir| format!("{dir}{file}"))
        .find(|path| root.join(path).is_file())?;
    let handle = assets.load(bevy::gltf::GltfAssetLabel::Scene(0).from_asset(path.clone()));
    Some(
        commands
            .spawn((
                Name::new(format!("CouncilModel:{role}")),
                bevy::world_serialization::WorldAssetRoot(handle.clone()),
                transform,
                Visibility::default(),
                CouncilModel {
                    role: role.to_owned(),
                    path,
                    scene: handle,
                },
                ChildOf(parent),
            ))
            .id(),
    )
}
