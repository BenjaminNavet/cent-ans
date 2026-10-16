// Sea lanes drawn in ink on the sea like a portolan chart (sea_lane.gdshader, lot SL1).
// Mesh: centre line (position.y = altitude in METRES, draped), NORMAL = perpendicular (map space),
// UV0 = (world width, side +-1), UV1.x = arc length (world). Width and dashes are held in screen
// pixels so the route stays legible and regular at any zoom.
import bevy_pbr::render::mesh_functions::get_world_from_local;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
import bevy_pbr::render::mesh_view_bindings::view;

struct Params {
    color: vec4<f32>,
    // x: min px, y: max px, z: dash period px, w: dash fill (0-1).
    a: vec4<f32>,
    // x: world lift above the draped ground, y: alpha (visibility fade).
    b: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var relief_floor: texture_2d<f32>;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv2: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) side: f32,
    @location(1) arc_px: f32,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_from_local = get_world_from_local(in.instance_index);
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz) + params.b.x;
    var world = world_from_local * vec4<f32>(in.position.x, ground, in.position.z, 1.0);
    let view_pos = (view.view_from_world * world).xyz;
    let dist = max(length(view_pos), 0.01);
    let world_per_px = dist * 2.0 / (abs(view.clip_from_view[1][1]) * view.viewport.w);
    let half_w = 0.5 * clamp(in.uv.x, params.a.x * world_per_px, params.a.y * world_per_px);
    let dir = (world_from_local * vec4<f32>(in.normal, 0.0)).xyz;
    world = vec4<f32>(world.xyz + dir * half_w, 1.0);
    var out: VertexOutput;
    out.clip = view.clip_from_world * world;
    out.side = in.uv.y;
    out.arc_px = in.uv2.x / world_per_px;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let soft = 1.0 - smoothstep(0.55, 1.0, abs(in.side));
    let phase = fract(in.arc_px / max(params.a.z, 1.0));
    let dash = 1.0 - smoothstep(params.a.w - 0.06, params.a.w, phase);
    return vec4<f32>(params.color.rgb, params.color.a * soft * dash * params.b.y);
}
