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
    // x cover (0 = no clouds), y softness, z scale, w fade-in height over the horizon.
    cloud_a: vec4<f32>,
    // xy: drift offset, z: opacity ceiling, w: band saturation (elevation share).
    cloud_b: vec4<f32>,
    cloud_lit: vec4<f32>,
    cloud_shade: vec4<f32>,
}

fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1030));
    let r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y);
}

// Four octaves: big banks, then the frayed edge.
fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0; i < 4; i = i + 1) {
        sum = sum + amp * vnoise(q);
        q = q * 2.03 + vec2<f32>(17.1, 9.2);
        amp = amp * 0.5;
    }
    return sum / 0.9375;
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
    // Painted horizon: the one haze colour only on the horizon line; just above it the sky blue
    // comes back (a thin pale line, not a milk band).
    let pull = smoothstep(0.0, max(params.cloud_b.w, 0.01), elevation);
    var col = mix(params.zenith.rgb, params.horizon.rgb, k * mix(1.0, 0.15, pull));
    let facing = max(dot(dir, normalize(params.sun.xyz)), 0.0);
    col = col + params.sun_color.rgb * params.sun.w * pow(facing, params.sun_color.a) * (1.0 - elevation * 0.5);
    // Clouds: banks on a plane above the camera, flattened by perspective towards the horizon.
    if (params.cloud_a.x > 0.0 && dir.y > 0.0) {
        let plane = dir.xz / (dir.y + 0.12) * params.cloud_a.z * 0.25 + params.cloud_b.xy;
        let density = fbm(plane);
        let threshold = 1.0 - params.cloud_a.x;
        let cover = smoothstep(threshold - params.cloud_a.y * 0.5, threshold + params.cloud_a.y * 0.5, density);
        // Lit side towards the sun: compare with the field shifted along the sun.
        let toward = normalize(params.sun.xz + vec2<f32>(1e-4));
        let ahead = fbm(plane + toward * 0.12);
        let lit = clamp(0.55 + (density - ahead) * 4.0, 0.0, 1.0);
        let body = mix(params.cloud_shade.rgb, params.cloud_lit.rgb, lit);
        let fade = smoothstep(0.0, max(params.cloud_a.w, 0.01), elevation);
        let alpha = cover * fade * params.cloud_b.z;
        // Towards the horizon the banks dissolve into the haze colour.
        let tinted = mix(body, params.horizon.rgb, 1.0 - fade);
        col = mix(col, tinted, alpha);
    }
    return vec4<f32>(col * params.zenith.a, 1.0);
}
