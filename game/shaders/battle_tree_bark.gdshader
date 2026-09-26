// Écorce des arbres de bataille (lot DA6) : texture Poly Haven `bark_brown_02` teintée par
// essence (chêne brun gris, hêtre gris lisse, saule…), normale, niveau de détail par instance.
shader_type spatial;
render_mode diffuse_burley;

#include "res://shaders/battle_tree_lod.gdshaderinc"

uniform sampler2D bark : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D bark_normal : hint_normal, filter_linear_mipmap, repeat_enable;
uniform vec3 tint : source_color = vec3(0.8, 0.78, 0.72);
uniform float smooth_bark = 0.0; // hêtre : écorce lisse (texture adoucie, moins de relief)

void vertex() {
	v_lod = tree_lod(MODEL_MATRIX[3].xyz, MAIN_CAM_INV_VIEW_MATRIX[3].xyz);
	if (v_lod.x <= 0.0 || v_lod.y <= 0.0) {
		VERTEX = vec3(0.0, -1000.0, 0.0);
	}
}

void fragment() {
	if (tree_lod_discard(FRAGCOORD.xy, v_lod)) {
		discard;
	}
	vec3 b = texture(bark, UV, smooth_bark * 3.0).rgb;
	float l = dot(b, vec3(0.3, 0.59, 0.11));
	ALBEDO = mix(b, vec3(l) * 1.25, smooth_bark * 0.7) * tint;
	NORMAL_MAP = texture(bark_normal, UV).rgb;
	NORMAL_MAP_DEPTH = mix(1.0, 0.3, smooth_bark);
	ROUGHNESS = 0.92;
}
