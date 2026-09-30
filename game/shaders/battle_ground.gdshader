// Sol des batailles (lot V4) : mélange de 9 couches PBR Poly Haven (Texture2DArray) selon une
// splatmap cuite par `battle_terrain.gd` (sous-bois, chemins, boue, galets, berges), la pente
// (roche), des parcelles cultivées procédurales et la météo (humidité, neige). Deux échelles
// d'échantillonnage et une macro-variation cassent le tuilage.
// Lot R2 : relief de détail sous la grille de 10 m (normales procédurales, rendu seulement) et
// couleurs selon la carte de relief cuite par `battle_terrain.gd` (pente, creux, crêtes) :
// roche affleurante sur les pentes et les crêtes des collines, terre et cailloux des crêtes,
// herbe grasse, humidité et ombre des creux.
// Lot GA2 : jusqu'à 4 couches supplémentaires (prairie fleurie, herbe piétinée, chaume, labour
// frais), identité et taille de répétition lues depuis les données (`layer_count`,
// `layer_tile_size[]`, `idx_*`, cf. `battle_terrain.gd::ground_layers()` et
// `data/fx/battle_ground_layers.json`) : les 9 rôles historiques ci-dessous restent des index
// fixes (occupation du sol déjà spécifique à chaque rôle), les rôles ajoutés sont des index de
// données (-1 si absents). Macro-variation supplémentaire (50–200 m, teinte et luminance, faible
// amplitude) sous `ga2_on` (`--no-ga2`).
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

#include "res://shaders/battle_common.gdshaderinc"
#include "res://shaders/ga_macro.gdshaderinc"

const float L_GRASS = 0.0;
const float L_MEADOW = 1.0;
const float L_FOREST = 2.0;
const float L_DIRT = 3.0;
const float L_MUD = 4.0;
const float L_PEBBLES = 5.0;
const float L_ROCK = 6.0;
const float L_PLOUGH = 7.0;
const float L_SNOW = 8.0;
// GA2 : taille max du tableau de couches (miroir de `BattleTerrain.MAX_GROUND_LAYERS`).
const int MAX_GROUND_LAYERS = 16;

uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray normal_array : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
// GA2 : nombre réel de couches et taille de répétition (m) de chacune, depuis les données.
uniform int layer_count = 9;
uniform float layer_tile_size[MAX_GROUND_LAYERS];
// GA2 : index de couche des rôles ajoutés (-1 si absents des données).
uniform int idx_flowering_meadow = -1;
uniform int idx_trodden_grass = -1;
uniform int idx_stubble = -1;
uniform int idx_fresh_plough = -1;
uniform float ga2_on = 1.0;
uniform sampler2D macro_noise : filter_linear_mipmap, repeat_enable;
// r = sous-bois, g = chemin, b = boue, a = galets (lit de rivière, gués).
uniform sampler2D splat_a : filter_linear, repeat_disable;
// r = berge humide, g = zone dégagée (pas de parcelles : places, abords de ville).
uniform sampler2D splat_b : filter_linear, repeat_disable;
uniform vec4 splat_rect = vec4(-400.0, -400.0, 2000.0, 1600.0);
// Rectangle (x0, z0, x1, z1) sans parcelles cultivées (centre du champ de bataille).
uniform vec4 calm_rect = vec4(150.0, 60.0, 1050.0, 740.0);
// EP6 : parcelles du décor (voir `bt_decor_field`), même rectangle que les splatmaps.
uniform sampler2D decor_fields : filter_nearest, repeat_disable;
uniform float decor_on = 0.0;
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
// Lot R2 : r = pente (1 = 50 %), g = creux (> 0,5) / bosse (< 0,5) à 30 m, b = idem à 90 m.
uniform sampler2D relief_map : filter_linear, repeat_disable;
uniform vec4 relief_rect = vec4(-405.0, -405.0, 2010.0, 1610.0);
uniform float relief_on = 0.0;
// Roche affleurante (0 en plaine, 1 en montagne) et force des normales de détail.
uniform float outcrops : hint_range(0.0, 1.0) = 0.0;
uniform float detail_bump = 0.7;
// DA6 : lisières douces des parcelles et détail du sol de près (`--no-da6` : 0).
uniform float da6_on = 0.0;
uniform float decor_saturation = 0.6; // DA6 : part de la saturation gardée (bible § 3.3)
uniform float detail_near = 45.0; // distance (m) sous laquelle le détail est plein
// PO4 : texture de détail proche (Poly Haven CC0, terre et herbe rase, `near_detail/SOURCE.md`),
// relue à l'échelle réelle sous la caméra rapprochée : sa luminance relative module l'albédo, sa
// normale s'ajoute ; fondue avec la distance (au-delà de `near_detail_dist`, aucun échantillon).
uniform sampler2D near_detail_albedo : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D near_detail_normal : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform float near_detail_on = 0.0;
uniform float near_detail_size = 1.4; // m par tuile
uniform float near_detail_dist = 32.0;
uniform float near_detail_strength = 0.6;

varying vec3 world_pos;
varying vec3 world_normal;
varying float view_dist;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	view_dist = length((MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz);
	// Projection plane en x/z : tangente = +X, binormale = +Z.
	TANGENT = normalize(vec3(1.0, 0.0, 0.0) - NORMAL * NORMAL.x);
	BINORMAL = normalize(cross(TANGENT, NORMAL));
}

// Taille (m) d'une répétition de chaque couche.
// GA2 : taille de répétition (m) depuis les données (`data/fx/battle_ground_layers.json`,
// `layer_tile_size[]`), plus codée en dur ici.
float layer_size(float layer) {
	return layer_tile_size[clamp(int(layer), 0, MAX_GROUND_LAYERS - 1)];
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

// Lot R2 : hauteur procédurale de détail (m) : mottes et ornières (3 m), bosses (9 m),
// ondulations (26 m) ; `rough` renforce les petites échelles (sol caillouteux).
float detail_height(vec2 p, float rough) {
	float h = (texture(macro_noise, p / 26.0 + vec2(0.13, 0.57)).r - 0.5) * 0.55;
	h += (texture(macro_noise, p / 9.0 + vec2(0.71, 0.29)).r - 0.5) * 0.22;
	h += (texture(macro_noise, p / 3.1 + vec2(0.43, 0.91)).r - 0.5) * mix(0.05, 0.14, rough);
	return h;
}

void fragment() {
	vec2 p = world_pos.xz;
	float macro = texture(macro_noise, p * 0.0021).r;
	float macro2 = texture(macro_noise, p * 0.00043 + vec2(0.3, 0.7)).r;
	float fine = texture(macro_noise, p * 0.017).r;
	float blend = smoothstep(0.35, 0.65, texture(macro_noise, p * 0.006 + 0.5).r);
	// Lot V4b : taches de prairie (herbe sèche, prairie grasse), calculées ici pour servir aussi
	// au partage prairie fleurie de GA2, ci-dessous.
	float patch = texture(macro_noise, p * 0.0013 + vec2(0.61, 0.13)).r;
	float patch2 = texture(macro_noise, p * 0.0047 + vec2(0.27, 0.83)).r;

	vec2 suv = (p - splat_rect.xy) / splat_rect.zw;
	// Bords des zones brisés par le bruit (la splatmap est à 4 m le texel).
	vec2 jitter = (vec2(fine, texture(macro_noise, p * 0.017 + 0.41).r) - 0.5) * 6.0 / splat_rect.zw;
	vec4 sa = texture(splat_a, suv + jitter);
	sa.b = texture(splat_a, suv + jitter + bt_mud_warp(p) / splat_rect.zw).b;
	vec4 sb = texture(splat_b, suv);
	bool inside = all(greaterThanEqual(suv, vec2(0.0))) && all(lessThanEqual(suv, vec2(1.0)));
	if (!inside) {
		sa = vec4(0.0);
		sb = vec4(0.0);
	}

	// Parcelles cultivées hors du centre du champ.
	vec2 calm_d = max(calm_rect.xy - p, p - calm_rect.zw);
	float outside_calm = smoothstep(0.0, 60.0, max(calm_d.x, calm_d.y));
	// DA6 : lisières ondulées et fondues (mêmes formules que l'herbe).
	vec3 field = da6_on > 0.5 ? bt_field_info_soft(p) : bt_field_info(p);
	float field_w = (field.x > 0.5 ? 1.0 : 0.0) * outside_calm * (1.0 - sb.g) * field.z;
	// EP6 : parcelles posées par le cœur (labours, blé, prés, semis, vignes, chaumes).
	float decor_kind = 0.0;
	float furrow_period = 1.3;
	if (decor_on > 0.5 && inside) {
		vec2 duv = da6_on > 0.5 ? (p + bt_edge_warp(p) - splat_rect.xy) / splat_rect.zw : suv;
		vec4 df = texture(decor_fields, duv);
		if (df.a > 0.5) {
			field = bt_decor_field(df, p);
			decor_kind = field.x;
			if (decor_kind == 4.0 || decor_kind == 5.0) {
				field.x = 1.0;
				furrow_period = decor_kind == 5.0 ? 2.2 : 0.9;
			} else if (decor_kind == 6.0) {
				field.x = 2.0;
			}
			field_w = da6_on > 0.5 ? bt_decor_edge_soft(field.z, p) : field.z;
		}
	}

	float w_forest = smoothstep(0.15, 0.6, sa.r + (fine - 0.5) * 0.4);
	float w_dirt = smoothstep(0.2, 0.6, sa.g + (fine - 0.5) * 0.3);
	// VN2 : canal boue lu à une position déformée (`bt_mud_warp`, sinon cercles parfaits).
	float w_mud = smoothstep(0.15, 0.55, sa.b + (macro - 0.5) * 0.4);
	float w_pebbles = smoothstep(0.25, 0.7, sa.a + (fine - 0.5) * 0.3);
	float slope = 1.0 - clamp(world_normal.y, 0.0, 1.0);
	float w_rock = smoothstep(0.16, 0.3, slope + (macro - 0.5) * 0.12);
	// Lot R2 : pente, creux et crêtes lus sur la carte de relief (grille de la simulation).
	vec4 relief = vec4(0.0, 0.5, 0.5, 1.0);
	if (relief_on > 0.0) {
		vec2 ruv = (p - relief_rect.xy) / relief_rect.zw;
		if (all(greaterThanEqual(ruv, vec2(0.0))) && all(lessThanEqual(ruv, vec2(1.0)))) {
			relief = texture(relief_map, ruv);
		}
	}
	float grade = relief.r * 0.5;
	float hollow = clamp((relief.g - 0.5) * 2.0 + (relief.b - 0.5) * 1.2, -1.0, 1.0);
	float crest = smoothstep(0.05, 0.5, -hollow);
	float dip = smoothstep(0.05, 0.5, hollow);
	// Roche affleurante : pentes fortes et crêtes des collines et montagnes, en taches.
	float outcrop = outcrops * smoothstep(0.5, 0.95, grade * 1.9 + crest * 0.4 + (fine - 0.5) * 0.5 + (macro - 0.5) * 0.35);
	w_rock = max(w_rock, outcrop);
	// Crêtes : sol maigre, terre et cailloux ; creux : pas de roche.
	float thin_soil = smoothstep(0.3, 0.8, -hollow) * smoothstep(0.55, 0.85, fine + macro * 0.3) * mix(0.3, 0.8, outcrops);
	w_dirt = max(w_dirt, thin_soil * 0.5);
	w_pebbles = max(w_pebbles, thin_soil * outcrops * 0.5);
	w_rock *= 1.0 - dip * 0.8;
	// VN2 : berges de rivière (lit creusé, pente forte) en herbe humide et non en roche nue.
	w_rock *= 1.0 - smoothstep(0.1, 0.5, sb.r) * 0.9;
	// GA2 : labour frais (parcelle « labour » du décor, EP6 `decor_kind == 1`) tient une texture
	// dédiée (`farm_furrows`) distincte du labour ambiant/procédural (loin des parcelles).
	bool decor_labour = decor_kind == 1.0 && idx_fresh_plough >= 0;
	float w_plough = (field.x == 1.0 && !decor_labour) ? field_w : 0.0;
	float w_fresh_plough = decor_labour ? field_w : 0.0;
	// GA2 : chaume/éteules (EP6 `decor_kind == 6`, `field.x` mis à 2.0 ci-dessus).
	float w_stubble = (decor_kind == 6.0 && idx_stubble >= 0) ? field_w : 0.0;
	float w_meadow = smoothstep(0.25, 0.55, macro2 + (fine - 0.5) * 0.2);
	if (field.x == 3.0) {
		w_meadow = max(w_meadow, field_w);
	}
	// GA2 : prairie fleurie — part des taches de prairie grasse (`lush`, cf. plus bas) rendue avec
	// une texture dédiée plutôt que la prairie caillouteuse ordinaire.
	float w_flower_meadow = 0.0;
	if (idx_flowering_meadow >= 0) {
		float flower_share = smoothstep(0.45, 0.7, (1.0 - patch) * 0.75 + patch2 * 0.35);
		w_flower_meadow = w_meadow * flower_share;
		w_meadow -= w_flower_meadow;
	}
	// GA2 : herbe piétinée — halo élargi des chemins et routes (`splat_b.g`, déjà cuit), hors
	// terre battue elle-même et sous-bois.
	float w_trodden_grass = 0.0;
	if (idx_trodden_grass >= 0) {
		w_trodden_grass = clamp(sb.g - w_dirt, 0.0, 1.0) * (1.0 - w_forest) * 0.6;
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
		w_mud = max(w_mud, smoothstep(0.3, 0.9, trampled) * (1.0 - snow) * mix(0.45, 0.8, fine));
	}

	// Empilement par priorité : chaque couche recouvre les précédentes. GA2 : 4 rôles ajoutés
	// (index de données, -1 si absents) intercalés parmi les 9 rôles historiques.
	float w[MAX_GROUND_LAYERS];
	float rest = 1.0;
	const int ORDER_COUNT = 12;
	float order[ORDER_COUNT] = float[](
		L_SNOW, L_PEBBLES, L_MUD, L_DIRT, float(idx_trodden_grass), L_ROCK, L_FOREST,
		float(idx_fresh_plough), L_PLOUGH, float(idx_stubble), float(idx_flowering_meadow), L_MEADOW
	);
	float ws[ORDER_COUNT] = float[](
		w_snow, w_pebbles, w_mud * (1.0 - w_snow * 0.5), w_dirt * (1.0 - w_snow * 0.6),
		w_trodden_grass, w_rock, w_forest, w_fresh_plough, w_plough, w_stubble, w_flower_meadow, w_meadow
	);
	for (int i = 0; i < MAX_GROUND_LAYERS; i++) {
		w[i] = 0.0;
	}
	for (int i = 0; i < ORDER_COUNT; i++) {
		int oi = int(order[i]);
		if (oi < 0) {
			continue; // rôle de données absent (-1) : pas de couche à empiler.
		}
		float wi = clamp(ws[i], 0.0, 1.0) * rest;
		w[oi] = wi;
		rest -= wi;
	}
	w[int(L_GRASS)] = rest;

	vec3 albedo = vec3(0.0);
	vec3 nrm = vec3(0.0);
	for (int i = 0; i < layer_count; i++) {
		sample_layer(float(i), p, blend, albedo, nrm, w[i]);
	}

	// Herbe : teinte de saison, blé doré, macro-variation de luminosité et de teinte. GA2 :
	// prairie fleurie et herbe piétinée sont des variantes d'herbe, teintées pareillement.
	float grassy = w[int(L_GRASS)] + w[int(L_MEADOW)]
		+ (idx_flowering_meadow >= 0 ? w[idx_flowering_meadow] : 0.0)
		+ (idx_trodden_grass >= 0 ? w[idx_trodden_grass] : 0.0);
	vec3 tint = grass_tint * mix(vec3(0.92, 1.0, 0.86), vec3(1.08, 1.02, 0.86), macro2);
	if (field.x == 2.0) {
		tint = mix(tint, decor_kind == 6.0 ? vec3(1.3, 1.1, 0.7) : vec3(1.45, 1.18, 0.62), field_w);
	}
	// Lot V4b : taches de prairie visibles de haut (herbe sèche blonde, prairie grasse sombre,
	// friches roussâtres) à deux échelles, pour casser l'uniformité du champ vu d'ensemble
	// (`patch`/`patch2` calculés plus haut, réutilisés par la prairie fleurie GA2).
	float dry = smoothstep(0.42, 0.66, patch * 0.7 + patch2 * 0.45);
	float lush = smoothstep(0.45, 0.7, (1.0 - patch) * 0.75 + patch2 * 0.35);
	float rust = smoothstep(0.68, 0.8, patch2) * (1.0 - dry);
	tint = mix(tint, tint * vec3(1.35, 1.12, 0.62), dry * 0.85);
	tint = mix(tint, tint * vec3(0.7, 0.86, 0.66), lush * 0.8);
	tint = mix(tint, tint * vec3(1.15, 0.92, 0.72), rust * 0.45);
	// Lot R2 : herbe grasse et sombre dans les creux, sèche et blonde sur les crêtes.
	tint = mix(tint, tint * vec3(0.78, 0.92, 0.72), dip * 0.7);
	tint = mix(tint, tint * vec3(1.14, 1.06, 0.8), crest * 0.55);
	albedo = mix(albedo, albedo * tint, grassy);
	albedo *= mix(0.84, 1.1, macro) * mix(0.93, 1.05, fine);
	// DA6 : détail de près (sol net sous la caméra rapprochée). La couche dominante est relue à
	// ~1/4 de son échelle, tournée : sa luminance seule module l'albédo (grain, brindilles,
	// mottes), sa normale s'ajoute à celle du sol ; fondu avec la distance (coût nul au loin).
	float near_k = da6_on > 0.5 ? 1.0 - smoothstep(detail_near, detail_near * 2.6, view_dist) : 0.0;
	if (near_k > 0.01) {
		float dom = 0.0;
		float wmax = -1.0;
		for (int i = 0; i < layer_count; i++) {
			if (w[i] > wmax) {
				wmax = w[i];
				dom = float(i);
			}
		}
		vec2 duv = mat2(vec2(0.6, -0.8), vec2(0.8, 0.6)) * p / (layer_size(dom) * 0.27) + vec2(0.19, 0.83);
		vec3 dcol = texture(albedo_array, vec3(duv, dom)).rgb;
		vec3 dref = textureLod(albedo_array, vec3(duv, dom), 9.0).rgb;
		float dl = dot(dcol, vec3(0.3, 0.59, 0.11)) / max(dot(dref, vec3(0.3, 0.59, 0.11)), 0.02);
		albedo *= mix(1.0, clamp(dl, 0.55, 1.5), near_k * 0.5);
		vec3 dn = texture(normal_array, vec3(duv, dom)).rgb;
		vec3 n_near = nrm * 2.0 - 1.0;
		n_near.xy += (dn.xy * 2.0 - 1.0) * near_k * 0.6;
		nrm = normalize(n_near) * 0.5 + 0.5;
	}
	// PO4 : détail proche réel (grain de terre, brins ras) sur l'herbe, les prés, la terre et la
	// boue ; pas sur la roche ni la neige. Lecture tournée (axes du champ).
	float nd_k = near_detail_on * (1.0 - smoothstep(near_detail_dist * 0.45, near_detail_dist, view_dist));
	nd_k *= clamp(1.0 - w[int(L_ROCK)] - w[int(L_SNOW)] - w[int(L_PEBBLES)] * 0.5, 0.0, 1.0);
	if (nd_k > 0.01) {
		vec2 nuv = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8)) * p / near_detail_size + vec2(0.37, 0.11);
		vec3 ncol = texture(near_detail_albedo, nuv).rgb;
		float nref = dot(textureLod(near_detail_albedo, nuv, 10.0).rgb, vec3(0.3, 0.59, 0.11));
		float nl = dot(ncol, vec3(0.3, 0.59, 0.11)) / max(nref, 0.02);
		albedo *= mix(1.0, clamp(nl, 0.45, 1.6), nd_k * near_detail_strength);
		vec3 nn = texture(near_detail_normal, nuv).rgb;
		vec3 n_nd = nrm * 2.0 - 1.0;
		n_nd.xy += (nn.xy * 2.0 - 1.0) * nd_k * 0.8;
		nrm = normalize(n_nd) * 0.5 + 0.5;
	}
	// Sillons des labours.
	if (w_plough > 0.0) {
		// Sillons estompés vers leur teinte moyenne quand leur période passe sous ~4 pixels
		// (sinon moiré et scintillement au loin).
		float furrow_px = fwidth(field.y / furrow_period);
		float furrow = mix(0.5, 0.5 + 0.5 * sin(field.y * 6.2832 / furrow_period), 1.0 - smoothstep(0.12, 0.3, furrow_px));
		albedo *= mix(1.0, mix(0.72, 1.05, furrow), w[int(L_PLOUGH)]);
		// EP6 : blé en herbe des semis (rangs verts sur la terre).
		if (decor_kind == 4.0) {
			albedo = mix(albedo, albedo * vec3(0.75, 1.25, 0.55), w[int(L_PLOUGH)] * (0.35 + 0.4 * furrow));
		}
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
	float wet = clamp(max(wetness * (0.55 + 0.45 * macro), sb.r * mix(0.4, 0.85, max(w_mud, w_pebbles))), 0.0, 1.0) * (1.0 - w[int(L_SNOW)]);
	// Lot R2 : l'eau reste dans les creux (plus humides sous la pluie), ombre douce des fonds.
	wet = clamp(wet + dip * (0.12 + wetness * 0.45) * (1.0 - w[int(L_SNOW)]), 0.0, 1.0);
	albedo *= mix(1.0, 0.62, wet);
	// VN2 : grève et vase au bord de l'eau : terre sombre, mouillée, un peu verdie (avant :
	// gravier pâle et sec en bande continue le long de la rivière).
	// VN2 : boue brun-olive plutôt que lie-de-vin (vue de haut, les zones de boue faisaient des
	// taches violacées).
	float puddle = smoothstep(0.62, 0.8, texture(macro_noise, p * 0.021 + vec2(0.53, 0.29)).r);
	albedo = mix(albedo, vec3(0.17, 0.14, 0.1) * (0.75 + 0.5 * fine) * mix(1.0, 0.6, puddle), w_mud * 0.7 * (1.0 - w[int(L_SNOW)]));
	float bank_wet = smoothstep(0.3, 0.8, sb.r) * max(w_mud, w_pebbles) * (1.0 - w[int(L_SNOW)]);
	albedo = mix(albedo, albedo * vec3(0.66, 0.72, 0.58), bank_wet * 0.6);
	albedo *= 1.0 - dip * 0.08;

	// GA2 : macro-variation supplémentaire (4 octaves, 50 à 200 m) de teinte et de luminance,
	// faible amplitude, pour casser la répétition à moyenne échelle (indépendante des taches de
	// prairie ci-dessus, à plus grande échelle). `--no-ga2` la coupe (comparaison A/B).
	if (ga2_on > 0.5) {
		float ga2_h = texture(macro_noise, p / 50.0 + vec2(0.11, 0.37)).r * 0.4
			+ texture(macro_noise, p / 90.0 + vec2(0.53, 0.02)).r * 0.3
			+ texture(macro_noise, p / 140.0 + vec2(0.29, 0.71)).r * 0.2
			+ texture(macro_noise, p / 200.0 + vec2(0.83, 0.19)).r * 0.1;
		albedo *= ga_macro_variation(ga2_h, 0.5, 1.0, 0.0);  // GA4 : fonction partagée
	}
	// DA6 (bible § 3.3) : décor sourd, saturation ramenée vers la luminance.
	if (da6_on > 0.5) {
		albedo = mix(vec3(dot(albedo, vec3(0.3, 0.59, 0.11))), albedo, decor_saturation);
	}
	ALBEDO = albedo;
	// Lot R2 : relief de détail sous la grille de 10 m (normales, rendu seulement), plus marqué sur
	// les sols caillouteux, effacé au loin (anti-crénelage) et sous la neige.
	float bump_fade = (1.0 - smoothstep(120.0, 420.0, view_dist)) * detail_bump * (1.0 - snowy_w * 0.7);
	if (bump_fade > 0.01) {
		float rough = clamp(outcrops + w[int(L_ROCK)] + w[int(L_PEBBLES)], 0.0, 1.0);
		float e = 0.6;
		float h0 = detail_height(p, rough);
		vec2 grad = vec2(detail_height(p + vec2(e, 0.0), rough) - h0, detail_height(p + vec2(0.0, e), rough) - h0) / e;
		vec3 n_ts = nrm * 2.0 - 1.0;
		n_ts.xy -= grad * 2.2 * bump_fade;
		nrm = normalize(n_ts) * 0.5 + 0.5;
	}
	NORMAL_MAP = nrm;
	NORMAL_MAP_DEPTH = 1.0;
	// Boue tassée : plus luisante que le sol détrempé environnant.
	float mud_gloss = clamp(wet + mud_trample * 0.7, 0.0, 1.0);
	// VN2 : luisance plafonnée au loin (en vue rasante la boue mouillée renvoyait le ciel : plaques
	// pâles au lieu de sombres).
	float gloss_far = mix(0.38, 0.62, smoothstep(80.0, 300.0, view_dist));
	ROUGHNESS = mix(0.95, gloss_far, mud_gloss * (0.5 + 0.5 * w[int(L_MUD)] + 0.3 * w[int(L_DIRT)]));
	SPECULAR = 0.35;
}
