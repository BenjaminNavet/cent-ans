shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, diffuse_lambert, specular_schlick_ggx, world_vertex_coords;

// Fleuves fins de près (lot ZG5b, ADR 0036), maillés par FineRibbonJob / FineGeoLayer.
// Sommets sur l'axe : VERTEX.y = niveau d'eau en MÈTRES (× height_scale ici : l'exagération
// verticale dynamique du lot ZG4 ne demande aucun remaillage) ; NORMAL = perpendiculaire signée ;
// UV = (largeur réelle en unités monde, côté ±1) ; UV2 = (abscisse vers l'aval, ordre de
// Strahler) ; COLOR = (divagant, marée, marais, intermittent).
// Le lit est creusé dans les pages du relief (FineBedCarver) : l'eau est à plat au niveau `z`,
// l'épaisseur (tampon de profondeur) donne la couleur. Ruban élargi à au moins `min_px` pixels
// écran (estompé s'il est plus fin), petits ordres effacés avec la distance, bandes de berge
// selon les drapeaux : grèves et bancs de sable (divagant), vasières découvertes par la marée
// (marée), roselières (marais), lit de galets (intermittent).

// ZG4 : unités monde par mètre ; ZG8 : hauteur affichée (relief local exagéré).
#include "res://shaders/campaign_relief.gdshaderinc"
#define height_scale campaign_vertical_scale
// Disque couvert par le rendu fin (centre x, z ; rayon ; poids du palier près).
uniform vec4 fine_zone = vec4(0.0, 0.0, -1.0, 0.0);
uniform float fine_weight = 1.0;
// SZ2b : ouverture de l'eau dans les emprises des colonies (maquettes masquées, villes 1:1 de
// ZG6 affichées) et dans les zones des villes 1:1 (VH, fondu de leur maquette L1/L2). Marque
// par sommet dans UV2.y (ordre + 100 × marque : 1 emprise, 2 zone).
uniform float cover_open = 0.0;
uniform float shallow_alpha = 0.9;
uniform float zone_open = 0.0;
// Débogage (`--fine-debug`) : eau en magenta opaque.
uniform bool debug_flat = false;

uniform vec3 shallow_color : source_color = vec3(0.22, 0.33, 0.34);
uniform vec3 deep_color : source_color = vec3(0.07, 0.16, 0.21);
uniform vec3 brackish_color : source_color = vec3(0.33, 0.37, 0.30);
uniform vec3 sky_color : source_color = vec3(0.62, 0.72, 0.80);
uniform float sky_reflect = 0.75;
uniform vec3 foam_color : source_color = vec3(0.80, 0.82, 0.78);
uniform vec3 sand_color : source_color = vec3(0.62, 0.55, 0.42);
uniform vec3 gravel_color : source_color = vec3(0.50, 0.47, 0.42);
uniform vec3 mud_color : source_color = vec3(0.26, 0.22, 0.16);
uniform vec3 reed_color : source_color = vec3(0.36, 0.38, 0.18);
uniform vec3 reed_dry_color : source_color = vec3(0.55, 0.48, 0.30);

uniform float min_px = 1.2;
// HB7 : largeur écran minimale × facteur selon l'ordre de Strahler (3-6 dans `order_px_lo`,
// 7-10 dans `order_px_hi`) : relais sans saut des rubans moyens (RiversRenderer, par importance).
uniform vec4 order_px_lo = vec4(1.0);
uniform vec4 order_px_hi = vec4(1.0);

float order_px_scale(float order) {
	int i = clamp(int(order + 0.5), 3, 10) - 3;
	return i < 4 ? order_px_lo[i] : order_px_hi[i - 4];
}
uniform float thin_fade = 0.7;
// Déplacement vers la caméra (unités monde par unité de distance) : l'eau n'est pas mangée par
// les facettes du relief entre deux sommets du quadtree.
uniform float depth_bias = 0.0015;
// Petits cours d'eau : ordre 3 visible sous `order_distance`, ×2 par ordre au-dessus.
uniform float order_distance = 30.0;
// Largeur des bandes de berge (part de la demi-largeur de l'eau, et au moins en mètres).
uniform float tidal_band = 0.8;
uniform float wetland_band = 0.9;
uniform float divagating_band = 0.35;
uniform float band_min_m = 25.0;
uniform float meters_per_unit = 719.0;
// Marée : période (s) et part de la largeur découverte à marée basse.
uniform float tide_period = 90.0;
uniform float tide_amount = 0.35;
uniform float flow_speed = 0.35;
uniform float ripple_scale = 90.0;   // rides par unité monde (≈ 8 m)
uniform float ripple_strength = 0.08;
uniform float roughness_value = 0.06;
// Épaisseur d'eau (unités monde) pour la couleur « profonde ».
uniform float depth_full = 0.012;

uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;

varying float v_alpha;
varying float v_side;      // côté en demi-largeurs d'EAU (|v_side| = 1 au bord de l'eau)
varying float v_band;      // demi-largeur totale / demi-largeur de l'eau (≥ 1)
varying vec2 v_flow;       // (abscisse, écart à l'axe) en unités monde
varying vec3 v_world;
varying vec4 v_flags;
varying float v_width;
varying float v_open;

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

float zone_mask(vec2 p) {
	if (fine_zone.z <= 0.0) {
		return 0.0;
	}
	return fine_zone.w * (1.0 - smoothstep(fine_zone.z * 0.85, fine_zone.z, distance(p, fine_zone.xy)));
}

void vertex() {
	vec3 world = vec3(VERTEX.x, campaign_display_height(VERTEX.y, VERTEX.xz), VERTEX.z);
	vec3 to_cam = CAMERA_POSITION_WORLD - world;
	float dist = max(length(to_cam), 0.01);
	// HB7 : abs() : sous Metal/Vulkan la projection porte un retournement de Y ([1][1] < 0), ce qui
	// rendait la largeur écran minimale inopérante (rubans sous-pixel, traits gris).
	float world_per_px = dist * 2.0 / (abs(PROJECTION_MATRIX[1][1]) * VIEWPORT_SIZE.y);
	float mark = floor(UV2.y / 100.0 + 0.001);
	float order = UV2.y - 100.0 * mark;
	float px_min = min_px * order_px_scale(order);
	// HB7 : l'élargissement écran est de l'EAU (pas une bande de berge grise d'un pixel) ; les
	// bandes de berge s'ajoutent au-delà, à leur largeur réelle.
	float water_half = max(0.5 * UV.x, px_min * world_per_px * 0.5);
	float band_m = max(COLOR.g * tidal_band, max(COLOR.b * wetland_band, COLOR.r * divagating_band));
	float band = band_m > 0.0 ? max(0.5 * UV.x * band_m, band_min_m / meters_per_unit) : 0.0;
	float half_w = water_half + band;
	world += NORMAL * half_w;
	world += to_cam / dist * min(depth_bias * dist, 0.5);
	VERTEX = world;
	float true_px = UV.x / world_per_px;
	v_open = mark < 0.5 ? 1.0 : (mark < 1.5 ? cover_open : zone_open);
	float reach = order_distance * exp2(order - 3.0);
	v_alpha = mix(1.0, clamp(true_px / px_min, 0.25, 1.0), thin_fade) * (1.0 - smoothstep(reach * 0.75, reach, dist));
	v_side = UV.y * half_w / max(water_half, 1e-5);
	v_band = half_w / max(water_half, 1e-5);
	v_flow = vec2(UV2.x, UV.y * half_w);
	v_flags = COLOR;
	v_width = UV.x;
	v_world = world;
	NORMAL = vec3(0.0, 1.0, 0.0);
}

void fragment() {
	float edge = abs(v_side);
	float zone = zone_mask(v_world.xz) * fine_weight * v_open;
	if (zone <= 0.001 || v_alpha <= 0.001) {
		discard;
	}
	// Épaisseur d'eau (lit creusé) d'après le tampon de profondeur.
	float scene_depth = texture(depth_tex, SCREEN_UV).r;
	vec4 scene_view = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, scene_depth, 1.0);
	scene_view.xyz /= scene_view.w;
	float thickness = max(VERTEX.z - scene_view.z, 0.0);
	float depth_t = clamp(thickness / depth_full, 0.0, 1.0);

	// Écoulement : bruit étiré dans le sens du courant (vers l'aval = abscisse croissante).
	vec2 q = vec2(v_flow.x * 0.8, v_flow.y * 2.2) * ripple_scale;
	float t = TIME * flow_speed * ripple_scale * 0.012;
	float n1 = vnoise(q + vec2(-t, 0.0));
	float n2 = vnoise(q * 1.9 + vec2(-t * 1.6, 3.7));
	float nx = vnoise(q + vec2(-t + 0.07, 0.0)) - n1;
	float ny = vnoise(q + vec2(-t, 0.07)) - n1;
	float ripple_fade = 1.0 - smoothstep(0.08, 0.45, fwidth(q.x));

	// Bord de l'eau (en demi-largeurs d'eau) selon le régime.
	float water_edge = 1.0;
	float tide = 0.5 + 0.5 * sin(TIME * 6.2832 / tide_period + v_world.x * 0.7);
	water_edge = mix(water_edge, 1.0 - tide_amount * (1.0 - tide), v_flags.g);
	water_edge = mix(water_edge, 0.45, v_flags.a);
	float coarse = vnoise(vec2(v_flow.x * 40.0, v_flow.y * 40.0));
	float wobble = (vnoise(vec2(v_flow.x * 25.0, 1.3)) - 0.5) * 0.25;
	float water = 1.0 - smoothstep(water_edge + wobble - 0.06, water_edge + wobble + 0.02, edge);
	// Bancs de sable et grèves dans le lit divagant (Loire, Rhône, Rhin supérieur).
	float bars = v_flags.r * smoothstep(0.62, 0.7, vnoise(vec2(v_flow.x * 9.0, v_flow.y * 14.0)) * 0.7 + coarse * 0.3) * (1.0 - smoothstep(0.55, 0.95, edge));
	water *= 1.0 - bars;

	vec3 water_col = mix(shallow_color, deep_color, depth_t);
	water_col = mix(water_col, brackish_color, v_flags.g * 0.6);
	water_col *= 0.92 + 0.16 * n2;
	// Liseré clair au contact de la berge (discret : les facettes du relief le découpent).
	float contact = 1.0 - smoothstep(0.0, 0.004, thickness);
	water_col = mix(water_col, foam_color, contact * 0.08 + smoothstep(0.85, 1.0, edge / max(water_edge, 0.1)) * 0.12 * (0.5 + n2));

	// Berges dans la bande hors de l'eau.
	vec3 bank_col = gravel_color * (0.85 + 0.3 * coarse);
	float bank_alpha = 0.0;
	if (edge > water_edge - 0.1 || bars > 0.0) {
		float wet = 1.0 - smoothstep(water_edge, water_edge + 0.35, edge);
		vec3 sand = mix(sand_color, gravel_color, v_flags.a) * (0.85 + 0.3 * coarse);
		vec3 mud = mud_color * (0.8 + 0.35 * coarse) * mix(1.0, 0.7, wet);
		float reeds = smoothstep(0.35, 0.65, vnoise(v_world.xz * 900.0) * 0.6 + vnoise(v_world.xz * 2600.0) * 0.4);
		vec3 reed = mix(reed_color, reed_dry_color, vnoise(v_world.xz * 300.0)) * (0.7 + 0.5 * reeds);
		bank_col = sand;
		bank_col = mix(bank_col, mud, v_flags.g);
		bank_col = mix(bank_col, reed, v_flags.b * (0.6 + 0.4 * reeds));
		bank_col = mix(bank_col, sand * vec3(1.02, 1.0, 0.95), bars);
		float outer = 1.0 - smoothstep(v_band * 0.8, v_band, edge);
		bank_alpha = max(outer * (v_band > 1.02 ? 0.95 : 0.0), bars);
	}

	vec3 col = mix(bank_col, water_col, water);
	ALBEDO = col;
	float fine_ripples = ripple_fade;
	ROUGHNESS = mix(mix(0.9, 0.5, v_flags.g * (1.0 - water)), mix(0.42, roughness_value, fine_ripples), water);
	METALLIC = 0.0;
	SPECULAR = mix(0.2, mix(0.22, 0.5, fine_ripples), water);
	vec3 n = normalize(vec3(-nx * ripple_strength * 10.0 * ripple_fade * water, 1.0, -ny * ripple_strength * 10.0 * ripple_fade * water));
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
	float fresnel = pow(1.0 - clamp(dot(NORMAL, VIEW), 0.0, 1.0), 4.0);
	ALBEDO = mix(ALBEDO, sky_color, clamp(fresnel * sky_reflect + 0.12, 0.0, 0.8) * water);
	// SZ2b : nappe peu transparente même sur un lit peu creusé (à 0,75 les facettes du relief
	// se voyaient à travers l'eau : effet de voile sur la Loire et la Seine au palier site).
	float water_alpha = mix(shallow_alpha, 0.97, smoothstep(0.0, 0.35, depth_t));
	float alpha = max(water * water_alpha, bank_alpha * (1.0 - water));
	ALPHA = clamp(alpha * v_alpha * zone, 0.0, 1.0);
	if (debug_flat) {
		ALBEDO = mix(vec3(1.0, 0.0, 1.0), vec3(1.0, 1.0, 0.0), step(0.98, edge));
		ALPHA = 1.0;
	}
}
