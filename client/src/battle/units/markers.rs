//! Floating regiment markers: above every regiment a plate in the side's colours with the
//! class icon, strength and morale bars, state badges and a star for the general; in a distant
//! view the plates of one side close to each other merge into a disc with the regiment count.
//! Overlapping plates are pushed aside. Port of game/scripts/battle/battle_unit_markers.gd
//! (screen-space Control), as Bevy UI nodes.
//!
//! A left click on a plate sends [`SelectBattleUnits`] (the regiments of the plate or group;
//! shift adds). Right click to attack and the hover name box stay with the orders and HUD lots.

use std::collections::HashMap;
use std::collections::hash_map::Entry as MapEntry;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::UiTransform;
use serde_json::json;
use sim_battle::{SideId, UnitState};

use super::banners::{BANNER_HEIGHT, banner_scale};
use super::{PoseFrames, drawn_units};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::orders::{BattleSelection, SelectBattleUnits};
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};
use crate::ui::kit::UiTheme;
use crate::ui::kit::icons::icon;

pub const WIDTH: f32 = 38.0;
pub const PLAQUE: f32 = 32.0;
const BAR_H: f32 = 4.0;
const MORALE_H: f32 = 3.0;
const TIP: f32 = 7.0;
pub const HEIGHT: f32 = PLAQUE + 2.0 + BAR_H + 1.0 + MORALE_H + TIP;
const GAP: f32 = 2.0;
/// Width of the badge column to the right of a plate (counted in overlaps).
const BADGE_W: f32 = 14.0;
const MAX_STACK: i32 = 16;
/// Distance (camera, m) above which close plates merge, with hysteresis.
pub const CLUSTER_ON: f32 = 480.0;
pub const CLUSTER_OFF: f32 = 420.0;
/// Two plates of one side closer than this (px) merge.
pub const CLUSTER_PX: f32 = 70.0;
/// Camera feel: markers shrink from `start` to `end` metres down to `min_scale`.
const SHRINK_START_M: f32 = 260.0;
const SHRINK_END_M: f32 = 700.0;
const MIN_SCALE: f32 = 0.5;
const NUDGES: [(f32, f32); 8] = [
    (0.0, -1.0),
    (1.0, 0.0),
    (-1.0, 0.0),
    (1.0, -1.0),
    (-1.0, -1.0),
    (0.0, -2.0),
    (2.0, 0.0),
    (-2.0, 0.0),
];
const INK: Color = Color::srgb(0.16, 0.10, 0.05);
const PARCHMENT: Color = Color::srgb(0.95, 0.90, 0.78);
const GOLD: Color = Color::srgb(1.0, 0.82, 0.22);
const ROUT_RED: Color = Color::srgb(0.85, 0.12, 0.08);

/// Marker scale for a camera distance (`scale_for_distance`).
pub fn scale_for_distance(distance: f32, start: f32, end: f32, min_scale: f32) -> f32 {
    if distance <= start || end <= start {
        return 1.0;
    }
    1.0 + (min_scale - 1.0) * ((distance - start) / (end - start)).clamp(0.0, 1.0)
}

/// Morale bar colour.
pub fn morale_color(ratio: f32) -> Color {
    if ratio >= 0.6 {
        Color::srgb(0.35, 0.72, 0.3)
    } else if ratio >= 0.3 {
        Color::srgb(0.92, 0.68, 0.15)
    } else {
        Color::srgb(0.85, 0.2, 0.12)
    }
}

/// A regiment as the layout sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: u32,
    pub side: SideId,
    pub anchor: Vec2,
    pub members: Vec<u32>,
    pub soldiers: f32,
    pub initial: f32,
    pub morale: f32,
}

impl Entry {
    fn single(
        id: u32,
        side: SideId,
        anchor: Vec2,
        soldiers: f32,
        initial: f32,
        morale: f32,
    ) -> Self {
        Self {
            id,
            side,
            anchor,
            members: vec![id],
            soldiers,
            initial,
            morale,
        }
    }
}

/// Groups the entries of one side whose anchors lie within [`CLUSTER_PX`] of a group's centre
/// (greedy in the order received, hence stable from frame to frame). A group keeps the id of
/// its first regiment, averages the anchors and sums soldiers; morale is the soldier-weighted mean.
pub fn cluster_entries(entries: &[Entry]) -> Vec<Entry> {
    struct Group {
        side: SideId,
        sum: Vec2,
        entries: Vec<Entry>,
    }
    let mut groups: Vec<Group> = Vec::new();
    for entry in entries {
        let mut best: Option<usize> = None;
        let mut best_distance = CLUSTER_PX;
        for (i, group) in groups.iter().enumerate() {
            if group.side != entry.side {
                continue;
            }
            let center = group.sum / group.entries.len() as f32;
            let distance = center.distance(entry.anchor);
            if distance < best_distance {
                best_distance = distance;
                best = Some(i);
            }
        }
        match best {
            Some(i) => {
                groups[i].sum += entry.anchor;
                groups[i].entries.push(entry.clone());
            }
            None => groups.push(Group {
                side: entry.side,
                sum: entry.anchor,
                entries: vec![entry.clone()],
            }),
        }
    }
    groups
        .into_iter()
        .map(|g| {
            let n = g.entries.len() as f32;
            let soldiers: f32 = g.entries.iter().map(|e| e.soldiers).sum();
            let weighted: f32 = g
                .entries
                .iter()
                .map(|e| e.morale * e.soldiers.max(1.0))
                .sum();
            let weight: f32 = g.entries.iter().map(|e| e.soldiers.max(1.0)).sum();
            Entry {
                id: g.entries[0].id,
                side: g.side,
                anchor: g.sum / n,
                members: g.entries.iter().flat_map(|e| e.members.clone()).collect(),
                soldiers,
                initial: g.entries.iter().map(|e| e.initial).sum(),
                morale: weighted / weight,
            }
        })
        .collect()
}

fn overlaps(rect: Rect, others: &[Rect]) -> bool {
    let wide = Rect::new(rect.min.x, rect.min.y, rect.max.x + BADGE_W, rect.max.y);
    others.iter().any(|other| {
        let grown = Rect::new(
            other.min.x - GAP,
            other.min.y - GAP,
            other.max.x + BADGE_W + GAP,
            other.max.y + GAP,
        );
        !(grown.max.x <= wide.min.x
            || grown.min.x >= wide.max.x
            || grown.max.y <= wide.min.y
            || grown.min.y >= wide.max.y)
    })
}

/// Rectangles of the markers, the nearest to the camera (lowest on screen) first so that they
/// keep their place; a marker that would overlap is nudged aside, then stacked upwards.
/// Returns `(entry index, rect, displaced)` in placement order.
pub fn layout(anchors: &[Vec2], scale: f32) -> Vec<(usize, Rect, bool)> {
    let mut order: Vec<usize> = (0..anchors.len()).collect();
    order.sort_by(|a, b| anchors[*b].y.total_cmp(&anchors[*a].y));
    let mut rects: Vec<Rect> = Vec::new();
    let mut out = Vec::new();
    for index in order {
        let anchor = anchors[index];
        let size = Vec2::new(WIDTH, HEIGHT) * scale;
        let base = Rect::from_corners(
            Vec2::new(anchor.x - size.x * 0.5, anchor.y - size.y),
            Vec2::new(anchor.x + size.x * 0.5, anchor.y),
        );
        let mut rect = base;
        let mut displaced = false;
        if overlaps(base, &rects) {
            let mut placed = false;
            for (nx, ny) in NUDGES {
                let shift = Vec2::new(nx * (WIDTH + BADGE_W + GAP), ny * (HEIGHT + GAP)) * scale;
                let candidate = Rect::from_corners(base.min + shift, base.max + shift);
                if !overlaps(candidate, &rects) {
                    rect = candidate;
                    placed = true;
                    break;
                }
            }
            let mut level = 3;
            while !placed && level < MAX_STACK {
                let shift = Vec2::new(0.0, level as f32 * (HEIGHT + GAP) * scale);
                rect = Rect::from_corners(base.min - shift, base.max - shift);
                placed = !overlaps(rect, &rects);
                level += 1;
            }
            displaced = true;
        }
        rects.push(rect);
        out.push((index, rect, displaced));
    }
    out
}

/// Badge letters of a regiment by priority (at most three): rout, rallied, charge, melee, shoot.
pub fn state_badges(state: UnitState) -> Vec<(&'static str, Color)> {
    match state {
        UnitState::Routing => vec![("D", ROUT_RED)],
        UnitState::Rallied => vec![("R", Color::srgb(0.35, 0.72, 0.3))],
        UnitState::Charging => vec![("C", Color::srgb(0.9, 0.5, 0.15))],
        UnitState::Melee => vec![("M", Color::srgb(0.85, 0.2, 0.12))],
        UnitState::Shooting => vec![("T", Color::srgb(0.3, 0.5, 0.85))],
        UnitState::Climbing => vec![("E", Color::srgb(0.6, 0.6, 0.6))],
        _ => Vec::new(),
    }
}

/// Counters of the markers (dump section `battle_markers`).
#[derive(Resource, Default, Debug, Clone)]
pub struct MarkerStats {
    pub plates: usize,
    pub clusters: usize,
    pub displaced: usize,
    pub on_screen: usize,
    pub scale: f32,
    pub clustered: bool,
    pub clicks: usize,
}

struct Plate {
    root: Entity,
    plaque: Entity,
    icon: Entity,
    count: Entity,
    star: Entity,
    strength: Entity,
    morale: Entity,
    badges: Entity,
    tip: Entity,
}

#[derive(Resource, Default)]
struct Markers {
    plates: HashMap<u32, Plate>,
    clustered: bool,
}

#[derive(Component)]
struct MarkerRoot(u32);

pub struct BattleMarkersPlugin;

impl Plugin for BattleMarkersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarkerStats>()
            .init_resource::<Markers>()
            .register_dump(dump_markers)
            .add_systems(
                Update,
                (update_markers, click_markers)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet)
                    .after(super::regiments::update_regiments),
            )
            .add_systems(OnExit(AppState::Battle), clear_markers);
    }
}

fn clear_markers(mut markers: ResMut<Markers>, mut stats: ResMut<MarkerStats>) {
    *markers = Markers::default();
    *stats = MarkerStats::default();
}

fn dump_markers(world: &World) -> (String, serde_json::Value) {
    let s = world
        .get_resource::<MarkerStats>()
        .cloned()
        .unwrap_or_default();
    (
        "battle_markers".into(),
        json!({
            "plates": s.plates,
            "clusters": s.clusters,
            "displaced": s.displaced,
            "on_screen": s.on_screen,
            "scale": s.scale,
            "clustered": s.clustered,
            "clicks": s.clicks,
        }),
    )
}

fn spawn_plate(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    unit_id: u32,
    icon_name: &str,
) -> Plate {
    let abs = |left: f32, top: f32, width: f32, height: f32| Node {
        position_type: PositionType::Absolute,
        left: Val::Px(left),
        top: Val::Px(top),
        width: Val::Px(width),
        height: Val::Px(height),
        ..default()
    };
    let mut ids = Vec::new();
    let root = commands
        .spawn((
            Name::new(format!("Marker{unit_id}")),
            MarkerRoot(unit_id),
            Hovered(false),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(WIDTH + BADGE_W),
                height: Val::Px(HEIGHT),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(-5),
            Visibility::Hidden,
            UiTransform::default(),
            DespawnOnExit(AppState::Battle),
        ))
        .with_children(|root| {
            let plaque = root
                .spawn((
                    Node {
                        border: UiRect::all(Val::Px(2.0)),
                        padding: UiRect::all(Val::Px(1.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..abs(0.0, 0.0, WIDTH, PLAQUE)
                    },
                    BackgroundColor(PARCHMENT),
                    BorderColor::all(INK),
                ))
                .with_children(|plaque| {
                    let ico = plaque
                        .spawn(icon(theme, assets, icon_name, PLAQUE - 6.0))
                        .id();
                    ids.push(ico);
                    let count = plaque
                        .spawn((
                            Text::new(""),
                            TextFont::from_font_size(16.0),
                            TextColor(INK),
                            Visibility::Hidden,
                            Node {
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                        ))
                        .id();
                    ids.push(count);
                    let star = plaque
                        .spawn((
                            Text::new("★"),
                            TextFont::from_font_size(11.0),
                            TextColor(GOLD),
                            Visibility::Hidden,
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(-2.0),
                                ..default()
                            },
                        ))
                        .id();
                    ids.push(star);
                })
                .id();
            ids.push(plaque);
            let strength = root
                .spawn((
                    abs(0.0, PLAQUE + 2.0, WIDTH, BAR_H),
                    BackgroundColor(Color::srgba(0.1, 0.07, 0.04, 0.85)),
                ))
                .with_children(|bar| {
                    ids.push(
                        bar.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.93, 0.9, 0.8)),
                        ))
                        .id(),
                    );
                })
                .id();
            let morale = root
                .spawn((
                    abs(0.0, PLAQUE + 2.0 + BAR_H + 1.0, WIDTH, MORALE_H),
                    BackgroundColor(Color::srgba(0.1, 0.07, 0.04, 0.85)),
                ))
                .with_children(|bar| {
                    ids.push(
                        bar.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.35, 0.72, 0.3)),
                        ))
                        .id(),
                    );
                })
                .id();
            let badges = root
                .spawn((
                    Text::new(""),
                    TextFont::from_font_size(11.0),
                    TextColor(Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(WIDTH + 2.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                ))
                .id();
            let tip = root
                .spawn((
                    abs(WIDTH * 0.5 - 1.5, HEIGHT - TIP, 3.0, TIP),
                    BackgroundColor(INK),
                ))
                .id();
            ids.push(strength);
            ids.push(morale);
            ids.push(badges);
            ids.push(tip);
        })
        .id();
    // Children were spawned in the order pushed above: icon, count, star, plaque, strength fill,
    // morale fill, strength, morale, badges, tip.
    Plate {
        root,
        icon: ids[0],
        count: ids[1],
        star: ids[2],
        plaque: ids[3],
        strength: ids[4],
        morale: ids[5],
        badges: ids[8],
        tip: ids[9],
    }
}

#[allow(clippy::too_many_arguments)]
fn update_markers(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut markers: ResMut<Markers>,
    mut stats: ResMut<MarkerStats>,
    frames: Res<PoseFrames>,
    selection: Option<Res<BattleSelection>>,
    camera_state: Option<Res<BattleCameraState>>,
    theme: Option<Res<UiTheme>>,
    assets: Res<AssetServer>,
    camera: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
    mut nodes: Query<(&mut Node, &mut Visibility, &mut UiTransform)>,
    mut backgrounds: Query<&mut BackgroundColor>,
    mut borders: Query<&mut BorderColor>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
) {
    let Some(theme) = theme else {
        return;
    };
    let Some((camera, camera_transform)) = camera.iter().next() else {
        return;
    };
    let distance = camera_state.map_or(150.0, |c| c.distance);
    let banner = banner_scale(distance);
    let marker_scale = scale_for_distance(distance, SHRINK_START_M, SHRINK_END_M, MIN_SCALE);
    markers.clustered = distance
        > if markers.clustered {
            CLUSTER_OFF
        } else {
            CLUSTER_ON
        };
    let colors = session.colors;
    let selected: Vec<u32> = selection.map(|s| s.units.clone()).unwrap_or_default();
    let sim = &*session.sim_mut();
    // Entries in unit order (stable): regiments whose top is in front of the camera.
    let mut entries: Vec<Entry> = Vec::new();
    let mut by_id = HashMap::new();
    for unit in drawn_units(sim) {
        let Some(poses) = frames.cur.get(&unit.id) else {
            continue;
        };
        let ground = poses.first().map_or(0.0, |p| p[1]);
        let top = Vec3::new(
            unit.x as f32,
            ground + (BANNER_HEIGHT + 0.6) * banner,
            unit.z as f32,
        );
        let Ok(anchor) = camera.world_to_viewport(camera_transform, top) else {
            continue;
        };
        by_id.insert(unit.id, unit);
        entries.push(Entry::single(
            unit.id,
            unit.side,
            anchor,
            unit.soldiers() as f32,
            unit.initial_soldiers as f32,
            (unit.morale / 100.0).clamp(0.0, 1.0) as f32,
        ));
    }
    stats.on_screen = entries.len();
    let shown = if markers.clustered {
        cluster_entries(&entries)
    } else {
        entries
    };
    let anchors: Vec<Vec2> = shown.iter().map(|e| e.anchor).collect();
    let placed = layout(&anchors, marker_scale);
    // Create the plates that are missing.
    for entry in &shown {
        if let MapEntry::Vacant(slot) = markers.plates.entry(entry.id) {
            let unit = by_id[&entry.id];
            let kind = super::library::render_kind(unit).unwrap_or("infantry");
            let category = format!("unit_category_{kind}");
            let icon_name = if theme.icons.has(&category) {
                category
            } else {
                unit.unit_type.to_string()
            };
            let plate = spawn_plate(&mut commands, &theme, &assets, entry.id, &icon_name);
            slot.insert(plate);
        }
    }
    let live: Vec<u32> = shown.iter().map(|e| e.id).collect();
    let mut displaced = 0;
    let mut clusters = 0;
    let mut plates = 0;
    for (index, rect, was_displaced) in &placed {
        let entry = &shown[*index];
        let Some(plate) = markers.plates.get(&entry.id) else {
            continue;
        };
        let unit = by_id[&entry.id];
        let group = entry.members.len() > 1;
        if group {
            clusters += 1;
        } else {
            plates += 1;
        }
        if *was_displaced {
            displaced += 1;
        }
        let side_color = {
            let [r, g, b] = colors[usize::from(entry.side == SideId::Defender)];
            Color::srgb(r, g, b)
        };
        let routing = !group && unit.state == UnitState::Routing;
        let frame = if routing { ROUT_RED } else { side_color };
        let is_selected = entry.members.iter().any(|m| selected.contains(m));
        // Root: scaled about its own centre, placed so that its tip sits on the rect's bottom.
        let full = Vec2::new(WIDTH + BADGE_W, HEIGHT);
        let tip_x = rect.min.x + WIDTH * marker_scale * 0.5;
        let centre = full * 0.5;
        if let Ok((mut node, mut visibility, mut transform)) = nodes.get_mut(plate.root) {
            node.left = Val::Px(tip_x - centre.x - (WIDTH * 0.5 - centre.x) * marker_scale);
            node.top = Val::Px(rect.max.y - centre.y - (full.y - centre.y) * marker_scale);
            *visibility = Visibility::Inherited;
            transform.scale = Vec2::splat(marker_scale);
        }
        if let Ok(mut border) = borders.get_mut(plate.plaque) {
            *border = BorderColor::all(if is_selected { GOLD } else { frame });
        }
        if let Ok((mut node, mut visibility, _)) = nodes.get_mut(plate.plaque) {
            node.border = UiRect::all(Val::Px(if is_selected { 3.0 } else { 2.0 }));
            node.border_radius = if group {
                BorderRadius::MAX
            } else {
                BorderRadius::ZERO
            };
            *visibility = Visibility::Inherited;
        }
        if let Ok(mut background) = backgrounds.get_mut(plate.plaque) {
            *background = BackgroundColor(if group {
                side_color
            } else if routing {
                Color::srgb(0.95, 0.78, 0.72)
            } else {
                PARCHMENT
            });
        }
        if let Ok(mut background) = backgrounds.get_mut(plate.tip) {
            *background = BackgroundColor(frame);
        }
        if let Ok((_, mut visibility, _)) = nodes.get_mut(plate.icon) {
            *visibility = if group {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
        if let Ok((_, mut visibility, _)) = nodes.get_mut(plate.count) {
            *visibility = if group {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if let Ok((mut text, mut color)) = texts.get_mut(plate.count) {
            text.0 = entry.members.len().to_string();
            color.0 = Color::WHITE;
        }
        if let Ok((_, mut visibility, _)) = nodes.get_mut(plate.star) {
            *visibility = if !group && unit.is_general {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        let strength = (entry.soldiers / entry.initial.max(1.0)).clamp(0.0, 1.0);
        if let Ok((mut node, _, _)) = nodes.get_mut(plate.strength) {
            node.width = Val::Percent(strength * 100.0);
        }
        if let Ok((mut node, _, _)) = nodes.get_mut(plate.morale) {
            node.width = Val::Percent(entry.morale * 100.0);
        }
        if let Ok(mut background) = backgrounds.get_mut(plate.morale) {
            *background = BackgroundColor(morale_color(entry.morale));
        }
        if let Ok((mut text, mut color)) = texts.get_mut(plate.badges) {
            let badges = if group {
                Vec::new()
            } else {
                state_badges(unit.state)
            };
            text.0 = badges
                .iter()
                .map(|(letter, _)| *letter)
                .collect::<Vec<_>>()
                .join("\n");
            color.0 = badges.first().map_or(Color::WHITE, |(_, c)| *c);
        }
    }
    // Plates not used this frame are hidden.
    for (id, plate) in &markers.plates {
        if !live.contains(id)
            && let Ok((_, mut visibility, _)) = nodes.get_mut(plate.root)
        {
            *visibility = Visibility::Hidden;
        }
    }
    stats.plates = plates;
    stats.clusters = clusters;
    stats.displaced = displaced;
    stats.scale = marker_scale;
    stats.clustered = markers.clustered;
}

/// Left click on a plate selects its regiments (shift adds).
fn click_markers(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    roots: Query<(&MarkerRoot, &Hovered)>,
    mut writer: MessageWriter<SelectBattleUnits>,
    mut stats: ResMut<MarkerStats>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some((marker, _)) = roots.iter().find(|(_, hovered)| hovered.get()) else {
        return;
    };
    let additive = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    writer.write(SelectBattleUnits {
        units: vec![marker.0],
        additive,
    });
    stats.clicks += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u32, side: SideId, x: f32, y: f32) -> Entry {
        Entry::single(id, side, Vec2::new(x, y), 100.0, 120.0, 0.8)
    }

    #[test]
    fn scale_shrinks_between_start_and_end() {
        assert_eq!(scale_for_distance(100.0, 260.0, 700.0, 0.5), 1.0);
        assert!((scale_for_distance(480.0, 260.0, 700.0, 0.5) - 0.75).abs() < 1e-6);
        assert_eq!(scale_for_distance(2000.0, 260.0, 700.0, 0.5), 0.5);
    }

    #[test]
    fn clusters_merge_close_regiments_of_one_side_only() {
        let entries = [
            entry(1, SideId::Attacker, 100.0, 100.0),
            entry(2, SideId::Attacker, 130.0, 110.0),
            entry(3, SideId::Defender, 110.0, 100.0),
            entry(4, SideId::Attacker, 600.0, 100.0),
        ];
        let groups = cluster_entries(&entries);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].members, vec![1, 2]);
        assert_eq!(groups[0].id, 1);
        assert_eq!(groups[0].soldiers, 200.0);
        assert_eq!(groups[0].anchor, Vec2::new(115.0, 105.0));
        assert_eq!(groups[1].members, vec![3]);
    }

    #[test]
    fn overlapping_markers_are_pushed_aside_nearest_first() {
        let anchors = [Vec2::new(300.0, 200.0), Vec2::new(300.0, 400.0)];
        let placed = layout(&anchors, 1.0);
        assert_eq!(placed[0].0, 1, "the lowest on screen is placed first");
        assert!(placed.iter().all(|p| !p.2), "far apart: nobody moves");
        let close = [Vec2::new(300.0, 400.0), Vec2::new(302.0, 400.0)];
        let placed = layout(&close, 1.0);
        assert!(!placed[0].2);
        assert!(placed[1].2, "second one displaced");
        assert!(!overlaps(placed[1].1, &[placed[0].1]));
    }

    #[test]
    fn crowds_stack_upwards() {
        let anchors = vec![Vec2::new(500.0, 500.0); 12];
        let placed = layout(&anchors, 1.0);
        let distinct = placed
            .iter()
            .map(|p| (p.1.min.x as i32, p.1.min.y as i32))
            .collect::<std::collections::HashSet<_>>();
        assert!(distinct.len() >= 9, "{}", distinct.len());
    }

    #[test]
    fn badges_follow_state() {
        assert_eq!(state_badges(UnitState::Routing)[0].0, "D");
        assert!(state_badges(UnitState::Idle).is_empty());
    }
}
