shader_type spatial;
// Lot M4 : bulle des cases atteignables ce tour (teinte parchemin). Le
// maillage épouse le relief ; `mask` porte une case de la grille de navigation par texel
// (R = atteignable ce tour, G = coût / budget, B = atteignable d'ici la fin du tour suivant,
// lot CV3-5). Chaque masque est adouci par 9 prises bilinéaires puis seuillé à 0,5 : contour
// arrondi, liseré doré anticrénelé d'au moins ~1,5 pixel à l'écran, ombre d'encre à l'extérieur,
// voile léger qui se renforce vers le bord de portée. Lot CV3-5 : second ton, plus pâle, pour
// le tour suivant (voile à peine teinté et liseré fin et estompé, sans ombre).
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, shadows_disabled, fog_disabled;

uniform sampler2D mask : filter_linear_mipmap, repeat_disable;
uniform vec2 origin = vec2(0.0);
uniform vec2 extent = vec2(1.0);
uniform vec2 texel = vec2(0.01);
uniform vec4 fill_color : source_color = vec4(0.98, 0.88, 0.58, 0.05);
uniform vec4 rim_fill_color : source_color = vec4(1.0, 0.82, 0.40, 0.20);
uniform vec4 edge_color : source_color = vec4(1.0, 0.86, 0.42, 1.0);
uniform vec4 shadow_color : source_color = vec4(0.16, 0.09, 0.03, 0.55);
uniform float edge_px = 1.6;
// Lot CV3-5 : anneau du tour suivant.
uniform bool show_next = true;
uniform vec4 next_fill_color : source_color = vec4(0.93, 0.90, 0.80, 0.07);
uniform vec4 next_edge_color : source_color = vec4(0.95, 0.90, 0.74, 0.55);
uniform float next_edge_px = 1.1;
// Décalage vers la caméra (fraction de la distance) : évite que le relief ne perce la bulle.
uniform float depth_pull = 0.004;

varying vec2 world_xz;

void vertex() {
	world_xz = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xz;
	vec3 view = (MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz;
	VERTEX = (inverse(MODELVIEW_MATRIX) * vec4(view * (1.0 - depth_pull), 1.0)).xyz;
}

// Masque adouci : `channel` choisit le canal (R ce tour, B tour suivant).
float field(vec2 uv, vec3 channel) {
	float s = 0.0;
	for (int dy = -1; dy <= 1; dy++) {
		for (int dx = -1; dx <= 1; dx++) {
			s += dot(texture(mask, uv + vec2(float(dx), float(dy)) * texel * 0.8).rgb, channel);
		}
	}
	return s / 9.0;
}

void fragment() {
	vec2 uv = (world_xz - origin) / extent;
	float f = field(uv, vec3(1.0, 0.0, 0.0));
	float cost = texture(mask, uv).g;
	float d = f - 0.5;
	float px = max(fwidth(f), 1e-4);
	float inside = smoothstep(-px, px, d);
	// Liseré centré sur le seuil, largeur constante à l'écran.
	float line = 1.0 - smoothstep(px * edge_px * 0.5, px * edge_px, abs(d));
	// Ombre d'encre juste à l'extérieur du liseré.
	float shadow = (1.0 - inside) * (1.0 - smoothstep(px * edge_px, px * edge_px * 3.5, -d));
	vec4 color = vec4(0.0);
	if (show_next) {
		float f2 = field(uv, vec3(0.0, 0.0, 1.0));
		float d2 = f2 - 0.5;
		float px2 = max(fwidth(f2), 1e-4);
		float inside2 = smoothstep(-px2, px2, d2) * (1.0 - inside);
		float line2 = (1.0 - smoothstep(px2 * next_edge_px * 0.5, px2 * next_edge_px, abs(d2))) * (1.0 - inside);
		color = vec4(next_fill_color.rgb, next_fill_color.a * inside2);
		color = mix(color, vec4(next_edge_color.rgb, next_edge_color.a), line2 * next_edge_color.a);
	}
	vec4 fill = mix(fill_color, rim_fill_color, smoothstep(0.55, 1.0, cost));
	// Hors des deux anneaux, la teinte reste celle du voile (seul l'alpha s'estompe).
	color = vec4(mix(color.rgb, fill.rgb, color.a > 0.0 ? inside : 1.0), mix(color.a, fill.a, inside));
	color = mix(color, vec4(shadow_color.rgb, shadow_color.a), shadow * shadow_color.a);
	color = mix(color, edge_color, line);
	ALBEDO = color.rgb;
	ALPHA = clamp(max(color.a, line * edge_color.a), 0.0, 1.0);
}
