// Depth-only draw of the campaign trees into the sun's shadow maps (Godot: the tree MultiMeshes
// cast shadows up to `generalised_shadow_distance`, `Vegetation._tree_shadows_on`). Same record
// layout and parameter block as `billboard.wesl`; group 0 is the prepass view layout (view,
// globals) that Bevy binds for the shadow views, group 2 the material of the batch.
//
// Modes (`params.frame.w`): 0 impostor card, facing the light here (Godot's billboards face the
// view of the pass, i.e. the light in the shadow pass) with the baked view of the light's azimuth;
// 3 near tree model. Other modes are not queued.
import bevy_render::view::View;
import bevy_render::globals::Globals;

struct Params {
    season: vec4<f32>,
    tint: vec4<f32>,
    atlas: vec4<f32>,
    frame: vec4<f32>,
    wind: vec4<f32>,
    extra: vec4<f32>,
    range: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    fog: vec4<f32>,
    fog_range: vec4<f32>,
    grade: vec4<f32>,
    lift: vec4<f32>,
    highlights: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;

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
    @location(0) uv0: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) misc: vec4<f32>, // seed, model-zone weight, unclipped depth, unused
}

const TAU: f32 = 6.2831853;
const MAX_LOD: f32 = 2.5;
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

// Same gust as `billboard.wesl` (the shadow follows the swaying crown).
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

fn season_deciduous(cls: f32) -> bool {
    return cls < 1.5 || cls > 3.5;
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
    let jitter = 1.0 + 0.25 * (2.0 * fract(seed * 53.17) - 1.0);
    let gust = wind_gust(origin.xz);
    let shiver = sin(globals.time * 1.7 + seed * 23.0) * (0.4 + 0.6 * gust);
    var world: vec3<f32>;
    out.uv0 = in.uv;
    out.local = in.uv;
    if mode < 0.5 {
        // Impostor card facing the light (+Z of the light view points back towards it).
        let views = params.atlas.x;
        let rows = params.atlas.y;
        let bx = vec3<f32>(c, 0.0, -s);
        let bz = vec3<f32>(s, 0.0, c);
        let toward = normalize(view.world_from_view[2].xyz);
        let ax = dot(toward, bx);
        let az = dot(toward, bz);
        let k0 = floor(fract(atan2(-az, ax) / TAU + 1.0) * views + 0.5) % views;
        // Upright card turned towards the light's azimuth (a card tilted to face a high sun
        // would cast a sliver).
        let flat = normalize(vec3<f32>(toward.x, 0.0, toward.z) + vec3<f32>(1e-4, 0.0, 0.0));
        let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), flat));
        let up = vec3<f32>(0.0, 1.0, 0.0);
        let height = in.scale_seed.x * jitter;
        let width = height * params.atlas.w;
        let corner = in.uv;
        let local = vec2<f32>(corner.x - 0.5, 1.0 - corner.y);
        let ortho = params.frame.x;
        let foot = params.frame.y;
        world = origin + right * local.x * width * ortho + up * (local.y - foot) * height * ortho;
        let flex = local.y * local.y * height * ortho;
        world = world + vec3<f32>(WIND_DIR.x, 0.0, WIND_DIR.y) * flex
            * (params.wind.x * gust + params.wind.y * shiver);
        out.uv0 = (vec2<f32>(k0, params.atlas.z) + corner) / vec2<f32>(views, rows);
        out.local = local;
    } else {
        // Tree model (mode 3).
        let size = in.scale_seed.x * jitter;
        world = rotate_y(in.position * size, c, s) + origin;
        let flex = in.position.y * in.position.y * size;
        world = world + vec3<f32>(WIND_DIR.x, 0.0, WIND_DIR.y) * flex
            * (params.wind.x * gust + params.wind.y * shiver);
        out.local = vec2<f32>(in.position.x + in.position.z, in.position.y) * 6.0;
    }
    out.position = view.clip_from_world * vec4<f32>(world, 1.0);
    out.misc = vec4<f32>(seed, in.scale_seed.y, out.position.z, 0.0);
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.position.z = min(out.position.z, 1.0);
}
    return out;
}

// Alpha cut of the crown (same tests as the main pass). Derivatives taken before any discard.
fn cut(in: VertexOutput) {
    let mode = params.frame.w;
    let seed = in.misc.x;
    let zone = in.misc.y;
    let cls = params.tint.w;
    let winter = season_deciduous(cls) && params.season.w > 0.5;
    let texels = vec2<f32>(textureDimensions(albedo_texture, 0));
    let duv_x = dpdx(in.uv0 * texels);
    let duv_y = dpdy(in.uv0 * texels);
    let lod = clamp(0.5 * log2(max(max(dot(duv_x, duv_x), dot(duv_y, duv_y)), 1e-8)), 0.0, MAX_LOD);
    if mode < 0.5 {
        // Model / impostor band: complementary shares, as in the main pass.
        if zone > 0.0 && bayer4(in.position.xy) < zone {
            discard;
        }
        let t = textureSampleLevel(albedo_texture, albedo_sampler, in.uv0, lod);
        if winter && value_noise(in.local * 40.0 + vec2<f32>(seed * 29.0, seed * 3.0)) < 0.45 {
            discard;
        }
        if t.a * (1.0 + lod * 0.35) < params.frame.z {
            discard;
        }
    } else {
        if bayer4(in.position.xy) >= zone {
            discard;
        }
        if winter && value_noise(in.local * 2.4) < 0.5 {
            discard;
        }
    }
}

@fragment
fn fragment(in: VertexOutput) {
    cut(in);
}

// Directional shadow views without the depth-clip-control feature: depth written unclamped.
@fragment
fn fragment_unclipped(in: VertexOutput) -> @builtin(frag_depth) f32 {
    cut(in);
    return in.misc.z;
}
