//! Parchment extras on top of the overlay: weather clouds, ghost tokens (enemy armies lost from
//! sight) and the stance cues of the army tokens. Ports `_draw_weather`, `_draw_ghost_tokens`
//! and the badge part of `_draw_army_tokens` (game/scripts/map/parchment_overlay.gd), `ArmyMemory`
//! (army_memory.gd) and `StanceBadge.texture_for` (the ink icon `hud_stance_<key>`).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::glyphs;
use super::overlay::{
    FontPx, Layer, OverlayAssets, OverlayRoot, Projector, absolute, format_men, rgb, set_node,
    show, token_alpha, view_scale,
};
use crate::app::AppState;
use crate::campaign::armies::ArmyViews;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::panels::diplomacy::stance_badge::badge_stance;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::weather::CampaignWeather;
use crate::sim::Sim;
use crate::ui::kit::{UiCamera, UiTheme};

// ---- token cues --------------------------------------------------------------------------------

/// Stance badge to show on each army token (id to stance key) and the ink icon of each stance.
#[derive(Resource, Default)]
pub struct TokenCues {
    pub stance: HashMap<String, String>,
    pub icons: HashMap<String, Handle<Image>>,
    /// Factions that are allies or vassals of the player (green ring, `stance_cues.json` `friend`).
    pub friends: HashSet<String>,
}

/// Refreshes the stance of every army (`StanceBadge`: none in the normal stance, an enemy ambush
/// is never shown).
pub fn refresh_cues(
    sim: Res<Sim>,
    theme: Option<Res<UiTheme>>,
    asset_server: Res<AssetServer>,
    mut cues: ResMut<TokenCues>,
) {
    if !sim.is_changed() {
        return;
    }
    let (Some(state), Some(theme)) = (sim.state.as_ref(), theme) else {
        return;
    };
    if let Some(data) = sim.data.as_ref() {
        let plan = sim_campaign::plan_cache::PlanCache::new(state);
        let friends: HashSet<String> = state
            .armies()
            .values()
            .map(|army| army.faction.clone())
            .filter(|faction| *faction != sim.player)
            .filter(|faction| {
                let key = sim_campaign::stance::diplomatic_stance_cached(
                    &plan,
                    data,
                    &sim.player,
                    faction,
                )
                .key();
                super::overlay::cue_category(key, false) == "friend"
            })
            .map(|faction| faction.as_str().to_owned())
            .collect();
        if cues.friends != friends {
            cues.friends = friends;
        }
    }
    let mut stance = HashMap::new();
    for (id, army) in state.armies() {
        if let Some(key) = badge_stance(army.stance.key(), army.faction == sim.player) {
            stance.insert(id.to_string(), key.to_owned());
        }
    }
    for key in stance.values() {
        if !cues.icons.contains_key(key)
            && let Some(entry) = theme.icons.get(&format!("hud_stance_{key}"))
        {
            let handle = asset_server.load::<Image>(entry.path.clone());
            cues.icons.insert(key.clone(), handle);
        }
    }
    if cues.stance != stance {
        cues.stance = stance;
    }
}

// ---- army memory -------------------------------------------------------------------------------

/// Last known situation of an enemy army.
#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    pub faction: String,
    /// Name of the commander when last seen (empty: none), for the hover bubble of the ghost.
    pub general_name: String,
    pub point: Vec2,
    pub men: u32,
    pub turn: u32,
}

/// Enemy armies lost from sight (`ArmyMemory`): ephemeral, UI only, never reads the simulation.
#[derive(Resource, Default, Debug)]
pub struct ArmyMemory {
    seen: HashMap<String, Seen>,
    pub revision: u64,
}

impl ArmyMemory {
    pub fn note(&mut self, id: &str, seen: Seen) {
        if self.seen.get(id) != Some(&seen) {
            self.seen.insert(id.to_owned(), seen);
            self.revision += 1;
        }
    }

    pub fn forget(&mut self, id: &str) {
        if self.seen.remove(id).is_some() {
            self.revision += 1;
        }
    }

    pub fn clear(&mut self) {
        self.seen.clear();
        self.revision += 1;
    }

    /// Ghosts at `turn`: noted armies absent from `visible_now`, seen at most `ttl` seasons ago
    /// (older ones are forgotten). Sorted by id.
    pub fn ghosts(
        &mut self,
        turn: u32,
        visible_now: &HashSet<String>,
        ttl: u32,
    ) -> Vec<(String, Seen)> {
        let stale: Vec<String> = self
            .seen
            .iter()
            .filter(|(id, s)| !visible_now.contains(*id) && turn.saturating_sub(s.turn) > ttl)
            .map(|(id, _)| id.clone())
            .collect();
        for id in stale {
            self.forget(&id);
        }
        let mut out: Vec<(String, Seen)> = self
            .seen
            .iter()
            .filter(|(id, _)| !visible_now.contains(*id))
            .map(|(id, s)| (id.clone(), s.clone()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

/// Ghosts of the current frame (what the ghost tokens show).
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct Ghosts(pub Vec<(String, Seen)>);

fn clear_memory(mut memory: ResMut<ArmyMemory>, mut ghosts: ResMut<Ghosts>) {
    memory.clear();
    ghosts.0.clear();
}

/// Notes every hostile army in sight, forgets those destroyed in sight, refreshes the ghosts.
pub fn update_memory(
    sim: Res<Sim>,
    views: Option<Res<ArmyViews>>,
    fog: Res<crate::campaign::weather::fog::FogOfWar>,
    assets: Option<Res<OverlayAssets>>,
    mut memory: ResMut<ArmyMemory>,
    mut ghosts: ResMut<Ghosts>,
) {
    let (Some(views), Some(assets), Some(state)) = (views, assets, sim.state.as_ref()) else {
        return;
    };
    if !(views.is_changed() || sim.is_changed()) {
        return;
    }
    let turn = state.turn();
    let mut visible: HashSet<String> = HashSet::new();
    for view in &views.0 {
        let id = view.id.to_string();
        if view.hostile && !view.player {
            memory.note(
                &id,
                Seen {
                    faction: view.faction.to_string(),
                    general_name: sim
                        .data
                        .as_ref()
                        .zip(view.general.as_deref())
                        .and_then(|(data, id)| {
                            data_model::CharacterId::new(id)
                                .ok()
                                .map(|id| state.character_name(data, &id))
                        })
                        .unwrap_or_default(),
                    point: Vec2::from(view.point),
                    men: view.men,
                    turn,
                },
            );
        }
        visible.insert(id);
    }
    // Destroyed (or merged) under the player's eyes: not a ghost.
    let gone: Vec<String> = memory
        .seen
        .iter()
        .filter(|(id, s)| {
            !visible.contains(*id)
                && state.armies().keys().all(|k| k.as_str() != id.as_str())
                && fog.is_point_visible(s.point.x, s.point.y)
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in gone {
        memory.forget(&id);
    }
    let list = memory.ghosts(turn, &visible, assets.memory_turns);
    if ghosts.0 != list {
        ghosts.0 = list;
    }
}

// ---- ghost tokens ------------------------------------------------------------------------------

#[derive(Component)]
pub struct GhostNode(pub String);

#[derive(Component)]
pub struct GhostMen;

/// Keeps one node per ghost.
pub fn sync_ghosts(
    mut commands: Commands,
    ghosts: Res<Ghosts>,
    theme: Option<Res<UiTheme>>,
    layers: Query<(Entity, &Layer)>,
    nodes: Query<(Entity, &GhostNode)>,
) {
    let Some(theme) = theme else {
        return;
    };
    if !ghosts.is_changed() {
        return;
    }
    let Some((layer, _)) = layers.iter().find(|(_, l)| **l == Layer::Tokens) else {
        return;
    };
    let wanted: HashSet<&str> = ghosts.0.iter().map(|(id, _)| id.as_str()).collect();
    let mut have = HashSet::new();
    for (entity, node) in &nodes {
        if wanted.contains(node.0.as_str()) {
            have.insert(node.0.clone());
        } else {
            commands.entity(entity).despawn();
        }
    }
    for (id, _) in &ghosts.0 {
        if have.contains(id) {
            continue;
        }
        let ghost = commands
            .spawn((
                Name::new(format!("parchment:ghost:{id}")),
                GhostNode(id.clone()),
                Node {
                    position_type: PositionType::Absolute,
                    border_radius: BorderRadius::MAX,
                    border: UiRect::all(Val::Px(1.4)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .with_children(|g| {
                g.spawn((
                    GhostMen,
                    absolute(),
                    Text::new(""),
                    TextFont {
                        font: theme.font_title.clone(),
                        font_size: 14.0.into(),
                        ..default()
                    },
                    TextColor(Color::NONE),
                    TextShadow {
                        offset: Vec2::new(1.0, 1.0),
                        color: Color::NONE,
                    },
                    Pickable::IGNORE,
                ));
            })
            .id();
        // Godot draws the ghosts before the tokens: they go under them.
        commands.entity(layer).insert_child(0, ghost);
    }
}

/// Places the ghosts: grey disc with an ink outline and the last known strength below.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ghosts(
    zoom: Res<ZoomView>,
    ghosts: Res<Ghosts>,
    assets: Option<Res<OverlayAssets>>,
    ui_scale: Res<UiScale>,
    ground: GroundHeight,
    cameras: Query<(&Camera, &GlobalTransform), (With<Camera3d>, Without<UiCamera>)>,
    mut nodes: Query<
        (
            &GhostNode,
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        Without<GhostMen>,
    >,
    mut texts: Query<
        (
            &mut Node,
            &mut Text,
            &mut TextColor,
            &mut TextShadow,
            &mut TextFont,
        ),
        With<GhostMen>,
    >,
) {
    let (Some(assets), Some((camera, transform))) =
        (assets, cameras.iter().find(|(c, _)| c.is_active))
    else {
        return;
    };
    let ta = token_alpha(zoom.strategic);
    let proj = Projector {
        camera,
        transform,
        ui_scale: ui_scale.0.max(0.1),
    };
    let viewport = camera
        .logical_viewport_size()
        .unwrap_or(Vec2::splat(4000.0))
        / proj.ui_scale;
    let view = Rect::from_corners(Vec2::splat(-60.0), viewport + 60.0);
    let s = view_scale(zoom.distance);
    let r = 10.0 * s;
    let alpha = assets.ghost_alpha * ta;
    for (ghost, mut node, mut visibility, mut background, mut border, children) in &mut nodes {
        let seen = ghosts.0.iter().find(|(id, _)| *id == ghost.0).map(|g| &g.1);
        let at = seen.and_then(|seen| {
            let world = Vec3::new(
                seen.point.x,
                ground.at(seen.point.x, seen.point.y).unwrap_or(0.0),
                seen.point.y,
            );
            proj.at(world).filter(|p| view.contains(*p) && ta > 0.01)
        });
        let (Some(seen), Some(p)) = (seen, at) else {
            show(&mut visibility, false);
            continue;
        };
        show(&mut visibility, true);
        set_node(&mut node, p - Vec2::splat(r), Some(Vec2::splat(2.0 * r)));
        background.0 = Color::srgba(0.55, 0.53, 0.5, alpha);
        *border = BorderColor::all(rgb(glyphs::INK, alpha));
        let men = format_men(seen.men);
        let fs = (12.0 * s).clamp(11.0, 15.0).floor();
        for child in children.iter() {
            let Ok((mut text_node, mut text, mut color, mut shadow, mut font)) =
                texts.get_mut(child)
            else {
                continue;
            };
            if text.0 != men {
                text.0 = men.clone();
            }
            if font.font_size.px() != fs {
                font.font_size = fs.into();
            }
            let w = men.chars().count() as f32 * fs * 0.5;
            set_node(&mut text_node, Vec2::new(r - w * 0.5, 2.0 * r + 2.0), None);
            color.0 = rgb(glyphs::INK, alpha);
            shadow.color = rgb(glyphs::PAPER, alpha);
        }
    }
}

// ---- weather -----------------------------------------------------------------------------------

#[derive(Component)]
pub struct WeatherIcon {
    pub index: usize,
    pub id: String,
    pub px: Vec2,
}

/// Counters for the dump.
#[derive(Resource, Default, Debug)]
pub struct ExtrasStats {
    pub weather_shown: usize,
    pub weather_kinds: Vec<String>,
    pub ghosts_shown: usize,
}

/// One icon node per province, built once the glyphs and the province map are there.
pub fn spawn_weather(
    mut commands: Commands,
    map: Option<Res<ProvinceMap>>,
    assets: Option<Res<OverlayAssets>>,
    layers: Query<(Entity, &Layer)>,
    existing: Query<(), With<WeatherIcon>>,
) {
    let (Some(map), Some(assets)) = (map, assets) else {
        return;
    };
    if !existing.is_empty() || assets.weather_cell <= 0.0 {
        return;
    }
    let Some((layer, _)) = layers.iter().find(|(_, l)| **l == Layer::Towns) else {
        return;
    };
    let Some((_, handle)) = assets.glyphs.get("rain") else {
        return;
    };
    for (index, province) in map.provinces().iter().enumerate() {
        let node = commands
            .spawn((
                Name::new(format!("parchment:weather:{}", province.id)),
                WeatherIcon {
                    index,
                    id: province.id.clone(),
                    px: Vec2::from(province.centroid),
                },
                absolute(),
                ImageNode::new(handle.clone()),
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(layer).add_child(node);
    }
}

/// One cloud per large screen cell (`clouds.parchment_icon_cell_px`), above the province centre.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_weather(
    zoom: Res<ZoomView>,
    weather: Option<Res<CampaignWeather>>,
    assets: Option<Res<OverlayAssets>>,
    ui_scale: Res<UiScale>,
    ground: GroundHeight,
    cameras: Query<(&Camera, &GlobalTransform), (With<Camera3d>, Without<UiCamera>)>,
    root: Query<&Visibility, (With<OverlayRoot>, Without<WeatherIcon>)>,
    mut stats: ResMut<ExtrasStats>,
    mut icons: Query<(&WeatherIcon, &mut Node, &mut ImageNode, &mut Visibility)>,
) {
    let (Some(assets), Some((camera, transform))) =
        (assets, cameras.iter().find(|(c, _)| c.is_active))
    else {
        return;
    };
    let on = root.iter().any(|v| *v != Visibility::Hidden);
    let mut list: Vec<_> = icons.iter_mut().collect();
    list.sort_by_key(|(icon, ..)| icon.index);
    let proj = Projector {
        camera,
        transform,
        ui_scale: ui_scale.0.max(0.1),
    };
    let viewport = camera
        .logical_viewport_size()
        .unwrap_or(Vec2::splat(4000.0))
        / proj.ui_scale;
    let view = Rect::from_corners(Vec2::splat(-60.0), viewport + 60.0);
    let s = view_scale(zoom.distance);
    let cell = (assets.weather_cell * s).max(1.0);
    let mut occupied: HashSet<(i32, i32)> = HashSet::new();
    let mut shown = 0;
    let mut kinds: Vec<String> = Vec::new();
    for (icon, node, image, visibility) in list {
        let mut node = node;
        let mut image = image;
        let mut visibility = visibility;
        let kind = weather
            .as_ref()
            .map(|w| w.entry(&icon.id).kind.key())
            .unwrap_or("clear");
        let glyph = assets.glyphs.get(kind);
        let spot = glyph.filter(|_| on).and_then(|_| {
            let world = Vec3::new(
                icon.px.x,
                ground.at(icon.px.x, icon.px.y).unwrap_or(0.0),
                icon.px.y,
            );
            proj.at(world)
                .map(|p| p + Vec2::new(18.0, -26.0) * s)
                .filter(|p| view.contains(*p))
        });
        let (Some((spec, handle)), Some(p)) = (glyph, spot) else {
            show(&mut visibility, false);
            continue;
        };
        if !occupied.insert(((p.x / cell) as i32, (p.y / cell) as i32)) {
            show(&mut visibility, false);
            continue;
        }
        let k = 11.0 * s / spec.unit;
        set_node(&mut node, p - spec.anchor * k, Some(spec.size * k));
        if image.image != *handle {
            image.image = handle.clone();
        }
        image.color = Color::srgba(1.0, 1.0, 1.0, 0.9 * zoom.strategic);
        show(&mut visibility, true);
        shown += 1;
        if !kinds.iter().any(|k2| k2 == kind) {
            kinds.push(kind.to_owned());
        }
    }
    stats.weather_shown = shown;
    stats.weather_kinds = kinds;
}

/// Registers the systems (called by the plugin).
pub fn register(app: &mut App) {
    app.init_resource::<TokenCues>()
        .init_resource::<ArmyMemory>()
        .init_resource::<Ghosts>()
        .init_resource::<ExtrasStats>()
        .add_systems(OnEnter(AppState::Campaign), clear_memory)
        .add_systems(
            Update,
            (
                refresh_cues,
                update_memory,
                sync_ghosts,
                update_ghosts,
                spawn_weather,
                update_weather,
            )
                .chain()
                .after(super::overlay::update_overlay)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(turn: u32) -> Seen {
        Seen {
            faction: "fac_england".into(),
            general_name: String::new(),
            point: Vec2::new(10.0, 20.0),
            men: 1200,
            turn,
        }
    }

    #[test]
    fn a_lost_army_is_a_ghost_for_ttl_seasons_then_forgotten() {
        let mut memory = ArmyMemory::default();
        memory.note("a1", seen(3));
        let none = HashSet::new();
        assert_eq!(memory.ghosts(3, &none, 4).len(), 1, "same turn");
        assert_eq!(memory.ghosts(7, &none, 4).len(), 1, "4 seasons later");
        assert!(memory.ghosts(8, &none, 4).is_empty(), "forgotten");
        assert!(memory.ghosts(8, &none, 4).is_empty());
    }

    #[test]
    fn an_army_back_in_sight_is_not_a_ghost() {
        let mut memory = ArmyMemory::default();
        memory.note("a1", seen(3));
        let visible: HashSet<String> = ["a1".to_owned()].into();
        assert!(memory.ghosts(4, &visible, 4).is_empty());
        assert_eq!(memory.ghosts(4, &HashSet::new(), 4).len(), 1);
    }

    #[test]
    fn zero_ttl_keeps_nothing_after_the_turn() {
        let mut memory = ArmyMemory::default();
        memory.note("a1", seen(3));
        assert!(memory.ghosts(4, &HashSet::new(), 0).is_empty());
    }
}
