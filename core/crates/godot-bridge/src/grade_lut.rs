//! Colour-grade LUT baking. Rendering support only, no game rule.
//!
//! Port of the former GDScript `AtmosphereLibrary.grade_lut` texel loop: grades (white balance,
//! lift/gain, gamma, shadow/highlight toning, S contrast, saturation) are composed in order and
//! blended with the identity by `strength`. Arithmetic keeps the GDScript precision split
//! (`f32` vectors, `f64` scalars) so that the 8-bit result matches.

use godot::classes::RefCounted;
use godot::prelude::*;
use serde_json::Value;

type V3 = [f32; 3];

const LUMA: V3 = [0.2126, 0.7152, 0.0722];

struct Grade {
    balance: V3,
    lift: V3,
    gamma: V3,
    gain: V3,
    contrast: f64,
    saturation: f64,
    shadow_saturation: f64,
    shadows: V3,
    highlights: V3,
}

/// Bakes the 3D grading LUT as raw RGB8 bytes.
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct GradeLut {
    base: Base<RefCounted>,
}

#[godot_api]
impl GradeLut {
    /// `size`³ texels, red fastest then green then blue (layer), 3 bytes per texel. `grades_json`
    /// is a JSON array of grade objects (empty ones ignored); `strength` blends with identity.
    #[func]
    fn bake(grades_json: GString, strength: f64, size: i64) -> PackedByteArray {
        let parsed: Value = serde_json::from_str(&grades_json.to_string()).unwrap_or(Value::Null);
        let grades: Vec<Grade> = parsed
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_object)
                    .filter(|object| !object.is_empty())
                    .map(|object| prepare(&Value::Object(object.clone())))
                    .collect()
            })
            .unwrap_or_default();
        PackedByteArray::from(bake_bytes(&grades, strength, size.clamp(2, 128) as usize).as_slice())
    }
}

fn number(value: &Value, key: &str, fallback: f64) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(fallback)
}

fn vector(value: &Value, key: &str, fallback: V3) -> V3 {
    match value.get(key).and_then(Value::as_array) {
        Some(list) if list.len() >= 3 => {
            let at = |i: usize| list[i].as_f64().unwrap_or(0.0) as f32;
            [at(0), at(1), at(2)]
        }
        _ => fallback,
    }
}

fn prepare(grade: &Value) -> Grade {
    let temperature = number(grade, "temperature", 0.0);
    let tint = number(grade, "tint", 0.0);
    Grade {
        balance: [
            (1.0 + 0.1 * temperature) as f32,
            (1.0 - 0.06 * tint) as f32,
            (1.0 - 0.12 * temperature) as f32,
        ],
        lift: vector(grade, "lift", [0.0; 3]),
        gamma: vector(grade, "gamma", [1.0; 3]),
        gain: vector(grade, "gain", [1.0; 3]),
        contrast: number(grade, "contrast", 1.0),
        saturation: number(grade, "saturation", 1.0),
        shadow_saturation: number(grade, "shadow_saturation", 1.0),
        shadows: vector(grade, "shadows", [1.0; 3]),
        highlights: vector(grade, "highlights", [1.0; 3]),
    }
}

fn bake_bytes(grades: &[Grade], strength: f64, size: usize) -> Vec<u8> {
    let scale = 1.0 / (size - 1) as f64;
    let weight = strength as f32;
    let mut bytes = Vec::with_capacity(size * size * size * 3);
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                let source: V3 = [
                    (r as f64 * scale) as f32,
                    (g as f64 * scale) as f32,
                    (b as f64 * scale) as f32,
                ];
                let mut color = source;
                for grade in grades {
                    color = grade_texel(color, grade);
                }
                for axis in 0..3 {
                    let mixed = source[axis] + (color[axis] - source[axis]) * weight;
                    bytes.push((f64::from(mixed).clamp(0.0, 1.0) * 255.0) as u8);
                }
            }
        }
    }
    bytes
}

fn dot(a: V3, b: V3) -> f64 {
    f64::from(a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
}

/// GDScript `is_equal_approx` on floats.
fn approx_one(value: f64) -> bool {
    if value == 1.0 {
        return true;
    }
    let tolerance = (1e-5 * 1.0f64).max(1e-5);
    (value - 1.0).abs() < tolerance
}

fn smoothstep(from: f64, to: f64, x: f64) -> f64 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn soft_clip(x: f64) -> f64 {
    if x < 0.05 {
        return if x > -1.0 {
            0.05 * ((x - 0.05) / 0.05).exp()
        } else {
            0.0
        };
    }
    if x > 0.95 {
        return 1.0 - 0.05 * (-(x - 0.95) / 0.05).exp();
    }
    x
}

fn grade_texel(mut c: V3, g: &Grade) -> V3 {
    for i in 0..3 {
        c[i] *= g.balance[i];
        c[i] = (c[i] + g.lift[i] * (1.0 - c[i])) * g.gain[i];
        c[i] = (f64::from(c[i]).max(0.0)).powf(1.0 / f64::from(g.gamma[i])) as f32;
    }
    let mut luma = dot(c, LUMA);
    let weight = smoothstep(0.0, 1.0, luma) as f32;
    for i in 0..3 {
        let toning = g.shadows[i] + (g.highlights[i] - g.shadows[i]) * weight;
        c[i] *= toning;
    }
    if !approx_one(g.contrast) {
        for value in &mut c {
            let linear = 0.45f32 + (*value - 0.45f32) * g.contrast as f32;
            *value = soft_clip(f64::from(linear)) as f32;
        }
    }
    if !approx_one(g.saturation) {
        luma = dot(c, LUMA);
        let (l, s) = (luma as f32, g.saturation as f32);
        for value in &mut c {
            *value = l + (*value - l) * s;
        }
    }
    if !approx_one(g.shadow_saturation) {
        luma = dot(c, LUMA);
        let keep =
            (g.shadow_saturation + (1.0 - g.shadow_saturation) * smoothstep(0.0, 0.5, luma)) as f32;
        let l = luma as f32;
        for value in &mut c {
            *value = l + (*value - l) * keep;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_without_grades() {
        let bytes = bake_bytes(&[], 1.0, 4);
        assert_eq!(bytes.len(), 4 * 4 * 4 * 3);
        assert_eq!(&bytes[0..3], &[0, 0, 0]);
        assert_eq!(&bytes[bytes.len() - 3..], &[255, 255, 255]);
    }
}
