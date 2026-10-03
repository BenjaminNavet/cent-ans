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
// Lot RJ-c (ADR 0175) : bannière possesseur / occupant. COLOR.r = case de l'écu du
// PROPRIÉTAIRE (de droit) ; COLOR.g = (case de l'écu de l'OCCUPANT + 1) × 16 + position du
// propriétaire × 4 + position de l'occupant (0 aucune, 1 soi, 2 ami, 3 ennemi ; occupant 0 /
// case 0 : place non occupée). Place occupée : quad élargi, petit écu de l'occupant en bas à
// droite, à demi posé sur celui du propriétaire. Chaque écu porte un liseré à la couleur de
// position (ADR 0155) ; aucun pour une faction neutre.
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
// RJ-c : couleurs de position (soi, ami, ennemi), marge du liseré (fraction de l'écu), côté de
// l'écu de l'occupant (fraction de l'écu du propriétaire) et chevauchement (fraction de son côté).
uniform vec3 cue_self : source_color = vec3(0.851, 0.659, 0.2);
uniform vec3 cue_friend : source_color = vec3(0.227, 0.604, 0.259);
uniform vec3 cue_enemy : source_color = vec3(0.824, 0.122, 0.09);
uniform float border = 0.09;
uniform float occupant_scale = 0.62;
uniform float occupant_overlap = 0.45;
uniform float shield_cell_px = 64.0;

varying float v_shield;
varying float v_highlight;
varying float v_alpha;
varying vec2 v_quad;
varying float v_occupant;
varying float v_owner_cue;
varying float v_occupant_cue;
varying float v_width;

void vertex() {
	v_highlight = INSTANCE_CUSTOM.g;
	v_shield = COLOR.r;
	float packed = max(round(COLOR.g), 0.0);
	v_occupant = floor(packed / 16.0) - 1.0;
	v_owner_cue = mod(floor(packed / 4.0), 4.0);
	v_occupant_cue = mod(packed, 4.0);
	// RJ-c : quad élargi d'une place occupée (écu du propriétaire + écu de l'occupant).
	v_width = v_occupant >= 0.0 ? 1.0 + occupant_scale * (1.0 - occupant_overlap) : 1.0;
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
	vec2 extent = vec2(v_width, 1.0);
	v_quad = (corner + 0.5) * extent; // repère écran en côtés d'écu : x à droite, y vers le haut
	// Bas de l'écu au-dessus du haut du nom.
	float lift = INSTANCE_CUSTOM.r * label_scale + gap_px;
	center_view.xy += ((corner * extent + vec2(0.0, 0.5)) * size + vec2(0.0, lift)) * world_per_px;
	bool hidden = v_alpha < 0.004 || v_shield < 0.0;
	POSITION = hidden ? vec4(2.0, 2.0, 2.0, 1.0) : PROJECTION_MATRIX * center_view;
}

vec4 over(vec4 top, vec4 base) {
	float a = top.a + base.a * (1.0 - top.a);
	vec3 rgb = (top.rgb * top.a + base.rgb * base.a * (1.0 - top.a)) / max(a, 1e-4);
	return vec4(rgb, a);
}

// Écu de la case `cell` au point `p` (repère de l'écu, 0-1, y vers le haut), réduit de `border`
// sur chaque bord pour laisser la place au liseré ; transparent hors de l'écu.
vec4 shield_at(float cell, vec2 p, float lod) {
	vec2 inside = (vec2(p.x, 1.0 - p.y) - border) / (1.0 - 2.0 * border);
	if (inside.x < 0.0 || inside.y < 0.0 || inside.x > 1.0 || inside.y > 1.0) {
		return vec4(0.0);
	}
	vec2 origin = vec2(mod(cell, shield_grid.x), floor(cell / shield_grid.x));
	return textureLod(shields, (origin + inside) / shield_grid, lod);
}

vec3 cue_color(float cue) {
	return cue > 2.5 ? cue_enemy : (cue > 1.5 ? cue_friend : cue_self);
}

// Écu + liseré de position (dilatation de l'alpha sur 8 directions) au point `p`.
vec4 banner_at(float cell, vec2 p, float cue, float lod) {
	vec4 col = shield_at(cell, p, lod);
	if (cue < 0.5) {
		return col;
	}
	float ring = 0.0;
	float r = border * 0.8;
	for (int k = 0; k < 8; k++) {
		float angle = float(k) * 0.785398;
		ring = max(ring, shield_at(cell, p + vec2(cos(angle), sin(angle)) * r, lod).a);
	}
	return over(col, vec4(cue_color(cue), ring));
}

void fragment() {
	vec2 uv = v_quad; // origine en bas à gauche (indépendant des UV du quad), en côtés d'écu
	float cell = round(v_shield);
	// Niveau de mipmap calculé hors des branches (échantillonnage conditionnel) : texels de case
	// d'atlas par pixel écran.
	float texels = max(length(dFdx(uv)), length(dFdy(uv))) * shield_cell_px / (1.0 - 2.0 * border);
	float lod = max(log2(max(texels, 1e-4)), 0.0);
	vec4 col = banner_at(cell, uv, v_owner_cue, lod);
	if (v_occupant >= 0.0) {
		// RJ-c : petit écu de l'occupant en bas à droite, posé sur celui du propriétaire.
		vec2 q = (uv - vec2(v_width - occupant_scale, 0.0)) / occupant_scale;
		if (q.x >= 0.0 && q.y >= 0.0 && q.x <= 1.0 && q.y <= 1.0) {
			col = over(banner_at(round(v_occupant), q, v_occupant_cue, lod + log2(1.0 / occupant_scale)), col);
		}
	}
	uv = vec2(uv.x / v_width, uv.y);
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
