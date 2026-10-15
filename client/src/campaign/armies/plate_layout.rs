//! Screen placement of the army plates (port of `army_plate_layout.gd`): plates keep their base
//! above the banner unless it collides with a town name or a higher priority plate, in which case
//! the shared [`LabelPlacer`] picks the first free fallback position (hysteresis: no flicker).

use std::collections::HashMap;

use bevy::prelude::*;

use crate::campaign::provinces::labels::{LabelPlacer, PlateItem};

/// Plates farther than this many camera distances are hidden in the grazing views.
pub const CLOSE_PLATE_RANGE_FACTOR: f32 = 40.0;
/// Screen margin around the viewport inside which a plate still counts as shown.
pub const VIEWPORT_MARGIN: f32 = 40.0;

/// One visible plate: its base rectangle and what decides its priority.
#[derive(Debug, Clone)]
pub struct PlateInput {
    pub id: String,
    pub base: Rect,
    pub selected: bool,
    pub player: bool,
    pub men: u32,
}

/// Priority order (`ArmyPlateLayout._before`): selected army, the player's armies, bigger
/// armies, then id.
fn before(a: &PlateInput, b: &PlateInput) -> std::cmp::Ordering {
    b.selected
        .cmp(&a.selected)
        .then(b.player.cmp(&a.player))
        .then(b.men.cmp(&a.men))
        .then(a.id.cmp(&b.id))
}

/// Offsets to add to each plate base so that plates avoid `obstacles` and one another.
pub fn place_plates(
    placer: &mut LabelPlacer,
    mut inputs: Vec<PlateInput>,
    obstacles: &[Rect],
) -> HashMap<String, Vec2> {
    inputs.sort_by(before);
    let items: Vec<PlateItem> = inputs
        .into_iter()
        .map(|i| PlateItem {
            id: i.id,
            rect: i.base,
        })
        .collect();
    placer.place(&items, obstacles)
}

/// Should the plate of an army far from a grazing camera be hidden (`CLOSE_PLATE_RANGE_FACTOR`)?
pub fn too_far_for_close_view(
    camera_distance: f32,
    anchor_distance: f32,
    selected: bool,
    close_knee: f32,
) -> bool {
    camera_distance < close_knee
        && !selected
        && anchor_distance > CLOSE_PLATE_RANGE_FACTOR * camera_distance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(id: &str, men: u32, player: bool, selected: bool) -> PlateInput {
        PlateInput {
            id: id.to_owned(),
            base: Rect::new(100.0, 100.0, 166.0, 120.0),
            selected,
            player,
            men,
        }
    }

    fn rect_of(i: &PlateInput, offsets: &HashMap<String, Vec2>) -> Rect {
        let o = offsets[&i.id];
        Rect::new(
            i.base.min.x + o.x,
            i.base.min.y + o.y,
            i.base.max.x + o.x,
            i.base.max.y + o.y,
        )
    }

    #[test]
    fn stacked_plates_are_pushed_apart() {
        let inputs = vec![
            input("a", 500, false, false),
            input("b", 900, false, false),
            input("c", 100, true, false),
        ];
        let offsets = place_plates(&mut LabelPlacer::default(), inputs.clone(), &[]);
        let rects: Vec<Rect> = inputs.iter().map(|i| rect_of(i, &offsets)).collect();
        for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                assert!(
                    rects[i].intersect(rects[j]).is_empty(),
                    "{i} overlaps {j}: {rects:?}"
                );
            }
        }
    }

    #[test]
    fn priority_keeps_the_base() {
        let inputs = vec![input("a", 900, false, false), input("z", 10, true, false)];
        let offsets = place_plates(&mut LabelPlacer::default(), inputs, &[]);
        assert_eq!(offsets["z"], Vec2::ZERO, "the player's army keeps its base");
        assert_ne!(offsets["a"], Vec2::ZERO);
        let inputs = vec![input("z", 10, true, false), input("a", 900, false, true)];
        let offsets = place_plates(&mut LabelPlacer::default(), inputs, &[]);
        assert_eq!(offsets["a"], Vec2::ZERO, "the selected army wins");
    }

    #[test]
    fn plates_avoid_town_names() {
        let wall = Rect::new(90.0, 90.0, 180.0, 125.0);
        let inputs = vec![input("a", 500, false, false)];
        let offsets = place_plates(&mut LabelPlacer::default(), inputs.clone(), &[wall]);
        assert!(rect_of(&inputs[0], &offsets).intersect(wall).is_empty());
    }

    #[test]
    fn far_plates_hide_only_in_grazing_views() {
        assert!(too_far_for_close_view(5.0, 500.0, false, 12.0));
        assert!(!too_far_for_close_view(5.0, 500.0, true, 12.0));
        assert!(!too_far_for_close_view(50.0, 500.0, false, 12.0));
    }
}
