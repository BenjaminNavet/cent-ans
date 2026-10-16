// Rain and snow of a battle (port of `_add_precipitation` of battle_atmosphere.gd, GPUParticles3D
// round the camera). One static mesh of quads, one quad per drop; the vertex shader moves each drop
// from its seed and the time (no CPU update): drops live in world space and wrap in a box that
// follows the camera, so they neither swim with the camera nor pop (faded at the box faces).
// Rain: thin streaks aligned on the fall; snow: round camera-facing flakes that drift. Faded near
// the lens like Godot's DISTANCE_FADE_PIXEL_ALPHA.
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct FallParams {
    // xyz: wrap box (m), w: unused.
    area: vec4<f32>,
    // xyz: box centre (world), w: mode (0 rain, 1 snow).
    centre: vec4<f32>,
    // xyz: fall velocity (m/s), w: speed jitter.
    velocity: vec4<f32>,
    // x: width, y: length, z: sway (m), w: sway (Hz).
    shape: vec4<f32>,
    // linear rgb, a: opacity.
    color: vec4<f32>,
    // x: fully faded below (m), y: fully visible beyond (m).
    fade: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> fall: FallParams;

struct VertexInput {
    // xy: quad corner in -0.5..0.5.
    @location(0) position: vec3<f32>,
    // Per-drop seed in 0..1.
    @location(1) seed: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
}

const TAU: f32 = 6.283185307;

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let box = fall.area.xyz;
    let t = globals.time;
    let speed = 1.0 + (in.seed.y - 0.5) * 2.0 * fall.velocity.w;
    var travel = fall.velocity.xyz * speed * t;
    travel = travel - box * floor(travel / box);
    var rel = in.seed * box + travel - fall.centre.xyz;
    rel = rel - box * floor(rel / box + 0.5);
    if fall.shape.z > 0.0 {
        let w = t * TAU * fall.shape.w;
        rel.x += sin(w + in.seed.x * TAU * 7.0) * fall.shape.z;
        rel.z += cos(w * 0.83 + in.seed.z * TAU * 5.0) * fall.shape.z;
    }
    // Faded at the box faces, where a drop wraps to the other side.
    let edge = abs(rel) / (box * 0.5);
    let edge_fade = 1.0 - smoothstep(0.82, 1.0, max(edge.x, max(edge.y, edge.z)));
    let centre = fall.centre.xyz + rel;
    let cam = view.world_position;
    let to_cam = normalize(cam - centre);
    var world: vec3<f32>;
    if fall.centre.w < 0.5 {
        let axis = normalize(-fall.velocity.xyz);
        let right = normalize(cross(axis, to_cam) + vec3<f32>(1e-5, 0.0, 0.0));
        world = centre + right * in.position.x * fall.shape.x + axis * in.position.y * fall.shape.y;
    } else {
        let right = view.world_from_view[0].xyz;
        let up = view.world_from_view[1].xyz;
        world = centre + right * in.position.x * fall.shape.x + up * in.position.y * fall.shape.y;
    }
    let near = smoothstep(fall.fade.x, fall.fade.y, distance(cam, centre));
    var out: VertexOutput;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    out.uv = in.uv;
    out.alpha = fall.color.a * near * edge_fade;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var shape = 1.0;
    if fall.centre.w < 0.5 {
        // Streak: soft across, tapered at both ends.
        shape = (1.0 - abs(in.uv.x * 2.0 - 1.0)) * (1.0 - smoothstep(0.6, 1.0, abs(in.uv.y * 2.0 - 1.0)));
    } else {
        shape = 1.0 - smoothstep(0.3, 0.5, length(in.uv - vec2<f32>(0.5)));
    }
    let a = in.alpha * shape;
    if a < 0.002 {
        discard;
    }
    return vec4<f32>(fall.color.rgb, a);
}
