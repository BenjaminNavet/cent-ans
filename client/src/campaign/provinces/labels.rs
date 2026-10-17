//! Province (region) names and the screen-space label placer. Ports game/scripts/map/
//! region_labels.gd (discreet French names in the medium view, biggest provinces first, never cut
//! by the screen edge, never touching a town name or another region name) and label_placer.gd
//! (army plate placement around obstacles: pure rectangle logic, reusable by the armies lot).

use std::collections::HashMap;
use std::path::Path;

use crate::ui::kit::viewport_to_ui;
use bevy::prelude::*;
use bevy::ui::UiScale;
use serde_json::Value;

use super::borders::smoothstep;
use super::data::ProvinceMap;
use super::modes::{hex, num};
use crate::app::AppState;
use crate::ui::kit::UiCamera;

/// Region font (IM Fell English Italic), relative to `game/assets`.
const FONT_PATH: &str = "third_party/fonts/im_fell_english/IMFeENit28P.ttf";

// ---- region labels ---------------------------------------------------------------------------

/// `region_labels` block of `data/ui/campaign_map.json`.
#[derive(Resource, Clone, Debug)]
pub struct RegionLabelTuning {
    pub min_distance: f32,
    pub max_distance: f32,
    pub fade: f32,
    pub font_size: f32,
    pub letter_spacing: f32,
    pub color: Srgba,
    pub alpha: f32,
    pub edge_margin: f32,
    pub max_on_screen: usize,
}

impl Default for RegionLabelTuning {
    fn default() -> Self {
        Self::from_json(&Value::Null)
    }
}

impl RegionLabelTuning {
    pub fn from_json(block: &Value) -> Self {
        Self {
            min_distance: num(block, &["min_distance"], 180.0),
            max_distance: num(block, &["max_distance"], 1250.0),
            fade: num(block, &["fade"], 60.0).max(1.0),
            font_size: num(block, &["font_size_px"], 17.0),
            letter_spacing: num(block, &["letter_spacing_px"], 2.0),
            color: block
                .get("color")
                .and_then(Value::as_str)
                .and_then(hex)
                .unwrap_or(Srgba::new(0.18, 0.13, 0.08, 1.0)),
            alpha: num(block, &["alpha"], 0.5),
            edge_margin: num(block, &["edge_margin_px"], 12.0),
            max_on_screen: num(block, &["max_on_screen"], 14.0) as usize,
        }
    }

    pub fn load(root: &Path) -> Self {
        let block = std::fs::read(root.join("data/ui/campaign_map.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v.get("region_labels").cloned())
            .unwrap_or(Value::Null);
        Self::from_json(&block)
    }

    /// Opacity of the names at camera `distance` (0 outside the medium view).
    pub fn weight_at(&self, distance: f32) -> f32 {
        smoothstep(self.min_distance, self.min_distance + self.fade, distance)
            * (1.0 - smoothstep(self.max_distance - self.fade, self.max_distance, distance))
    }
}

/// One name candidate: text and screen position of its anchor.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelCandidate {
    pub name: String,
    pub screen: Vec2,
}

/// A name that fits: its text and rectangle on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedLabel {
    pub name: String,
    pub rect: Rect,
}

/// Rough text extent (the UI has no font metrics here): `0.5 em` per glyph plus letter spacing.
pub fn text_extent(name: &str, size: f32, spacing: f32) -> Vec2 {
    let glyphs = name.chars().count() as f32;
    Vec2::new(glyphs * (size * 0.5 + spacing), size * 1.2)
}

/// Names retained for the screen, candidates ordered by priority (biggest province first): inside
/// `safe`, entirely, not touching `obstacles` (grown by 4 px) nor a name already placed (grown by
/// 6 px), at most `limit`.
pub fn layout_labels(
    candidates: &[LabelCandidate],
    safe: Rect,
    obstacles: &[Rect],
    limit: usize,
    size: f32,
    spacing: f32,
) -> Vec<PlacedLabel> {
    let mut placed = Vec::new();
    if safe.width() <= 0.0 || safe.height() <= 0.0 {
        return placed;
    }
    let mut taken: Vec<Rect> = obstacles.iter().map(|r| inflate(*r, 4.0)).collect();
    for candidate in candidates {
        if placed.len() >= limit {
            break;
        }
        if !safe.contains(candidate.screen) {
            continue;
        }
        let extent = text_extent(&candidate.name, size, spacing);
        let rect = Rect::from_center_size(candidate.screen, extent);
        if !(safe.min.x <= rect.min.x
            && safe.min.y <= rect.min.y
            && rect.max.x <= safe.max.x
            && rect.max.y <= safe.max.y)
        {
            continue; // never a name cut by the screen edge
        }
        if taken.iter().any(|other| !other.intersect(rect).is_empty()) {
            continue;
        }
        taken.push(inflate(rect, 6.0));
        placed.push(PlacedLabel {
            name: candidate.name.clone(),
            rect,
        });
    }
    placed
}

fn inflate(rect: Rect, by: f32) -> Rect {
    Rect {
        min: rect.min - Vec2::splat(by),
        max: rect.max + Vec2::splat(by),
    }
}

/// Screen rectangles to keep clear of region names (town names shown by the markers lot) and the
/// height of the HUD strip at the top; written by other lots, read here every frame.
#[derive(Resource, Default, Clone)]
pub struct RegionLabelObstacles {
    pub rects: Vec<Rect>,
    pub top_inset: f32,
}

/// What the last pass drew, for tests and the dump.
#[derive(Resource, Default, Clone, Debug)]
pub struct RegionLabelStats {
    pub weight: f32,
    pub names: Vec<String>,
    /// Screen rectangles of the names on screen (river names keep clear of them).
    pub rects: Vec<Rect>,
}

#[derive(Component)]
struct RegionLabelSlot(usize);

pub(super) fn build(app: &mut App) {
    app.init_resource::<RegionLabelObstacles>()
        .init_resource::<RegionLabelStats>()
        .init_resource::<RegionLabelTuning>()
        .add_systems(OnEnter(AppState::Campaign), spawn_pool)
        .add_systems(Update, update_labels.run_if(resource_exists::<ProvinceMap>));
}

fn spawn_pool(
    mut commands: Commands,
    theme: Option<Res<crate::ui::kit::UiTheme>>,
    mut fonts: ResMut<Assets<Font>>,
    mut tuning: ResMut<RegionLabelTuning>,
) {
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    *tuning = RegionLabelTuning::load(&root);
    // UI theme italic title font (IM Fell English Italic with the accent / symbol fallbacks).
    let font: FontSource = match theme {
        Some(theme) => theme.font_title_italic.clone(),
        None => std::fs::read(crate::paths::asset_root(&root).join(FONT_PATH))
            .map(|bytes| fonts.add(Font::from_bytes(bytes)))
            .unwrap_or_default()
            .into(),
    };
    for slot in 0..tuning.max_on_screen {
        commands.spawn((
            Name::new(format!("RegionLabel{slot}")),
            RegionLabelSlot(slot),
            DespawnOnExit(AppState::Campaign),
            Text::new(""),
            TextFont {
                font: font.clone(),
                font_size: tuning.font_size.into(),
                ..default()
            },
            TextColor(Color::NONE),
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

/// Distance from the camera to the ground point at the centre of the view (world units).
pub fn camera_ground_distance(camera: &Camera, transform: &GlobalTransform) -> Option<f32> {
    let size = camera.logical_viewport_size()?;
    let ray = camera.viewport_to_world(transform, size * 0.5).ok()?;
    ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
}

#[allow(clippy::too_many_arguments)]
fn update_labels(
    map: Res<ProvinceMap>,
    tuning: Res<RegionLabelTuning>,
    obstacles: Res<RegionLabelObstacles>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ground: crate::campaign::terrain::surface::GroundHeight,
    ui_scale: Res<UiScale>,
    mut stats: ResMut<RegionLabelStats>,
    mut slots: Query<
        (
            &RegionLabelSlot,
            &mut Text,
            &mut Node,
            &mut TextColor,
            &mut Visibility,
        ),
        Without<UiCamera>,
    >,
    mut order: Local<Vec<u16>>,
) {
    stats.names.clear();
    stats.rects.clear();
    stats.weight = 0.0;
    let hide = |slots: &mut Query<
        (
            &RegionLabelSlot,
            &mut Text,
            &mut Node,
            &mut TextColor,
            &mut Visibility,
        ),
        Without<UiCamera>,
    >| {
        for (_, _, _, _, mut visibility) in slots.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    };
    let Some((camera, transform)) = cameras.iter().find(|(c, _)| c.is_active) else {
        return hide(&mut slots);
    };
    let Some(distance) = camera_ground_distance(camera, transform) else {
        return hide(&mut slots);
    };
    let weight = tuning.weight_at(distance);
    stats.weight = weight;
    let Some(viewport) = camera
        .logical_viewport_size()
        .map(|v| viewport_to_ui(v, &ui_scale))
        .filter(|_| weight > 0.01)
    else {
        return hide(&mut slots);
    };
    if order.is_empty() {
        let mut by_area: Vec<_> = map.provinces().iter().collect();
        by_area.sort_by(|a, b| b.area_px.total_cmp(&a.area_px).then(a.index.cmp(&b.index)));
        order.extend(by_area.iter().map(|p| p.index as u16));
    }
    let candidates: Vec<LabelCandidate> = order
        .iter()
        .filter_map(|index| {
            let province = map.province(*index)?;
            let (cx, cz) = (province.centroid[0], province.centroid[1]);
            let world = Vec3::new(cx, ground.at(cx, cz).unwrap_or(0.0), cz);
            let screen =
                viewport_to_ui(camera.world_to_viewport(transform, world).ok()?, &ui_scale);
            Some(LabelCandidate {
                name: province.name.clone(),
                screen,
            })
        })
        .collect();
    let margin = tuning.edge_margin;
    let safe = Rect {
        min: Vec2::new(margin, margin + obstacles.top_inset),
        max: viewport - Vec2::splat(margin),
    };
    let placed = layout_labels(
        &candidates,
        safe,
        &obstacles.rects,
        tuning.max_on_screen,
        tuning.font_size,
        tuning.letter_spacing,
    );
    let mut ink = tuning.color;
    ink.alpha = tuning.alpha * weight;
    let by_slot: HashMap<usize, &PlacedLabel> = placed.iter().enumerate().collect();
    for (slot, mut text, mut node, mut color, mut visibility) in &mut slots {
        match by_slot.get(&slot.0) {
            Some(label) => {
                if text.0 != label.name {
                    text.0.clone_from(&label.name);
                }
                node.left = Val::Px(label.rect.min.x);
                node.top = Val::Px(label.rect.min.y);
                color.0 = ink.into();
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    stats.rects = placed.iter().map(|p| p.rect).collect();
    stats.names = placed.into_iter().map(|p| p.name).collect();
}

// ---- LabelPlacer (army plates) -------------------------------------------------------------

const CELL: f32 = 64.0;
/// Gap between a moved plate and the position it leaves.
const GAP: f32 = 3.0;
/// Lateral offset on top of the half width (the banner is in the middle of the base).
const SIDE_CLEARANCE: f32 = 14.0;
/// Margin required around the base before returning to it.
const HYSTERESIS_PX: f32 = 6.0;

/// Candidate offsets (from the base) for a plate of `size`, by preference: base, above, right,
/// left, two steps above, below, then diagonals and farther sides.
pub fn candidate_offsets(size: Vec2) -> [Vec2; 10] {
    let up = size.y + GAP;
    let side = size.x * 0.5 + SIDE_CLEARANCE;
    [
        Vec2::ZERO,
        Vec2::new(0.0, -up),
        Vec2::new(side, up * 0.5),
        Vec2::new(-side, up * 0.5),
        Vec2::new(0.0, -2.0 * up),
        Vec2::new(0.0, up),
        Vec2::new(side, -up),
        Vec2::new(-side, -up),
        Vec2::new(side * 1.8, up * 0.5),
        Vec2::new(-side * 1.8, up * 0.5),
    ]
}

/// Plate item: `id` and base rectangle.
#[derive(Debug, Clone)]
pub struct PlateItem {
    pub id: String,
    pub rect: Rect,
}

/// Places army plates out of the obstacles and of each other. A moved plate keeps its fallback
/// position while it stays free and only returns to its base when the base, grown by
/// [`HYSTERESIS_PX`], is free (no flicker).
#[derive(Default)]
pub struct LabelPlacer {
    choice: HashMap<String, usize>,
}

impl LabelPlacer {
    /// `items` by priority (the first keep their base), `obstacles` screen rectangles.
    /// Returns id -> screen offset to add to the base position.
    pub fn place(&mut self, items: &[PlateItem], obstacles: &[Rect]) -> HashMap<String, Vec2> {
        let mut grid = SpatialGrid::default();
        for rect in obstacles {
            grid.insert(*rect);
        }
        let mut offsets = HashMap::new();
        let mut seen = std::collections::HashSet::new();
        for item in items {
            let candidates = candidate_offsets(item.rect.size());
            let previous = self.choice.get(&item.id).copied().unwrap_or(0);
            let shifted = |i: usize| {
                Rect::from_corners(item.rect.min + candidates[i], item.rect.max + candidates[i])
            };
            let mut chosen = None;
            if previous != 0 && previous < candidates.len() {
                if !grid.hits(inflate(item.rect, HYSTERESIS_PX)) {
                    chosen = Some(0);
                } else if !grid.hits(shifted(previous)) {
                    chosen = Some(previous);
                }
            }
            let chosen = chosen.unwrap_or_else(|| first_free(&grid, item, &candidates, previous));
            self.choice.insert(item.id.clone(), chosen);
            seen.insert(item.id.clone());
            offsets.insert(item.id.clone(), candidates[chosen]);
            grid.insert(shifted(chosen));
        }
        self.choice.retain(|id, _| seen.contains(id));
        offsets
    }

    /// Candidate kept for `id` at the last placement (0 = base).
    pub fn choice_of(&self, id: &str) -> Option<usize> {
        self.choice.get(id).copied()
    }

    pub fn reset(&mut self) {
        self.choice.clear();
    }
}

/// First free candidate; failing that the one that overlaps least (the previous one on a tie).
fn first_free(
    grid: &SpatialGrid,
    item: &PlateItem,
    candidates: &[Vec2; 10],
    previous: usize,
) -> usize {
    let mut best = 0;
    let mut best_overlap = f32::INFINITY;
    for (i, offset) in candidates.iter().enumerate() {
        let rect = Rect::from_corners(item.rect.min + *offset, item.rect.max + *offset);
        let overlap = grid.overlap_area(rect);
        if overlap <= 0.0 {
            return i;
        }
        if overlap < best_overlap - 0.5 || ((overlap - best_overlap).abs() <= 0.5 && i == previous)
        {
            best_overlap = overlap;
            best = i;
        }
    }
    best
}

/// Rectangles in square cells of [`CELL`] px. A rectangle covering more than [`BIG_CELLS`] cells
/// (e.g. the top-bar strip `armies::markers` passes as `(-1e4, -1e4)..(1e5, inset)`, ~270 000
/// cells) is kept in a short linear list instead of being hashed into every cell: indexing it was
/// the 30 ms/frame cost of `update_plates`.
#[derive(Default)]
struct SpatialGrid {
    rects: Vec<Rect>,
    cells: HashMap<(i32, i32), Vec<usize>>,
    big: Vec<usize>,
}

/// Rectangles over this many cells are not indexed per cell.
const BIG_CELLS: i64 = 256;

impl SpatialGrid {
    fn bounds(rect: Rect) -> (i32, i32, i32, i32) {
        (
            (rect.min.x / CELL).floor() as i32,
            (rect.min.y / CELL).floor() as i32,
            (rect.max.x / CELL).floor() as i32,
            (rect.max.y / CELL).floor() as i32,
        )
    }

    fn keys(rect: Rect) -> impl Iterator<Item = (i32, i32)> {
        let (x0, y0, x1, y1) = Self::bounds(rect);
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| (x, y)))
    }

    fn insert(&mut self, rect: Rect) {
        let index = self.rects.len();
        self.rects.push(rect);
        let (x0, y0, x1, y1) = Self::bounds(rect);
        let cells = i64::from(x1 - x0 + 1) * i64::from(y1 - y0 + 1);
        if cells > BIG_CELLS {
            self.big.push(index);
            return;
        }
        for key in Self::keys(rect) {
            self.cells.entry(key).or_default().push(index);
        }
    }

    /// Indices of the rectangles that may touch `rect`, ascending and unique.
    fn candidates(&self, rect: Rect) -> Vec<usize> {
        let mut found: Vec<usize> = Self::keys(rect)
            .filter_map(|k| self.cells.get(&k))
            .flatten()
            .chain(self.big.iter())
            .copied()
            .collect();
        found.sort_unstable();
        found.dedup();
        found
    }

    fn hits(&self, rect: Rect) -> bool {
        self.candidates(rect)
            .into_iter()
            .any(|i| !self.rects[i].intersect(rect).is_empty())
    }

    fn overlap_area(&self, rect: Rect) -> f32 {
        self.candidates(rect)
            .into_iter()
            .map(|i| {
                let overlap = self.rects[i].intersect(rect);
                if overlap.is_empty() {
                    0.0
                } else {
                    overlap.width() * overlap.height()
                }
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_corners(Vec2::new(x, y), Vec2::new(x + w, y + h))
    }

    #[test]
    fn weight_is_zero_outside_the_medium_view() {
        let t = RegionLabelTuning::default();
        assert_eq!(t.weight_at(50.0), 0.0);
        assert_eq!(t.weight_at(5000.0), 0.0);
        assert!((t.weight_at(600.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn labels_stay_whole_and_apart() {
        let safe = rect(12.0, 12.0, 800.0, 500.0);
        let candidates = vec![
            LabelCandidate {
                name: "Normandie".into(),
                screen: Vec2::new(300.0, 200.0),
            },
            LabelCandidate {
                name: "Bretagne".into(),
                screen: Vec2::new(310.0, 205.0),
            }, // overlaps the first
            LabelCandidate {
                name: "Anjou".into(),
                screen: Vec2::new(5.0, 200.0),
            }, // outside
            LabelCandidate {
                name: "Champagne".into(),
                screen: Vec2::new(795.0, 300.0),
            }, // cut by the edge
            LabelCandidate {
                name: "Auvergne".into(),
                screen: Vec2::new(500.0, 400.0),
            },
        ];
        let placed = layout_labels(&candidates, safe, &[], 14, 17.0, 2.0);
        let names: Vec<&str> = placed.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Normandie", "Auvergne"]);
        // an obstacle (a town name) removes one more
        let blocked = layout_labels(
            &candidates,
            safe,
            &[rect(480.0, 390.0, 60.0, 20.0)],
            14,
            17.0,
            2.0,
        );
        assert_eq!(blocked.len(), 1);
        // the limit caps the count
        assert_eq!(layout_labels(&candidates, safe, &[], 1, 17.0, 2.0).len(), 1);
    }

    #[test]
    fn placer_keeps_base_when_free_and_moves_when_blocked() {
        let mut placer = LabelPlacer::default();
        let items = vec![PlateItem {
            id: "a".into(),
            rect: rect(100.0, 100.0, 40.0, 20.0),
        }];
        assert_eq!(placer.place(&items, &[])["a"], Vec2::ZERO);
        let blocker = rect(95.0, 95.0, 60.0, 30.0);
        let moved = placer.place(&items, &[blocker])["a"];
        assert_ne!(moved, Vec2::ZERO);
        assert!(placer.choice_of("a").unwrap() > 0);
        // hysteresis: it stays moved while the blocker is just out of the base but within 6 px
        let near = rect(100.0 - 25.0 + 40.0 + 3.0, 95.0, 20.0, 30.0);
        placer.place(&items, &[near]);
        assert!(placer.choice_of("a").unwrap() > 0);
        // a plate that disappeared is forgotten
        placer.place(&[], &[]);
        assert_eq!(placer.choice_of("a"), None);
    }

    #[test]
    fn two_plates_do_not_overlap() {
        let mut placer = LabelPlacer::default();
        let items = vec![
            PlateItem {
                id: "a".into(),
                rect: rect(100.0, 100.0, 40.0, 20.0),
            },
            PlateItem {
                id: "b".into(),
                rect: rect(105.0, 102.0, 40.0, 20.0),
            },
        ];
        let offsets = placer.place(&items, &[]);
        let place = |i: usize, id: &str| {
            Rect::from_corners(
                items[i].rect.min + offsets[id],
                items[i].rect.max + offsets[id],
            )
        };
        assert!(place(0, "a").intersect(place(1, "b")).is_empty());
    }

    #[test]
    fn huge_obstacle_is_not_indexed_per_cell_but_still_blocks() {
        let mut grid = SpatialGrid::default();
        // The top-bar strip armies pass: ~270 000 cells if hashed.
        grid.insert(Rect::new(-1.0e4, -1.0e4, 1.0e5, 48.0));
        grid.insert(rect(300.0, 300.0, 100.0, 40.0));
        assert!(grid.cells.len() < 64, "{} cells hashed", grid.cells.len());
        assert!(grid.hits(rect(500.0, 10.0, 80.0, 30.0)));
        assert!(!grid.hits(rect(500.0, 100.0, 80.0, 30.0)));
        assert!(grid.hits(rect(350.0, 320.0, 10.0, 10.0)));
        assert!(grid.overlap_area(rect(500.0, 10.0, 10.0, 10.0)) > 99.0);
    }
}
