shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, diffuse_lambert, specular_schlick_ggx;

// Eau des fleuves de la carte de campagne (lot V4, A1-11), maillée par RiversRenderer.
// Sommets posés sur la ligne médiane : NORMAL = perpendiculaire (unitaire, signée selon le côté),
// UV = (largeur monde, côté ±1), UV2 = (abscisse le long du fleuve vers l'aval, px carte ; largeur
// à l'embouchure). Le ruban est élargi à au moins `min_px` pixels écran (fleuves lisibles au
// dézoom, estompés s'ils sont plus fins). Contrairement à l'ancien trait (terrain_line), le
// test de profondeur est actif : murs, ponts et collines cachent l'eau.
// Hauteur : dans les tuiles de relief fin (lit creusé, river_bed.gdshaderinc), l'eau descend de
// `water_sink` sous la berge ; ailleurs elle flotte de `lift_base + lift_per_distance × d`
// au-dessus du relief (LOD grossier qui ne suit pas le fond de vallée).
// Lot SS3 (ADR 0142) : mode « nappe » (`sheet`) pour les lacs de LakesRenderer : sommets posés
// tels quels (pas d'élargissement ni de lit creusé), flottant du même décalage que les fleuves,
// rides sans courant, opacité `sheet_alpha`, pas de relais du réseau fin.

#include "res://shaders/river_bed.gdshaderinc"
// Lot CM2 : fleuves à l'encre bleue en vue parchemin.
#include "res://shaders/parchment_common.gdshaderinc"
uniform vec3 pm_river_ink = vec3(0.04, 0.10, 0.12);

uniform vec2 map_size = vec2(1.0, 1.0);  // taille du monde (map.json size_px), fournie par le script
// ZG4 : rubans cuits en altitude × `baked_vertical_scale` (MapData.HEIGHT_SCALE, SANS relief exagéré),
// posés à la hauteur affichée courante (ZG8 : `campaign_display_height`) sans reconstruction.
#include "res://shaders/campaign_relief.gdshaderinc"
uniform float baked_vertical_scale = 0.006;
uniform vec3 shallow_color : source_color = vec3(0.30, 0.40, 0.36);
uniform vec3 deep_color : source_color = vec3(0.13, 0.26, 0.31);
// Reflet du ciel (fresnel), accordé à la brume chaude de la carte.
uniform vec3 sky_color : source_color = vec3(0.66, 0.72, 0.76);
uniform float sky_reflect = 0.55;
uniform vec3 foam_color : source_color = vec3(0.80, 0.82, 0.78);
uniform float min_px = 1.6;
uniform float thin_fade = 0.7;
// RC (ADR 0141) : largeur écran minimale × facteur selon l'importance (UV2.y, 0-6) : au dézoom,
// les grands fleuves restent plus épais que leurs affluents.
// Importances 0-3 dans `importance_px_lo`, 4-6 dans `importance_px_hi`.
uniform vec4 importance_px_lo = vec4(1.0);
uniform vec3 importance_px_hi = vec3(1.0);

float importance_px_scale(float importance) {
	int i = clamp(int(importance + 0.5), 0, 6);
	return i < 4 ? importance_px_lo[i] : importance_px_hi[i - 4];
}
// RC : liseré de berge à l'encre (contraste avec le relief, lecture cartographique au dézoom).
uniform vec3 bank_ink_color : source_color = vec3(0.05, 0.13, 0.20);
uniform float bank_ink_strength = 0.0;
// HB7 : liseré et fondu du bord en PIXELS écran (un ruban de 3 px n'est plus que liseré sombre).
uniform float bank_ink_px = 0.8;
uniform float edge_soft_px = 0.7;
// RC : fondu des rivières mineures avec la distance (x = début, y = fin, unités monde ; y = 0 : aucun).
uniform vec2 distance_fade = vec2(0.0, 0.0);
uniform float far_alpha = 1.0;
uniform float water_sink = 0.022;
uniform float lift_base = 0.012;
uniform float lift_per_distance = 0.0009;
// Débordement du ruban sur les berges creusées (l'eau s'arrête d'elle-même sur la pente).
uniform float carved_overlap = 1.35;
uniform float flow_speed = 0.35;
uniform float ripple_scale = 3.2;
uniform float ripple_strength = 0.08;
uniform float roughness_value = 0.06;
// Épaisseur d'eau (unités monde) à partir de laquelle la couleur est « profonde ».
uniform float depth_full = 0.05;
// Gués et bacs : (x, z, rayon px, 1 = gué / 0 = bac).
uniform vec4 fords[32];
uniform int ford_count = 0;

uniform bool sheet = false;
uniform float sheet_alpha = 0.88;
uniform float sheet_deep = 0.75;

uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
// RC5 (ADR 0141) : détail tuilable Nano Banana 2 (`river_large` / `river_small`) le long du
// courant ; force 0 (défaut, texture absente) = rendu procédural seul.
#include "res://shaders/water_detail.gdshaderinc"
uniform float water_detail_flow = 0.05;

varying float v_alpha;
varying float v_side;
varying vec2 v_flow;
varying vec3 v_world;
varying float v_carved;
varying float v_width;
varying float v_far;

float hash21(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float vnoise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash21(i), hash21(i + vec2(1.0, 0.0)), u.x), mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), u.x), u.y);
}

void vertex() {
	vec2 axis_xz = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xz;
	VERTEX.y = campaign_display_height(VERTEX.y / baked_vertical_scale, axis_xz);
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 view_pos = (VIEW_MATRIX * vec4(world, 1.0)).xyz;
	float dist = max(length(view_pos), 0.01);
	if (sheet) {
		VERTEX.y += lift_base + lift_per_distance * dist;
		v_alpha = 1.0;
		v_far = 1.0;
		v_side = 0.0;
		v_flow = world.xz * vec2(0.35, 0.5);
		v_carved = 0.0;
		v_width = 0.0;
		v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
		NORMAL = vec3(0.0, 1.0, 0.0);
	} else {
		// HB7 : abs() : sous Metal/Vulkan la projection porte un retournement de Y ([1][1] < 0), ce qui
		// rendait la largeur écran minimale inopérante (rubans sous-pixel, traits gris).
		float world_per_px = dist * 2.0 / (abs(PROJECTION_MATRIX[1][1]) * VIEWPORT_SIZE.y);
		float true_px = UV.x / world_per_px;
		float carved = has_river_bed ? carve_weight(world.xz) : 0.0;
		float width = UV.x * mix(1.0, carved_overlap, carved);
		float imp_px = min_px * importance_px_scale(UV2.y);
		float half_w = 0.5 * max(width, imp_px * world_per_px);
		VERTEX += NORMAL * half_w;
		VERTEX.y += mix(lift_base + lift_per_distance * dist, -water_sink, carved);
		v_alpha = mix(1.0, clamp(true_px / imp_px, 0.25, 1.0), thin_fade);
		v_far = distance_fade.y > 0.0 ? mix(1.0, far_alpha, smoothstep(distance_fade.x, distance_fade.y, dist)) : 1.0;
		// HB7 : hors du lit creusé, |v_side| = 1 au bord du ruban AFFICHÉ (élargi à `min_px`) : l'eau
		// remplit toute la largeur écran au lieu de n'occuper que le cœur sous-pixel (trait gris).
		// Dans le lit creusé, 1 = berge réelle (le ruban déborde sur la pente).
		v_side = UV.y * mix(1.0, half_w / max(0.5 * UV.x, 1e-4), carved);
		v_flow = vec2(UV2.x, UV.y * 0.5 * UV.x);
		v_carved = carved;
		v_width = UV.x;
		v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
		NORMAL = vec3(0.0, 1.0, 0.0);
	}
}

void fragment() {
	// Distance au bord de l'eau réelle (1 = berge théorique du lit).
	float edge = abs(v_side);
	// Épaisseur d'eau d'après le tampon de profondeur (lit creusé de près).
	float thickness = 0.0;
	if (v_carved > 0.001) {
		float scene_depth = texture(depth_tex, SCREEN_UV).r;
		vec4 scene_view = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, scene_depth, 1.0);
		scene_view.xyz /= scene_view.w;
		thickness = max(VERTEX.z - scene_view.z, 0.0);
	}
	float depth_t = clamp(thickness / depth_full, 0.0, 1.0);
	// Loin (non creusé) : profondeur fictive décroissant vers les bords du ruban.
	float fake_depth = 1.0 - smoothstep(0.35, 1.05, edge);
	float deep = sheet ? sheet_deep : mix(fake_depth, depth_t, v_carved);

	// Écoulement : deux couches de bruit étirées dans le sens du courant, décalées dans le temps.
	vec2 q = vec2(v_flow.x * 0.8, v_flow.y * 2.2) * ripple_scale;
	float t = TIME * flow_speed * ripple_scale * (sheet ? 0.25 : 1.0);
	float n1 = vnoise(q + vec2(-t, 0.0));
	float n2 = vnoise(q * 1.9 + vec2(-t * 1.6, 3.7));
	float nx = vnoise(q + vec2(-t + 0.07, 0.0)) - n1;
	float ny = vnoise(q + vec2(-t, 0.07)) - n1;

	// Gués / bacs : eau claire et peu profonde, remous et cailloux au gué.
	float ford = 0.0;
	float riffle = 0.0;
	// Gués : seulement de près (au loin, ils font moins d'un pixel).
	int fords_here = (!sheet && fwidth(v_flow.x) < 0.5) ? ford_count : 0;
	for (int i = 0; i < 32; i++) {
		if (i >= fords_here) {
			break;
		}
		vec4 f = fords[i];
		float d = distance(v_world.xz, f.xy);
		float w = (1.0 - smoothstep(f.z * 0.4, f.z, d)) * f.w;
		ford = max(ford, w);
	}
	if (ford > 0.0) {
		float stones = smoothstep(0.62, 0.7, vnoise(v_world.xz * 14.0)) * ford;
		riffle = smoothstep(0.55, 0.85, vnoise(vec2(v_flow.x * 9.0 - TIME * 1.5, v_flow.y * 30.0))) * ford;
		deep = min(deep, 1.0 - 0.8 * ford);
		riffle = max(riffle, stones * 0.6);
	}

	vec3 col = mix(shallow_color, deep_color, deep);
	col *= 0.92 + 0.16 * n2;
	// RC5 : coordonnées du courant (abscisse aval, travers) : la texture suit le lit.
	vec2 rc_uv = vec2(v_flow.x, v_flow.y) * water_detail_scale;
	col = water_detail_tint(col, rc_uv);
	// Écume légère au contact des berges et sur les remous du gué.
	float bank_foam = smoothstep(0.82, 1.0, edge) * (0.5 + 0.5 * n2) * (1.0 - v_carved) * 0.25;
	float contact = v_carved * (1.0 - smoothstep(0.0, 0.006, thickness)) * 0.5;
	float foam = clamp(bank_foam + contact + riffle, 0.0, 1.0);
	col = mix(col, foam_color, foam * 0.7);
	// RC : liseré d'encre sur les bords du ruban (pas dans le lit creusé, où la berge suffit).
	// HB7 : demi-largeur du ruban et distance au bord en pixels écran (dérivées écran de v_side).
	float half_px = sheet ? 1000.0 : 1.0 / max(fwidth(v_side), 1e-4);
	float px_edge = max(1.0 - edge, 0.0) * half_px;
	float ink = (1.0 - smoothstep(0.0, bank_ink_px, px_edge)) * smoothstep(1.2, 3.0, half_px) * bank_ink_strength * (1.0 - v_carved);

	ALBEDO = col;
	// Rides trop fines pour l'écran : surface plus mate (pas de scintillement du soleil).
	float fine_ripples = 1.0 - smoothstep(0.08, 0.45, fwidth(q.x));
	ROUGHNESS = mix(mix(0.42, roughness_value, fine_ripples), 0.35, foam);
	METALLIC = 0.0;
	SPECULAR = mix(0.22, 0.5, fine_ripples);
	// Rides atténuées quand elles deviennent plus fines que le pixel (pas de scintillement).
	float ripple_fade = 1.0 - smoothstep(0.08, 0.45, fwidth(q.x));
	vec3 n = normalize(vec3(-nx * ripple_strength * 10.0 * ripple_fade, 1.0, -ny * ripple_strength * 10.0 * ripple_fade));
	n = normalize(n + (water_detail_normal_ws(v_flow, TIME, vec2(-water_detail_flow, 0.0)) - vec3(0.0, 1.0, 0.0)) * ripple_fade);
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
	float fresnel = pow(1.0 - clamp(dot(NORMAL, VIEW), 0.0, 1.0), 4.0);
	ALBEDO = mix(ALBEDO, sky_color, clamp(fresnel * sky_reflect + 0.12, 0.0, 0.8) * (1.0 - foam));
	ALBEDO = mix(ALBEDO, bank_ink_color, ink);
	// Bord : fondu (ruban) ou contact avec la berge (lit creusé : l'eau finit sur la pente).
	float soft = 1.0 - (1.0 - smoothstep(0.0, edge_soft_px, px_edge)) * (1.0 - v_carved);
	float carved_alpha = mix(0.7, 0.97, smoothstep(0.0, 0.35, depth_t));
	float alpha = mix(0.9, carved_alpha, v_carved) * soft * v_alpha * v_far;
	ALPHA = clamp(alpha * (1.0 - ford * 0.35), 0.0, 1.0);
	if (sheet) {
		ALPHA = sheet_alpha;
	} else {
		ALPHA *= 1.0 - fine_zone_mask(v_world.xz);  // ZG5b : relais du réseau fin
	}
	float pm_w = pm_weight();  // CM2 : trait d'encre bleue, sans reflet
	ALBEDO = mix(ALBEDO, pm_river_ink * (1.0 - pm_emission), pm_w);
	EMISSION = pm_river_ink * pm_emission * pm_emission_gain * pm_w;
	ALPHA = mix(ALPHA, clamp(v_alpha * 1.2, 0.0, 0.95), pm_w);
}
