// Lot SR5 : usure procédurale des bâtiments, commune aux maquettes de campagne
// (`building_atlas.gdshader`), aux villes à l'échelle (`town_building.gdshader`) et aux bâtiments
// de bataille (`building_pbr.gdshader`). Reprend la logique `aging` de `landmark.gdshader`
// (pied des murs, coulures, crasse, mousse sur les faces au ciel) sans texture de masques :
// bruit de valeur procédural, ramené à sa moyenne quand un pixel couvre plus de quelques
// décimètres (`ba_far`), pour rester doux et sans scintillement de loin. Rendu seulement.

float ba_hash21(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float ba_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	float a = ba_hash21(i);
	float b = ba_hash21(i + vec2(1.0, 0.0));
	float c = ba_hash21(i + vec2(0.0, 1.0));
	float d = ba_hash21(i + vec2(1.0, 1.0));
	return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Graine par bâtiment (0-1) : origine monde de l'instance (MultiMesh compris) et INSTANCE_ID.
float ba_seed(vec3 origin, int instance_id) {
	return ba_hash21(floor(origin.xz * 7.0) + vec2(float(instance_id) * 0.618, 0.37));
}

// 0 : détail fin visible ; 1 : un pixel couvre ≥ 0,8 m (bruit fin ramené à sa moyenne).
float ba_far(vec3 p_m) {
	vec3 fw = fwidth(p_m);
	return smoothstep(0.15, 0.8, max(fw.x, max(fw.y, fw.z)));
}

// Couleur vieillie. `ba_p` position locale (m), `ba_n` normale locale unitaire, `ba_hm` hauteur
// au-dessus du pied du bâtiment (m), `ba_s` graine du bâtiment, `ba_g` poids murs (0-1), `ba_m`
// poids mousse sur les faces au ciel (0-1), `ba_a` intensité (aging × variation), `ba_f` (ba_far).
// Noms préfixés : pas de collision avec les uniformes et varyings des shaders qui l'incluent.
vec3 ba_age(vec3 ba_col, vec3 ba_p, vec3 ba_n, float ba_hm, float ba_s, float ba_g, float ba_m, float ba_a, float ba_f) {
	if (ba_a <= 0.0) {
		return ba_col;
	}
	float ba_vertical = 1.0 - abs(ba_n.y);
	// Coordonnée horizontale le long du mur (tangente de la façade).
	vec2 ba_t = normalize(vec2(-ba_n.z, ba_n.x) + vec2(1e-4, 0.0));
	float ba_u = dot(ba_p.xz, ba_t) + ba_s * 37.0;
	float ba_h = max(ba_hm, 0.0);
	// Grandes nuances (~6 m) : visibles de près comme de loin.
	float ba_macro = ba_noise(vec2(ba_u, ba_p.y) * 0.16 + ba_s * 13.0);
	// Pied des murs : boue et remontées d'humidité, 0,5-1,4 m, bord éclaboussé de près.
	float ba_fine = mix(ba_noise(vec2(ba_u * 5.0, ba_h * 7.0)), 0.5, ba_f);
	float ba_foot_h = 0.5 + 0.9 * ba_macro;
	float ba_foot = (1.0 - smoothstep(0.0, ba_foot_h, ba_h - (ba_fine - 0.5) * 0.35)) * ba_vertical * ba_g;
	ba_col = mix(ba_col, ba_col * vec3(0.52, 0.47, 0.38), clamp(ba_foot * 0.85 * ba_a, 0.0, 0.8));
	// Coulures sous les appuis : colonnes (pas ~2,5 m) qui partent sous chaque niveau de baies
	// (appui à 1 m + 3 m par étage) et s'estompent en descendant.
	float ba_column = smoothstep(0.55, 0.8, ba_noise(vec2(ba_u * 0.45, 3.1 + ba_s * 5.0)));
	float ba_thread = mix(ba_noise(vec2(ba_u * 3.2, ba_h * 0.35)), 0.5, ba_f);
	float ba_below = fract((1.0 + 0.3 * ba_s - ba_h) / 3.0) * 3.0;
	float ba_streak = ba_column * exp(-ba_below * 0.7) * (0.45 + ba_thread) * smoothstep(0.8, 2.0, ba_h);
	ba_streak = mix(ba_streak, 0.12, ba_f) * ba_vertical * ba_g;
	ba_col *= 1.0 - clamp(ba_streak * 0.4 * ba_a, 0.0, 0.45);
	// Crasse diffuse (plaques).
	ba_col *= 1.0 - 0.14 * ba_a * smoothstep(0.45, 0.85, ba_macro) * ba_g;
	// Faces au ciel : toits assombris, mousse et lichen par plaques.
	float ba_up = smoothstep(0.25, 0.75, ba_n.y);
	float ba_blotch = mix(ba_noise(ba_p.xz * 0.6 + ba_s * 19.0), 0.5, ba_f * 0.7);
	float ba_moss = ba_up * ba_m * smoothstep(0.42, 0.78, ba_blotch * 0.7 + ba_macro * 0.3 + 0.12 * (ba_s - 0.5)) * ba_a;
	ba_col *= 1.0 - 0.12 * ba_up * ba_m * ba_a;
	ba_col = mix(ba_col, vec3(0.10, 0.12, 0.05) * (0.8 + 0.4 * ba_macro), clamp(ba_moss * 0.5, 0.0, 0.6));
	return ba_col;
}
