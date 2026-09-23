// Houppiers des arbres et buissons (lot V4) : cartes alpha (découpe), teinte par instance
// (couleur du MultiMesh, qui module COLOR), balancement au vent croissant avec la hauteur,
// frémissement des feuilles, lumière transmise à contre-jour.
shader_type spatial;
render_mode cull_disabled, diffuse_burley, depth_draw_opaque;

uniform sampler2D leaves : source_color, filter_linear_mipmap, repeat_disable;
uniform float alpha_cut = 0.5;
uniform float wind_strength = 1.0;
uniform float sway_height = 12.0; // hauteur (m) où le balancement est plein
uniform vec3 tint : source_color = vec3(0.92, 0.98, 0.85);

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	float h = clamp(VERTEX.y / sway_height, 0.0, 1.0);
	float t = TIME * 1.15 + origin.x * 0.061 + origin.z * 0.047;
	float sway = (sin(t) * 0.6 + sin(t * 2.3 + 1.7) * 0.25) * h * h * wind_strength;
	VERTEX.x += sway * 0.35;
	VERTEX.z += sway * 0.22;
	VERTEX += NORMAL * sin(TIME * 4.0 + VERTEX.x * 3.1 + VERTEX.y * 2.3) * 0.04 * wind_strength;
}

void fragment() {
	vec4 tex = texture(leaves, UV);
	ALBEDO = tex.rgb * tint * COLOR.rgb;
	ALPHA = tex.a;
	ALPHA_SCISSOR_THRESHOLD = alpha_cut;
	ROUGHNESS = 0.8;
	SPECULAR = 0.25;
	BACKLIGHT = vec3(0.35, 0.4, 0.15);
}
