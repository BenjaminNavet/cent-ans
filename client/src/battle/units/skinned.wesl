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
    arms: vec4<f32>,          // atlas layers: lord (or faction), 3 bannerets (-1 = none)
    chest_box: vec4<f32>,     // bust of the rest pose: centre x/y, width, height
    chest: vec4<f32>,         // z min, z max, banneret share, chest arms (nobles)
    badge_cross: vec4<f32>,   // livery cross of the commons (a > 0.5 = active)
    badge_patch: vec4<f32>,   // cloth patch under the cross
    fine: vec4<f32>,          // FG3 atlas layer (< 0 none), map distance (m), steel occlusion
    prop: vec4<f32>,          // rider's lance: bone (< 0 none), pennon share
    prop_grip: vec4<f32>,
    prop_axis: vec4<f32>,
    prop_side: vec4<f32>,
    clips: array<vec4<f32>, 96>,
}

@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;
@group(2) @binding(2) var bone_texture: texture_2d<f32>;
@group(2) @binding(3) var<uniform> params: Params;
@group(2) @binding(4) var arms_atlas: texture_2d_array<f32>;
@group(2) @binding(5) var arms_sampler: sampler;
// FG3 fine maps (fine_maps.rs): atlases of LOD0 / LOD1, shared detail tiles + albedo, horse coat.
@group(2) @binding(6) var fine_atlas0: texture_2d_array<f32>;
@group(2) @binding(7) var fine_atlas1: texture_2d_array<f32>;
@group(2) @binding(8) var fine_atlas_sampler: sampler;
@group(2) @binding(9) var fine_detail: texture_2d_array<f32>;
@group(2) @binding(10) var fine_detail_albedo: texture_2d_array<f32>;
@group(2) @binding(11) var fine_horse: texture_2d<f32>;
@group(2) @binding(12) var fine_repeat_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) atlas: f32,
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
    @location(7) rest_normal: vec3<f32>,
    @location(8) atlas: vec2<f32>,
    // 0 none, 1 LOD0 atlas, 3 LOD1 atlas, 2 horse coat (constant over a mesh).
    @location(9) @interpolate(flat) src: u32,
    // 1 on the rider's lance (Godot `v_prop`).
    @location(10) prop: f32,
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
        out.rest_normal = vec3<f32>(0.0, 1.0, 0.0);
        out.atlas = vec2<f32>(0.0);
        out.src = 0u;
        out.prop = 0.0;
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
    // CR4: the rider's own lance (length, angle, pennon wave) about the rest-pose grip, before
    // the skinning.
    var rest_pos = in.position;
    var rest_nrm = in.normal;
    var on_prop = 0.0;
    if params.prop.x >= 0.0 && idx.x == i32(params.prop.x + 0.5) && in.weights.x > 0.99 {
        on_prop = 1.0;
        let grip = params.prop_grip.xyz;
        let axis = params.prop_axis.xyz;
        let side = params.prop_side.xyz;
        let out_dir = normalize(cross(axis, side));
        var r = in.position - grip;
        let s_ax = dot(r, axis);
        if i32(in.color.a + 0.5) == C_LIVERY {
            if fract(h.z * 13.1 + h.x * 3.7) > params.prop.y {
                r = vec3<f32>(0.0);
            } else {
                // Pennon: a wave running to the free tip, growing in amplitude.
                let d = length(r - axis * s_ax);
                r += side * (0.07 * d * sin(anim_time * 7.0 - d * 11.0 + h.y * 6.283));
            }
        }
        let k_len = mix(0.9, 1.1, fract(h.x * 7.31 + h.z * 3.17));
        r += axis * (max(s_ax - 0.3, 0.0) * (k_len - 1.0));
        // Angle: +-3 degrees (lowered or raised), +-2 degrees sideways.
        let t1 = radians(3.0) * (fract(h.y * 5.7 + h.w * 1.3) * 2.0 - 1.0);
        let t2 = radians(2.0) * (fract(h.w * 9.1 + h.x * 2.3) * 2.0 - 1.0);
        r += t1 * cross(side, r) + t2 * cross(out_dir, r);
        rest_nrm = normalize(in.normal + t1 * cross(side, in.normal) + t2 * cross(out_dir, in.normal));
        rest_pos = grip + r;
    }
    out.prop = on_prop;
    let p = vec4<f32>(rest_pos, 1.0);
    var v = vec3<f32>(dot(pose.a, p), dot(pose.b, p), dot(pose.c, p));
    let n = vec3<f32>(dot(pose.a.xyz, rest_nrm), dot(pose.b.xyz, rest_nrm), dot(pose.c.xyz, rest_nrm));
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
    out.rest_normal = in.normal;
    // FG3 packed atlas uv: u, v on 11 bits, source on 2 (read only when the figure has a layer).
    let packed = u32(in.atlas + 0.5);
    out.src = select(0u, (packed >> 22u) & 3u, params.fine.x >= 0.0);
    out.atlas = vec2<f32>(f32(packed & 2047u), f32((packed >> 11u) & 2047u)) / 2047.0;
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

// DA1: arms layer of the figure (lord, or one of the bannerets for `chest.z` of them).
fn arms_layer(h: vec4<f32>) -> i32 {
    let l = params.arms;
    var layer = i32(l.x);
    let count = i32(l.y >= 0.0) + i32(l.z >= 0.0) + i32(l.w >= 0.0);
    let share = params.chest.z;
    let r = fract(h.x * 17.23 + h.w * 7.91);
    if count > 0 && r < share {
        let k = min(i32(r / share * f32(count)), count - 1);
        layer = select(select(i32(l.w + 0.5), i32(l.z + 0.5), k == 1), i32(l.y + 0.5), k == 0);
    }
    return layer;
}

// Explicit gradients: the samples sit in per-fragment branches (material code).
fn arms_sample(uv: vec2<f32>, layer: i32, ddx: vec2<f32>, ddy: vec2<f32>) -> vec4<f32> {
    if params.arms.x < 0.0 {
        return vec4<f32>(0.0);
    }
    return textureSampleGrad(arms_atlas, arms_sampler, uv, layer, ddx, ddy);
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
    // Arms (nobles) or livery cross (commons) on the torso patch, so a regiment keeps its
    // heraldry beyond 300 m (Godot paints none on its billboards). Gradients first: the sample
    // sits in branches.
    let rider = params.extra.z > 1.5;
    let patch_lo = select(vec2<f32>(-0.15, 0.2), vec2<f32>(-0.09, 0.12), rider);
    let patch_hi = select(vec2<f32>(0.15, 0.5), vec2<f32>(0.09, 0.4), rider);
    let patch_uv = (vec2<f32>(u, v) - patch_lo) / (patch_hi - patch_lo);
    let patch_arms_uv = mix(vec2<f32>(0.1, 0.06), vec2<f32>(0.9, 0.94), patch_uv);
    let patch_dx = dpdx(patch_arms_uv);
    let patch_dy = dpdy(patch_arms_uv);
    let in_patch = all(patch_uv > vec2<f32>(0.0)) && all(patch_uv < vec2<f32>(1.0));
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
    if alpha > 0.5 && in_patch {
        if params.chest.w > 0.5 && h5 < params.livery.w {
            let arms = arms_sample(patch_arms_uv, arms_layer(h), patch_dx, patch_dy);
            rgb = mix(rgb, arms.rgb * mix(0.86, 0.98, h.y), arms.a);
        } else if params.badge_cross.a > 0.5 {
            let d = abs(patch_uv - vec2<f32>(0.5, 0.56));
            if (d.x < 0.07 && d.y < 0.26) || (abs(patch_uv.y - 0.5) < 0.06 && d.x < 0.26) {
                rgb = params.badge_cross.rgb * mix(0.8, 0.95, h.y);
            }
        }
    }
    return vec4<f32>(rgb, alpha);
}

// --- FG3 fine maps (port of the FG3_BAKED parts of battle_soldier_skinned.gdshader) ---

// Normal perturbed by a tangent-space map without stored tangents: tangent T (towards +u) and
// bitangent B (towards +v of the map space, v up) solved from the screen derivatives of the
// position and of the uv, scaled by 1 / max(|T|, |B|) like Godot's cotangent frame. Unlike the
// Schuler formula, the sign of the uv determinant is applied explicitly, so the result does not
// depend on the direction of the screen y axis (`dpdy` is downwards in wgpu). Mirrored in Rust
// (`perturb_reference`, tested for handedness).
fn fg3_perturb(n: vec3<f32>, dp1: vec3<f32>, dp2: vec3<f32>, duv1: vec2<f32>, duv2: vec2<f32>, nxy: vec2<f32>) -> vec3<f32> {
    let det = duv1.x * duv2.y - duv1.y * duv2.x;
    let sgn = select(-1.0, 1.0, det >= 0.0);
    let t = (dp1 * duv2.y - dp2 * duv1.y) * sgn;
    let b = (dp2 * duv1.x - dp1 * duv2.x) * sgn;
    let m = max(dot(t, t), dot(b, b));
    if m < 1e-20 {
        return n;
    }
    let inv = inverseSqrt(m);
    return normalize(t * (inv * nxy.x) + b * (inv * nxy.y) + n);
}

// Skin tones (multiplied by the vertex colour): pale, rosy, tanned, olive, weathered, brown.
fn fg3_skin(i: i32) -> vec3<f32> {
    switch i {
        case 0: { return vec3<f32>(1.12, 1.04, 0.98); }
        case 1: { return vec3<f32>(1.08, 0.94, 0.86); }
        case 2: { return vec3<f32>(0.96, 0.82, 0.66); }
        case 3: { return vec3<f32>(0.88, 0.80, 0.62); }
        case 4: { return vec3<f32>(0.84, 0.66, 0.52); }
        default: { return vec3<f32>(0.62, 0.48, 0.36); }
    }
}

// Hair and beards (linear): chestnut, brown, ash blond, red, black, grizzled, white.
fn fg3_hair(i: i32) -> vec3<f32> {
    switch i {
        case 0: { return vec3<f32>(0.11, 0.065, 0.035); }
        case 1: { return vec3<f32>(0.065, 0.04, 0.022); }
        case 2: { return vec3<f32>(0.30, 0.22, 0.12); }
        case 3: { return vec3<f32>(0.26, 0.10, 0.04); }
        case 4: { return vec3<f32>(0.035, 0.03, 0.028); }
        case 5: { return vec3<f32>(0.20, 0.18, 0.16); }
        default: { return vec3<f32>(0.42, 0.40, 0.37); }
    }
}

// Side (m) of one tile of the generated detail layers (data/art/materials.yaml `tile_m`).
fn ga1_tile_size(i: i32) -> f32 {
    switch i {
        case 0: { return 0.15; }
        case 1: { return 0.13; }
        case 2: { return 0.16; }
        case 3: { return 0.32; }
        case 4: { return 0.51; }
        case 5: { return 0.3; }
        case 6: { return 0.3; }
        case 7: { return 0.8; }
        case 8: { return 0.06; }
        case 9: { return 0.05; }
        default: { return 0.1; }
    }
}

const GA1_WOOL = 0;
const GA1_LINEN = 1;
const GA1_FUSTIAN = 2;
const GA1_GAMBESON = 3;
const GA1_MAIL = 4;
const GA1_LEATHER = 5;
const GA1_PLATE = 6;
const GA1_WOOD = 7;
const GA1_SKIN = 8;
const GA1_HAIR = 9;
const GA1_COAT_DARK = 11;

// Plane of the surface (rest pose) used as the tile uv.
fn plane_uv(p: vec3<f32>, n: vec3<f32>) -> vec2<f32> {
    let an = abs(n);
    if an.x > an.y && an.x > an.z {
        return p.zy;
    } else if an.z > an.y {
        return p.xy;
    }
    return p.xz;
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
    // CR2/CR4 specular occlusion of steel and mail (Godot `steel_occlusion`, 0 = SR1 look).
    let steel_occ = params.fine.z;
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
    // DA1: arms coordinates on the bust (rest pose) and on the painted shield faces.
    let rn = in.rest_normal;
    let cb = params.chest_box;
    let cq = vec2<f32>((in.rest.x - cb.x) / cb.z + 0.5, (cb.y + 0.5 * cb.w - in.rest.y) / cb.w);
    // The bust faces -Z in the rest pose: front and back read the same way (dexter on the
    // viewer's left).
    let chest_uv = mix(vec2<f32>(0.1, 0.06), vec2<f32>(0.9, 0.94), vec2<f32>(select(cq.x, 1.0 - cq.x, rn.z > 0.0), cq.y));
    let chest_dx = dpdx(chest_uv);
    let chest_dy = dpdy(chest_uv);
    let shield_uv = vec2<f32>(mix(0.08, 0.92, in.uv.x), mix(0.06, 0.94, in.uv.y));
    let shield_dx = dpdx(shield_uv);
    let shield_dy = dpdy(shield_uv);
    let layer = arms_layer(h);
    var normal = normalize(in.world_normal);
    if !is_front {
        normal = -normal;
    }
    // FG3 derivatives (uniform control flow): position, atlas uv, tile plane uv, normal.
    var fg_q = plane_uv(in.rest, in.rest_normal);
    // CR4: coordinates of the shaft (turn, length along the axis): wood grain along the lance.
    var lance_s = 0.0;
    let on_lance = in.prop > 0.5;
    if on_lance {
        let lr = in.rest - params.prop_grip.xyz;
        let l_axis = params.prop_axis.xyz;
        let l_side = params.prop_side.xyz;
        lance_s = dot(lr, l_axis);
        let rr = lr - l_axis * lance_s;
        fg_q = vec2<f32>(atan2(dot(rr, l_side), dot(rr, normalize(cross(l_axis, l_side)))) * 0.03, lance_s);
    }
    if code == C_COAT && in.src == 2u {
        // Coat lying backwards and down on the trunk (tile: hair along v), straight on the legs.
        let ang = 1.15 * smoothstep(0.75, 1.0, in.rest.y);
        let ca = cos(ang);
        let sa = sin(ang);
        fg_q = mat2x2<f32>(vec2<f32>(ca, sa), vec2<f32>(-sa, ca)) * fg_q;
    }
    let fg_dp1 = dpdx(in.world_position);
    let fg_dp2 = dpdy(in.world_position);
    let fg_da1 = dpdx(in.atlas);
    let fg_da2 = dpdy(in.atlas);
    let fg_dq1 = dpdx(fg_q);
    let fg_dq2 = dpdy(fg_q);
    let fg_dn1 = dpdx(normal);
    let fg_dn2 = dpdy(normal);
    var fg_ao = 1.0;

    if code == C_LIVERY {
        let wears = h5 < params.livery.w;
        if wears {
            albedo = dyed * in.color.rgb;
            livery_surface = true;
        } else {
            var plain = plain_colour(i32(h6 * 7.99));
            if plain_count > 0 {
                plain = params.plain[min(i32(h6 * f32(plain_count)), plain_count - 1)].rgb;
            }
            albedo = plain * mix(0.85, 1.15, h.z) * in.color.rgb * 1.6;
        }
        // Arms (nobles) or livery cross (commons) on the chest and back.
        let torso = abs(rn.z) > 0.35 && in.rest.z > params.chest.x && in.rest.z < params.chest.y
            && cq.x > 0.0 && cq.x < 1.0 && cq.y > 0.0 && cq.y < 1.0 && dist < 60.0;
        if torso && params.chest.w > 0.5 && wears {
            let arms = arms_sample(chest_uv, layer, chest_dx, chest_dy);
            albedo = mix(albedo, arms.rgb * mix(0.86, 0.98, h.y) * in.color.rgb, arms.a);
        } else if torso && params.badge_cross.a > 0.5 {
            let d = abs(cq - vec2<f32>(0.5, 0.56));
            if params.badge_patch.a > 0.5 && d.x < 0.3 && d.y < 0.28 {
                albedo = params.badge_patch.rgb * mix(0.8, 0.95, h.y) * in.color.rgb;
            }
            if (d.x < 0.07 && d.y < 0.26) || (abs(cq.y - 0.5) < 0.06 && d.x < 0.26) {
                albedo = params.badge_cross.rgb * mix(0.8, 0.95, h.y) * in.color.rgb;
            }
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
        let inside = in.uv.x >= 0.0 && in.uv.x <= 1.0 && in.uv.y >= 0.0 && in.uv.y <= 1.0;
        var arms = vec4<f32>(0.0);
        if inside {
            arms = arms_sample(shield_uv, layer, shield_dx, shield_dy);
        }
        // Saturated arms over the dyed cloth (±6 % brightness).
        albedo = mix(dyed, arms.rgb * mix(0.84, 0.96, h.y), arms.a);
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
        var is_body = abs(in.color.r - 0.3294) > 0.003;
        if in.src != 0u {
            is_body = in.src == 2u;
        }
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

    // FG3: baked maps (LOD0 / LOD1 of the fine figures and the horse coat); LOD2 and the
    // impostors have src = 0 and read nothing. Explicit gradients: the samples sit in branches.
    if in.src != 0u && dist < params.fine.y {
        let layer_i = i32(params.fine.x + 0.5);
        var at = vec4<f32>(0.5, 0.5, 1.0, 1.0);
        if in.src == 2u {
            at = textureSampleGrad(fine_horse, fine_repeat_sampler, in.atlas, fg_da1, fg_da2);
        } else if in.src == 1u {
            at = textureSampleGrad(fine_atlas0, fine_atlas_sampler, in.atlas, layer_i, fg_da1, fg_da2);
        } else {
            at = textureSampleGrad(fine_atlas1, fine_atlas_sampler, in.atlas, layer_i, fg_da1, fg_da2);
        }
        // The map's v goes down: the frame takes -v. Plates and trims: attenuated shape normal.
        let shape = select(1.0, 0.35, code == C_PLATE || code == C_TRIM);
        var n = fg3_perturb(normal, fg_dp1, fg_dp2, vec2<f32>(fg_da1.x, -fg_da1.y), vec2<f32>(fg_da2.x, -fg_da2.y), (at.rg * 2.0 - 1.0) * shape);
        var mask = at.a;
        if in.src == 2u {
            // Horse: coat relief and fine occlusion.
            albedo *= mix(0.6, 1.4, at.b);
            fg_ao = at.a;
            mask = 1.0;
        } else {
            fg_ao = at.b;
        }
        if code == C_PLATE || code == C_TRIM {
            fg_ao = mix(0.7, 1.0, smoothstep(0.25, 0.85, fg_ao));
        }
        var tl = false;
        var amt = 0.0;
        var relief = 0.5;
        let skin_tone = vec3<f32>(0.50, 0.33, 0.24) * fg3_skin(i32(h6 * 5.99));
        if code == C_MAIL {
            tl = true;
            amt = 1.0;
            relief = 0.0;
        } else if code == C_PLATE {
            tl = true;
            amt = mix(0.05, 0.22, mask) + 0.3 * steel_occ;
            relief = 0.06;
        } else if code == C_TRIM {
            tl = true;
            amt = 0.1;
            relief = 0.06;
        } else if code == C_LIVERY || code == C_ARMS || code == C_CLOTH {
            tl = true;
            amt = 0.55;
            relief = 0.25;
        } else if code == C_QUILT {
            tl = true;
            amt = 0.7;
            relief = 0.35;
        } else if code == C_LEATHER {
            tl = true;
            amt = 0.8;
            relief = 0.45;
        } else if code == C_WOOD {
            tl = true;
            amt = 0.45;
            relief = 0.22;
        } else if code == C_SKIN {
            tl = true;
            amt = 0.35;
            relief = 0.12;
            // Varied skin tones; the mask darkens eyebrows and stubble towards the hair.
            albedo = in.color.rgb * fg3_skin(i32(h6 * 5.99)) * mix(0.94, 1.06, h.z);
            albedo = mix(fg3_hair(i32(h5 * 6.99)) * 1.4, albedo, mix(0.35, 1.0, mask));
        } else if code == C_HAIR || code == C_COAT {
            tl = true;
            let mane = code == C_COAT && in.src != 2u;
            amt = select(0.6, 0.9, code == C_HAIR || mane);
            relief = select(select(0.3, 0.35, mane), 0.0, code == C_HAIR);
        }
        // Generated layer of the material (GA1) and share of the detail albedo.
        var gl = -1;
        var ga_mix = 0.0;
        if tl {
            if code == C_MAIL {
                gl = GA1_MAIL;
                ga_mix = 1.0;
            } else if code == C_PLATE || code == C_TRIM {
                gl = GA1_PLATE;
                ga_mix = select(0.3, 0.6, code == C_PLATE);
            } else if code == C_LIVERY || code == C_ARMS {
                gl = GA1_FUSTIAN;
                ga_mix = 0.6;
            } else if code == C_CLOTH {
                gl = select(GA1_LINEN, GA1_WOOL, h5 < 0.5);
                ga_mix = 0.8;
            } else if code == C_QUILT {
                gl = GA1_GAMBESON;
                ga_mix = 0.8;
            } else if code == C_LEATHER {
                gl = GA1_LEATHER;
                ga_mix = 0.85;
            } else if code == C_WOOD {
                gl = GA1_WOOD;
                ga_mix = 0.8;
            } else if code == C_SKIN {
                gl = GA1_SKIN;
                ga_mix = 0.45;
            } else if code == C_HAIR {
                gl = GA1_HAIR;
                ga_mix = 0.6;
            } else {
                // Lying-hair tile for every coat; manes: hair tile.
                gl = select(GA1_COAT_DARK, GA1_HAIR, in.src != 2u);
                ga_mix = select(0.45, 0.7, in.src != 2u);
            }
            let size = ga1_tile_size(gl);
            let tq = vec2<f32>(fg_q.x, -fg_q.y) / size;
            let tg1 = vec2<f32>(fg_dq1.x, -fg_dq1.y) / size;
            let tg2 = vec2<f32>(fg_dq2.x, -fg_dq2.y) / size;
            let d = textureSampleGrad(fine_detail, fine_repeat_sampler, tq, gl, tg1, tg2);
            n = fg3_perturb(n, fg_dp1, fg_dp2, fg_dq1, fg_dq2, (d.rg * 2.0 - 1.0) * amt);
            let ga_albedo = mix(vec3<f32>(1.0), textureSampleGrad(fine_detail_albedo, fine_repeat_sampler, tq, gl, tg1, tg2).rgb * 2.0, ga_mix);
            albedo *= ga_albedo;
            if code == C_MAIL {
                // Rings and hollows carried by the generated albedo; rings smooth (~0.35),
                // hollows rougher and nearly black.
                albedo = in.color.rgb * mix(0.85, 1.1, h.z) * ga_albedo * mix(1.0, 0.55, steel_occ);
                albedo *= mix(1.0, mix(0.08, 0.95, smoothstep(0.38, 0.66, d.b)), steel_occ);
                metallic = 0.6;
                roughness = max(d.a, mix(0.0, 0.5, steel_occ));
                fg_ao = mix(0.55, 1.0, fg_ao);
            } else if code == C_HAIR {
                // Beards and hair: lighter and varied colour, the density (mask) melts edges
                // into the skin, strands break the border.
                let hair = fg3_hair(i32(h6 * 6.99)) * select(1.0, 1.2, in.color.r > 0.2);
                let cover = smoothstep(0.25, 0.75, mask * (0.55 + 0.9 * d.b));
                albedo = mix(skin_tone, hair * mix(0.75, 1.2, d.b) * ga_albedo, cover);
                roughness = mix(0.6, 0.75, cover);
            } else {
                roughness = mix(roughness, d.a, min(amt, 1.0));
            }
        }
        // SR2 wear (procedural noise only), faded before the end of the maps.
        let sr_w = params.wear.w * (1.0 - smoothstep(params.fine.y * 0.75, params.fine.y, dist));
        if sr_w > 0.0 {
            let dyed_cloth = code == C_LIVERY || code == C_CLOTH || code == C_ARMS || (code == C_COAT && in.src != 2u);
            let fabric = dyed_cloth || code == C_QUILT;
            if dyed_cloth {
                let sr_l = dot(albedo, vec3<f32>(0.3, 0.59, 0.11));
                albedo = mix(albedo, vec3<f32>(sr_l) * 1.08, 0.25 * sr_w);
            }
            // Grime of the hollows: baked occlusion darkens and desaturates cloth and leather.
            if fabric || code == C_LEATHER {
                let cav = (1.0 - fg_ao) * sr_w;
                let sr_l = dot(albedo, vec3<f32>(0.3, 0.59, 0.11));
                albedo = mix(albedo, vec3<f32>(sr_l) * vec3<f32>(0.9, 0.85, 0.75), 0.6 * cav);
                albedo *= 1.0 - 0.35 * cav;
            }
            if code == C_PLATE {
                let lf = noise2(fg_q * 3.0 + h.xy * 17.0);
                let rough = clamp(
                    mix(mix(0.45, 0.2, steel_occ), mix(0.75, 0.42, steel_occ), lf)
                        + (roughness - 0.4) * mix(0.4, 0.2, steel_occ),
                    mix(0.4, 0.16, steel_occ), 0.8);
                roughness = mix(roughness, rough, sr_w);
                let steel = mix(vec3<f32>(0.50, 0.50, 0.52), vec3<f32>(0.42, 0.415, 0.40), steel_occ) * mix(0.85, 1.1, lf);
                albedo = mix(albedo, steel * (0.6 + 0.4 * dot(albedo, vec3<f32>(0.33))), 0.7 * sr_w);
                albedo *= mix(1.0, mix(0.55, 1.0, fg_ao), sr_w);
                let gb = smoothstep(0.5, 0.85, noise2(fg_q * 7.0 + h.zw * 29.0));
                let grime = clamp((1.0 - smoothstep(0.5, 0.95, fg_ao)) * 0.8 + gb * 0.35, 0.0, 1.0) * sr_w * steel_occ;
                albedo = mix(albedo, vec3<f32>(0.13, 0.11, 0.085), 0.75 * grime);
                roughness = mix(roughness, 0.85, grime);
                metallic = mix(metallic, 0.2, grime);
                let curv = (length(fg_dn1) + length(fg_dn2)) / max(length(fg_dp1) + length(fg_dp2), 1e-5);
                let wear = smoothstep(0.55, 0.85, noise2(fg_q * 55.0 + 3.1)) * smoothstep(12.0, 50.0, curv);
                albedo *= 1.0 + 0.25 * wear * sr_w;
                roughness -= 0.12 * wear * sr_w;
            } else if code == C_MAIL {
                let low = 1.0 - smoothstep(params.wear.z * 1.6, params.wear.z * 2.4, in.rest.y);
                let ox = low * smoothstep(0.45, 0.8, noise2(fg_q * 6.0 + h.zw * 23.0)) * 0.5 * sr_w;
                albedo = mix(albedo, vec3<f32>(0.20, 0.10, 0.045), ox);
                roughness = mix(roughness, 0.8, ox);
                metallic = mix(metallic, 0.3, ox);
            }
            // Mud and dust rising from the feet (noisy edge), own amount per soldier.
            let sr_k = select(
                select(
                    select(select(0.0, 0.45, code == C_PLATE || code == C_MAIL || code == C_TRIM), 0.7, code == C_WOOD || code == C_SKIN),
                    0.9, code == C_COAT),
                1.0, fabric || code == C_LEATHER);
            let edge_n = noise2(in.rest.xz * 9.0 + h.xy * 13.0) * 0.7 + noise2(in.rest.xz * 31.0) * 0.3;
            let mud_top = params.wear.z * mix(0.75, 1.1, h.w);
            let rise = 1.0 - smoothstep(mud_top * 0.45, mud_top, in.rest.y + (edge_n - 0.5) * 0.14);
            let blotch = mix(0.55, 1.0, smoothstep(0.25, 0.7, noise2(fg_q * 9.0 + h.yz * 31.0)));
            let mud = rise * blotch * sr_k * mix(0.35, 1.0, fract(h.z * 13.7 + h.w * 5.3)) * 0.85 * sr_w;
            let mud_c = mix(vec3<f32>(0.16, 0.12, 0.08), vec3<f32>(0.30, 0.25, 0.18), smoothstep(0.0, 0.12, in.rest.y)) * mix(0.8, 1.1, edge_n);
            albedo = mix(albedo, mud_c, mud);
            roughness = mix(roughness, 0.9, mud);
            metallic = mix(metallic, 0.0, mud);
        }
        albedo *= mix(1.0, fg_ao, 0.55);
        normal = n;
    }

    if on_lance && code == C_WOOD {
        // CR4: worn grip (wood blackened and polished by the gauntlets) from the handle to the
        // vamplate.
        let grip = smoothstep(-0.34, -0.22, lance_s) * (1.0 - smoothstep(0.06, 0.14, lance_s));
        albedo = mix(albedo, albedo * vec3<f32>(0.42, 0.36, 0.3), grip);
        roughness = mix(roughness, 0.42, grip);
    }

    // Occlusion near the ground, selection highlight.
    let ao = mix(0.6, 1.0, smoothstep(0.0, 1.2, in.rest.y));
    albedo = albedo * ao * (1.0 + in.misc.y * 0.25);

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.diffuse_occlusion = vec3<f32>(fg_ao);
    var spec_occ = fg_ao;
    if steel_occ > 0.0 && code == C_MAIL {
        // CR4: grazing mail (backlit): the rings hide each other, no Fresnel veil. Godot lowers
        // SPECULAR to 0.08 (f0 = 0.08 * SPECULAR, Bevy: 0.16 * reflectance^2 -> 0.2).
        let graz = 1.0 - smoothstep(0.12, 0.55, abs(dot(normal, pbr_functions::calculate_view(vec4<f32>(in.world_position, 1.0), false))));
        metallic = mix(metallic, 0.0, graz * steel_occ);
        pbr_input.material.reflectance = vec3<f32>(mix(0.447, 0.2, graz * steel_occ));
    }
    if steel_occ > 0.0 && (code == C_PLATE || code == C_TRIM || code == C_MAIL) {
        // CR2: sky occlusion of the reflection (Godot RADIANCE alpha): the reflection only reaches
        // the free sky when looking up (troops, ground, tree lines hide the horizon), hardly at all
        // in the baked hollows. Transition widened with roughness. Applied to the indirect
        // specular through the specular occlusion.
        let view_dir = pbr_functions::calculate_view(vec4<f32>(in.world_position, 1.0), false);
        let ry = reflect(-view_dir, normal).y;
        let w = 0.3 * clamp(roughness, 0.05, 1.0);
        let sky_seen = smoothstep(0.02 - w, 0.62 + w, ry);
        var occ = mix(0.12, 1.0, sky_seen * sky_seen);
        if code == C_MAIL {
            // Interlocked rings: each hides a good part of the sky from its neighbours.
            occ *= 0.5;
        }
        if in.src != 0u && dist < params.fine.y {
            occ *= mix(0.35, 1.0, smoothstep(0.35, 0.95, fg_ao));
        }
        spec_occ = fg_ao * mix(1.0, occ, steel_occ);
    }
    pbr_input.specular_occlusion = spec_occ;
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
