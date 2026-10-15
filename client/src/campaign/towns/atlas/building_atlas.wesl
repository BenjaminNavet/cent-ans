// Shared material of the Blender kit buildings (port of game/shaders/building_atlas.gdshader,
// town_building.gdshader and building_aging.gdshaderinc, ADR 0021 / SR5 / TB3).
// Every matter of the kit (plaster, rubble, ashlar, timber, tiles, slate, thatch, glass...) is one
// layer of a 2D texture array; the layer of a face is coded in the alpha of the vertex colour
// ((index + 0.5) / 16), the rgb carrying building tint x baked occlusion x patina. Meshes without
// vertex colours (town blocks, walls, towers) use `style.x` as their layer and box UVs (metres).
// Lit by Bevy's PBR (sun, shadows, fog, IBL).
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::view,
    mesh_functions,
};

struct AtlasParams {
    // rgb: tint of the layer (linear), a: roughness.
    tint: array<vec4<f32>, 16>,
    // Repeats per metre of the 16 layers, packed by four.
    tile: array<vec4<f32>, 4>,
    // x: first plain layer, y: last plain layer, z: first roof layer, w: last roof layer.
    ranges: vec4<f32>,
    // x: aging (0 = none), y: snow on roofs (0-1), z: normal-map strength, w: 1 = box UVs.
    opts: vec4<f32>,
    // x: layer of meshes without vertex colours, y: soot (0-1), z: metres per world unit,
    // w: distance (world units) where the textures fade to their mean colour (0 = never).
    style: vec4<f32>,
    // rgb: colour multiplier (block tint), a: wall moss weight.
    color: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> atlas: AtlasParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var albedo_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var albedo_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var normal_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var normal_sampler: sampler;

fn ba_hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn ba_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = ba_hash21(i);
    let b = ba_hash21(i + vec2<f32>(1.0, 0.0));
    let c = ba_hash21(i + vec2<f32>(0.0, 1.0));
    let d = ba_hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// 0: fine detail visible; 1: a pixel covers >= 0.8 m (fine noise brought to its mean).
fn ba_far(p_m: vec3<f32>) -> f32 {
    let fw = fwidth(p_m);
    return smoothstep(0.15, 0.8, max(fw.x, max(fw.y, fw.z)));
}

// Aged colour (building_aging.gdshaderinc): mud at the foot of the walls, streaks under the
// sills, patches of grime, darker and mossy faces towards the sky.
fn ba_age(col_in: vec3<f32>, p: vec3<f32>, n: vec3<f32>, hm: f32, s: f32, g: f32, m: f32, a: f32, f: f32) -> vec3<f32> {
    var col = col_in;
    if a <= 0.0 {
        return col;
    }
    let vertical = 1.0 - abs(n.y);
    let t = normalize(vec2<f32>(-n.z, n.x) + vec2<f32>(1e-4, 0.0));
    let u = dot(p.xz, t) + s * 37.0;
    let h = max(hm, 0.0);
    let macro_n = ba_noise(vec2<f32>(u, p.y) * 0.16 + s * 13.0);
    let fine = mix(ba_noise(vec2<f32>(u * 5.0, h * 7.0)), 0.5, f);
    let foot_h = 0.5 + 0.9 * macro_n;
    let foot = (1.0 - smoothstep(0.0, foot_h, h - (fine - 0.5) * 0.35)) * vertical * g;
    col = mix(col, col * vec3<f32>(0.52, 0.47, 0.38), clamp(foot * 0.85 * a, 0.0, 0.8));
    let column = smoothstep(0.55, 0.8, ba_noise(vec2<f32>(u * 0.45, 3.1 + s * 5.0)));
    let thread = mix(ba_noise(vec2<f32>(u * 3.2, h * 0.35)), 0.5, f);
    let below = fract((1.0 + 0.3 * s - h) / 3.0) * 3.0;
    var streak = column * exp(-below * 0.7) * (0.45 + thread) * smoothstep(0.8, 2.0, h);
    streak = mix(streak, 0.12, f) * vertical * g;
    col = col * (1.0 - clamp(streak * 0.4 * a, 0.0, 0.45));
    col = col * (1.0 - 0.14 * a * smoothstep(0.45, 0.85, macro_n) * g);
    let up = smoothstep(0.25, 0.75, n.y);
    let blotch = mix(ba_noise(p.xz * 0.6 + s * 19.0), 0.5, f * 0.7);
    let moss = up * m * smoothstep(0.42, 0.78, blotch * 0.7 + macro_n * 0.3 + 0.12 * (s - 0.5)) * a;
    col = col * (1.0 - 0.12 * up * m * a);
    col = mix(col, vec3<f32>(0.10, 0.12, 0.05) * (0.8 + 0.4 * macro_n), clamp(moss * 0.5, 0.0, 0.6));
    return col;
}

fn layer_tile(layer: i32) -> f32 {
    return atlas.tile[layer / 4][layer % 4];
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    var vcol = vec3<f32>(1.0);
    var layer = i32(atlas.style.x + 0.5);
    @if(VERTEX_COLORS)
    {
        vcol = in.color.rgb;
        layer = clamp(i32(in.color.a * 16.0), 0, 15);
    }
    let mpu = atlas.style.z;
    let world_m = in.world_position.xyz * mpu;
    let n_world = normalize(in.world_normal);
    // Model frame (metres of the kit for the glb pieces) for the aging noise.
    let local_from_world = mesh_functions::get_local_from_world(in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);
    let local = (local_from_world * vec4<f32>(in.world_position.xyz, 1.0)).xyz;
    let origin = world_from_local[3].xyz;
    let seed = ba_hash21(floor(origin.xz * mpu * 7.0) + vec2<f32>(0.37, 0.37));

    var uv = vec2<f32>(0.0);
    @if(VERTEX_UVS_A)
    {
        uv = in.uv;
    }
    if atlas.opts.w > 0.5 {
        let an = abs(n_world);
        if an.y > max(an.x, an.z) {
            uv = world_m.xz;
        } else if an.x > an.z {
            uv = world_m.zy;
        } else {
            uv = world_m.xy;
        }
        uv.y = -uv.y;
    }

    var tint = atlas.tint[layer];
    var base = vec3<f32>(1.0);
    let textured = f32(layer) < atlas.ranges.x || f32(layer) > atlas.ranges.y;
    let roof = f32(layer) >= atlas.ranges.z && f32(layer) <= atlas.ranges.w;
    let tile = layer_tile(layer);
    let uvt = uv * tile;
    let dist = distance(in.world_position.xyz, view.world_position);
    // Sampled outside any branch (uniform control flow for the derivatives).
    let sampled = textureSample(albedo_array, albedo_sampler, uvt, layer).rgb;
    let mean = textureSampleLevel(albedo_array, albedo_sampler, vec2<f32>(0.5), layer, 12.0).rgb;
    let sampled_n = textureSample(normal_array, normal_sampler, uvt, layer).xy * 2.0 - 1.0;
    var normal_xy = vec2<f32>(0.0);
    if textured {
        base = sampled;
        if atlas.style.w > 0.0 {
            // Far away the texture is its mean (last mip): no shimmer on sub-pixel houses.
            base = mix(base, mean, smoothstep(atlas.style.w * 0.5, atlas.style.w, dist));
        }
        normal_xy = sampled_n * atlas.opts.z;
    }
    var albedo = base * tint.rgb * vcol * atlas.color.rgb;
    // Aging frame: the model frame (kit metres, foot at y = 0) for the glb pieces, world metres
    // around the instance origin for the box-UV meshes (unit cubes scaled to the block).
    let boxed = atlas.opts.w > 0.5;
    let age_pos = select(local, world_m - origin * mpu, boxed);
    let far = ba_far(age_pos);
    let hm = age_pos.y;
    let local_n = select(normalize((local_from_world * vec4<f32>(n_world, 0.0)).xyz), n_world, boxed);
    if atlas.opts.x > 0.0 && textured {
        let age = atlas.opts.x * (0.6 + 0.8 * seed);
        let moss_k = select(atlas.color.a, 1.0 - atlas.opts.y, roof);
        albedo = ba_age(albedo, age_pos, local_n, hm, seed, select(1.0, 0.0, roof), moss_k, age, far);
    }
    if atlas.opts.y > 0.0 && roof {
        albedo = mix(albedo, vec3<f32>(0.9, 0.92, 0.97) * (0.85 + 0.15 * base.r), atlas.opts.y);
    }
    var roughness = tint.a;
    let soot = atlas.style.y;
    if soot > 0.0 {
        // Soot patches (~16 m, ragged edge ~3 m): roofs brown-black, walls barely touched (TB3).
        let sn = ba_noise(world_m.xz * 0.06) * 0.65 + ba_noise(world_m.xz * 0.33) * 0.35;
        let patch = smoothstep(1.0 - soot - 0.12, 1.0 - soot + 0.12, sn);
        let k = patch * select(0.15, 0.6, roof);
        albedo = mix(albedo, vec3<f32>(0.11, 0.075, 0.055), k);
        roughness = mix(roughness, 1.0, k);
    }
    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);
    pbr_input.material.perceptual_roughness = roughness;
    pbr_input.material.metallic = 0.0;

    // Cotangent frame from the screen derivatives (the kit has no tangents).
    let dp1 = dpdx(in.world_position.xyz);
    let dp2 = dpdy(in.world_position.xyz);
    let duv1 = dpdx(uvt);
    let duv2 = dpdy(uvt);
    let fade = 1.0 - smoothstep(40.0 / max(mpu, 1e-4), 120.0 / max(mpu, 1e-4), dist);
    if textured && atlas.opts.z > 0.0 && fade > 0.001 {
        let nrm = pbr_input.N;
        let dp2perp = cross(dp2, nrm);
        let dp1perp = cross(nrm, dp1);
        let t = dp2perp * duv1.x + dp1perp * duv2.x;
        let b = dp2perp * duv1.y + dp1perp * duv2.y;
        let invmax = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-20));
        // Godot normal maps are OpenGL style (+Y up) while the V axis of the UVs points down.
        let nxy = normal_xy * fade;
        pbr_input.N = normalize(nrm + (t * invmax) * nxy.x - (b * invmax) * nxy.y);
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
