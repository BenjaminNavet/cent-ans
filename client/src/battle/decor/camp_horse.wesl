// Tethered horses of the camps (port of game/shaders/camp_horse.gdshader, lot AS1/AS8c): a copy of
// Bevy's mesh vertex stage that moves the vertices of the kit horse models (grazing head, chewing,
// breathing, weight shifting, tail swishing). Model frame of the kit: nose towards -X, tail towards
// +X, feet at y = 0, side along Z. The seed is derived from the instance position like Godot's
// `MODEL_MATRIX[3].xz`. `hair.x` = 1 on the mane / tail material.
import bevy_pbr::render::{
    mesh_bindings::mesh,
    mesh_functions,
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
};

struct CampHorseParams {
    head: vec4<f32>,   // pivot x, pivot y, neck start x, head end x
    head2: vec4<f32>,  // head lift (rad), cycle (s), share of the cycle head up, nod (rad)
    head3: vec4<f32>,  // nod frequency, chew frequency, chew amplitude, ramp (s)
    tail: vec4<f32>,   // start x, end x, top y, length
    tail2: vec4<f32>,  // swing (m), frequency, swish period (s), 0
    shift: vec4<f32>,  // period (s), hoof lift, hoof back, sway
    breath: vec4<f32>, // vertical, lateral, frequency, hair flag
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> horse: CampHorseParams;

const TAU: f32 = 6.2831853;

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn pose(v_in: vec3<f32>, origin: vec2<f32>) -> vec3<f32> {
    let hair = horse.breath.w;
    let seed = hash2(origin * 0.37);
    let t = globals.time + seed * 97.0;
    var v = v_in;
    let side = select(-1.0, 1.0, v.z >= 0.0);

    // Breathing: flanks and back.
    let barrel = (1.0 - smoothstep(0.5, 0.8, abs(v.x))) * smoothstep(0.6, 0.95, v.y);
    let breath = sin(t * TAU * horse.breath.z);
    v.z *= 1.0 + horse.breath.y * breath * barrel;
    v.y += horse.breath.x * breath * smoothstep(0.8, 1.5, v.y) * (1.0 - smoothstep(0.7, 1.1, abs(v.x)));

    // Weight passing from one hind hoof to the other.
    let sv = sin(t * TAU / horse.shift.x);
    let cock = select(smoothstep(0.55, 0.95, -sv), smoothstep(0.55, 0.95, sv), side > 0.0);
    let leg_w = clamp((0.8 - v.y) / 0.8, 0.0, 1.0) * (1.0 - hair) * smoothstep(0.35, 0.5, v.x);
    v.y += horse.shift.y * cock * leg_w;
    v.x -= horse.shift.z * cock * leg_w;
    let lean = smoothstep(0.55, 0.95, sv) - smoothstep(0.55, 0.95, -sv);
    v.z -= horse.shift.w * lean * smoothstep(0.3, 1.2, v.y);

    // Head: raised from time to time, chews when low, slight nod.
    var hw = clamp((horse.head.z - v.x) / (horse.head.z - horse.head.w), 0.0, 1.0);
    hw = hw * hw * (3.0 - 2.0 * hw);
    if hw > 0.0 {
        let u = fract(t / horse.head2.y);
        var ramp = 0.05;
        if horse.head3.w > 0.0 {
            ramp = clamp(horse.head3.w / horse.head2.y, 0.01, 0.2);
        }
        let up = smoothstep(0.0, ramp, u) * (1.0 - smoothstep(horse.head2.z - ramp, horse.head2.z, u));
        var angle = -horse.head2.x * up;
        angle += sin(t * TAU * horse.head3.y) * horse.head3.z * (1.0 - up);
        angle += sin(t * TAU * horse.head3.x) * horse.head2.w;
        angle *= hw;
        let d = v.xy - horse.head.xy;
        let c = cos(angle);
        let s = sin(angle);
        let r = horse.head.xy + vec2<f32>(d.x * c - d.y * s, d.y * c + d.x * s);
        v = vec3<f32>(r.x, r.y, v.z);
    }

    // Tail: swishes the flies away in bursts.
    let tw = hair * smoothstep(horse.tail.x, horse.tail.y, v.x) * clamp((horse.tail.z - v.y) / horse.tail.w, 0.0, 1.0);
    if tw > 0.0 {
        let swish = 0.3 + 0.7 * pow(max(0.0, sin(t * TAU / horse.tail2.z)), 3.0);
        v.z += tw * horse.tail2.x * swish * sin(t * TAU * horse.tail2.y);
    }
    return v;
}

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    @if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);

    let local = pose(vertex.position, world_from_local[3].xz);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(local, 1.0));

    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);

    @if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    @if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;
    @if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex_in.instance_index);
    @if(VERTEX_COLORS)
    out.color = vertex.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);
    return out;
}
