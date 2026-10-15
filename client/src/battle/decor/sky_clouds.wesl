// Sky dome with drifting clouds: the vertex colours (gradient + sun glow) are the clear sky, the
// fragment adds fractal-noise cumulus projected on a plane above the camera, lit on the sun side.
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    mesh_view_bindings::{view, globals},
};

struct CloudParams {
    // x: cover (0 clear .. 1 overcast), y: drift speed, z: darkness of the underside, w: unused.
    shape: vec4<f32>,
    // xyz: direction towards the sun.
    sun: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> clouds: CloudParams;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 5; i = i + 1) {
        v = v + a * noise(q);
        q = q * 2.03 + vec2<f32>(17.0, 9.0);
        a = a * 0.5;
    }
    return v;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let pbr_input = pbr_input_from_standard_material(in, is_front);
    var colour = pbr_input.material.base_color.rgb;
    let dir = normalize(in.world_position.xyz - view.world_position);
    if dir.y > 0.01 && clouds.shape.x > 0.001 {
        // Plane projection: clouds shrink towards the horizon.
        let plane = dir.xz / (dir.y + 0.12);
        let drift = vec2<f32>(1.0, 0.35) * globals.time * clouds.shape.y;
        let p = plane * 0.9 + drift;
        let density = fbm(p) * 0.75 + fbm(p * 2.7 + 3.0) * 0.25;
        let threshold = 0.62 - clouds.shape.x * 0.32;
        let body = smoothstep(threshold, threshold + 0.22, density);
        // Thinner above the horizon band (the haze already owns it), full overhead.
        let cover = body * smoothstep(0.01, 0.2, dir.y);
        // Sun-side lit edges: density gradient towards the sun along the plane.
        let sun_plane = normalize(clouds.sun.xz + vec2<f32>(0.0001, 0.0));
        let ahead = fbm(p + sun_plane * 0.12) * 0.75 + fbm((p + sun_plane * 0.12) * 2.7 + 3.0) * 0.25;
        let lit = clamp((density - ahead) * 6.0 + 0.55, 0.0, 1.0);
        let shade = mix(1.0 - clouds.shape.z, 1.0, lit);
        let cloud = vec3<f32>(1.0, 0.985, 0.95) * shade;
        colour = mix(colour, cloud, cover * 0.92);
    }
    var out: FragmentOutput;
    out.color = vec4<f32>(colour, 1.0);
    return out;
}
