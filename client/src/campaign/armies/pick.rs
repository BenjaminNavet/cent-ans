//! Unified army picking under a screen point (port of `army_picker.gd`, ADR 0160): pure function
//! over projected silhouettes and plates.

use bevy::prelude::*;

/// Distance (pixels) under which a point near a silhouette counts as "almost".
pub const PICK_NEAR_PX: f32 = 10.0;

/// One pickable army on screen.
#[derive(Debug, Clone)]
pub struct Pickable {
    pub id: String,
    /// Projected silhouette; `None` when off screen or behind the camera.
    pub rect: Option<Rect>,
    pub plate: Option<Rect>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickResult {
    pub id: String,
    /// 0 on a plate, else normalised distance to the centre (0-1 inside, 1-2 near).
    pub score: f32,
    pub direct: bool,
}

/// Nearest army to `cursor`; `exclude` is ignored (target picking for the selection).
pub fn pick_scored(items: &[Pickable], cursor: Vec2, exclude: Option<&str>) -> Option<PickResult> {
    let mut best: Option<PickResult> = None;
    for item in items {
        if exclude == Some(item.id.as_str()) {
            continue;
        }
        if item.plate.is_some_and(|p| p.contains(cursor)) {
            return Some(PickResult {
                id: item.id.clone(),
                score: 0.0,
                direct: true,
            });
        }
        let Some(rect) = item.rect else { continue };
        let center = rect.center();
        let half = rect.size() * 0.5;
        let score = if rect.contains(cursor) {
            let o = (cursor - center).abs() / half.max(Vec2::splat(1e-6));
            o.x.max(o.y)
        } else {
            let outside = (cursor - center).abs() - half;
            let gap = outside.max(Vec2::ZERO).length();
            if gap < PICK_NEAR_PX {
                1.0 + gap / PICK_NEAR_PX
            } else {
                f32::INFINITY
            }
        };
        if score.is_finite() && best.as_ref().is_none_or(|b| score < b.score) {
            best = Some(PickResult {
                id: item.id.clone(),
                score,
                direct: score <= 1.0,
            });
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, min: Vec2, max: Vec2) -> Pickable {
        Pickable {
            id: id.into(),
            rect: Some(Rect::from_corners(min, max)),
            plate: None,
        }
    }

    #[test]
    fn inside_beats_near_and_nearest_centre_wins() {
        let items = [
            item("a", Vec2::new(0.0, 0.0), Vec2::new(20.0, 20.0)),
            item("b", Vec2::new(18.0, 0.0), Vec2::new(60.0, 20.0)),
        ];
        let r = pick_scored(&items, Vec2::new(19.0, 10.0), None).unwrap();
        assert_eq!(r.id, "a"); // inside both, a's centre closer
        assert!(r.direct);
    }

    #[test]
    fn near_is_not_direct_and_far_is_none() {
        let items = [item("a", Vec2::ZERO, Vec2::new(20.0, 20.0))];
        let near = pick_scored(&items, Vec2::new(25.0, 10.0), None).unwrap();
        assert!(!near.direct && near.score > 1.0 && near.score < 2.0);
        assert!(pick_scored(&items, Vec2::new(50.0, 10.0), None).is_none());
    }

    #[test]
    fn plate_wins_and_exclude_skips() {
        let mut a = item("a", Vec2::ZERO, Vec2::new(10.0, 10.0));
        a.plate = Some(Rect::from_corners(
            Vec2::new(100.0, 100.0),
            Vec2::new(140.0, 120.0),
        ));
        let items = [
            a,
            item("b", Vec2::new(100.0, 100.0), Vec2::new(140.0, 120.0)),
        ];
        assert_eq!(
            pick_scored(&items, Vec2::new(110.0, 110.0), None)
                .unwrap()
                .id,
            "a"
        );
        assert_eq!(
            pick_scored(&items, Vec2::new(110.0, 110.0), Some("a"))
                .unwrap()
                .id,
            "b"
        );
    }
}
