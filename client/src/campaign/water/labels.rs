//! River name placement (port of the layout part of `river_labels.gd`): label anchors along named
//! river pieces, spaced by importance, laid along the flow. Pure data; rendering of the text is
//! left to the HUD/markers lots (3D text needs the UI kit fonts).

use std::collections::HashMap;

use bevy::prelude::*;

use super::data::{River, WaterData, Zone};

/// Half window (map px) smoothing the text direction along the course.
const TANGENT_WINDOW: f32 = 6.0;
/// Two labels of the same name closer than this share of the spacing are merged.
const SAME_NAME_CLEARANCE: f32 = 0.6;

/// A placed river label.
#[derive(Debug, Clone, PartialEq)]
pub struct RiverLabel {
    pub text: String,
    pub px: Vec2,
    /// Reading direction (never right-to-left), unit vector in map pixels.
    pub dir: Vec2,
    pub importance: i64,
    /// Camera distance (world units) beyond which the label is hidden.
    pub visible_to: f32,
}

fn length(points: &[Vec2]) -> f32 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Point at curvilinear abscissa `at`.
fn sample(points: &[Vec2], at: f32) -> Vec2 {
    let mut run = 0.0;
    for w in points.windows(2) {
        let seg = w[0].distance(w[1]);
        if run + seg >= at && seg > 0.0 {
            return w[0].lerp(w[1], (at - run) / seg);
        }
        run += seg;
    }
    *points.last().unwrap_or(&Vec2::ZERO)
}

fn tangent(points: &[Vec2], at: f32, total: f32) -> Vec2 {
    let a = sample(points, (at - TANGENT_WINDOW).max(0.0));
    let b = sample(points, (at + TANGENT_WINDOW).min(total));
    let dir = (b - a).normalize_or_zero();
    if dir == Vec2::ZERO { Vec2::X } else { dir }
}

fn in_zone(zones: &[Zone], p: Vec2, margin: f32) -> bool {
    zones
        .iter()
        .any(|z| p.distance(z.px) < z.radius_px + margin)
}

/// Places the labels of `data.rivers` following `data.display.labels`.
pub fn place(data: &WaterData) -> Vec<RiverLabel> {
    let cfg = &data.display.labels;
    if !cfg.enabled {
        return Vec::new();
    }
    let extent = data.map_size.x.max(data.map_size.y);
    let major_distance = cfg.major_max_distance * extent;
    // Most important and longest pieces first: they win the space.
    let lengths: Vec<f32> = data
        .rivers
        .iter()
        .map(|r: &River| length(&r.points))
        .collect();
    let mut order: Vec<usize> = (0..data.rivers.len()).collect();
    order.sort_by(|&a, &b| {
        let (ia, ib) = (data.rivers[a].importance, data.rivers[b].importance);
        ib.cmp(&ia).then(lengths[b].total_cmp(&lengths[a]))
    });
    let mut labels: Vec<RiverLabel> = Vec::new();
    let mut taken: HashMap<String, Vec<Vec2>> = HashMap::new();
    for ri in order {
        if labels.len() >= cfg.max_labels {
            break;
        }
        let river = &data.rivers[ri];
        let importance = river.importance.clamp(0, 6);
        let text = data.display_name(&river.name);
        if text.is_empty() || importance < cfg.min_importance || lengths[ri] < cfg.min_segment_px {
            continue;
        }
        let step = cfg
            .spacing_px
            .get(importance as usize)
            .copied()
            .unwrap_or(0.0);
        let total = lengths[ri];
        let n = if step <= 0.0 {
            1
        } else {
            ((total / step).floor() as usize).max(1)
        };
        let gap = total / n as f32;
        let mut visible_to = cfg
            .max_distance_by_importance
            .get(importance as usize)
            .copied()
            .unwrap_or(0.1)
            * extent;
        if cfg.major_names.contains(&river.name) || cfg.major_names.contains(&text) {
            visible_to = visible_to.max(major_distance);
        }
        let clearance = step.max(gap) * SAME_NAME_CLEARANCE;
        for k in 0..n {
            let at = gap * (k as f32 + 0.5);
            let px = sample(&river.points, at);
            if in_zone(&data.zones, px, cfg.clear_px_from_settlements) {
                continue;
            }
            let previous = taken.entry(text.clone()).or_default();
            if previous.iter().any(|q| q.distance(px) < clearance) {
                continue;
            }
            previous.push(px);
            let mut dir = tangent(&river.points, at, total);
            if dir.x < 0.0 {
                dir = -dir;
            }
            labels.push(RiverLabel {
                text: text.clone(),
                px,
                dir,
                importance,
                visible_to,
            });
            if labels.len() >= cfg.max_labels {
                break;
            }
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> WaterData {
        let mut data = WaterData {
            map_size: Vec2::new(1000.0, 1000.0),
            ..default()
        };
        data.display.labels.enabled = true;
        data.display.labels.spacing_px = vec![0.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0];
        data.display.labels.max_distance_by_importance = vec![0.05; 7];
        data.river_names.insert("Loire".into(), "Loire".into());
        data.rivers.push(River {
            name: "Loire".into(),
            importance: 4,
            points: vec![Vec2::new(400.0, 0.0), Vec2::new(0.0, 0.0)],
            widths: vec![0.3, 0.3],
        });
        data
    }

    #[test]
    fn spaces_labels_and_reads_left_to_right() {
        let labels = place(&data());
        assert_eq!(labels.len(), 4);
        assert!(labels.iter().all(|l| l.dir.x > 0.0 && l.text == "Loire"));
        assert_eq!(labels[0].visible_to, 50.0);
    }

    #[test]
    fn zones_and_disabled_config_hide_labels() {
        let mut d = data();
        d.zones.push(Zone {
            id: "z".into(),
            px: Vec2::new(200.0, 0.0),
            radius_px: 1000.0,
        });
        assert!(place(&d).is_empty());
        let mut d = data();
        d.display.labels.enabled = false;
        assert!(place(&d).is_empty());
    }

    #[test]
    fn hidden_name_is_skipped() {
        let mut d = data();
        d.river_names.insert("Loire".into(), String::new());
        assert!(place(&d).is_empty());
    }
}
