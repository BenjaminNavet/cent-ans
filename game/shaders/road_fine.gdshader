shader_type spatial;
render_mode blend_mix, cull_disabled, depth_draw_never, specular_disabled, world_vertex_coords;

// Routes drapées de près (lot ZG5b, ADR 0036), maillées par FineRibbonJob / FineGeoLayer.
// Sommets sur l'axe : VERTEX.y = hauteur de la route en MÈTRES (surface des pages chargées ou
// `z` de la tuile, le plus haut des deux : remblais, tabliers) × height_scale ; NORMAL =
// perpendiculaire signée ; UV = (largeur réelle en unités monde, côté ±1) ; UV2 = (abscisse,
// largeur en m) ; COLOR = (principale, chaussée en zone humide, pavée, calculée).
// Largeur réelle (3 à 6 m), jamais moins de `min_px` pixels écran : chemins de terre à ornières,
// voies principales plus larges et plus claires, pavé près des villes, chaussée surélevée
// (talus sombres) dans les marais.

// ZG4 : unités monde par mètre ; ZG8 : hauteur affichée (relief local exagéré).
#include "res://shaders/campaign_relief.gdshaderinc"
#define height_scale campaign_vertical_scale
uniform vec4 fine_zone = vec4(0.0, 0.0, -1.0, 0.0);
uniform float fine_weight = 1.0;
uniform bool debug_flat = false;  // `--fine-debug` : routes en jaune opaque
uniform float min_px = 2.2;
uniform float main_min_px = 3.2;
uniform float depth_bias = 0.0012;
uniform vec3 dirt_color : source_color = vec3(0.47, 0.38, 0.25);
uniform vec3 main_color : source_color = vec3(0.60, 0.51, 0.36);
uniform vec3 rut_color : source_color = vec3(0.32, 0.25, 0.16);
uniform vec3 paved_color : source_color = vec3(0.52, 0.50, 0.46);
uniform vec3 joint_color : source_color = vec3(0.30, 0.28, 0.25);
uniform vec3 grass_edge : source_color = vec3(0.30, 0.31, 0.16);
uniform vec3 bank_color : source_color = vec3(0.20, 0.22, 0.12);
uniform float meters_per_unit = 719.0;

varying float v_side;
varying float v_alpha;
varying vec2 v_road;      // (abscisse en m, écart à l'axe en m)
varying vec4 v_kind;
varying vec2 v_xz;

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

void vertex() {
	vec3 world = vec3(VERTEX.x, campaign_display_height(VERTEX.y, VERTEX.xz), VERTEX.z);
	vec3 to_cam = CAMERA_POSITION_WORLD - world;
	float dist = max(length(to_cam), 0.01);
	float world_per_px = dist * 2.0 / abs(PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
	float floor_px = mix(min_px, main_min_px, COLOR.r);
	// Chaussée : talus de part et d'autre (+60 % de largeur).
	float real_half = 0.5 * UV.x * (1.0 + 0.6 * COLOR.g);
	float half_w = max(real_half, floor_px * world_per_px * 0.5);
	world += NORMAL * half_w;
	world += to_cam / dist * min(depth_bias * dist, 0.4);
	VERTEX = world;
	v_alpha = mix(0.55, 1.0, clamp(UV.x / world_per_px / floor_px, 0.0, 1.0));
	v_side = UV.y;
	v_road = vec2(UV2.x * meters_per_unit, UV.y * half_w * meters_per_unit);
	v_kind = COLOR;
	v_xz = world.xz;
	NORMAL = vec3(0.0, 1.0, 0.0);
}

void fragment() {
	float zone = fine_weight * (fine_zone.z > 0.0 ? 1.0 - smoothstep(fine_zone.z * 0.85, fine_zone.z, distance(v_xz, fine_zone.xy)) : 0.0);
	if (zone <= 0.001) {
		discard;
	}
	float side = abs(v_side);
	float n = noise(vec2(v_road.x * 0.35, v_road.y * 0.8));
	// Chaussée surélevée : bande centrale de roulement, talus sombres sur les bords.
	float causeway_bank = v_kind.g * smoothstep(0.55, 0.7, side);
	float s = mix(side, side / 0.62, v_kind.g);
	float ruts = smoothstep(0.1, 0.0, abs(s - 0.42)) * 0.6 * (1.0 - v_kind.b);
	vec3 base = mix(dirt_color, main_color, v_kind.r);
	vec3 col = mix(base, rut_color, ruts);
	// Pavé (villes) : joints d'un pavé de ~0,6 m, estompés quand ils passent sous le pixel.
	vec2 cell = v_road / 0.6;
	float joints = max(smoothstep(0.42, 0.5, abs(fract(cell.x + 0.5 * floor(cell.y)) - 0.5)), smoothstep(0.42, 0.5, abs(fract(cell.y) - 0.5)));
	joints *= 1.0 - smoothstep(0.3, 0.8, fwidth(cell.x));
	vec3 paved = mix(paved_color * (0.9 + 0.2 * hash(floor(cell))), joint_color, joints);
	col = mix(col, paved, v_kind.b);
	col = mix(col, grass_edge, smoothstep(0.6, 1.0, s + (n - 0.5) * 0.3) * (1.0 - v_kind.b * 0.8));
	col = mix(col, bank_color, causeway_bank);
	col *= 0.9 + 0.2 * n;
	ALBEDO = col;
	ROUGHNESS = mix(1.0, 0.85, v_kind.b);
	float edge = 1.0 - smoothstep(0.75, 1.0, side + (n - 0.5) * 0.2);
	ALPHA = edge * v_alpha * zone;
	if (debug_flat) {
		ALBEDO = vec3(1.0, 0.9, 0.0);
		ALPHA = 1.0;
	}
}
