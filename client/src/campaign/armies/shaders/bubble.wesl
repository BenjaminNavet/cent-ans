// Bubble of the cells an army reaches (port of game/shaders/reachable_bubble.gdshader, lot
// armies6). `mask` holds one cell of the navigation grid per texel (R = reachable this turn,
// G = cost / budget, B = reachable by the end of the next turn). Each mask is softened by nine
// bilinear taps then thresholded at 0.5: rounded contour, golden rim of at least ~1.5 screen
// pixels, ink shadow outside, veil that strengthens towards the edge of the range; the next turn
// has a second, paler tone (veil barely tinted, thin faded rim, no shadow).
import bevy_pbr::render::mesh_functions::get_world_from_local;
import bevy_pbr::render::mesh_view_bindings::view;

struct Params {
    fill_color: vec4<f32>,
    rim_fill_color: vec4<f32>,
    edge_color: vec4<f32>,
    shadow_color: vec4<f32>,
    next_fill_color: vec4<f32>,
    next_edge_color: vec4<f32>,
    // xy origin of the mask in map pixels, zw its extent.
    frame: vec4<f32>,
    // xy texel size, z edge_px, w next_edge_px.
    texel: vec4<f32>,
    // x show_next, y depth_pull (fraction of the distance towards the camera).
    flags: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var mask: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var mask_sampler: sampler;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_xz: vec2<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let world_from_local = get_world_from_local(in.instance_index);
    let world = world_from_local * vec4<f32>(in.position, 1.0);
    // Pulled towards the camera so that the relief does not pierce the bubble.
    let seen = view.view_from_world * world;
    let pulled = vec4<f32>(seen.xyz * (1.0 - params.flags.y), 1.0);
    var out: VertexOutput;
    out.clip = view.clip_from_view * pulled;
    out.world_xz = world.xz;
    return out;
}

// Softened mask: `channel` picks the channel (R this turn, B next turn).
fn field(uv: vec2<f32>, channel: vec3<f32>) -> f32 {
    var s = 0.0;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let offset = vec2<f32>(f32(dx), f32(dy)) * params.texel.xy * 0.8;
            s = s + dot(textureSample(mask, mask_sampler, uv + offset).rgb, channel);
        }
    }
    return s / 9.0;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = (in.world_xz - params.frame.xy) / params.frame.zw;
    let f = field(uv, vec3<f32>(1.0, 0.0, 0.0));
    let f2 = field(uv, vec3<f32>(0.0, 0.0, 1.0));
    let cost = textureSample(mask, mask_sampler, uv).g;
    let edge_px = params.texel.z;
    let next_edge_px = params.texel.w;
    let d = f - 0.5;
    let px = max(fwidth(f), 1e-4);
    let inside = smoothstep(-px, px, d);
    // Rim centred on the threshold, constant width on screen.
    let line = 1.0 - smoothstep(px * edge_px * 0.5, px * edge_px, abs(d));
    // Ink shadow just outside the rim.
    let shadow = (1.0 - inside) * (1.0 - smoothstep(px * edge_px, px * edge_px * 3.5, -d));
    var color = vec4<f32>(0.0);
    if params.flags.x > 0.5 {
        let d2 = f2 - 0.5;
        let px2 = max(fwidth(f2), 1e-4);
        let inside2 = smoothstep(-px2, px2, d2) * (1.0 - inside);
        let line2 = (1.0 - smoothstep(px2 * next_edge_px * 0.5, px2 * next_edge_px, abs(d2)))
            * (1.0 - inside);
        color = vec4<f32>(params.next_fill_color.rgb, params.next_fill_color.a * inside2);
        color = mix(
            color,
            vec4<f32>(params.next_edge_color.rgb, params.next_edge_color.a),
            line2 * params.next_edge_color.a,
        );
    }
    let fill = mix(params.fill_color, params.rim_fill_color, smoothstep(0.55, 1.0, cost));
    // Outside both rings the tint stays the veil's (only the alpha fades).
    let tint_weight = select(1.0, inside, color.a > 0.0);
    color = vec4<f32>(mix(color.rgb, fill.rgb, tint_weight), mix(color.a, fill.a, inside));
    color = mix(color, vec4<f32>(params.shadow_color.rgb, params.shadow_color.a), shadow * params.shadow_color.a);
    color = mix(color, params.edge_color, line);
    let alpha = clamp(max(color.a, line * params.edge_color.a), 0.0, 1.0);
    return vec4<f32>(color.rgb, alpha);
}
