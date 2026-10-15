//! Stance badge at the top of an army's standard.
//!
//! Ports `map/stance_badge.gd`: a parchment disc ringed with gold and the ink icon of the stance
//! (`hud_stance_<key>`) above the finial, turned towards the camera, absent in the normal stance.
//! The standard (`armies::standard`) is a set of free entities, not a parent, so the badge is a
//! free entity that follows the finial. Not ported: the semi-transparent figures of the player's
//! army in ambush (`AMBUSH_TRANSPARENCY`; the figures belong to the armies lot).

use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_campaign::ArmyId;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::hover_text::visible_stance;
use crate::campaign::armies::standard::{StandardOf, StandardPart};
use crate::sim::Sim;
use crate::ui::kit::UiTheme;

/// Badge width in marker space (the flag is 3.4 wide).
pub const WORLD_WIDTH: f32 = 2.4;
/// Height above the finial.
pub const LIFT: f32 = 1.5;
const GOLD: Color = Color::srgb(0.78, 0.62, 0.22);
const PARCHMENT: Color = Color::srgb(0.95, 0.90, 0.78);

#[derive(Component)]
pub struct StanceBadge {
    pub army: ArmyId,
    pub stance: String,
}

#[derive(Resource, Default)]
pub struct Badges {
    entities: HashMap<ArmyId, Entity>,
    disc: Option<(
        Handle<Mesh>,
        Handle<StandardMaterial>,
        Handle<StandardMaterial>,
    )>,
}

pub struct StanceBadgePlugin;

impl Plugin for StanceBadgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Badges>()
            .register_dump(dump_badges)
            .add_systems(Update, update_badges.run_if(in_state(AppState::Campaign)))
            .add_systems(OnExit(AppState::Campaign), clear_badges);
    }
}

/// Stance key worth a badge (`None`: normal, siege, hidden ambush of an enemy).
pub fn badge_stance(stance: &str, is_player: bool) -> Option<&str> {
    visible_stance(stance, is_player)?;
    (stance != "siege").then_some(stance)
}

fn clear_badges(mut commands: Commands, mut badges: ResMut<Badges>) {
    for (_, entity) in badges.entities.drain() {
        commands.entity(entity).try_despawn();
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_badges(
    mut commands: Commands,
    mut badges: ResMut<Badges>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    theme: Res<UiTheme>,
    sim: Res<Sim>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    finials: Query<(&StandardOf, &StandardPart, &Transform, &Visibility), Without<StanceBadge>>,
    mut existing: Query<(&mut StanceBadge, &mut Transform, &mut Visibility)>,
) {
    let Some(state) = sim.state.as_ref() else {
        return;
    };
    let facing = camera.iter().next().map(|c| c.rotation());
    let mut live: Vec<ArmyId> = Vec::new();
    for (of, part, finial, finial_visibility) in &finials {
        if *part != StandardPart::Finial {
            continue;
        }
        let Some(army) = state.armies().get(&of.0) else {
            continue;
        };
        let is_player = army.faction == sim.player;
        let Some(stance) = badge_stance(army.stance.key(), is_player) else {
            continue;
        };
        let Some(icon) = theme
            .icons
            .get(&format!("hud_{}", icon_name(stance)))
            .map(|entry| entry.path.clone())
        else {
            continue;
        };
        live.push(of.0.clone());
        let position = finial.translation + Vec3::Y * LIFT * finial.scale.y;
        let visibility = if *finial_visibility == Visibility::Hidden {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if let Some(entity) = badges.entities.get(&of.0).copied()
            && let Ok((badge, mut transform, mut vis)) = existing.get_mut(entity)
        {
            transform.translation = position;
            transform.scale = finial.scale;
            if let Some(rotation) = facing {
                transform.rotation = rotation;
            }
            if *vis != visibility {
                *vis = visibility;
            }
            if badge.stance != stance {
                // Rare: the stance changed under a live badge; respawn it.
                commands.entity(entity).try_despawn();
                badges.entities.remove(&of.0);
            }
            continue;
        }
        let (mesh, gold, parchment) = badges
            .disc
            .get_or_insert_with(|| {
                (
                    meshes.add(Circle::new(WORLD_WIDTH * 0.5)),
                    materials.add(unlit(GOLD, None)),
                    materials.add(unlit(PARCHMENT, None)),
                )
            })
            .clone();
        let glyph_mesh = meshes.add(Rectangle::new(WORLD_WIDTH * 0.66, WORLD_WIDTH * 0.66));
        let glyph = materials.add(unlit(Color::WHITE, Some(assets.load(icon))));
        let entity = commands
            .spawn((
                Name::new(format!("StanceBadge_{}", of.0.as_str())),
                StanceBadge {
                    army: of.0.clone(),
                    stance: stance.to_owned(),
                },
                Transform::from_translation(position).with_scale(finial.scale),
                visibility,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(gold),
                    Transform::default(),
                ));
                parent.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(parchment),
                    Transform::from_xyz(0.0, 0.0, 0.01).with_scale(Vec3::splat(0.84)),
                ));
                parent.spawn((
                    Mesh3d(glyph_mesh),
                    MeshMaterial3d(glyph),
                    Transform::from_xyz(0.0, 0.0, 0.02),
                ));
            })
            .id();
        badges.entities.insert(of.0.clone(), entity);
    }
    let gone: Vec<ArmyId> = badges
        .entities
        .keys()
        .filter(|id| !live.contains(id))
        .cloned()
        .collect();
    for id in gone {
        if let Some(entity) = badges.entities.remove(&id) {
            commands.entity(entity).try_despawn();
        }
    }
}

fn icon_name(stance: &str) -> String {
    format!("stance_{stance}")
}

fn unlit(color: Color, texture: Option<Handle<Image>>) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        base_color_texture: texture,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

fn dump_badges(world: &World) -> (String, Value) {
    let mut query = world.try_query::<&StanceBadge>();
    let list: Vec<String> = query
        .as_mut()
        .map(|q| {
            q.iter(world)
                .map(|b| format!("{}:{}", b.army.as_str(), b.stance))
                .collect()
        })
        .unwrap_or_default();
    (
        "stance_badges".to_owned(),
        json!({ "count": list.len(), "badges": list }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_visible_non_siege_stances_get_a_badge() {
        assert_eq!(badge_stance("normal", true), None);
        assert_eq!(badge_stance("siege", true), None);
        assert_eq!(badge_stance("raid", false), Some("raid"));
        assert_eq!(badge_stance("ambush", true), Some("ambush"));
        assert_eq!(badge_stance("ambush", false), None);
    }
}
