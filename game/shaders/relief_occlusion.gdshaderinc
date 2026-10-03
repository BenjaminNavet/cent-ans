// Lot RV-D (chantier RV, ADR 0167) — occlusion de vallée précalculée.
// `data/map/relief_occlusion.png` (`cent-ans geo relief-occlusion`, tools/cent_ans_tools/geo/
// relief_occlusion.py) : L8 sur la demi-grille de la carte, 128 neutre, > 128 vallée ou bassin
// (horizon relevé), < 128 crête ou plateau dégagé. Basse fréquence (rayons 6-25 km),
// indépendante du soleil : rend la lecture des grandes formes que l'estompage directionnel ne
// donne pas. Vallées plus sombres et plus froides, crêtes un peu plus claires ; sortie AO du
// shader spatial pour que la lumière ambiante en tienne compte.
// Inclus par terrain.gdshader après les autres crochets ; un seul appel avant `ALBEDO = col;`.

uniform sampler2D rvd_occlusion : filter_linear_mipmap, repeat_disable, hint_default_black;
uniform bool rvd_has_occlusion = false;
// Assombrissement des fonds (0 = aucun) et teinte froide vers laquelle ils tirent.
uniform float rvd_strength : hint_range(0.0, 1.0) = 0.40;
uniform vec3 rvd_cool_tint : source_color = vec3(0.70, 0.78, 0.92);
// Éclaircissement des crêtes et plateaux (légèrement chaud).
uniform float rvd_crest_lift : hint_range(0.0, 0.5) = 0.14;
uniform vec3 rvd_warm_tint : source_color = vec3(1.0, 0.97, 0.90);
// Occlusion de la lumière ambiante au fond des vallées (sortie AO, 1 = aucune).
uniform float rvd_ao_strength : hint_range(0.0, 1.0) = 0.6;
// Fondu de près : empreinte (px carte / px écran) où l'effet tombe à `rvd_near_strength`
// (x) et où il est plein (y), seulement si le quadtree fin porte le relief.
uniform vec2 rvd_near_footprint = vec2(0.04, 0.3);
uniform float rvd_near_strength : hint_range(0.0, 1.0) = 0.5;

// Applique l'occlusion à `col` (terre seulement, poids `land`) et rend la valeur AO.
float rvd_apply(vec2 uv, float footprint, float land, inout vec3 col) {
	if (!rvd_has_occlusion || land <= 0.001) {
		return 1.0;
	}
	float o = texture(rvd_occlusion, uv).r * (255.0 / 127.0) - (128.0 / 127.0);  // [-1, 1]
	float w = land * (1.0 - pm_weight());  // s'efface sous le parchemin
	if (qt_has_page()) {
		w *= mix(rvd_near_strength, 1.0, smoothstep(rvd_near_footprint.x, rvd_near_footprint.y, footprint));
	}
	float valley = max(o, 0.0) * w;
	float crest = max(-o, 0.0) * w;
	// Fond de vallée (o = 1) : luminance × (1 − rvd_strength), teinte froide de même moyenne.
	vec3 cool = rvd_cool_tint / max(dot(rvd_cool_tint, vec3(1.0 / 3.0)), 1e-3) * (1.0 - rvd_strength);
	col *= mix(vec3(1.0), cool, valley);
	col *= mix(vec3(1.0), rvd_warm_tint * (1.0 + rvd_crest_lift), crest);
	return 1.0 - rvd_ao_strength * valley;
}
