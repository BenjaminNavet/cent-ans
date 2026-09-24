// Lot CV1 : surcouche des maquettes de colonies et des hameaux (`material_overlay`) :
// - neige sur les toits et le haut des murs l'hiver (poids global `campaign_season`) ;
// - suie et toits effondrés des villages en ruine (dévastation), par instance (`ruin`).
shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_back, specular_disabled;

global uniform vec4 campaign_season;
instance uniform float ruin = 0.0;
uniform vec3 snow_color : source_color = vec3(0.86, 0.88, 0.92);
uniform vec3 soot_color : source_color = vec3(0.035, 0.03, 0.028);

varying vec3 v_world;
varying vec3 v_world_normal;

float hash31(vec3 p) {
	p = fract(p * 0.3183099 + 0.1);
	p *= 17.0;
	return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float noise3(vec3 p) {
	vec3 i = floor(p);
	vec3 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(
		mix(mix(hash31(i), hash31(i + vec3(1, 0, 0)), f.x), mix(hash31(i + vec3(0, 1, 0)), hash31(i + vec3(1, 1, 0)), f.x), f.y),
		mix(mix(hash31(i + vec3(0, 0, 1)), hash31(i + vec3(1, 0, 1)), f.x), mix(hash31(i + vec3(0, 1, 1)), hash31(i + vec3(1, 1, 1)), f.x), f.y),
		f.z);
}

void vertex() {
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	float up = v_world_normal.y;
	float n = noise3(v_world * 2.5) * 0.6 + noise3(v_world * 7.0) * 0.4;
	float snow = campaign_season.w * smoothstep(0.25, 0.6, up) * smoothstep(0.25, 0.45, n + 0.15);
	float soot = ruin * smoothstep(0.35, 0.6, n + ruin * 0.3);
	vec3 col = mix(snow_color, soot_color, soot > snow ? 1.0 : 0.0);
	float alpha = max(snow * 0.9, soot * 0.92);
	if (alpha < 0.01) {
		discard;
	}
	ALBEDO = col;
	ROUGHNESS = mix(0.8, 1.0, soot);
	ALPHA = alpha;
}
