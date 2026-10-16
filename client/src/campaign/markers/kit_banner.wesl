// Controller colour on the banner faces of the kit maquettes (port of
// game/shaders/maquette_kit.gdshader): the faces of the old `Banner` surface carry a vertex
// colour alpha of 0 (1 elsewhere); they take the colour of the place's controller (linear),
// every other face keeps its baked vertex colour. Lit by Bevy's PBR.
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing, visibility_range_dither},
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> kit_tint: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    // Crossfade of a visibility range (range fade of the maquettes): dithered discard.
@if(VISIBILITY_RANGE_DITHER)
    visibility_range_dither(in.position, in.visibility_range_dither);
    var pbr_input = pbr_input_from_standard_material(in, is_front);
@if(VERTEX_COLORS)
    if in.color.a < 0.5 {
        pbr_input.material.base_color = vec4<f32>(kit_tint.rgb, 1.0);
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
