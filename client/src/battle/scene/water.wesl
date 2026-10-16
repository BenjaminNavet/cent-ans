// Battle water: river and brooks (kind 0), sea (kind 1), pier foam (kind 2). Port of
// game/shaders/battle_water.gdshader, battle_sea.gdshader and battle_foam.gdshader.
// The water is lit by Bevy's PBR; the depth of the river comes from the vertex colour (red).
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{view, globals},
};

struct WaterParams {
    // rgb: shallows colour, a: turbidity.
    shallow: vec4<f32>,
    // rgb: deep colour, a: clarity (m).
    deep: vec4<f32>,
    // rgb: sky colour reflected, a: rain ripples.
    sky: vec4<f32>,
    // x: kind, y: flow speed, z: shore x, w: flank.
    misc: vec4<f32>,
    // x: ribbon width.
    misc2: vec4<f32>,
    // rgb: shore water of the sea.
    shore: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> water: WaterParams;

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
    for (var k = 0; k < 4; k = k + 1) {
        v = v + a * noise(q);
        q = q * 2.03;
        a = a * 0.5;
    }
    return v;
}

// Height of two noise octaves; its gradient is the wave slope.
fn wave_h(p: vec2<f32>) -> f32 {
    return noise(p) * 0.65 + noise(p * 2.3 + vec2<f32>(9.0, 4.0)) * 0.35;
}

// Slope (dh/dx, dh/dz) of the waves at p.
fn wave_slope(p: vec2<f32>) -> vec2<f32> {
    let e = 0.12;
    let dx = wave_h(p + vec2<f32>(e, 0.0)) - wave_h(p - vec2<f32>(e, 0.0));
    let dz = wave_h(p + vec2<f32>(0.0, e)) - wave_h(p - vec2<f32>(0.0, e));
    return vec2<f32>(dx, dz) / (2.0 * e);
}

fn perturb(base: vec3<f32>, slope: vec2<f32>, strength: f32) -> vec3<f32> {
    return normalize(base + vec3<f32>(-slope.x, 0.0, -slope.y) * strength);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var out: FragmentOutput;
    let kind = water.misc.x;

    // Pier foam: wake in a V that opens and fades downstream, boils near the pier.
    if kind > 1.5 {
        let down = in.uv.y;
        let across = abs(in.uv.x - 0.5) * 2.0;
        let spread = mix(0.35, 1.0, down);
        let wake = 1.0 - smoothstep(spread * 0.55, spread, across);
        let fade = smoothstep(1.0, 0.7, down) * smoothstep(0.0, 0.08, down);
        let t = globals.time * water.misc.y;
        let n = fbm(vec2<f32>(in.uv.x * 6.0, down * 5.0 - t * 1.6));
        let streaks = fbm(vec2<f32>(in.uv.x * 14.0, down * 2.0 - t * 2.2));
        let boil = smoothstep(0.35, 0.0, down) * fbm(vec2<f32>(in.uv.x * 9.0 + t * 0.3, down * 9.0 - t));
        let foam = wake * fade * smoothstep(0.42, 0.7, n * 0.65 + streaks * 0.45) + boil * 0.8;
        out.color = vec4<f32>(0.86, 0.88, 0.86, clamp(foam, 0.0, 0.85));
        return out;
    }

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let view_dir = normalize(view.world_position - in.world_position.xyz);
    let dist = length(view.world_position - in.world_position.xyz);
    let wp = in.world_position.xz;
    let t_raw = globals.time;
    let ripple = water.sky.w;
    var colour = vec3<f32>(0.1);
    var alpha = 1.0;
    var roughness = 0.1;
    var normal = up;

    if kind < 0.5 {
        // River: waves flow along the ribbon (uv.y = metres along the course).
        let flow = vec2<f32>(in.uv.x * water.misc2.x, in.uv.y);
        let t = t_raw * water.misc.y;
        var slope = wave_slope(flow * vec2<f32>(0.8, 0.45) + vec2<f32>(0.0, -t * 1.1));
        slope = slope + wave_slope(flow * vec2<f32>(0.45, 0.2) + vec2<f32>(3.1, -t * 0.7)) * 0.7;
        normal = perturb(up, slope, 0.22);
        if ripple > 0.0 {
            let drops = wave_slope(wp * 0.9 + vec2<f32>(t_raw * 1.3, -t_raw * 1.0));
            normal = perturb(normal, drops, 0.12 * ripple);
        }
        let depth = in.color.r;
        let murk0 = 1.0 - exp(-depth / water.deep.a);
        var murk = clamp(murk0 + water.shallow.a * 0.35, 0.0, 1.0);
        // Shoals: the edge of the ribbon keeps a clear green water.
        let edge = 1.0 - smoothstep(0.0, 0.3, min(in.uv.x, 1.0 - in.uv.x));
        murk = murk * mix(1.0, 0.45, edge);
        let bed = vec3<f32>(0.22, 0.21, 0.17) * water.shallow.rgb * 2.2;
        var water_colour = mix(bed, water.deep.rgb, murk);
        // Sky reflection (Schlick), capped: a bit of bright sky turns dark water milky.
        let view_n = normalize(mix(up, normal, 0.25));
        let cos_v = clamp(dot(view_n, view_dir), 0.0, 1.0);
        let fresnel = 0.03 + 0.97 * pow(1.0 - cos_v, 5.0);
        let sky = water.sky.rgb * 0.7 * mix(0.85, 1.1, noise(wp * 0.004));
        water_colour = mix(water_colour, sky, clamp(fresnel * 1.1, 0.0, 0.4) * smoothstep(0.02, 0.2, depth));
        // Foam on the shoals and along the banks, torn and carried by the current.
        let foam_noise = noise(flow * vec2<f32>(0.3, 0.12) * 4.0 + vec2<f32>(0.0, -t * 0.35 * 4.0));
        let shore = 1.0 - smoothstep(0.02, 0.12, depth);
        let foam = smoothstep(0.55, 0.8, foam_noise) * shore * 0.6;
        water_colour = mix(water_colour, vec3<f32>(0.7, 0.72, 0.68), foam * 0.4);
        colour = water_colour;
        roughness = mix(0.1, 0.35, foam);
        // Bank melted into the ground.
        alpha = smoothstep(0.0, 0.1, depth);
    } else {
        // Sea: two-scale swell plus a long one, world coordinates, calmer in the distance.
        let t = t_raw;
        var slope = wave_slope(wp * 0.27 + vec2<f32>(t * 0.1, t * 0.03));
        slope = slope + wave_slope(wp * 0.1 + vec2<f32>(-t * 0.04, t * 0.07)) * 1.3;
        slope = slope + wave_slope(wp * 0.025 + vec2<f32>(t * 0.02, -t * 0.015)) * 2.0;
        let calm = smoothstep(600.0, 3000.0, dist);
        normal = perturb(up, slope * (1.0 - calm * 0.8), 0.16);
        if ripple > 0.0 {
            let drops = wave_slope(wp * 0.9 + vec2<f32>(t * 1.3, -t));
            normal = perturb(normal, drops, 0.1 * ripple);
        }
        let offshore = max((in.world_position.x - water.misc.z) * water.misc.w, 0.0);
        let near_shore = 1.0 - smoothstep(0.0, 90.0, offshore);
        let macro_n = noise(wp * 0.0015 * 4.0);
        let base = mix(water.deep.rgb, water.shore.rgb, near_shore * 0.8) * mix(0.85, 1.15, macro_n);
        let cos_v = clamp(dot(normal, view_dir), 0.0, 1.0);
        let fresnel = pow(1.0 - cos_v, 4.0);
        var albedo = mix(base, water.sky.rgb * 0.8, clamp(fresnel * 0.75 + calm * 0.25, 0.0, 0.85));
        // Foam: a broken fringe against the beach.
        let foam_band = 1.0 - smoothstep(0.0, 6.0 + macro_n * 6.0, offshore);
        let foam_noise = noise(wp * 0.08 * 4.0 + vec2<f32>(t * 0.08, 0.0));
        let foam = foam_band * smoothstep(0.4, 0.7, foam_noise + foam_band * 0.3);
        albedo = mix(albedo, vec3<f32>(0.86, 0.88, 0.86), foam * 0.8);
        colour = albedo;
        roughness = mix(0.08, 0.6, foam);
    }

    pbr_input.material.base_color = vec4<f32>(colour, alpha);
    pbr_input.material.perceptual_roughness = roughness;
    pbr_input.N = normal;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
