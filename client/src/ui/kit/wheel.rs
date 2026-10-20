//! Does the mouse wheel belong to the interface? A wheel turned over a panel scrolls that panel
//! and must not zoom the map underneath (campaign and battle cameras).
//!
//! The picking hover map often reports only the topmost node (a text line inside a scroll list),
//! which has no background of its own: every hit is followed up its ancestors until a drawn node
//! (image or visible background) or a scroll area is found.

use bevy::picking::hover::HoverMap;
use bevy::prelude::*;

/// Alpha above which a plain background counts as drawn interface.
const VISIBLE_ALPHA: f32 = 0.05;

/// What a node says about the wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelHit {
    /// Drawn interface or scroll area: the wheel is the interface's.
    Ui,
    /// Transparent container: look at its parent.
    Through,
}

/// Classifies one node from its look.
pub fn classify(background_alpha: Option<f32>, has_image: bool, scrolls: bool) -> WheelHit {
    if scrolls || has_image || background_alpha.is_some_and(|alpha| alpha > VISIBLE_ALPHA) {
        WheelHit::Ui
    } else {
        WheelHit::Through
    }
}

/// UI nodes as seen by [`over_ui`].
pub type WheelNodes<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static BackgroundColor>,
        Option<&'static ImageNode>,
        &'static Node,
    ),
>;

/// Whether the wheel is over interface. `own` marks the map's own nodes (they let the zoom through).
pub fn over_ui(
    hover: Option<&HoverMap>,
    nodes: &WheelNodes,
    parents: &Query<&ChildOf>,
    own: impl Fn(Entity) -> bool,
) -> bool {
    let Some(hover) = hover else { return false };
    hover.values().flat_map(|hits| hits.keys()).any(|hit| {
        for entity in std::iter::once(*hit).chain(parents.iter_ancestors(*hit)) {
            if own(entity) {
                return false;
            }
            let Ok((background, image, node)) = nodes.get(entity) else {
                continue;
            };
            let scrolls =
                node.overflow.y == OverflowAxis::Scroll || node.overflow.x == OverflowAxis::Scroll;
            match classify(background.map(|b| b.0.alpha()), image.is_some(), scrolls) {
                WheelHit::Ui => return true,
                WheelHit::Through => {}
            }
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawn_or_scrolling_nodes_take_the_wheel() {
        assert_eq!(classify(Some(0.9), false, false), WheelHit::Ui);
        assert_eq!(classify(None, true, false), WheelHit::Ui);
        assert_eq!(classify(None, false, true), WheelHit::Ui);
        assert_eq!(classify(Some(0.0), false, false), WheelHit::Through);
        assert_eq!(classify(None, false, false), WheelHit::Through);
    }

    /// A text line with no background inside a parchment panel: the panel's ancestor wins.
    #[test]
    fn text_inside_a_panel_is_interface() {
        let mut world = World::new();
        let panel = world
            .spawn((Node::default(), BackgroundColor(Color::WHITE)))
            .id();
        let line = world.spawn((Node::default(), ChildOf(panel))).id();
        let root = world.spawn(Node::default()).id();
        let mut hover = HoverMap::default();
        hover.insert(
            bevy::picking::pointer::PointerId::Mouse,
            [(line, dummy_hit())].into_iter().collect(),
        );
        let mut state: bevy::ecs::system::SystemState<(WheelNodes, Query<&ChildOf>)> =
            bevy::ecs::system::SystemState::new(&mut world);
        let (nodes, parents) = state.get(&world).expect("read-only params");
        assert!(over_ui(Some(&hover), &nodes, &parents, |_| false));
        assert!(!over_ui(Some(&hover), &nodes, &parents, |e| e == panel));
        let mut bare = HoverMap::default();
        bare.insert(
            bevy::picking::pointer::PointerId::Mouse,
            [(root, dummy_hit())].into_iter().collect(),
        );
        assert!(!over_ui(Some(&bare), &nodes, &parents, |_| false));
    }

    fn dummy_hit() -> bevy::picking::backend::HitData {
        bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None)
    }
}
