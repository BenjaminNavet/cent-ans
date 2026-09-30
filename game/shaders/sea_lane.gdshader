shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// Lot SL1 : routes maritimes tracées sur la mer, à l'encre, en tirets comme sur un portulan.
// Le maillage porte la ligne médiane, la perpendiculaire (NORMAL), la largeur monde (UV.x),
// le côté (UV.y) et l'abscisse curviligne monde (UV2.x). Largeur et tirets sont tenus en
// pixels écran : la route reste lisible et régulière à tout zoom.

uniform vec4 color : source_color = vec4(0.12, 0.20, 0.33, 0.85);
uniform float min_px = 1.6;
// Période des tirets et part pleine (0-1), en pixels écran.
uniform float dash_px = 14.0;
uniform float dash_fill = 0.62;

varying float v_side;
varying float v_arc_px;

void vertex() {
	vec3 view_pos = (MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz;
	float dist = max(length(view_pos), 0.01);
	float world_per_px = dist * 2.0 / (PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
	float half_w = 0.5 * max(UV.x, min_px * world_per_px);
	VERTEX += NORMAL * half_w;
	v_side = UV.y;
	v_arc_px = UV2.x / world_per_px;
}

void fragment() {
	float soft = 1.0 - smoothstep(0.55, 1.0, abs(v_side));
	float phase = fract(v_arc_px / max(dash_px, 1.0));
	float dash = 1.0 - smoothstep(dash_fill - 0.06, dash_fill, phase);
	ALBEDO = color.rgb;
	ALPHA = color.a * soft * dash;
}
