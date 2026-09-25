// Lot EP6 : flamme d'un feu de camp (quad tourné vers la caméra, additif, vacillement animé).
// INSTANCE_CUSTOM.r : phase propre à chaque feu.
shader_type spatial;
render_mode blend_add, unshaded, cull_disabled, depth_draw_never, shadows_disabled;

varying float phase;

void vertex() {
	phase = INSTANCE_CUSTOM.r * 37.0;
	// Face à la caméra autour de l'axe vertical.
	vec3 cam_right = normalize(vec3(INV_VIEW_MATRIX[0].x, 0.0, INV_VIEW_MATRIX[0].z));
	vec3 origin = MODEL_MATRIX[3].xyz;
	float sway = sin(TIME * 7.0 + phase) * 0.08 * UV.y;
	vec3 world = origin + cam_right * (VERTEX.x + sway) + vec3(0.0, VERTEX.y * (0.85 + 0.2 * sin(TIME * 5.3 + phase)), 0.0);
	VERTEX = (inverse(MODEL_MATRIX) * vec4(world, 1.0)).xyz;
}

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

void fragment() {
	vec2 uv = UV;
	float h = 1.0 - uv.y; // 0 en bas, 1 en haut
	float n = noise(vec2(uv.x * 4.0, h * 3.0 - TIME * 3.5 + phase));
	float body = 1.0 - smoothstep(0.0, 0.5 - 0.35 * h, abs(uv.x - 0.5) + (n - 0.5) * 0.25);
	float fade = (1.0 - smoothstep(0.35, 0.95, h + n * 0.25)) * smoothstep(0.0, 0.08, h);
	float a = clamp(body * fade, 0.0, 1.0);
	vec3 hot = vec3(1.0, 0.85, 0.45);
	vec3 cool = vec3(0.95, 0.32, 0.05);
	ALBEDO = mix(cool, hot, clamp(1.0 - h * 1.4 + n * 0.2, 0.0, 1.0)) * a * 2.2;
}
