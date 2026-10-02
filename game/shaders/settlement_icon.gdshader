shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// Écu des lieux (lot C6, DA3, refondu au lot DV2, ADR 0124) : quads d'un `MultiMesh`, face
// caméra, taille constante à l'écran. Le quad ne compose plus que l'écu du détenteur
// (armoiries de la faction, atlas `HeraldryAtlas`, case COLOR.r, -1 = aucun : quad replié), posé
// au-dessus du nom du lieu :
//   ancre = position monde du nom (`Label3D.position`, transformation de l'instance) ;
//   INSTANCE_CUSTOM.r = haut du texte au-dessus de l'ancre, en px du `Label3D` (× `label_scale`
//     = px écran) ; le bas de l'écu est `gap_px` plus haut ;
//   INSTANCE_CUSTOM.b = côté de l'écu en px écran (taille par rang) ;
//   INSTANCE_CUSTOM.g = surbrillance (> 0,5 sélection ; entre 0,2 et 0,5 survol, ADR 0160).
// Dé-encombrement sans CPU par image : INSTANCE_CUSTOM.a = distance caméra (÷ 100) jusqu'à
// laquelle le lieu reste affiché ; fondu sur `fade_distance`, quad replié au-delà.
// Lot DA7d : dé-encombrement écran (calculé côté CPU seulement quand la caméra bouge nettement) :
// COLOR.b = 1 affiché / 0 cède la place, COLOR.a = instant de la bascule ; fondu sur
// `declutter_fade` secondes jusqu'à `hidden_alpha` (horloge `declutter_now`).

uniform sampler2D shields : source_color, filter_linear_mipmap, repeat_disable;
uniform vec2 shield_grid = vec2(8.0, 4.0);
uniform float size_scale = 1.0;
uniform float label_scale = 1.0;
uniform float gap_px = 1.0;
uniform float alpha = 1.0;
uniform float camera_distance = 0.0;
uniform float fade_distance = 60.0;
uniform vec3 highlight_color : source_color = vec3(0.949, 0.761, 0.188);
uniform vec3 hover_color : source_color = vec3(1.0, 0.96, 0.82);
uniform float declutter_now = 0.0;
uniform float declutter_fade = 0.25;
uniform float hidden_alpha = 0.0;

varying float v_shield;
varying float v_highlight;
varying float v_alpha;
varying vec2 v_quad;

void vertex() {
	v_highlight = INSTANCE_CUSTOM.g;
	v_shield = COLOR.r;
	float until = INSTANCE_CUSTOM.a * 100.0;
	v_alpha = alpha * (1.0 - smoothstep(until - fade_distance, until, camera_distance));
	float switched = clamp((declutter_now - COLOR.a) / max(declutter_fade, 1e-3), 0.0, 1.0);
	float shown = mix(1.0 - COLOR.b, COLOR.b, switched);
	v_alpha *= mix(hidden_alpha, 1.0, shown);
	float size = INSTANCE_CUSTOM.b * size_scale * (v_highlight > 0.5 ? 1.18 : (v_highlight > 0.2 ? 1.12 : 1.0));
	vec4 center_view = MODELVIEW_MATRIX * vec4(0.0, 0.0, 0.0, 1.0);
	float dist = max(-center_view.z, 0.01);
	// abs : la projection peut être retournée en Y (cible de rendu), ce qui retournerait le quad.
	float world_per_px = dist * 2.0 / abs(PROJECTION_MATRIX[1][1] * VIEWPORT_SIZE.y);
	vec2 corner = VERTEX.xy; // quad de -0,5 à 0,5
	v_quad = corner + 0.5; // repère écran de l'écu : x à droite, y vers le haut
	// Bas de l'écu au-dessus du haut du nom.
	float lift = INSTANCE_CUSTOM.r * label_scale + gap_px;
	center_view.xy += ((corner + vec2(0.0, 0.5)) * size + vec2(0.0, lift)) * world_per_px;
	bool hidden = v_alpha < 0.004 || v_shield < 0.0;
	POSITION = hidden ? vec4(2.0, 2.0, 2.0, 1.0) : PROJECTION_MATRIX * center_view;
}

vec4 over(vec4 top, vec4 base) {
	float a = top.a + base.a * (1.0 - top.a);
	vec3 rgb = (top.rgb * top.a + base.rgb * base.a * (1.0 - top.a)) / max(a, 1e-4);
	return vec4(rgb, a);
}

void fragment() {
	vec2 uv = v_quad; // origine en bas à gauche (indépendant des UV du quad)
	vec2 inside = clamp(vec2(uv.x, 1.0 - uv.y), vec2(0.0), vec2(1.0));
	float cell = round(v_shield);
	vec2 origin = vec2(mod(cell, shield_grid.x), floor(cell / shield_grid.x));
	vec4 col = texture(shields, (origin + inside) / shield_grid);
	if (v_highlight > 0.5) {
		// Halo doré de la sélection derrière l'écu.
		float r = length(uv - vec2(0.5));
		col = over(col, vec4(highlight_color, 0.8 * (1.0 - smoothstep(0.36, 0.5, r))));
	}
	else if (v_highlight > 0.2) {
		// Halo clair du survol (ce que le clic prendrait).
		float r = length(uv - vec2(0.5));
		col = over(col, vec4(hover_color, 0.75 * (1.0 - smoothstep(0.36, 0.5, r))));
	}
	if (col.a < 0.01) {
		discard;
	}
	ALBEDO = col.rgb;
	ALPHA = col.a * v_alpha;
}
