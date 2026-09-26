shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// Marqueurs de lieux (lot C6, refondus au lot DA3, ADR 0060) : quads d'un `MultiMesh`, face
// caméra, taille constante à l'écran. Langage unique (bible DA § 8) :
//   forme = type : pictogramme peint pris dans un atlas unique (`atlas`, case INSTANCE_CUSTOM.r) ;
//   écu = détenteur : armoiries de la faction composées à l'exécution (`shields`, case COLOR.r,
//     -1 = aucun) ; insigne de port (case `port_cell`) si COLOR.g > 0,5 ;
//   taille = rang : INSTANCE_CUSTOM.b = taille écran en px.
// Dé-encombrement sans CPU par image : INSTANCE_CUSTOM.a = distance caméra (÷ 100) jusqu'à
// laquelle le lieu reste affiché ; fondu sur `fade_distance`, quad replié au-delà.
// INSTANCE_CUSTOM.g = surbrillance (sélection).

uniform sampler2D atlas : source_color, filter_linear_mipmap, repeat_disable;
uniform sampler2D shields : source_color, filter_linear_mipmap, repeat_disable;
uniform vec2 atlas_grid = vec2(4.0, 3.0);
uniform vec2 shield_grid = vec2(8.0, 4.0);
uniform float port_cell = -1.0;
// Centre (xy, fraction du quad, origine en bas à gauche) et demi-taille (z) de l'écu et de l'insigne.
uniform vec3 shield_place = vec3(0.74, 0.30, 0.25);
uniform vec3 badge_place = vec3(0.22, 0.24, 0.20);
uniform float size_scale = 1.0;
uniform float alpha = 1.0;
uniform float camera_distance = 0.0;
uniform float fade_distance = 60.0;
uniform vec3 highlight_color : source_color = vec3(0.949, 0.761, 0.188);

varying float v_cell;
varying float v_shield;
varying float v_port;
varying float v_highlight;
varying float v_alpha;
varying vec2 v_quad;

void vertex() {
	v_cell = INSTANCE_CUSTOM.r;
	v_highlight = INSTANCE_CUSTOM.g;
	v_shield = COLOR.r;
	v_port = COLOR.g;
	float until = INSTANCE_CUSTOM.a * 100.0;
	v_alpha = alpha * (1.0 - smoothstep(until - fade_distance, until, camera_distance));
	float size = INSTANCE_CUSTOM.b * size_scale * (v_highlight > 0.5 ? 1.18 : 1.0);
	vec4 center_view = MODELVIEW_MATRIX * vec4(0.0, 0.0, 0.0, 1.0);
	float dist = max(-center_view.z, 0.01);
	// abs : la projection peut être retournée en Y (cible de rendu), ce qui retournerait le quad.
	float world_per_px = dist * 2.0 / abs(PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
	vec2 corner = VERTEX.xy; // quad de -0,5 à 0,5
	v_quad = corner + 0.5; // repère écran du marqueur : x à droite, y vers le haut
	// Ancré par le bas (la base du pictogramme sur le lieu), légèrement relevé.
	center_view.xy += (corner + vec2(0.0, 0.42)) * size * world_per_px;
	POSITION = v_alpha < 0.004 ? vec4(2.0, 2.0, 2.0, 1.0) : PROJECTION_MATRIX * center_view;
}

// Échantillon d'une case d'une grille (uv local 0-1, origine en bas à gauche).
vec4 cell_sample(sampler2D tex, vec2 grid, float cell, vec2 uv) {
	vec2 inside = clamp(vec2(uv.x, 1.0 - uv.y), vec2(0.0), vec2(1.0));
	vec2 origin = vec2(mod(cell, grid.x), floor(cell / grid.x));
	return texture(tex, (origin + inside) / grid);
}

vec4 over(vec4 top, vec4 base) {
	float a = top.a + base.a * (1.0 - top.a);
	vec3 rgb = (top.rgb * top.a + base.rgb * base.a * (1.0 - top.a)) / max(a, 1e-4);
	return vec4(rgb, a);
}

vec4 placed(sampler2D tex, vec2 grid, float cell, vec2 uv, vec3 place) {
	vec2 local = (uv - place.xy) / (2.0 * place.z) + 0.5;
	if (any(lessThan(local, vec2(0.0))) || any(greaterThan(local, vec2(1.0)))) {
		return vec4(0.0);
	}
	return cell_sample(tex, grid, cell, local);
}

void fragment() {
	vec2 uv = v_quad; // origine en bas à gauche (indépendant des UV du quad)
	vec4 col = vec4(0.0);
	if (v_highlight > 0.5) {
		float r = length((uv - vec2(0.5, 0.45)) * vec2(1.0, 1.15));
		col = vec4(highlight_color, 0.75 * (1.0 - smoothstep(0.30, 0.50, r)));
	}
	if (v_cell >= 0.0) {
		col = over(cell_sample(atlas, atlas_grid, round(v_cell), uv), col);
	}
	if (v_port > 0.5 && port_cell >= 0.0) {
		col = over(placed(atlas, atlas_grid, port_cell, uv, badge_place), col);
	}
	if (v_shield >= 0.0) {
		col = over(placed(shields, shield_grid, round(v_shield), uv, shield_place), col);
	}
	if (col.a < 0.01) {
		discard;
	}
	ALBEDO = col.rgb;
	ALPHA = col.a * v_alpha;
}
