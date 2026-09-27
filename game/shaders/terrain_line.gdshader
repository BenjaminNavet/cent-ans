shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// Traits posés sur le terrain (fleuves, côte) : le maillage porte la ligne médiane, la
// direction perpendiculaire (NORMAL) et la largeur monde (UV.x). Le vertex shader élargit
// le ruban à au moins `min_px` pixels écran ; un cours d'eau plus fin que `min_px` à l'écran
// s'estompe (alpha) au lieu de s'épaissir, d'où des fleuves fins et nets à tout zoom.

uniform vec4 color : source_color = vec4(0.10, 0.25, 0.33, 0.85);
uniform float min_px = 1.2;
// Proportion de l'estompage des traits sous-pixel (0 = aucun, 1 = alpha ∝ largeur réelle).
uniform float thin_fade = 0.7;
// Assombrissement / éclaircissement au centre du trait (reflet de l'eau).
uniform float center_highlight = 0.12;
// CV3-0 (#6) : liseré sombre sur le bord du trait (contraste sur tout type de terrain), en
// fraction de la demi-largeur ; 0 = désactivé (fleuves, côte : inchangés).
uniform vec4 casing_color : source_color = vec4(0.0, 0.0, 0.0, 0.9);
uniform float casing_width = 0.0;

varying float v_alpha;
varying float v_side;

void vertex() {
	vec3 view_pos = (MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz;
	float dist = max(length(view_pos), 0.01);
	float world_per_px = dist * 2.0 / (PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
	float true_px = UV.x / world_per_px;
	float half_w = 0.5 * max(UV.x, min_px * world_per_px);
	VERTEX += NORMAL * half_w;
	v_alpha = mix(1.0, clamp(true_px / min_px, 0.25, 1.0), thin_fade);
	v_side = UV.y;
}

void fragment() {
	float edge = abs(v_side);
	float soft = 1.0 - smoothstep(0.55, 1.0, edge);
	vec3 albedo = color.rgb * (1.0 + center_highlight * (1.0 - edge));
	if (casing_width > 0.0) {
		float band = smoothstep(1.0 - casing_width - 0.08, 1.0 - casing_width, edge);
		albedo = mix(albedo, casing_color.rgb, band * casing_color.a);
	}
	ALBEDO = albedo;
	ALPHA = color.a * v_alpha * soft;
}
