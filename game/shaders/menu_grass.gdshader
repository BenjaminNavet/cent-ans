shader_type spatial;
render_mode cull_disabled, diffuse_burley;

// Lot MM1 — touffes d'herbe du premier plan du décor de menu (quads croisés, texture de la
// bataille `grass_clump.png`), balancées par le vent. Teinte par instance (COLOR).

uniform sampler2D grass_texture : source_color, filter_linear_mipmap, repeat_disable;
uniform vec3 tint : source_color = vec3(0.55, 0.62, 0.36);

varying vec3 v_color;

void vertex() {
	v_color = COLOR.rgb;
	float top = 1.0 - UV.y;
	vec3 origin = (MODEL_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
	float phase = dot(origin.xz, vec2(0.21, 0.17));
	float sway = sin(TIME * 1.7 + phase) * 0.08 + sin(TIME * 3.1 + phase * 1.9) * 0.03;
	VERTEX.x += sway * top * top;
	VERTEX.z += sway * 0.6 * top * top;
}

void fragment() {
	vec4 texel = texture(grass_texture, UV);
	ALBEDO = texel.rgb * tint * v_color * 1.6;
	ALPHA = texel.a;
	ALPHA_SCISSOR_THRESHOLD = 0.45;
	ROUGHNESS = 0.9;
	NORMAL = normalize(mix(NORMAL, (VIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz, 0.6));
}
