shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_disabled;

// Mer : plan à Y = 0, couleur unie légèrement animée (vaguelettes de normale).

uniform vec3 color : source_color = vec3(0.16, 0.38, 0.48);
uniform float alpha : hint_range(0.0, 1.0) = 0.82;
uniform float wave_scale = 0.06;
uniform float wave_speed = 0.6;

varying vec3 world_pos;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	float t = TIME * wave_speed;
	float w1 = sin(world_pos.x * wave_scale + t) * cos(world_pos.z * wave_scale * 1.3 - t * 0.7);
	float w2 = sin((world_pos.x + world_pos.z) * wave_scale * 2.1 - t * 1.4);
	float w = w1 * 0.6 + w2 * 0.4;
	// Les vaguelettes s'estompent avec la distance (évite le moiré en vue rasante).
	float fade = clamp(1.0 - (-VERTEX.z) / 500.0, 0.0, 1.0) * 0.06;
	vec3 n = normalize(vec3(w * fade, 1.0, -w * fade * 0.8));
	ALBEDO = color + vec3(w * 0.02 * fade / 0.06);
	ALPHA = alpha;
	ROUGHNESS = 0.25;
	SPECULAR = 0.55;
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}
