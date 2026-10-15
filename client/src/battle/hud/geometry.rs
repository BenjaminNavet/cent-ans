//! Geometry of the battle HUD (layout by calculation, no screenshot).
//!
//! Layout (uibattle, after battle_hud.gd): one bottom band (leader seal, regiment cards grouped
//! by « bataille », order icons and « Retraite générale ») and, in the bottom right corner, one
//! panel with the minimap and the speed column; the leader's orders bar sits above the band on
//! the left; the journal and the alerts stack in a column at the top left, under the title bar.
//! Pieces built by different plugins are parented to the [`HudSlot`] nodes the HUD root creates,
//! so they follow the band instead of each guessing absolute offsets.
//!
//! Every bottom-anchored piece takes its offsets from the constants below, and the unit test
//! checks on a range of window sizes that the pieces that must not cover each other do not. The
//! `hud_layout` dump section lists the real rectangles measured after layout, with the overlaps
//! found, so the same check runs on the laid-out UI (`layout_probe` style).

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use serde_json::{Value, json};

/// Bottom margin of the card band and of the corner panel.
pub const BAND_BOTTOM: f32 = 6.0;
/// Height of the card band and of the corner panel (Godot `BAND_HEIGHT`, 140 px): it stays under
/// the group formations panel of the orders lot (150 px from the bottom).
pub const BAND_HEIGHT: f32 = 140.0;
/// Vertical / horizontal inner padding of the band and the corner (Godot `page_box(6)`).
pub const BAND_PAD_Y: f32 = 6.0;
pub const BAND_PAD_X: f32 = 10.0;
/// Inner height of the band (what the seal, the cards and the commands share).
pub const BAND_INNER: f32 = BAND_HEIGHT - 2.0 * BAND_PAD_Y;
/// Free space kept between the band and what sits above it.
pub const BAND_GAP: f32 = 6.0;
/// Bottom offset of anything that sits above the card band.
pub const ABOVE_BAND_BOTTOM: f32 = BAND_BOTTOM + BAND_HEIGHT + BAND_GAP;
/// Left margin of the band (the seal is inside it).
pub const BAND_LEFT: f32 = 8.0;
/// Right margin of the corner panel.
pub const CORNER_RIGHT: f32 = 8.0;
/// Gap between the speed column and the minimap inside the corner.
pub const CORNER_GAP: f32 = 6.0;
/// Width of the corner panel (minimap + speed column + padding).
pub const CORNER_WIDTH: f32 =
    2.0 * BAND_PAD_X + super::minimap::MAP_SIZE.x + CORNER_GAP + super::speed::SPEED_PANEL_WIDTH;
/// Right margin of the band: the corner panel plus a gap.
pub const BAND_RIGHT_MARGIN: f32 = CORNER_RIGHT + CORNER_WIDTH + 6.0;
/// Top of the left column (journal, alerts), under the title bar.
pub const LEFT_STACK_TOP: f32 = 66.0;
/// Width of the left column.
pub const LEFT_STACK_WIDTH: f32 = 320.0;

// Checked at compile time: the band stays under the group formations panel of the orders lot
// (battle/orders/formations.rs, 150 px above the bottom), and what it holds fits its height.
const _: () = assert!(BAND_BOTTOM + BAND_HEIGHT <= 150.0);
const _: () = assert!(super::orders_bar::SEAL_SIZE <= BAND_INNER);
const _: () = assert!(super::speed::SPEED_COLUMN_HEIGHT <= BAND_INNER + 0.5);
const _: () = assert!(super::minimap::MAP_SIZE.y <= BAND_INNER + 0.5);
const _: () = assert!(
    super::cards::GROUP_CAPTION_H
        + 1.0
        + super::cards::CARD_HEIGHT
        + 1.0
        + super::abilities::BUTTON_SIZE
        + 2.0
        <= BAND_INNER + 0.5
);

/// Places the HUD root offers to the pieces built by other plugins of this folder.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudSlot {
    /// Leader seal, at the left end of the band.
    Seal,
    /// First cell of the command grid (the « Formation » button of `formation_menu`).
    Formation,
    /// Minimap, in the corner panel.
    Map,
    /// Journal, top of the left column.
    Journal,
    /// Alerts, under the journal.
    Alerts,
}

/// Axis-aligned rectangle in logical px, y downward.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box2 {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Box2 {
    /// `other` lies inside `self` (1 px tolerance).
    pub fn contains(&self, other: &Box2) -> bool {
        other.left >= self.left - 1.0
            && other.right <= self.right + 1.0
            && other.top >= self.top - 1.0
            && other.bottom <= self.bottom + 1.0
    }

    pub fn overlaps(&self, other: &Box2) -> bool {
        self.left < other.right - 0.5
            && other.left < self.right - 0.5
            && self.top < other.bottom - 0.5
            && other.top < self.bottom - 0.5
    }
}

/// Rectangle of a node anchored `left` / `bottom` (px) in a window of `window`.
pub fn from_left_bottom(window: Vec2, left: f32, bottom: f32, width: f32, height: f32) -> Box2 {
    Box2 {
        left,
        right: left + width,
        bottom: window.y - bottom,
        top: window.y - bottom - height,
    }
}

/// Rectangle of a node anchored `right` / `bottom` (px).
pub fn from_right_bottom(window: Vec2, right: f32, bottom: f32, width: f32, height: f32) -> Box2 {
    from_left_bottom(window, window.x - right - width, bottom, width, height)
}

/// The designed rectangles of the bottom HUD pieces, by name, for a window.
pub fn designed(window: Vec2) -> Vec<(&'static str, Box2)> {
    use super::commands::{CELL, GRID_WIDTH};
    use super::formation_menu::{MENU_BOTTOM, MENU_HEIGHT, MENU_RIGHT, MENU_WIDTH};
    use super::minimap::MAP_SIZE;
    use super::orders_bar::{ORDERS_BAR_SIZE, SEAL_SIZE};
    use super::speed::{SPEED_COLUMN_HEIGHT, SPEED_PANEL_WIDTH};
    let band_width = (window.x - BAND_LEFT - BAND_RIGHT_MARGIN).max(0.0);
    let inner_bottom = BAND_BOTTOM + BAND_PAD_Y;
    let grid_right = BAND_RIGHT_MARGIN + BAND_PAD_X;
    let cards_left = BAND_LEFT + BAND_PAD_X + SEAL_SIZE + super::cards::DIVIDER_SPACE;
    let cards_right = grid_right + GRID_WIDTH + super::cards::DIVIDER_SPACE;
    vec![
        (
            "band",
            from_left_bottom(window, BAND_LEFT, BAND_BOTTOM, band_width, BAND_HEIGHT),
        ),
        (
            "seal",
            from_left_bottom(
                window,
                BAND_LEFT + BAND_PAD_X,
                inner_bottom + (BAND_INNER - SEAL_SIZE) * 0.5,
                SEAL_SIZE,
                SEAL_SIZE,
            ),
        ),
        (
            "cards",
            from_left_bottom(
                window,
                cards_left,
                inner_bottom,
                (window.x - cards_left - cards_right).max(0.0),
                BAND_INNER,
            ),
        ),
        (
            "commands",
            from_right_bottom(window, grid_right, inner_bottom, GRID_WIDTH, BAND_INNER),
        ),
        (
            "orders_bar",
            from_left_bottom(
                window,
                BAND_LEFT,
                ABOVE_BAND_BOTTOM,
                ORDERS_BAR_SIZE.x,
                ORDERS_BAR_SIZE.y,
            ),
        ),
        (
            "formation_button",
            from_right_bottom(
                window,
                grid_right + GRID_WIDTH - CELL.x,
                inner_bottom + BAND_INNER - CELL.y,
                CELL.x,
                CELL.y,
            ),
        ),
        (
            "formation_menu",
            from_right_bottom(window, MENU_RIGHT, MENU_BOTTOM, MENU_WIDTH, MENU_HEIGHT),
        ),
        (
            "corner",
            from_right_bottom(window, CORNER_RIGHT, BAND_BOTTOM, CORNER_WIDTH, BAND_HEIGHT),
        ),
        (
            "minimap",
            from_right_bottom(
                window,
                CORNER_RIGHT + BAND_PAD_X + SPEED_PANEL_WIDTH + CORNER_GAP,
                BAND_BOTTOM + (BAND_HEIGHT - MAP_SIZE.y) * 0.5,
                MAP_SIZE.x,
                MAP_SIZE.y,
            ),
        ),
        (
            "speed",
            from_right_bottom(
                window,
                CORNER_RIGHT + BAND_PAD_X,
                BAND_BOTTOM + (BAND_HEIGHT - SPEED_COLUMN_HEIGHT) * 0.5,
                SPEED_PANEL_WIDTH,
                SPEED_COLUMN_HEIGHT,
            ),
        ),
    ]
}

/// Pieces that must lie inside another one: `(inner, outer)`.
pub const INSIDE: [(&str, &str); 7] = [
    ("seal", "band"),
    ("cards", "band"),
    ("commands", "band"),
    ("formation_button", "commands"),
    ("minimap", "corner"),
    ("speed", "corner"),
    ("formation_button", "band"),
];

/// Pairs of pieces that must never overlap.
pub const FORBIDDEN: [(&str, &str); 11] = [
    ("band", "formation_menu"),
    ("band", "orders_bar"),
    ("band", "corner"),
    ("corner", "formation_menu"),
    ("corner", "orders_bar"),
    ("seal", "cards"),
    ("cards", "commands"),
    ("seal", "commands"),
    ("minimap", "speed"),
    ("formation_button", "formation_menu"),
    ("orders_bar", "formation_button"),
];

/// Pairs of [`INSIDE`] whose inner piece pokes out of the outer one (1 px tolerance).
pub fn escapes(boxes: &[(&str, Box2)]) -> Vec<(String, String)> {
    let find = |name: &str| boxes.iter().find(|(n, _)| *n == name).map(|(_, b)| *b);
    INSIDE
        .iter()
        .filter(|(a, b)| {
            matches!((find(a), find(b)), (Some(inner), Some(outer)) if !outer.contains(&inner))
        })
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

/// Overlapping pairs among `boxes` (names from [`FORBIDDEN`]).
pub fn overlaps(boxes: &[(&str, Box2)]) -> Vec<(String, String)> {
    let find = |name: &str| boxes.iter().find(|(n, _)| *n == name).map(|(_, b)| *b);
    FORBIDDEN
        .iter()
        .filter(|(a, b)| matches!((find(a), find(b)), (Some(ra), Some(rb)) if ra.overlaps(&rb)))
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

fn rect_of(node: &ComputedNode, transform: &UiGlobalTransform) -> Box2 {
    let center = transform.translation * node.inverse_scale_factor();
    let half = node.size() * node.inverse_scale_factor() * 0.5;
    Box2 {
        left: center.x - half.x,
        top: center.y - half.y,
        right: center.x + half.x,
        bottom: center.y + half.y,
    }
}

/// Measured rectangles of the visible nodes whose name passes `keep`.
fn measure(world: &World, keep: impl Fn(&str) -> bool) -> Vec<(String, Box2)> {
    let mut found = Vec::new();
    for entity in world.iter_entities() {
        let Some(name) = entity.get::<Name>() else {
            continue;
        };
        if !keep(name.as_str()) {
            continue;
        }
        let (Some(node), Some(transform)) = (
            entity.get::<ComputedNode>(),
            entity.get::<UiGlobalTransform>(),
        ) else {
            continue;
        };
        if node.size().x < 1.0 {
            continue;
        }
        found.push((name.to_string(), rect_of(node, transform)));
    }
    found
}

/// Overlaps among the top pieces (title bar, siege line, journal, alerts column).
fn left_stack_overlaps(world: &World) -> Value {
    const TOP: [&str; 4] = ["TopBar", "SiegeFrame", "BattleLog", "BattleAlertsColumn"];
    let boxes = measure(world, |n| TOP.contains(&n));
    let mut pairs = Vec::new();
    for (i, (a, ra)) in boxes.iter().enumerate() {
        for (b, rb) in &boxes[i + 1..] {
            if ra.overlaps(rb) {
                pairs.push(json!([a, b]));
            }
        }
    }
    Value::Array(pairs)
}

/// Every card slot and command button lies inside the band panel (nothing clipped or poking
/// out); the names of those that do not.
fn strip_fits(world: &World) -> Value {
    let band = measure(world, |n| n == "BandPanel");
    let Some((_, band)) = band.first() else {
        return Value::Null;
    };
    let out: Vec<String> = measure(world, |n| {
        n.starts_with("cardslot:") || n.starts_with("Command_") || n == "WithdrawAll"
    })
    .into_iter()
    .filter(|(_, r)| r.top < band.top - 1.0 || r.bottom > band.bottom + 1.0)
    .map(|(n, _)| n)
    .collect();
    json!(out)
}

/// Dump section: measured rectangles of the pieces (by node name), overlaps among them, and
/// whether the measured band fits [`BAND_HEIGHT`].
pub fn dump(world: &World) -> Value {
    const NAMES: [(&str, &str); 11] = [
        ("FormationMenu", "formation_menu"),
        ("BandPanel", "band"),
        ("LeaderSeal", "seal"),
        ("CardStrip", "cards"),
        ("CommandGrid", "commands"),
        ("LeaderOrdersBar", "orders_bar"),
        ("formation_menu", "formation_button"),
        ("MapCorner", "corner"),
        ("Minimap", "minimap"),
        ("SpeedControls", "speed"),
        ("TopBar", "top_bar"),
    ];
    let mut measured: Vec<(&str, Box2)> = Vec::new();
    let mut extra = serde_json::Map::new();
    for entity in world.iter_entities() {
        let Some(name) = entity.get::<Name>() else {
            continue;
        };
        let (Some(node), Some(transform)) = (
            entity.get::<ComputedNode>(),
            entity.get::<UiGlobalTransform>(),
        ) else {
            continue;
        };
        if node.size().x < 1.0 {
            continue; // display: none
        }
        let rect = rect_of(node, transform);
        if let Some((_, key)) = NAMES.iter().find(|(n, _)| name.as_str() == *n) {
            measured.push((key, rect));
        } else if name.as_str().starts_with("speed:") {
            extra.insert(
                name.to_string(),
                json!([rect.left, rect.top, rect.right, rect.bottom]),
            );
        }
    }
    let band_height = measured
        .iter()
        .find(|(n, _)| *n == "band")
        .map(|(_, b)| b.bottom - b.top);
    let rects: serde_json::Map<String, Value> = measured
        .iter()
        .map(|(n, b)| ((*n).to_owned(), json!([b.left, b.top, b.right, b.bottom])))
        .collect();
    json!({
        "rects": rects,
        "extra": extra,
        "overlaps": overlaps(&measured),
        "escapes": escapes(&measured),
        "left_stack": left_stack_overlaps(world),
        "band_height": band_height,
        "band_height_ok": band_height.is_none_or(|h| h <= BAND_HEIGHT + 0.5),
        "strip_fits": strip_fits(world),
        "designed_band_height": BAND_HEIGHT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn designed_pieces_never_overlap() {
        for (w, h) in [
            (1024.0, 640.0),
            (1280.0, 800.0),
            (1422.0, 889.0),
            (1920.0, 1080.0),
            (2560.0, 1440.0),
        ] {
            let window = Vec2::new(w, h);
            let boxes = designed(window);
            assert!(
                overlaps(&boxes).is_empty(),
                "{w}x{h}: overlapping {:?}",
                overlaps(&boxes)
            );
            assert!(
                escapes(&boxes).is_empty(),
                "{w}x{h}: escaping {:?}",
                escapes(&boxes)
            );
        }
    }

    #[test]
    fn orders_bar_sits_above_the_band() {
        let boxes = designed(Vec2::new(1280.0, 800.0));
        let band = boxes.iter().find(|(n, _)| *n == "band").unwrap().1;
        let bar = boxes.iter().find(|(n, _)| *n == "orders_bar").unwrap().1;
        assert!(bar.bottom <= band.top - BAND_GAP + 0.5);
    }

    #[test]
    fn overlap_detector_catches_a_collision() {
        let a = Box2 {
            left: 0.0,
            top: 0.0,
            right: 10.0,
            bottom: 10.0,
        };
        let b = Box2 {
            left: 5.0,
            top: 5.0,
            right: 20.0,
            bottom: 20.0,
        };
        let c = Box2 {
            left: 10.0,
            top: 0.0,
            right: 20.0,
            bottom: 10.0,
        };
        assert!(a.overlaps(&b));
        assert!(!a.overlaps(&c));
    }
}
