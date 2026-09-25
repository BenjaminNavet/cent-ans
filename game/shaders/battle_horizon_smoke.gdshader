// Fumée lointaine d'un village (lot EP2) : panneau tourné vers la caméra autour de l'axe
// vertical, colonne qui s'élargit et se couche avec la hauteur, bruit qui monte. Le brouillard
// de la scène l'estompe comme le reste de l'horizon.
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, shadows_disabled;

uniform vec3 smoke_color : source_color = vec3(0.62, 0.62, 0.64);
uniform vec3 light_tint : source_color = vec3(1.0);
instance uniform float seed = 0.0;

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}

void vertex() {
	// Billboard sur l'axe Y.
	MODELVIEW_MATRIX = VIEW_MATRIX * mat4(INV_VIEW_MATRIX[0], MODEL_MATRIX[1], vec4(normalize(cross(INV_VIEW_MATRIX[0].xyz, MODEL_MATRIX[1].xyz)), 0.0), MODEL_MATRIX[3]);
}

void fragment() {
	float h = 1.0 - UV.y;  // 0 en bas, 1 en haut
	float drift = h * h * 0.35;
	float x = (UV.x - 0.5 - drift) / mix(0.12, 0.5, h);
	float t = TIME * 0.05 + seed;
	float n = noise(vec2(UV.x * 5.0 + seed, UV.y * 7.0 + t * 3.0)) * 0.6 + noise(vec2(UV.x * 13.0, UV.y * 17.0 + t * 6.0)) * 0.4;
	float body = (1.0 - smoothstep(0.3, 1.0, abs(x))) * smoothstep(0.0, 0.06, h) * (1.0 - smoothstep(0.45, 1.0, h));
	ALBEDO = smoke_color * light_tint * mix(0.85, 1.05, n);
	ALPHA = clamp(body * (n * 0.9 + 0.1) * 0.55, 0.0, 1.0);
}
