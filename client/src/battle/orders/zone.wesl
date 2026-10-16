// Deployment zone of the player (port of game/shaders/deployment_zone.gdshader): a bright edge
// line, an inner glow that fades inwards, diagonal hatching in a border band and an almost
// invisible veil in the middle. Hatching and glow fade as the camera moves away; the line keeps
// a minimum width on screen so it stays readable from afar.
import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::view,
};

struct ZoneParams {
    // xy: rect_min, zw: rect_max (world x, z).
    rect: vec4<f32>,
    edge_color: vec4<f32>,
    // x: line_width, y: glow_width, z: hatch_band, w: hatch_period (m).
    shape: vec4<f32>,
    // x: interior_alpha, y: fade_near, z: fade_far.
    fade: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> zone: ZoneParams;

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let world_xz = in.world_position.xz;
    let rect_min = zone.rect.xy;
    let rect_max = zone.rect.zw;
    let inner = min(world_xz - rect_min, rect_max - world_xz);
    let d = max(min(inner.x, inner.y), 0.0);
    let cam_dist = length(in.world_position.xyz - view.world_position);
    let detail = 1.0 - smoothstep(zone.fade.y, zone.fade.z, cam_dist);

    // Edge line: at least ~1.6 px on screen at any distance.
    let px = max(fwidth(d), 1e-4);
    let w = max(zone.shape.x, px * 1.6);
    let line = 1.0 - smoothstep(w - px, w + px, d);

    let glow = exp(-d / zone.shape.y) * mix(0.18, 0.42, detail);

    // Anti-aliased diagonal hatching, limited to the border band.
    let s = (world_xz.x + world_xz.y) / zone.shape.w;
    let sw = max(fwidth(s), 1e-4);
    let f = abs(fract(s) - 0.5);
    let stripe = 1.0 - smoothstep(0.08, 0.08 + sw * 1.5, f);
    // Hatching thinner than a pixel: fade it out (anti-moire).
    let moire = 1.0 - smoothstep(0.25, 0.5, sw);
    let band = 1.0 - smoothstep(zone.shape.z * 0.35, zone.shape.z, d);
    let hatch = stripe * band * 0.28 * detail * moire;

    let alpha = max(max(line * 0.95, glow), max(hatch, zone.fade.x));
    var out: FragmentOutput;
    out.color = vec4<f32>(
        zone.edge_color.rgb * (1.0 + line * 0.6),
        clamp(alpha, 0.0, 1.0) * zone.edge_color.a,
    );
    return out;
}
