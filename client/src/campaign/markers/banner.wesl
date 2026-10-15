// Wind wave of the procedural banners of the campaign places (port of
// game/shaders/maquette_banner.gdshader + campaign_wind.gdshaderinc, lot AS5). Vertex stage only;
// the fragment is StandardMaterial's (flat faction colour, two-sided).
// The cloth moves along its normal: phase = position in the cloth plane, advanced by the time;
// amplitude (part of the banner size) grows with the shared gust of the map. Amplitude 0: still.
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::forward_io::{Vertex, VertexOutput, decompress_vertex};
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::globals;

struct BannerParams {
    // x amplitude, y banner size, z wave speed, w wave length.
    wave: vec4<f32>,
    // x gust gain, y calm.
    gust: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> banner: BannerParams;

const WIND_GUST_DIR: vec2<f32> = vec2<f32>(0.98, 0.2);
const WIND_GUST_SPEED: f32 = 2.6;
const WIND_GUST_SIZE: vec2<f32> = vec2<f32>(34.0, 16.0);
const WIND_GUST_SHARE: f32 = 0.38;
const WIND_GUST_SOFT: f32 = 0.22;
const TAU: f32 = 6.2831;

fn wind_hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn wind_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(wind_hash(i), wind_hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(wind_hash(i + vec2<f32>(0.0, 1.0)), wind_hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

fn wind_gust(p: vec2<f32>) -> f32 {
    let d = normalize(WIND_GUST_DIR + vec2<f32>(1e-5, 0.0));
    var q = vec2<f32>(dot(p, d), dot(p, vec2<f32>(-d.y, d.x)));
    q.x = q.x - globals.time * WIND_GUST_SPEED;
    let n = wind_noise(q / WIND_GUST_SIZE) * 0.7
        + wind_noise(q / (WIND_GUST_SIZE * 0.45) + vec2<f32>(globals.time * 0.05, 3.7)) * 0.3;
    let edge = 1.0 - WIND_GUST_SHARE;
    return smoothstep(edge - WIND_GUST_SOFT * 0.5, edge + WIND_GUST_SOFT * 0.5, n);
}

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    var local = vertex.position;
    var normal = vec3<f32>(0.0, 0.0, 1.0);
    @if(VERTEX_NORMALS)
    {
        normal = vertex.normal;
    }
    let amplitude = banner.wave.x;
    if amplitude > 0.0 {
        let anchor = world_from_local[3].xyz;
        let gust = wind_gust(anchor.xz);
        let phase = dot(local, vec3<f32>(1.0, 0.6, 1.0)) / max(banner.wave.w * banner.wave.y, 1e-4)
            - globals.time * banner.wave.z + wind_hash(anchor.xz) * TAU;
        let wave = sin(phase) * 0.7 + sin(phase * 2.1 + 1.3) * 0.3;
        local = local + normal * wave * amplitude * banner.wave.y
            * (banner.gust.y + banner.gust.x * gust);
    }
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(local, 1.0));
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);
    @if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);
    @if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    @if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;
    @if(VERTEX_COLORS)
    out.color = vertex.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);
    return out;
}
