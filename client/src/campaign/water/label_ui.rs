//! On-screen river names (render side of `river_labels.gd`): a pool of UI text nodes, placed on
//! the draped ground, rotated along the flow, in the UI theme's italic title font (accents).

use bevy::prelude::*;

use super::WaterStats;
use crate::app::AppState;
use crate::campaign::terrain::surface::GroundHeight;
use crate::ui::kit::UiTheme;

const POOL: usize = 40;
const FONT_SIZE: f32 = 15.0;
/// Fraction of `visible_to` beyond which a label fades out.
const FADE_START: f32 = 0.7;

#[derive(Component)]
struct RiverLabelSlot(usize);

/// Names of the labels on screen (dump).
#[derive(Resource, Default)]
pub struct RiverLabelStats {
    pub shown: Vec<String>,
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<RiverLabelStats>()
        .add_systems(OnEnter(AppState::Campaign), spawn_pool)
        .add_systems(
            Update,
            update_labels
                .run_if(in_state(AppState::Campaign))
                .run_if(resource_exists::<WaterStats>),
        );
}

fn spawn_pool(mut commands: Commands, theme: Option<Res<UiTheme>>) {
    let font = theme
        .map(|t| t.font_title_italic.clone())
        .unwrap_or_default();
    for slot in 0..POOL {
        commands.spawn((
            Name::new(format!("RiverLabel{slot}")),
            RiverLabelSlot(slot),
            DespawnOnExit(AppState::Campaign),
            Text::new(""),
            TextFont {
                font: font.clone(),
                font_size: FONT_SIZE.into(),
                ..default()
            },
            TextColor(Color::NONE),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            UiTransform::default(),
            Visibility::Hidden,
            GlobalZIndex(-1),
            Pickable::IGNORE,
        ));
    }
}

/// Screen angle (radians, clockwise, readable left to right) of the map direction `dir` at `px`.
fn screen_angle(a: Vec2, b: Vec2) -> f32 {
    let d = b - a;
    let mut angle = d.y.atan2(d.x);
    if angle > std::f32::consts::FRAC_PI_2 {
        angle -= std::f32::consts::PI;
    } else if angle < -std::f32::consts::FRAC_PI_2 {
        angle += std::f32::consts::PI;
    }
    angle
}

#[allow(clippy::too_many_arguments)]
fn update_labels(
    stats: Res<WaterStats>,
    ground: GroundHeight,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut shown: ResMut<RiverLabelStats>,
    mut slots: Query<(
        &RiverLabelSlot,
        &mut Text,
        &mut Node,
        &mut UiTransform,
        &mut TextColor,
        &mut Visibility,
    )>,
) {
    shown.shown.clear();
    let Some((camera, transform)) = cameras.iter().find(|(c, _)| c.is_active) else {
        return;
    };
    let eye = transform.translation();
    let viewport = camera.logical_viewport_size().unwrap_or(Vec2::ONE);
    let mut candidates: Vec<(i64, f32, Vec2, f32, &str)> = Vec::new();
    for label in &stats.labels {
        let lift = |p: Vec2| Vec3::new(p.x, ground.at(p.x, p.y).unwrap_or(0.0) + 0.02, p.y);
        let world = lift(label.px);
        let distance = eye.distance(world);
        if distance > label.visible_to {
            continue;
        }
        let Ok(a) = camera.world_to_viewport(transform, world) else {
            continue;
        };
        if a.x < 0.0 || a.y < 0.0 || a.x > viewport.x || a.y > viewport.y {
            continue;
        }
        let Ok(b) = camera.world_to_viewport(transform, lift(label.px + label.dir * 4.0)) else {
            continue;
        };
        let fade =
            1.0 - ((distance / label.visible_to - FADE_START) / (1.0 - FADE_START)).clamp(0.0, 1.0);
        candidates.push((
            label.importance,
            fade,
            a,
            screen_angle(a, b),
            label.text.as_str(),
        ));
    }
    candidates.sort_by_key(|c| std::cmp::Reverse(c.0));
    candidates.truncate(POOL);
    for (slot, mut text, mut node, mut ui, mut color, mut visibility) in &mut slots {
        match candidates.get(slot.0) {
            Some((_, fade, at, angle, name)) => {
                if text.0 != *name {
                    text.0 = (*name).to_owned();
                }
                let width = name.chars().count() as f32 * FONT_SIZE * 0.5;
                node.left = Val::Px(at.x - width * 0.5);
                node.top = Val::Px(at.y - FONT_SIZE * 0.6);
                ui.rotation = Rot2::radians(*angle);
                color.0 = Color::srgba(0.86, 0.93, 0.98, 0.9 * fade);
                *visibility = Visibility::Inherited;
                shown.shown.push((*name).to_owned());
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
