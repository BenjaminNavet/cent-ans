// Province colour wash, draped on the exaggerated relief. Mesh: grid, position.y = altitude (m),
// UV0 over the whole map; the texture holds one colour per province (nearest sampling).
import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::mesh_view_bindings::view;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};

struct Params {
    tint: vec4<f32>,
    lift_base: f32,
    lift_per_distance: f32,
    hatch: f32,
    pad1: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var relief_floor: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var fill_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var fill_sampler: sampler;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz);
    var world = get_world_from_local(in.instance_index) * vec4<f32>(in.position.x, ground, in.position.z, 1.0);
    let dist = length(view.world_position - world.xyz);
    world.y = world.y + params.lift_base + params.lift_per_distance * dist;
    var out: VertexOutput;
    out.clip = view.clip_from_world * world;
    out.uv = in.uv;
    return out;
}

const FILL_SATURATION = 0.55;
// Colour-blind hatching: stripes of HATCH_PERIOD screen pixels, a province-colour hash picks one of
// three directions (/, \ and horizontal) so neighbouring realms differ without their colours.
const HATCH_PERIOD = 8.0;
const HATCH_DARKEN = 0.72;

fn hatch_factor(pixel: vec2<f32>, color: vec3<f32>, strength: f32) -> f32 {
    let seed = fract(sin(dot(color, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
    var phase = pixel.x + pixel.y;
    if seed > 0.66 {
        phase = pixel.y * 1.4142;
    } else if seed > 0.33 {
        phase = pixel.x - pixel.y;
    }
    let stripe = step(0.5, fract(phase / HATCH_PERIOD));
    return mix(1.0, HATCH_DARKEN, stripe * clamp(strength * 2.0, 0.0, 1.0));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(fill_texture, fill_sampler, in.uv);
    // Multiplicative blend (AlphaMode::Multiply): the faction colour at constant luminance
    // (`colorized` of terrain.gdshader, `faction_saturation` 0.55) so the relief is never washed
    // pastel; 1 = ground unchanged.
    let c = texel.rgb * params.tint.rgb;
    let lc = max(dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.02);
    let ratio = min(mix(vec3<f32>(1.0), c / lc, FILL_SATURATION), vec3<f32>(2.5));
    let a = clamp(texel.a * params.tint.a, 0.0, 1.0);
    var wash = mix(vec3<f32>(1.0), ratio, a);
    if params.hatch > 0.5 && texel.a > 0.01 {
        wash = wash * hatch_factor(in.clip.xy, texel.rgb, a);
    }
    return vec4<f32>(wash, 1.0);
}
