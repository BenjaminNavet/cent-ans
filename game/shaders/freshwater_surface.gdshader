shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, specular_schlick_ggx;

// Lot DN-ME4 : bassins de salines (COLOR par sommet), écume de torrent (ruban, UV.x le long du
// cours) et gué. `mode` : 0 salin, 1 écume, 2 gué. Rapproché de la caméra (la surface affichée est
// plus grossière au loin).
uniform int mode = 0;
uniform float fade_start = 950.0;
uniform float fade_end = 1200.0;
uniform float bias_factor = 0.004;
uniform float flow_speed = 0.6;
uniform vec3 foam_color : source_color = vec3(0.88, 0.92, 0.94);

varying float v_dist;

float hash12(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float vnoise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash12(i), hash12(i + vec2(1, 0)), f.x), mix(hash12(i + vec2(0, 1)), hash12(i + vec2(1, 1)), f.x), f.y);
}

void vertex() {
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 to_cam = CAMERA_POSITION_WORLD - world;
	v_dist = length(to_cam);
	VERTEX += (inverse(mat3(MODEL_MATRIX)) * normalize(to_cam)) * v_dist * bias_factor;
}

void fragment() {
	float fade = 1.0 - smoothstep(fade_start, fade_end, v_dist);
	if (mode == 0) {
		ALBEDO = COLOR.rgb;
		ROUGHNESS = 0.2;
		SPECULAR = 0.5;
		ALPHA = COLOR.a * fade;
	} else if (mode == 1) {
		float across = abs(UV.y - 0.5) * 2.0;
		float n = vnoise(vec2(UV.x * 5.0 - TIME * flow_speed, UV.y * 6.0)) * 0.6 + vnoise(vec2(UV.x * 13.0 - TIME * flow_speed * 1.7, UV.y * 14.0)) * 0.4;
		float a = smoothstep(0.35, 0.7, n) * (1.0 - smoothstep(0.55, 1.0, across)) * COLOR.a;
		ALBEDO = mix(foam_color * 0.75, foam_color, n);
		ROUGHNESS = 0.3;
		ALPHA = a * fade;
	} else {
		float across = abs(UV.y - 0.5) * 2.0;
		float stones = step(0.62, vnoise(UV * vec2(40.0, 14.0)));
		ALBEDO = mix(vec3(0.28, 0.33, 0.34), vec3(0.45, 0.43, 0.38), stones);
		ROUGHNESS = mix(0.15, 0.8, stones);
		ALPHA = (1.0 - smoothstep(0.7, 1.0, across)) * 0.8 * fade;
	}
}
