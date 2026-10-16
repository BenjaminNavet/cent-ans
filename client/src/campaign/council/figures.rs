//! Seated figures and seat furniture of the council room (ports `populate`, `_add_seat_furniture`,
//! `_painted_figure`, `_ground_shadow`, `_make_figure` of `council_room.gd`).
//!
//! Default style `painted`: the cut-out illustration `illustrations/council/council_<key>.png` on
//! an upright quad, feet on the floor, 1.3 m high, turned toward the camera (Godot's
//! `BILLBOARD_FIXED_Y`), alpha-scissored, with a soft dark decal on the floor. The sovereign sits
//! in front of the throne model; a vacant office shows an empty stool.

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::pick::{self, SEAT_OFFICES};
use super::room;
use super::{CouncilCamera, CouncilState, FiguresRoot, Style, council_open};
use crate::campaign::panels::court::CourtUi;

pub const PAINTED_HEIGHT: f32 = 1.3;
const PAINTED_BASE: Color = Color::srgb(0.82, 0.8, 0.78);
const PAINTED_HOVER: Color = Color::srgb(1.25, 1.1, 0.9);

/// One seated figure (holder entity at the seat).
#[derive(Component)]
pub struct Figure {
    pub key: String,
    /// `painted`, `glb` or `capsule` (the fallback when the player has no image nor model).
    pub kind: &'static str,
    pub image: String,
    pub fitted: bool,
    pub texture: Handle<Image>,
    pub material: Handle<StandardMaterial>,
    pub quad: Option<Entity>,
    pub shadow: Option<Entity>,
}

/// Additive glow laid over every mesh of the hovered figure (`material_overlay` of Godot).
#[derive(Component)]
pub struct HoverOverlay {
    pub key: String,
}

/// Colour of the fallback figure of a seat (`ROLE_COLORS`).
fn role_color(key: &str) -> Color {
    match key {
        "ruler" => Color::srgb(0.55, 0.12, 0.14),
        "chancellor" => Color::srgb(0.25, 0.30, 0.55),
        "constable" => Color::srgb(0.45, 0.45, 0.5),
        "treasurer" => Color::srgb(0.55, 0.45, 0.15),
        "chaplain" => Color::srgb(0.2, 0.2, 0.22),
        "chamberlain" => Color::srgb(0.3, 0.5, 0.35),
        _ => Color::srgb(0.4, 0.4, 0.4),
    }
}

/// How a seat is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Painted,
    Glb,
    Capsule,
}

/// Rendering of the figure of `key`: the wanted style first, then the player's other assets
/// (image or model, whichever exists), the capsule only when he has neither.
pub fn figure_kind(state: &CouncilState, key: &str) -> Kind {
    let has_png = figure_file(state, key, Style::Painted);
    let has_glb = figure_file(state, key, Style::Mesh);
    match (state.style, has_png, has_glb) {
        (Style::Painted, true, _) | (_, true, false) => Kind::Painted,
        (_, _, true) => Kind::Glb,
        _ => Kind::Capsule,
    }
}

fn figure_file(state: &CouncilState, key: &str, style: Style) -> bool {
    let Some(root) = &state.asset_root else {
        return false;
    };
    match style {
        Style::Painted => root
            .join(format!("illustrations/council/council_{key}.png"))
            .is_file(),
        // The wanted mesh style first, then the other mesh folders.
        _ => {
            let wanted = if state.style == Style::Painted {
                Style::Mesh
            } else {
                state.style
            };
            room::figure_dirs(wanted)
                .iter()
                .any(|dir| root.join(format!("{dir}council_{key}.glb")).is_file())
        }
    }
}

/// Quad of a painted figure (turned toward the camera each frame).
#[derive(Component)]
pub struct PaintedQuad;

fn shadow_image() -> Image {
    let size = 64usize;
    let mut data = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 + 0.5 - 32.0, y as f32 + 0.5 - 32.0);
            let r = ((dx * dx + dy * dy).sqrt() / 32.0).clamp(0.0, 1.0);
            data.extend_from_slice(&[0, 0, 0, ((1.0 - r) * 0.6 * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Rebuilds the figures when the occupancy of the seats changes.
#[allow(clippy::too_many_arguments)]
pub fn populate(
    mut commands: Commands,
    ui: Res<CourtUi>,
    mut state: ResMut<CouncilState>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    roots: Query<Entity, With<FiguresRoot>>,
    mut last_root: Local<Option<Entity>>,
    mut decal: Local<Option<Handle<Image>>>,
) {
    if !council_open(&ui) {
        return;
    }
    let Ok(root) = roots.single() else {
        return;
    };
    let signature: Vec<(String, bool)> = SEAT_OFFICES
        .iter()
        .map(|key| {
            let occupied = ui
                .snapshot
                .offices
                .iter()
                .any(|o| o.id == *key && !o.holder_id.is_empty());
            ((*key).to_owned(), occupied)
        })
        .collect();
    if *last_root == Some(root) && state.signature == signature {
        return;
    }
    *last_root = Some(root);
    state.signature = signature.clone();
    state.populated += 1;
    state.hovered = None;
    commands.entity(root).despawn_children();
    let decal = decal
        .get_or_insert_with(|| images.add(shadow_image()))
        .clone();
    let style = state.style;
    for key in pick::seat_keys() {
        let position = pick::seat_position(key);
        let occupied = key == "ruler" || signature.iter().any(|(k, o)| k == key && *o);
        let yaw = Quat::from_rotation_y(pick::facing(position, Vec3::new(position.x, 0.0, 4.0)));
        let kind = figure_kind(&state, key);
        // A vacant seat, or a stand-in capsule, sits on a stool.
        let figure_available = occupied && kind != Kind::Capsule;
        // Seat furniture: the throne behind the sovereign, a stool under a vacant seat.
        if key == "ruler" {
            room::spawn_model(
                &mut commands,
                &assets,
                state.asset_root.as_deref(),
                &room::furniture_dirs(style),
                "council_throne.glb",
                root,
                Transform::from_translation(position + Vec3::new(0.0, 0.0, -0.3)),
                "throne",
            );
        } else if !figure_available {
            let spawned = room::spawn_model(
                &mut commands,
                &assets,
                state.asset_root.as_deref(),
                &room::furniture_dirs(style),
                "council_stool.glb",
                root,
                Transform::from_translation(position).with_rotation(yaw),
                &format!("stool:{key}"),
            );
            if spawned.is_none() {
                let wood = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.25, 0.17, 0.12),
                    perceptual_roughness: 0.9,
                    ..default()
                });
                commands.spawn((
                    Name::new(format!("Seat_{key}")),
                    Mesh3d(meshes.add(Cuboid::new(0.45, 0.45, 0.45))),
                    MeshMaterial3d(wood),
                    Transform::from_translation(position + Vec3::Y * 0.225),
                    ChildOf(root),
                ));
            }
        }
        if !occupied {
            continue;
        }
        match kind {
            Kind::Painted => {
                let image = format!("illustrations/council/council_{key}.png");
                let texture: Handle<Image> = assets.load(image.clone());
                let material = materials.add(StandardMaterial {
                    base_color: PAINTED_BASE,
                    base_color_texture: Some(texture.clone()),
                    alpha_mode: AlphaMode::Mask(0.5),
                    perceptual_roughness: 1.0,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                });
                let mut holder = commands.spawn((
                    Name::new(format!("Figure_{key}")),
                    Transform::from_translation(position),
                    Visibility::default(),
                    ChildOf(root),
                ));
                let holder_id = holder.id();
                let quad = holder
                    .commands()
                    .spawn((
                        Name::new("Painted"),
                        PaintedQuad,
                        Mesh3d(meshes.add(Rectangle::new(PAINTED_HEIGHT * 0.75, PAINTED_HEIGHT))),
                        MeshMaterial3d(material.clone()),
                        NotShadowCaster,
                        Visibility::Hidden,
                        Transform::from_xyz(0.0, PAINTED_HEIGHT * 0.5, 0.0),
                        ChildOf(holder_id),
                    ))
                    .id();
                let shadow = holder
                    .commands()
                    .spawn((
                        Name::new("GroundShadow"),
                        Mesh3d(meshes.add(Rectangle::new(PAINTED_HEIGHT * 0.75 * 1.1, 0.7))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color_texture: Some(decal.clone()),
                            unlit: true,
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        })),
                        NotShadowCaster,
                        Visibility::Hidden,
                        Transform::from_xyz(0.0, 0.012, 0.0)
                            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                        ChildOf(holder_id),
                    ))
                    .id();
                commands.entity(holder_id).insert(Figure {
                    key: key.to_owned(),
                    kind: "painted",
                    image,
                    fitted: false,
                    texture,
                    material,
                    quad: Some(quad),
                    shadow: Some(shadow),
                });
            }
            Kind::Glb => {
                let holder = commands
                    .spawn((
                        Name::new(format!("Figure_{key}")),
                        Transform::from_translation(position).with_rotation(yaw),
                        Visibility::default(),
                        ChildOf(root),
                    ))
                    .id();
                let model = room::spawn_model(
                    &mut commands,
                    &assets,
                    state.asset_root.as_deref(),
                    &room::figure_dirs(if style == Style::Painted {
                        Style::Mesh
                    } else {
                        style
                    }),
                    &format!("council_{key}.glb"),
                    holder,
                    Transform::default(),
                    &format!("figure:{key}"),
                );
                if model.is_none() {
                    commands.entity(holder).despawn();
                } else {
                    commands.entity(holder).insert(Figure {
                        key: key.to_owned(),
                        kind: "glb",
                        image: String::new(),
                        fitted: true,
                        texture: Handle::default(),
                        material: Handle::default(),
                        quad: None,
                        shadow: None,
                    });
                }
            }
            Kind::Capsule => {
                let holder = commands
                    .spawn((
                        Name::new(format!("Figure_{key}")),
                        Transform::from_translation(position).with_rotation(yaw),
                        Visibility::default(),
                        Figure {
                            key: key.to_owned(),
                            kind: "capsule",
                            image: String::new(),
                            fitted: true,
                            texture: Handle::default(),
                            material: Handle::default(),
                            quad: None,
                            shadow: None,
                        },
                        ChildOf(root),
                    ))
                    .id();
                let color = if key == "ruler" {
                    // `faction_color.darkened(0.2)`
                    let c = state.faction_color.to_linear();
                    Color::linear_rgb(c.red * 0.8, c.green * 0.8, c.blue * 0.8)
                } else {
                    role_color(key)
                };
                spawn_capsule(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    holder,
                    color,
                    key == "ruler",
                );
            }
        }
    }
}

/// Stand-in figure when the player has neither image nor model (`_placeholder`): a capsule body
/// in the colour of the role, a head, a gold ring for the sovereign.
fn spawn_capsule(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    holder: Entity,
    color: Color,
    crowned: bool,
) {
    let mut part =
        |commands: &mut Commands, name: &str, mesh: Mesh, material: StandardMaterial, at: Vec3| {
            commands.spawn((
                Name::new(name.to_owned()),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(material)),
                Transform::from_translation(at),
                ChildOf(holder),
            ));
        };
    part(
        commands,
        "Body",
        Capsule3d::new(0.26, 0.9 - 0.52).into(),
        StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.8,
            ..default()
        },
        Vec3::new(0.0, 0.95, 0.0),
    );
    part(
        commands,
        "Head",
        Sphere::new(0.15).into(),
        StandardMaterial {
            base_color: Color::srgb(0.72, 0.58, 0.46),
            ..default()
        },
        Vec3::new(0.0, 1.62, 0.0),
    );
    if crowned {
        part(
            commands,
            "Crown",
            Cylinder::new(0.14, 0.08).into(),
            StandardMaterial {
                base_color: Color::srgb(0.85, 0.68, 0.2),
                metallic: 0.8,
                perceptual_roughness: 0.35,
                ..default()
            },
            Vec3::new(0.0, 1.8, 0.0),
        );
    }
}

/// Sizes the quad and decal of a painted figure once its image is known (aspect of the cut-out).
pub fn fit_painted(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut figures: Query<&mut Figure>,
) {
    for mut figure in &mut figures {
        if figure.fitted {
            continue;
        }
        let (Some(quad), Some(shadow)) = (figure.quad, figure.shadow) else {
            continue;
        };
        let Some(image) = images.get(&figure.texture) else {
            continue;
        };
        let aspect = image.width() as f32 / image.height().max(1) as f32;
        commands.entity(quad).insert((
            Mesh3d(meshes.add(Rectangle::new(PAINTED_HEIGHT * aspect, PAINTED_HEIGHT))),
            Visibility::Inherited,
        ));
        commands.entity(shadow).insert((
            Mesh3d(meshes.add(Rectangle::new(PAINTED_HEIGHT * aspect * 1.1, 0.7))),
            Visibility::Inherited,
        ));
        figure.fitted = true;
    }
}

type QuadItem = (&'static ChildOf, &'static mut Transform);

/// Lights up the hovered figure (the painted ones are brightened rather than overlaid) and turns
/// the quads toward the camera around the vertical axis.
pub fn apply_hover(
    state: Res<CouncilState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    figures: Query<(&Figure, &GlobalTransform)>,
    camera: Query<&Transform, With<CouncilCamera>>,
    mut quads: Query<QuadItem, (With<PaintedQuad>, Without<CouncilCamera>)>,
) {
    for (figure, _) in figures.iter().filter(|(f, _)| f.kind == "painted") {
        let wanted = if state.hovered.as_deref() == Some(figure.key.as_str()) {
            PAINTED_HOVER
        } else {
            PAINTED_BASE
        };
        if materials
            .get(&figure.material)
            .is_some_and(|m| m.base_color != wanted)
            && let Some(mut material) = materials.get_mut(&figure.material)
        {
            material.base_color = wanted;
        }
    }
    let Ok(camera) = camera.single() else {
        return;
    };
    for (parent, mut transform) in &mut quads {
        let Ok((_, holder)) = figures.get(parent.parent()) else {
            continue;
        };
        let to_camera = camera.translation - holder.translation();
        transform.rotation = Quat::from_rotation_y(to_camera.x.atan2(to_camera.z));
    }
}

/// Glow of the hovered figure when it is a model or a stand-in (`_highlight_material`): every mesh
/// of the figure gets a child copy with an additive warm material, removed when the cursor leaves.
#[allow(clippy::too_many_arguments)]
pub fn apply_overlay(
    mut commands: Commands,
    state: Res<CouncilState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    figures: Query<(Entity, &Figure)>,
    children: Query<&Children>,
    meshes: Query<&Mesh3d, Without<HoverOverlay>>,
    overlays: Query<(Entity, &HoverOverlay, &ChildOf)>,
    mut material: Local<Option<Handle<StandardMaterial>>>,
) {
    let hovered = state.hovered.as_deref();
    for (entity, overlay, _) in &overlays {
        if Some(overlay.key.as_str()) != hovered {
            commands.entity(entity).try_despawn();
        }
    }
    let Some((holder, figure)) = figures
        .iter()
        .find(|(_, f)| f.kind != "painted" && Some(f.key.as_str()) == hovered)
    else {
        return;
    };
    let material = material
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::srgba(0.9, 0.62, 0.25, 0.28),
                emissive: LinearRgba::rgb(1.0, 0.7, 0.3),
                unlit: true,
                alpha_mode: AlphaMode::Add,
                depth_bias: 1.0,
                ..default()
            })
        })
        .clone();
    let covered: Vec<Entity> = overlays
        .iter()
        .map(|(_, _, parent)| parent.parent())
        .collect();
    let targets = std::iter::once(holder).chain(children.iter_descendants(holder));
    for target in targets {
        if covered.contains(&target) {
            continue;
        }
        let Ok(mesh) = meshes.get(target) else {
            continue;
        };
        commands.spawn((
            Name::new("HoverOverlay"),
            HoverOverlay {
                key: figure.key.clone(),
            },
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(material.clone()),
            NotShadowCaster,
            Transform::default(),
            ChildOf(target),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(files: &[&str], style: Style) -> (CouncilState, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "council_figs_{}_{}",
            std::process::id(),
            files.join("_").replace('/', "-")
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for file in files {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"x").unwrap();
        }
        let state = CouncilState {
            asset_root: Some(dir.clone()),
            style,
            ..default()
        };
        (state, dir)
    }

    #[test]
    fn capsule_only_when_player_has_nothing() {
        let (state, dir) = state_with(&[], Style::Painted);
        assert_eq!(figure_kind(&state, "treasurer"), Kind::Capsule);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn painted_style_uses_png_then_glb_then_capsule() {
        let png = "illustrations/council/council_chancellor.png";
        let (state, dir) = state_with(&[png], Style::Painted);
        assert_eq!(figure_kind(&state, "chancellor"), Kind::Painted);
        let _ = std::fs::remove_dir_all(dir);
        let glb_dir = room::figure_dirs(Style::Mesh)[0];
        let glb = format!("{glb_dir}council_chancellor.glb");
        let (state, dir) = state_with(&[glb.as_str()], Style::Painted);
        assert_eq!(figure_kind(&state, "chancellor"), Kind::Glb);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn mesh_style_falls_back_to_png() {
        let png = "illustrations/council/council_chaplain.png";
        let (state, dir) = state_with(&[png], Style::Mesh);
        assert_eq!(figure_kind(&state, "chaplain"), Kind::Painted);
        let _ = std::fs::remove_dir_all(dir);
    }
}
