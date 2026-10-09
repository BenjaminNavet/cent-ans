shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

// Lot ME5 : bancs de brouillard du matin sur les fleuves. Cartes molles tournées autour de Y,
// bord fondu par le bruit, posées au ras de l'eau ; l'opacité suit la levée de la brume du matin
// (`opacity` posée par le script), INSTANCE_CUSTOM.x = tirage (présence selon la saison).

#include "res://shaders/map_atmosphere_common.gdshaderinc"

uniform float opacity = 0.34;
uniform float present = 1.0;
uniform vec3 fog_color : source_color = vec3(0.86, 0.87, 0.86);

varying vec3 world_pos;
varying vec2 card_uv;
varying float card_rand;

void vertex() {
	card_rand = INSTANCE_CUSTOM.x;
	vec3 base = MODEL_MATRIX[3].xyz;
	float w = length(MODEL_MATRIX[0].xyz);
	float h = length(MODEL_MATRIX[1].xyz);
	vec3 right = normalize(vec3(INV_VIEW_MATRIX[0].x, 0.0, INV_VIEW_MATRIX[0].z));
	world_pos = base + right * VERTEX.x * w + vec3(0.0, VERTEX.y * h, 0.0);
	card_uv = vec2(VERTEX.x * 2.0, VERTEX.y * 2.0 - 1.0 + 0.0);
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world_pos, 1.0);
}

void fragment() {
	if (card_rand > present || atm_fade <= 0.001) {
		discard;
	}
	vec2 seed = vec2(card_rand * 71.3, card_rand * 19.9);
	float n = atm_fbm(card_uv * 1.4 + seed + vec2(TIME * 0.02, 0.0));
	float shape = 1.0 - length(vec2(card_uv.x, card_uv.y * 1.6)) + (n - 0.5) * 0.8;
	float density = smoothstep(0.0, 0.5, shape);
	if (density <= 0.002) {
		discard;
	}
	float a = density * opacity * atm_fade * smoothstep(8.0, 60.0, length(VERTEX))
			* atm_focus_clear(world_pos, CAMERA_POSITION_WORLD);
	ALBEDO = fog_color * mix(vec3(1.0), atm_sun_color, 0.35);
	ALPHA = clamp(a, 0.0, 0.6);
}
