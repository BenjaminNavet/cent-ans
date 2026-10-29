// VIS-R1 campaign sky dome. The mesh is a unit sphere; the vertex stage ignores the model matrix
// and places every vertex around the camera at the far plane (reverse-Z depth 0), so the dome
// only fills what nothing else drew. Gradient zenith -> horizon from the live fog colour; below
// the horizon the horizon colour continues (the far ground is fogged to the same tone).
import bevy_pbr::render::mesh_view_bindings::view;

struct DomeParams {
    // rgb: linear horizon colour, a: gradient exponent.
    horizon: vec4<f32>,
    // rgb: linear zenith colour, a: gain.
    zenith: vec4<f32>,
    // xyz: direction towards the sun, w: glow strength.
    sun: vec4<f32>,
    // rgb: linear sun colour, a: glow exponent.
    sun_color: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: DomeParams;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) dir: vec3<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let dir = normalize(in.position);
    var clip = view.clip_from_world * vec4<f32>(view.world_position + dir * 10.0, 1.0);
    // Reverse-Z: depth 0 is the far plane.
    clip.z = 0.0;
    var out: VertexOutput;
    out.clip = clip;
    out.dir = dir;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.dir);
    let elevation = asin(clamp(dir.y, 0.0, 1.0)) / 1.5707963;
    let k = pow(1.0 - elevation, 1.0 / max(params.horizon.a, 0.05));
    var col = mix(params.zenith.rgb, params.horizon.rgb, k);
    let facing = max(dot(dir, normalize(params.sun.xyz)), 0.0);
    col = col + params.sun_color.rgb * params.sun.w * pow(facing, params.sun_color.a) * (1.0 - elevation * 0.5);
    return vec4<f32>(col * params.zenith.a, 1.0);
}
