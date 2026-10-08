//! Plane geometry shared by the battle code: points are `(x, z)` in metres.
//! The formulas are the ones the battle rules were tuned with: change their
//! rounding and the replays change.

/// Parameter in `[0, 1]` of the point of the segment `a`-`b` nearest `p`
/// (0 for a degenerate segment).
pub fn segment_param(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx * dx + dz * dz).max(1e-9);
    (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0)
}

/// Point of the segment `a`-`b` nearest `p`.
pub fn closest_on_segment(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> (f64, f64) {
    let t = segment_param(a, b, p);
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Distance from `p` to the segment `a`-`b`, through the closest point.
pub fn distance_to_segment(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
    let (cx, cz) = closest_on_segment(a, b, p);
    ((p.0 - cx).powi(2) + (p.1 - cz).powi(2)).sqrt()
}

/// Distance from `p` to the segment `a`-`b` (`hypot` form of the river and
/// AI code).
pub fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dz * dz;
    let t = if len2 < 1e-9 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - dx * t).hypot(p.1 - a.1 - dz * t)
}

/// Is `p` inside the closed polygon `ring` (even-odd rule)?
pub fn point_in_polygon(ring: &[(f64, f64)], p: (f64, f64)) -> bool {
    let mut inside = false;
    let mut j = ring.len() - 1;
    for (i, &(xi, zi)) in ring.iter().enumerate() {
        let (xj, zj) = ring[j];
        if (zi > p.1) != (zj > p.1) && p.0 < (xj - xi) * (p.1 - zi) / (zj - zi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Twice the signed area of the triangle `o`, `a`, `b`.
fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Proper intersection of the segments `p1`-`p2` and `q1`-`q2`.
pub fn segments_intersect(p1: (f64, f64), p2: (f64, f64), q1: (f64, f64), q2: (f64, f64)) -> bool {
    let d1 = cross(q1, q2, p1);
    let d2 = cross(q1, q2, p2);
    let d3 = cross(p1, p2, q1);
    let d4 = cross(p1, p2, q2);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_clamp_to_the_ends() {
        let (a, b) = ((0.0, 0.0), (10.0, 0.0));
        assert_eq!(distance_to_segment(a, b, (5.0, 3.0)), 3.0);
        assert_eq!(distance_to_segment(a, b, (13.0, 4.0)), 5.0);
        assert_eq!(segment_distance((13.0, 4.0), a, b), 5.0);
        assert_eq!(closest_on_segment(a, b, (-4.0, 1.0)), a);
        assert_eq!(distance_to_segment(a, a, (3.0, 4.0)), 5.0);
    }

    #[test]
    fn polygon_and_segment_tests() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_polygon(&square, (5.0, 5.0)));
        assert!(!point_in_polygon(&square, (15.0, 5.0)));
        assert!(segments_intersect(
            (0.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (10.0, 0.0)
        ));
        assert!(!segments_intersect(
            (0.0, 0.0),
            (1.0, 1.0),
            (5.0, 0.0),
            (5.0, 9.0)
        ));
    }
}
