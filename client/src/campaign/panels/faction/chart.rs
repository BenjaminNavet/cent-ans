//! Geometry of the treasury chart (`treasury_chart.gd`): pure, so it is unit-tested; the view
//! turns each [`Bar`] into UI nodes.

/// One season of the chart, in px from the top of the plot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub top: f32,
    pub height: f32,
    /// Vertical position of the value (the curve dot).
    pub dot_y: f32,
    pub negative: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartGeometry {
    pub bars: Vec<Bar>,
    /// Vertical position of the zero line.
    pub zero_y: f32,
    pub low: i64,
    pub high: i64,
}

/// Lays `values` out in a plot `height` px tall. The scale always includes zero (the slope is
/// judged honestly), as in Godot.
pub fn layout(values: &[i64], height: f32) -> ChartGeometry {
    let low = values.iter().copied().min().unwrap_or(0).min(0);
    let mut high = values.iter().copied().max().unwrap_or(0).max(0);
    if high == low {
        high = low + 1;
    }
    let span = (high - low) as f32;
    let y_of = |value: i64| (high - value) as f32 / span * height;
    let zero_y = y_of(0);
    let bars = values
        .iter()
        .map(|&value| {
            let y = y_of(value);
            Bar {
                top: y.min(zero_y),
                height: (y - zero_y).abs().max(1.0),
                dot_y: y,
                negative: value < 0,
            }
        })
        .collect();
    ChartGeometry {
        bars,
        zero_y,
        low,
        high,
    }
}

/// Width in px of the plot area (the curve is laid out in px, so the area has a fixed width).
pub const PLOT_WIDTH: f32 = 440.0;
/// Thickness in px of the curve.
pub const LINE_THICKNESS: f32 = 2.0;

/// One segment of the treasury curve between two consecutive seasons: a rectangle of `length`
/// px centred on (`mid_x`, `mid_y`) and turned by `angle` radians (clockwise on screen).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub mid_x: f32,
    pub mid_y: f32,
    pub length: f32,
    pub angle: f32,
    /// One end is below zero.
    pub negative: bool,
}

/// The curve through the dots of `geometry` in a plot `width` px wide; the dot of season `i`
/// sits at the middle of its column, like the bars.
pub fn segments(geometry: &ChartGeometry, width: f32) -> Vec<Segment> {
    let count = geometry.bars.len().max(1) as f32;
    let x_of = |index: usize| (index as f32 + 0.5) / count * width;
    geometry
        .bars
        .windows(2)
        .enumerate()
        .map(|(index, pair)| {
            let (x0, y0) = (x_of(index), pair[0].dot_y);
            let (x1, y1) = (x_of(index + 1), pair[1].dot_y);
            let (dx, dy) = (x1 - x0, y1 - y0);
            Segment {
                mid_x: (x0 + x1) / 2.0,
                mid_y: (y0 + y1) / 2.0,
                length: dx.hypot(dy),
                angle: dy.atan2(dx),
                negative: pair[0].negative || pair[1].negative,
            }
        })
        .collect()
}

/// `12400` -> « 12,4 k », `-1500000` -> « −1,5 M » (axis labels).
pub fn short(value: i64) -> String {
    let magnitude = value.unsigned_abs();
    let sign = if value < 0 { "\u{2212}" } else { "" };
    if magnitude >= 1_000_000 {
        format!("{sign}{:.1} M", magnitude as f64 / 1e6).replace('.', ",")
    } else if magnitude >= 10_000 {
        format!("{sign}{:.1} k", magnitude as f64 / 1e3).replace('.', ",")
    } else {
        format!("{sign}{magnitude}")
    }
}

const SEASONS: [&str; 4] = ["Printemps", "Été", "Automne", "Hiver"];
const START_YEAR: u32 = 1337;

/// `5` -> « Été 1338 ».
pub fn season_label(turn: u32) -> String {
    format!("{} {}", SEASONS[(turn % 4) as usize], START_YEAR + turn / 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_values_rise_from_the_zero_line() {
        let geometry = layout(&[0, 50, 100], 100.0);
        assert_eq!(geometry.zero_y, 100.0);
        assert_eq!(geometry.bars[2].top, 0.0);
        assert_eq!(geometry.bars[2].height, 100.0);
        assert_eq!(geometry.bars[1].top, 50.0);
        assert!(!geometry.bars[1].negative);
    }

    #[test]
    fn negative_values_hang_below_zero() {
        let geometry = layout(&[100, -100], 100.0);
        assert_eq!(geometry.zero_y, 50.0);
        assert_eq!(geometry.bars[1].top, 50.0);
        assert_eq!(geometry.bars[1].height, 50.0);
        assert!(geometry.bars[1].negative);
        assert_eq!(geometry.low, -100);
    }

    #[test]
    fn flat_or_empty_series_do_not_divide_by_zero() {
        assert_eq!(layout(&[], 80.0).bars.len(), 0);
        let flat = layout(&[0, 0], 80.0);
        assert!(flat.bars.iter().all(|bar| bar.height >= 1.0));
    }

    #[test]
    fn curve_has_one_segment_per_pair_of_seasons() {
        let geometry = layout(&[0, 100, 100, -100], 100.0);
        let curve = segments(&geometry, 400.0);
        assert_eq!(curve.len(), 3);
        // Flat piece: no tilt, as long as the column pitch.
        assert!(curve[1].angle.abs() < 1e-6);
        assert!((curve[1].length - 100.0).abs() < 1e-3);
        // Rising piece tilts counter-clockwise on screen (negative angle), falling one the other way.
        assert!(curve[0].angle < 0.0);
        assert!(curve[2].angle > 0.0 && curve[2].negative);
        assert!(segments(&layout(&[5], 100.0), 400.0).is_empty());
    }

    #[test]
    fn labels() {
        assert_eq!(short(950), "950");
        assert_eq!(short(12_400), "12,4 k");
        assert_eq!(short(-1_500_000), "\u{2212}1,5 M");
        assert_eq!(season_label(0), "Printemps 1337");
        assert_eq!(season_label(5), "Été 1338");
    }
}
