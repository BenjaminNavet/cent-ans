// Écorce des arbres de bataille (lot DA6) : texture Poly Haven `bark_brown_02` teintée par
// essence (chêne brun gris, hêtre gris lisse, saule…), normale, niveau de détail par instance.
shader_type spatial;
render_mode diffuse_burley;

#include "res://shaders/battle_tree_lod.gdshaderinc"

uniform sampler2D bark : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D bark_normal : hint_normal, filter_linear_mipmap, repeat_enable;
// TX T3 (ADR 0241) : écorce générée par essence (tableau de couches, `TreeTextures`) ; `bark_layer`
// -1 : texture d'avant. Micro-détail (grain fin) fondu entre `micro_fade.y` et `micro_fade.z` m.
uniform sampler2DArray bark_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray bark_normal_array : hint_normal, filter_linear_mipmap, repeat_enable;
uniform int bark_layer = -1;
uniform sampler2D micro_bark : filter_linear_mipmap, repeat_enable, hint_default_white;
uniform vec3 micro_fade = vec3(0.0, 3.0, 14.0);
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
	vec3 b;
	if (bark_layer >= 0) {
		b = texture(bark_array, vec3(UV, float(bark_layer)), smooth_bark * 3.0).rgb;
		NORMAL_MAP = texture(bark_normal_array, vec3(UV, float(bark_layer))).rgb;
	} else {
		b = texture(bark, UV, smooth_bark * 3.0).rgb;
		NORMAL_MAP = texture(bark_normal, UV).rgb;
	}
	if (micro_fade.x > 0.0) {
		float g = texture(micro_bark, UV * 3.7).r;
		b *= 1.0 + (g - 0.5) * 1.6 * micro_fade.x * (1.0 - smoothstep(micro_fade.y, micro_fade.z, length(VERTEX)));
	}
	float l = dot(b, vec3(0.3, 0.59, 0.11));
	ALBEDO = mix(b, vec3(l) * 1.25, smooth_bark * 0.7) * tint;
	NORMAL_MAP_DEPTH = mix(1.0, 0.3, smooth_bark);
	ROUGHNESS = 0.92;
}
