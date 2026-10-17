// River ribbons and lake sheets. Port of river_water.gdshader (carved bed, fords, fine-zone fade). Mesh: centre line, NORMAL = signed perpendicular,
// UV0 = (world width, side +-1), UV1 = (distance downstream, importance 0-6). Lakes: `sheet` = 1.
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::get_world_from_local;
import cent_ans_client::campaign::terrain::relief::{ReliefParams, relief_display_height};
import bevy_pbr::render::mesh_view_bindings::{view, globals, lights, fog};
import bevy_pbr::render::mesh_view_types;
import bevy_pbr::render::fog as bevy_fog;
import cent_ans_client::campaign::weather::ground::wx_scatter;

struct Params {
    shallow_color: vec4<f32>,
    deep_color: vec4<f32>,
    sky_color: vec4<f32>,
    foam_color: vec4<f32>,
    bank_ink_color: vec4<f32>,
    importance_px_lo: vec4<f32>,
    importance_px_hi: vec4<f32>,
    detail: vec4<f32>,
    detail_mean: vec4<f32>,
    silt_color: vec4<f32>,
    far_color: vec4<f32>,
    mud_color: vec4<f32>,
    glint_color: vec4<f32>,
    look: vec4<f32>,
    look2: vec4<f32>,
    look3: vec4<f32>,
    look4: vec4<f32>,
    fine_zone: vec4<f32>,
    fords: array<vec4<f32>, 32>,
    map_size: vec2<f32>,
    distance_fade: vec2<f32>,
    ford_count: f32,
    min_px: f32,
    thin_fade: f32,
    far_alpha: f32,
    bank_ink_strength: f32,
    bank_ink_px: f32,
    edge_soft_px: f32,
    sky_reflect: f32,
    lift_base: f32,
    lift_per_distance: f32,
    flow_speed: f32,
    ripple_scale: f32,
    ripple_strength: f32,
    sheet: f32,
    sheet_alpha: f32,
    gen_width_scale: f32,
    gen_width_ref: f32,
    has_coast_dist: f32,
    coast_dist_scale: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var coast_dist: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var coast_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> relief: ReliefParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var relief_floor: texture_2d<f32>;
// RC5 detail (water_detail.gdshaderinc): normal, shared repeating sampler, albedo (alpha = foam).
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var detail_normal: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var detail_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) var detail_albedo: texture_2d<f32>;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv2: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) side: f32,
    @location(1) alpha: f32,
    @location(2) flow: vec2<f32>,
    @location(3) world: vec3<f32>,
    @location(4) far: f32,
    @location(5) carved: f32,
    @location(6) sw: f32,
    @location(7) ratio: f32,
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

// river_bed.gdshaderinc `fine_zone_mask`: 1 inside the fine hydrography disc, soft edge at 85-100 %.
fn fine_zone_mask(p: vec2<f32>) -> f32 {
    let z = params.fine_zone;
    if z.z <= 0.0 || z.w <= 0.0 {
        return 0.0;
    }
    return z.w * (1.0 - smoothstep(z.z * 0.85, z.z, distance(p, z.xy)));
}

fn importance_px_scale(importance: f32) -> f32 {
    let i = clamp(i32(importance + 0.5), 0, 6);
    if i < 4 {
        return params.importance_px_lo[i];
    }
    return params.importance_px_hi[i - 4];
}

fn gen_width(w: f32) -> f32 {
    if params.gen_width_scale > 0.0 {
        return max(w, params.gen_width_scale * sqrt(max(w, 0.0) * params.gen_width_ref));
    }
    return w;
}

// Carved bed (river_bed.gdshaderinc, river_water.gdshader). The Godot terrain carves the fine relief
// tiles listed in `carved_rects`; the Bevy terrain carves the close ground by camera distance
// (`river_carve` of terrain.wesl, same two radii), so the water follows the same weight: it sinks
// `WATER_SINK` under the bank and the ribbon overlaps the carved slope by `CARVED_OVERLAP`.
const WATER_SINK: f32 = 0.022;
const CARVED_OVERLAP: f32 = 1.35;
const CARVE_NEAR: vec2<f32> = vec2<f32>(220.0, 380.0);

fn camera_distance_fog(color: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    if fog.mode == mesh_view_types::FOG_MODE_OFF {
        return color;
    }
    let to_pixel = world - view.world_position;
    let distance = length(to_pixel);
    let c = vec4<f32>(color, 1.0);
    var none = vec3<f32>(0.0);
    if fog.directional_light_color.a > 0.0 && lights.n_directional_lights > 0u {
        let light = lights.directional_lights[0];
        none = wx_scatter(to_pixel, light.direction_to_light, light.color.rgb * view.exposure, 1.0, fog.directional_light_exponent);
    }
    if fog.mode == mesh_view_types::FOG_MODE_LINEAR {
        return bevy_fog::linear_fog(fog, c, distance, none).rgb;
    } else if fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL {
        return bevy_fog::exponential_fog(fog, c, distance, none).rgb;
    } else if fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL_SQUARED {
        return bevy_fog::exponential_squared_fog(fog, c, distance, none).rgb;
    }
    return color;
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_from_local = get_world_from_local(in.instance_index);
    // position.y is the altitude in metres: drape it on the exaggerated relief.
    let ground = relief_display_height(relief_floor, relief, in.position.y, in.position.xz);
    var world = world_from_local * vec4<f32>(in.position.x, ground, in.position.z, 1.0);
    let view_pos = (view.view_from_world * world).xyz;
    let dist = max(length(view_pos), 0.01);
    var out: VertexOutput;
    if params.sheet > 0.5 {
        world.y = world.y + params.lift_base + params.lift_per_distance * dist;
        out.alpha = 1.0;
        out.far = 1.0;
        out.side = 0.0;
        out.sw = 1.0;
        out.ratio = 1.0;
        out.carved = 0.0;
        out.flow = world.xz * vec2<f32>(0.35, 0.5);
    } else {
        let world_per_px = dist * 2.0 / (abs(view.clip_from_view[1][1]) * view.viewport.w);
        let gw = gen_width(in.uv.x);
        let true_px = gw / world_per_px;
        let imp_px = params.min_px * importance_px_scale(in.uv2.y);
        let carved = 1.0 - smoothstep(CARVE_NEAR.x, CARVE_NEAR.y, distance(world.xyz, view.world_position));
        let half_w = 0.5 * max(gw * mix(1.0, CARVED_OVERLAP, carved), imp_px * world_per_px);
        let dir = (world_from_local * vec4<f32>(in.normal, 0.0)).xyz;
        // VQ: wet ground strip outside the water, `look3.z` px wide (transparent dark falloff).
        let band_w = min(params.look3.z * world_per_px, 0.6 * gw + 0.5 * world_per_px * params.look3.z) * select(0.0, 1.0, params.look3.w > 0.0);
        let half_tot = half_w + band_w;
        world = vec4<f32>(world.xyz + dir * half_tot, 1.0);
        world.y = world.y + mix(params.lift_base + params.lift_per_distance * dist, -WATER_SINK, carved);
        out.carved = carved;
        out.alpha = mix(1.0, clamp(true_px / imp_px, 0.25, 1.0), params.thin_fade);
        out.far = 1.0;
        if params.distance_fade.y > 0.0 {
            out.far = mix(1.0, params.far_alpha, smoothstep(params.distance_fade.x, params.distance_fade.y, dist));
        }
        out.side = in.uv.y * half_tot / max(0.5 * gw, 1e-4);
        out.sw = half_w / max(0.5 * gw, 1e-4);
        out.ratio = half_tot / max(half_w, 1e-6);
        out.flow = vec2<f32>(in.uv2.x, in.uv.y * 0.5 * gw);
    }
    out.clip = view.clip_from_world * world;
    out.world = world.xyz;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let sheet = params.sheet > 0.5;
    let side_ext = abs(in.side);
    let edge = min(side_ext, in.sw);
    // 0 on the water, 0..1 across the wet strip beyond it.
    let band_t = clamp((side_ext / max(in.sw, 1e-4) - 1.0) / max(in.ratio - 1.0, 1e-4), 0.0, 1.0);
    let in_band = select(0.0, 1.0, side_ext > in.sw * 1.0005 && in.ratio > 1.0005);
    var deep = 1.0 - smoothstep(0.1, 1.0, edge);
    var lake_coast = -8.0;
    if sheet {
        deep = 0.75;
        if params.has_coast_dist > 0.5 {
            let uv = in.world.xz / params.map_size;
            lake_coast = (textureSampleLevel(coast_dist, coast_sampler, uv, 0.0).r * 255.0 - 128.0) / params.coast_dist_scale;
            deep = mix(0.12, 0.75, smoothstep(0.0, -7.0, lake_coast));
        }
    }
    let t = globals.time * params.flow_speed * params.ripple_scale * select(1.0, 0.25, sheet);
    let q = vec2<f32>(in.flow.x * 0.8, in.flow.y * 2.2) * params.ripple_scale;
    let n1 = vnoise(q + vec2<f32>(-t, 0.0));
    let n2 = vnoise(q * 1.9 + vec2<f32>(-t * 1.6, 3.7));
    let nx = vnoise(q + vec2<f32>(-t + 0.07, 0.0)) - n1;
    let ny = vnoise(q + vec2<f32>(-t, 0.07)) - n1;

    // Fords and ferries: clear shallow water, riffles and stones (only close up, like Godot).
    var ford = 0.0;
    var riffle = 0.0;
    var fords_here = 0;
    if !sheet && fwidth(in.flow.x) < 0.5 {
        fords_here = i32(params.ford_count);
    }
    for (var i = 0; i < 32; i = i + 1) {
        if i >= fords_here {
            break;
        }
        let f = params.fords[i];
        let d = distance(in.world.xz, f.xy);
        ford = max(ford, (1.0 - smoothstep(f.z * 0.4, f.z, d)) * f.w);
    }
    if ford > 0.0 {
        let stones = smoothstep(0.62, 0.7, vnoise(in.world.xz * 14.0)) * ford;
        riffle = smoothstep(0.55, 0.85, vnoise(vec2<f32>(in.flow.x * 9.0 - globals.time * 1.5, in.flow.y * 30.0))) * ford;
        deep = min(deep, 1.0 - 0.8 * ford);
        riffle = max(riffle, stones * 0.6);
    }

    var col = mix(params.shallow_color.rgb, params.deep_color.rgb, deep);
    col = col * (0.92 + 0.16 * n2);
    if sheet {
        let cur = vnoise(in.world.xz * 0.045) * 0.6 + vnoise(in.world.xz * 0.13 + 7.0) * 0.4;
        col = col * (0.8 + 0.4 * cur);
        col = mix(col, col * vec3<f32>(0.9, 1.08, 1.0), smoothstep(0.35, 0.7, vnoise(in.world.xz * 0.02 + 3.0)));
    }
    // RC5: detail coordinates follow the bed (downstream, across), scaled per unit.
    let d_on = select(0.0, 1.0, params.detail.x > 0.0);
    let d_strength = params.detail.x * d_on;
    let rc_uv = in.flow * params.detail.y;
    let ratio = textureSample(detail_albedo, detail_sampler, rc_uv).rgb / params.detail_mean.rgb;
    col = col * mix(vec3<f32>(1.0), clamp(ratio, vec3<f32>(0.5), vec3<f32>(1.6)), d_strength);
    // DN: silt covers the bed (denser in deep water), never total.
    col = mix(col, params.silt_color.rgb * (0.9 + 0.2 * n2), params.detail.w * mix(0.55, 1.0, deep));
    // DN: foam streaks along the bank, stretched along the current (albedo alpha).
    let streak = textureSample(detail_albedo, detail_sampler, rc_uv * vec2<f32>(0.6, 1.4) + vec2<f32>(-globals.time * params.detail.z * 1.5, 0.0)).a * d_on;
    // Foam sits a little inside the bank (never a bright outline on the edge itself).
    let foam_in = smoothstep(0.55, 0.8, edge) * (1.0 - smoothstep(0.88, 1.0, edge));
    var bank_foam = foam_in * (0.5 + 0.5 * n2) * (1.0 - in.carved) * 0.25;
    bank_foam = (bank_foam + foam_in * streak * 0.35 * (1.0 - in.carved * 0.5)) * params.look3.y;
    let foam = clamp(bank_foam + riffle, 0.0, 1.0);
    col = mix(col, params.foam_color.rgb, foam * 0.7);

    let half_px = select(1.0 / max(fwidth(in.side), 1e-4), 1000.0, sheet);
    let px_edge = max(1.0 - edge, 0.0) * half_px;
    let ink = (1.0 - smoothstep(0.0, params.bank_ink_px, px_edge)) * smoothstep(1.2, 3.0, half_px) * params.bank_ink_strength * (1.0 - in.carved);

    let fine_ripples = 1.0 - smoothstep(0.08, 0.45, fwidth(q.x));
    var n = normalize(vec3<f32>(-nx * params.ripple_strength * 10.0 * fine_ripples, 1.0, -ny * params.ripple_strength * 10.0 * fine_ripples));
    // RC5: detail normal drifting with the current, faded when the ripples get sub-pixel.
    let warp = vec2<f32>(sin(in.flow.y * 0.071 + 1.3), cos(in.flow.x * 0.057 + 0.4)) * 0.9;
    let flow_drift = vec2<f32>(-params.detail.z, 0.0) * globals.time;
    let duv1 = mat2x2<f32>(vec2<f32>(0.92, 0.39), vec2<f32>(-0.39, 0.92)) * in.flow * params.detail.y + warp;
    let duv2 = mat2x2<f32>(vec2<f32>(0.6, -0.8), vec2<f32>(0.8, 0.6)) * in.flow * params.detail.y * 0.37 + vec2<f32>(0.31, 0.57) - warp * 0.5;
    let dn1 = textureSample(detail_normal, detail_sampler, duv1 + flow_drift).xyz * 2.0 - 1.0;
    let dn2 = textureSample(detail_normal, detail_sampler, duv2 - flow_drift * 0.6).xyz * 2.0 - 1.0;
    let detail_raw = vec3<f32>(dn1.x + dn2.x, 0.0, -(dn1.y + dn2.y)) * 0.5 * d_strength;
    let detail_n = normalize(vec3<f32>(detail_raw.x, 1.0, detail_raw.z)) - vec3<f32>(0.0, 1.0, 0.0);
    n = normalize(n + detail_n * fine_ripples);
    let to_eye = normalize(view.world_position - in.world);
    let fresnel = pow(1.0 - clamp(dot(n, to_eye), 0.0, 1.0), 4.0);
    var sheet_sky = 0.0;
    if sheet {
        sheet_sky = 0.04 + 0.26 * vnoise(in.world.xz * 0.06 + globals.time * 0.02);
    }
    let eye_dist = length(view.world_position - in.world);
    let far_t = smoothstep(params.look2.x, max(params.look2.y, params.look2.x + 1.0), eye_dist);
    // VQ: muddy, lighter shallows along the banks (silt stirred up where the water is thin).
    let shallow = 1.0 - deep;
    col = mix(col, params.mud_color.rgb * (0.85 + 0.3 * n2), clamp(shallow * shallow * params.mud_color.w, 0.0, 1.0) * (1.0 - far_t));
    // Sky reflection: fresnel only (grazing), weaker with the water's own colour below.
    col = mix(col, params.sky_color.rgb, clamp(fresnel * params.sky_reflect + params.look.w + sheet_sky, 0.0, 0.85) * (1.0 - foam));
    // Sun glint (Blinn-Phong on the ripple normal), strong near, gone at distance.
    if lights.n_directional_lights > 0u && params.look.z > 0.0 {
        let to_sun = lights.directional_lights[0].direction_to_light;
        let h = normalize(to_sun + to_eye);
        let spec = pow(max(dot(n, h), 0.0), params.glint_color.w) * smoothstep(0.0, 0.12, to_sun.y);
        col = col + params.glint_color.rgb * spec * params.look.z * (1.0 - 0.8 * far_t) * (1.0 - foam);
    }
    // Campaign grade: water is not more saturated / brighter than the ground it lies on.
    let luma = dot(col, vec3<f32>(0.2126, 0.7152, 0.0722));
    col = (vec3<f32>(luma) + (col - vec3<f32>(luma)) * params.look.x) * params.look.y;
    // Far: sober blue-grey line, still readable (opacity is untouched).
    col = mix(col, params.far_color.rgb, far_t * select(1.0, 0.6, sheet));
    // Wet bank: the outer edge of the ribbon darkens like soaked ground.
    let wet = smoothstep(params.look2.z, 1.0, edge) * (1.0 - in.carved * 0.5);
    col = mix(col, params.bank_ink_color.rgb, clamp(ink + wet * params.bank_ink_strength, 0.0, 1.0));
    // VQ: dark wet banks (shade, soaked soil), no light outline.
    col = mix(col, params.look4.gba, clamp(smoothstep(params.look2.z, 1.0, edge) * params.look3.x, 0.0, 1.0));
    // Wet ground strip: same colour, fading out.
    col = mix(col, params.look4.gba, in_band);

    var alpha = 0.9 * in.alpha * in.far;
    if !sheet {
        // The water ends on the carved slope by itself (depth test): no soft border there.
        let soft = 1.0 - (1.0 - smoothstep(0.0, params.edge_soft_px, px_edge)) * (1.0 - in.carved);
        alpha = mix(0.9, 0.97, in.carved) * in.alpha * in.far * soft;
        alpha = alpha * (1.0 - ford * 0.35) * (1.0 - fine_zone_mask(in.world.xz));
    } else {
        alpha = params.sheet_alpha;
        if params.has_coast_dist > 0.5 {
            let shore_noise = vnoise(in.world.xz * 3.1) * 0.5 + vnoise(in.world.xz * 7.7) * 0.5;
            alpha = alpha * (1.0 - smoothstep(-1.6, 0.3 + 0.5 * (shore_noise - 0.5), lake_coast));
        }
    }
    if in_band > 0.5 {
        let wet_a = params.look3.w * (1.0 - smoothstep(0.0, 1.0, band_t)) * in.alpha * (1.0 - 0.6 * far_t);
        alpha = wet_a;
    }
    // Aerial haze: the same camera `DistanceFog` as the ground, so far water blends like it.
    col = mix(col, camera_distance_fog(col, in.world), params.look4.x);
    return vec4<f32>(col, clamp(alpha, 0.0, 1.0));
}
