// Flipbook billboards (flames and smoke). Port of fx_flipbook.gdshaderinc + fire_flame /
// fire_smoke / life_flame .gdshader. One quad per sprite; the per-instance values travel in the
// MeshTag (no per-sprite material, so sprites batch):
//   bits 0-15  phase (flame particles, 0..1 turns) or age (smoke, 0..1)
//   bits 16-23 age 0..1 (flame particles cool with it) or seed (static flames)
//   bits 24-31 spin angle (turns / 256)
import bevy_pbr::render::mesh_functions::{get_world_from_local, get_tag};
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct Params {
    // x: columns, y: rows, z: mode (0 flame particle, 1 smoke, 2 static flame), w: 1 = upright
    // billboard (base on the origin, turns around the vertical), 0 = camera-facing.
    frames: vec4<f32>,
    // x: emission, y: opacity, z: density, w: loops per second (static flames).
    shape: vec4<f32>,
    // x: young density, y: age darken, z: ember glow energy, w: flicker rate (rad/s).
    extra: vec4<f32>,
    cold: vec4<f32>,
    mid: vec4<f32>,
    hot: vec4<f32>,
    core: vec4<f32>,
    // rgb: smoke colour, a: base alpha.
    smoke: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var sheet: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var sheet_sampler: sampler;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) values: vec3<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let model = get_world_from_local(in.instance_index);
    let tag = get_tag(in.instance_index);
    let phase = f32(tag & 0xFFFFu) / 65535.0;
    let second = f32((tag >> 16u) & 0xFFu) / 255.0;
    let angle = f32(tag >> 24u) / 256.0 * 6.2831853;
    let origin = model[3].xyz;
    let size = vec2<f32>(length(model[0].xyz), length(model[1].xyz));
    let cam_right = view.world_from_view[0].xyz;
    let cam_up = view.world_from_view[1].xyz;
    var world: vec3<f32>;
    if params.frames.w > 0.5 {
        let right = normalize(vec3<f32>(cam_right.x, 0.0, cam_right.z) + vec3<f32>(1e-4, 0.0, 0.0));
        let up = normalize(mix(vec3<f32>(0.0, 1.0, 0.0), cam_up, 0.55));
        world = origin + right * in.position.x * size.x + up * (in.position.y + 0.5) * size.y;
    } else {
        let c = cos(angle);
        let s = sin(angle);
        let right = cam_right * c + cam_up * s;
        let up = cam_up * c - cam_right * s;
        world = origin + right * in.position.x * size.x + up * in.position.y * size.y;
    }
    var out: VertexOutput;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    out.uv = in.uv;
    out.values = vec3<f32>(phase, second, f32(in.instance_index & 1u));
    return out;
}

fn frame_uv(uv: vec2<f32>, frame: f32) -> vec2<f32> {
    let column = frame % params.frames.x;
    let row = floor(frame / params.frames.x);
    return (vec2<f32>(column, row) + uv) / params.frames.xy;
}

fn blackbody(heat: f32) -> vec3<f32> {
    var c = mix(params.cold.rgb, params.mid.rgb, smoothstep(0.0, 0.35, heat));
    c = mix(c, params.hot.rgb, smoothstep(0.35, 0.7, heat));
    return mix(c, params.core.rgb, smoothstep(0.7, 1.0, heat));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let total = params.frames.x * params.frames.y;
    let mode = params.frames.z;
    var first: f32;
    var next: f32;
    var blend: f32;
    if mode > 0.5 && mode < 1.5 {
        // Smoke: age picks the image, no loop.
        let f = clamp(in.values.x, 0.0, 1.0) * (total - 1.0);
        first = floor(f);
        next = min(first + 1.0, total - 1.0);
        blend = f - first;
    } else {
        var turns = in.values.x;
        if mode > 1.5 {
            turns = globals.time * params.shape.w + in.values.y * 7.0;
        }
        let f = fract(turns) * total;
        first = floor(f);
        next = (first + 1.0) % total;
        blend = f - first;
    }
    let texel = mix(
        textureSample(sheet, sheet_sampler, frame_uv(in.uv, first)),
        textureSample(sheet, sheet_sampler, frame_uv(in.uv, next)),
        blend,
    );
    if mode > 0.5 && mode < 1.5 {
        let age = clamp(in.values.x, 0.0, 1.0);
        let fade_in = smoothstep(0.0, 0.06, age);
        let young = mix(params.extra.x, 1.0, smoothstep(0.0, 0.25, age));
        let alpha = clamp(texel.a * params.smoke.a * mix(1.0, 0.6, age) * params.shape.z * fade_in * young, 0.0, 1.0);
        let lighting = texel.g;
        let tint = params.smoke.rgb * mix(1.0, params.extra.y, smoothstep(0.0, 1.0, age));
        let glow = pow(1.0 - age, 7.0) * (1.0 - lighting * 0.7);
        let color = tint * (0.45 + 0.75 * lighting) + vec3<f32>(1.0, 0.38, 0.08) * glow * params.extra.z;
        return vec4<f32>(color, alpha);
    }
    var coverage = texel.a;
    var heat = texel.r;
    var flicker = 1.0;
    if mode < 0.5 {
        let age = in.values.y;
        heat = heat * mix(1.0, 0.55, age);
        coverage = coverage * smoothstep(0.0, 0.12, age) * (1.0 - smoothstep(0.65, 1.0, age));
    } else {
        flicker = 0.85 + 0.15 * sin(globals.time * params.extra.w + in.values.y * 40.0);
    }
    if coverage < 0.01 {
        discard;
    }
    let glow = blackbody(heat) * params.shape.x * (0.35 + heat * 1.2) * flicker;
    return vec4<f32>(glow * coverage, coverage * params.shape.y);
}
