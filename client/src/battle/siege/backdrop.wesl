// Backdrop of the siege of an emblematic town (port of game/shaders/landmark.gdshader, battle use:
// `fade` = 1, `height_in_meters` = false, `drape_scale` = 1, no tint, no debug views).
//
// Vertex: the model is draped on the battle ground. The UV of a vertex is the anchor of its
// building (local x, z); the height of the field under the anchor is read from `height_map`
// (192 x 192 baked by `backdrop.rs`, bilinear by hand since R32Float need not be filterable) and
// added to every vertex of the building, so the buildings stay rigid.
// Fragment: matter of the shared atlas (`textures/landmarks`, 16 layers) chosen by the vertex
// alpha (layer * 16 + exaggeration step), triplanar in the model's own frame (metres), palette
// colour x photo detail, one variation per building (hash of the anchor), ageing (dirt at the
// foot of the walls, rain streaks, moss on the upward faces), flat normals from derivatives.
import bevy_pbr::render::{
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
    forward_io::{Vertex, VertexOutput, FragmentOutput, decompress_vertex},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

struct BackdropParams {
    // x, y map origin (world), z map extent (m).
    map: vec4<f32>,
    // xyz position of the root, w its yaw (rad).
    root: vec4<f32>,
    // x water (1 = water), y ground height (m), z metres per unit, w ageing.
    look: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> params: BackdropParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var height_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var albedo_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var albedo_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var normal_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var normal_sampler: sampler;

// Size of a repetition of each layer (m), roughness (-1: the palette's), moss on the upward faces,
// dirt at the foot of the walls.
var<private> TILE: array<f32, 16> = array<f32, 16>(4.6, 3.5, 2.2, 2.6, 2.4, 2.6, 3.0, 5.0, 2.4, 3.0, 3.0, 2.4, 4.0, 6.0, 4.0, 9.0);
var<private> ROUGH: array<f32, 16> = array<f32, 16>(0.88, 0.92, 0.9, 0.55, 0.78, 0.85, 0.93, 0.9, 0.85, 0.97, 0.92, 0.42, 0.18, 0.97, -1.0, 0.9);
var<private> MOSS: array<f32, 16> = array<f32, 16>(0.35, 0.6, 0.35, 0.55, 0.75, 1.0, 0.3, 0.3, 0.5, 0.8, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0);
var<private> GRIME: array<f32, 16> = array<f32, 16>(1.0, 1.0, 0.8, 0.4, 0.5, 0.6, 1.0, 0.9, 0.8, 0.4, 0.3, 0.3, 0.0, 0.0, 0.0, 0.0);

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Height of the field (m) under the world point `xz`, bilinear like a linear-filtered sampler.
fn field_height(xz: vec2<f32>) -> f32 {
    let dims = vec2<f32>(textureDimensions(height_map));
    let uv = clamp((xz - params.map.xy) / params.map.z, vec2<f32>(0.0), vec2<f32>(1.0));
    let p = uv * dims - vec2<f32>(0.5);
    let p0 = floor(p);
    let f = p - p0;
    let lo = vec2<i32>(clamp(p0, vec2<f32>(0.0), dims - vec2<f32>(1.0)));
    let hi = vec2<i32>(clamp(p0 + vec2<f32>(1.0), vec2<f32>(0.0), dims - vec2<f32>(1.0)));
    let h00 = textureLoad(height_map, vec2<i32>(lo.x, lo.y), 0).r;
    let h10 = textureLoad(height_map, vec2<i32>(hi.x, lo.y), 0).r;
    let h01 = textureLoad(height_map, vec2<i32>(lo.x, hi.y), 0).r;
    let h11 = textureLoad(height_map, vec2<i32>(hi.x, hi.y), 0).r;
    return mix(mix(h00, h10, f.x), mix(h01, h11, f.x), f.y);
}

// Height added to the vertices of the building anchored at the local point `anchor`.
fn drape(world_from_local: mat4x4<f32>, anchor: vec2<f32>) -> f32 {
    let anchor_world = (world_from_local * vec4<f32>(anchor.x, 0.0, anchor.y, 1.0)).xyz;
    return field_height(anchor_world.xz);
}

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    var local = vertex.position;
    @if(VERTEX_UVS_A)
    {
        local.y = local.y + drape(world_from_local, vertex.uv);
    }
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(local, 1.0));
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);
    @if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);
    @if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    @if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;
    @if(VERTEX_COLORS)
    out.color = vertex.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, world_from_local[3]);
    return out;
}

// Rotation about Y by `a` (the root's yaw): world = rot_y(a) * local + position.
fn rot_y(v: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
}

// Triplanar sample of one layer; `w` weights of the three planes.
fn tri_albedo(p: vec3<f32>, w: vec3<f32>, layer: i32) -> vec3<f32> {
    let x = textureSample(albedo_array, albedo_sampler, p.zy, layer).rgb;
    let y = textureSample(albedo_array, albedo_sampler, p.xz, layer).rgb;
    let z = textureSample(albedo_array, albedo_sampler, p.xy, layer).rgb;
    return x * w.x + y * w.y + z * w.z;
}

fn tri_normal(p: vec3<f32>, w: vec3<f32>, layer: i32) -> vec3<f32> {
    let x = textureSample(normal_array, normal_sampler, p.zy, layer).xyz;
    let y = textureSample(normal_array, normal_sampler, p.xz, layer).xyz;
    let z = textureSample(normal_array, normal_sampler, p.xy, layer).xyz;
    return (x * w.x + y * w.y + z * w.z) * 2.0 - vec3<f32>(1.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let base_color = pbr_input.material.base_color.rgb;
    var base = base_color;
    var rough = pbr_input.material.perceptual_roughness;
    let yaw = params.root.w;
    let meters_per_unit = params.look.z;

    // Position in the model's own frame, before the drape (what the Godot shader calls local_pos).
    let rel = in.world_position.xyz - params.root.xyz;
    var local_pos = rot_y(rel, -yaw);
    // The drape lifted the building by the field height under its anchor.
    var anchor = vec2<f32>(0.0);
    var code_alpha = 0.0;
    @if(VERTEX_UVS_A)
    {
        anchor = in.uv;
    }
    @if(VERTEX_COLORS)
    {
        code_alpha = in.color.a;
    }
    let anchor_world = rot_y(vec3<f32>(anchor.x, 0.0, anchor.y), yaw) + params.root.xyz;
    local_pos.y = local_pos.y - field_height(anchor_world.xz);

    // Flat normal of the face, in the model's frame, turned towards the viewer.
    var n_local = normalize(cross(dpdx(local_pos), dpdy(local_pos)));
    let to_view = normalize(view.world_position - in.world_position.xyz);
    if dot(rot_y(n_local, yaw), to_view) < 0.0 {
        n_local = -n_local;
    }

    let code = i32(round(code_alpha * 255.0));
    let layer = clamp(code / 16, 0, 15);
    let exaggeration = exp2(f32(code % 16 - 4) * 0.25);
    let water = params.look.x > 0.5;
    let detailed = !water && layer != 14;

    // Atlas samples (all taken unconditionally: derivatives need uniform control flow).
    let seed = hash12(floor(anchor * 97.0) + vec2<f32>(0.37));
    let seed2 = hash12(floor(anchor * 61.0) + vec2<f32>(11.7));
    let p_m = local_pos * meters_per_unit / exaggeration;
    let tile = TILE[layer];
    let p = p_m / tile + vec3<f32>(seed * 7.0, seed2 * 3.0, seed2 * 5.0);
    var w = pow(abs(n_local), vec3<f32>(4.0));
    w = w / max(w.x + w.y + w.z, 1e-5);
    let d = tri_albedo(p, w, layer);
    let nm = tri_normal(p, w, layer);
    let masks = tri_albedo(p_m / TILE[15], w, 15);
    let macro_tone = tri_albedo(p_m / 41.0 + vec3<f32>(seed), w, 15);

    var normal_world = pbr_input.N;
    if detailed {
        var detail_col = base * d * 2.0;
        if layer == 12 {
            detail_col = d * 0.1;
        }
        detail_col = detail_col * (0.9 + 0.2 * seed);
        detail_col = detail_col * mix(vec3<f32>(1.0), vec3<f32>(1.04, 1.0, 0.94), seed2 - 0.3);
        detail_col = detail_col * (0.84 + 0.32 * macro_tone.r);
        detail_col = detail_col * mix(vec3<f32>(1.0), vec3<f32>(0.95, 0.97, 1.03), macro_tone.b);
        let h_m = max((local_pos.y - params.look.y) * meters_per_unit / exaggeration, 0.0);
        let vertical = 1.0 - abs(n_local.y);
        let age = params.look.w * (0.55 + 0.9 * seed2);
        let grime = GRIME[layer];
        let foot = (1.0 - smoothstep(0.0, 2.5 + 2.5 * masks.r, h_m)) * vertical * grime;
        detail_col = mix(detail_col, detail_col * vec3<f32>(0.5, 0.48, 0.4), clamp(foot * 0.9 * age, 0.0, 0.85));
        let streak = masks.g * vertical * grime * smoothstep(1.5, 5.0, h_m);
        detail_col = detail_col * (1.0 - clamp(streak * 0.45 * age, 0.0, 0.6));
        detail_col = detail_col * (1.0 - 0.22 * age * smoothstep(0.4, 0.85, masks.r) * grime);
        let up = smoothstep(0.25, 0.8, n_local.y);
        let moss = up * MOSS[layer] * smoothstep(0.5, 0.78, masks.b + 0.18 * (seed - 0.5)) * age;
        detail_col = mix(detail_col, vec3<f32>(0.11, 0.13, 0.05) * (0.8 + 0.4 * masks.r), clamp(moss * 0.55, 0.0, 0.7));
        base = detail_col;
        let r_layer = ROUGH[layer];
        rough = select(clamp(r_layer + (0.5 - d.g) * 0.2 + moss * 0.1, 0.05, 1.0), rough, r_layer < 0.0);
        // Whiteout blend of the three tangent-space normals, in the model's frame.
        let nx = normalize(vec3<f32>(nm.z * sign(n_local.x), nm.y, nm.x));
        let ny = normalize(vec3<f32>(nm.x, nm.z * sign(n_local.y), nm.y));
        let nz = normalize(vec3<f32>(nm.x, nm.y, nm.z * sign(n_local.z)));
        let bumped = normalize(nx * w.x + ny * w.y + nz * w.z);
        let n_mix = normalize(n_local + (bumped - vec3<f32>(sign(n_local.x) * w.x, sign(n_local.y) * w.y, sign(n_local.z) * w.z)) * 0.8);
        normal_world = normalize(rot_y(n_mix, yaw));
        if layer == 12 {
            pbr_input.material.reflectance = vec3<f32>(0.7);
        }
    } else {
        normal_world = normalize(rot_y(n_local, yaw));
    }
    if water {
        rough = 0.08;
        pbr_input.material.reflectance = vec3<f32>(0.6);
        pbr_input.material.metallic = 0.0;
    }
    pbr_input.N = normal_world;
    pbr_input.material.base_color = vec4<f32>(base, 1.0);
    pbr_input.material.perceptual_roughness = rough;
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
