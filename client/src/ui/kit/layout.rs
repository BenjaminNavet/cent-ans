//! Shared screen layout of the campaign: top-bar height, edge margins and the zones panels fit
//! in (port of the zone idea of `game/scripts/ui/ui_layout.gd`, bible DA § 12.1).
//!
//! A panel root puts a [`FitZone`] on itself instead of hard-coded `top` / `right` offsets; the
//! [`apply_fit_zones`] system then places it so it never covers the top bar nor the bottom-right
//! HUD cluster (alerts, end-of-season seal, minimap), whose size the HUD publishes in
//! [`HudReserve`]. Numbers are logical px (after `UiScale`), recomputed when the window changes.

use bevy::prelude::*;
use bevy::ui::ComputedUiRenderTargetInfo;

/// Height of the campaign top bar.
pub const TOP_BAR_HEIGHT: f32 = 40.0;
/// Margin between HUD / panels and the window edge.
pub const EDGE: f32 = 8.0;
/// Gap between two neighbouring HUD blocks.
pub const GAP: f32 = 6.0;
/// Top offset of everything docked under the top bar.
pub const PANEL_TOP: f32 = TOP_BAR_HEIGHT + GAP;
/// A side panel is never squeezed below this height (it then overlaps rather than vanish).
pub const MIN_SIDE_HEIGHT: f32 = 260.0;
/// Bottom panels (army sheet) take at most this share of the window height.
pub const BOTTOM_MAX_SHARE: f32 = 0.46;
/// Height of the army strip (« l'ost de … ») at the bottom left, kept visible under bottom panels.
pub const ARMY_STRIP_HEIGHT: f32 = 34.0;

/// Size (logical px) of the HUD blocks panels must keep clear of, published by the HUD.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct HudReserve {
    /// Bottom-right cluster: alerts + seal + minimap.
    pub bottom_right: Vec2,
}

/// Where a panel root sits.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitZone {
    /// Docked on the right under the top bar, ending above the bottom-right cluster.
    Side,
    /// Like [`FitZone::Side`] but as tall as the zone allows (stretched, for panels with a list
    /// that scrolls inside).
    SideTall,
    /// Docked on the left under the advisor column's top, above the army strip.
    Left,
    /// Docked at the bottom left, just above the army strip, leaving the advisor column (top
    /// left) and the map centre free (tutorial card).
    BottomLeft,
    /// Docked at the bottom, from the left edge to the bottom-right cluster.
    Bottom,
    /// Large window: under the top bar, the whole width and height.
    Full,
}

/// Offsets a zone gives its node (`None` = `Val::Auto`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneBox {
    pub left: Option<f32>,
    pub right: Option<f32>,
    pub top: Option<f32>,
    pub bottom: Option<f32>,
    pub max_height: Option<f32>,
}

/// Offsets of `zone` on a `screen` (logical px) with the HUD `reserve`. Pure, tested.
pub fn zone_box(zone: FitZone, screen: Vec2, reserve: HudReserve) -> ZoneBox {
    match zone {
        FitZone::Side => {
            let free = screen.y - PANEL_TOP - reserve.bottom_right.y - EDGE - GAP;
            ZoneBox {
                left: None,
                right: Some(EDGE),
                top: Some(PANEL_TOP),
                bottom: None,
                max_height: Some(free.max(MIN_SIDE_HEIGHT.min(screen.y - PANEL_TOP - EDGE))),
            }
        }
        FitZone::SideTall => {
            let bottom = (EDGE + reserve.bottom_right.y + GAP)
                .min(screen.y - PANEL_TOP - MIN_SIDE_HEIGHT.min(screen.y - PANEL_TOP - EDGE));
            ZoneBox {
                left: None,
                right: Some(EDGE),
                top: Some(PANEL_TOP),
                bottom: Some(bottom.max(EDGE)),
                max_height: None,
            }
        }
        FitZone::Left => ZoneBox {
            left: Some(EDGE),
            right: None,
            top: Some(PANEL_TOP),
            bottom: None,
            max_height: Some((screen.y - PANEL_TOP - 80.0).max(200.0)),
        },
        FitZone::BottomLeft => ZoneBox {
            left: Some(EDGE),
            right: None,
            top: None,
            bottom: Some(EDGE + ARMY_STRIP_HEIGHT + GAP),
            max_height: Some((screen.y * 0.5 - ARMY_STRIP_HEIGHT).max(160.0)),
        },
        FitZone::Bottom => ZoneBox {
            left: Some(EDGE),
            right: Some(EDGE + reserve.bottom_right.x + GAP),
            top: None,
            bottom: Some(EDGE + ARMY_STRIP_HEIGHT + GAP),
            max_height: Some((screen.y * BOTTOM_MAX_SHARE).max(200.0)),
        },
        FitZone::Full => ZoneBox {
            left: Some(EDGE),
            right: Some(EDGE),
            top: Some(PANEL_TOP),
            bottom: Some(EDGE),
            max_height: None,
        },
    }
}

fn px(value: Option<f32>) -> Val {
    value.map_or(Val::Auto, Val::Px)
}

/// Places every [`FitZone`] node from the window size and the [`HudReserve`].
pub fn apply_fit_zones(
    reserve: Res<HudReserve>,
    mut nodes: Query<(
        &FitZone,
        &mut Node,
        &ComputedUiRenderTargetInfo,
        Option<&super::resize_grip::ResizableWindow>,
    )>,
) {
    for (zone, mut node, target, resizable) in &mut nodes {
        let screen = target.logical_size();
        if screen.y < 1.0 {
            continue;
        }
        let b = zone_box(*zone, screen, *reserve);
        let max_height = px(b.max_height);
        // A window the player resized keeps the height they chose.
        let max_height = if b.max_height.is_none() || resizable.is_some_and(|r| r.user_sized) {
            node.max_height
        } else {
            max_height
        };
        let wanted = (px(b.left), px(b.right), px(b.top), px(b.bottom), max_height);
        let current = (
            node.left,
            node.right,
            node.top,
            node.bottom,
            node.max_height,
        );
        if current != wanted {
            node.position_type = PositionType::Absolute;
            (
                node.left,
                node.right,
                node.top,
                node.bottom,
                node.max_height,
            ) = wanted;
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<HudReserve>().add_systems(
        PostUpdate,
        apply_fit_zones.before(bevy::ui::UiSystems::Layout),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREENS: [Vec2; 3] = [
        Vec2::new(1280.0, 720.0),
        Vec2::new(1920.0, 1080.0),
        Vec2::new(2560.0, 1440.0),
    ];

    fn reserve() -> HudReserve {
        HudReserve {
            bottom_right: Vec2::new(420.0, 230.0),
        }
    }

    #[test]
    fn side_panel_ends_above_the_bottom_right_cluster() {
        for screen in SCREENS {
            let b = zone_box(FitZone::Side, screen, reserve());
            let bottom = b.top.unwrap() + b.max_height.unwrap();
            assert!(b.top.unwrap() >= TOP_BAR_HEIGHT);
            assert!(
                bottom <= screen.y - reserve().bottom_right.y - EDGE,
                "{screen:?}: {bottom}"
            );
        }
    }

    #[test]
    fn bottom_panel_stops_left_of_the_cluster() {
        for screen in SCREENS {
            let b = zone_box(FitZone::Bottom, screen, reserve());
            assert!(b.right.unwrap() >= reserve().bottom_right.x + EDGE);
            assert!(b.max_height.unwrap() < screen.y - PANEL_TOP);
            // The army strip stays visible under the panel.
            assert!(b.bottom.unwrap() >= EDGE + ARMY_STRIP_HEIGHT);
        }
    }

    #[test]
    fn tall_side_panel_stretches_down_to_the_cluster() {
        for screen in SCREENS {
            let b = zone_box(FitZone::SideTall, screen, reserve());
            let height = screen.y - b.top.unwrap() - b.bottom.unwrap();
            assert!(height >= MIN_SIDE_HEIGHT.min(screen.y - PANEL_TOP - EDGE) - 0.1);
            assert!(b.bottom.unwrap() >= reserve().bottom_right.y + EDGE);
        }
    }

    #[test]
    fn side_panel_keeps_a_floor_on_short_screens() {
        let b = zone_box(
            FitZone::Side,
            Vec2::new(1280.0, 500.0),
            HudReserve {
                bottom_right: Vec2::new(400.0, 400.0),
            },
        );
        assert_eq!(b.max_height, Some(MIN_SIDE_HEIGHT));
    }
}
