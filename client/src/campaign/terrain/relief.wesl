// Shared exaggerated-relief module (port of game/shaders/campaign_relief.gdshaderinc, ADR 0353).
//
//     y = s * (h - K*max(h-base,0) + g*(1-K)*max(h-floor,0) + c*D),   K = c*k
//
// Import from any campaign material:
//     import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
//
// BINDING PATTERN (Bevy materials cannot share globals the way Godot's `global uniform` did, so each
// material binds the same two resources in its OWN bind group and passes them to these functions):
//   * a uniform `ReliefParams`  (Rust: `terrain::relief_gpu::ReliefParams`, ShaderType),
//   * the floor texture, Rgba32Float, non-filterable (Rust: `ReliefGpu::floor`, channels
//     R floor (m), G base (m), B squash k, A massif lift D (m)).
// Rust side of a material:
//     #[uniform(N)]  pub relief: ReliefParams,
//     #[texture(N+1, sample_type = "float", filterable = false)] pub relief_floor: Handle<Image>,
// and a system copying `Res<ReliefGpu>` into every such material each frame the `ReliefGpu` changed
// (`ReliefGpu::params` + `ReliefGpu::floor.clone()`). In WGSL:
//     @group(constants::MATERIAL_BIND_GROUP) @binding(N)   var<uniform> relief: ReliefParams;
//     @group(constants::MATERIAL_BIND_GROUP) @binding(N+1) var relief_floor: texture_2d<f32>;
//     let y = relief_display_height(relief_floor, relief, h_metres, world.xz);
// With `relief.scale.w == 0` (no floor yet) the texture is never read and y = s*h.

struct ReliefParams {
    // x: vertical scale s (world units per metre), y: local gain g, z: squash weight c,
    // w: 1 when the floor texture is valid.
    scale: vec4<f32>,
    // x: cell size (map px), y: centre offset of cell 0 (map px, (cell-1)/2).
    info: vec4<f32>,
};

fn relief_active(p: ReliefParams) -> bool {
    return p.scale.w > 0.5 && (p.scale.y != 0.0 || p.scale.z != 0.0);
}

// Fields (floor, base, k, D) at the world point xz: manual bilinear between cell centres, edges
// replicated. `grad_*` are their derivatives per world unit (x, z) of floor, base, k and D.
fn relief_fields_grad(
    floor_tex: texture_2d<f32>, p: ReliefParams, xz: vec2<f32>,
    grad_floor: ptr<function, vec2<f32>>, grad_base: ptr<function, vec2<f32>>,
    grad_k: ptr<function, vec2<f32>>, grad_lift: ptr<function, vec2<f32>>,
) -> vec4<f32> {
    *grad_floor = vec2<f32>(0.0);
    *grad_base = vec2<f32>(0.0);
    *grad_k = vec2<f32>(0.0);
    *grad_lift = vec2<f32>(0.0);
    if (!relief_active(p)) {
        return vec4<f32>(0.0);
    }
    let sz = vec2<i32>(textureDimensions(floor_tex, 0));
    let cell = max(p.info.x, 1.0);
    let f = clamp((xz - vec2<f32>(p.info.y)) / cell, vec2<f32>(0.0), vec2<f32>(sz - vec2<i32>(1)));
    let i = min(vec2<i32>(f), sz - vec2<i32>(2));
    let t = f - vec2<f32>(i);
    let a = textureLoad(floor_tex, i, 0);
    let b = textureLoad(floor_tex, i + vec2<i32>(1, 0), 0);
    let c = textureLoad(floor_tex, i + vec2<i32>(0, 1), 0);
    let d = textureLoad(floor_tex, i + vec2<i32>(1, 1), 0);
    let gx = mix(b - a, d - c, t.y) / cell;
    let gz = mix(c - a, d - b, t.x) / cell;
    *grad_floor = vec2<f32>(gx.x, gz.x);
    *grad_base = vec2<f32>(gx.y, gz.y);
    *grad_k = vec2<f32>(gx.z, gz.z);
    *grad_lift = vec2<f32>(gx.w, gz.w);
    return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
}

fn relief_fields(floor_tex: texture_2d<f32>, p: ReliefParams, xz: vec2<f32>) -> vec4<f32> {
    var g0 = vec2<f32>(0.0);
    var g1 = vec2<f32>(0.0);
    var g2 = vec2<f32>(0.0);
    var g3 = vec2<f32>(0.0);
    return relief_fields_grad(floor_tex, p, xz, &g0, &g1, &g2, &g3);
}

// Valley floor (m) at the world point xz.
fn relief_floor_m(floor_tex: texture_2d<f32>, p: ReliefParams, xz: vec2<f32>) -> f32 {
    return relief_fields(floor_tex, p, xz).x;
}

// Displayed height with the fields already read at the same point.
fn relief_display_height_at_fields(p: ReliefParams, h_m: f32, fields: vec4<f32>) -> f32 {
    if (!relief_active(p)) {
        return h_m * p.scale.x;
    }
    let k = p.scale.z * fields.z;
    return p.scale.x * (h_m - k * max(h_m - fields.y, 0.0)
        + p.scale.y * (1.0 - k) * max(h_m - fields.x, 0.0) + p.scale.z * fields.w);
}

// Displayed height (world units) of the altitude h_m (m) at the world point xz.
fn relief_display_height(floor_tex: texture_2d<f32>, p: ReliefParams, h_m: f32, xz: vec2<f32>) -> f32 {
    return relief_display_height_at_fields(p, h_m, relief_fields(floor_tex, p, xz));
}

// Gradient of the displayed surface (world units per world unit) from the altitude gradient
// `grad_m` (m per world unit): shading and cliff slope.
fn relief_display_gradient(
    floor_tex: texture_2d<f32>, p: ReliefParams, h_m: f32, grad_m: vec2<f32>, xz: vec2<f32>,
) -> vec2<f32> {
    if (!relief_active(p)) {
        return grad_m * p.scale.x;
    }
    var gf = vec2<f32>(0.0);
    var gb = vec2<f32>(0.0);
    var gk = vec2<f32>(0.0);
    var gl = vec2<f32>(0.0);
    let fields = relief_fields_grad(floor_tex, p, xz, &gf, &gb, &gk, &gl);
    let above_floor = step(fields.x, h_m);
    let above_base = step(fields.y, h_m);
    let k = p.scale.z * fields.z;
    let g = p.scale.y * (1.0 - k);
    let d = grad_m * (1.0 - k * above_base + g * above_floor)
        + above_base * (k * gb - p.scale.z * gk * (h_m - fields.y))
        - above_floor * (g * gf + p.scale.y * p.scale.z * gk * (h_m - fields.x))
        + p.scale.z * gl;
    return p.scale.x * d;
}
