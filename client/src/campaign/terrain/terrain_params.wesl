// Campaign terrain module `terrain_params` (VIS-L1, split of terrain.wesl): uniform blocks, material bindings and shared constants of the campaign ground.
// Pure move of the code, no change of behaviour.

import bevy_pbr::render::mesh_view_bindings::{fog};
import cent_ans_client::campaign::terrain::ground_detail::GdParams;

struct TerrainParams {
    // xyz: direction towards the sun.
    sun_dir: vec4<f32>,
    // rgb: sun colour x energy.
    sun_color: vec4<f32>,
    // rgb: ambient colour x energy.
    ambient: vec4<f32>,
    // rgb: distance fog colour, a: fog strength.
    fog: vec4<f32>,
    // x: vertical scale s, y: local gain g, z: squash weight c, w: edge haze width (px, 0 = off).
    scale: vec4<f32>,
    // xy: world size (map units), z: fog begin, w: 1 when the colormap is bound.
    map: vec4<f32>,
    // rgb: edge haze colour (linear), a: fog end.
    edge: vec4<f32>,
    // x: saturation, y: contrast, z: shadow saturation, w: strength of the grade (Godot Environment adjustment).
    grade: vec4<f32>,
    lift: vec4<f32>,
    highlights: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: TerrainParams;

@group(constants::MATERIAL_BIND_GROUP) @binding(1) var colormap: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(2) var colormap_sampler: sampler;

struct SeasonParams {
    // spring, summer, autumn, winter weights.
    weights: vec4<f32>,
    // per season: rgb gain, a saturation.
    gain: array<vec4<f32>, 4>,
    snow_lines: vec4<f32>,
    summer_gold: vec4<f32>,
    // rgb: winter ground tint, a: winter desaturation.
    winter_ground: vec4<f32>,
    // x, y: plain-snow south fade (fractions of the map height), z: plain snow share.
    snow_plain: vec4<f32>,
    // rgb: sunlit snow colour.
    snow_color: vec4<f32>,
    // TS-T1 (data `campaign_seasons.json` snow / ground_look): rgb shaded snow, a snow kept under forests.
    snow_shade: vec4<f32>,
    // x patch frequency (1 / map px), y snow-line jitter (m), z, w displayed-normal y start / full.
    snow_mask: vec4<f32>,
    // rgb mud, a fringe width.
    mud: vec4<f32>,
    // rgb dark rock, a strength.
    rock: vec4<f32>,
    // x, y displayed-normal y of full / no rock, z, w altitude start / full (m).
    rock_band: vec4<f32>,
    // x slope contrast, y crest light, z strata.
    rock_light: vec4<f32>,
    // rgb forest-floor tint, a strength.
    forest_floor: vec4<f32>,
    // x frequency (1 / map px), y strength, z sharpening.
    patches: vec4<f32>,
    // VIS-R4 (`mountain`): x, y tree line (m) south / north, z, w map-height fractions of the north-south fade.
    mt_line: vec4<f32>,
    // x slope forest start (m), y alpine meadow thickness (m), z scree transition (m), w sunny-face shift (m).
    mt_band: vec4<f32>,
    // x displayed-normal y under which the forest stops, y, z full / no scree, w pattern frequency (1 / map px).
    mt_slope: vec4<f32>,
    // x stage-limit jitter (m), y, z altitude (m) where the snow leaves the plain grey / is white.
    mt_edge: vec4<f32>,
    // rgb colour, a strength.
    mt_forest: vec4<f32>,
    mt_alpage: vec4<f32>,
    mt_scree: vec4<f32>,
    // rgb alpine snow in the sun / in the shade.
    mt_snow_sun: vec4<f32>,
    mt_snow_shade: vec4<f32>,
    // rgb: moorland colour of the northern alpine meadow.
    mt_moor: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> season: SeasonParams;

struct LookParams {
    // fog of war: desaturation, dim, veil strength, 1 when a mask is bound.
    fog_a: vec4<f32>,
    // veil colour (linear), jitter of the veil edge (map px).
    fog_veil: vec4<f32>,
    // mist colour (linear), mist max opacity.
    fog_mist: vec4<f32>,
    // mask extent (map px), mist min opacity, mist drift scale.
    fog_map: vec4<f32>,
    // wet ground, fresh snow, parchment weight.
    weather: vec4<f32>,
    // geomorph: 1 when on, quadtree vertex px, morph ratio.
    morph: vec4<f32>,
    // RV-C cumulus shadows: strength (0 off), scale, edge, drift / cover low, high, cloud height,
    // fade cell px / cool tint, fade cell px (none) / CPU-side tuning.
    cumulus_a: vec4<f32>,
    cumulus_b: vec4<f32>,
    cumulus_c: vec4<f32>,
    cumulus_d: vec4<f32>,
    // Morning mist: fog weight under the focus, morning share, wet dim, valley mist opacity.
    mist: vec4<f32>,
    // TB6 valley mist: depth (m, from, full), plain altitude (m, full, none) / crest height (m),
    // plain share, bank scale / drift, grazing gain, lifted share, near share / colour.
    valley_a: vec4<f32>,
    valley_b: vec4<f32>,
    valley_c: vec4<f32>,
    valley_color: vec4<f32>,
    // Aerial perspective: season fog colour (sRGB) / 1 when set; sky tint (sRGB) / share at the pitch.
    aerial_base: vec4<f32>,
    aerial_sky: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(4) var<uniform> look: LookParams;

@group(constants::MATERIAL_BIND_GROUP) @binding(5) var fog_mask: texture_2d<f32>;

// Regional ground (ground.rs, TX ADR 0243): 45 layers by (biome, role), biome blend, splat, grain.
struct GroundParams {
    // layer of (biome, role): index biome * 7 + role.
    layers: array<vec4<u32>, 27>,
    means: array<vec4<f32>, 48>,
    hues: array<vec4<f32>, 48>,
    micro_layers: array<vec4<u32>, 2>,
    micro_mean: array<vec4<f32>, 2>,
    // 1 when bound, blend width (texels), min blend weight, hue keep.
    bg: vec4<f32>,
    // tile screen px, base tile (map px), detail strength, detail normal strength.
    tiling: vec4<f32>,
    // micro on, micro tile (m), micro strength, metres per map px.
    micro: vec4<f32>,
    // detail fade footprints (xy), micro fade footprints (zw).
    fades: vec4<f32>,
    // splat bound, rock line (m).
    splat: vec4<f32>,
    // GA4 macro variation: periods (map px), octave weights, (tint amp, lum amp, -, TB6 canopy).
    macro_periods: vec4<f32>,
    macro_weights: vec4<f32>,
    macro_amp: vec4<f32>,
    // TB6 dark masses: floor, knee, feather (mip levels), kept contrast.
    dark: vec4<f32>,
    // RV-D valley occlusion: bound, valley darkening, crest lift, valley ambient occlusion.
    occlusion: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(7) var<uniform> ground: GroundParams;

@group(constants::MATERIAL_BIND_GROUP) @binding(8) var bg_albedo: texture_2d_array<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(9) var ground_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(10) var bg_normal: texture_2d_array<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(11) var ab_dist: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(13) var splat: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(15) var micro_albedo: texture_2d_array<f32>;

// Painted fields table (fields.rs): crops by biome / landscape, view and geo tuning.
struct FieldParams {
    rows_a: array<vec4<f32>, 24>,
    rows_b: array<vec4<f32>, 24>,
    slots: array<vec4<f32>, 288>,
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<f32>,
    v3: vec4<f32>,
    v4: vec4<f32>,
    v5: vec4<f32>,
    v6: vec4<f32>,
    v7: vec4<f32>,
    v8: vec4<f32>,
    vine: vec4<f32>,
    meadow: vec4<f32>,
    hedge: vec4<f32>,
    // per (row, slot): parcels material layer, tile (m), season class.
    slot_tex: array<vec4<f32>, 288>,
    // per row: canopy layer / tile, rock layer / tile.
    rows_c: array<vec4<f32>, 24>,
    // per row: the two wild materials (layer, tile).
    rows_d: array<vec4<f32>, 24>,
    // per row: material tint, 1 when the row has materials.
    rows_e: array<vec4<f32>, 24>,
    // vine layer / tile, meadow layer / tile.
    tex_extra: vec4<f32>,
    // season class of the vine, of the meadow; 1 when the parcels array is bound.
    tex_flags: vec4<f32>,
    tex_means: array<vec4<f32>, 48>,
    // VIS-R3b (parcel.rs): colormap share at the closest, typed forms on; VIS-L1 strip fade widths (px, zw).
    pc0: vec4<f32>,
    forms_a: array<vec4<f32>, 5>,
    forms_b: array<vec4<f32>, 5>,
    forms_c: array<vec4<f32>, 5>,
    forms_d: array<vec4<f32>, 5>,
    forms_e: array<vec4<f32>, 5>,
    shares: array<vec4<f32>, 40>,
    // VIS-L1 ground look: saturation gain, hue target (deg), hue pull, 1 when on.
    look_a: vec4<f32>,
    // VIS-L1 ground look: hue range start / end (deg), forest value, plain value.
    look_b: vec4<f32>,
    // VIS-L1 ground look: cold hue pull, its ramp (deg), ploughland gain, stubble gain.
    look_c: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(16) var<uniform> fields: FieldParams;

@group(constants::MATERIAL_BIND_GROUP) @binding(17) var height_lod: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(18) var wash_tex: texture_2d<f32>;

// Close patchwork (campvis3): parcels materials, land use, terroir mask.
@group(constants::MATERIAL_BIND_GROUP) @binding(20) var parcels_albedo: texture_2d_array<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(21) var parcels_normal: texture_2d_array<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(22) var landuse_tex: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(24) var terroir_tex: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(25) var terroir_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(26) var occlusion_tex: texture_2d<f32>;

// Coast band, glaciers, post adjustment, weather flags (coast.rs, material.rs `TerrainExtras`).
struct CoastParams {
    // 1 when bound, probe px, cliff min m, cliff max m.
    cliff: vec4<f32>,
    // cliff px, cliff widen, sea reach px, beach px.
    band_a: vec4<f32>,
    // wet px, strength, relief, far keep.
    band_b: vec4<f32>,
    // swash amount, period s, out px, in px / edge px, film px, cliff share.
    swash_a: vec4<f32>,
    swash_b: vec4<f32>,
    foam: vec4<f32>,
    // rock, chalk, granite, sand, shingle: rgb colour, a streak / rgb shade.
    color: array<vec4<f32>, 5>,
    shade: array<vec4<f32>, 5>,
    // coast_dist size (px), stored distance scale.
    texel: vec4<f32>,
};

struct GlacierParams {
    // low limit (m), full cover (m), max slope (m/m), strength.
    line: vec4<f32>,
    ice: vec4<f32>,
    crevasse: vec4<f32>,
    neve: vec4<f32>,
};

struct TerrainExtras {
    coast: CoastParams,
    glacier: GlacierParams,
    // Godot environment adjustment: contrast, saturation, on.
    post: vec4<f32>,
    // weather field bound and some weather, selection bound.
    wx: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(27) var<uniform> extras: TerrainExtras;

@group(constants::MATERIAL_BIND_GROUP) @binding(40) var<uniform> gd: GdParams;

@group(constants::MATERIAL_BIND_GROUP) @binding(28) var coast_dist_tex: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(30) var coast_probe_tex: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(31) var coast_types_tex: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(32) var wx_field_tex: texture_2d<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(34) var wx_clear_tex: texture_2d<f32>;

// Ground cache (ADR 0742): one layer per relief node, the regional ground baked once
// (rgb albedo, a forest share; rg normal offset), sampled with `terroir_sampler`.
// Relief shade (ADR 0745, `shade.rs`): march settings of the bake, baked sun visibility (r) and
// horizon occlusion (g) per node layer.
struct ShadeParams {
    // xyz: quantised direction towards the sun, w: 1 when baked.
    sun: vec4<f32>,
    // reach (map px), steps, penumbra (m), bias (m).
    march: vec4<f32>,
    // shadow strength, AO strength, AO reach (map px), displayed units per metre.
    mix: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(43) var<uniform> shade: ShadeParams;

@group(constants::MATERIAL_BIND_GROUP) @binding(44) var cache_shade: texture_2d_array<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(41) var cache_albedo: texture_2d_array<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(42) var cache_dn: texture_2d_array<f32>;

const VINE_LEAF = vec3<f32>(0.07, 0.10, 0.03);
