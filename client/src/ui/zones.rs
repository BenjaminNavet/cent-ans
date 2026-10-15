//! Fixed screen zones (port of the rectangles of game/scripts/ui/ui_layout.gd, bible DA § 12.1).
//!
//! Panels claim a zone instead of absolute coordinates; the rectangles are proportions of the
//! window, independent of children sizes. Godot's host/claim/reparent plumbing is not ported: in
//! Bevy a screen reads [`rect`] (or spawns a [`zone_node`]) and the panel stack handles
//! exclusivity (`ui::kit::panel_stack`) and the modal queue the dim veil (`ui::kit::modal`).
//! Toast stacking (3 at most, 6 s) is `ui::kit::modal::ToastRequest`.

use bevy::prelude::*;

/// Zones of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    TopBar,
    BottomSelection,
    Minimap,
    SidePanel,
    Toasts,
    Modal,
}

impl Zone {
    pub const ALL: [Zone; 6] = [
        Zone::TopBar,
        Zone::BottomSelection,
        Zone::Minimap,
        Zone::SidePanel,
        Zone::Toasts,
        Zone::Modal,
    ];

    /// Share of the screen: `(x, y, width, height)` (`ZONE_RECTS`; A6-U11 side panel 0,33 wide).
    pub const fn share(self) -> [f32; 4] {
        match self {
            Zone::TopBar => [0.0, 0.0, 1.0, 0.08],
            Zone::BottomSelection => [0.01, 0.80, 0.80, 0.20],
            Zone::Minimap => [0.82, 0.72, 0.18, 0.28],
            Zone::SidePanel => [0.67, 0.09, 0.33, 0.61],
            Zone::Toasts => [0.01, 0.09, 0.26, 0.48],
            Zone::Modal => [0.2, 0.12, 0.6, 0.76],
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Zone::TopBar => "top_bar",
            Zone::BottomSelection => "bottom_selection",
            Zone::Minimap => "minimap",
            Zone::SidePanel => "side_panel",
            Zone::Toasts => "toasts",
            Zone::Modal => "modal",
        }
    }
}

/// Dim of modal windows (bible § 12.1: black 45 %).
pub const MODAL_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.45);

/// Rectangle of `zone` in a logical `screen` size.
pub fn rect(zone: Zone, screen: Vec2) -> Rect {
    let [x, y, w, h] = zone.share();
    Rect::from_corners(
        Vec2::new(x * screen.x, y * screen.y),
        Vec2::new((x + w) * screen.x, (y + h) * screen.y),
    )
}

/// Absolutely positioned, clipping, percent-based node of `zone` (children larger than the
/// zone are cut: they scroll or shrink, the zone never grows).
pub fn zone_node(zone: Zone) -> impl Bundle {
    let [x, y, w, h] = zone.share();
    (
        Name::new(format!("Zone_{}", zone.name())),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(x * 100.0),
            top: Val::Percent(y * 100.0),
            width: Val::Percent(w * 100.0),
            height: Val::Percent(h * 100.0),
            overflow: Overflow::clip(),
            ..default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_stay_on_screen_at_every_resolution() {
        for screen in [
            Vec2::new(1280.0, 720.0),
            Vec2::new(1920.0, 1080.0),
            Vec2::new(3840.0, 2160.0),
        ] {
            let full = Rect::from_corners(Vec2::ZERO, screen);
            for zone in Zone::ALL {
                let r = rect(zone, screen);
                assert!(
                    r.min.x >= full.min.x && r.max.x <= full.max.x + 0.01,
                    "{zone:?}"
                );
                assert!(
                    r.min.y >= full.min.y && r.max.y <= full.max.y + 0.01,
                    "{zone:?}"
                );
            }
        }
    }

    #[test]
    fn side_panel_is_422_wide_at_1280() {
        let r = rect(Zone::SidePanel, Vec2::new(1280.0, 720.0));
        assert!((r.width() - 422.4).abs() < 0.1);
        // The top bar and the side panel never overlap.
        assert!(rect(Zone::TopBar, Vec2::new(1280.0, 720.0)).max.y <= r.min.y);
    }
}
