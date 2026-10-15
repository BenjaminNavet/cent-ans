// Screen-width lines (coast, main/minor roads). Port of terrain_line.gdshader and road_line.gdshader.
// Mesh: centre line, NORMAL = signed unit perpendicular (map space), UV0 = (world width, side +-1).
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::get_world_from_local;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
import bevy_pbr::render::mesh_view_bindings::view;

struct Params {
    fill_color: vec4<f32>,
    casing_color: vec4<f32>,
    alpha: f32,
    near_px: f32,
    far_px: f32,
    near_distance: f32,
    far_distance: f32,
    far_alpha: f32,
    casing_px: f32,
    thin_fade: f32,
    lift: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var relief_floor: texture_2d<f32>;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) side: f32,
    @location(1) px: f32,
    @location(2) fade: f32,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_from_local = get_world_from_local(in.instance_index);
    // position.y is the altitude in metres: drape it on the exaggerated relief.
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz) + params.lift;
    var world = world_from_local * vec4<f32>(in.position.x, ground, in.position.z, 1.0);
    let view_pos = (view.view_from_world * world).xyz;
    let dist = max(length(view_pos), 0.01);
    let world_per_px = dist * 2.0 / (abs(view.clip_from_view[1][1]) * view.viewport.w);
    let t = smoothstep(params.near_distance, params.far_distance, dist);
    let width = in.uv.x;
    let px = max(mix(params.near_px, params.far_px, t), width / world_per_px);
    let true_px = width / world_per_px;
    let dir = (world_from_local * vec4<f32>(in.normal, 0.0)).xyz;
    world = vec4<f32>(world.xyz + dir * 0.5 * px * world_per_px, 1.0);
    var out: VertexOutput;
    out.clip = view.clip_from_world * world;
    out.side = in.uv.y;
    out.px = px;
    let thin = mix(1.0, clamp(true_px / max(px, 0.001), 0.25, 1.0), params.thin_fade);
    out.fade = mix(1.0, params.far_alpha, t) * thin;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let edge = abs(in.side);
    let core = clamp(1.0 - 2.0 * params.casing_px / max(in.px, 0.01), 0.0, 1.0);
    let aa = 1.0 / max(in.px, 1.0);
    let casing = smoothstep(core - aa, core + aa, edge);
    let rgb = mix(params.fill_color.rgb, params.casing_color.rgb, casing);
    let soft = 1.0 - smoothstep(1.0 - aa * 1.5, 1.0, edge);
    let a = params.alpha * in.fade * soft * mix(params.fill_color.a, params.casing_color.a, casing);
    return vec4<f32>(rgb, a);
}
