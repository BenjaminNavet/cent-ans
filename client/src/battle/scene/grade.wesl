// Battle colour grading after tonemapping (port of the 3D LUT of AtmosphereLibrary.grade_lut,
// texel formula of core/crates/godot-bridge/src/grade_lut.rs). Godot baked the season, weather
// and hour grades into a LUT applied in sRGB after tonemapping; here the same formula runs per
// pixel (no LUT quantisation): white balance, lift / gain / gamma per channel, shadow and
// highlight toning, S contrast around 0.45 with a soft clip, saturation, shadow saturation, then
// a blend with the source by `strength`.
import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

struct Grade {
    balance: vec4<f32>,
    lift: vec4<f32>,
    gamma: vec4<f32>,
    gain: vec4<f32>,
    shadows: vec4<f32>,
    highlights: vec4<f32>,
    // x: contrast, y: saturation, z: shadow saturation, w: 1 when the grade is used.
    scalars: vec4<f32>,
}

struct BattleGrade {
    a: Grade,
    b: Grade,
    c: Grade,
    // x: strength, yzw: unused.
    global: vec4<f32>,
}

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> grade: BattleGrade;

const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn soft_clip(x: f32) -> f32 {
    if x < 0.05 {
        if x > -1.0 {
            return 0.05 * exp((x - 0.05) / 0.05);
        }
        return 0.0;
    }
    if x > 0.95 {
        return 1.0 - 0.05 * exp(-(x - 0.95) / 0.05);
    }
    return x;
}

fn apply(c0: vec3<f32>, g: Grade) -> vec3<f32> {
    if g.scalars.w < 0.5 {
        return c0;
    }
    var c = c0 * g.balance.rgb;
    c = (c + g.lift.rgb * (vec3<f32>(1.0) - c)) * g.gain.rgb;
    c = pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0) / g.gamma.rgb);
    var luma = dot(c, LUMA);
    let weight = smoothstep(0.0, 1.0, luma);
    c = c * mix(g.shadows.rgb, g.highlights.rgb, weight);
    if abs(g.scalars.x - 1.0) > 1e-5 {
        let l = vec3<f32>(0.45) + (c - vec3<f32>(0.45)) * g.scalars.x;
        c = vec3<f32>(soft_clip(l.r), soft_clip(l.g), soft_clip(l.b));
    }
    if abs(g.scalars.y - 1.0) > 1e-5 {
        luma = dot(c, LUMA);
        c = vec3<f32>(luma) + (c - vec3<f32>(luma)) * g.scalars.y;
    }
    if abs(g.scalars.z - 1.0) > 1e-5 {
        luma = dot(c, LUMA);
        let keep = g.scalars.z + (1.0 - g.scalars.z) * smoothstep(0.0, 0.5, luma);
        c = vec3<f32>(luma) + (c - vec3<f32>(luma)) * keep;
    }
    return c;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(screen, screen_sampler, in.uv);
    let s = clamp(to_srgb(source.rgb), vec3<f32>(0.0), vec3<f32>(1.0));
    var c = apply(s, grade.a);
    c = apply(c, grade.b);
    c = apply(c, grade.c);
    c = clamp(s + (c - s) * grade.global.x, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(to_linear(c), source.a);
}
