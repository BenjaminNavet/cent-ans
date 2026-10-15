// Campaign vegetation drawn with one instanced draw per batch (port of
// `campaign_tree_impostor.gdshader(inc)`, `foliage_common.gdshaderinc` seasonal part,
// `campaign_wind.gdshaderinc` and `freshwater_reeds.gdshader`). Lit with Bevy's PBR code.
//
// Instance record (40 bytes): (x, y, z, yaw_deg) (scale, 0, seed) (r, g, b).
// Modes (`params.frame.w`): 0 tree impostor (camera-facing card, baked view chosen by azimuth and
// dithered between the two nearest, seasonal tint, wind), 1 reed clump (two crossed cards),
// 2 pond (flat disc).
import bevy_pbr::render::{
    pbr_functions,
    pbr_types,
    mesh_view_bindings::{view, globals},
    view_transformations::position_world_to_clip,
};

struct Params {
    season: vec4<f32>,  // spring, summer, autumn, winter weights
    tint: vec4<f32>,    // rgb multiplier, w: season class (0 oak, 1 beech, 2 conifer, 3 hedge, 4 gold)
    atlas: vec4<f32>,   // views, rows, row, width / height ratio
    frame: vec4<f32>,   // ortho, foot, alpha cutoff, mode
    wind: vec4<f32>,    // bend, calm, light, gain
    extra: vec4<f32>,   // reeds: width, height (world units), sway, far scale
    range: vec4<f32>,   // reeds, ponds: scale near distance, scale far distance, pond far scale, opacity (dither)
}

@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;
@group(2) @binding(2) var normal_texture: texture_2d<f32>;
@group(2) @binding(3) var<uniform> params: Params;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(8) at_yaw: vec4<f32>,
    @location(9) scale_seed: vec3<f32>,
    @location(10) tint: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) uv0: vec2<f32>,
    @location(2) uv1: vec2<f32>,
    @location(3) right: vec3<f32>,
    @location(4) up: vec3<f32>,
    @location(5) face: vec3<f32>,
    @location(6) tint: vec3<f32>,
    @location(7) local: vec2<f32>,
    @location(8) misc: vec4<f32>, // blend, seed, ground y, mode-specific
}

const TAU: f32 = 6.2831853;
const WIND_DIR: vec2<f32> = vec2<f32>(0.98, 0.2);

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash12(i), hash12(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash12(i + vec2<f32>(0.0, 1.0)), hash12(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

// Gust strength in [0, 1] at the map point `p` (`campaign_wind.gdshaderinc`).
fn wind_gust(p: vec2<f32>) -> f32 {
    let d = normalize(WIND_DIR);
    var q = vec2<f32>(dot(p, d), dot(p, vec2<f32>(-d.y, d.x)));
    q.x = q.x - globals.time * 2.6;
    let size = vec2<f32>(34.0, 16.0);
    let n = value_noise(q / size) * 0.7
        + value_noise(q / (size * 0.45) + vec2<f32>(globals.time * 0.05, 3.7)) * 0.3;
    return smoothstep(0.62 - 0.11, 0.62 + 0.11, n);
}

fn bayer4(frag: vec2<f32>) -> f32 {
    let x = u32(frag.x) % 4u;
    let y = u32(frag.y) % 4u;
    var m = array<f32, 16>(
        0.0, 8.0, 2.0, 10.0,
        12.0, 4.0, 14.0, 6.0,
        3.0, 11.0, 1.0, 9.0,
        15.0, 7.0, 13.0, 5.0,
    );
    return (m[y * 4u + x] + 0.5) / 16.0;
}

fn rotate_y(v: vec3<f32>, c: f32, s: f32) -> vec3<f32> {
    return vec3<f32>(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
}

@vertex
fn vertex(in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let origin = in.at_yaw.xyz;
    let yaw = radians(in.at_yaw.w);
    let c = cos(yaw);
    let s = sin(yaw);
    let seed = in.scale_seed.z;
    let mode = params.frame.w;
    out.misc = vec4<f32>(0.0, seed, origin.y, 0.0);
    out.tint = in.tint;
    out.local = in.uv;
    out.right = vec3<f32>(1.0, 0.0, 0.0);
    out.up = vec3<f32>(0.0, 1.0, 0.0);
    out.face = vec3<f32>(0.0, 0.0, 1.0);
    out.uv0 = in.uv;
    out.uv1 = in.uv;
    var world: vec3<f32>;
    if mode < 0.5 {
        // Tree impostor.
        let views = params.atlas.x;
        let rows = params.atlas.y;
        let bx = vec3<f32>(c, 0.0, -s);
        let bz = vec3<f32>(s, 0.0, c);
        let to_cam = view.world_position - origin;
        // Azimuth of the camera in the frame of the tree (Blender (x, y) = (x, -z)).
        let ax = dot(to_cam, bx);
        let az = dot(to_cam, bz);
        let f = fract(atan2(-az, ax) / TAU + 1.0) * views;
        let k0 = floor(f) % views;
        let k1 = (k0 + 1.0) % views;
        out.misc.x = fract(f);
        let face = normalize(to_cam + vec3<f32>(0.0, 1e-4, 0.0));
        let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), face) + vec3<f32>(1e-5, 0.0, 0.0));
        let up = cross(face, right);
        // Stand variety: tone by a low-frequency noise on the position, scale jitter per tree.
        let jitter = 1.0 + 0.25 * (2.0 * fract(seed * 53.17) - 1.0);
        let n = value_noise(origin.xz / 22.0) + 0.18 * (fract(seed * 13.1) - 0.5);
        var tone = vec3<f32>(1.0);
        if n < 0.5 {
            tone = mix(vec3<f32>(0.84, 0.9, 0.86), vec3<f32>(1.0), smoothstep(0.3, 0.45, n));
        } else {
            tone = mix(vec3<f32>(1.0), vec3<f32>(1.07, 1.04, 0.9), smoothstep(0.55, 0.7, n));
        }
        let height = in.scale_seed.x * jitter;
        let width = height * params.atlas.w;
        let corner = in.uv;
        let local = vec2<f32>(corner.x - 0.5, 1.0 - corner.y);
        let ortho = params.frame.x;
        let foot = params.frame.y;
        world = origin + right * local.x * width * ortho + up * (local.y - foot) * height * ortho;
        // Wind: the crown bends along the gust (foot fixed), each tree shivers on its own.
        let gust = wind_gust(origin.xz);
        let flex = local.y * local.y * height * ortho;
        let shiver = sin(globals.time * 1.7 + seed * 23.0) * (0.4 + 0.6 * gust);
        world = world + vec3<f32>(WIND_DIR.x, 0.0, WIND_DIR.y) * flex
            * (params.wind.x * gust + params.wind.y * shiver);
        out.uv0 = (vec2<f32>(k0, params.atlas.z) + corner) / vec2<f32>(views, rows);
        out.uv1 = (vec2<f32>(k1, params.atlas.z) + corner) / vec2<f32>(views, rows);
        out.right = right;
        out.up = up;
        out.face = face;
        out.tint = params.tint.rgb * tone * (1.0 + params.wind.z * gust);
        out.local = local;
    } else if mode < 1.5 {
        // Reed clump: two crossed cards, magnified with the distance of the camera.
        let d = distance(view.world_position, origin);
        let near = params.range.x;
        let far = max(params.range.y, near * 1.01);
        let t = clamp(log(max(d, 1e-3) / near) / log(far / near), 0.0, 1.0);
        let magnify = mix(1.0, params.extra.w, smoothstep(0.0, 1.0, t));
        var p = in.position;
        p.x = p.x + sin(globals.time * 1.3 + seed * TAU) * params.extra.z * (1.0 - in.uv.y) * 0.12 * magnify;
        let size = in.scale_seed.x * magnify;
        let scaled = vec3<f32>(p.x * params.extra.x * size, p.y * params.extra.y * size, p.z * params.extra.x * size);
        world = rotate_y(scaled, c, s) + origin;
    } else {
        // Pond: flat disc of diameter `scale`, slightly magnified with the distance.
        let d = distance(view.world_position, origin);
        let near = params.range.x;
        let far = max(params.range.y, near * 1.01);
        let t = clamp(log(max(d, 1e-3) / near) / log(far / near), 0.0, 1.0);
        let size = in.scale_seed.x * mix(1.0, params.range.z, smoothstep(0.0, 1.0, t));
        let scaled = vec3<f32>(in.position.x * size, in.position.y, in.position.z * size);
        world = rotate_y(scaled, c, s) + origin;
    }
    out.world_position = world;
    out.position = position_world_to_clip(world);
    return out;
}

fn season_deciduous(cls: f32) -> bool {
    return cls < 1.5 || cls > 3.5;
}

// `foliage_season` of `foliage_common.gdshaderinc`.
fn foliage_season(albedo: vec3<f32>, cls: f32, seed: f32, leaves: f32, up: f32, world_y: f32) -> vec3<f32> {
    let w = params.season;
    let lum = dot(albedo, vec3<f32>(0.2126, 0.7152, 0.0722));
    let broadleaf = season_deciduous(cls);
    let conifer = cls > 1.5 && cls < 2.5;
    let hedge = cls > 2.5 && cls < 3.5;
    var spring = albedo * vec3<f32>(1.03, 1.10, 0.9);
    if conifer {
        spring = albedo;
    }
    var turn = smoothstep(0.15, 0.45, fract(seed * 7.13)) * mix(0.7, 1.0, leaves);
    if conifer {
        turn = 0.0;
    }
    var target_color = vec3<f32>(0.20, 0.07, 0.03);
    if cls < 0.5 {
        target_color = vec3<f32>(0.19, 0.13, 0.035);
    } else if cls < 1.5 {
        target_color = vec3<f32>(0.30, 0.10, 0.018);
    } else if cls > 3.5 {
        target_color = vec3<f32>(0.34, 0.22, 0.03);
    }
    let autumn = mix(albedo, target_color * (lum / 0.09) * mix(0.8, 1.2, leaves), turn * 0.85);
    let bare = vec3<f32>(0.105, 0.090, 0.082);
    var winter: vec3<f32>;
    if broadleaf {
        winter = bare * mix(0.8, 1.25, leaves);
    } else if hedge {
        winter = mix(albedo, bare, 0.7);
    } else {
        let snow = max(smoothstep(3.0, 5.0, world_y), 0.5 * smoothstep(0.45, 0.9, up));
        winter = mix(albedo * 0.8, vec3<f32>(0.75, 0.78, 0.82), snow * smoothstep(0.35, 0.8, leaves) * 0.7);
    }
    return spring * w.x + albedo * w.y + autumn * w.z + winter * w.w;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let mode = params.frame.w;
    var color = vec4<f32>(1.0);
    var normal = vec3<f32>(0.0, 1.0, 0.0);
    var occlusion = 1.0;
    var roughness = 0.9;
    if mode < 0.5 {
        let pick = in.misc.x > bayer4(in.position.xy);
        let uv = select(in.uv0, in.uv1, pick);
        let t = textureSample(albedo_texture, albedo_sampler, uv);
        let nt = textureSample(normal_texture, albedo_sampler, uv);
        let seed = in.misc.y;
        let cls = params.tint.w;
        let leaves = value_noise(in.local * 24.0 + vec2<f32>(seed * 17.0, seed * 5.0));
        // Winter: broadleaf crowns are openwork.
        if season_deciduous(cls) && params.season.w > 0.5
            && value_noise(in.local * 40.0 + vec2<f32>(seed * 29.0, seed * 3.0)) < 0.45 {
            discard;
        }
        if t.a < params.frame.z {
            discard;
        }
        let n = nt.rgb * 2.0 - 1.0;
        let baked = normalize(in.right * n.x + in.up * n.y + in.face * max(n.z, 0.05));
        // Lifted towards the sky: the instanced pipeline has no backlight.
        normal = normalize(mix(baked, vec3<f32>(0.0, 1.0, 0.0), 0.45));
        var albedo = t.rgb * in.tint * mix(0.85, 1.12, leaves) * params.wind.w;
        let lum = max(dot(albedo, vec3<f32>(0.2126, 0.7152, 0.0722)), 1e-3);
        albedo = albedo * max(1.0, 0.045 / lum);
        color = vec4<f32>(foliage_season(albedo, cls, seed, leaves, baked.y, in.misc.z), 1.0);
        occlusion = mix(0.55, 1.0, nt.a);
    } else if mode < 1.5 {
        let t = textureSample(albedo_texture, albedo_sampler, in.uv0);
        if t.a < params.frame.z || hash12(in.position.xy) > params.range.w {
            discard;
        }
        color = vec4<f32>(t.rgb * in.tint * 1.6, 1.0);
        roughness = 0.85;
    } else {
        let r = length(in.uv0 - vec2<f32>(0.5)) * 2.0;
        if r > 1.0 || hash12(in.position.xy) > smoothstep(1.0, 0.7, r) * params.range.w {
            discard;
        }
        color = vec4<f32>(in.tint, 1.0);
        roughness = 0.12;
    }

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = roughness;
    pbr_input.material.metallic = 0.0;
    pbr_input.material.flags = pbr_input.material.flags | pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    pbr_input.diffuse_occlusion = vec3<f32>(occlusion);
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = vec4<f32>(in.world_position, 1.0);
    pbr_input.world_normal = normal;
    pbr_input.N = normal;
    pbr_input.V = pbr_functions::calculate_view(pbr_input.world_position, false);

    let lit = pbr_functions::apply_pbr_lighting(pbr_input);
    return pbr_functions::main_pass_post_lighting_processing(pbr_input, lit);
}
