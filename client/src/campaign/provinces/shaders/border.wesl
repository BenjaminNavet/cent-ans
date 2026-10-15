// Province and realm border ribbons, draped on the exaggerated relief. Port of the band geometry of
// game/shaders/faction_borders.gdshaderinc: widths are in SCREEN pixels, so the ribbon is expanded
// here, per vertex, by `offset_px * (world units per screen pixel)` along the inward direction.
//
// Mesh: position = (x, altitude in m, z) of the edge point; UV0 = inward unit direction (map xz);
// UV1 = (offset_px, param): signed distance from the edge line in pixels (baked, before the view
// scale) and the band kind: 0 = solid, > 0 = glow of that extent in px (gradient (1 - u)^exponent),
// < 0 = occupation hatching over a band of -param px (soft fall-off 0.6..1, diagonal stripes).
// COLOR = linear rgba per vertex.
import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::mesh_view_bindings::view;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};

struct Params {
    tint: vec4<f32>,
    lift_base: f32,
    lift_per_distance: f32,
    pad0: f32,
    pad1: f32,
}

struct Shape {
    // Zoom / parchment width factor (`fr1_width_scale`, parchment_width_scale), applied to every offset.
    width_scale: f32,
    // Extra factor of the glow extent (parchment_glow_scale).
    glow_scale: f32,
    glow_exponent: f32,
    hatch_period_px: f32,
    hatch_duty: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var relief_floor: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> shape: Shape;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) dir: vec2<f32>,
    @location(3) band: vec2<f32>,
    @location(5) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    // x = u (offset / extent), y = band kind sign, z = world units per screen pixel.
    @location(1) info: vec3<f32>,
    @location(2) world_xz: vec2<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz);
    var world = get_world_from_local(in.instance_index) * vec4<f32>(in.position.x, ground, in.position.z, 1.0);
    let dist = length(view.world_position - world.xyz);
    // World size of one screen pixel at this depth (perspective: clip_from_view[1][1] = 1 / tan(fov / 2)).
    let depth = max(-(view.view_from_world * world).z, 1e-3);
    let world_per_px = 2.0 * depth / (view.clip_from_view[1][1] * view.viewport.w);
    var scale = shape.width_scale;
    if in.band.y > 0.0 {
        scale = scale * shape.glow_scale;
    }
    let shifted = in.dir * (in.band.x * scale * world_per_px);
    world = vec4<f32>(world.x + shifted.x, world.y + params.lift_base + params.lift_per_distance * dist, world.z + shifted.y, 1.0);
    var out: VertexOutput;
    out.clip = view.clip_from_world * world;
    out.color = in.color;
    let extent = max(abs(in.band.y), 1e-3);
    out.info = vec3<f32>(select(0.0, in.band.x / extent, in.band.y != 0.0), sign(in.band.y), world_per_px);
    out.world_xz = world.xz;
    return out;
}

// Diagonal hatching fixed to the map, period in screen pixels: two scale octaves cross-faded
// (fr1_hatch_at).
fn hatch_at(p: vec2<f32>, world_per_px: f32) -> f32 {
    let lv = log2(max(world_per_px, 1e-4));
    let l0 = floor(lv);
    let s0 = exp2(l0) * shape.hatch_period_px;
    let c = (p.x + p.y) * 0.70710678;
    let a0 = fract(c / s0);
    let a1 = fract(c / (s0 * 2.0));
    let edge = 0.08;
    let h0 = smoothstep(0.0, edge, a0) * (1.0 - smoothstep(shape.hatch_duty, shape.hatch_duty + edge, a0));
    let h1 = smoothstep(0.0, edge, a1) * (1.0 - smoothstep(shape.hatch_duty, shape.hatch_duty + edge, a1));
    return mix(h0, h1, lv - l0);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var alpha = in.color.a * params.tint.a;
    let u = clamp(in.info.x, 0.0, 1.0);
    if in.info.y > 0.0 {
        alpha = alpha * pow(1.0 - u, shape.glow_exponent);
    } else if in.info.y < 0.0 {
        alpha = alpha * (1.0 - smoothstep(0.6, 1.0, u)) * hatch_at(in.world_xz, in.info.z);
    }
    return vec4<f32>(in.color.rgb * params.tint.rgb, alpha);
}
