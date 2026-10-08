// Lot AS5 : flammes de la carte de campagne (villes assiégées, hameaux brûlés, villages en ruine,
// bûchers), instanciées par MultiMesh avec les mêmes instances que les fumées d'incendie
// (life_effects.gd). Planche de flammes libre FA2 (Flame03, CC0, 8×8 images : RVB = chaleur,
// A = couverture), deux images mêlées lues avec TIME (phase par instance), chaleur convertie en
// couleur de corps noir émise en HDR, alpha prémultiplié (comme fire_flame.gdshader). Quad
// vertical face à la caméra autour de la verticale, incliné sous la rafale partagée
// (campaign_wind.gdshaderinc). Même convention d'instance que life_smoke.gdshader : origine au
// sol, levée dans MODEL_MATRIX[2].y, tout suit `prop_scale`. INSTANCE_CUSTOM.r = graine.
shader_type spatial;
render_mode blend_premul_alpha, depth_draw_never, cull_disabled, unshaded, shadows_disabled, fog_disabled;

#include "res://shaders/campaign_wind.gdshaderinc"

uniform sampler2D flipbook : filter_linear_mipmap, repeat_disable;
uniform float frames_h = 8.0;
uniform float frames_v = 8.0;
uniform float loops_per_s = 0.55;
uniform float flicker_rate = 8.0; // rad/s, mesuré sur vidéo (AS8d), data/fx/map_fire_wind.json
uniform float emission = 2.6;
uniform float opacity = 0.85;
uniform float lean = 0.22;
uniform float fade = 1.0;
uniform float prop_scale = 1.0;
uniform vec3 color_cold : source_color = vec3(0.35, 0.03, 0.0);
uniform vec3 color_mid : source_color = vec3(1.0, 0.32, 0.04);
uniform vec3 color_hot : source_color = vec3(1.0, 0.72, 0.28);
uniform vec3 color_core : source_color = vec3(1.0, 0.93, 0.78);

varying vec2 uv_a;
varying vec2 uv_b;
varying float frame_blend;
varying float seed_v;

vec2 frame_uv(vec2 uv, float frame) {
	float column = mod(frame, frames_h);
	float row = floor(frame / frames_h);
	return (vec2(column, row) + uv) / vec2(frames_h, frames_v);
}

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	float sx = length(MODEL_MATRIX[0].xyz) * prop_scale;
	float sy = length(MODEL_MATRIX[1].xyz) * prop_scale;
	origin.y += MODEL_MATRIX[2].y * prop_scale;
	seed_v = INSTANCE_CUSTOM.r;
	vec3 right = INV_VIEW_MATRIX[0].xyz;
	right = normalize(vec3(right.x, 0.0, right.z) + vec3(1e-4, 0.0, 0.0));
	vec3 up = normalize(mix(vec3(0.0, 1.0, 0.0), INV_VIEW_MATRIX[1].xyz, 0.55));
	float t = VERTEX.y;
	float gust = wind_gust(origin.xz);
	vec3 world = origin + right * VERTEX.x * sx + up * t * sy
		+ vec3(wind_gust_dir.x, 0.0, wind_gust_dir.y) * (t * t * sy * lean * (0.35 + gust));
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
	float total = frames_h * frames_v;
	float f = fract(TIME * loops_per_s + seed_v * 7.0) * total;
	float first = floor(f);
	frame_blend = f - first;
	vec2 uv = vec2(VERTEX.x + 0.5, 1.0 - VERTEX.y);
	uv_a = frame_uv(uv, first);
	uv_b = frame_uv(uv, mod(first + 1.0, total));
}

vec3 blackbody(float heat) {
	vec3 c = mix(color_cold, color_mid, smoothstep(0.0, 0.35, heat));
	c = mix(c, color_hot, smoothstep(0.35, 0.7, heat));
	return mix(c, color_core, smoothstep(0.7, 1.0, heat));
}

void fragment() {
	vec4 texel = mix(texture(flipbook, uv_a), texture(flipbook, uv_b), frame_blend);
	float heat = texel.r;
	float coverage = texel.a * fade;
	if (coverage < 0.01) {
		discard;
	}
	// Vacillement lent de l'intensité, propre à chaque foyer.
	float flicker = 0.85 + 0.15 * sin(TIME * flicker_rate + seed_v * 40.0);
	ALBEDO = blackbody(heat) * emission * (0.35 + heat * 1.2) * flicker * coverage;
	ALPHA = coverage * opacity;
}
