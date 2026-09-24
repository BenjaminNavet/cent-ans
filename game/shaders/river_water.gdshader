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

#include "res://shaders/river_bed.gdshaderinc"

uniform vec2 map_size = vec2(4096.0, 4096.0);
uniform vec3 shallow_color : source_color = vec3(0.23, 0.33, 0.28);
uniform vec3 deep_color : source_color = vec3(0.07, 0.17, 0.20);
uniform vec3 foam_color : source_color = vec3(0.80, 0.82, 0.78);
uniform float min_px = 1.6;
uniform float thin_fade = 0.7;
uniform float water_sink = 0.022;
uniform float lift_base = 0.012;
uniform float lift_per_distance = 0.0009;
// Débordement du ruban sur les berges creusées (l'eau s'arrête d'elle-même sur la pente).
uniform float carved_overlap = 1.35;
uniform float flow_speed = 0.35;
uniform float ripple_scale = 3.2;
uniform float ripple_strength = 0.35;
uniform float roughness_value = 0.06;
// Épaisseur d'eau (unités monde) à partir de laquelle la couleur est « profonde ».
uniform float depth_full = 0.05;
// Gués et bacs : (x, z, rayon px, 1 = gué / 0 = bac).
uniform vec4 fords[32];
uniform int ford_count = 0;

uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;

varying float v_alpha;
varying float v_side;
varying vec2 v_flow;
varying vec3 v_world;
varying float v_carved;
varying float v_width;

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
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 view_pos = (VIEW_MATRIX * vec4(world, 1.0)).xyz;
	float dist = max(length(view_pos), 0.01);
	float world_per_px = dist * 2.0 / (PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
	float true_px = UV.x / world_per_px;
	float carved = has_river_bed ? carve_weight(world.xz) : 0.0;
	float width = UV.x * mix(1.0, carved_overlap, carved);
	float half_w = 0.5 * max(width, min_px * world_per_px);
	VERTEX += NORMAL * half_w;
	VERTEX.y += mix(lift_base + lift_per_distance * dist, -water_sink, carved);
	v_alpha = mix(1.0, clamp(true_px / min_px, 0.25, 1.0), thin_fade);
	v_side = UV.y * half_w / max(0.5 * UV.x, 1e-4);
	v_flow = vec2(UV2.x, UV.y * 0.5 * UV.x);
	v_carved = carved;
	v_width = UV.x;
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	NORMAL = vec3(0.0, 1.0, 0.0);
}

void fragment() {
	// Distance au bord de l'eau réelle (1 = berge théorique du lit).
	float edge = abs(v_side);
	// Épaisseur d'eau d'après le tampon de profondeur (lit creusé de près).
	float scene_depth = texture(depth_tex, SCREEN_UV).r;
	vec4 scene_view = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, scene_depth, 1.0);
	scene_view.xyz /= scene_view.w;
	float thickness = max(VERTEX.z - scene_view.z, 0.0);
	float depth_t = clamp(thickness / depth_full, 0.0, 1.0);
	// Loin (non creusé) : profondeur fictive décroissant vers les bords du ruban.
	float fake_depth = 1.0 - smoothstep(0.35, 1.05, edge);
	float deep = mix(fake_depth, depth_t, v_carved);

	// Écoulement : deux couches de bruit étirées dans le sens du courant, décalées dans le temps.
	vec2 q = vec2(v_flow.x * 0.8, v_flow.y * 2.2) * ripple_scale;
	float t = TIME * flow_speed * ripple_scale;
	float n1 = vnoise(q + vec2(-t, 0.0));
	float n2 = vnoise(q * 1.9 + vec2(-t * 1.6, 3.7));
	float nx = vnoise(q + vec2(-t + 0.07, 0.0)) - n1;
	float ny = vnoise(q + vec2(-t, 0.07)) - n1;

	// Gués / bacs : eau claire et peu profonde, remous et cailloux au gué.
	float ford = 0.0;
	float riffle = 0.0;
	for (int i = 0; i < 32; i++) {
		if (i >= ford_count) {
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
	// Écume légère au contact des berges et sur les remous du gué.
	float bank_foam = smoothstep(0.82, 1.0, edge) * (0.5 + 0.5 * n2) * (1.0 - v_carved) * 0.25;
	float contact = v_carved * (1.0 - smoothstep(0.0, 0.006, thickness)) * 0.5;
	float foam = clamp(bank_foam + contact + riffle, 0.0, 1.0);
	col = mix(col, foam_color, foam * 0.7);

	ALBEDO = col;
	ROUGHNESS = mix(roughness_value, 0.35, foam);
	METALLIC = 0.0;
	SPECULAR = 0.5;
	vec3 n = normalize(vec3(-nx * ripple_strength * 10.0, 1.0, -ny * ripple_strength * 10.0));
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
	// Bord : fondu (ruban) ou contact avec la berge (lit creusé : l'eau finit sur la pente).
	float soft = 1.0 - smoothstep(0.7, 1.0, edge) * (1.0 - v_carved);
	float carved_alpha = mix(0.55, 0.96, smoothstep(0.0, 0.35, depth_t));
	float alpha = mix(0.9, carved_alpha, v_carved) * soft * v_alpha;
	ALPHA = clamp(alpha * (1.0 - ford * 0.35), 0.0, 1.0);
}
