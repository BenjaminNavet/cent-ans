//! Geometry of the battle HUD (layout by calculation, no screenshot).
//!
//! Every bottom-anchored piece of the HUD takes its offsets from the `BAND_*` constants below, and
//! the unit test checks on a range of window sizes that the pieces that must not cover each other
//! (card band, leader seal, orders bar, formation button, minimap, speed column) do not. The
//! `hud_layout` dump section lists the real rectangles measured after layout, with the overlaps
//! found, so the same check runs on the laid-out UI (`layout_probe` style).

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use serde_json::{Value, json};

/// Bottom margin of the card band.
pub const BAND_BOTTOM: f32 = 6.0;
/// Height of the card band (panel padding + title row + card + ability row). The dump reports the
/// measured height next to it (`band_height_ok`); raise it if the cards grow.
pub const BAND_HEIGHT: f32 = 192.0;
/// Free space kept between the band and what sits above it.
pub const BAND_GAP: f32 = 6.0;
/// Bottom offset of anything that sits above the card band.
pub const ABOVE_BAND_BOTTOM: f32 = BAND_BOTTOM + BAND_HEIGHT + BAND_GAP;

/// Axis-aligned rectangle in logical px, y downward.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box2 {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Box2 {
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
    use super::cards::BAND_RIGHT_MARGIN;
    use super::minimap::MAP_SIZE;
    use super::orders_bar::{BAND_LEFT_MARGIN, ORDERS_BAR_SIZE, SEAL_SIZE};
    use super::speed::{SPEED_COLUMN_HEIGHT, SPEED_PANEL_WIDTH};
    let band_width = (window.x - BAND_LEFT_MARGIN - BAND_RIGHT_MARGIN).max(0.0);
    vec![
        (
            "band",
            from_left_bottom(
                window,
                BAND_LEFT_MARGIN,
                BAND_BOTTOM,
                band_width,
                BAND_HEIGHT,
            ),
        ),
        (
            "seal",
            from_left_bottom(window, 8.0, BAND_BOTTOM, SEAL_SIZE, SEAL_SIZE),
        ),
        (
            "orders_bar",
            from_left_bottom(
                window,
                8.0,
                ABOVE_BAND_BOTTOM,
                ORDERS_BAR_SIZE.x,
                ORDERS_BAR_SIZE.y,
            ),
        ),
        (
            "formation_button",
            from_left_bottom(
                window,
                8.0,
                super::formation_menu::BUTTON_BOTTOM,
                super::formation_menu::BUTTON_W,
                super::formation_menu::BUTTON_H,
            ),
        ),
        (
            "formation_menu",
            from_left_bottom(
                window,
                8.0,
                super::formation_menu::MENU_BOTTOM,
                super::formation_menu::MENU_WIDTH,
                super::formation_menu::MENU_HEIGHT,
            ),
        ),
        (
            "minimap",
            from_right_bottom(
                window,
                super::minimap::MAP_RIGHT,
                BAND_BOTTOM,
                MAP_SIZE.x,
                MAP_SIZE.y,
            ),
        ),
        (
            "speed",
            from_right_bottom(
                window,
                8.0,
                BAND_BOTTOM,
                SPEED_PANEL_WIDTH,
                SPEED_COLUMN_HEIGHT,
            ),
        ),
    ]
}

/// Pairs of pieces that must never overlap.
pub const FORBIDDEN: [(&str, &str); 10] = [
    ("band", "formation_menu"),
    ("band", "seal"),
    ("band", "orders_bar"),
    ("band", "formation_button"),
    ("band", "minimap"),
    ("band", "speed"),
    ("seal", "orders_bar"),
    ("seal", "formation_button"),
    ("orders_bar", "formation_button"),
    ("minimap", "speed"),
];

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

/// Dump section: measured rectangles of the pieces (by node name), overlaps among them, and
/// whether the measured band fits [`BAND_HEIGHT`].
pub fn dump(world: &World) -> Value {
    const NAMES: [(&str, &str); 7] = [
        ("FormationMenu", "formation_menu"),
        ("BandPanel", "band"),
        ("LeaderSeal", "seal"),
        ("LeaderOrdersBar", "orders_bar"),
        ("formation_menu", "formation_button"),
        ("Minimap", "minimap"),
        ("SpeedControls", "speed"),
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
        "band_height": band_height,
        "band_height_ok": band_height.is_none_or(|h| h <= BAND_HEIGHT + 0.5),
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
