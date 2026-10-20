// Campaign post pass (after the tonemapper, campaign camera only: the UI has its own camera).
// 1. TS-T3 atmosphere from the view depth: valley mist by altitude (the low ground fills, the
//    peaks emerge), aerial perspective (the far land dissolves into a cold grey-blue), both
//    colder in winter / snow / the north / high up. It reads the depth buffer, so it covers the
//    whole world (ground, trees, towns, armies).
// 2. Season colour grade of the environment, then Godot `apply_bcs` (contrast then saturation).
// 3. TS-T3 grade: weighted desaturation (plain green above all) sparing the already saturated,
//    warm or bright hues (banners, fires, glows, enemy red), and a value S-curve.
import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;

struct CampaignAdjust {
    // x: contrast, y: saturation (legacy apply_bcs), z: value contrast, w: 1 = TS-T3 grade on.
    params: vec4<f32>,
    // Environment grade: x saturation, y contrast, z shadow saturation, w strength.
    grade: vec4<f32>,
    lift: vec4<f32>,
    highlights: vec4<f32>,
    // x desaturation, y extra green desaturation, zw chroma protected from z to w.
    desat: vec4<f32>,
    // x warm protection, yz luma protected from y to z, w master strength (0 on the parchment).
    desat2: vec4<f32>,
    // Valley mist: x full altitude (m), y top altitude (m), z opacity, w share near the focus.
    mist_a: vec4<f32>,
    // xy: distance ramp (world) from the near share to full, z noise scale (1/px), w noise amount.
    mist_b: vec4<f32>,
    // xy: noise drift offset (px), z share over the sea, w metres per world unit of height.
    mist_c: vec4<f32>,
    // rgb: mist colour (linear), w: coldness at the focus (0..1).
    mist_color: vec4<f32>,
    // rgb: cold mist colour (linear), w: unused.
    mist_cold: vec4<f32>,
    // Aerial perspective: x begin, y end (world distance), z strength, w desaturation.
    aerial_a: vec4<f32>,
    // rgb: haze colour (linear), w: share applied to the sky.
    aerial_color: vec4<f32>,
    // rgb: cold haze colour (linear), w: unused.
    aerial_cold: vec4<f32>,
    // x, y: altitude (m) ramp of the cold, z: its share, w: unused.
    cold_alt: vec4<f32>,
    // Relative hollows: xy depth (m) below the neighbourhood from no mist to full, z tap radius
    // (share of the screen height), w share of the plain (absolute low-ground) mist.
    mist_d: vec4<f32>,
}

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> adjust: CampaignAdjust;
@group(0) @binding(3) var<uniform> view: View;
@if(MULTISAMPLED)
@group(0) @binding(4) var depth_texture: texture_depth_multisampled_2d;
@else
@group(0) @binding(4) var depth_texture: texture_depth_2d;

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

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1030));
    let r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn scene_depth(uv: vec2<f32>) -> f32 {
    let size = vec2<f32>(textureDimensions(depth_texture));
    let coord = vec2<i32>(clamp(uv * size, vec2<f32>(0.0), size - 1.0));
    return textureLoad(depth_texture, coord, 0);
}

// World height at screen `uv`, w = 0 on the sky.
fn world_y_at(uv: vec2<f32>) -> vec2<f32> {
    let d = scene_depth(uv);
    if (d <= 0.0) {
        return vec2<f32>(0.0, 0.0);
    }
    let w = view.world_from_clip * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, d, 1.0);
    return vec2<f32>(w.y / w.w, 1.0);
}

// How far (world height) the pixel sits below the mean of a screen-space ring of neighbours:
// zero on a plain or a regular slope (symmetric taps average out), positive in a valley floor
// or a river bottom, negative on a crest.
fn hollow_depth(uv: vec2<f32>, y: f32) -> f32 {
    let size = vec2<f32>(textureDimensions(depth_texture));
    let r = vec2<f32>(adjust.mist_d.z * size.y / size.x, adjust.mist_d.z);
    var sum = 0.0;
    var count = 0.0;
    for (var i = 0; i < 8; i = i + 1) {
        let a = f32(i) * 0.785398 + 0.3;
        let dir = vec2<f32>(cos(a), sin(a));
        for (var k = 0; k < 2; k = k + 1) {
            let q = uv + dir * r * (0.5 + 0.5 * f32(k));
            let s = world_y_at(clamp(q, vec2<f32>(0.001), vec2<f32>(0.999)));
            sum = sum + s.x * s.y;
            count = count + s.y;
        }
    }
    if (count < 4.0) {
        return 0.0;
    }
    return sum / count - y;
}

// Valley mist and aerial perspective on a linear colour.
fn atmosphere(c: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let master = adjust.desat2.w;
    if (master <= 0.0) {
        return c;
    }
    let depth = scene_depth(uv);
    let coldness = adjust.mist_color.w;
    // Sky (reverse-Z cleared to 0): only a share of the far haze, so the horizon stays one veil.
    if (depth <= 0.0) {
        let a = adjust.aerial_a.z * adjust.aerial_color.w * master;
        let haze = mix(adjust.aerial_color.rgb, adjust.aerial_cold.rgb, coldness);
        return mix(c, haze, a);
    }
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let w = view.world_from_clip * ndc;
    let world = w.xyz / w.w;
    let dist = distance(world, view.world_position);
    var out = c;
    // Valley mist: in the relative hollows of the relief (valleys, river bottoms) in partial
    // banks; the absolute low ground (plains) only gets the `mist_d.w` share (morning, fog).
    let alt_raw = world.y * adjust.mist_c.w;
    let alt = max(alt_raw, 0.0);
    let cold_here = clamp(coldness + adjust.cold_alt.z * smoothstep(adjust.cold_alt.x, adjust.cold_alt.y, alt), 0.0, 1.0);
    if (adjust.mist_a.z > 0.0) {
        let low = 1.0 - smoothstep(adjust.mist_a.x, adjust.mist_a.y, alt);
        let hollow = hollow_depth(uv, world.y) * adjust.mist_c.w;
        let rel = smoothstep(adjust.mist_d.x, adjust.mist_d.y, hollow);
        let shape = max(rel * (0.35 + 0.65 * low), low * low * adjust.mist_d.w);
        let p = world.xz * adjust.mist_b.z + adjust.mist_c.xy * adjust.mist_b.z;
        let n = vnoise(p) * 0.65 + vnoise(p * 2.7 + 5.3) * 0.35;
        let banks = mix(1.0, smoothstep(0.2, 0.8, n), adjust.mist_b.w);
        let far = mix(adjust.mist_a.w, 1.0, smoothstep(adjust.mist_b.x, adjust.mist_b.y, dist));
        let sea = select(1.0, adjust.mist_c.z, alt_raw < -0.5);
        let mist = clamp(shape * banks * far * sea * adjust.mist_a.z, 0.0, 1.0) * master;
        let colour = mix(adjust.mist_color.rgb, adjust.mist_cold.rgb, cold_here);
        out = mix(out, colour, mist);
    }
    // Aerial perspective: the far land loses its colour, then dissolves into the cold haze.
    if (adjust.aerial_a.z > 0.0) {
        let a = smoothstep(adjust.aerial_a.x, adjust.aerial_a.y, dist) * adjust.aerial_a.z * master;
        out = mix(out, vec3<f32>(luma(out)), a * adjust.aerial_a.w);
        let haze = mix(adjust.aerial_color.rgb, adjust.aerial_cold.rgb, cold_here);
        out = mix(out, haze, a);
    }
    return out;
}

// Colour grade of the campaign environment (`campaign.seasons.<season>.grade`), once for the whole
// frame (ground, sea, rivers, towns, trees, armies): contrast about the mid grey, lift in the
// shadows, highlight tint, saturation that drops further in the shadows. Linear values.
fn campaign_grade(c: vec3<f32>) -> vec3<f32> {
    if (adjust.grade.w <= 0.0) {
        return c;
    }
    var col = (c - vec3<f32>(0.5)) * adjust.grade.y + vec3<f32>(0.5);
    col = max(col, vec3<f32>(0.0));
    let l = luma(col);
    col = col + adjust.lift.rgb * (1.0 - smoothstep(0.0, 0.5, l));
    col = col * mix(vec3<f32>(1.0), adjust.highlights.rgb, smoothstep(0.3, 0.9, l));
    let sat = mix(adjust.grade.z, adjust.grade.x, smoothstep(0.02, 0.35, l));
    col = vec3<f32>(l) + (col - vec3<f32>(l)) * sat;
    return mix(c, max(col, vec3<f32>(0.0)), adjust.grade.w);
}

// TS-T3 grade on display values: desaturate the dull and green hues, spare the vivid, warm and
// bright ones, then a gentle S-curve on the values.
fn ts_grade(s: vec3<f32>) -> vec3<f32> {
    if (adjust.params.w <= 0.0) {
        return s;
    }
    let mx = max(s.r, max(s.g, s.b));
    let mn = min(s.r, min(s.g, s.b));
    let chroma = mx - mn;
    let inv = 1.0 / max(chroma, 1e-3);
    let green = clamp((s.g - max(s.r, s.b)) * inv, 0.0, 1.0) * smoothstep(0.02, 0.1, chroma);
    let warm = clamp((s.r - max(s.g, s.b)) * inv, 0.0, 1.0) * smoothstep(0.12, 0.3, chroma);
    var protect = smoothstep(adjust.desat.z, adjust.desat.w, chroma);
    protect = max(protect, warm * adjust.desat2.x);
    protect = max(protect, smoothstep(adjust.desat2.y, adjust.desat2.z, mx));
    let amount = clamp((adjust.desat.x + adjust.desat.y * green) * (1.0 - protect), 0.0, 1.0);
    var out = mix(s, vec3<f32>(dot(s, vec3<f32>(0.2126, 0.7152, 0.0722))), amount);
    let x = clamp(out, vec3<f32>(0.0), vec3<f32>(1.0));
    out = mix(out, x * x * (3.0 - 2.0 * x), adjust.params.z);
    return out;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let source = textureSample(screen, screen_sampler, in.uv);
    let aired = atmosphere(source.rgb, in.uv);
    let graded = clamp(campaign_grade(aired), vec3<f32>(0.0), vec3<f32>(1.0));
    var s = to_srgb(graded);
    s = mix(vec3<f32>(0.5), s, adjust.params.x);
    s = mix(vec3<f32>(dot(s, vec3<f32>(1.0 / 3.0))), s, adjust.params.y);
    s = ts_grade(clamp(s, vec3<f32>(0.0), vec3<f32>(1.0)));
    return vec4<f32>(to_linear(clamp(s, vec3<f32>(0.0), vec3<f32>(1.0))), source.a);
}
