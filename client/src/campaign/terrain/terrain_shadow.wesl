// Shadow caster of the campaign ground (prepass vertex shader of `TerrainMaterial`): the relief
// casts its shadow on itself and on the objects (Godot: the terrain is a lit mesh in the sun's
// shadow maps, `relief_shadow_cascades`). Same displayed height as `terrain.wesl` (`displayed_y`),
// without the geomorph (the light view has no camera distance) nor the skirts (the normal
// attribute is not bound in a depth-only pass). The uniform blocks repeat the ones of
// `terrain.wesl` (same bindings 0 and 4).

import bevy_pbr::{
    prepass::io::VertexOutput,
    render::{
        mesh_functions,
        view_transformations::position_world_to_clip,
    },
};

struct TerrainParams {
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    fog: vec4<f32>,
    scale: vec4<f32>,
    map: vec4<f32>,
    edge: vec4<f32>,
    grade: vec4<f32>,
    lift: vec4<f32>,
    highlights: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: TerrainParams;

struct ShadowVertex {
    @builtin(instance_index) instance_index: u32,
    // x, altitude (m), z
    @location(0) position: vec3<f32>,
    // valley floor (m), base (m)
    @location(1) fields_a: vec2<f32>,
    // squash factor k, massif lift D (m)
    @location(2) fields_b: vec2<f32>,
};

fn displayed_y(h: f32, fa: vec2<f32>, fb: vec2<f32>) -> f32 {
    let s = params.scale.x;
    let g = params.scale.y;
    let c = params.scale.z;
    let k = c * fb.x;
    return s * (h - k * max(h - fa.y, 0.0) + g * (1.0 - k) * max(h - fa.x, 0.0) + c * fb.y);
}

@vertex
fn vertex(v: ShadowVertex) -> VertexOutput {
    var out: VertexOutput;
    var y = displayed_y(v.position.y, v.fields_a, v.fields_b);
    let edge_px = params.scale.w;
    if (edge_px > 0.0) {
        let to_edge = min(v.position.xz, params.map.xy - v.position.xz);
        let sink = 1.0 - smoothstep(0.0, edge_px, min(to_edge.x, to_edge.y));
        y = mix(y, min(y, 0.2), sink);
    }
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position.x, y, v.position.z, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
}
    return out;
}
