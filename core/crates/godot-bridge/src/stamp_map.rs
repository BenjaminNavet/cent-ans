//! Lot PB3e (ADR 0090): ground marks of the battle (trampled snow or mud of
//! `battle_terrain.gd`, flattened and bloodied grass of
//! `battle_grass_flatten.gd`) stamped in Rust instead of GDScript loops over
//! texels. Rendering only, no game rule: the GDScript keeps deciding what to
//! stamp and where; this class only fills the bytes and uploads them.
//!
//! Lot SC PF-08 adds the RGBA8 splatmaps of `battle_terrain.gd` (4 channels):
//! `stamp_soft_disc`, `raise_pixel` and `image` replace its per-texel
//! `Image.get_pixel` / `set_pixel` loops.
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

    /// See [`StampMap::stamp_soft_disc`]. Follows `battle_terrain.gd::_stamp_disc`
    /// (`Image.get_pixel` / `set_pixel` on RGBA8: reads `byte / 255`, writes the
    /// truncated `value * 255`), the texel corner (not centre) being the sample.
    fn stamp_soft_disc(
        &mut self,
        center: (f32, f32),
        radius: f64,
        channel: usize,
        feather: f64,
        strength: f64,
    ) {
        if self.bytes.is_empty() || channel >= self.channels {
            return;
        }
        let texel = f64::from(self.texel);
        let cx = (f64::from(center.0) - f64::from(self.origin.0)) / texel;
        let cz = (f64::from(center.1) - f64::from(self.origin.1)) / texel;
        let r = (radius + feather) / texel;
        let x0 = ((cx - r) as i64).max(0);
        let x1 = (((cx + r) as i64) + 1).min(self.width as i64 - 1);
        let z0 = ((cz - r) as i64).max(0);
        let z1 = (((cz + r) as i64) + 1).min(self.height as i64 - 1);
        for iz in z0..=z1 {
            for ix in x0..=x1 {
                let v = ((ix as f64 - cx) as f32, (iz as f64 - cz) as f32);
                let d = f64::from((v.0 * v.0 + v.1 * v.1).sqrt()) * texel;
                if d > radius + feather {
                    continue;
                }
                let v =
                    (1.0 - Self::smooth(radius - feather * 0.5, radius + feather, d)) * strength;
                let i = (iz as usize * self.width + ix as usize) * self.channels + channel;
                let current = f64::from((f64::from(self.bytes[i]) / 255.0) as f32);
                if v > current {
                    self.bytes[i] = (f64::from(v as f32) * 255.0).clamp(0.0, 255.0) as u8;
                    self.dirty = true;
                }
            }
        }
    }

    /// Godot `smoothstep` (equal edges: a step).
    fn smooth(from: f64, to: f64, x: f64) -> f64 {
        if from == to || (from - to).abs() < (1e-5 * from.abs()).max(1e-5) {
            return if x <= from { 0.0 } else { 1.0 };
        }
        let s = ((x - from) / (to - from)).clamp(0.0, 1.0);
        s * s * (3.0 - 2.0 * s)
    }

    /// See [`StampMap::raise_pixel`].
    fn raise_pixel(&mut self, ix: i64, iz: i64, channel: usize, value: f64) {
        if ix < 0 || iz < 0 || ix >= self.width as i64 || iz >= self.height as i64 {
            return;
        }
        if channel >= self.channels {
            return;
        }
        let i = (iz as usize * self.width + ix as usize) * self.channels + channel;
        let current = f64::from((f64::from(self.bytes[i]) / 255.0) as f32);
        if value > current {
            self.bytes[i] = (f64::from(value as f32) * 255.0).clamp(0.0, 255.0) as u8;
            self.dirty = true;
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
        let channels = match channels {
            ..=1 => 1,
            2 => 2,
            _ => 4,
        };
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

    /// Soft-edged disc into `channel`, keeping the maximum (`strength` ceiling):
    /// the splatmap stamp of `battle_terrain.gd`, `feather` metres of fade.
    #[func]
    fn stamp_soft_disc(
        &mut self,
        center: Vector2,
        radius: f64,
        channel: i64,
        feather: f64,
        strength: f64,
    ) {
        self.marks.stamp_soft_disc(
            vec2_pair(center),
            radius,
            channel.max(0) as usize,
            feather,
            strength,
        );
    }

    /// Raises `channel` of texel (ix, iz) to `value` (0-1) if it is lower.
    #[func]
    fn raise_pixel(&mut self, ix: i64, iz: i64, channel: i64, value: f64) {
        self.marks
            .raise_pixel(ix, iz, channel.max(0) as usize, value);
    }

    /// Raises `channel` of the whole column `ix` to `value` (0-1) where it is lower.
    #[func]
    fn raise_column(&mut self, ix: i64, channel: i64, value: f64) {
        for iz in 0..self.marks.height as i64 {
            self.marks
                .raise_pixel(ix, iz, channel.max(0) as usize, value);
        }
    }

    /// Texel value (0-1) of `channel` at (ix, iz), 0 outside.
    #[func]
    fn pixel(&self, ix: i64, iz: i64, channel: i64) -> f64 {
        let m = &self.marks;
        let ch = channel.max(0) as usize;
        if ix < 0 || iz < 0 || ix >= m.width as i64 || iz >= m.height as i64 || ch >= m.channels {
            return 0.0;
        }
        f64::from(m.bytes[(iz as usize * m.width + ix as usize) * m.channels + ch]) / 255.0
    }

    /// The map as a new `Image` (RGBA8 for 4 channels).
    #[func]
    fn image(&self) -> Gd<Image> {
        let m = &self.marks;
        let bytes = PackedByteArray::from(m.bytes.as_slice());
        Image::create_from_data(m.width as i32, m.height as i32, false, m.format(), &bytes)
            .unwrap_or_else(Image::new_gd)
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
        match self.channels {
            4 => Format::RGBA8,
            2 => Format::RG8,
            _ => Format::L8,
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

    #[test]
    fn soft_disc_keeps_the_maximum_and_fades() {
        // 4-channel map, 4 m texels from (-40, -40): the GDScript `_stamp_disc` arithmetic.
        let mut m = map(40, 40, 4);
        m.stamp_soft_disc((20.0, 20.0), 12.0, 1, 8.0, 1.0);
        let at = |m: &Marks, x: f32, z: f32, ch: usize| {
            let ix = ((x - m.origin.0) / m.texel) as usize;
            let iz = ((z - m.origin.1) / m.texel) as usize;
            m.bytes[(iz * m.width + ix) * 4 + ch]
        };
        assert_eq!(at(&m, 20.0, 20.0, 1), 255);
        assert_eq!(at(&m, 20.0, 20.0, 0), 0);
        assert_eq!(at(&m, 20.0, 60.0, 1), 0);
        let edge = at(&m, 36.0, 20.0, 1);
        assert!(edge > 0 && edge < 255, "edge {edge}");
        // A weaker second stamp never lowers the first.
        m.stamp_soft_disc((20.0, 20.0), 12.0, 1, 8.0, 0.4);
        assert_eq!(at(&m, 20.0, 20.0, 1), 255);
        // Reference from the GDScript run before the port: strength 0.5 truncates to 127.
        let mut m = map(40, 40, 4);
        m.stamp_soft_disc((20.0, 20.0), 12.0, 2, 8.0, 0.5);
        assert_eq!(at(&m, 20.0, 20.0, 2), 127);
        m.raise_pixel(0, 0, 3, 0.2);
        assert_eq!(m.bytes[3], 51);
    }
}
