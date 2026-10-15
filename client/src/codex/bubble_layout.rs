//! Placement of Codex bubbles beside their parent (port of game/scripts/codex/bubble_layout.gd,
//! `place_beside` / `free_y`).
//!
//! The kit's `tooltip::place` puts a child bubble at the parent's top; this variant aligns the
//! child on the link's line (`anchor_y`), and slides it below any open ancestor that would be
//! covered, so a chain of bubbles never hides the one before. Right of the parent first, then
//! left, then below; `None` when nothing fits (the caller collapses the oldest ancestor, as in
//! Godot). Pure functions, logical pixels.

use bevy::prelude::*;

/// Gap between a child and its parent.
pub const GAP: f32 = 6.0;

/// Top-left of a bubble of `size` beside `parent`, inside `area`, clear of `avoid`.
pub fn place_beside(
    size: Vec2,
    parent: Rect,
    anchor_y: f32,
    area: Rect,
    avoid: &[Rect],
    gap: f32,
) -> Option<Vec2> {
    let top = anchor_y.clamp(area.min.y, area.min.y.max(area.max.y - size.y));
    for x in [parent.max.x + gap, parent.min.x - gap - size.x] {
        if x < area.min.x || x + size.x > area.max.x {
            continue;
        }
        if let Some(y) = free_y(x, top, size, area, avoid, gap) {
            return Some(Vec2::new(x, y));
        }
    }
    let below_x = parent
        .min
        .x
        .clamp(area.min.x, area.min.x.max(area.max.x - size.x));
    free_y(below_x, parent.max.y + gap, size, area, avoid, gap).map(|y| Vec2::new(below_x, y))
}

/// First ordinate at or below `y` where the rectangle fits in `area` clear of `avoid`.
pub fn free_y(x: f32, mut y: f32, size: Vec2, area: Rect, avoid: &[Rect], gap: f32) -> Option<f32> {
    for _ in 0..=avoid.len() {
        if y + size.y > area.max.y {
            return None;
        }
        let rect = Rect::from_corners(Vec2::new(x, y), Vec2::new(x, y) + size);
        match avoid
            .iter()
            .find(|other| !rect.intersect(**other).is_empty())
        {
            None => return Some(y),
            Some(blocker) => y = blocker.max.y + gap,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::new(0.0, 0.0, 1000.0, 600.0)
    }

    #[test]
    fn right_of_parent_then_left_then_below() {
        let size = Vec2::new(200.0, 100.0);
        let parent = Rect::new(300.0, 100.0, 500.0, 300.0);
        let at = place_beside(size, parent, 150.0, area(), &[], GAP).unwrap();
        assert_eq!(at, Vec2::new(506.0, 150.0));
        let right_edge = Rect::new(780.0, 100.0, 980.0, 300.0);
        let at = place_beside(size, right_edge, 150.0, area(), &[], GAP).unwrap();
        assert_eq!(at, Vec2::new(574.0, 150.0));
        let wide = Rect::new(10.0, 100.0, 990.0, 300.0);
        let at = place_beside(size, wide, 150.0, area(), &[], GAP).unwrap();
        assert_eq!(at.y, 306.0);
    }

    #[test]
    fn slides_below_a_blocking_ancestor() {
        let size = Vec2::new(200.0, 100.0);
        let parent = Rect::new(300.0, 100.0, 500.0, 300.0);
        let ancestor = Rect::new(500.0, 120.0, 720.0, 260.0);
        let at = place_beside(size, parent, 150.0, area(), &[ancestor], GAP).unwrap();
        assert_eq!(at, Vec2::new(506.0, 266.0));
    }

    #[test]
    fn none_when_nothing_fits() {
        let size = Vec2::new(200.0, 500.0);
        let parent = Rect::new(300.0, 0.0, 500.0, 590.0);
        let blocker = Rect::new(0.0, 0.0, 1000.0, 100.0);
        assert!(place_beside(size, parent, 0.0, area(), &[blocker], GAP).is_none());
    }
}
