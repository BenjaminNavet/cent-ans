//! Enemy armies lost from sight, on the 3D map (lot armies6). Port of the `ArmyMarkers` part of
//! `army_memory.gd` (ghost labels) and of `campaign_map.gd` (`ghost_at`, `ghost_bubble_text`).
//! The memory itself (`ArmyMemory`, `Ghosts`, the duration `fog_memory.turns` of
//! `data/map/stance_cues.json`) lives in `parchment/extras.rs` and is shared with the parchment
//! view: it keeps the last known situation of each enemy army seen, never reads the simulation
//! for what the player has not seen, and forgets an army after `turns` seasons.
//!
//! Here: a grey label "✕ 1 200" at the last known position of every ghost (faded out by the
//! parchment view, which draws its own tokens), and the hover bubble of a ghost ("Dernière
//! position connue", "Vue il y a 2 saisons") within [`GHOST_PICK_PX`] of the pointer.

use crate::ui::kit::viewport_to_ui;
use bevy::prelude::*;
use bevy::ui::UiScale;
use data_model::FactionId;
use serde_json::{Value, json};

use super::hover_text::{ArmyBubble, army_text};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::parchment::extras::{Ghosts, Seen};
use crate::campaign::parchment::overlay::{OverlayAssets, format_men};
use crate::campaign::terrain::surface::GroundHeight;
use crate::sim::Sim;

/// Pick radius of a ghost around its label (`GHOST_PICK_PX`).
pub const GHOST_PICK_PX: f32 = 22.0;
/// Height of the label above the ground, in marker units (`6.0 * scale` in Godot).
pub const LABEL_LIFT: f32 = 6.0;
const LABEL_WIDTH: f32 = 90.0;
const LABEL_HEIGHT: f32 = 20.0;
/// Grey of the label (`Color(0.62, 0.6, 0.58)`).
const LABEL_GREY: Color = Color::srgb(0.62, 0.6, 0.58);

/// UI label of one ghost.
#[derive(Component)]
pub struct GhostLabel(pub String);

/// Counters of the dump.
#[derive(Resource, Default, Debug)]
pub struct GhostStats {
    pub labels: usize,
    pub shown: usize,
}

/// Text of the label ("✕ 1 200").
pub fn label_text(men: u32) -> String {
    format!("✕ {}", format_men(men))
}

/// Keeps one label per ghost.
pub fn sync_labels(
    mut commands: Commands,
    ghosts: Res<Ghosts>,
    labels: Query<(Entity, &GhostLabel)>,
    mut texts: Query<(&GhostLabel, &mut Text)>,
    mut stats: ResMut<GhostStats>,
) {
    if !ghosts.is_changed() {
        return;
    }
    let mut have = std::collections::HashSet::new();
    for (entity, label) in &labels {
        if ghosts.0.iter().any(|(id, _)| *id == label.0) {
            have.insert(label.0.clone());
        } else {
            commands.entity(entity).despawn();
        }
    }
    for (id, seen) in &ghosts.0 {
        if have.contains(id) {
            continue;
        }
        commands.spawn((
            Name::new(format!("Ghost_{id}")),
            GhostLabel(id.clone()),
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(LABEL_WIDTH),
                height: Val::Px(LABEL_HEIGHT),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(-1),
            Text::new(label_text(seen.men)),
            TextFont {
                font_size: bevy::text::FontSize::Px(14.0),
                ..default()
            },
            TextColor(LABEL_GREY),
            TextLayout::justify(Justify::Center),
        ));
    }
    for (label, mut text) in &mut texts {
        if let Some((_, seen)) = ghosts.0.iter().find(|(id, _)| *id == label.0) {
            let wanted = label_text(seen.men);
            if text.0 != wanted {
                text.0 = wanted;
            }
        }
    }
    stats.labels = ghosts.0.len();
}

/// Screen position of the label of a ghost, if it is in front of the camera.
pub fn screen_of(
    seen: &Seen,
    camera: &Camera,
    transform: &GlobalTransform,
    ground: &GroundHeight,
) -> Option<Vec2> {
    let y = ground.at(seen.point.x, seen.point.y).unwrap_or(0.0);
    camera
        .world_to_viewport(
            transform,
            Vec3::new(seen.point.x, y + LABEL_LIFT, seen.point.y),
        )
        .ok()
}

/// Places the labels over the ghosts; the parchment view fades them out (it draws its own).
#[allow(clippy::too_many_arguments)]
pub fn place_labels(
    ghosts: Res<Ghosts>,
    zoom: Option<Res<ZoomView>>,
    assets: Option<Res<OverlayAssets>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ground: GroundHeight,
    mut labels: Query<(&GhostLabel, &mut Node, &mut Visibility, &mut TextColor)>,
    mut stats: ResMut<GhostStats>,
    ui_scale: Res<UiScale>,
) {
    let Some((camera, transform)) = camera.iter().find(|(c, _)| c.is_active) else {
        return;
    };
    let fade = 1.0 - zoom.map_or(0.0, |z| z.strategic).clamp(0.0, 1.0);
    let alpha = assets.map_or(0.6, |a| a.ghost_alpha) * fade;
    let viewport = camera
        .logical_viewport_rect()
        .unwrap_or(Rect::new(0.0, 0.0, 1.0e5, 1.0e5))
        .inflate(60.0);
    let mut shown = 0;
    for (label, mut node, mut visibility, mut color) in &mut labels {
        let at = ghosts
            .0
            .iter()
            .find(|(id, _)| *id == label.0)
            .and_then(|(_, seen)| screen_of(seen, camera, transform, &ground))
            .filter(|p| viewport.contains(*p) && alpha > 0.01);
        let wanted = if at.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
        if let Some(p) = at.map(|p| viewport_to_ui(p, &ui_scale)) {
            node.left = Val::Px((p.x - LABEL_WIDTH * 0.5).round());
            node.top = Val::Px((p.y - LABEL_HEIGHT * 0.5).round());
            color.0 = LABEL_GREY.with_alpha(alpha);
            shown += 1;
        }
    }
    stats.shown = shown;
}

/// Ghost nearest to `cursor` (screen) within [`GHOST_PICK_PX`].
pub fn ghost_at<'a>(
    ghosts: &'a Ghosts,
    cursor: Vec2,
    camera: &Camera,
    transform: &GlobalTransform,
    ground: &GroundHeight,
) -> Option<&'a (String, Seen)> {
    let mut best: Option<(&(String, Seen), f32)> = None;
    for entry in &ghosts.0 {
        let Some(p) = screen_of(&entry.1, camera, transform, ground) else {
            continue;
        };
        let distance = p.distance(cursor);
        if distance < GHOST_PICK_PX && best.is_none_or(|(_, d)| distance < d) {
            best = Some((entry, distance));
        }
    }
    best.map(|(entry, _)| entry)
}

/// Bubble of a ghost (`ghost_bubble_text`): last known situation, no composition.
pub fn ghost_bubble(sim: &Sim, seen: &Seen) -> Option<String> {
    let (data, state) = (sim.data.as_ref()?, sim.state.as_ref()?);
    let faction = FactionId::new(&seen.faction).ok()?;
    Some(army_text(&ArmyBubble {
        general_name: seen.general_name.clone(),
        faction_name: data.faction_name(&faction),
        men: seen.men,
        seen_ago: Some(state.turn.saturating_sub(seen.turn)),
        ..Default::default()
    }))
}

fn dump_ghosts(world: &World) -> (String, Value) {
    let ghosts = world.resource::<Ghosts>();
    let stats = world.resource::<GhostStats>();
    let sim = world.resource::<Sim>();
    let samples: Vec<Value> = ghosts
        .0
        .iter()
        .take(3)
        .map(|(id, seen)| {
            json!({
                "id": id,
                "faction": seen.faction,
                "men": seen.men,
                "turn": seen.turn,
                "point": [seen.point.x, seen.point.y],
                "label": label_text(seen.men),
                "bubble": ghost_bubble(sim, seen),
            })
        })
        .collect();
    (
        "campaign_ghosts".into(),
        json!({
            "ghosts": ghosts.0.len(),
            "labels": stats.labels,
            "shown": stats.shown,
            "turns_memory": world.get_resource::<OverlayAssets>().map(|a| a.memory_turns),
            "samples": samples,
        }),
    )
}

pub struct GhostPlugin;

impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GhostStats>()
            .register_dump(dump_ghosts)
            .add_systems(
                Update,
                (sync_labels, place_labels)
                    .chain()
                    .after(crate::campaign::parchment::extras::update_memory)
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_groups_thousands() {
        assert!(label_text(1200).starts_with("✕ "));
        assert!(label_text(1200).contains("200"));
    }

    #[test]
    fn one_label_per_ghost_and_gone_with_it() {
        let seen = |men| Seen {
            faction: "fac_england".into(),
            general_name: String::new(),
            point: Vec2::new(10.0, 20.0),
            men,
            turn: 1,
        };
        let mut app = App::new();
        app.init_resource::<Ghosts>()
            .init_resource::<GhostStats>()
            .add_systems(Update, sync_labels);
        app.world_mut().resource_mut::<Ghosts>().0 =
            vec![("army_1".into(), seen(1200)), ("army_2".into(), seen(300))];
        app.update();
        let count = |app: &mut App| {
            app.world_mut()
                .query::<&GhostLabel>()
                .iter(app.world())
                .count()
        };
        assert_eq!(count(&mut app), 2);
        assert_eq!(app.world().resource::<GhostStats>().labels, 2);
        app.world_mut().resource_mut::<Ghosts>().0 = vec![("army_2".into(), seen(250))];
        app.update();
        assert_eq!(count(&mut app), 1);
        let texts: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect();
        assert_eq!(texts, vec![label_text(250)]);
    }

    #[test]
    fn ghost_bubble_is_the_last_known_situation() {
        let text = army_text(&ArmyBubble {
            general_name: "Édouard".into(),
            faction_name: "Angleterre".into(),
            men: 1200,
            seen_ago: Some(2),
            ..Default::default()
        });
        assert!(text.contains("Dernière position connue — Ost de Édouard"));
        assert!(text.contains("Vue il y a 2 saisons"));
        assert!(!text.contains("Au repos"));
    }
}
