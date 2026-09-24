shader_type spatial;
// A1-02 : zone de déploiement discrète, façon Warhammer III. Un liseré lumineux net sur le
// contour, un halo qui se fond vers l'intérieur, des hachures diagonales dans une bande de bordure
// et un voile presque invisible au centre. Les hachures et le halo s'estompent quand la caméra
// s'éloigne ; le liseré garde une largeur minimale à l'écran pour rester lisible de loin.
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, shadows_disabled, fog_disabled;

uniform vec2 rect_min = vec2(0.0);
uniform vec2 rect_max = vec2(100.0);
uniform vec4 edge_color : source_color = vec4(1.0, 0.86, 0.42, 1.0);
uniform float line_width = 0.9;          // mètres
uniform float glow_width = 7.0;          // mètres, décroissance du halo intérieur
uniform float hatch_band = 22.0;         // mètres de bande hachurée depuis le bord
uniform float hatch_period = 5.0;        // mètres entre deux hachures
uniform float interior_alpha = 0.03;
uniform float fade_near = 140.0;         // distance caméra où le détail commence à s'estomper
uniform float fade_far = 700.0;

varying vec2 world_xz;

void vertex() {
	world_xz = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xz;
}

void fragment() {
	vec2 inner = min(world_xz - rect_min, rect_max - world_xz);
	float d = max(min(inner.x, inner.y), 0.0);
	float cam_dist = length(VERTEX);
	float detail = 1.0 - smoothstep(fade_near, fade_far, cam_dist);

	// Liseré : au moins ~1,6 pixel à l'écran, quelle que soit la distance.
	float px = max(fwidth(d), 1e-4);
	float w = max(line_width, px * 1.6);
	float line = 1.0 - smoothstep(w - px, w + px, d);

	float glow = exp(-d / glow_width) * mix(0.18, 0.42, detail);

	// Hachures diagonales, anticrénelées ; limitées à la bande de bordure.
	float s = (world_xz.x + world_xz.y) / hatch_period;
	float sw = max(fwidth(s), 1e-4);
	float f = abs(fract(s) - 0.5);
	float stripe = 1.0 - smoothstep(0.08, 0.08 + sw * 1.5, f);
	// Hachures illisibles quand elles deviennent plus fines qu'un pixel : fondu anti-moiré.
	float moire = 1.0 - smoothstep(0.25, 0.5, sw);
	float band = 1.0 - smoothstep(hatch_band * 0.35, hatch_band, d);
	float hatch = stripe * band * 0.28 * detail * moire;

	float alpha = max(max(line * 0.95, glow), max(hatch, interior_alpha));
	ALBEDO = edge_color.rgb;
	EMISSION = edge_color.rgb * line * 0.6;
	ALPHA = clamp(alpha, 0.0, 1.0) * edge_color.a;
}
