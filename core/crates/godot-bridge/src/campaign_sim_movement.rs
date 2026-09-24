//! `CampaignSim` free army movement (lot M4, spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 6): the reachable
//! "bubble" as a mask image, the turn-by-turn path preview, and the move,
//! attack and embark orders with the march they caused.

use data_model::SettlementId;
use godot::classes::image::Format;
use godot::classes::Image;
use godot::prelude::*;
use serde_json::Value;
use sim_campaign::{ArmyId, CampaignState, MoveReport, Order, OrderOutcome};

use crate::campaign_sim::{events_array, CampaignSim};
use crate::convert::variant_to_json;

/// Empty cells kept around the bubble in the mask (smooth contour).
const MASK_MARGIN: u32 = 2;

#[godot_api(secondary)]
impl CampaignSim {
    /// Cells the army can reach this turn, as a mask cropped to their
    /// bounding box: `{image, origin, size, cell_px, cells, budget}`.
    /// `image` is an RG8 image of one texel per grid cell: R = 255 inside
    /// the bubble, G = cost / budget × 255. `origin` and `size` are the map
    /// pixels covered (top-left corner, extent). Empty dictionary when the
    /// army is unknown.
    #[func]
    fn get_reachable_area(&self, army_id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return VarDictionary::new();
        };
        let Some(entry) = state.army(&army) else {
            return VarDictionary::new();
        };
        let grid = data.navgrid();
        let area = state.reachable_area(data, &army);
        let start = state.army_cell(data, entry);
        let (mut x0, mut y0, mut x1, mut y1) = (
            u32::from(start.x),
            u32::from(start.y),
            u32::from(start.x),
            u32::from(start.y),
        );
        for (cell, _) in &area {
            x0 = x0.min(u32::from(cell.x));
            y0 = y0.min(u32::from(cell.y));
            x1 = x1.max(u32::from(cell.x));
            y1 = y1.max(u32::from(cell.y));
        }
        let x0 = x0.saturating_sub(MASK_MARGIN);
        let y0 = y0.saturating_sub(MASK_MARGIN);
        let x1 = (x1 + MASK_MARGIN).min(grid.width.saturating_sub(1));
        let y1 = (y1 + MASK_MARGIN).min(grid.height.saturating_sub(1));
        let width = x1 - x0 + 1;
        let height = y1 - y0 + 1;
        let budget = entry.movement_left.max(1);
        let mut bytes = vec![0u8; (width * height * 2) as usize];
        for (cell, cost) in &area {
            let index =
                (((u32::from(cell.y) - y0) * width + (u32::from(cell.x) - x0)) * 2) as usize;
            bytes[index] = 255;
            bytes[index + 1] = ((u64::from(*cost) * 255) / u64::from(budget)).min(255) as u8;
        }
        close_thin_gaps(&mut bytes, width as usize, height as usize);
        let packed = PackedByteArray::from(bytes.as_slice());
        let Some(image) =
            Image::create_from_data(width as i32, height as i32, false, Format::RG8, &packed)
        else {
            return VarDictionary::new();
        };
        let scale = grid.scale as f32;
        vdict! {
            "image" => &image,
            "origin" => Vector2::new(x0 as f32 * scale, y0 as f32 * scale),
            "size" => Vector2::new(width as f32 * scale, height as f32 * scale),
            "cell_px" => scale,
            "cells" => area.len() as i64,
            "budget" => i64::from(entry.movement_left),
        }
    }

    /// Movement constants for the map: `{px_per_km, zoc_radius_km,
    /// zoc_radius_px, engage_radius_km, engage_radius_px}` (zone of control
    /// ring shown over enemy armies). Empty before the data is loaded.
    #[func]
    fn get_movement_rules(&self) -> VarDictionary {
        let Some(data) = &self.data else {
            return VarDictionary::new();
        };
        let rules = data.free_movement_rules();
        let px_per_km = data.navgrid().px_per_km();
        vdict! {
            "px_per_km" => px_per_km,
            "zoc_radius_km" => rules.zoc_radius_km,
            "zoc_radius_px" => rules.zoc_radius_km * px_per_km,
            "engage_radius_km" => rules.engage_radius_km,
            "engage_radius_px" => rules.engage_radius_km * px_per_km,
        }
    }

    /// Turn-by-turn preview of a march to map pixel `(x, y)`:
    /// `{ok, points, cost, cost_this_turn, stop_index, turn_ends, turns,
    /// reachable_this_turn}`. `points` is the polyline in map pixels (the
    /// army's point first); the march stops this turn at
    /// `points[stop_index]`, and each later turn at the next `turn_ends`.
    /// `{ok: false}` when unreachable.
    #[func]
    fn find_path_points(&self, army_id: GString, x: f64, y: f64) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return vdict! { "ok" => false };
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return vdict! { "ok" => false };
        };
        let Some(plan) = state.plan_path(data, &army, [x as f32, y as f32]) else {
            return vdict! { "ok" => false };
        };
        let points: PackedVector2Array = plan
            .points
            .iter()
            .map(|p| Vector2::new(p[0], p[1]))
            .collect();
        let turn_ends: PackedInt32Array = plan.turn_ends.iter().map(|i| *i as i32).collect();
        vdict! {
            "ok" => true,
            "points" => &points,
            "cost" => i64::from(plan.cost),
            "cost_this_turn" => i64::from(plan.cost_this_turn),
            "stop_index" => plan.stop_index() as i64,
            "turn_ends" => &turn_ends,
            "turns" => plan.turns() as i64,
            "reachable_this_turn" => plan.reachable_this_turn(),
        }
    }

    /// Order `move_army` to map pixel `(x, y)`, executed at once; returns the
    /// march report (see `submit_order_report`).
    #[func]
    fn move_army_to(&mut self, army_id: GString, x: f64, y: f64) -> VarDictionary {
        let order = serde_json::json!({
            "type": "move_army",
            "army": army_id.to_string(),
            "target": {"x": x, "y": y},
        });
        self.run_order_json(order)
    }

    /// Order `move_army` to a settlement (march, stop, siege or capture).
    #[func]
    fn move_army_to_settlement(&mut self, army_id: GString, settlement: GString) -> VarDictionary {
        let order = serde_json::json!({
            "type": "move_army",
            "army": army_id.to_string(),
            "target": settlement.to_string(),
        });
        self.run_order_json(order)
    }

    /// Order `attack`: close in on `target_army` and fight it at once.
    #[func]
    fn attack_army(&mut self, army_id: GString, target_army: GString) -> VarDictionary {
        let order = serde_json::json!({
            "type": "attack",
            "army": army_id.to_string(),
            "target_army": target_army.to_string(),
        });
        self.run_order_json(order)
    }

    /// Order `embark`: sea crossing from the army's port to `to_port`.
    #[func]
    fn embark_army(&mut self, army_id: GString, to_port: GString) -> VarDictionary {
        let order = serde_json::json!({
            "type": "embark",
            "army": army_id.to_string(),
            "to_port": to_port.to_string(),
        });
        self.run_order_json(order)
    }

    /// Test and capture hook (not a game rule): puts `army_id` in the field
    /// at map pixel `(x, y)`. `false` when the army is unknown.
    #[func]
    fn debug_place_army(&mut self, army_id: GString, x: f64, y: f64) -> bool {
        let Some(state) = &mut self.state else {
            return false;
        };
        let Some(army) =
            ArmyId::parse(&army_id.to_string()).and_then(|id| state.armies.get_mut(&id))
        else {
            return false;
        };
        army.position = sim_campaign::ArmyPosition::field([x as f32, y as f32]);
        army.clear_plan();
        true
    }

    /// Like `submit_order`, with what the order did: `{ok, error, army,
    /// walked, cost, stop, stop_settlement, stop_army, planned_path,
    /// position, settlement, events}`. `walked` is the polyline walked in
    /// map pixels, start included (animation); `stop` the reason the march
    /// stopped (`arrived`, `stationed`, `siege_started`, `settlement_taken`,
    /// `out_of_movement`, `enemy_zone_of_control`, `enemy_settlement`,
    /// `blocked`, `engaged`, or "" for an order without a march); `events`
    /// the events the order raised (battles, sieges, captures).
    #[func]
    fn submit_order_report(&mut self, order: VarDictionary) -> VarDictionary {
        match variant_to_json(&order.to_variant()) {
            Ok(json) => self.run_order_json(json),
            Err(error) => failure(&format!("ordre invalide : {error}")),
        }
    }
}

impl CampaignSim {
    fn run_order_json(&mut self, json: Value) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return failure("aucune campagne en cours");
        };
        let order = match serde_json::from_value::<Order>(json) {
            Ok(order) => order,
            Err(error) => return failure(&format!("ordre invalide : {error}")),
        };
        let army = order_army(&order);
        let start = army
            .as_ref()
            .and_then(|id| state.army(id))
            .map(|a| state.army_point(data, a));
        let events_before = state.pending_events.len();
        let outcome = match state.submit_order_outcome(data, order) {
            Ok(outcome) => outcome,
            Err(error) => return failure(&error.to_string()),
        };
        let new_events = state
            .pending_events
            .get(events_before..)
            .map(events_array)
            .unwrap_or_default();
        let mut dict = match &outcome {
            OrderOutcome::Moved(report) => report_dict(state, data, report, start),
            OrderOutcome::Done => done_dict(state, data, army.as_ref(), start),
        };
        dict.set("events", &new_events);
        dict
    }
}

/// Display only: fills the gaps of at most `THIN_GAP` cells between two
/// reachable cells of a row or a column (rivers crossed by a bridge further
/// on), so that the bubble reads as one area instead of being striped by
/// every river. `bytes` is the RG8 mask of `get_reachable_area`.
fn close_thin_gaps(bytes: &mut [u8], width: usize, height: usize) {
    const THIN_GAP: usize = 2;
    let inside = |bytes: &[u8], x: usize, y: usize| bytes[(y * width + x) * 2] == 255;
    let mut fills: Vec<(usize, u8)> = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if inside(bytes, x, y) {
                continue;
            }
            let mut best: Option<u8> = None;
            for (dx, dy) in [(1usize, 0usize), (0, 1)] {
                for before in 1..=THIN_GAP {
                    for after in 1..=THIN_GAP {
                        if before + after > THIN_GAP + 1 {
                            continue;
                        }
                        let (Some(bx), Some(by)) =
                            (x.checked_sub(before * dx), y.checked_sub(before * dy))
                        else {
                            continue;
                        };
                        let (ax, ay) = (x + after * dx, y + after * dy);
                        if ax >= width || ay >= height {
                            continue;
                        }
                        if inside(bytes, bx, by) && inside(bytes, ax, ay) {
                            let cost = bytes[(by * width + bx) * 2 + 1]
                                .max(bytes[(ay * width + ax) * 2 + 1]);
                            best = Some(best.map_or(cost, |b| b.min(cost)));
                        }
                    }
                }
            }
            if let Some(cost) = best {
                fills.push(((y * width + x) * 2, cost));
            }
        }
    }
    for (index, cost) in fills {
        bytes[index] = 255;
        bytes[index + 1] = cost;
    }
}

/// The army an order moves, if any.
fn order_army(order: &Order) -> Option<ArmyId> {
    match order {
        Order::MoveArmy { army, .. } | Order::Attack { army, .. } | Order::Embark { army, .. } => {
            Some(army.clone())
        }
        _ => None,
    }
}

fn failure(error: &str) -> VarDictionary {
    vdict! {
        "ok" => false,
        "error" => error,
        "walked" => &PackedVector2Array::new(),
        "stop" => "",
    }
}

fn to_vec2(point: [f32; 2]) -> Vector2 {
    Vector2::new(point[0], point[1])
}

/// Current position fields of `army` (after the order).
fn position_fields(
    dict: &mut VarDictionary,
    state: &CampaignState,
    data: &data_model::GameData,
    army: Option<&ArmyId>,
) {
    let entry = army.and_then(|id| state.army(id));
    dict.set("alive", entry.is_some());
    let Some(entry) = entry else {
        return;
    };
    dict.set("position", to_vec2(state.army_point(data, entry)));
    dict.set(
        "settlement",
        entry.settlement().map_or("", SettlementId::as_str),
    );
    dict.set("movement_left", i64::from(entry.movement_left));
}

fn report_dict(
    state: &CampaignState,
    data: &data_model::GameData,
    report: &MoveReport,
    start: Option<[f32; 2]>,
) -> VarDictionary {
    let mut walked = PackedVector2Array::new();
    if let Some(start) = start {
        walked.push(to_vec2(start));
    }
    for point in &report.walked {
        let point = to_vec2(*point);
        if walked.as_slice().last() != Some(&point) {
            walked.push(point);
        }
    }
    let grid = data.navgrid();
    let planned: PackedVector2Array = report
        .planned_path
        .iter()
        .map(|cell| to_vec2(cell.center(grid)))
        .collect();
    let stop = serde_json::to_value(&report.stop).unwrap_or(Value::Null);
    let field = |key: &str| {
        stop.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut dict = vdict! {
        "ok" => true,
        "error" => "",
        "army" => report.army.as_str(),
        "walked" => &walked,
        "cost" => i64::from(report.cost),
        "stop" => field("kind").as_str(),
        "stop_settlement" => field("settlement").as_str(),
        "stop_army" => field("army").as_str(),
        "planned_path" => &planned,
    };
    position_fields(&mut dict, state, data, Some(&report.army));
    dict
}

fn done_dict(
    state: &CampaignState,
    data: &data_model::GameData,
    army: Option<&ArmyId>,
    start: Option<[f32; 2]>,
) -> VarDictionary {
    let mut walked = PackedVector2Array::new();
    if let (Some(start), Some(entry)) = (start, army.and_then(|id| state.army(id))) {
        let end = state.army_point(data, entry);
        if end != start {
            walked.push(to_vec2(start));
            walked.push(to_vec2(end));
        }
    }
    let mut dict = vdict! {
        "ok" => true,
        "error" => "",
        "army" => army.map_or("", |a| a.as_str()),
        "walked" => &walked,
        "cost" => 0,
        "stop" => "",
        "stop_settlement" => "",
        "stop_army" => "",
        "planned_path" => &PackedVector2Array::new(),
    };
    position_fields(&mut dict, state, data, army);
    dict
}
