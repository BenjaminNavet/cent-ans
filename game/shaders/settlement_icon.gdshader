shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// Icônes de colonies (lot C6, palier moyen) : quads en `MultiMesh`, face caméra, taille
// constante à l'écran. Forme selon le type (INSTANCE_CUSTOM.r) :
//   0 cité (écu crénelé, plus grand), 1 ville (disque crénelé), 2 château (carré à créneaux),
//   3 abbaye (croix dans un disque), 4 village (petit disque).
// Couleur = contrôleur (COLOR d'instance) ; INSTANCE_CUSTOM.g = surbrillance (sélection),
// INSTANCE_CUSTOM.b = échelle relative.

uniform float size_px = 16.0;
uniform float alpha = 1.0;
uniform vec3 ink = vec3(0.10, 0.07, 0.04);
uniform vec3 highlight_color = vec3(1.0, 0.86, 0.35);

varying float v_kind;
varying float v_highlight;
varying vec4 v_color;

void vertex() {
	v_kind = INSTANCE_CUSTOM.r;
	v_highlight = INSTANCE_CUSTOM.g;
	v_color = COLOR;
	float scale = INSTANCE_CUSTOM.b > 0.0 ? INSTANCE_CUSTOM.b : 1.0;
	vec4 center_view = MODELVIEW_MATRIX * vec4(0.0, 0.0, 0.0, 1.0);
	float dist = max(-center_view.z, 0.01);
	float world_per_px = dist * 2.0 / (PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
	vec2 corner = VERTEX.xy; // quad de -0,5 à 0,5
	center_view.xy += corner * size_px * scale * world_per_px;
	POSITION = PROJECTION_MATRIX * center_view;
}

float sd_box(vec2 p, vec2 b) {
	vec2 d = abs(p) - b;
	return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);
}

// Distance signée de la forme (négative à l'intérieur), p dans [-1, 1].
float shape(vec2 p, int kind) {
	if (kind == 0) {
		// Écu : rectangle + pointe, créneaux en haut.
		float body = sd_box(p - vec2(0.0, 0.12), vec2(0.72, 0.5));
		float tip = length(p - vec2(0.0, -0.1)) - 0.72;
		float shield = min(body, max(tip, p.y - 0.1));
		float notch = sd_box(vec2(mod(p.x + 0.72, 0.48) - 0.24, p.y - 0.62), vec2(0.08, 0.12));
		return max(shield, -notch);
	} else if (kind == 1) {
		float a = atan(p.y, p.x);
		bool notch = fract(a * 10.0 / 6.2831853) < 0.4;
		return notch ? length(p) - 0.64 : length(p) - 0.8;
	} else if (kind == 2) {
		float sq = sd_box(p, vec2(0.6, 0.5));
		float notch = sd_box(vec2(mod(p.x + 0.6, 0.4) - 0.2, p.y - 0.5), vec2(0.07, 0.12));
		return max(min(sq, sd_box(p - vec2(0.0, 0.1), vec2(0.6, 0.62))), -notch);
	} else if (kind == 3) {
		float disc = length(p) - 0.78;
		return disc;
	}
	return length(p) - 0.62;
}

void fragment() {
	// Le quad de QuadMesh a UV.y = 0 en bas dans ce repère : les formes sont décrites y vers le haut.
	vec2 p = (UV - vec2(0.5)) * 2.0;
	int kind = int(round(v_kind));
	float d = shape(p, kind);
	float aa = fwidth(d) * 1.2;
	float fill = 1.0 - smoothstep(-aa, aa, d + 0.22);
	float outline = 1.0 - smoothstep(-aa, aa, d);
	float halo = v_highlight > 0.5 ? (1.0 - smoothstep(0.0, 0.25, d)) : 0.0;
	vec3 col = mix(ink, v_color.rgb, fill);
	if (kind == 3) {
		// Croix claire sur l'abbaye.
		float cross_d = min(sd_box(p, vec2(0.12, 0.5)), sd_box(p - vec2(0.0, 0.14), vec2(0.36, 0.11)));
		col = mix(col, vec3(0.95, 0.92, 0.84), 1.0 - smoothstep(-aa, aa, cross_d));
	}
	if (kind == 0) {
		float dot_d = length(p - vec2(0.0, 0.05)) - 0.18;
		col = mix(col, vec3(0.97, 0.94, 0.86), 1.0 - smoothstep(-aa, aa, dot_d));
	}
	col = v_highlight > 0.5 ? mix(highlight_color, col, outline) : col;
	ALBEDO = col;
	ALPHA = max(outline, halo) * alpha;
}
