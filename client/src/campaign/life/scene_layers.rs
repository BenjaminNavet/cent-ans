//! Staging of the province scenes by layers (`life_folk/folk_scene_layers.gd`, SC MC6): each kind
//! of scene of `data/rules/folk_scene_layouts.json` is a list of layers of four parameterised
//! archetypes (procession, convoy, scatter, site). Pure: the layers are interpreted into
//! [`Item`]s, the spawning is `scenes.rs`.
//!
//! Positions of the data are `[ahead, along]` in metres from the anchor of the frame (ahead =
//! away from the settlement, along = to the right of ahead). Yaws follow `FolkSceneLayers`: the
//! heading of a figure that looks along the map vector `d` is `atan2(d.x, d.y)`.

use std::f32::consts::TAU;

use bevy::prelude::*;
use serde_json::Value;

use super::data::h01;

/// Frame of a scene: `anchor` at the edge of the settlement, `out` away from it, `along` to the
/// right of `out`.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub anchor: Vec2,
    pub out: Vec2,
    pub along: Vec2,
    pub seed: u64,
    pub center: Vec2,
    pub index: usize,
}

/// One thing to place.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Walks back and forth between `from` and `to` (a figure, or a vehicle that rolls).
    Mover {
        model: String,
        activity: String,
        from: Vec2,
        to: Vec2,
        phase: f32,
        lateral_m: f32,
        behind_m: f32,
    },
    /// Stands still at `at`, facing `yaw`.
    Static {
        model: String,
        activity: String,
        at: Vec2,
        yaw: f32,
    },
}

/// What the stager asks of the world.
pub trait Env {
    /// Map units per metre of the scene.
    fn scale(&self) -> f32;
    /// Slot of a figure around a prop (`FolkModels.slot`).
    fn slot(&self, prop: &str, slot: &str, fallback: Vec2) -> Vec2;
    fn escape_path(
        &mut self,
        index: usize,
        group: usize,
        from: Vec2,
        dir: Vec2,
        salt: u64,
    ) -> Vec<Vec2>;
    fn river_points(
        &mut self,
        index: usize,
        center: Vec2,
        range: f32,
        count: usize,
        spacing: f32,
    ) -> Vec<Vec2>;
    fn footprint(&self, index: usize) -> f32;
}

pub struct Stager<'a, E: Env> {
    pub env: &'a mut E,
    pub items: Vec<Item>,
    /// Items still allowed (the pool budget).
    pub budget: usize,
}

fn hh(seed: u64, salt: i64) -> f32 {
    h01(seed, salt as u64, 0)
}

fn num(v: &Value, k: &str, d: f32) -> f32 {
    v[k].as_f64().map_or(d, |x| x as f32)
}

fn point(v: &Value) -> [f32; 2] {
    [
        v[0].as_f64().unwrap_or(0.0) as f32,
        v[1].as_f64().unwrap_or(0.0) as f32,
    ]
}

fn vec2(v: &Value) -> Vec2 {
    let p = point(v);
    Vec2::new(p[0], p[1])
}

/// floor(base + per_n x figures), bounded (`FolkSceneLayers.count_of`).
pub fn count_of(spec: &Value, figures: usize) -> usize {
    if let Some(n) = spec["fixed"].as_i64() {
        return n.max(0) as usize;
    }
    let value = (num(spec, "base", 0.0) + num(spec, "per_n", 0.0) * figures as f32).floor();
    let lo = spec["min"].as_i64().unwrap_or(0) as f32;
    let hi = spec["max"].as_i64().map_or(f32::MAX, |m| m as f32);
    value.clamp(lo, hi.max(lo)) as usize
}

fn yaw_of_dir(d: Vec2) -> f32 {
    d.x.atan2(d.y)
}

fn right(d: Vec2) -> Vec2 {
    Vec2::new(-d.y, d.x)
}

/// Direction of a token (`along`, `-out*row`...); zero for `random`.
fn dir_of(frame: &Frame, token: &str, row: f32) -> Vec2 {
    let mut sign = 1.0;
    let mut t = token;
    if let Some(rest) = t.strip_prefix('-') {
        sign = -1.0;
        t = rest;
    }
    if let Some(rest) = t.strip_suffix("*row") {
        sign *= row;
        t = rest;
    }
    match t {
        "along" => frame.along * sign,
        "out" => frame.out * sign,
        _ => Vec2::ZERO,
    }
}

fn yaw_of(frame: &Frame, token: &str, row: f32, salt: i64) -> f32 {
    let d = dir_of(frame, token, row);
    if d == Vec2::ZERO {
        hh(frame.seed, salt) * TAU
    } else {
        yaw_of_dir(d)
    }
}

fn pick(roles: &Value, index: usize) -> &str {
    let list = roles.as_array();
    match list {
        Some(l) if !l.is_empty() => l[index % l.len()].as_str().unwrap_or("peasant"),
        _ => "peasant",
    }
}

impl<E: Env> Stager<'_, E> {
    fn m(&self, meters: f32) -> f32 {
        meters * self.env.scale()
    }

    fn at(&self, frame: &Frame, p: &Value, extra_out_m: f32, extra_along_m: f32) -> Vec2 {
        let p = point(p);
        frame.anchor
            + frame.out * self.m(p[0] + extra_out_m)
            + frame.along * self.m(p[1] + extra_along_m)
    }

    fn add(
        &mut self,
        model: &str,
        activity: &str,
        from: Vec2,
        to: Vec2,
        phase: f32,
        lateral_m: f32,
        behind_m: f32,
    ) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        self.items.push(Item::Mover {
            model: model.to_string(),
            activity: activity.to_string(),
            from,
            to,
            phase,
            lateral_m,
            behind_m,
        });
        true
    }

    fn add_static(&mut self, model: &str, activity: &str, at: Vec2, yaw: f32) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        self.items.push(Item::Static {
            model: model.to_string(),
            activity: activity.to_string(),
            at,
            yaw,
        });
        true
    }

    /// Places the layers of a scene kind (`layout` = `scenes.<kind>`).
    pub fn stage(&mut self, frame: &Frame, layout: &Value, figures: usize) {
        let river_range = layout["effects"]["river_cache"].as_f64().unwrap_or(0.0) as f32;
        for layer in layout["layers"].as_array().into_iter().flatten() {
            match layer["type"].as_str().unwrap_or("") {
                "procession" => self.procession(frame, layer, figures),
                "convoy" => self.convoy(frame, layer, figures),
                "scatter" => self.scatter(frame, layer, figures),
                "site" => self.site(frame, layer, figures, river_range),
                _ => {}
            }
        }
    }

    fn procession(&mut self, frame: &Frame, layer: &Value, figures: usize) {
        if layer["path"].as_str() == Some("escape") {
            self.escape_groups(frame, layer, figures);
            return;
        }
        let from = self.at(frame, &layer["from"], 0.0, 0.0);
        let to = self.at(frame, &layer["to"], 0.0, 0.0);
        let phase = hh(frame.seed, layer["phase_salt"].as_i64().unwrap_or(0));
        let banners = layer["banners"].as_i64().unwrap_or(0).max(0) as usize;
        if layer["cross"].as_bool().unwrap_or(false)
            && self.add("monk", "procession", from, to, phase, 0.0, 0.0)
        {
            self.add("procession_cross", "", from, to, phase, 0.35, 0.0);
        }
        for b in 0..banners {
            let behind = 1.6 * (b + 1) as f32;
            if self.add("monk", "procession", from, to, phase, 0.0, behind) {
                self.add("procession_banner", "", from, to, phase, 0.35, behind);
            }
        }
        let start = 1.6 * (banners + 1) as f32 + 0.8;
        let roles = &layer["roles"];
        for k in 0..count_of(&layer["walkers"], figures) {
            let lateral = -0.5 + (k % 2) as f32;
            if !self.add(
                pick(roles, k),
                "procession",
                from,
                to,
                phase,
                lateral,
                start + 1.2 * (k / 2) as f32,
            ) {
                break;
            }
        }
    }

    /// Refugees with bundles leaving the settlement in small groups along an escape path.
    fn escape_groups(&mut self, frame: &Frame, layer: &Value, figures: usize) {
        let groups = count_of(&layer["groups"], figures).max(1);
        let size_range = [
            layer["group_size"][0].as_i64().unwrap_or(2),
            layer["group_size"][1].as_i64().unwrap_or(4),
        ];
        let roles = &layer["roles"];
        let inner = frame.anchor.distance(frame.center) - self.m(6.0) * 0.5;
        let mut placed = 0;
        for g in 0..groups {
            let angle = hh(frame.seed, 5) * TAU
                + (g as f32 + hh(frame.seed, 10 + g as i64) * 0.5) * TAU / groups as f32;
            let dir = Vec2::new(angle.cos(), angle.sin());
            let path = self.env.escape_path(
                frame.index,
                g,
                frame.center + dir * inner,
                dir,
                frame.seed + g as u64,
            );
            if path.len() < 2 {
                continue;
            }
            let piece = g % (path.len() - 1);
            let phase = hh(frame.seed, 30 + g as i64);
            let span = (size_range[1] - size_range[0] + 1).max(1);
            let size = size_range[0] + (hh(frame.seed, 50 + g as i64) * span as f32) as i64;
            for k in 0..size.max(0) as usize {
                if placed >= figures {
                    return;
                }
                if !self.add(
                    pick(roles, k),
                    "procession",
                    path[piece],
                    path[piece + 1],
                    phase,
                    0.6 * (k % 2) as f32,
                    1.2 * k as f32,
                ) {
                    return;
                }
                placed += 1;
            }
        }
    }

    fn convoy(&mut self, frame: &Frame, layer: &Value, figures: usize) {
        let vehicle = layer["vehicle"].as_str().unwrap_or("dead_cart");
        let from_step = point(&layer["from_step"]);
        let to_step = point(&layer["to_step"]);
        let count = if layer["count"].is_object() {
            count_of(&layer["count"], figures)
        } else {
            1
        };
        for c in 0..count {
            let cf = c as f32;
            let from = self.at(frame, &layer["from"], from_step[0] * cf, from_step[1] * cf);
            let to = self.at(frame, &layer["to"], to_step[0] * cf, to_step[1] * cf);
            let phase = hh(
                frame.seed,
                layer["phase_salt"].as_i64().unwrap_or(0) + c as i64,
            );
            let activity = layer["activity"].as_str().unwrap_or("");
            if !self.add(vehicle, activity, from, to, phase, 0.0, 0.0) {
                continue;
            }
            for part in layer["attached"].as_array().into_iter().flatten() {
                let fallback = vec2(&part["default"]);
                let slot = self
                    .env
                    .slot(vehicle, part["slot"].as_str().unwrap_or(""), fallback);
                self.add(
                    part["role"].as_str().unwrap_or("peasant"),
                    part["activity"].as_str().unwrap_or("walk"),
                    from,
                    to,
                    phase,
                    slot.x,
                    -slot.y,
                );
            }
        }
    }

    fn scatter(&mut self, frame: &Frame, layer: &Value, figures: usize) {
        let count = count_of(&layer["count"], figures);
        match layer["pattern"].as_str().unwrap_or("") {
            "grid" => self.grid(frame, layer, count),
            "disc" => self.disc(frame, layer, count),
            "patrol" => {
                let from = self.at(frame, &layer["origin"], 0.0, 0.0);
                let to = from + frame.along * self.m(num(layer, "length_m", 0.0));
                let salt = layer["salt"].as_i64().unwrap_or(0);
                for k in 0..count {
                    self.add(
                        pick(&layer["roles"], k),
                        layer["activity"].as_str().unwrap_or("walk"),
                        from,
                        to,
                        hh(frame.seed, salt + k as i64),
                        0.0,
                        0.0,
                    );
                }
            }
            _ => {}
        }
    }

    fn facing_yaw(&self, frame: &Frame, layer: &Value, at: Vec2, k: usize) -> f32 {
        let salt = layer["salt"].as_i64().unwrap_or(0);
        let jitter = (hh(frame.seed, salt + 80 + k as i64) - 0.5) * num(layer, "yaw_jitter", 0.0);
        match layer["facing"].as_str().unwrap_or("") {
            "in" => yaw_of_dir(-frame.out) + jitter,
            "out" => yaw_of_dir(frame.out) + jitter,
            "center" => yaw_of_dir((frame.center - at).normalize_or_zero()) + jitter,
            _ => hh(frame.seed, salt + 80 + k as i64) * TAU,
        }
    }

    fn grid(&mut self, frame: &Frame, layer: &Value, count: usize) {
        let salt = layer["salt"].as_i64().unwrap_or(0);
        let per_row = if layer["per_row"].as_str() == Some("auto") {
            ((count as f32 * 1.5).sqrt().ceil() as usize).clamp(2, 8)
        } else {
            layer["per_row"].as_i64().unwrap_or(4).max(1) as usize
        };
        let columns = per_row.min(count);
        let along_axis = layer["i_axis"].as_str() == Some("along");
        for k in 0..count {
            let mut i = (k % per_row) as f32;
            let mut j = (k / per_row) as f32;
            if layer["center_rows"].as_bool().unwrap_or(false) {
                i -= (columns as f32 - 1.0) * 0.5;
            }
            i *= num(layer, "i_step_m", 1.0);
            j *= num(layer, "j_step_m", 1.0);
            let jitter =
                (hh(frame.seed, salt + 30 + k as i64) - 0.5) * num(layer, "jitter_along_m", 0.0);
            let at = if along_axis {
                self.at(frame, &layer["origin"], j, i + jitter)
            } else {
                self.at(frame, &layer["origin"], i, j + jitter)
            };
            let yaw = self.facing_yaw(frame, layer, at, k);
            if !self.add_static(
                pick(&layer["roles"], k),
                layer["activity"].as_str().unwrap_or("idle"),
                at,
                yaw,
            ) {
                return;
            }
        }
    }

    fn disc(&mut self, frame: &Frame, layer: &Value, count: usize) {
        let salt = layer["salt"].as_i64().unwrap_or(0);
        let a0 = layer["angle"][0].as_f64().unwrap_or(0.0) as f32;
        let a1 = layer["angle"][1].as_f64().unwrap_or(TAU as f64) as f32;
        let radii = if layer.get("radius_n_scale").is_some() {
            Vec2::new(
                0.0,
                num(layer, "radius_n_scale", 1.0) * (count as f32).sqrt(),
            )
        } else {
            vec2(&layer["radius_m"])
        };
        let walk = &layer["walk"];
        let walkers = (num(walk, "share", 0.0) * count as f32).floor() as usize;
        for k in 0..count {
            let angle = a0 + (a1 - a0) * hh(frame.seed, salt + k as i64);
            let radius = radii.x
                + (radii.y - radii.x)
                    * hh(frame.seed, salt + 30 + k as i64).powf(num(layer, "radius_pow", 1.0));
            let at = self.at(
                frame,
                &layer["origin"],
                angle.cos() * radius,
                angle.sin() * radius * num(layer, "along_stretch", 1.0),
            );
            if k < walkers {
                let dir = if walk["direction"].as_str() == Some("along_alternate") {
                    frame.along * if k % 2 == 0 { 1.0 } else { -1.0 }
                } else {
                    let a = hh(frame.seed, salt + 60 + k as i64) * TAU;
                    Vec2::new(a.cos(), a.sin())
                };
                if !self.add(
                    pick(&layer["roles"], k),
                    walk["activity"].as_str().unwrap_or("walk"),
                    at,
                    at + dir * self.m(num(walk, "distance_m", 0.0)),
                    hh(frame.seed, salt + 90 + k as i64),
                    0.0,
                    0.0,
                ) {
                    return;
                }
                continue;
            }
            let yaw = self.facing_yaw(frame, layer, at, k);
            if !self.add_static(
                pick(&layer["roles"], k),
                layer["activity"].as_str().unwrap_or("idle"),
                at,
                yaw,
            ) {
                return;
            }
        }
    }

    fn site(&mut self, frame: &Frame, layer: &Value, figures: usize, river_range: f32) {
        for prop in layer["props"].as_array().into_iter().flatten() {
            let model = prop["model"].as_str().unwrap_or("");
            if prop["place"].as_str() == Some("river") {
                self.river_props(frame, prop, figures, river_range);
            } else if prop.get("repeat").is_some() {
                self.repeated_props(frame, prop, figures);
            } else if prop.get("count").is_some() {
                let salt = prop["salt"].as_i64().unwrap_or(0);
                let scatter = point(&prop["scatter_m"]);
                for k in 0..count_of(&prop["count"], figures) {
                    let eo = (hh(frame.seed, salt + k as i64) - 0.5) * scatter[0];
                    let ea = (hh(frame.seed, salt + 10 + k as i64) - 0.5) * scatter[1];
                    let at = self.at(frame, &prop["at"], eo, ea);
                    let yaw = yaw_of(
                        frame,
                        prop["yaw"].as_str().unwrap_or("random"),
                        1.0,
                        salt + 20 + k as i64,
                    );
                    self.add_static(model, "", at, yaw);
                }
            } else {
                let at = self.at(frame, &prop["at"], 0.0, 0.0);
                let yaw = yaw_of(
                    frame,
                    prop["yaw"].as_str().unwrap_or("random"),
                    1.0,
                    prop["salt"].as_i64().unwrap_or(0),
                );
                self.add_static(model, "", at, yaw);
                self.attach(frame, prop, at, 1.0, 0);
            }
        }
        if layer.get("shuttle").is_some() {
            self.shuttle(frame, &layer["shuttle"], figures);
        }
    }

    /// Figures on the slots of a prop at `at`, oriented by the yaw token of the prop.
    fn attach(&mut self, frame: &Frame, prop: &Value, at: Vec2, row: f32, index: usize) {
        let token = prop["yaw"].as_str().unwrap_or("along");
        let facing = dir_of(frame, token, row);
        let model = prop["model"].as_str().unwrap_or("");
        for part in prop["attached"].as_array().into_iter().flatten() {
            let slot = self.env.slot(
                model,
                part["slot"].as_str().unwrap_or(""),
                vec2(&part["default"]),
            );
            let place = at + (facing * slot.y + right(facing) * slot.x) * self.env.scale();
            let role = match part["role"].as_str() {
                Some(r) => r.to_string(),
                None => pick(&part["roles"], index).to_string(),
            };
            let yaw = yaw_of(frame, part["yaw"].as_str().unwrap_or(token), row, 0);
            self.add_static(
                &role,
                part["activity"].as_str().unwrap_or("idle"),
                place,
                yaw,
            );
        }
    }

    /// Rows of props on both sides of an alley (stalls), a seller and a customer per stall.
    fn repeated_props(&mut self, frame: &Frame, prop: &Value, figures: usize) {
        let repeat = &prop["repeat"];
        let count = count_of(&repeat["count"], figures);
        let step = num(repeat, "step_m", 4.0);
        let half = (count / 2) as f32 * step * 0.5;
        for s in 0..count {
            let row = if s % 2 == 0 { 1.0 } else { -1.0 };
            let p = serde_json::json!([
                row * num(repeat, "row_out_m", 3.0),
                -half + step * (s / 2) as f32
            ]);
            let at = self.at(frame, &p, 0.0, 0.0);
            let yaw = yaw_of(frame, prop["yaw"].as_str().unwrap_or("along"), row, 0);
            self.add_static(prop["model"].as_str().unwrap_or(""), "", at, yaw);
            self.attach(frame, prop, at, row, s);
        }
    }

    /// Sheets of water along the nearby river (else in front of the settlement).
    fn river_props(&mut self, frame: &Frame, prop: &Value, figures: usize, river_range: f32) {
        let salt = prop["salt"].as_i64().unwrap_or(0);
        let footprint = self.env.footprint(frame.index);
        let spacing = self.m(num(prop, "spacing_m", 10.0));
        let mut points = self.env.river_points(
            frame.index,
            frame.center,
            river_range + footprint,
            count_of(&prop["count"], figures),
            spacing,
        );
        if points.is_empty() {
            let fallback = &prop["fallback"];
            let step = point(&fallback["step_m"]);
            let scatter = point(&fallback["scatter_m"]);
            for k in 0..fallback["count"].as_i64().unwrap_or(0).max(0) {
                let kf = k as f32;
                let jitter = hh(frame.seed, 5 + k) - 0.5;
                points.push(self.at(
                    frame,
                    &fallback["at"],
                    step[0] * kf + jitter * scatter[0],
                    step[1] * kf + jitter * scatter[1],
                ));
            }
        }
        for (k, p) in points.iter().enumerate() {
            self.add_static(
                prop["model"].as_str().unwrap_or(""),
                "",
                *p,
                hh(frame.seed, salt + k as i64) * TAU,
            );
        }
    }

    /// Walkers going back and forth between two points (masons, customers).
    fn shuttle(&mut self, frame: &Frame, shuttle: &Value, figures: usize) {
        let salt = shuttle["salt"].as_i64().unwrap_or(0);
        let first = self.at(frame, &shuttle["between"][0], 0.0, 0.0);
        let second = self.at(frame, &shuttle["between"][1], 0.0, 0.0);
        for k in 0..count_of(&shuttle["count"], figures) {
            let forward = k % 2 == 0;
            let lateral =
                (hh(frame.seed, salt + 30 + k as i64) - 0.5) * num(shuttle, "lateral_m", 0.0) * 2.0;
            let behind = num(shuttle, "behind_every_m", 0.0) * (k % 3) as f32;
            let (from, to) = if forward {
                (first, second)
            } else {
                (second, first)
            };
            if !self.add(
                pick(&shuttle["roles"], k),
                shuttle["activity"].as_str().unwrap_or("walk"),
                from,
                to,
                hh(frame.seed, salt + k as i64),
                lateral,
                behind,
            ) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestEnv {
        river: Vec<Vec2>,
    }

    impl Env for TestEnv {
        fn scale(&self) -> f32 {
            0.01
        }
        fn slot(&self, _prop: &str, _slot: &str, fallback: Vec2) -> Vec2 {
            fallback
        }
        fn escape_path(
            &mut self,
            _i: usize,
            g: usize,
            from: Vec2,
            dir: Vec2,
            _s: u64,
        ) -> Vec<Vec2> {
            let step = dir * 0.5;
            (0..4)
                .map(|k| from + step * k as f32 + Vec2::splat(g as f32))
                .collect()
        }
        fn river_points(
            &mut self,
            _i: usize,
            _c: Vec2,
            _r: f32,
            count: usize,
            _s: f32,
        ) -> Vec<Vec2> {
            self.river.iter().take(count).copied().collect()
        }
        fn footprint(&self, _i: usize) -> f32 {
            1.0
        }
    }

    fn layouts() -> Value {
        let root = crate::paths::find_root().expect("root");
        let text = std::fs::read_to_string(
            crate::paths::data_dir(&root).join("rules/folk_scene_layouts.json"),
        )
        .unwrap();
        serde_json::from_str::<Value>(&text).unwrap()["scenes"].clone()
    }

    fn frame() -> Frame {
        Frame {
            anchor: Vec2::new(100.0, 100.0),
            out: Vec2::X,
            along: Vec2::Y,
            seed: 777,
            center: Vec2::new(99.0, 100.0),
            index: 0,
        }
    }

    fn stage(kind: &str, figures: usize, river: Vec<Vec2>) -> Vec<Item> {
        let layouts = layouts();
        let mut env = TestEnv { river };
        let mut stager = Stager {
            env: &mut env,
            items: Vec::new(),
            budget: 500,
        };
        stager.stage(&frame(), &layouts[kind], figures);
        stager.items
    }

    #[test]
    fn count_follows_base_per_n_and_bounds() {
        let spec = json!({"base": -4, "per_n": 1, "min": 2});
        assert_eq!(count_of(&spec, 3), 2);
        assert_eq!(count_of(&spec, 10), 6);
        assert_eq!(count_of(&json!({"fixed": 4}), 99), 4);
        assert_eq!(count_of(&json!({"per_n": 0.5, "max": 3}), 20), 3);
    }

    #[test]
    fn every_kind_of_scene_places_something_and_is_deterministic() {
        let layouts = layouts();
        for kind in layouts.as_object().unwrap().keys() {
            let a = stage(kind, 12, vec![Vec2::new(105.0, 103.0)]);
            let b = stage(kind, 12, vec![Vec2::new(105.0, 103.0)]);
            assert!(!a.is_empty(), "{kind} places nothing");
            assert_eq!(a, b, "{kind} is deterministic");
        }
    }

    #[test]
    fn more_figures_give_more_items() {
        for kind in ["fair", "plague", "muster"] {
            assert!(
                stage(kind, 20, vec![]).len() >= stage(kind, 4, vec![]).len(),
                "{kind}"
            );
        }
    }

    #[test]
    fn procession_has_a_cross_and_banners() {
        let items = stage("celebration", 10, vec![]);
        let models: Vec<&str> = items
            .iter()
            .map(|i| match i {
                Item::Mover { model, .. } | Item::Static { model, .. } => model.as_str(),
            })
            .collect();
        assert!(models.contains(&"procession_cross") || models.contains(&"procession_banner"));
        assert!(models.contains(&"monk"));
    }

    #[test]
    fn flood_lays_water_on_the_river_or_in_front() {
        let on_river = stage(
            "flood",
            8,
            vec![Vec2::new(120.0, 120.0), Vec2::new(121.0, 121.0)],
        );
        let water: Vec<&Item> = on_river
            .iter()
            .filter(|i| matches!(i, Item::Static { model, .. } if model == "flood_water"))
            .collect();
        assert!(water.iter().any(|i| matches!(i, Item::Static { at, .. } if (*at - Vec2::new(120.0, 120.0)).length() < 1e-3)));
        let fallback = stage("flood", 8, vec![]);
        assert!(
            fallback
                .iter()
                .any(|i| matches!(i, Item::Static { model, .. } if model == "flood_water"))
        );
    }

    #[test]
    fn budget_stops_the_scene() {
        let layouts = layouts();
        let mut env = TestEnv { river: vec![] };
        let mut stager = Stager {
            env: &mut env,
            items: Vec::new(),
            budget: 3,
        };
        stager.stage(&frame(), &layouts["fair"], 24);
        assert!(stager.items.len() <= 3);
    }
}
