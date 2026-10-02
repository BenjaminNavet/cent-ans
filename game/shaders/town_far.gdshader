shader_type spatial;
render_mode world_vertex_coords;

// Lot VT-C (ADR 0138) : maillage lointain des villes (paliers F1/F2, `town_far_builder.gd`).
// Contrat de sommets (lot VT-B) :
// - VERTEX.xz : position monde (unités carte) ; VERTEX.y : altitude ABSOLUE en mètres ;
// - UV2.x : index de ville (0-4095) ; UV2.y : hauteur (m) au-dessus du sol local (négative sur la
//   jupe), d'où le sol du sommet = VERTEX.y − UV2.y ;
// - COLOR : teinte ; NORMAL : normale monde.
// Pose comme `town_building.gdshader` : sol posé par `campaign_display_height` (ZG8, relief
// exagéré) et bâti au-dessus du sol à l'échelle 1:1 (mètres / `meters_per_unit`, comme sous le
// nœud racine d'une ville mis à l'échelle 1 / m par unité).
// Toits : teinte « masse de toits » partagée (`roofscape.gdshaderinc`) sur les faces tournées vers
// le ciel ; murs et jupe gardent COLOR. Mat (pas de spéculaire, rugosité 1) : pas de scintillement.
// Passage au plan complet : `built_mask` (R8 64 × 64, texel (i % 64, i / 64)) vaut 1 pour une ville
// dont la ville 1:1 est construite ; ses sommets à moins de `sink_distance` de la caméra sont
// enfoncés sous le sol (−`sink_depth_m`), avec une bande de transition sur la dernière fraction
// `sink_band` de la distance (complément du fondu des blocs de `town_builder.gd`, FADE_SELF).

uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform vec4 layer_tint[16];
uniform int roof_first = 7;
uniform int roof_last = 10;
uniform float snow = 0.0;
uniform float meters_per_unit = 719.0;
// Poids de la teinte de toits partagée sur les faces de toit (0 : COLOR seule).
uniform float roof_mix = 1.0;
#include "res://shaders/campaign_relief.gdshaderinc"
#include "res://shaders/roofscape.gdshaderinc"

uniform sampler2D built_mask : filter_nearest, repeat_disable;
uniform float sink_distance = 0.0;
uniform float sink_band = 0.1;
uniform float sink_depth_m = 30.0;
// Caméra principale (monde), la même pour la passe d'ombre ; y ≥ 1e5 : caméra du rendu courant.
uniform vec3 sink_camera = vec3(0.0, 1e6, 0.0);
// Lot TB3 (ADR 0162) : suie par ville, même index que `built_mask` (R8 64 × 64, 0-1).
uniform sampler2D soot_mask : filter_nearest, repeat_disable;
uniform vec3 soot_color : source_color = vec3(0.035, 0.03, 0.028);

const int MASK_SIDE = 64;

varying vec3 world_m;
varying vec3 world_n;
varying float v_soot;

// Enfoncement 0-1 d'un sommet de la ville `index` à la position monde `pos`.
float town_far_sink(int index, vec3 pos, vec3 camera) {
	if (sink_distance <= 0.0 || index < 0 || index >= MASK_SIDE * MASK_SIDE) {
		return 0.0;
	}
	if (texelFetch(built_mask, ivec2(index % MASK_SIDE, index / MASK_SIDE), 0).r < 0.5) {
		return 0.0;
	}
	float d = distance(pos, camera);
	return 1.0 - smoothstep(sink_distance * (1.0 - sink_band), sink_distance, d);
}

void vertex() {
	vec2 xz = VERTEX.xz;
	float above_m = UV2.y;
	float ground_y = campaign_display_height(VERTEX.y - above_m, xz);
	vec3 camera = sink_camera.y >= 1e5 ? CAMERA_POSITION_WORLD : sink_camera;
	int town = int(UV2.x + 0.5);
	float sink = town_far_sink(town, vec3(xz.x, ground_y, xz.y), camera);
	v_soot = town >= 0 && town < MASK_SIDE * MASK_SIDE ? texelFetch(soot_mask, ivec2(town % MASK_SIDE, town / MASK_SIDE), 0).r : 0.0;
	float h_m = mix(above_m, -sink_depth_m, sink);
	VERTEX.y = ground_y + h_m / meters_per_unit;
	world_m = VERTEX * meters_per_unit;
	world_n = NORMAL;
}

void fragment() {
	vec3 albedo = COLOR.rgb;
	float roughness = 1.0;
	float up = smoothstep(0.35, 0.7, normalize(world_n).y) * roof_mix;
	if (up > 0.0) {
		// Cellule plus petite qu'un pixel : teinte moyenne (pas de tirage par cellule qui scintille).
		float footprint = length(fwidth(world_m.xz));
		float detail = 1.0 - smoothstep(roofscape_cell_m * 0.5, roofscape_cell_m * 2.0, footprint);
		float r_cell;
		float r_avg;
		vec3 roof = mix(roofscape_average(world_m, r_avg), roofscape_color(world_m, r_cell), detail);
		albedo = mix(albedo, roof, up);
	}
	if (v_soot > 0.0) {
		float n = roof_hash(floor(world_m.xz / 16.0)) * 0.65 + roof_hash(floor(world_m.xz / 3.0)) * 0.35;
		// Vue de loin : plaques sous-pixel ramenées à leur moyenne (pas de scintillement).
		float patch = mix(smoothstep(1.0 - v_soot - 0.12, 1.0 - v_soot + 0.12, n), v_soot, smoothstep(8.0, 32.0, length(fwidth(world_m.xz))));
		albedo = mix(albedo, soot_color, patch * 0.9);
	}
	ALBEDO = albedo;
	ROUGHNESS = roughness;
	SPECULAR = 0.0;
}
