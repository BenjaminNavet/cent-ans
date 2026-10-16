// Fresh water of the campaign map (freshwater_surface.gdshader + freshwater_pool.gdshader, lot DN-ME4).
// One material, four modes: 0 salt pan (COLOR per vertex), 1 torrent foam (ribbon, UV.x along the
// course), 2 ford, 3 pond/lagoon disc (instances merged per block).
// Vertices: position = (map x, altitude in METRES, map z), draped on the exaggerated relief;
// normal.xz = offset of the corner from the pond centre (world units, scaled by `fw_scale`);
// uv; color (pond: rgb = water colour, a = seed).
import bevy_pbr::render::mesh_functions::get_world_from_local;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct Params {
    foam_color: vec4<f32>,
    sky_color: vec4<f32>,
    // x: mode, y: fade start, z: fade end, w: bias factor.
    a: vec4<f32>,
    // x: flow speed, y: pond scale (fw_scale), z: pond alpha (fw_alpha), w: pond sky reflection.
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
    @location(5) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) dist: f32,
    @location(3) up_dot_view: f32,
}

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (3.0 - 2.0 * f0);
    return mix(mix(hash12(i), hash12(i + vec2<f32>(1.0, 0.0)), f.x), mix(hash12(i + vec2<f32>(0.0, 1.0)), hash12(i + vec2<f32>(1.0, 1.0)), f.x), f.y);
}

// World lift above the draped ground, per mode (Godot builds: pans 0.004, foam 0.01, ford 0.006, pond 0.002).
fn lift_for(mode: i32) -> f32 {
    if mode == 0 { return 0.004; }
    if mode == 1 { return 0.01; }
    if mode == 2 { return 0.006; }
    return 0.002;
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let mode = i32(params.a.x + 0.5);
    let world_from_local = get_world_from_local(in.instance_index);
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz) + lift_for(mode);
    var local = vec3<f32>(in.position.x, ground, in.position.z);
    if mode == 3 {
        local.x = local.x + in.normal.x * params.b.y;
        local.z = local.z + in.normal.z * params.b.y;
    }
    var world = world_from_local * vec4<f32>(local, 1.0);
    // Pulled towards the camera: the displayed surface (coarser far away) does not hide the water.
    let to_cam = view.world_position - world.xyz;
    let dist = length(to_cam);
    world = vec4<f32>(world.xyz + normalize(to_cam) * dist * params.a.w, 1.0);
    var out: VertexOutput;
    out.clip = view.clip_from_world * world;
    out.uv = in.uv;
    out.color = in.color;
    out.dist = dist;
    out.up_dot_view = clamp(to_cam.y / max(dist, 1e-4), 0.0, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let mode = i32(params.a.x + 0.5);
    let fade = 1.0 - smoothstep(params.a.y, params.a.z, in.dist);
    var rgb = vec3<f32>(0.0);
    var alpha = 0.0;
    if mode == 0 {
        rgb = in.color.rgb;
        alpha = in.color.a * fade;
    } else if mode == 1 {
        let across = abs(in.uv.y - 0.5) * 2.0;
        let t = globals.time * params.b.x;
        let n = vnoise(vec2<f32>(in.uv.x * 5.0 - t, in.uv.y * 6.0)) * 0.6 + vnoise(vec2<f32>(in.uv.x * 13.0 - t * 1.7, in.uv.y * 14.0)) * 0.4;
        let a = smoothstep(0.35, 0.7, n) * (1.0 - smoothstep(0.55, 1.0, across)) * in.color.a;
        rgb = mix(params.foam_color.rgb * 0.75, params.foam_color.rgb, n);
        alpha = a * fade;
    } else if mode == 2 {
        let across = abs(in.uv.y - 0.5) * 2.0;
        let stones = step(0.62, vnoise(in.uv * vec2<f32>(40.0, 14.0)));
        rgb = mix(vec3<f32>(0.28, 0.33, 0.34), vec3<f32>(0.45, 0.43, 0.38), stones);
        alpha = (1.0 - smoothstep(0.7, 1.0, across)) * 0.8 * fade;
    } else {
        // Pond: dark water, sky reflection by incidence, a sand bar on one pond in three.
        let r = length(in.uv - vec2<f32>(0.5)) * 2.0;
        let edge = 1.0 - smoothstep(0.55, 1.0, r + (hash12(floor(in.uv * 9.0) + vec2<f32>(in.color.a * 7.0)) - 0.5) * 0.35);
        let fres = pow(1.0 - in.up_dot_view, 3.0);
        var col = mix(in.color.rgb, params.sky_color.rgb, clamp(params.b.w * 0.3 + fres * params.b.w, 0.0, 0.6));
        let bar = smoothstep(0.35, 0.0, length(in.uv - vec2<f32>(0.62, 0.42) * 0.8 - vec2<f32>(0.1)) * 2.0) * step(0.66, hash12(vec2<f32>(in.color.a * 91.0, 3.0)));
        col = mix(col, vec3<f32>(0.55, 0.52, 0.42), bar * 0.7);
        rgb = col;
        alpha = edge * params.b.z * 0.8;
    }
    return vec4<f32>(rgb, alpha);
}
