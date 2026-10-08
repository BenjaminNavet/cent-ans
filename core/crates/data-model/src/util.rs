//! Small numeric and text helpers shared by the simulation crates.

/// Euclidean distance between two map points.
pub fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Distance from `p` to the segment `a`-`b`.
pub fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    if len2 <= f32::EPSILON {
        return dist(p, a);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0);
    dist(p, [a[0] + t * dx, a[1] + t * dy])
}

/// SplitMix64 finaliser (well spread bits from a plain hash).
pub fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// `text` with its first character lowercased.
pub fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_distance_clamps_to_ends() {
        assert_eq!(
            segment_distance([5.0, 1.0], [0.0, 0.0], [2.0, 0.0]),
            dist([5.0, 1.0], [2.0, 0.0])
        );
        assert_eq!(segment_distance([1.0, 3.0], [0.0, 0.0], [2.0, 0.0]), 3.0);
        assert_eq!(
            segment_distance([1.0, 1.0], [0.0, 0.0], [0.0, 0.0]),
            dist([1.0, 1.0], [0.0, 0.0])
        );
    }

    #[test]
    fn lowercase_first_handles_empty() {
        assert_eq!(lowercase_first(""), "");
        assert_eq!(lowercase_first("École"), "école");
    }
}
