//! Layout probe for the dump section `kit_layout`: for every kit frame (panel, illuminated page,
//! note, inset, card, scroll list), the computed frame rect against the rects of its direct
//! children, so that padding bugs show up as numbers instead of screenshots.
//!
//! `inner_left` = distance between the frame's left edge and its leftmost child; `ok` is true when
//! it is at least the texture content margin (`required`) and no child pokes out of the frame.

use bevy::prelude::*;
use bevy::text::TextLayoutInfo;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use serde_json::{Value, json};

use super::UiTheme;
use super::widgets::KitWidget;
use bevy::ui_widgets::{Scrollbar, ScrollbarThumb};

/// Texture name whose content margin a frame kind must respect.
pub fn frame_texture(kind: &str) -> Option<&'static str> {
    match kind {
        "panel" => Some("panel"),
        "panel_illuminated" => Some("panel_illuminated"),
        "panel_note" | "tooltip_view" => Some("tooltip"),
        "panel_inset" | "scroll_list" => Some("inset"),
        _ => None,
    }
}

/// Rect `(left, top, right, bottom)` of a UI node in logical pixels.
fn rect(node: &ComputedNode, transform: &UiGlobalTransform) -> [f32; 4] {
    let center = transform.translation * node.inverse_scale_factor;
    let half = node.size() * node.inverse_scale_factor * 0.5;
    [
        center.x - half.x,
        center.y - half.y,
        center.x + half.x,
        center.y + half.y,
    ]
}

pub fn dump(world: &World) -> Value {
    let theme = world.resource::<UiTheme>();
    let mut frames = Vec::new();
    let mut failures = 0;
    for entity in world.iter_entities() {
        let Some(kind) = entity.get::<KitWidget>() else {
            continue;
        };
        let is_card = kind.0 == "card";
        let (Some(node), Some(transform)) = (
            entity.get::<ComputedNode>(),
            entity.get::<UiGlobalTransform>(),
        ) else {
            continue;
        };
        let required = match frame_texture(kind.0) {
            Some(texture) => {
                let metric = theme.metric(texture);
                [0, 1, 2, 3].map(|i| metric.content[i].max(metric.margins[i]))
            }
            None if is_card => [1.0; 4],
            None => continue,
        };
        let frame = rect(node, transform);
        let mut inner = [f32::MAX, f32::MAX, f32::MAX, f32::MAX];
        let mut children = 0;
        let mut rects = Vec::new();
        if let Some(list) = entity.get::<Children>() {
            for child in list.iter() {
                let Some(child_entity) = world.get_entity(child).ok() else {
                    continue;
                };
                let (Some(cn), Some(ct)) = (
                    child_entity.get::<ComputedNode>(),
                    child_entity.get::<UiGlobalTransform>(),
                ) else {
                    continue;
                };
                if cn.size() == Vec2::ZERO || cn.is_empty() && cn.size().x < 1.0 {
                    continue; // hidden tab pane / display:none
                }
                if child_entity.contains::<Scrollbar>()
                    || child_entity.contains::<super::resize_grip::ResizeGrip>()
                {
                    continue; // absolute overlay
                }
                let r = rect(cn, ct);
                inner[0] = inner[0].min(r[0] - frame[0]);
                inner[1] = inner[1].min(r[1] - frame[1]);
                inner[2] = inner[2].min(frame[2] - r[2]);
                inner[3] = inner[3].min(frame[3] - r[3]);
                children += 1;
                rects.push(json!([
                    r[0] - frame[0],
                    r[1] - frame[1],
                    r[2] - r[0],
                    r[3] - r[1]
                ]));
            }
        }
        // A scroll list's content overflows by design: only the left/top/right sides count.
        let sides = if kind.0 == "scroll_list" { 3 } else { 4 };
        let ok = children == 0 || (0..sides).all(|side| inner[side] >= required[side] - 0.5);
        if !ok {
            failures += 1;
        }
        let name = entity
            .get::<Name>()
            .map_or_else(|| "-".to_owned(), |name| name.to_string());
        frames.push(json!({
            "kind": kind.0,
            "name": name,
            "frame": frame,
            "size": [frame[2] - frame[0], frame[3] - frame[1]],
            "required": required,
            "inner_ltrb": if children == 0 { Value::Null } else { json!(inner) },
            "children": children,
            "child_rects_xywh": rects,
            "ok": ok,
        }));
    }
    let mut scrollbars = Vec::new();
    for entity in world.iter_entities() {
        let (Some(bar), Some(node)) = (entity.get::<Scrollbar>(), entity.get::<ComputedNode>())
        else {
            continue;
        };
        let target = world.get_entity(bar.target).ok();
        let target_node = target.as_ref().and_then(|t| t.get::<ComputedNode>());
        let thumb = entity.get::<Children>().and_then(|list| {
            list.iter().find_map(|child| {
                let e = world.get_entity(child).ok()?;
                e.get::<ScrollbarThumb>()?;
                let cn = e.get::<ComputedNode>()?;
                Some(cn.size() * cn.inverse_scale_factor)
            })
        });
        scrollbars.push(json!({
            "track": node.size() * node.inverse_scale_factor,
            "thumb": thumb,
            "visible": target_node.map(|n| n.size() * n.inverse_scale_factor),
            "content": target_node.map(|n| n.content_size() * n.inverse_scale_factor),
        }));
    }
    let mut texts = Vec::new();
    for entity in world.iter_entities() {
        let (Some(text), Some(info), Some(node)) = (
            entity.get::<Text>(),
            entity.get::<TextLayoutInfo>(),
            entity.get::<ComputedNode>(),
        ) else {
            continue;
        };
        if texts.len() >= 12 {
            break;
        }
        let first = info
            .glyphs
            .first()
            .map(|g| g.position.x * node.inverse_scale_factor);
        texts.push(json!({
            "text": text.0.chars().take(14).collect::<String>(),
            "node_w": node.size().x * node.inverse_scale_factor,
            "layout_w": info.size.x * node.inverse_scale_factor,
            "first_glyph_x": first,
            "glyphs": info.glyphs.len(),
        }));
    }
    json!({ "frames": frames, "failures": failures, "scrollbars": scrollbars, "texts": texts })
}
