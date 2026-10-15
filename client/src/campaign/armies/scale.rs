//! Army marker scale law versus camera distance (port of `army_scale.gd`, ADR 0160).

/// Token scale per unit of camera distance (parchment view).
pub const SCALE_PER_DISTANCE: f32 = 0.014;
/// Default exponent of the normal-view law (1 = constant screen size, 0 = fixed world size).
pub const DEFAULT_SCALE_EXPONENT: f32 = 0.65;
pub const MIN_SCALE: f32 = 0.27;
pub const MAX_SCALE: f32 = 14.0;
/// Below this distance the scale is proportional to the distance again.
pub const CLOSE_KNEE_DISTANCE: f32 = 12.0;

/// Marker scale for a camera distance; `strategic_weight` 0 = normal view, 1 = parchment.
pub fn scale_for_distance(camera_distance: f32, strategic_weight: f32, exponent: f32) -> f32 {
    if camera_distance < CLOSE_KNEE_DISTANCE {
        return MIN_SCALE * camera_distance.max(0.02) / CLOSE_KNEE_DISTANCE;
    }
    let token = (camera_distance * SCALE_PER_DISTANCE).clamp(MIN_SCALE, MAX_SCALE);
    let knee = MIN_SCALE / SCALE_PER_DISTANCE;
    let posed = (MIN_SCALE * (camera_distance / knee).max(1.0).powf(exponent)).min(token);
    let weight = strategic_weight.clamp(0.0, 1.0);
    posed + (token - posed) * weight
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_view_is_proportional() {
        let s = scale_for_distance(6.0, 0.0, DEFAULT_SCALE_EXPONENT);
        assert!((s - MIN_SCALE * 0.5).abs() < 1e-6);
    }

    #[test]
    fn normal_view_is_sublinear_and_parchment_is_linear() {
        let d = 400.0;
        let posed = scale_for_distance(d, 0.0, DEFAULT_SCALE_EXPONENT);
        let token = scale_for_distance(d, 1.0, DEFAULT_SCALE_EXPONENT);
        assert!(posed < token);
        let knee = MIN_SCALE / SCALE_PER_DISTANCE;
        let expected = MIN_SCALE * (d / knee).powf(DEFAULT_SCALE_EXPONENT);
        assert!((posed - expected).abs() < 1e-4);
        assert!((token - d * SCALE_PER_DISTANCE).abs() < 1e-4);
    }

    #[test]
    fn exponent_one_gives_constant_screen_size() {
        let a = scale_for_distance(100.0, 0.0, 1.0) / 100.0;
        let b = scale_for_distance(800.0, 0.0, 1.0) / 800.0;
        assert!((a - b).abs() < 1e-5);
    }
}
