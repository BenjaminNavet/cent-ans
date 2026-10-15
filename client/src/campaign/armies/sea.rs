//! Sea voyages from a port (ADR 0167): routes along `data/map/sea_lanes_px.json`, click on the
//! water, refusal texts, and the glide of the army along the route (`SeaLaneLayer.lane_points`,
//! `army_movement_controller.gd` `order_embark` / `sea_route_points` / `_water_refusal`).
//! Naval battles are never played in 3D: the core fights an interception itself
//! (`sim-campaign::naval`); a pending one of the player is auto-resolved here
//! (`CampaignSim.auto_resolve_naval_battle`), see [`auto_resolve_pending`].

use std::collections::HashMap;

use bevy::prelude::*;
use data_model::{GameData, SettlementId};
use sim_campaign::{ArmyId, CampaignState};

use crate::campaign::terrain::surface::Surface;
use crate::paths;
use crate::sim::Sim;

/// Map pixels per second of the glide along a voyage.
pub const GLIDE_SPEED: f32 = 140.0;
pub const GLIDE_MIN_SECONDS: f32 = 1.5;
pub const GLIDE_MAX_SECONDS: f32 = 6.0;

/// Refusal when the army is in the field (`_water_refusal`).
pub const REFUSAL_NOT_IN_PORT: &str =
    "Pour prendre la mer, l’armée doit d’abord entrer dans un port.";
pub const REFUSAL_NO_LANE: &str = "Aucune route maritime depuis ce port.";

/// Lanes of the sea, in map pixels, from `data/map/sea_lanes_px.json`.
#[derive(Resource, Default, Debug, Clone)]
pub struct SeaLanes {
    lanes: Vec<(String, String, Vec<Vec2>)>,
}

impl SeaLanes {
    pub fn parse(text: &str) -> SeaLanes {
        let Ok(root) = serde_json::from_str::<serde_json::Value>(text) else {
            return SeaLanes::default();
        };
        let lanes = root
            .get("lanes")
            .and_then(|l| l.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|lane| {
                        let from = lane.get("from")?.as_str()?.to_owned();
                        let to = lane.get("to")?.as_str()?.to_owned();
                        let points: Vec<Vec2> = lane
                            .get("points")?
                            .as_array()?
                            .iter()
                            .filter_map(|p| {
                                Some(Vec2::new(
                                    p.get(0)?.as_f64()? as f32,
                                    p.get(1)?.as_f64()? as f32,
                                ))
                            })
                            .collect();
                        (points.len() >= 2).then_some((from, to, points))
                    })
                    .collect()
            })
            .unwrap_or_default();
        SeaLanes { lanes }
    }

    pub fn len(&self) -> usize {
        self.lanes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lanes.is_empty()
    }

    /// Points of the lane `from` -> `to` (either way round), `None` when no lane joins them.
    pub fn lane_points(&self, from: &str, to: &str) -> Option<Vec<Vec2>> {
        self.lanes.iter().find_map(|(a, b, points)| {
            if a == from && b == to {
                Some(points.clone())
            } else if a == to && b == from {
                Some(points.iter().rev().copied().collect())
            } else {
                None
            }
        })
    }
}

pub fn load_sea_lanes(mut commands: Commands) {
    let lanes = paths::find_root()
        .ok()
        .and_then(|root| {
            std::fs::read_to_string(paths::data_dir(&root).join("map/sea_lanes_px.json")).ok()
        })
        .map(|text| SeaLanes::parse(&text))
        .unwrap_or_default();
    commands.insert_resource(lanes);
}

/// Route of a voyage: the lane of each leg, or a straight line when a leg has no lane
/// (`sea_route_points`). `point_of` gives the map pixel of a settlement.
pub fn route_points(
    lanes: &SeaLanes,
    point_of: &dyn Fn(&str) -> Option<Vec2>,
    from: &str,
    stops: &[String],
) -> Vec<Vec2> {
    let mut route: Vec<Vec2> = Vec::new();
    let mut leg_from = from;
    for leg_to in stops {
        let leg = lanes
            .lane_points(leg_from, leg_to)
            .or_else(|| Some(vec![point_of(leg_from)?, point_of(leg_to)?]))
            .unwrap_or_default();
        let skip = usize::from(!route.is_empty());
        route.extend(leg.into_iter().skip(skip));
        leg_from = leg_to;
    }
    route
}

/// Length of a polyline.
pub fn route_length(route: &[Vec2]) -> f32 {
    route.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Position and heading (unit direction) at `distance` along the polyline.
pub fn sample_route(route: &[Vec2], distance: f32) -> Option<(Vec2, Vec2)> {
    let mut left = distance.max(0.0);
    let mut last_dir = Vec2::X;
    for pair in route.windows(2) {
        let length = pair[0].distance(pair[1]);
        let dir = (pair[1] - pair[0]).normalize_or_zero();
        if dir != Vec2::ZERO {
            last_dir = dir;
        }
        if left <= length && length > 0.0 {
            return Some((pair[0].lerp(pair[1], left / length), last_dir));
        }
        left -= length;
    }
    route.last().map(|p| (*p, last_dir))
}

/// An army sliding along its voyage (cosmetic: the core already moved it).
#[derive(Debug, Clone)]
pub struct Glide {
    pub route: Vec<Vec2>,
    pub length: f32,
    pub elapsed: f32,
    pub duration: f32,
}

impl Glide {
    pub fn new(route: Vec<Vec2>) -> Option<Glide> {
        let length = route_length(&route);
        if route.len() < 2 || length <= 0.0 {
            return None;
        }
        Some(Glide {
            duration: (length / GLIDE_SPEED).clamp(GLIDE_MIN_SECONDS, GLIDE_MAX_SECONDS),
            route,
            length,
            elapsed: 0.0,
        })
    }

    /// Position and heading now.
    pub fn pose(&self) -> Option<(Vec2, Vec2)> {
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        // Ease in and out: ships do not start at full speed.
        let eased = t * t * (3.0 - 2.0 * t);
        sample_route(&self.route, self.length * eased)
    }

    pub fn done(&self) -> bool {
        self.elapsed >= self.duration
    }
}

#[derive(Resource, Default)]
pub struct Glides {
    pub active: HashMap<ArmyId, Glide>,
    /// Voyages started / finished since the campaign screen opened.
    pub started: u32,
    pub finished: u32,
}

/// Is `(x, z)` open water (`MapData.is_land_px`: no land mask here, height <= 0 as the Godot
/// fallback does)?
pub fn is_water(surface: &Surface, x: f32, z: f32) -> bool {
    surface.heights.height_m_at(x, z) <= 0.0
}

/// What a right click on the water does for an army.
#[derive(Debug, Clone, PartialEq)]
pub enum SeaClick {
    /// Sail to this port.
    Embark(SettlementId),
    Refused(&'static str),
}

/// Right click on the water at `point` (`sea_port_at` + `_water_refusal`).
pub fn sea_click(
    state: &CampaignState,
    data: &GameData,
    army: &ArmyId,
    point: [f32; 2],
) -> SeaClick {
    let Some(entry) = state.army(army) else {
        return SeaClick::Refused(REFUSAL_NOT_IN_PORT);
    };
    let Some(from) = entry.settlement() else {
        return SeaClick::Refused(REFUSAL_NOT_IN_PORT);
    };
    match sim_campaign::voyage::port_near(state, data, &entry.faction, from, point) {
        Some(port) => SeaClick::Embark(port),
        None => SeaClick::Refused(REFUSAL_NO_LANE),
    }
}

/// Ports of call of the voyage of `army` to `port` (destination last).
pub fn voyage_stops(
    state: &CampaignState,
    data: &GameData,
    army: &ArmyId,
    port: &SettlementId,
) -> Vec<SettlementId> {
    let Some(entry) = state.army(army) else {
        return Vec::new();
    };
    let Some(from) = entry.settlement() else {
        return Vec::new();
    };
    sim_campaign::voyage::sea_voyage(state, data, &entry.faction, from, port).unwrap_or_default()
}

/// Hover text of a voyage preview (`_preview_voyage`).
pub fn voyage_hint(data: &GameData, port: &SettlementId, stops: &[SettlementId]) -> String {
    let calls: Vec<String> = stops
        .iter()
        .take(stops.len().saturating_sub(1))
        .map(|s| data.settlement_name(s))
        .collect();
    let mut text = format!(
        "⚓ → {} : traversée, toute la saison",
        data.settlement_name(port)
    );
    if !calls.is_empty() {
        text.push_str(&format!(" (escales : {})", calls.join(", ")));
    }
    text + " — clic droit pour embarquer"
}

/// Naval battles never play out in 3D: every pending battle of the player is auto-resolved
/// (`auto_resolve_naval_battle`). Returns the number resolved.
pub fn auto_resolve_pending(sim: &mut Sim) -> usize {
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        return 0;
    };
    let mut resolved = 0;
    while !state.naval.pending.is_empty() && resolved < 64 {
        let before = state.naval.pending.len();
        let _ = state.auto_resolve_naval_battle(&data, 0);
        if state.naval.pending.len() >= before {
            // An error that left the queue untouched: drop the stale head.
            state.naval.pending.remove(0);
        }
        resolved += 1;
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lanes() -> SeaLanes {
        SeaLanes::parse(
            r#"{"lanes":[{"id":"a","from":"set_a","to":"set_b","points":[[0,0],[10,0],[10,10]]}]}"#,
        )
    }

    #[test]
    fn lane_either_way() {
        let l = lanes();
        assert_eq!(l.len(), 1);
        assert_eq!(l.lane_points("set_a", "set_b").unwrap().len(), 3);
        let back = l.lane_points("set_b", "set_a").unwrap();
        assert_eq!(back[0], Vec2::new(10.0, 10.0));
        assert!(l.lane_points("set_a", "set_c").is_none());
    }

    #[test]
    fn route_joins_legs_and_falls_back_to_a_line() {
        let l = lanes();
        let point_of = |id: &str| match id {
            "set_b" => Some(Vec2::new(10.0, 10.0)),
            "set_c" => Some(Vec2::new(10.0, 30.0)),
            _ => None,
        };
        let route = route_points(
            &l,
            &point_of,
            "set_a",
            &["set_b".to_owned(), "set_c".to_owned()],
        );
        assert_eq!(route.len(), 4);
        assert_eq!(route.last(), Some(&Vec2::new(10.0, 30.0)));
        assert!((route_length(&route) - 40.0).abs() < 1e-4);
    }

    #[test]
    fn sampling_follows_the_polyline() {
        let route = vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)];
        let (p, d) = sample_route(&route, 15.0).unwrap();
        assert!((p - Vec2::new(10.0, 5.0)).length() < 1e-4);
        assert_eq!(d, Vec2::Y);
        assert_eq!(sample_route(&route, 99.0).unwrap().0, Vec2::new(10.0, 10.0));
    }

    #[test]
    fn glide_reaches_the_end() {
        let mut g = Glide::new(vec![Vec2::ZERO, Vec2::new(300.0, 0.0)]).unwrap();
        assert!(!g.done());
        assert_eq!(g.pose().unwrap().0, Vec2::ZERO);
        g.elapsed = g.duration;
        assert!(g.done());
        assert!((g.pose().unwrap().0 - Vec2::new(300.0, 0.0)).length() < 1e-3);
        assert!(Glide::new(vec![Vec2::ZERO]).is_none());
    }
}
