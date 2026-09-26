// Feuillage des arbres de bataille (lot DA6) : rameaux alpha (`leaf_spray`, ramilles nues ou
// feuilles sèches l'hiver) posés au bout des branches du squelette procédural (`BattleTrees`).
// Couleur = carte × teinte de l'essence × couleur de sommet (ombre propre du houppier, cuite) ×
// teinte d'instance (saison, variation) ; désaturée vers la luminance (bible DA § 3.3).
// Balancement au vent croissant avec la hauteur, frémissement, lumière transmise à contre-jour.
// Niveau de détail par instance (`battle_tree_lod.gdshaderinc`).
shader_type spatial;
render_mode cull_disabled, diffuse_burley, depth_draw_opaque;

#include "res://shaders/battle_tree_lod.gdshaderinc"

uniform sampler2D leaves : source_color, filter_linear_mipmap, repeat_disable;
uniform float alpha_cut = 0.5;
uniform float wind_strength = 1.0;
uniform float sway_height = 12.0; // hauteur (m) où le balancement est plein
uniform vec3 tint : source_color = vec3(1.0);
uniform float decor_saturation = 0.66;
uniform vec3 backlight = vec3(0.3, 0.34, 0.14);

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	v_lod = tree_lod(origin, MAIN_CAM_INV_VIEW_MATRIX[3].xyz);
	if (v_lod.x <= 0.0 || v_lod.y <= 0.0) {
		VERTEX = vec3(0.0, -1000.0, 0.0);
	} else {
		float h = clamp(VERTEX.y / sway_height, 0.0, 1.0);
		float t = TIME * 1.15 + origin.x * 0.061 + origin.z * 0.047;
		float sway = (sin(t) * 0.6 + sin(t * 2.3 + 1.7) * 0.25) * h * h * wind_strength;
		VERTEX.x += sway * 0.35;
		VERTEX.z += sway * 0.22;
		VERTEX += NORMAL * sin(TIME * 4.0 + VERTEX.x * 3.1 + VERTEX.y * 2.3) * 0.04 * wind_strength;
	}
}

void fragment() {
	if (tree_lod_discard(FRAGCOORD.xy, v_lod)) {
		discard;
	}
	// Les deux faces d'une carte gardent la normale arrondie du houppier.
	NORMAL = FRONT_FACING ? NORMAL : -NORMAL;
	vec4 tex = texture(leaves, UV);
	vec3 col = tex.rgb * tint * COLOR.rgb;
	ALBEDO = mix(vec3(dot(col, vec3(0.3, 0.59, 0.11))), col, decor_saturation);
	// Couverture gardée au loin : dans les mipmaps l'alpha moyen des feuilles baisse (houppiers
	// qui se vident) ; relevé selon le niveau lu.
	float lod = max(textureQueryLod(leaves, UV).x, 0.0);
	ALPHA = tex.a * (1.0 + lod * 0.35);
	ALPHA_SCISSOR_THRESHOLD = alpha_cut;
	ROUGHNESS = 0.82;
	SPECULAR = 0.22;
	BACKLIGHT = backlight;
}
