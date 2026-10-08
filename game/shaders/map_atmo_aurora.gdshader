shader_type spatial;
render_mode blend_add, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

// Lot ME5 : aurore boréale, rideau vertical le long du bord nord de la carte (hiver, soirée).
// Rais verticaux qui ondulent, pied vif et net, sommet qui s'éteint, vert -> bleu -> violet.

#include "res://shaders/map_atmosphere_common.gdshaderinc"

uniform float strength = 0.0;
uniform float speed = 0.05;
uniform vec3 color_low : source_color = vec3(0.2, 0.95, 0.55);
uniform vec3 color_mid : source_color = vec3(0.35, 0.55, 1.0);
uniform vec3 color_high : source_color = vec3(0.75, 0.3, 0.9);

varying vec2 quad_uv;  // x : 0..1 d'ouest en est, y : 0 (pied) .. 1 (sommet)

void vertex() {
	quad_uv = vec2(VERTEX.x + 0.5, VERTEX.y + 0.5);
}

void fragment() {
	if (strength <= 0.001 || atm_fade <= 0.001) {
		discard;
	}
	float t = TIME * speed;
	float wave = atm_fbm(vec2(quad_uv.x * 4.0 + t * 3.0, t)) - 0.5;
	float foot = 0.1 + wave * 0.18;
	float y = quad_uv.y - foot;
	float rays = atm_noise(vec2(quad_uv.x * 70.0 + wave * 12.0, t * 2.0));
	rays = mix(0.35, 1.0, smoothstep(0.2, 0.9, rays));
	float band = smoothstep(0.0, 0.05, y) * pow(clamp(1.0 - y * 1.25, 0.0, 1.0), 1.6);
	float patches = smoothstep(0.25, 0.7, atm_fbm(vec2(quad_uv.x * 3.0 - t * 4.0, 1.7)));
	float k = band * rays * patches * smoothstep(0.0, 0.12, quad_uv.x) * smoothstep(0.0, 0.12, 1.0 - quad_uv.x);
	vec3 col = mix(color_low, color_mid, smoothstep(0.1, 0.5, y));
	col = mix(col, color_high, smoothstep(0.45, 0.9, y));
	ALBEDO = col * k * strength * atm_fade;
	ALPHA = 1.0;
}
