//! Lot PB3e (ADR 0090): ground marks of the battle (trampled snow or mud of
//! `battle_terrain.gd`, flattened and bloodied grass of
//! `battle_grass_flatten.gd`) stamped in Rust instead of GDScript loops over
//! texels. Rendering only, no game rule: the GDScript keeps deciding what to
//! stamp and where; this class only fills the bytes and uploads them.
//!
//! The arithmetic follows the GDScript it replaces (`Vector2` in single
//! precision, `float` in double), so the maps are the same texel for texel.

use crate::convert::vec2_pair;
use godot::classes::image::Format;
use godot::classes::{Image, ImageTexture, RefCounted};
use godot::prelude::*;

/// A `width × height` map of 1 or 2 byte channels laid over a world
/// rectangle (`origin`, `texel` metres per texel). Plain Rust.
#[derive(Debug, Clone, Default)]
struct Marks {
    width: usize,
    height: usize,
    channels: usize,
    origin: (f32, f32),
    texel: f32,
    bytes: Vec<u8>,
    dirty: bool,
}

/// Godot handle on a [`Marks`] map.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct StampMap {
    marks: Marks,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for StampMap {
    fn init(base: Base<RefCounted>) -> Self {
        StampMap {
            marks: Marks::default(),
            base,
        }
    }
}

impl Marks {
    /// Texel range `[lo, hi)` covered by `centre ± reach` along one axis,
    /// as GDScript `maxi(int(c - r), 0)` / `mini(int(c + r) + 1, n)`.
    fn span(centre: f32, reach: f64, n: usize) -> std::ops::Range<usize> {
        let c = f64::from(centre);
        let lo = ((c - reach) as i64).max(0);
        let hi = ((c + reach) as i64 + 1).min(n as i64);
        if hi <= lo {
            0..0
        } else {
            lo as usize..hi as usize
        }
    }

    /// See [`StampMap::stamp_box`].
    fn stamp_box(
        &mut self,
        center: (f32, f32),
        facing: f64,
        half: (f32, f32),
        add_r: i64,
        add_g: i64,
        cap_r: i64,
    ) -> bool {
        if self.bytes.is_empty() {
            return false;
        }
        let axis_x = (facing.cos() as f32, -facing.sin() as f32);
        let axis_z = (facing.sin() as f32, facing.cos() as f32);
        let reach = f64::from((half.0 * half.0 + half.1 * half.1).sqrt());
        let c = (
            (center.0 - self.origin.0) / self.texel,
            (center.1 - self.origin.1) / self.texel,
        );
        let r = reach / f64::from(self.texel);
        let mut changed = false;
        for iz in Self::span(c.1, r, self.height) {
            for ix in Self::span(c.0, r, self.width) {
                let d = (
                    self.origin.0 + (ix as f32 + 0.5) * self.texel - center.0,
                    self.origin.1 + (iz as f32 + 0.5) * self.texel - center.1,
                );
                let dx = d.0 * axis_x.0 + d.1 * axis_x.1;
                let dz = d.0 * axis_z.0 + d.1 * axis_z.1;
                if dx.abs() > half.0 || dz.abs() > half.1 {
                    continue;
                }
                let i = (iz * self.width + ix) * self.channels;
                let old = i64::from(self.bytes[i]);
                if old < cap_r {
                    self.bytes[i] = (old + add_r).min(cap_r).clamp(0, 255) as u8;
                    changed = true;
                }
                if add_g > 0 && self.channels > 1 {
                    let g = i64::from(self.bytes[i + 1]);
                    self.bytes[i + 1] = (g + add_g).min(255) as u8;
                    changed = true;
                }
            }
        }
        self.dirty |= changed;
        changed
    }

    /// See [`StampMap::stamp_disc`].
    fn stamp_disc(&mut self, center: (f32, f32), radius: f64, r_value: i64, g_value: i64) {
        if self.bytes.is_empty() {
            return;
        }
        let texel = f64::from(self.texel);
        let c = (
            (center.0 - self.origin.0) / self.texel,
            (center.1 - self.origin.1) / self.texel,
        );
        let rr = radius / texel + 1.0;
        let smoothstep = |from: f64, to: f64, x: f64| {
            let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        for iz in Self::span(c.1, rr, self.height) {
            for ix in Self::span(c.0, rr, self.width) {
                let v = (ix as f32 + 0.5 - c.0, iz as f32 + 0.5 - c.1);
                let d = f64::from((v.0 * v.0 + v.1 * v.1).sqrt()) * texel;
                let k = 1.0 - smoothstep(radius * 0.8, radius + texel * 0.5, d);
                if k <= 0.0 {
                    continue;
                }
                let i = (iz * self.width + ix) * self.channels;
                if r_value > 0 {
                    let v = (r_value as f64 * k) as i64;
                    self.bytes[i] = i64::from(self.bytes[i]).max(v).clamp(0, 255) as u8;
                }
                if g_value > 0 && self.channels > 1 {
                    let v = (g_value as f64 * k) as i64;
                    self.bytes[i + 1] = (i64::from(self.bytes[i + 1]) + v).min(255) as u8;
                }
                self.dirty = true;
            }
        }
    }
}

#[godot_api]
impl StampMap {
    /// `width × height` texels of `channels` bytes (1: L8, 2: RG8), all zero,
    /// laid from `origin` (world x, z) at `texel` metres per texel.
    #[func]
    fn setup(&mut self, width: i64, height: i64, channels: i64, origin: Vector2, texel: f64) {
        let (width, height) = (width.max(0) as usize, height.max(0) as usize);
        let channels = channels.clamp(1, 2) as usize;
        self.marks = Marks {
            width,
            height,
            channels,
            origin: vec2_pair(origin),
            texel: texel.max(1e-3) as f32,
            bytes: vec![0; width * height * channels],
            dirty: true,
        };
    }

    /// Oriented rectangle (`half` half-sides, x along the front, `facing` in
    /// radians): channel R gains `add_r` up to `cap_r` (never lowered), G
    /// gains `add_g` up to 255. True if a byte changed.
    #[func]
    fn stamp_box(
        &mut self,
        center: Vector2,
        facing: f64,
        half: Vector2,
        add_r: i64,
        add_g: i64,
        cap_r: i64,
    ) -> bool {
        self.marks.stamp_box(
            vec2_pair(center),
            facing,
            vec2_pair(half),
            add_r,
            add_g,
            cap_r,
        )
    }

    /// Disc with a soft edge: R and G rise towards `r_value` / `g_value`.
    #[func]
    fn stamp_disc(&mut self, center: Vector2, radius: f64, r_value: i64, g_value: i64) {
        self.marks
            .stamp_disc(vec2_pair(center), radius, r_value, g_value);
    }

    /// Byte of `channel` at world point (x, z), 0-1 (0 outside).
    #[func]
    fn sample(&self, x: f64, z: f64, channel: i64) -> f64 {
        self.marks.sample(x, z, channel)
    }

    /// Sends the map to `texture` through `image` if it changed (full
    /// upload: Godot 4.7 has no partial texture update). True if sent.
    #[func]
    fn upload(&mut self, mut image: Gd<Image>, mut texture: Gd<ImageTexture>) -> bool {
        if !self.marks.dirty || self.marks.bytes.is_empty() {
            return false;
        }
        self.marks.dirty = false;
        let bytes = PackedByteArray::from(self.marks.bytes.as_slice());
        let m = &self.marks;
        image.set_data(m.width as i32, m.height as i32, false, m.format(), &bytes);
        texture.update(&image);
        true
    }
}

impl Marks {
    fn sample(&self, x: f64, z: f64, channel: i64) -> f64 {
        let ix = ((x as f32 - self.origin.0) / self.texel).floor();
        let iz = ((z as f32 - self.origin.1) / self.texel).floor();
        if ix < 0.0 || iz < 0.0 || ix >= self.width as f32 || iz >= self.height as f32 {
            return 0.0;
        }
        let ch = (channel.max(0) as usize).min(self.channels - 1);
        let i = (iz as usize * self.width + ix as usize) * self.channels + ch;
        f64::from(self.bytes[i]) / 255.0
    }
}

impl Marks {
    fn format(&self) -> Format {
        if self.channels == 2 {
            Format::RG8
        } else {
            Format::L8
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(w: usize, h: usize, channels: usize) -> Marks {
        Marks {
            width: w,
            height: h,
            channels,
            origin: (-40.0, -40.0),
            texel: 4.0,
            bytes: vec![0; w * h * channels],
            dirty: false,
        }
    }

    #[test]
    fn box_is_capped_and_oriented() {
        let mut m = map(40, 40, 2);
        for _ in 0..10 {
            m.stamp_box((20.0, 20.0), 0.3, (12.0, 3.0), 30, 5, 150);
        }
        let at = |m: &Marks, x: f32, z: f32| {
            let ix = ((x - m.origin.0) / m.texel) as usize;
            let iz = ((z - m.origin.1) / m.texel) as usize;
            m.bytes[(iz * m.width + ix) * 2]
        };
        assert_eq!(at(&m, 20.0, 20.0), 150);
        assert_eq!(at(&m, 20.0, 40.0), 0);
        assert!(m.dirty);
    }
}
