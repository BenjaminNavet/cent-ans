// Painted horizon panorama of a battle (port of game/shaders/battle_panorama.gdshader): a cylinder
// round the camera; each column of the painting is stretched so that its painted crest lands on
// the real skyline of the bearing (baked profile, exaggerated and bounded); above it the sky shows
// (fragment discarded). Sea sectors: flat sea horizon. Own haze and tint (fog disabled).
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::view,
};

struct PanoramaParams {
    // x: repeats, y: paint_offset, z: paint_crest, w: paint_detail.
    paint: vec4<f32>,
    // x: ref_y, y: radius, z: bottom_deg, w: has_painting.
    frame: vec4<f32>,
    // x: min_angle_deg, y: max_angle_deg, z: exaggeration, w: sea_threshold.
    crest: vec4<f32>,
    // x: haze, y: exposure, z: fog_density, w: unused.
    haze: vec4<f32>,
    fog_color: vec4<f32>,
    light_tint: vec4<f32>,
    sea_color: vec4<f32>,
    land_fallback: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> pano: PanoramaParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var painting: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var painting_sampler: sampler;
// Per world bearing (turns, 0 = north, clockwise): r skyline angle (deg), g distance (km),
// b sea share; a: painted crest of the painting column (0 = top of the band).
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var skyline: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var skyline_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var paint_skyline: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var paint_skyline_sampler: sampler;

const TAU: f32 = 6.283185307;

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let cam = view.world_position;
    let plan = in.world_position.xz - cam.xz;
    let bearing = fract(atan2(plan.x, -plan.y) / TAU + 1.0);
    let elevation = degrees(atan((in.world_position.y - pano.frame.x) / pano.frame.y));
    let real = textureSampleLevel(skyline, skyline_sampler, vec2<f32>(bearing, 0.5), 0.0);
    let crest = clamp(max(real.r * pano.crest.z, pano.crest.x), pano.crest.x, pano.crest.y);
    let sea = smoothstep(pano.crest.w - 0.15, pano.crest.w + 0.15, real.b)
        * (1.0 - smoothstep(0.15, 0.5, real.r));
    var u = pano.paint.y + bearing * pano.paint.x;
    u = abs(fract(u * 0.5) * 2.0 - 1.0);
    let column_crest = textureSampleLevel(paint_skyline, paint_skyline_sampler, vec2<f32>(u, 0.5), 0.0).r;
    let anchor = mix(column_crest, pano.paint.z, pano.paint.w);
    let bottom = pano.frame.z;
    let s = (elevation - bottom) / max(crest - bottom, 0.01);
    let v = 1.0 - s * (1.0 - anchor);
    var paint = vec4<f32>(pano.land_fallback.rgb, step(elevation, crest));
    if pano.frame.w > 0.5 {
        if v < 0.0 {
            paint = vec4<f32>(0.0);
        } else {
            paint = textureSampleLevel(painting, painting_sampler, vec2<f32>(u, clamp(v, 0.002, 0.998)), 0.0);
        }
    }
    let land = paint.rgb * pano.haze.y * pano.light_tint.rgb;
    let sea_alpha = step(elevation, -0.02);
    var col = mix(land, pano.sea_color.rgb * pano.light_tint.rgb, sea);
    let alpha = mix(paint.a, sea_alpha, sea);
    if alpha < 0.5 {
        discard;
    }
    let far = smoothstep(20.0, 120.0, real.g);
    let land_haze = clamp(pano.haze.x * mix(0.85, 1.12, far), 0.0, 1.0);
    let sea_haze = 1.0 - exp(-pano.haze.z * pano.frame.y);
    col = mix(col, pano.fog_color.rgb, mix(land_haze, sea_haze, sea));
    var out: FragmentOutput;
    out.color = vec4<f32>(col, 1.0);
    return out;
}
