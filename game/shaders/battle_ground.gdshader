// Sol des batailles (lot V4) : mélange de 9 couches PBR Poly Haven (Texture2DArray) selon une
// splatmap cuite par `battle_terrain.gd` (sous-bois, chemins, boue, galets, berges), la pente
// (roche), des parcelles cultivées procédurales et la météo (humidité, neige). Deux échelles
// d'échantillonnage et une macro-variation cassent le tuilage.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

#include "res://shaders/battle_common.gdshaderinc"

const float L_GRASS = 0.0;
const float L_MEADOW = 1.0;
const float L_FOREST = 2.0;
const float L_DIRT = 3.0;
const float L_MUD = 4.0;
const float L_PEBBLES = 5.0;
const float L_ROCK = 6.0;
const float L_PLOUGH = 7.0;
const float L_SNOW = 8.0;

uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray normal_array : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D macro_noise : filter_linear_mipmap, repeat_enable;
// r = sous-bois, g = chemin, b = boue, a = galets (lit de rivière, gués).
uniform sampler2D splat_a : filter_linear, repeat_disable;
// r = berge humide, g = zone dégagée (pas de parcelles : places, abords de ville).
uniform sampler2D splat_b : filter_linear, repeat_disable;
uniform vec4 splat_rect = vec4(-400.0, -400.0, 2000.0, 1600.0);
// Rectangle (x0, z0, x1, z1) sans parcelles cultivées (centre du champ de bataille).
uniform vec4 calm_rect = vec4(150.0, 60.0, 1050.0, 740.0);
uniform float wetness : hint_range(0.0, 1.0) = 0.0;
uniform float snow : hint_range(0.0, 1.0) = 0.0;
// Lot B5 : altitude des neiges (sommets de l'horizon en montagne ou en hiver).
uniform float snow_line = 10000.0;
// Lot B7 : neige piétinée par les troupes (carte cumulée par `battle_terrain.gd`, même
// rectangle que les splatmaps ; 0 = neige vierge), active seulement quand il neige au sol.
uniform sampler2D trample_map : filter_linear, repeat_disable;
uniform float trample_on = 0.0;
uniform vec3 grass_tint : source_color = vec3(0.9, 1.0, 0.8);
uniform float detail_scale = 1.0;

varying vec3 world_pos;
varying vec3 world_normal;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	// Projection plane en x/z : tangente = +X, binormale = +Z.
	TANGENT = normalize(vec3(1.0, 0.0, 0.0) - NORMAL * NORMAL.x);
	BINORMAL = normalize(cross(TANGENT, NORMAL));
}

// Taille (m) d'une répétition de chaque couche.
float layer_size(float layer) {
	if (layer == L_GRASS) return 5.0;
	if (layer == L_MEADOW) return 7.0;
	if (layer == L_FOREST) return 4.0;
	if (layer == L_DIRT) return 5.0;
	if (layer == L_MUD) return 4.5;
	if (layer == L_PEBBLES) return 3.5;
	if (layer == L_ROCK) return 9.0;
	if (layer == L_PLOUGH) return 4.0;
	return 6.0;
}

// Échantillon anti-tuilage : deux échelles tournées, mélangées selon le bruit macro.
void sample_layer(float layer, vec2 p, float blend, inout vec3 albedo, inout vec3 nrm, float w) {
	if (w < 0.004) {
		return;
	}
	float s = layer_size(layer) * detail_scale;
	vec2 uv1 = p / s;
	vec2 uv2 = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8)) * p / (s * 2.3) + vec2(0.37, 0.71);
	vec3 a = mix(texture(albedo_array, vec3(uv1, layer)).rgb, texture(albedo_array, vec3(uv2, layer)).rgb, blend);
	vec3 n = mix(texture(normal_array, vec3(uv1, layer)).rgb, texture(normal_array, vec3(uv2, layer)).rgb, blend);
	albedo += a * w;
	nrm += n * w;
}

void fragment() {
	vec2 p = world_pos.xz;
	float macro = texture(macro_noise, p * 0.0021).r;
	float macro2 = texture(macro_noise, p * 0.00043 + vec2(0.3, 0.7)).r;
	float fine = texture(macro_noise, p * 0.017).r;
	float blend = smoothstep(0.35, 0.65, texture(macro_noise, p * 0.006 + 0.5).r);

	vec2 suv = (p - splat_rect.xy) / splat_rect.zw;
	// Bords des zones brisés par le bruit (la splatmap est à 4 m le texel).
	vec2 jitter = (vec2(fine, texture(macro_noise, p * 0.017 + 0.41).r) - 0.5) * 6.0 / splat_rect.zw;
	vec4 sa = texture(splat_a, suv + jitter);
	vec4 sb = texture(splat_b, suv);
	bool inside = all(greaterThanEqual(suv, vec2(0.0))) && all(lessThanEqual(suv, vec2(1.0)));
	if (!inside) {
		sa = vec4(0.0);
		sb = vec4(0.0);
	}

	// Parcelles cultivées hors du centre du champ.
	vec2 calm_d = max(calm_rect.xy - p, p - calm_rect.zw);
	float outside_calm = smoothstep(0.0, 60.0, max(calm_d.x, calm_d.y));
	vec3 field = bt_field_info(p);
	float field_w = (field.x > 0.5 ? 1.0 : 0.0) * outside_calm * (1.0 - sb.g) * field.z;

	float w_forest = smoothstep(0.15, 0.6, sa.r + (fine - 0.5) * 0.4);
	float w_dirt = smoothstep(0.2, 0.6, sa.g + (fine - 0.5) * 0.3);
	float w_mud = smoothstep(0.15, 0.55, sa.b + (macro - 0.5) * 0.4);
	float w_pebbles = smoothstep(0.25, 0.7, sa.a + (fine - 0.5) * 0.3);
	float slope = 1.0 - clamp(world_normal.y, 0.0, 1.0);
	float w_rock = smoothstep(0.16, 0.3, slope + (macro - 0.5) * 0.12);
	float w_plough = field.x == 1.0 ? field_w : 0.0;
	float w_meadow = smoothstep(0.25, 0.55, macro2 + (fine - 0.5) * 0.2);
	if (field.x == 3.0) {
		w_meadow = max(w_meadow, field_w);
	}
	float w_snow = snow * smoothstep(0.35, 0.75, world_normal.y * 0.6 + macro * 0.5 + fine * 0.2);
	w_snow = max(w_snow, smoothstep(snow_line, snow_line + 60.0, world_pos.y + (macro - 0.5) * 50.0) * smoothstep(0.25, 0.6, world_normal.y));
	// B7 : neige piétinée (bords brisés par le bruit) : la terre affleure par endroits.
	float trampled = 0.0;
	if (trample_on > 0.0 && inside) {
		float tm = texture(trample_map, suv + jitter * 0.5).r;
		trampled = smoothstep(0.08, 0.85, tm + (fine - 0.5) * 0.35 * tm) * trample_on;
		w_snow *= 1.0 - smoothstep(0.7, 1.0, trampled) * smoothstep(0.6, 0.9, fine + macro * 0.3) * 0.25;
		// B8 : sol détrempé (pas de neige) : les passages font apparaître la couche de boue.
		w_mud = max(w_mud, smoothstep(0.15, 0.8, trampled) * (1.0 - snow) * mix(0.6, 0.9, fine));
	}

	// Empilement par priorité : chaque couche recouvre les précédentes.
	float w[9];
	float rest = 1.0;
	float order[8] = float[](L_SNOW, L_PEBBLES, L_MUD, L_DIRT, L_ROCK, L_FOREST, L_PLOUGH, L_MEADOW);
	float ws[8] = float[](w_snow, w_pebbles, w_mud * (1.0 - w_snow * 0.5), w_dirt * (1.0 - w_snow * 0.6), w_rock, w_forest, w_plough, w_meadow);
	for (int i = 0; i < 9; i++) {
		w[i] = 0.0;
	}
	for (int i = 0; i < 8; i++) {
		float wi = clamp(ws[i], 0.0, 1.0) * rest;
		w[int(order[i])] = wi;
		rest -= wi;
	}
	w[int(L_GRASS)] = rest;

	vec3 albedo = vec3(0.0);
	vec3 nrm = vec3(0.0);
	for (int i = 0; i < 9; i++) {
		sample_layer(float(i), p, blend, albedo, nrm, w[i]);
	}

	// Herbe : teinte de saison, blé doré, macro-variation de luminosité et de teinte.
	float grassy = w[int(L_GRASS)] + w[int(L_MEADOW)];
	vec3 tint = grass_tint * mix(vec3(0.92, 1.0, 0.86), vec3(1.08, 1.02, 0.86), macro2);
	if (field.x == 2.0) {
		tint = mix(tint, vec3(1.45, 1.18, 0.62), field_w);
	}
	// Lot V4b : taches de prairie visibles de haut (herbe sèche blonde, prairie grasse sombre,
	// friches roussâtres) à deux échelles, pour casser l'uniformité du champ vu d'ensemble.
	float patch = texture(macro_noise, p * 0.0013 + vec2(0.61, 0.13)).r;
	float patch2 = texture(macro_noise, p * 0.0047 + vec2(0.27, 0.83)).r;
	float dry = smoothstep(0.42, 0.66, patch * 0.7 + patch2 * 0.45);
	float lush = smoothstep(0.45, 0.7, (1.0 - patch) * 0.75 + patch2 * 0.35);
	float rust = smoothstep(0.68, 0.8, patch2) * (1.0 - dry);
	tint = mix(tint, tint * vec3(1.35, 1.12, 0.62), dry * 0.85);
	tint = mix(tint, tint * vec3(0.7, 0.86, 0.66), lush * 0.8);
	tint = mix(tint, tint * vec3(1.15, 0.92, 0.72), rust * 0.45);
	albedo = mix(albedo, albedo * tint, grassy);
	albedo *= mix(0.84, 1.1, macro) * mix(0.93, 1.05, fine);
	// Sillons des labours.
	if (w_plough > 0.0) {
		float furrow = 0.5 + 0.5 * sin(field.y * 6.2832 / 1.3);
		albedo *= mix(1.0, mix(0.72, 1.05, furrow), w[int(L_PLOUGH)]);
	}
	// Lot B7 : relief de lecture de la neige. Congères étirées par le vent (bruit anisotrope),
	// grain fin, ombre bleutée et neige plus mince sur les pentes, crêtes plus blanches ; neige
	// piétinée grise. (Des ondulations dans la carte de normales ont été essayées : elles
	// donnaient au sol l'air d'une mer agitée.)
	float snowy_w = w[int(L_SNOW)];
	if (snowy_w > 0.01) {
		vec2 wind = mat2(vec2(0.94, 0.34), vec2(-0.34, 0.94)) * p;
		float drift = texture(macro_noise, wind * vec2(0.0012, 0.005)).r;
		float drift2 = texture(macro_noise, wind * vec2(0.004, 0.013) + vec2(0.17, 0.53)).r;
		float lee = smoothstep(0.02, 0.22, slope);
		float crest = smoothstep(0.55, 0.8, drift * 0.7 + drift2 * 0.4);
		vec3 snow_col = vec3(mix(0.84, 1.06, drift) * mix(0.96, 1.03, drift2));
		snow_col = mix(snow_col, snow_col * vec3(0.8, 0.87, 1.0), lee * 0.8 + (1.0 - crest) * 0.12);
		snow_col *= mix(0.97, 1.03, fine);
		albedo = mix(albedo, albedo * snow_col, snowy_w);
		// Piétinement : neige tassée grise, mêlée de boue.
		albedo = mix(albedo, albedo * vec3(0.8, 0.79, 0.77), trampled * snowy_w);
	}
	// B8 : boue piétinée (sol détrempé, hors neige) — traces sombres et luisantes des passages,
	// distinctes de la neige tassée grise ci-dessus (`trampled * (1 - snowy_w)`).
	float mud_trample = trampled * (1.0 - snowy_w);
	albedo = mix(albedo, albedo * vec3(0.62, 0.54, 0.46), mud_trample * 0.7);
	// Berges humides et sol trempé par la pluie.
	float wet = clamp(max(wetness * (0.55 + 0.45 * macro), sb.r * 0.4), 0.0, 1.0) * (1.0 - w[int(L_SNOW)]);
	albedo *= mix(1.0, 0.62, wet);

	ALBEDO = albedo;
	NORMAL_MAP = nrm;
	NORMAL_MAP_DEPTH = 1.0;
	// Boue tassée : plus luisante que le sol détrempé environnant.
	float mud_gloss = clamp(wet + mud_trample * 0.7, 0.0, 1.0);
	ROUGHNESS = mix(0.95, 0.38, mud_gloss * (0.5 + 0.5 * w[int(L_MUD)] + 0.3 * w[int(L_DIRT)]));
	SPECULAR = 0.35;
}
