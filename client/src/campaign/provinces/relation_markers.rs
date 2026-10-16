//! Colour-blind symbols of the Diplomacy map mode. Ports `map_mode_controller.gd::
//! _place_relation_markers` (⚔ war, ⚭ alliance, ⚜ vassal on each province, readable without the
//! colours) and `diplomatic_stances.gd::SYMBOL_KEYS`. Godot puts a billboard `Label3D` per
//! province; here a pool of screen-space texts follows the projected centroids (the nearest to
//! the centre of the screen first), like the region labels.

use bevy::prelude::*;

use super::data::ProvinceMap;
use super::modes::{MapMode, MapModeState};
use super::overlay::OverlayState;
use crate::app::AppState;
use crate::settings::palette::Palette;
use crate::ui::kit::UiTheme;
use crate::ui::kit::contrast::relation_symbol;

/// Texts available per frame (provinces on screen beyond this are left unmarked).
const POOL: usize = 160;
/// Glyph size in px (`font_size 72` * `pixel_size 0.0009` is fixed-size in Godot: ~ 24 px).
const FONT_PX: f32 = 24.0;
/// Godot `DiplomaticStances.SYMBOL_KEYS`: stance key -> relation key (unmarked stances absent).
pub fn relation_of_stance(stance: &str) -> Option<&'static str> {
    match stance {
        "ally" => Some("alliance"),
        "war" => Some("war"),
        "vassal" => Some("vassal"),
        _ => None,
    }
}

#[derive(Component)]
struct RelationMarker(usize);

/// Markers placed last frame (dump).
#[derive(Resource, Default, Debug)]
pub struct RelationMarkerStats {
    pub shown: usize,
    pub symbols: Vec<String>,
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<RelationMarkerStats>()
        .add_systems(OnEnter(AppState::Campaign), spawn_pool)
        .add_systems(
            Update,
            update_markers
                .run_if(resource_exists::<ProvinceMap>)
                .run_if(resource_exists::<OverlayState>)
                .run_if(in_state(AppState::Campaign)),
        );
}

fn spawn_pool(mut commands: Commands, theme: Option<Res<UiTheme>>) {
    for slot in 0..POOL {
        let font = theme
            .as_ref()
            .map_or_else(TextFont::default, |t| t.text_font(true, false, FONT_PX));
        commands.spawn((
            Name::new(format!("RelationMarker{slot}")),
            RelationMarker(slot),
            DespawnOnExit(AppState::Campaign),
            Text::new(""),
            font,
            TextColor(Color::NONE),
            TextShadow::default(),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(-1),
            Pickable::IGNORE,
        ));
    }
}

/// Darkened stance colour of a marker (`RELATION_COLORS.darkened(0.35)`).
fn marker_color(stance: &str) -> Color {
    super::modes::diplomacy_color(stance)
        .map_or(Color::BLACK, |c| Color::from(c).mix(&Color::BLACK, 0.35))
}

#[allow(clippy::too_many_arguments)]
fn update_markers(
    palette: Option<Res<Palette>>,
    mode: Res<MapModeState>,
    overlay: Res<OverlayState>,
    map: Res<ProvinceMap>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ground: crate::campaign::terrain::surface::GroundHeight,
    mut stats: ResMut<RelationMarkerStats>,
    mut slots: Query<(
        &RelationMarker,
        &mut Text,
        &mut Node,
        &mut TextColor,
        &mut Visibility,
    )>,
) {
    stats.shown = 0;
    stats.symbols.clear();
    let active = mode.mode == MapMode::Diplomacy && palette.is_some_and(|p| p.colorblind);
    let camera = cameras.iter().find(|(c, _)| c.is_active);
    let (Some((camera, transform)), true) = (camera, active) else {
        for (_, _, _, _, mut visibility) in &mut slots {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let viewport = camera.logical_viewport_size().unwrap_or(Vec2::ONE);
    let centre = viewport * 0.5;
    let mut candidates: Vec<(f32, Vec2, &'static str, Color)> = Vec::new();
    for province in map.provinces() {
        let index = province.index as u16;
        let controller = overlay.view.controller_of(index);
        if controller.is_empty() {
            continue;
        }
        let stance = overlay.view.stance_of(controller);
        let Some(relation) = relation_of_stance(stance) else {
            continue;
        };
        let (cx, cz) = (province.centroid[0], province.centroid[1]);
        let world = Vec3::new(cx, ground.at(cx, cz).unwrap_or(0.0) + 0.5, cz);
        let Ok(screen) = camera.world_to_viewport(transform, world) else {
            continue;
        };
        if screen.x < 0.0 || screen.y < 0.0 || screen.x > viewport.x || screen.y > viewport.y {
            continue;
        }
        candidates.push((
            screen.distance_squared(centre),
            screen,
            relation_symbol(relation),
            marker_color(stance),
        ));
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (slot, mut text, mut node, mut color, mut visibility) in &mut slots {
        match candidates.get(slot.0) {
            Some((_, screen, symbol, tint)) => {
                if text.0 != *symbol {
                    text.0 = (*symbol).to_owned();
                }
                node.left = Val::Px(screen.x - FONT_PX * 0.5);
                node.top = Val::Px(screen.y - FONT_PX * 0.6);
                color.0 = *tint;
                *visibility = Visibility::Inherited;
                stats.shown += 1;
                if stats.symbols.len() < 8 {
                    stats.symbols.push((*symbol).to_owned());
                }
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_war_alliance_and_vassal_are_marked() {
        assert_eq!(relation_of_stance("war"), Some("war"));
        assert_eq!(relation_of_stance("ally"), Some("alliance"));
        assert_eq!(relation_of_stance("vassal"), Some("vassal"));
        for other in ["self", "neutral", "agreement", "tension", ""] {
            assert_eq!(relation_of_stance(other), None);
        }
        assert_eq!(relation_symbol("alliance"), "⚭");
    }
}
