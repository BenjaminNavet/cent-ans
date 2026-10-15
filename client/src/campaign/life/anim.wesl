// Vertex animation of the beasts, carts and herds of the living map. Port of
// game/shaders/animal_motion.gdshaderinc (draught beasts, carts, ploughs: four-beat legs on the
// measured step curves, nodding or grazing head, tail, breath, wheels turning with the distance
// travelled, measured jolt and roll of the box) and of the gait part of game/shaders/fauna.gdshader
// (herds: diagonal legs, lift, bounce, head down when stopped).
//
// The vertex stage is a copy of Bevy's mesh vertex stage (as wind.wesl). Per-instance data come
// from the mesh tag (`MeshTag`):
//   mode 0 (prop):  low 16 bits = speed in mm/s, high 16 bits = seed / 65536;
//   mode 1 (herd):  low 16 bits = leg phase (cycles) / 65536, bits 16..24 = moving / 255.
// Time is `globals.time`. Everything is computed in the Godot frame of the model (front along +Z):
// the mesh is turned by `frame.x` (the `yaw_deg` of the prop table) and turned back afterwards.
import bevy_pbr::render::{
    mesh_bindings::mesh,
    mesh_functions,
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
};

struct AnimParams {
    body: vec4<f32>,       // beast centre on the advance axis, half length, belly height, withers height
    neck: vec4<f32>,       // neck start, neck length, pivot height, tail length
    gait: vec4<f32>,       // stride, swing, lift, bob
    head: vec4<f32>,       // grazing head angle, nod, cycle (s), grazing share
    tail: vec4<f32>,       // tail swing, frequency, walk gain, sway
    breath: vec4<f32>,     // breath, breath frequency, chew frequency, chew amplitude
    cart: vec4<f32>,       // wheel radius, minimal lateral offset, jolt gain, reference stride
    axle: vec4<f32>,       // z of the first axle, z of the second, second present, 0
    extra: vec4<f32>,      // head ramp (s), box roll (rad), 0, 0
    jolt_amp: vec4<f32>,
    jolt_ratio: vec4<f32>,
    frame: vec4<f32>,      // yaw (rad), mode (0 prop, 1 herd), on (beast present), 0
    herd_gait: vec4<f32>,  // cadence, swing, lift, bounce
    herd_origin: vec4<f32>, // (centre x, foot y, centre z), graze
    herd_size: vec4<f32>,  // length, height, long axis is X, 0
    swing: array<vec4<f32>, 4>,
    lift: array<vec4<f32>, 4>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> anim: AnimParams;

const TAU: f32 = 6.2831853;
const PI: f32 = 3.1415927;

fn rotate_y(v: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
}

// Pitch (a > 0: the front goes down) around the X axis through `pivot` in the (y, z) plane.
fn pitch(yz: vec2<f32>, pivot: vec2<f32>, a: f32) -> vec2<f32> {
    let d = yz - pivot;
    let c = cos(a);
    let s = sin(a);
    return pivot + vec2<f32>(d.x * c - d.y * s, d.y * c + d.x * s);
}

fn lut_value(table: array<vec4<f32>, 4>, i: i32) -> f32 {
    let k = u32(i);
    return table[k >> 2u][k & 3u];
}

fn swing_at(u: f32) -> f32 {
    let x = fract(u) * 16.0;
    let i = i32(floor(x));
    return mix(lut_value(anim.swing, i), lut_value(anim.swing, (i + 1) % 16), x - f32(i));
}

fn lift_at(u: f32) -> f32 {
    let x = fract(u) * 16.0;
    let i = i32(floor(x));
    return mix(lut_value(anim.lift, i), lut_value(anim.lift, (i + 1) % 16), x - f32(i));
}

fn prop_pose(v_in: vec3<f32>, t: f32, speed: f32, seed: f32) -> vec3<f32> {
    var v = v_in;
    let moving = smoothstep(0.05, 0.3, speed);
    let turn = TAU * (t * speed / max(anim.gait.x, 0.05) + seed);

    // Wheels roll at the advance speed.
    var is_wheel = false;
    if anim.cart.x > 0.0 && abs(v.x) > anim.cart.y {
        var axle_z = anim.axle.x;
        let on_first = length(vec2<f32>(v.y - anim.cart.x, v.z - anim.axle.x)) < anim.cart.x + 0.03;
        let on_second = anim.axle.z > 0.5
            && length(vec2<f32>(v.y - anim.cart.x, v.z - anim.axle.y)) < anim.cart.x + 0.03;
        if on_second && !on_first {
            axle_z = anim.axle.y;
        }
        if on_first || on_second {
            is_wheel = true;
            let a = t * speed / anim.cart.x;
            let yz = pitch(v.yz, vec2<f32>(anim.cart.x, axle_z), a);
            v = vec3<f32>(v.x, yz.x, yz.y);
        }
    }
    // Jolt and roll of the box (not the wheels): three measured lines whose period is a multiple of
    // the stride of the draught beast.
    if speed > 0.0 && !is_wheel {
        let dist = t * speed;
        var jolt = 0.0;
        for (var k = 0; k < 3; k++) {
            jolt += anim.jolt_amp[k] * sin(TAU * (dist / (anim.jolt_ratio[k] * anim.cart.w) + seed * (1.0 + 0.37 * f32(k))));
        }
        v.y += anim.cart.z * moving * jolt;
        if anim.cart.x > 0.0 && abs(v.z - anim.body.x) > anim.body.y + 0.2 {
            let roll = anim.extra.y * moving * sin(TAU * (dist / (anim.jolt_ratio.x * anim.cart.w) + seed * 1.7));
            let xy = pitch(v.xy, vec2<f32>(0.0, anim.cart.x), roll);
            v = vec3<f32>(xy.x, xy.y, v.z);
        }
    }

    if anim.frame.z < 0.5 {
        return v;
    }
    let zr = v.z - anim.body.x;
    let half_len = anim.body.y;
    if zr < -half_len - anim.neck.w - 0.05 || zr > half_len + 1.4 {
        return v;
    }
    let belly = anim.body.z;
    let withers = anim.body.w;
    let side = select(-1.0, 1.0, v.x >= 0.0);
    let leg_w = clamp((belly - v.y) / belly, 0.0, 1.0);
    let body_w = 1.0 - leg_w;
    let cyc_t = t + seed * 40.0;

    // Legs: four beats, swing growing towards the hoof.
    if abs(zr) < half_len * 0.95 {
        let front = select(0.0, 0.25, zr >= 0.0);
        let leg_u = turn / TAU + select(0.5, 0.0, side > 0.0) + front;
        v.z += swing_at(leg_u) * anim.gait.y * leg_w * moving;
        v.y += lift_at(leg_u) * anim.gait.z * leg_w * leg_w * moving;
    }
    // Body bob and sway (standing, the weight shifts from side to side).
    v.y += anim.gait.w * sin(2.0 * turn + 1.0) * body_w * moving;
    v.x += anim.tail.w * sin(cyc_t * 0.7) * smoothstep(0.0, withers, v.y) * (1.0 - moving);
    // Breath: flanks and back.
    v.y += anim.breath.x * sin(cyc_t * TAU * anim.breath.y) * smoothstep(belly, withers, v.y);

    // Head: grazes (head down, chews) then lifts; nods when walking.
    let head_w = smoothstep(anim.neck.x, anim.neck.x + anim.neck.y, zr) * smoothstep(belly * 0.75, belly * 1.1, v.y);
    if head_w > 0.0 {
        let u = fract(cyc_t / anim.head.z);
        let ramp = clamp(anim.extra.x / max(anim.head.z, 1.0), 0.01, 0.3);
        var graze = smoothstep(0.0, ramp, u) * (1.0 - smoothstep(anim.head.w - ramp, anim.head.w, u));
        graze *= 1.0 - moving;
        let chew = sin(cyc_t * TAU * anim.breath.z) * anim.breath.w * graze;
        let nod = sin(turn) * anim.head.y * moving + sin(cyc_t * 0.9) * anim.head.y * 0.4 * (1.0 - moving);
        let angle = (graze * anim.head.x + chew + nod) * head_w;
        let yz = pitch(v.yz, vec2<f32>(anim.neck.z, anim.body.x + anim.neck.x), angle);
        v = vec3<f32>(v.x, yz.x, yz.y);
    }
    // Tail: swats the flies.
    let tail_w = clamp((-zr - half_len) / anim.neck.w, 0.0, 1.0)
        * step(abs(v.x), 0.1) * step(belly * 0.6, v.y);
    if tail_w > 0.0 {
        let swish = 0.5 + 0.5 * pow(max(0.0, sin(cyc_t * 0.37)), 3.0);
        let rate = anim.tail.y * (1.0 + (anim.tail.z - 1.0) * moving);
        v.x += tail_w * anim.tail.x * swish * sin(cyc_t * TAU * rate);
    }
    return v;
}

fn herd_pose(v_in: vec3<f32>, phase_cycles: f32, moving: f32) -> vec3<f32> {
    let long_x = anim.herd_size.z > 0.5;
    var p = v_in - anim.herd_origin.xyz;
    if long_x {
        p = vec3<f32>(-p.z, p.y, p.x);
    }
    let len = anim.herd_size.x;
    let hgt = anim.herd_size.y;
    let phase = phase_cycles * TAU;
    // Diagonal legs (front left with rear right), lift, body bounce, head down when stopped.
    let low = 1.0 - smoothstep(0.22 * hgt, 0.6 * hgt, p.y);
    let front = step(0.0, p.z);
    let left = step(0.0, p.x);
    let leg_phase = phase + PI * (front + left);
    p.z += sin(leg_phase) * anim.herd_gait.y * len * low * moving;
    p.y += max(0.0, cos(leg_phase)) * anim.herd_gait.z * len * low * moving;
    p.y += abs(sin(phase)) * anim.herd_gait.w * len * moving;
    let head = smoothstep(0.2 * len, 0.5 * len, p.z) * smoothstep(0.5 * hgt, 0.95 * hgt, p.y);
    p.y -= head * anim.herd_origin.w * hgt * (1.0 - moving);
    p.z += head * anim.herd_origin.w * hgt * 0.3 * (1.0 - moving);
    if long_x {
        p = vec3<f32>(p.z, p.y, -p.x);
    }
    return p + anim.herd_origin.xyz;
}

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    @if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);

    let tag = mesh_functions::get_tag(vertex_in.instance_index);
    var local = vertex.position;
    if anim.frame.y < 0.5 {
        let speed = f32(tag & 0xffffu) * 0.001;
        let seed = f32(tag >> 16u) / 65536.0;
        local = rotate_y(prop_pose(rotate_y(local, anim.frame.x), globals.time, speed, seed), -anim.frame.x);
    } else {
        let phase = f32(tag & 0xffffu) / 65536.0;
        let moving = f32((tag >> 16u) & 0xffu) / 255.0;
        local = herd_pose(local, phase, moving);
    }
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
