// Braises (lot V3, A1-13) : petites étincelles incandescentes qui montent du foyer, dérivent au
// vent et s'éteignent (orange vif → rouge sombre). Émission HDR (halo), mélange additif.
// Particules de `GPUParticles3D` : INSTANCE_CUSTOM.y = âge (0-1).
shader_type spatial;
render_mode blend_add, unshaded, cull_disabled, depth_draw_never, shadows_disabled, fog_disabled;

uniform vec3 hot : source_color = vec3(1.0, 0.62, 0.2);
uniform vec3 cold : source_color = vec3(0.6, 0.08, 0.0);
uniform float energy = 6.0;

varying float age;

void vertex() {
	mat4 world = mat4(normalize(INV_VIEW_MATRIX[0]), normalize(INV_VIEW_MATRIX[1]), normalize(INV_VIEW_MATRIX[2]), MODEL_MATRIX[3]);
	MODELVIEW_MATRIX = VIEW_MATRIX * world * mat4(
		vec4(length(MODEL_MATRIX[0].xyz), 0.0, 0.0, 0.0),
		vec4(0.0, length(MODEL_MATRIX[1].xyz), 0.0, 0.0),
		vec4(0.0, 0.0, length(MODEL_MATRIX[2].xyz), 0.0),
		vec4(0.0, 0.0, 0.0, 1.0));
	age = clamp(INSTANCE_CUSTOM.y, 0.0, 1.0);
}

void fragment() {
	float r = length(UV - vec2(0.5)) * 2.0;
	float spot = pow(clamp(1.0 - r, 0.0, 1.0), 2.0);
	// Scintillement : chaque braise pulse à sa propre phase (angle de particule absent : l'âge suffit).
	float twinkle = 0.7 + 0.3 * sin(age * 40.0 + COLOR.r * 20.0);
	float life = 1.0 - smoothstep(0.5, 1.0, age);
	ALBEDO = mix(hot, cold, age) * energy * spot * life * twinkle * COLOR.a;
}
