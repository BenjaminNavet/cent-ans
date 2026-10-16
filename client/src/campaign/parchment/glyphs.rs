//! Ink vignettes of the parchment, painted once on the CPU (tiny-skia) into textures, as the Godot
//! layer painted them once into SubViewports: town (mound, spire, crenellated wall, two towers
//! with red roofs), ship (hull, square sail with a red cross, pennant, waves), sea serpent, whale,
//! owner pennant and its outline. Ports `ParchmentOverlay.paint_town`, `_draw_ship`,
//! `_draw_serpent`, `_draw_whale`, `_draw_waves`, `_draw_pennant`.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform as SkTransform,
};

/// Glyphs are painted at this multiple of their logical size (sharp up to the 1.6 view scale).
pub const OVERSAMPLE: f32 = 3.0;

pub const INK: [f32; 3] = [0.20, 0.13, 0.07];
pub const RED_INK: [f32; 3] = [0.62, 0.14, 0.09];
pub const PAPER: [f32; 3] = [0.93, 0.86, 0.70];
pub const ROOF: [f32; 3] = [0.70, 0.25, 0.15];
pub const SEA_GREEN: [f32; 3] = [0.38, 0.52, 0.40];
pub const GOLD: [f32; 3] = [0.95, 0.76, 0.28];

/// A painted glyph: logical size, anchor (logical px) and the drawing unit it was painted with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphSpec {
    pub size: Vec2,
    pub anchor: Vec2,
    pub unit: f32,
}

pub const TOWN: GlyphSpec = GlyphSpec {
    size: Vec2::new(72.0, 72.0),
    anchor: Vec2::new(36.0, 58.0),
    unit: 22.0,
};
pub const SHIP: GlyphSpec = GlyphSpec {
    size: Vec2::new(72.0, 72.0),
    anchor: Vec2::new(36.0, 42.0),
    unit: 24.0,
};
pub const SERPENT: GlyphSpec = GlyphSpec {
    size: Vec2::new(100.0, 48.0),
    anchor: Vec2::new(52.0, 26.0),
    unit: 20.0,
};
pub const WHALE: GlyphSpec = GlyphSpec {
    size: Vec2::new(90.0, 52.0),
    anchor: Vec2::new(40.0, 30.0),
    unit: 20.0,
};
/// Pennant, 1 × 0.46 of its length; painted white (tinted by the node colour).
pub const PENNANT: GlyphSpec = GlyphSpec {
    size: Vec2::new(32.0, 16.0),
    anchor: Vec2::new(0.0, 0.0),
    unit: 32.0,
};

/// Weather cloud (`WEATHER_GLYPH_SIZE`, `WEATHER_ANCHOR`, `WEATHER_UNIT` of parchment_overlay.gd).
pub const WEATHER: GlyphSpec = GlyphSpec {
    size: Vec2::new(64.0, 56.0),
    anchor: Vec2::new(32.0, 24.0),
    unit: 14.0,
};
/// Weather glyph names (`WEATHER_GLYPHS`).
pub const WEATHER_KINDS: [&str; 4] = ["rain", "snow", "storm", "fog"];

fn lighten(c: [f32; 3], k: f32) -> [f32; 3] {
    [
        c[0] + (1.0 - c[0]) * k,
        c[1] + (1.0 - c[1]) * k,
        c[2] + (1.0 - c[2]) * k,
    ]
}

fn darken(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * (1.0 - k), c[1] * (1.0 - k), c[2] * (1.0 - k)]
}

/// Drawing surface in logical glyph units.
pub struct Canvas {
    pixmap: Pixmap,
    transform: SkTransform,
}

impl Canvas {
    pub fn new(size: Vec2) -> Self {
        let w = (size.x * OVERSAMPLE).ceil() as u32;
        let h = (size.y * OVERSAMPLE).ceil() as u32;
        Self {
            pixmap: Pixmap::new(w.max(1), h.max(1)).expect("glyph size"),
            transform: SkTransform::from_scale(OVERSAMPLE, OVERSAMPLE),
        }
    }

    fn paint(color: [f32; 3], alpha: f32) -> Paint<'static> {
        let mut paint = Paint::default();
        paint.set_color_rgba8(
            (color[0].clamp(0.0, 1.0) * 255.0) as u8,
            (color[1].clamp(0.0, 1.0) * 255.0) as u8,
            (color[2].clamp(0.0, 1.0) * 255.0) as u8,
            (alpha.clamp(0.0, 1.0) * 255.0) as u8,
        );
        paint.anti_alias = true;
        paint
    }

    fn path(points: &[Vec2], close: bool) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        let first = points.first()?;
        pb.move_to(first.x, first.y);
        for p in &points[1..] {
            pb.line_to(p.x, p.y);
        }
        if close {
            pb.close();
        }
        pb.finish()
    }

    pub fn fill(&mut self, points: &[Vec2], color: [f32; 3], alpha: f32) {
        if let Some(path) = Self::path(points, true) {
            self.pixmap.fill_path(
                &path,
                &Self::paint(color, alpha),
                FillRule::Winding,
                self.transform,
                None,
            );
        }
    }

    /// Stroke of `width` screen pixels at scale 1 (as Godot's `draw_polyline` widths).
    pub fn stroke(
        &mut self,
        points: &[Vec2],
        color: [f32; 3],
        alpha: f32,
        width: f32,
        close: bool,
    ) {
        if let Some(path) = Self::path(points, close) {
            let stroke = Stroke {
                width,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..default()
            };
            self.pixmap.stroke_path(
                &path,
                &Self::paint(color, alpha),
                &stroke,
                self.transform,
                None,
            );
        }
    }

    pub fn circle(&mut self, c: Vec2, r: f32, color: [f32; 3], alpha: f32) {
        if let Some(path) = PathBuilder::from_circle(c.x, c.y, r.max(0.1)) {
            self.pixmap.fill_path(
                &path,
                &Self::paint(color, alpha),
                FillRule::Winding,
                self.transform,
                None,
            );
        }
    }

    fn rect_points(min: Vec2, size: Vec2) -> [Vec2; 4] {
        [
            min,
            min + Vec2::new(size.x, 0.0),
            min + size,
            min + Vec2::new(0.0, size.y),
        ]
    }

    /// Straight-alpha sRGB image with a mip chain (the glyphs are drawn much smaller than painted).
    pub fn into_image(self) -> Image {
        let (w, h) = (self.pixmap.width(), self.pixmap.height());
        let mut data = Vec::with_capacity((w * h * 4) as usize);
        for px in self.pixmap.pixels() {
            let c = px.demultiply();
            data.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
        rgba_image_with_mips(w, h, data)
    }

    /// Raw straight-alpha pixels (tests).
    pub fn alpha_at(&self, x: u32, y: u32) -> u8 {
        self.pixmap
            .pixel(x, y)
            .map_or(0, |p| p.demultiply().alpha())
    }
}

/// RGBA8 sRGB image plus a box-filtered mip chain and a trilinear sampler.
pub fn rgba_image_with_mips(width: u32, height: u32, data: Vec<u8>) -> Image {
    let mut levels = 1;
    let mut all = data.clone();
    let (mut w, mut h, mut level) = (width, height, data);
    while w > 1 && h > 1 && levels < 8 {
        let (nw, nh) = (w / 2, h / 2);
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                // Premultiplied average so transparent texels do not darken the edges.
                let mut acc = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let i = (((y * 2 + dy) * w + x * 2 + dx) * 4) as usize;
                    let a = u32::from(level[i + 3]);
                    acc[0] += u32::from(level[i]) * a;
                    acc[1] += u32::from(level[i + 1]) * a;
                    acc[2] += u32::from(level[i + 2]) * a;
                    acc[3] += a;
                }
                let o = ((y * nw + x) * 4) as usize;
                if let (Some(r), Some(g), Some(b)) = (
                    acc[0].checked_div(acc[3]),
                    acc[1].checked_div(acc[3]),
                    acc[2].checked_div(acc[3]),
                ) {
                    next[o] = r as u8;
                    next[o + 1] = g as u8;
                    next[o + 2] = b as u8;
                }
                next[o + 3] = (acc[3] / 4) as u8;
            }
        }
        all.extend_from_slice(&next);
        level = next;
        w = nw;
        h = nh;
        levels += 1;
    }
    let base = all[..(width * height * 4) as usize].to_vec();
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        base,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(all);
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

fn closed_lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Town vignette at `p` with unit `u` (`paint_town`).
pub fn paint_town(canvas: &mut Canvas, p: Vec2, u: f32) {
    let ground: Vec<Vec2> = (0..16)
        .map(|i| {
            let ang = std::f32::consts::TAU * i as f32 / 16.0;
            p + Vec2::new(ang.cos() * 1.05, 0.05 + ang.sin() * 0.28) * u
        })
        .collect();
    canvas.fill(&ground, [0.55, 0.45, 0.28], 0.45);
    let mut wall = vec![p + Vec2::new(-0.8, 0.0) * u, p + Vec2::new(-0.8, -0.45) * u];
    let merlons = 6;
    for i in 0..merlons {
        let x0 = closed_lerp(-0.8, 0.8, i as f32 / merlons as f32);
        let x1 = closed_lerp(-0.8, 0.8, (i as f32 + 0.5) / merlons as f32);
        let x2 = closed_lerp(-0.8, 0.8, (i + 1) as f32 / merlons as f32);
        wall.push(p + Vec2::new(x0, -0.55) * u);
        wall.push(p + Vec2::new(x1, -0.55) * u);
        wall.push(p + Vec2::new(x1, -0.45) * u);
        wall.push(p + Vec2::new(x2, -0.45) * u);
    }
    wall.push(p + Vec2::new(0.8, 0.0) * u);
    let spire_x = 0.15;
    paint_box(
        canvas,
        p + Vec2::new(spire_x - 0.14, -1.05) * u,
        Vec2::new(0.28, 0.7) * u,
        darken(PAPER, 0.08),
    );
    paint_roof(
        canvas,
        p + Vec2::new(spire_x, -1.05) * u,
        0.2 * u,
        0.75 * u,
        [0.35, 0.33, 0.36],
    );
    canvas.fill(&wall, lighten(PAPER, 0.1), 1.0);
    canvas.stroke(&wall, INK, 1.0, 1.6, true);
    canvas.fill(
        &Canvas::rect_points(p + Vec2::new(-0.12, -0.28) * u, Vec2::new(0.24, 0.28) * u),
        INK,
        0.8,
    );
    for tx in [-0.8, 0.8] {
        paint_box(
            canvas,
            p + Vec2::new(tx - 0.16, -0.75) * u,
            Vec2::new(0.32, 0.75) * u,
            PAPER,
        );
        paint_roof(
            canvas,
            p + Vec2::new(tx, -0.75) * u,
            0.22 * u,
            0.45 * u,
            ROOF,
        );
    }
}

fn paint_box(canvas: &mut Canvas, min: Vec2, size: Vec2, fill: [f32; 3]) {
    let pts = Canvas::rect_points(min, size);
    canvas.fill(&pts, fill, 1.0);
    canvas.stroke(&pts, INK, 1.0, 1.4, true);
}

fn paint_roof(canvas: &mut Canvas, base: Vec2, half_width: f32, height: f32, color: [f32; 3]) {
    let tri = [
        base + Vec2::new(-half_width, 0.0),
        base + Vec2::new(0.0, -height),
        base + Vec2::new(half_width, 0.0),
    ];
    canvas.fill(&tri, color, 1.0);
    canvas.stroke(&tri, INK, 1.0, 1.4, true);
}

fn waves(canvas: &mut Canvas, p: Vec2, u: f32) {
    for k in 0..3 {
        let y = p.y + k as f32 * 0.18 * u;
        let pts: Vec<Vec2> = (0..9)
            .map(|i| {
                let x = closed_lerp(-1.2, 1.2, i as f32 / 8.0) * (1.0 - k as f32 * 0.2);
                Vec2::new(p.x + x * u, y + (i as f32 * 1.6).sin() * 0.06 * u)
            })
            .collect();
        canvas.stroke(&pts, INK, 0.55 - k as f32 * 0.15, 1.0, false);
    }
}

/// Ship facing east (`_draw_ship`, `facing` = 1).
pub fn paint_ship(canvas: &mut Canvas, p: Vec2, u: f32) {
    let mut hull = Vec::new();
    for i in 0..9 {
        let x = closed_lerp(-1.1, 1.1, i as f32 / 8.0);
        hull.push(p + Vec2::new(x, 0.28 * x * x - 0.05) * u);
    }
    for i in (0..9).rev() {
        let x = closed_lerp(-0.8, 0.85, i as f32 / 8.0);
        hull.push(p + Vec2::new(x, 0.42 + 0.06 * x * x) * u);
    }
    canvas.fill(&hull, [0.52, 0.33, 0.18], 1.0);
    canvas.stroke(&hull, INK, 1.0, 1.2, true);
    canvas.stroke(
        &[p + Vec2::new(0.05, 0.0) * u, p + Vec2::new(0.05, -1.55) * u],
        INK,
        1.0,
        1.3,
        false,
    );
    let mut sail = Vec::new();
    for i in 0..7 {
        let t = i as f32 / 6.0;
        let y = closed_lerp(-1.35, -0.2, t);
        sail.push(p + Vec2::new(-0.55 + 0.12 * (t * std::f32::consts::PI).sin(), y) * u);
    }
    for i in (0..7).rev() {
        let t = i as f32 / 6.0;
        let y = closed_lerp(-1.35, -0.2, t);
        sail.push(p + Vec2::new(0.65 + 0.22 * (t * std::f32::consts::PI).sin(), y) * u);
    }
    canvas.fill(&sail, lighten(PAPER, 0.15), 1.0);
    canvas.stroke(&sail, INK, 1.0, 1.0, true);
    canvas.stroke(
        &[
            p + Vec2::new(0.08, -1.2) * u,
            p + Vec2::new(0.08, -0.35) * u,
        ],
        RED_INK,
        1.0,
        1.6,
        false,
    );
    canvas.stroke(
        &[
            p + Vec2::new(-0.35, -0.8) * u,
            p + Vec2::new(0.55, -0.8) * u,
        ],
        RED_INK,
        1.0,
        1.6,
        false,
    );
    let flag = [
        p + Vec2::new(0.05, -1.55) * u,
        p + Vec2::new(0.55, -1.45) * u,
        p + Vec2::new(0.05, -1.38) * u,
    ];
    canvas.fill(&flag, RED_INK, 1.0);
    waves(canvas, p + Vec2::new(0.0, 0.55) * u, u);
}

/// Sea serpent, head raised on the left (`_draw_serpent`, `t` = 0).
pub fn paint_serpent(canvas: &mut Canvas, p: Vec2, u: f32) {
    use std::f32::consts::PI;
    for k in 0..3 {
        let cx = p.x + (k as f32 - 1.0) * 0.85 * u;
        let mut arc = Vec::new();
        for i in 0..11 {
            let ang = PI + PI * i as f32 / 10.0;
            arc.push(Vec2::new(
                cx + ang.cos() * 0.36 * u,
                p.y + ang.sin() * (0.42 + 0.04 * (k as f32).sin()) * u,
            ));
        }
        for i in (0..11).rev() {
            let ang = PI + PI * i as f32 / 10.0;
            arc.push(Vec2::new(
                cx + ang.cos() * 0.2 * u,
                p.y + ang.sin() * 0.25 * u,
            ));
        }
        canvas.fill(&arc, SEA_GREEN, 1.0);
        canvas.stroke(&arc, INK, 1.0, 1.0, true);
    }
    let head: Vec<Vec2> = [
        (-1.1, 0.0),
        (-1.3, -0.8),
        (-1.9, -0.95),
        (-2.15, -0.8),
        (-1.75, -0.7),
        (-1.95, -0.55),
        (-1.5, -0.45),
        (-1.4, 0.0),
    ]
    .iter()
    .map(|(x, y)| p + Vec2::new(*x, *y) * u)
    .collect();
    canvas.fill(&head, SEA_GREEN, 1.0);
    canvas.stroke(&head, INK, 1.0, 1.0, true);
    canvas.circle(
        p + Vec2::new(-1.62, -0.8) * u,
        (0.06 * u).max(1.0),
        RED_INK,
        1.0,
    );
    let tail: Vec<Vec2> = [
        (1.25, 0.0),
        (1.6, -0.55),
        (1.55, -0.2),
        (1.95, -0.45),
        (1.45, 0.0),
    ]
    .iter()
    .map(|(x, y)| p + Vec2::new(*x, *y) * u)
    .collect();
    canvas.fill(&tail, SEA_GREEN, 1.0);
    canvas.stroke(&tail, INK, 1.0, 1.0, true);
    waves(canvas, p + Vec2::new(0.0, 0.1) * u, u * 1.4);
}

/// Whale spouting (`_draw_whale`, `t` = 0).
pub fn paint_whale(canvas: &mut Canvas, p: Vec2, u: f32) {
    let body = [0.36, 0.40, 0.42];
    let back: Vec<Vec2> = (0..17)
        .map(|i| {
            let x = closed_lerp(-1.4, 1.2, i as f32 / 16.0);
            let k = ((x + 0.1) / 1.3).powi(2);
            p + Vec2::new(x, -0.55 * (1.0 - k).max(0.0).sqrt()) * u
        })
        .collect();
    canvas.fill(&back, body, 1.0);
    canvas.stroke(&back, INK, 1.0, 1.1, true);
    canvas.circle(
        p + Vec2::new(-0.95, -0.25) * u,
        (0.05 * u).max(1.0),
        INK,
        1.0,
    );
    let top = p + Vec2::new(-0.6, -0.55) * u;
    for k in 0..5 {
        let ang = (-150.0 + k as f32 * 30.0f32).to_radians();
        canvas.stroke(
            &[
                top,
                top + Vec2::new(ang.cos(), ang.sin() - 0.6) * u * 0.5 * 0.8,
            ],
            [0.30, 0.42, 0.50],
            1.0,
            1.2,
            false,
        );
    }
    let fluke: Vec<Vec2> = [
        (1.1, -0.05),
        (1.55, -0.65),
        (1.4, -0.3),
        (1.85, -0.5),
        (1.3, 0.0),
    ]
    .iter()
    .map(|(x, y)| p + Vec2::new(*x, *y) * u)
    .collect();
    canvas.fill(&fluke, body, 1.0);
    canvas.stroke(&fluke, INK, 1.0, 1.0, true);
    waves(canvas, p + Vec2::new(0.0, 0.1) * u, u * 1.5);
}

/// Pennant triangle (white, tinted by the node), or its outline only.
pub fn paint_pennant(canvas: &mut Canvas, outline: bool) {
    let length = PENNANT.size.x - 2.0;
    let h = length * 0.46;
    let at = Vec2::new(1.0, 1.0);
    let flag = [at, at + Vec2::new(length, h * 0.5), at + Vec2::new(0.0, h)];
    if outline {
        canvas.stroke(&flag, [1.0; 3], 1.0, 2.0, true);
    } else {
        canvas.fill(&flag, [1.0; 3], 1.0);
    }
}

/// Ink cloud (scallops), rain as oblique strokes, snow as stars, storm as a bolt, fog as wavy
/// lines (`paint_weather`).
pub fn paint_weather(canvas: &mut Canvas, p: Vec2, u: f32, kind: &str) {
    let ink = [0.25, 0.22, 0.24];
    let fill = if kind == "storm" {
        [0.45, 0.44, 0.50]
    } else {
        [0.86, 0.85, 0.82]
    };
    if kind == "fog" {
        for k in 0..3 {
            let line: Vec<Vec2> = (0..13)
                .map(|i| {
                    let x = -1.6 + 3.2 * (i as f32 / 12.0);
                    let y = -0.4 + k as f32 * 0.55 + (i as f32 * 1.3 + k as f32).sin() * 0.12;
                    p + Vec2::new(x, y) * u
                })
                .collect();
            canvas.stroke(&line, ink, 0.8, 2.0, false);
        }
        return;
    }
    let lobes = [
        Vec2::new(-0.9, 0.1),
        Vec2::new(-0.35, -0.35),
        Vec2::new(0.35, -0.3),
        Vec2::new(0.9, 0.1),
        Vec2::new(0.0, 0.15),
    ];
    let radii = [0.55, 0.7, 0.62, 0.5, 0.6];
    for (lobe, radius) in lobes.iter().zip(radii) {
        canvas.circle(p + *lobe * u, radius * u + 1.6, ink, 1.0);
    }
    for (lobe, radius) in lobes.iter().zip(radii) {
        canvas.circle(p + *lobe * u, radius * u, fill, 1.0);
    }
    let below = p + Vec2::new(0.0, 0.8) * u;
    match kind {
        "rain" => {
            for i in 0..5 {
                let x = -1.0 + 2.0 * (i as f32 / 4.0);
                let line = [
                    below + Vec2::new(x, 0.0) * u,
                    below + Vec2::new(x - 0.3, 0.9) * u,
                ];
                canvas.stroke(&line, [0.16, 0.28, 0.45], 1.0, 2.0, false);
            }
        }
        "snow" => {
            for i in 0..4 {
                let c = below
                    + Vec2::new(-0.9 + 1.8 * (i as f32 / 3.0), 0.35 + 0.3 * (i % 2) as f32) * u;
                for k in 0..3 {
                    let ang = k as f32 * std::f32::consts::PI / 3.0;
                    let d = Vec2::new(ang.cos(), ang.sin()) * 0.22 * u;
                    canvas.stroke(&[c - d, c + d], [0.25, 0.35, 0.55], 1.0, 1.6, false);
                }
            }
        }
        "storm" => {
            let bolt: Vec<Vec2> = [
                (0.1, -0.1),
                (-0.35, 0.55),
                (0.05, 0.5),
                (-0.25, 1.15),
                (0.45, 0.3),
                (0.05, 0.35),
                (0.35, -0.1),
            ]
            .iter()
            .map(|(x, y)| below + Vec2::new(*x, *y) * u)
            .collect();
            canvas.fill(&bolt, [0.95, 0.72, 0.15], 1.0);
            canvas.stroke(&bolt, [0.45, 0.12, 0.05], 1.0, 1.4, true);
        }
        _ => {}
    }
}

/// Every glyph the overlay needs, by name.
pub fn paint(name: &str) -> Option<(GlyphSpec, Image)> {
    let spec = match name {
        "town" => TOWN,
        "ship" => SHIP,
        "serpent" => SERPENT,
        "whale" => WHALE,
        "pennant" | "pennant_edge" => PENNANT,
        k if WEATHER_KINDS.contains(&k) => WEATHER,
        _ => return None,
    };
    let mut canvas = Canvas::new(spec.size);
    match name {
        "town" => paint_town(&mut canvas, spec.anchor, spec.unit),
        "ship" => paint_ship(&mut canvas, spec.anchor, spec.unit),
        "serpent" => paint_serpent(&mut canvas, spec.anchor, spec.unit),
        "whale" => paint_whale(&mut canvas, spec.anchor, spec.unit),
        "pennant" => paint_pennant(&mut canvas, false),
        "pennant_edge" => paint_pennant(&mut canvas, true),
        kind => paint_weather(&mut canvas, spec.anchor, spec.unit, kind),
    }
    Some((spec, canvas.into_image()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn town_is_inked_at_its_anchor_and_clear_in_the_corners() {
        let mut canvas = Canvas::new(TOWN.size);
        paint_town(&mut canvas, TOWN.anchor, TOWN.unit);
        let at = |p: Vec2| canvas.alpha_at((p.x * OVERSAMPLE) as u32, (p.y * OVERSAMPLE) as u32);
        assert!(
            at(TOWN.anchor + Vec2::new(0.0, -5.0)) > 200,
            "wall above the anchor"
        );
        assert_eq!(at(Vec2::new(1.0, 1.0)), 0);
    }

    #[test]
    fn every_glyph_paints_with_mips() {
        for name in [
            "town",
            "ship",
            "serpent",
            "whale",
            "pennant",
            "pennant_edge",
            "rain",
            "snow",
            "storm",
            "fog",
        ] {
            let (spec, image) = paint(name).unwrap();
            assert_eq!(image.width(), (spec.size.x * OVERSAMPLE).ceil() as u32);
            assert!(image.texture_descriptor.mip_level_count > 1, "{name}");
        }
        assert!(paint("dragon").is_none());
    }
}
