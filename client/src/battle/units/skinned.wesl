// GPU-skinned, GPU-animated figures (port of the vertex stage of battle_soldier_skinned.gdshader
// and of its colour logic for the generated `GA3_TEX` figures). Lit with Bevy's PBR code.
// Instance record: (x, y, z, yaw_deg) (scale, id, anim_time, far_blend) (volley_time, highlight, 0, 0).
import bevy_pbr::render::{
    mesh_types,
    pbr_functions,
    pbr_types,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
};

struct Params {
    livery: vec4<f32>,        // rgb, livery share
    trim: vec4<f32>,
    cur_set: array<vec4<f32>, 2>,
    prev_set: array<vec4<f32>, 2>,
    cur: vec4<f32>,           // speed, mode, size, cycle
    prev: vec4<f32>,          // speed, mode, size, blend_since
    timing: vec4<f32>,        // blend_time, reload_time, release, size_jitter
    jitter: vec4<f32>,        // cur jitter, prev jitter, bone fps, variant count
    ga3: vec4<f32>,           // lum livery, lum cloth, head_y, has albedo
    wear: vec4<f32>,          // plain count, coat count, mud height, weathering
    plain: array<vec4<f32>, 4>,
    coat: array<vec4<f32>, 4>,
    extra: vec4<f32>,         // prev cycle, prev release, impostor width (0 = figures), impostor height
    fx: vec4<f32>,            // cycle blend (s), hit time, hit length, hit share
    hitn: vec4<f32>,          // hit set size
    hit_set: array<vec4<f32>, 2>,
    clips: array<vec4<f32>, 96>,
}

@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;
@group(2) @binding(2) var bone_texture: texture_2d<f32>;
@group(2) @binding(3) var<uniform> params: Params;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) color: vec4<f32>,
    @location(10) bones: vec4<f32>,
    @location(11) weights: vec4<f32>,
    @location(12) mask: f32,
    @location(13) at_yaw: vec4<f32>,
    @location(14) sidt: vec4<f32>,
    @location(15) extra: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) rest: vec3<f32>,
    @location(5) hash: vec4<f32>,
    @location(6) misc: vec2<f32>,
}

const C_LIVERY = 0;
const C_TRIM = 1;
const C_PLATE = 2;
const C_MAIL = 3;
const C_CLOTH = 4;
const C_ARMS = 6;
const C_SKIN = 7;
const C_LEATHER = 8;
const C_HAIR = 9;
const C_COAT = 10;
const C_WOOD = 11;
const C_QUILT = 12;

fn hash1(n: f32) -> f32 {
    return fract(sin(n * 12.9898 + 4.1414) * 43758.5453);
}

fn rotate_y(v: vec3<f32>, c: f32, s: f32) -> vec3<f32> {
    return vec3<f32>(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
}

fn set_at(a: vec4<f32>, b: vec4<f32>, i: i32) -> i32 {
    let k = clamp(i, 0, 7);
    if k < 4 {
        return i32(a[k] + 0.5);
    }
    return i32(b[k - 4] + 0.5);
}

struct Pose {
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
}

fn bone_row(row: i32, idx: vec4<i32>, w: vec4<f32>) -> Pose {
    var a = vec4<f32>(0.0);
    var b = vec4<f32>(0.0);
    var c = vec4<f32>(0.0);
    for (var k = 0; k < 4; k++) {
        let wk = w[k];
        if wk > 0.0 {
            let x = idx[k] * 3;
            a += textureLoad(bone_texture, vec2<i32>(x, row), 0) * wk;
            b += textureLoad(bone_texture, vec2<i32>(x + 1, row), 0) * wk;
            c += textureLoad(bone_texture, vec2<i32>(x + 2, row), 0) * wk;
        }
    }
    return Pose(a, b, c);
}

fn hit_at(k: i32) -> i32 {
    let i = clamp(k, 0, 7);
    return i32(params.hit_set[i >> 2u][i & 3] + 0.5);
}

// Skinning rows of `clip` at local time `t` (seconds).
fn clip_pose(clip: i32, t: f32, idx: vec4<i32>, w: vec4<f32>) -> Pose {
    let cdef = params.clips[clamp(clip, 0, 95)];
    let frames = max(cdef.y, 1.0);
    var x = max(t, 0.0) * params.jitter.z;
    var f0 = 0.0;
    var f1 = 0.0;
    if cdef.z > 0.5 {
        x = x - frames * floor(x / frames);
        f0 = floor(x);
        f1 = f0 + 1.0;
        if f1 >= frames {
            f1 = 0.0;
        }
    } else {
        x = min(x, frames - 1.0);
        f0 = floor(x);
        f1 = min(f0 + 1.0, frames - 1.0);
    }
    let f = x - f0;
    let r0 = i32(cdef.x + f0);
    let r1 = i32(cdef.x + f1);
    var pose = bone_row(r0, idx, w);
    if f > 0.01 && r1 != r0 {
        let q = bone_row(r1, idx, w);
        pose = Pose(mix(pose.a, q.a, f), mix(pose.b, q.b, f), mix(pose.c, q.c, f));
    }
    return pose;
}

// Animation of one soldier for a clip set: returns the skinning rows at its time.
// Mode 3 (fixed): corpses and wounded, clip `pick` of the set, played once from the fall time
// `anim_time` up to `max_t` seconds then held; `params.cur.w` is the current time.
fn anim_pose(
    s0: vec4<f32>, s1: vec4<f32>, size: f32, speed: f32, jit: f32, mode: f32, cycle_len: f32,
    release: f32, h: vec4<f32>, anim_time: f32, volley_time: f32, pick: f32, max_t: f32,
    idx: vec4<i32>, w: vec4<f32>,
) -> Pose {
    let n = max(i32(size + 0.5), 1);
    var clip = 0;
    var t = 0.0;
    var blend_prev = false;
    var prev_clip = 0;
    var prev_t = 0.0;
    var prev_k = 1.0;
    if mode > 2.5 {
        clip = set_at(s0, s1, clamp(i32(pick + 0.5), 0, n - 1));
        t = clamp(params.cur.w - anim_time, 0.0, max_t);
    } else if mode > 1.5 {
        clip = set_at(s0, s1, min(i32(h.x * f32(n)), n - 1));
        let reload = max(params.timing.y, 0.5);
        let raw = anim_time - volley_time + release + (h.z - 0.5) * 0.4;
        t = raw - reload * floor(raw / reload);
    } else if mode > 0.5 {
        let local = anim_time * mix(0.9, 1.1, h.y) + h.z * cycle_len;
        let cyc = floor(local / cycle_len);
        clip = set_at(s0, s1, min(i32(hash1(h.x * 91.7 + cyc * 13.1) * f32(n)), n - 1));
        t = local - cycle_len * cyc;
        // Reaction to losses: a share of the soldiers plays a `hit` clip.
        let hit_age = anim_time - params.fx.y;
        let hn = i32(params.hitn.x + 0.5);
        var reacting = false;
        if hn > 0 && hit_age >= 0.0 && hit_age < params.fx.z + 0.2 {
            let key = floor(params.fx.y * 8.0);
            let delay = h.z * 0.25;
            if hash1(h.x * 57.3 + key * 3.7) < params.fx.w && hit_age > delay {
                clip = hit_at(min(i32(hash1(h.y * 31.1 + key) * f32(hn)), hn - 1));
                t = hit_age - delay;
                reacting = true;
            }
        }
        // Fade from the previous exchange of the cycle.
        if params.fx.x > 0.001 && n >= 2 && !reacting && (local - cycle_len * cyc) < params.fx.x {
            let pcyc = cyc - 1.0;
            prev_clip = set_at(s0, s1, min(i32(hash1(h.x * 91.7 + pcyc * 13.1) * f32(n)), n - 1));
            if prev_clip != clip {
                blend_prev = true;
                prev_t = cycle_len + t;
                prev_k = smoothstep(0.0, params.fx.x, t);
            }
        }
    } else {
        clip = set_at(s0, s1, min(i32(h.x * f32(n)), n - 1));
        t = anim_time * speed * mix(1.0 - jit, 1.0 + jit, h.y) + h.z * 11.0;
    }
    var pose = clip_pose(clip, t, idx, w);
    if blend_prev {
        let q = clip_pose(prev_clip, prev_t, idx, w);
        pose = Pose(mix(q.a, pose.a, prev_k), mix(q.b, pose.b, prev_k), mix(q.c, pose.c, prev_k));
    }
    return pose;
}

@vertex
fn vertex(in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let id = in.sidt.y;
    if params.extra.z > 0.0 {
        // Impostor: a quad standing on the soldier's feet, turned towards the camera.
        let to_cam = view.world_position - in.at_yaw.xyz;
        let flat = normalize(vec3<f32>(to_cam.x, 0.0, to_cam.z) + vec3<f32>(0.0001, 0.0, 0.0));
        let right = vec3<f32>(flat.z, 0.0, -flat.x);
        let world = in.at_yaw.xyz + right * (in.position.x * params.extra.z * in.sidt.x)
            + vec3<f32>(0.0, in.position.y * params.extra.w * in.sidt.x, 0.0);
        out.world_position = world;
        out.world_normal = normalize(mix(vec3<f32>(0.0, 1.0, 0.0), flat, 0.55));
        out.position = position_world_to_clip(world);
        out.uv = in.uv;
        out.color = vec4<f32>(0.0);
        out.rest = vec3<f32>(0.0);
        out.hash = vec4<f32>(
            hash1(id * 1.37 + 0.11), hash1(id * 2.71 + 0.53),
            hash1(id * 0.73 + 1.9), hash1(id * 3.91 + 2.3),
        );
        out.misc = vec2<f32>(in.sidt.w, in.extra.y);
        return out;
    }
    let anim_time = in.sidt.z;
    let h = vec4<f32>(
        hash1(id * 1.37 + 0.11), hash1(id * 2.71 + 0.53),
        hash1(id * 0.73 + 1.9), hash1(id * 3.91 + 2.3),
    );
    // Variant of the soldier: parts absent from it collapse to a point.
    let variants = max(i32(params.jitter.w + 0.5), 1);
    let mask = i32(in.mask + 0.5);
    let variant = min(i32(h.w * f32(variants)), variants - 1);
    let bits = mask & 63;
    let shown = bits == 0 || ((bits >> u32(variant)) & 1) == 1;

    let idx = vec4<i32>(in.bones + vec4<f32>(0.5));
    var pose = anim_pose(
        params.cur_set[0], params.cur_set[1], params.cur.z, params.cur.x, params.jitter.x,
        params.cur.y, params.cur.w, params.timing.z, h, anim_time, in.extra.x, in.extra.z, in.extra.w, idx, in.weights,
    );
    let blend = clamp((anim_time - params.prev.w) / max(params.timing.x, 0.01), 0.0, 1.0);
    if blend < 1.0 {
        let prev = anim_pose(
            params.prev_set[0], params.prev_set[1], params.prev.z, params.prev.x, params.jitter.y,
            params.prev.y, params.extra.x, params.extra.y, h, anim_time, in.extra.x, in.extra.z, in.extra.w, idx, in.weights,
        );
        let s = smoothstep(0.0, 1.0, blend);
        pose = Pose(mix(prev.a, pose.a, s), mix(prev.b, pose.b, s), mix(prev.c, pose.c, s));
    }
    let p = vec4<f32>(in.position, 1.0);
    var v = vec3<f32>(dot(pose.a, p), dot(pose.b, p), dot(pose.c, p));
    let n = vec3<f32>(dot(pose.a.xyz, in.normal), dot(pose.b.xyz, in.normal), dot(pose.c.xyz, in.normal));
    let sj = params.timing.w;
    v *= mix(1.0 - sj, 1.0 + sj, h.y);

    let yaw = radians(in.at_yaw.w);
    let c = cos(yaw);
    let s = sin(yaw);
    var local_pos = v * in.sidt.x;
    if !shown {
        local_pos = vec3<f32>(0.0);
    }
    let world = rotate_y(local_pos, c, s) + in.at_yaw.xyz;
    out.world_position = world;
    out.world_normal = rotate_y(normalize(n), c, s);
    out.position = position_world_to_clip(world);
    out.uv = in.uv;
    out.color = in.color;
    out.rest = in.position;
    out.hash = h;
    out.misc = vec2<f32>(in.sidt.w, in.extra.y);
    return out;
}

// Natural colours (linear) of undyed clothes.
fn plain_colour(i: i32) -> vec3<f32> {
    switch i {
        case 0: { return vec3<f32>(0.50, 0.42, 0.27); }
        case 1: { return vec3<f32>(0.17, 0.10, 0.05); }
        case 2: { return vec3<f32>(0.26, 0.11, 0.05); }
        case 3: { return vec3<f32>(0.26, 0.25, 0.21); }
        case 4: { return vec3<f32>(0.07, 0.065, 0.055); }
        case 5: { return vec3<f32>(0.15, 0.16, 0.08); }
        case 6: { return vec3<f32>(0.13, 0.16, 0.23); }
        default: { return vec3<f32>(0.40, 0.28, 0.10); }
    }
}

fn hair_colour(i: i32) -> vec3<f32> {
    switch i {
        case 0: { return vec3<f32>(0.035, 0.022, 0.012); }
        case 1: { return vec3<f32>(0.09, 0.05, 0.02); }
        case 2: { return vec3<f32>(0.20, 0.12, 0.05); }
        case 3: { return vec3<f32>(0.012, 0.010, 0.009); }
        default: { return vec3<f32>(0.28, 0.26, 0.23); }
    }
}

fn coat_colour(i: i32) -> vec3<f32> {
    switch i {
        case 0: { return vec3<f32>(0.16, 0.07, 0.03); }
        case 1: { return vec3<f32>(0.26, 0.10, 0.03); }
        case 2: { return vec3<f32>(0.03, 0.025, 0.022); }
        case 3: { return vec3<f32>(0.34, 0.33, 0.31); }
        default: { return vec3<f32>(0.36, 0.26, 0.13); }
    }
}

fn skin_colour(i: i32) -> vec3<f32> {
    switch i {
        case 0: { return vec3<f32>(1.0, 0.93, 0.85); }
        case 1: { return vec3<f32>(0.9, 0.78, 0.66); }
        case 2: { return vec3<f32>(0.78, 0.6, 0.48); }
        case 3: { return vec3<f32>(0.66, 0.5, 0.4); }
        default: { return vec3<f32>(0.55, 0.4, 0.32); }
    }
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash1(i.x + i.y * 57.0);
    let b = hash1(i.x + 1.0 + i.y * 57.0);
    let c = hash1(i.x + (i.y + 1.0) * 57.0);
    let d = hash1(i.x + 1.0 + (i.y + 1.0) * 57.0);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn impostor_colour(in: VertexOutput) -> vec4<f32> {
    // Silhouette painted from the regiment's colours: head, torso (livery or undyed cloth), legs;
    // a rider over a horse body when the quad is wide.
    let h = in.hash;
    let livery = params.livery.rgb;
    let lum = dot(livery, vec3<f32>(0.3, 0.59, 0.11));
    let dyed = mix(livery, vec3<f32>(lum), 0.3) * mix(0.62, 0.9, h.x);
    let h5 = fract(h.x * 7.31 + h.z * 3.17);
    var cloth = dyed;
    if h5 >= params.livery.w {
        var plain = vec3<f32>(0.30, 0.25, 0.17);
        let plain_count = i32(params.wear.x + 0.5);
        if plain_count > 0 {
            plain = params.plain[min(i32(h.y * f32(plain_count)), plain_count - 1)].rgb;
        }
        cloth = plain;
    }
    let u = in.uv.x - 0.5;
    let v = in.uv.y;
    var rgb = vec3<f32>(0.0);
    var alpha = 0.0;
    if params.extra.z > 1.5 {
        let coat = select(vec3<f32>(0.16, 0.08, 0.04), params.coat[min(i32(h.w * max(params.wear.y, 1.0)), 3)].rgb, params.wear.y > 0.5);
        if v < 0.42 && abs(u) < 0.13 {
            rgb = select(vec3<f32>(0.18, 0.16, 0.15), cloth, v > 0.1);
            alpha = 1.0;
        }
        if v > 0.3 && v < 0.95 && (u * u) / 0.2 + ((v - 0.62) * (v - 0.62)) / 0.12 < 1.0 {
            rgb = coat;
            alpha = 1.0;
        }
    } else if abs(u) < mix(0.42, 0.28, smoothstep(0.55, 1.0, v)) {
        alpha = 1.0;
        if v < 0.16 {
            rgb = vec3<f32>(0.22, 0.2, 0.19);
        } else if v < 0.58 {
            rgb = cloth;
        } else {
            rgb = vec3<f32>(0.07, 0.06, 0.055);
        }
    }
    return vec4<f32>(rgb, alpha);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    if params.extra.z > 0.0 {
        let imp = impostor_colour(in);
        if imp.a < 0.5 {
            discard;
        }
        var pbr_input = pbr_types::pbr_input_new();
        pbr_input.material.base_color = vec4<f32>(imp.rgb * (1.0 + in.misc.y * 0.25), 1.0);
        pbr_input.material.perceptual_roughness = 0.8;
        pbr_input.material.flags = pbr_input.material.flags | pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
        // Receive the cascaded sun shadows (pbr_input_new leaves the mesh flags at 0).
        pbr_input.flags = mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT;
        // Same glow as the figures' livery at a distance.
        pbr_input.material.emissive = vec4<f32>(imp.rgb * 0.12, 1.0);
        pbr_input.frag_coord = in.position;
        pbr_input.world_position = vec4<f32>(in.world_position, 1.0);
        pbr_input.world_normal = normalize(in.world_normal);
        pbr_input.N = normalize(in.world_normal);
        pbr_input.V = pbr_functions::calculate_view(pbr_input.world_position, false);
        let lit_imp = pbr_functions::apply_pbr_lighting(pbr_input);
        return pbr_functions::main_pass_post_lighting_processing(pbr_input, lit_imp);
    }
    let code = i32(in.color.a + 0.5);
    let h = in.hash;
    let h5 = fract(h.x * 7.31 + h.z * 3.17);
    let h6 = fract(h.y * 5.13 + h.x * 11.7);
    let dist = length(in.world_position - view.world_position);
    let livery = params.livery.rgb;
    let lum = dot(livery, vec3<f32>(0.3, 0.59, 0.11));
    let dyed = mix(livery, vec3<f32>(lum), 0.3) * mix(0.62, 0.9, h.x);
    var albedo = in.color.rgb;
    var metallic = 0.0;
    var roughness = 0.8;
    var livery_surface = false;
    let plain_count = i32(params.wear.x + 0.5);
    let coat_count = i32(params.wear.y + 0.5);

    if code == C_LIVERY {
        if h5 < params.livery.w {
            albedo = dyed * in.color.rgb;
            livery_surface = true;
        } else {
            var plain = plain_colour(i32(h6 * 7.99));
            if plain_count > 0 {
                plain = params.plain[min(i32(h6 * f32(plain_count)), plain_count - 1)].rgb;
            }
            albedo = plain * mix(0.85, 1.15, h.z) * in.color.rgb * 1.6;
        }
    } else if code == C_TRIM {
        albedo = params.trim.rgb * in.color.rgb;
        metallic = 0.5;
        roughness = 0.45;
    } else if code == C_PLATE {
        albedo = in.color.rgb * mix(0.8, 1.1, h.z) * mix(vec3<f32>(1.0), vec3<f32>(0.95, 0.88, 0.8), step(0.8, h6));
        metallic = 0.85;
        roughness = mix(0.28, 0.5, h5);
    } else if code == C_MAIL {
        albedo = in.color.rgb * 0.9 * mix(0.85, 1.1, h.z);
        metallic = 0.8;
        roughness = 0.6;
    } else if code == C_CLOTH || code == C_QUILT {
        albedo = in.color.rgb * mix(0.8, 1.12, h.x);
    } else if code == C_ARMS {
        albedo = dyed;
        roughness = 0.65;
        livery_surface = true;
    } else if code == C_SKIN {
        albedo = in.color.rgb * mix(vec3<f32>(1.1, 1.06, 1.0), vec3<f32>(0.62, 0.5, 0.4), h6);
        roughness = 0.6;
    } else if code == C_LEATHER || code == C_WOOD {
        let grain = noise2(in.rest.xy * 40.0);
        albedo = in.color.rgb * mix(0.8, 1.15, grain) * mix(0.85, 1.12, h.z);
        roughness = select(0.6, 0.75, code == C_WOOD);
    } else if code == C_HAIR {
        albedo = hair_colour(i32(h6 * 4.99)) * select(1.0, 1.2, in.color.r > 0.2);
        roughness = 0.7;
    } else if code == C_COAT {
        var coat = coat_colour(i32(h.w * 4.99));
        if coat_count > 0 {
            coat = params.coat[min(i32(h.w * f32(coat_count)), coat_count - 1)].rgb;
        }
        albedo = coat * in.color.rgb;
        roughness = 0.6;
        let black_points = (coat.r < 0.2 && coat.r > 0.06 && coat.g < 0.5 * coat.r) || (coat.g > 0.2 && coat.b < 0.55 * coat.g);
        let lower = 1.0 - smoothstep(0.3, 0.62, in.rest.y);
        let is_body = abs(in.color.r - 0.3294) > 0.003;
        if !is_body && black_points {
            albedo = vec3<f32>(0.028, 0.022, 0.018) * mix(0.8, 1.2, h.z);
        } else if is_body {
            albedo = mix(albedo, select(albedo * 0.55, vec3<f32>(0.03, 0.024, 0.02), black_points), lower * select(0.6, 0.9, black_points));
            if fract(h.w * 37.3) > 0.83 && in.rest.x > 0.0 && in.rest.z < 0.0 && in.rest.y < 0.22 {
                albedo = mix(albedo, vec3<f32>(0.62, 0.6, 0.55), smoothstep(0.22, 0.16, in.rest.y));
            }
        }
    }

    // Generated figure: painted albedo; alpha = tintable area (livery or undyed cloth).
    if params.ga3.w > 0.5 && in.uv.x >= 0.0 && code != C_ARMS {
        let g3 = textureSampleLevel(albedo_texture, albedo_sampler, vec2<f32>(in.uv.x, 1.0 - in.uv.y), 0.0);
        let g3_l = dot(g3.rgb, vec3<f32>(0.3, 0.59, 0.11));
        if code == C_LIVERY {
            albedo = mix(g3.rgb, albedo * clamp(g3_l / params.ga3.x, 0.2, 2.2), g3.a);
        } else if code == C_CLOTH {
            let hose = plain_colour(i32(fract(h6 * 3.7 + h.w) * 7.99)) * mix(0.85, 1.15, h.z) * 1.5;
            albedo = mix(g3.rgb, hose * clamp(g3_l / params.ga3.y, 0.2, 2.2), g3.a);
        } else {
            albedo = g3.rgb;
            if in.rest.y > params.ga3.z {
                let sat = (g3.r - g3.b) / max(g3.r, 0.001);
                let skin = smoothstep(0.08, 0.16, g3_l) * smoothstep(0.03, 0.08, g3.r - g3.g)
                    * smoothstep(0.0, 0.02, g3.g - g3.b) * (1.0 - smoothstep(0.8, 0.9, sat));
                let calm = mix(g3.rgb, vec3<f32>(g3_l) * vec3<f32>(1.3, 0.93, 0.74), 0.4);
                let tone = mix(vec3<f32>(1.1, 1.06, 1.0), vec3<f32>(0.62, 0.5, 0.4), h6) / vec3<f32>(0.93, 0.86, 0.79);
                albedo = mix(albedo, calm * tone, skin);
            }
        }
        // Wear: faded dyes, mud rising from the feet.
        let sr_w = params.wear.w * (1.0 - smoothstep(60.0, 80.0, dist));
        if sr_w > 0.0 {
            let sr_l = dot(albedo, vec3<f32>(0.3, 0.59, 0.11));
            if code == C_LIVERY || code == C_CLOTH {
                albedo = mix(albedo, vec3<f32>(sr_l) * 1.08, 0.2 * sr_w);
            }
            let sr_k = select(select(0.9, 0.45, code == C_PLATE || code == C_MAIL || code == C_TRIM), 1.0, code == C_LIVERY || code == C_CLOTH || code == C_LEATHER);
            let edge_n = noise2(in.rest.xz * 9.0 + h.xy * 13.0) * 0.7 + noise2(in.rest.xz * 31.0) * 0.3;
            let mud_top = params.wear.z * mix(0.75, 1.1, h.w);
            let rise = 1.0 - smoothstep(mud_top * 0.45, mud_top, in.rest.y + (edge_n - 0.5) * 0.14);
            let mud = rise * sr_k * mix(0.35, 1.0, fract(h.z * 13.7 + h.w * 5.3)) * 0.85 * sr_w;
            let mud_c = mix(vec3<f32>(0.16, 0.12, 0.08), vec3<f32>(0.30, 0.25, 0.18), smoothstep(0.0, 0.12, in.rest.y)) * mix(0.8, 1.1, edge_n);
            albedo = mix(albedo, mud_c, mud);
            roughness = mix(roughness, 0.9, mud);
            metallic = mix(metallic, 0.0, mud);
        }
    }

    // Occlusion near the ground, selection highlight.
    let ao = mix(0.6, 1.0, smoothstep(0.0, 1.2, in.rest.y));
    albedo = albedo * ao * (1.0 + in.misc.y * 0.25);

    var normal = normalize(in.world_normal);
    if !is_front {
        normal = -normal;
    }
    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);
    pbr_input.material.perceptual_roughness = clamp(roughness, 0.05, 1.0);
    pbr_input.material.metallic = metallic;
    pbr_input.material.flags = pbr_input.material.flags | pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    // Receive the cascaded sun shadows (pbr_input_new leaves the mesh flags at 0).
    pbr_input.flags = mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT;
    // Far readability: livery surfaces glow a little with distance (A1-01).
    let far = smoothstep(70.0, 175.0, dist);
    if livery_surface && far > 0.0 {
        pbr_input.material.emissive = vec4<f32>(dyed * far * 0.35, 1.0);
    }
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = vec4<f32>(in.world_position, 1.0);
    pbr_input.world_normal = normal;
    pbr_input.N = normal;
    pbr_input.V = pbr_functions::calculate_view(pbr_input.world_position, false);

    let lit = pbr_functions::apply_pbr_lighting(pbr_input);
    return pbr_functions::main_pass_post_lighting_processing(pbr_input, lit);
}
