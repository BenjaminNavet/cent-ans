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
    // 1 = Feudal mode: stripes of the second texture (holder) over the first (sovereign).
    feudal: f32,
    // VIS-R2: (band width px, falloff, flat-wash weight, world units per fill texel); w = 0: no band.
    band: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var relief_floor: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var fill_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var fill_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var hatch_texture: texture_2d<f32>;
// Stance colours of the inner band (alpha = edge opacity), same layout as the fill texture.
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var band_texture: texture_2d<f32>;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_xz: vec2<f32>,
    @location(2) world_per_px: f32,
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
    out.world_xz = world.xz;
    let depth = max(-(view.view_from_world * world).z, 1e-3);
    out.world_per_px = 2.0 * depth / (view.clip_from_view[1][1] * view.viewport.w);
    return out;
}

const FILL_SATURATION = 0.55;
// Feudal mode: holder stripes over the sovereign's background (same pattern as the occupation
// bands of border.wesl).
const FEUDAL_PERIOD_PX = 7.0;
const FEUDAL_DUTY = 0.5;

//@include hatch
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

// Neighbouring fill texels of another colour are the edge of a stance region.
const EDGE_EPSILON = 0.02;

fn fill_at(p: vec2<i32>, last: vec2<i32>) -> vec4<f32> {
    return textureLoad(fill_texture, clamp(p, vec2<i32>(0), last), 0);
}

// Distance (texels) to the nearest differing neighbour along `step`: 8 taps in all (4 directions,
// radii 1 and 2 texels) on the fill texture already bound; `own` is the distance from the fragment
// to the border of its own texel on that side. 99 = no edge within two texels.
fn edge_distance(cell: vec2<i32>, last: vec2<i32>, centre: vec4<f32>, step: vec2<i32>, own: f32) -> f32 {
    if length(fill_at(cell + step, last) - centre) > EDGE_EPSILON {
        return own;
    }
    if length(fill_at(cell + step * 2, last) - centre) > EDGE_EPSILON {
        return own + 1.0;
    }
    return 99.0;
}

// Wash ratio of a colour at constant luminance (see `fragment`), 1 = ground unchanged.
fn wash_ratio(rgb: vec3<f32>) -> vec3<f32> {
    let lc = max(dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.02);
    return min(mix(vec3<f32>(1.0), rgb / lc, FILL_SATURATION), vec3<f32>(2.5));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var texel = textureSample(fill_texture, fill_sampler, in.uv);
    if params.feudal > 0.5 {
        let holder = textureSample(hatch_texture, fill_sampler, in.uv);
        if holder.a > 0.01 {
            let stripe = hatch_stripes(in.world_xz, in.world_per_px, FEUDAL_PERIOD_PX, FEUDAL_DUTY);
            texel = vec4<f32>(mix(texel.rgb, holder.rgb, stripe), texel.a);
        }
    }
    // Multiplicative blend (AlphaMode::Multiply): the faction colour at constant luminance
    // (`colorized` of terrain.gdshader, `faction_saturation` 0.55) so the relief is never washed
    // pastel; 1 = ground unchanged.
    let c = texel.rgb * params.tint.rgb;
    let ratio = wash_ratio(c);
    var a = clamp(texel.a * params.tint.a, 0.0, 1.0);
    var wash = mix(vec3<f32>(1.0), ratio, a);
    if params.band.w > 0.0 && params.band.z < 1.0 {
        // VIS-R2: in 3D the stance lives in an inner band that fades towards the province core;
        // the flat wash above only returns with `band.z` (distance / parchment).
        let size = vec2<f32>(textureDimensions(fill_texture));
        let last = vec2<i32>(size) - vec2<i32>(1);
        let p = in.uv * size;
        let cell = vec2<i32>(floor(p));
        let f = p - floor(p);
        let centre = fill_at(cell, last);
        var edge = edge_distance(cell, last, centre, vec2<i32>(1, 0), 1.0 - f.x);
        edge = min(edge, edge_distance(cell, last, centre, vec2<i32>(-1, 0), f.x));
        edge = min(edge, edge_distance(cell, last, centre, vec2<i32>(0, 1), 1.0 - f.y));
        edge = min(edge, edge_distance(cell, last, centre, vec2<i32>(0, -1), f.y));
        let edge_px = edge * params.band.w / max(in.world_per_px, 1e-4);
        let weight = pow(1.0 - clamp(edge_px / max(params.band.x, 1e-3), 0.0, 1.0), params.band.y);
        let stance = textureLoad(band_texture, clamp(cell, vec2<i32>(0), last), 0);
        let band_a = clamp(stance.a * weight, 0.0, 1.0);
        let band_wash = mix(vec3<f32>(1.0), wash_ratio(stance.rgb), band_a);
        wash = mix(band_wash, wash, params.band.z);
        a = mix(band_a, a, params.band.z);
    }
    if params.hatch > 0.5 && texel.a > 0.01 {
        wash = wash * hatch_factor(in.clip.xy, texel.rgb, a);
    }
    return vec4<f32>(wash, 1.0);
}
