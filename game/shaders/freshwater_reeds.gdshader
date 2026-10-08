shader_type spatial;
render_mode cull_disabled, depth_prepass_alpha, specular_schlick_ggx;

// Lot DN-ME4 : touffes de roseaux en croix (deux plans) posées en MultiMesh sur les zones humides.
// Lame procédurale tant que la carte de roseaux générée (card_reed_*) n'existe pas ; `use_card`
// bascule sur la texture RGBA. INSTANCE_CUSTOM.rgb : teinte, .a : phase (vent, variation).
uniform float fw_scale = 1.0;       // grossissement selon la distance caméra
uniform float fw_alpha = 1.0;       // fondu par tramage (palier près / patch / lointain)
uniform bool use_card = false;
uniform sampler2D card : source_color, filter_linear_mipmap, repeat_disable;
uniform float wind = 0.25;

varying vec4 v_tint;
varying float v_height;

float hash12(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

void vertex() {
	v_tint = INSTANCE_CUSTOM;
	v_height = UV.y;
	VERTEX *= fw_scale;
	float sway = sin(TIME * 1.3 + INSTANCE_CUSTOM.a * 6.283) * wind * (1.0 - UV.y) * 0.12 * fw_scale;
	VERTEX.x += sway;
}

void fragment() {
	float up = 1.0 - UV.y;  // 0 au pied, 1 en haut
	float alpha;
	vec3 col;
	if (use_card) {
		vec4 c = texture(card, UV);
		alpha = c.a;
		col = c.rgb * v_tint.rgb * 1.6;
	} else {
		float blades = 0.0;
		for (int i = 0; i < 8; i++) {
			float fi = float(i);
			float x0 = fract(sin(fi * 12.9898 + v_tint.a * 78.233) * 43758.5453);
			float h = 0.55 + 0.45 * fract(sin(fi * 4.1 + v_tint.a * 9.7) * 24634.6);
			float lean = (fract(sin(fi * 7.7) * 913.1) - 0.5) * 0.18 * up;
			float d = abs(UV.x - x0 - lean);
			blades = max(blades, step(d, 0.07 * (1.0 - up * 0.8)) * step(up, h));
		}
		alpha = blades;
		col = v_tint.rgb * mix(0.55, 1.15, up) * (0.9 + 0.2 * hash12(vec2(UV.x * 40.0, v_tint.a * 50.0)));
	}
	float dither = hash12(FRAGCOORD.xy);
	if (alpha < 0.5 || dither > fw_alpha) {
		discard;
	}
	ALBEDO = col;
	ROUGHNESS = 0.85;
	SPECULAR = 0.15;
}
