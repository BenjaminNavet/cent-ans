// Distant smoke of a horizon village (port of game/shaders/battle_horizon_smoke.gdshader): a
// panel turned to the camera round the vertical, a column that widens and leans with the height,
// noise that rises. Unlit; the distance fog of the scene is applied here (exponential, like the
// battle camera's `DistanceFog`), Godot's fog greyed it the same way.
import bevy_pbr::render::mesh_functions::{get_world_from_local, get_tag};
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct SmokeParams {
    // linear rgb, a: opacity.
    color: vec4<f32>,
    // linear rgb: light tint, a: unused.
    light: vec4<f32>,
    // linear rgb: fog colour, a: fog density (1/m).
    fog: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> smoke: SmokeParams;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) seed: f32,
    @location(2) fog: f32,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let model = get_world_from_local(in.instance_index);
    let seed = f32(get_tag(in.instance_index)) / 1000.0;
    let origin = model[3].xyz;
    let size = vec2<f32>(length(model[0].xyz), length(model[1].xyz));
    let cam_right = view.world_from_view[0].xyz;
    let right = normalize(vec3<f32>(cam_right.x, 0.0, cam_right.z) + vec3<f32>(1e-4, 0.0, 0.0));
    // Quad corners in -0.5..0.5; the column stands on its base.
    let world = origin + right * in.position.x * size.x + vec3<f32>(0.0, (in.position.y + 0.5) * size.y, 0.0);
    var out: VertexOutput;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    out.uv = in.uv;
    out.seed = seed;
    out.fog = 1.0 - exp(-distance(view.world_position, world) * smoke.fog.a);
    return out;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), f.x), mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), f.x), f.y);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let h = 1.0 - uv.y;
    let drift = h * h * 0.35;
    let x = (uv.x - 0.5 - drift) / mix(0.12, 0.5, h);
    let t = globals.time * 0.05 + in.seed;
    let n = noise(vec2<f32>(uv.x * 5.0 + in.seed, uv.y * 7.0 + t * 3.0)) * 0.6
        + noise(vec2<f32>(uv.x * 13.0, uv.y * 17.0 + t * 6.0)) * 0.4;
    let body = (1.0 - smoothstep(0.3, 1.0, abs(x))) * smoothstep(0.0, 0.06, h) * (1.0 - smoothstep(0.45, 1.0, h));
    let albedo = smoke.color.rgb * smoke.light.rgb * mix(0.85, 1.05, n);
    let alpha = clamp(body * (n * 0.9 + 0.1) * smoke.color.a, 0.0, 1.0);
    if alpha < 0.003 {
        discard;
    }
    return vec4<f32>(mix(albedo, smoke.fog.rgb, in.fog), alpha);
}
