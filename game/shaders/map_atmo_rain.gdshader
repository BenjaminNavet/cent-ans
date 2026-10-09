shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

// Lot ME5 : rideaux de pluie lointains (province en pluie ou orage au cœur). MultiMesh de quads
// verticaux tournés autour de Y vers la caméra : pied = translation de l'instance, taille =
// échelle, INSTANCE_CUSTOM.x = tirage, .y = orage (0..1). Voile plus dense sous la base de la
// nuée, estompé vers le sol, traînées obliques qui défilent.

#include "res://shaders/map_atmosphere_common.gdshaderinc"

uniform float opacity = 0.3;
uniform float slant = 0.18;
uniform float speed = 1.6;
uniform vec3 rain_tint : source_color = vec3(0.62, 0.67, 0.76);
uniform vec3 storm_tint : source_color = vec3(0.34, 0.36, 0.43);

varying vec3 world_pos;
varying vec2 curtain_uv;   // x : -0.5..0.5 sur la largeur, y : 0 (sol) .. 1 (base de nuée)
varying vec2 curtain_data; // tirage, orage
varying float width_units;

void vertex() {
	curtain_data = INSTANCE_CUSTOM.xy;
	vec3 base = MODEL_MATRIX[3].xyz;
	float w = length(MODEL_MATRIX[0].xyz);
	float h = length(MODEL_MATRIX[1].xyz);
	width_units = w;
	vec3 right = normalize(vec3(INV_VIEW_MATRIX[0].x, 0.0, INV_VIEW_MATRIX[0].z));
	vec3 lean = normalize(vec3(atm_wind.x, 0.0, atm_wind.y) + vec3(1e-4)) * slant * VERTEX.y * h;
	world_pos = base + right * VERTEX.x * w + vec3(0.0, VERTEX.y * h, 0.0) + lean;
	curtain_uv = vec2(VERTEX.x, VERTEX.y);
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world_pos, 1.0);
}

void fragment() {
	if (atm_fade <= 0.001) {
		discard;
	}
	float seed = curtain_data.x * 53.0;
	float column = curtain_uv.x * width_units * 1.3 + seed;
	float id = floor(column);
	float line = 1.0 - abs(fract(column) - 0.5) * 2.0;
	float streak_rand = atm_hash(vec2(id, seed));
	float flow = curtain_uv.y * 3.0 * (0.8 + streak_rand) + TIME * speed * (0.7 + streak_rand * 0.6) + streak_rand * 9.0;
	float streak = smoothstep(0.35, 1.0, line) * smoothstep(0.45, 0.9, atm_noise(vec2(id * 0.7, flow * 1.6)));
	// Voile : nappes de pluie plus ou moins denses qui bougent lentement.
	float veil = 0.35 + 0.65 * atm_fbm(vec2(curtain_uv.x * 3.0 + seed, curtain_uv.y * 2.0 - TIME * 0.12));
	float vertical = smoothstep(0.0, 0.85, curtain_uv.y) * 0.75 + 0.25;
	float top_fade = 1.0 - smoothstep(0.88, 1.0, curtain_uv.y);
	float side = smoothstep(0.0, 0.3, 0.5 - abs(curtain_uv.x));
	float density = (veil * 0.55 + streak * 0.6) * vertical * top_fade * side;
	float a = density * opacity * atm_fade * atm_focus_clear(world_pos, CAMERA_POSITION_WORLD);
	if (a <= 0.003) {
		discard;
	}
	ALBEDO = mix(rain_tint, storm_tint, curtain_data.y) * mix(vec3(1.0), atm_sun_color, 0.25);
	ALPHA = clamp(a, 0.0, 0.7);
}
