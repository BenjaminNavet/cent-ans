//! Is the pointer over a piece of interface? The map ignores clicks, hover, path preview and the
//! context cursor while it is (a panel, a dialog, a button), except over the army plates and the
//! hover hint, which belong to the map. Port of the "mouse over a control" tests of
//! `army_movement_controller.gd`.

use bevy::picking::hover::HoverMap;
use bevy::prelude::*;

use super::feedback::HintNode;
use super::markers::ArmyPlate;

/// Alpha above which a plain background counts as a visible piece of interface.
pub const VISIBLE_ALPHA: f32 = 0.05;

/// The pointer is over interface that is not the map's own.
#[derive(Resource, Default, Debug)]
pub struct PointerOverUi {
    pub over: bool,
    /// Frames during which the map ignored the pointer.
    pub blocked_frames: usize,
}

/// A node blocks the map pointer when it is drawn (image or visible background) and is not the
/// map's own.
pub fn blocks_pointer(background_alpha: Option<f32>, has_image: bool, own: bool) -> bool {
    !own && (has_image || background_alpha.is_some_and(|a| a > VISIBLE_ALPHA))
}

#[allow(clippy::type_complexity)]
pub fn track_pointer_over_ui(
    hover: Option<Res<HoverMap>>,
    nodes: Query<
        (
            Option<&BackgroundColor>,
            Option<&ImageNode>,
            Has<ArmyPlate>,
            Has<HintNode>,
        ),
        With<Node>,
    >,
    mut state: ResMut<PointerOverUi>,
) {
    let over = hover.is_some_and(|hover| {
        hover.values().flat_map(|hits| hits.keys()).any(|entity| {
            nodes
                .get(*entity)
                .is_ok_and(|(background, image, plate, hint)| {
                    blocks_pointer(
                        background.map(|b| b.0.alpha()),
                        image.is_some(),
                        plate || hint,
                    )
                })
        })
    });
    if state.over != over {
        state.over = over;
    }
    if over {
        state.blocked_frames += 1;
    }
}

/// Run condition: the pointer is free for the map.
pub fn pointer_free(state: Res<PointerOverUi>) -> bool {
    !state.over
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panels_block_and_map_nodes_do_not() {
        assert!(blocks_pointer(Some(0.9), false, false));
        assert!(blocks_pointer(None, true, false));
        assert!(!blocks_pointer(Some(0.0), false, false));
        assert!(!blocks_pointer(None, false, false));
        assert!(!blocks_pointer(Some(0.94), false, true));
    }
}
