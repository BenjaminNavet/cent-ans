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
    pub kind: &'static str,
    pub image: String,
    pub fitted: bool,
    pub texture: Handle<Image>,
    pub material: Handle<StandardMaterial>,
    pub quad: Entity,
    pub shadow: Entity,
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
        let figure_available = occupied && figure_exists(&state, key);
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
        match style {
            Style::Painted => {
                let image = format!("illustrations/council/council_{key}.png");
                if !figure_available {
                    continue;
                }
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
                    quad,
                    shadow,
                });
            }
            Style::Mesh | Style::MeshMultiview => {
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
                    &room::figure_dirs(style),
                    &format!("council_{key}.glb"),
                    holder,
                    Transform::default(),
                    &format!("figure:{key}"),
                );
                if model.is_none() {
                    commands.entity(holder).despawn();
                }
            }
        }
    }
}

/// Whether the figure of `key` exists in the current style.
fn figure_exists(state: &CouncilState, key: &str) -> bool {
    let Some(root) = &state.asset_root else {
        return false;
    };
    match state.style {
        Style::Painted => root
            .join(format!("illustrations/council/council_{key}.png"))
            .is_file(),
        style => room::figure_dirs(style)
            .iter()
            .any(|dir| root.join(format!("{dir}council_{key}.glb")).is_file()),
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
        let Some(image) = images.get(&figure.texture) else {
            continue;
        };
        let aspect = image.width() as f32 / image.height().max(1) as f32;
        commands.entity(figure.quad).insert((
            Mesh3d(meshes.add(Rectangle::new(PAINTED_HEIGHT * aspect, PAINTED_HEIGHT))),
            Visibility::Inherited,
        ));
        commands.entity(figure.shadow).insert((
            Mesh3d(meshes.add(Rectangle::new(PAINTED_HEIGHT * aspect * 1.1, 0.7))),
            Visibility::Inherited,
        ));
        figure.fitted = true;
    }
}

/// Lights up the hovered figure (the painted ones are brightened rather than overlaid) and turns
/// the quads toward the camera around the vertical axis.
pub fn apply_hover(
    state: Res<CouncilState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    figures: Query<(&Figure, &GlobalTransform)>,
    camera: Query<&Transform, With<CouncilCamera>>,
    mut quads: Query<(&ChildOf, &mut Transform), (With<PaintedQuad>, Without<CouncilCamera>)>,
) {
    for (figure, _) in &figures {
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
