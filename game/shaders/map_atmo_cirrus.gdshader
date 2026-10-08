shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

// Lot ME5 : cirrus de haute couche (plan au-dessus des nuées) : fibres étirées dans le sens du vent,
// très fines, teintées par le soleil de la carte.

#include "res://shaders/map_atmosphere_common.gdshaderinc"

uniform float opacity = 0.2;
uniform float scale = 0.0035;
uniform float stretch = 5.0;
uniform float drift_share = 0.35;
uniform vec3 cirrus_color : source_color = vec3(0.96, 0.97, 1.0);

varying vec3 world_pos;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	if (atm_fade <= 0.001) {
		discard;
	}
	vec2 wind_dir = normalize(atm_wind + vec2(1e-4));
	vec2 side = vec2(-wind_dir.y, wind_dir.x);
	vec2 p = (world_pos.xz - atm_wind * TIME * drift_share) * scale;
	float u = dot(p, wind_dir);
	float v = dot(p, side);
	v += (atm_fbm(vec2(u * 0.7, v * 0.7) + 3.1) - 0.5) * 0.45;  // gauchissement léger
	float mass = smoothstep(0.52, 0.8, atm_fbm(vec2(u / stretch, v) * 1.1));
	mass *= smoothstep(0.45, 0.7, atm_fbm(vec2(u * 0.35, v * 0.5) + 11.0));  // plages discontinues
	float fibres = atm_noise(vec2(u / (stretch * 2.2), v * 22.0)) * 0.6 + atm_noise(vec2(u / (stretch * 1.1), v * 48.0)) * 0.4;
	float density = mass * smoothstep(0.45, 0.85, fibres);
	if (density <= 0.002) {
		discard;
	}
	float sun_side = clamp(dot(normalize(atm_sun_dir.xz + vec2(1e-4)), -wind_dir) * 0.5 + 0.5, 0.0, 1.0);
	vec3 col = cirrus_color * mix(vec3(1.0), atm_sun_color, 0.55) * (0.88 + 0.16 * sun_side);
	float a = density * opacity * atm_fade * atm_edge_fade(world_pos.xz)
			* atm_focus_clear(world_pos, CAMERA_POSITION_WORLD);
	ALBEDO = col;
	ALPHA = clamp(a, 0.0, 0.6);
}
