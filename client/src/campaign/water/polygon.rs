//! Polygon helpers for lakes and sea basins: ear-clipping triangulation (replaces
//! `Geometry2D.triangulate_polygon`), outward offset (replaces `offset_polygon`, mitered), point
//! in polygon and scan-line fill.

use bevy::prelude::*;

/// Twice the signed area (positive for counter-clockwise in a y-up frame).
pub fn signed_area2(polygon: &[Vec2]) -> f32 {
    let n = polygon.len();
    (0..n)
        .map(|i| polygon[i].perp_dot(polygon[(i + 1) % n]))
        .sum()
}

fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = (b - a).perp_dot(p - a);
    let d2 = (c - b).perp_dot(p - b);
    let d3 = (a - c).perp_dot(p - c);
    d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0
}

/// Triangle indices (into `polygon`) by ear clipping; empty if the polygon is degenerate or
/// self-intersecting beyond repair. O(n^2), fine for lake outlines.
pub fn triangulate(polygon: &[Vec2]) -> Vec<u32> {
    let n = polygon.len();
    if n < 3 || signed_area2(polygon).abs() < 1e-9 {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..n).collect();
    if signed_area2(polygon) < 0.0 {
        order.reverse();
    }
    let mut out = Vec::with_capacity((n - 2) * 3);
    let mut guard = 0;
    while order.len() > 3 && guard < n * n {
        guard += 1;
        let m = order.len();
        let mut clipped = false;
        for k in 0..m {
            let (ia, ib, ic) = (order[(k + m - 1) % m], order[k], order[(k + 1) % m]);
            let (a, b, c) = (polygon[ia], polygon[ib], polygon[ic]);
            if (b - a).perp_dot(c - b) <= 1e-9 {
                continue; // reflex or flat
            }
            let blocked = order.iter().any(|&j| {
                j != ia && j != ib && j != ic && {
                    let p = polygon[j];
                    p != a && p != b && p != c && in_triangle(p, a, b, c)
                }
            });
            if blocked {
                continue;
            }
            out.extend_from_slice(&[ia as u32, ib as u32, ic as u32]);
            order.remove(k);
            clipped = true;
            break;
        }
        if !clipped {
            // Degenerate remainder: drop a flat vertex to keep going, else give up.
            let m = order.len();
            let flat = (0..m).find(|&k| {
                let (a, b, c) = (
                    polygon[order[(k + m - 1) % m]],
                    polygon[order[k]],
                    polygon[order[(k + 1) % m]],
                );
                (b - a).perp_dot(c - b).abs() <= 1e-9
            });
            match flat {
                Some(k) => {
                    order.remove(k);
                }
                None => return Vec::new(),
            }
        }
    }
    if order.len() == 3 {
        out.extend(order.iter().map(|&i| i as u32));
    }
    out
}

/// Outline grown by `distance` along vertex bisectors (miter, limited to 2x), a cheap stand-in for
/// `Geometry2D.offset_polygon(JOIN_ROUND)`: lakes only need the water to slip under the shore.
pub fn offset(polygon: &[Vec2], distance: f32) -> Vec<Vec2> {
    let n = polygon.len();
    if n < 3 || distance == 0.0 {
        return polygon.to_vec();
    }
    let sign = if signed_area2(polygon) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    (0..n)
        .map(|i| {
            let prev = polygon[(i + n - 1) % n];
            let next = polygon[(i + 1) % n];
            let p = polygon[i];
            // Outward normals of the two edges (counter-clockwise: right-hand side is outside).
            let n1 = (p - prev).normalize_or_zero();
            let n2 = (next - p).normalize_or_zero();
            let o1 = Vec2::new(n1.y, -n1.x) * sign;
            let o2 = Vec2::new(n2.y, -n2.x) * sign;
            let bisector = (o1 + o2).normalize_or_zero();
            if bisector == Vec2::ZERO {
                return p;
            }
            let miter = (1.0 / bisector.dot(o1).max(0.5)).min(2.0);
            p + bisector * distance * miter
        })
        .collect()
}

/// Even-odd point in polygon.
pub fn contains(polygon: &[Vec2], p: Vec2) -> bool {
    let mut inside = false;
    let n = polygon.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Vec2> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(0.0, 4.0),
        ]
    }

    fn area_of(polygon: &[Vec2], tris: &[u32]) -> f32 {
        tris.chunks(3)
            .map(|t| {
                let (a, b, c) = (
                    polygon[t[0] as usize],
                    polygon[t[1] as usize],
                    polygon[t[2] as usize],
                );
                ((b - a).perp_dot(c - a)).abs() * 0.5
            })
            .sum()
    }

    #[test]
    fn triangulates_square_and_concave() {
        let sq = square();
        let tris = triangulate(&sq);
        assert_eq!(tris.len(), 6);
        assert!((area_of(&sq, &tris) - 16.0).abs() < 1e-4);
        // L shape, clockwise input.
        let mut l = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 2.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(2.0, 4.0),
            Vec2::new(0.0, 4.0),
        ];
        l.reverse();
        let tris = triangulate(&l);
        assert_eq!(tris.len(), 12);
        assert!((area_of(&l, &tris) - 12.0).abs() < 1e-4);
    }

    #[test]
    fn degenerate_polygon_gives_nothing() {
        assert!(triangulate(&[Vec2::ZERO, Vec2::X]).is_empty());
        assert!(triangulate(&[Vec2::ZERO, Vec2::X, Vec2::new(2.0, 0.0)]).is_empty());
    }

    #[test]
    fn offset_grows_square() {
        let grown = offset(&square(), 1.0);
        assert!(grown[0].x < 0.0 && grown[0].y < 0.0);
        assert!(grown[2].x > 4.0 && grown[2].y > 4.0);
        assert!(signed_area2(&grown).abs() > signed_area2(&square()).abs());
    }

    #[test]
    fn point_in_polygon() {
        let sq = square();
        assert!(contains(&sq, Vec2::new(2.0, 2.0)));
        assert!(!contains(&sq, Vec2::new(5.0, 2.0)));
    }
}
