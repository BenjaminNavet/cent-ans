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
    fn labels() {
        assert_eq!(short(950), "950");
        assert_eq!(short(12_400), "12,4 k");
        assert_eq!(short(-1_500_000), "\u{2212}1,5 M");
        assert_eq!(season_label(0), "Printemps 1337");
        assert_eq!(season_label(5), "Été 1338");
    }
}
