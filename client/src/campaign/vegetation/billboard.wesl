// Campaign vegetation drawn with one instanced draw per batch (port of
// `campaign_tree_impostor.gdshader(inc)`, `foliage_common.gdshaderinc` seasonal part,
// `campaign_wind.gdshaderinc` and `freshwater_reeds.gdshader`). Lit with Bevy's PBR code.
//
// Instance record (40 bytes): (x, y, z, yaw_deg) (scale, model-zone weight, seed) (r, g, b).
// Modes (`params.frame.w`): 0 tree impostor (camera-facing card, baked view chosen by azimuth and
// dithered between the two nearest, seasonal tint, wind), 1 reed clump (two crossed cards),
// 2 pond (flat disc), 3 near tree model (normalised glb mesh, `foliage_model.gdshader`: texture
// saturated by `model_chroma`, gain, seasonal tint, wind). Cast shadows: `shadow_cast.wesl`.
import bevy_pbr::render::{
    pbr_functions,
    pbr_types,
    mesh_view_bindings::{view, globals, fog, lights},
    mesh_view_types,
    fog as bevy_fog,
    view_transformations::position_world_to_clip,
    shadows,
};

struct Params {
    season: vec4<f32>,  // spring, summer, autumn, winter weights
    tint: vec4<f32>,    // rgb multiplier, w: season class (0 oak, 1 beech, 2 conifer, 3 hedge, 4 gold)
    atlas: vec4<f32>,   // views, rows, row, width / height ratio
    frame: vec4<f32>,   // ortho, foot, alpha cutoff, mode
    wind: vec4<f32>,    // bend, calm, light, gain
    extra: vec4<f32>,   // reeds: width, height (world units), sway, far scale
    range: vec4<f32>,   // reeds, ponds: scale near distance, scale far distance, pond far scale, opacity (dither)
    // Ground lighting of the terrain material (trees lit and fogged like the ground).
    sun_dir: vec4<f32>,   // direction to the sun, w: 1 when set (else Bevy PBR lighting)
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    fog: vec4<f32>,       // distance fog colour, strength
    fog_range: vec4<f32>, // fog begin, end
    grade: vec4<f32>,      // saturation, contrast, shadow saturation, strength (terrain grade)
    lift: vec4<f32>,
    highlights: vec4<f32>,
    // Forest look (`look.rs`): value, saturation, tree / stand value spreads; rgb balance, crown
    // shade; stand period, height spread, height period, frost green share; far growth start,
    // end, gain, bare openwork; frost rgb, strength; frost altitudes; bare rgb, look set.
    canopy: vec4<f32>,
    balance: vec4<f32>,
    shape: vec4<f32>,
    far: vec4<f32>,
    frost: vec4<f32>,
    winter: vec4<f32>,
    bare: vec4<f32>,
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
    @location(8) misc: vec4<f32>, // blend, seed, ground y, model-zone weight
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

const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

// Stand and tree value variety (TS-T2): a low-frequency stand noise and a per-tree draw, the darker
// stands leaning blue-green, the lighter ones warmer.
fn canopy_tone(p: vec2<f32>, seed: f32) -> vec3<f32> {
    let stand = value_noise(p / max(params.shape.x, 1.0));
    let tree = fract(seed * 71.37);
    let v = (1.0 + params.canopy.w * (2.0 * stand - 1.0)) * (1.0 + params.canopy.z * (2.0 * tree - 1.0));
    return mix(vec3<f32>(0.94, 1.0, 1.03), vec3<f32>(1.04, 1.02, 0.92), stand) * v;
}

// Stand height variety and growth with the camera distance (a far forest stays a mass of trees).
fn canopy_size(p: vec2<f32>, origin: vec3<f32>) -> f32 {
    let h = 1.0 + params.shape.y * (2.0 * value_noise(p / max(params.shape.z, 1.0) + vec2<f32>(7.3, 1.9)) - 1.0);
    let d = distance(view.world_position, origin);
    return h * (1.0 + params.far.z * smoothstep(params.far.x, max(params.far.y, params.far.x + 1.0), d));
}

// Darker, greener canopy (less uniform light olive): saturation, balance and value.
fn canopy_grade(c: vec3<f32>, top: f32) -> vec3<f32> {
    let l = dot(c, LUMA);
    let s = vec3<f32>(l) + (c - vec3<f32>(l)) * params.canopy.y;
    let shade = mix(1.0 - params.balance.w, 1.0, smoothstep(0.0, 0.8, top));
    return max(s * params.balance.rgb * params.canopy.x * shade, vec3<f32>(0.0));
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
    out.misc = vec4<f32>(0.0, seed, origin.y, in.scale_seed.y);
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
        // Stand variety: tone and height by low-frequency noises on the position, jitter per tree.
        let jitter = 1.0 + 0.25 * (2.0 * fract(seed * 53.17) - 1.0);
        let tone = canopy_tone(origin.xz, seed);
        let height = in.scale_seed.x * jitter * canopy_size(origin.xz, origin);
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
        // Per-instance tint (brightness and stand tint of the scatter) like the near models: without
        // it the impostors lost the stand's darker, olive tone and read lighter than the models.
        out.tint = params.tint.rgb * in.tint * tone * (1.0 + params.wind.z * gust);
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
    } else if mode > 2.5 {
        // Tree model: mesh of height 1 (foot at 0), crown width already set by the normalisation.
        let jitter = 1.0 + 0.25 * (2.0 * fract(seed * 53.17) - 1.0);
        let size = in.scale_seed.x * jitter * canopy_size(origin.xz, origin);
        let p = in.position * size;
        world = rotate_y(p, c, s) + origin;
        let gust = wind_gust(origin.xz);
        let flex = in.position.y * in.position.y * size;
        let shiver = sin(globals.time * 1.7 + seed * 23.0) * (0.4 + 0.6 * gust);
        world = world + vec3<f32>(WIND_DIR.x, 0.0, WIND_DIR.y) * flex
            * (params.wind.x * gust + params.wind.y * shiver);
        out.face = rotate_y(in.normal, c, s);
        out.uv0 = in.uv;
        out.local = vec2<f32>(in.position.x + in.position.z, in.position.y) * 6.0;
        out.tint = params.tint.rgb * in.tint * canopy_tone(origin.xz, seed) * (1.0 + params.wind.z * gust);
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
fn foliage_season(albedo: vec3<f32>, cls: f32, seed: f32, leaves: f32, up: f32, world_y: f32, top: f32) -> vec3<f32> {
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
    // Winter (TS-T2): bare broadleaves grey-brown (never black dots), conifers frosted on the top
    // of the crown, more with the altitude, a share of them staying green in the lowlands.
    let bare = params.bare.rgb;
    let alt = smoothstep(params.winter.x, max(params.winter.y, params.winter.x + 0.01), world_y);
    var winter: vec3<f32>;
    if broadleaf {
        winter = mix(bare * mix(0.8, 1.2, leaves), params.frost.rgb, 0.45 * alt * smoothstep(0.4, 0.9, top));
    } else if hedge {
        winter = mix(albedo, bare, 0.7);
    } else {
        let cover = smoothstep(0.15, 0.75, 0.55 * up + 0.6 * top + (leaves - 0.5) * 0.6 + 0.25 * alt);
        var frost = params.frost.w * cover * mix(0.55, 1.0, alt);
        if fract(seed * 41.7) < params.shape.w * (1.0 - alt) {
            frost = frost * 0.2;
        }
        winter = mix(albedo * 0.85, params.frost.rgb, frost);
    }
    return spring * w.x + albedo * w.y + autumn * w.z + winter * w.w;
}

// Camera haze (`DistanceFog`), as `camera_distance_fog` of the terrain shader.
fn camera_fog(color: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    if (fog.mode == mesh_view_types::FOG_MODE_OFF) {
        return color;
    }
    let distance = length(world - view.world_position);
    let c = vec4<f32>(color, 1.0);
    let none = vec3<f32>(0.0);
    if (fog.mode == mesh_view_types::FOG_MODE_LINEAR) {
        return bevy_fog::linear_fog(fog, c, distance, none).rgb;
    } else if (fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL) {
        return bevy_fog::exponential_fog(fog, c, distance, none).rgb;
    } else if (fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL_SQUARED) {
        return bevy_fog::exponential_squared_fog(fog, c, distance, none).rgb;
    }
    return color;
}

// Foliage lit like the ground (`terrain.wesl`: ambient + sun x N.L, then its distance fog and the
// camera haze), with the Godot foliage terms: wrapped Lambert (`diffuse_lambert_wrap`), light
// seen through the leaves against the sun (`BACKLIGHT`), ambient occlusion on the sky light.
// Visibility of the sun (cascaded shadow map of the `CampaignSun`, as `sun_shadow` of the
// terrain material): 1 lit, 0 in the shadow of the relief or of a town. The normal handed to the
// bias is the sun direction, so that a camera-facing card never shadows itself.
fn foliage_sun_shadow(world: vec3<f32>, l: vec3<f32>, frag_xy: vec2<f32>) -> f32 {
    if lights.n_directional_lights == 0u {
        return 1.0;
    }
    if (lights.directional_lights[0].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) == 0u {
        return 1.0;
    }
    let wp = vec4<f32>(world, 1.0);
    let view_z = dot(vec4<f32>(view.view_from_world[0].z, view.view_from_world[1].z,
        view.view_from_world[2].z, view.view_from_world[3].z), wp);
    return shadows::fetch_directional_shadow(0u, wp, l, view_z, frag_xy);
}

// `campaign_grade` of `terrain.wesl`.
fn foliage_grade(c: vec3<f32>) -> vec3<f32> {
    if (params.grade.w <= 0.0) {
        return c;
    }
    var col = (c - vec3<f32>(0.5)) * params.grade.y + vec3<f32>(0.5);
    col = max(col, vec3<f32>(0.0));
    let luma = dot(col, vec3<f32>(0.2126, 0.7152, 0.0722));
    col = col + params.lift.rgb * (1.0 - smoothstep(0.0, 0.5, luma));
    col = col * mix(vec3<f32>(1.0), params.highlights.rgb, smoothstep(0.3, 0.9, luma));
    let sat = mix(params.grade.z, params.grade.x, smoothstep(0.02, 0.35, luma));
    col = vec3<f32>(luma) + (col - vec3<f32>(luma)) * sat;
    return mix(c, max(col, vec3<f32>(0.0)), params.grade.w);
}

fn shade_foliage(albedo: vec3<f32>, n: vec3<f32>, occlusion: f32, backlight: f32, world: vec3<f32>, frag_xy: vec2<f32>) -> vec3<f32> {
    let l = normalize(params.sun_dir.xyz);
    // Cast shadow of the relief / towns on the sun term only (Godot lights the foliage in
    // `light()`, where ATTENUATION carries the shadow).
    let shadow = foliage_sun_shadow(world, l, frag_xy);
    let ndl = dot(n, l);
    let wrap = max((ndl + 0.5) / 1.5, 0.0);
    let v = normalize(view.world_position - world);
    let through = max(-ndl, 0.0) * smoothstep(-0.2, 0.6, dot(-v, l));
    // AO: full on the sky light, 0.4 on the sun (`AO_LIGHT_AFFECT`).
    // Exposure (ambient.a) and grade of the ground: Godot tonemapped and graded the whole frame,
    // so the trees read as pale blobs on the darker graded ground without them (showfix, 16/10).
    var color = albedo * (params.ambient.rgb * occlusion
        + params.sun_color.rgb * (wrap + backlight * through) * mix(1.0, occlusion, 0.4) * shadow)
        * select(1.0, params.ambient.a, params.ambient.a > 0.0);
    color = foliage_grade(color);
    let d = distance(world, view.world_position);
    let begin = params.fog_range.x;
    let t = clamp((d - begin) / max(params.fog_range.y - begin, 1.0), 0.0, 1.0);
    color = mix(color, params.fog.rgb, pow(t, 1.6) * params.fog.a);
    return camera_fog(color, world);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let mode = params.frame.w;
    var color = vec4<f32>(1.0);
    var normal = vec3<f32>(0.0, 1.0, 0.0);
    var occlusion = 1.0;
    var roughness = 0.9;
    // Bounded mip level (`max_lod` of `campaign_tree_impostor.gdshaderinc`): far away, the mean
    // alpha of a whole cell would cut the crown into scattered dots (or fill the card); the
    // alpha lost by the filtering is compensated below. Derivatives taken before any discard.
    let texels = vec2<f32>(textureDimensions(albedo_texture, 0));
    let duv_x = dpdx(in.uv0 * texels);
    let duv_y = dpdy(in.uv0 * texels);
    let lod = clamp(0.5 * log2(max(max(dot(duv_x, duv_x), dot(duv_y, duv_y)), 1e-8)), 0.0, MAX_LOD);
    // Dithered model / impostor band (ZF-C): complementary Bayer shares of the same tree.
    let zone = in.misc.w;
    if mode < 0.5 && zone > 0.0 && bayer4(in.position.xy) < zone {
        discard;
    }
    if mode > 2.5 && mode < 3.5 && bayer4(in.position.xy) >= zone {
        discard;
    }
    if mode < 0.5 {
        // The two nearest baked views blended (TS-T2: a Bayer pick between them showed a 4x4
        // pattern on every crown); colour weighted by the alpha of each view.
        let b = in.misc.x;
        let t0 = textureSampleLevel(albedo_texture, albedo_sampler, in.uv0, lod);
        let t1 = textureSampleLevel(albedo_texture, albedo_sampler, in.uv1, lod);
        let n0 = textureSampleLevel(normal_texture, albedo_sampler, in.uv0, lod);
        let n1 = textureSampleLevel(normal_texture, albedo_sampler, in.uv1, lod);
        let a = mix(t0.a, t1.a, b);
        let wa = t0.a * (1.0 - b);
        let wb = t1.a * b;
        let t = vec4<f32>((t0.rgb * wa + t1.rgb * wb) / max(wa + wb, 1e-4), a);
        let nt = (n0 * wa + n1 * wb) / max(wa + wb, 1e-4);
        let seed = in.misc.y;
        let cls = params.tint.w;
        let leaves = value_noise(in.local * 24.0 + vec2<f32>(seed * 17.0, seed * 5.0));
        // Winter: broadleaf crowns are openwork near by (far away the holes would stipple).
        if season_deciduous(cls) && params.season.w > 0.5 && lod < 1.5
            && value_noise(in.local * 16.0 + vec2<f32>(seed * 29.0, seed * 3.0)) < params.far.w {
            discard;
        }
        if t.a * (1.0 + lod * 0.35) < params.frame.z {
            discard;
        }
        let n = nt.rgb * 2.0 - 1.0;
        let baked = normalize(in.right * n.x + in.up * n.y + in.face * max(n.z, 0.05));
        // Slightly lifted towards the sky (Godot adds the sky's own fill light to the baked normal).
        normal = normalize(mix(baked, vec3<f32>(0.0, 1.0, 0.0), 0.2));
        var albedo = t.rgb * in.tint * mix(0.85, 1.12, leaves) * params.wind.w;
        let lum = max(dot(albedo, vec3<f32>(0.2126, 0.7152, 0.0722)), 1e-3);
        albedo = albedo * max(1.0, 0.045 / lum);
        let top = smoothstep(params.frame.y, 1.0, in.local.y);
        albedo = canopy_grade(albedo, top);
        color = vec4<f32>(foliage_season(albedo, cls, seed, leaves, baked.y, in.misc.z, top), 1.0);
        occlusion = mix(0.55, 1.0, nt.a);
    } else if mode < 1.5 {
        let t = textureSample(albedo_texture, albedo_sampler, in.uv0);
        if t.a < params.frame.z || hash12(in.position.xy) > params.range.w {
            discard;
        }
        color = vec4<f32>(t.rgb * in.tint * 1.6, 1.0);
        roughness = 0.85;
    } else if mode > 2.5 {
        let t = textureSample(albedo_texture, albedo_sampler, in.uv0);
        let seed = in.misc.y;
        let cls = params.tint.w;
        let leaves = value_noise(in.local) * 0.6 + value_noise(in.local * 2.3) * 0.4;
        let luma = dot(t.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let rgb = max(vec3<f32>(luma) + (t.rgb - vec3<f32>(luma)) * params.extra.x, vec3<f32>(0.0));
        let top = clamp(in.local.y / 6.0, 0.0, 1.0);
        let albedo = canopy_grade(rgb * params.extra.yzw * in.tint * mix(0.86, 1.1, leaves), top);
        normal = normalize(in.face);
        if season_deciduous(cls) && params.season.w > 0.5 && value_noise(in.local * 2.4) < params.far.w + 0.05 {
            discard;
        }
        color = vec4<f32>(foliage_season(albedo, cls, seed, leaves, normal.y, in.misc.z, top), 1.0);
    } else {
        let r = length(in.uv0 - vec2<f32>(0.5)) * 2.0;
        if r > 1.0 || hash12(in.position.xy) > smoothstep(1.0, 0.7, r) * params.range.w {
            discard;
        }
        color = vec4<f32>(in.tint, 1.0);
        roughness = 0.12;
    }

    if mode < 1.5 || mode > 2.5 {
        if params.sun_dir.w > 0.5 {
            // Backlight share (Godot: deciduous 0.35 -> 0.1 in winter, others 0.35).
            var back = 0.35;
            if (mode < 0.5 || mode > 2.5) && season_deciduous(params.tint.w) {
                back = mix(0.35, 0.1, params.season.w);
            }
            return vec4<f32>(shade_foliage(color.rgb, normal, occlusion, back, in.world_position, in.position.xy), 1.0);
        }
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
    var post = pbr_functions::main_pass_post_lighting_processing(pbr_input, lit);
    post.a = 1.0;
    return post;
}
