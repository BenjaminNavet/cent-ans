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

// Couleur vieillie. `p_m` position locale (m), `n` normale locale unitaire, `h_m` hauteur au-dessus
// du pied du bâtiment (m), `seed` graine du bâtiment, `grime` poids murs (0-1), `moss_k` poids
// mousse sur les faces au ciel (0-1), `age` intensité (aging × variation), `far` (ba_far).
vec3 ba_age(vec3 col, vec3 p_m, vec3 n, float h_m, float seed, float grime, float moss_k, float age, float far) {
	if (age <= 0.0) {
		return col;
	}
	float vertical = 1.0 - abs(n.y);
	// Coordonnée horizontale le long du mur (tangente de la façade).
	vec2 t = normalize(vec2(-n.z, n.x) + vec2(1e-4, 0.0));
	float u = dot(p_m.xz, t) + seed * 37.0;
	float h = max(h_m, 0.0);
	// Grandes nuances (~6 m) : visibles de près comme de loin.
	float macro = ba_noise(vec2(u, p_m.y) * 0.16 + seed * 13.0);
	// Pied des murs : boue et remontées d'humidité, 0,5-1,4 m, bord éclaboussé de près.
	float fine = mix(ba_noise(vec2(u * 5.0, h * 7.0)), 0.5, far);
	float foot_h = 0.5 + 0.9 * macro;
	float foot = (1.0 - smoothstep(0.0, foot_h, h - (fine - 0.5) * 0.35)) * vertical * grime;
	col = mix(col, col * vec3(0.52, 0.47, 0.38), clamp(foot * 0.85 * age, 0.0, 0.8));
	// Coulures sous les appuis : colonnes (pas ~2,5 m) qui partent sous chaque niveau de baies
	// (appui à 1 m + 3 m par étage) et s'estompent en descendant.
	float column = smoothstep(0.55, 0.8, ba_noise(vec2(u * 0.45, 3.1 + seed * 5.0)));
	float thread = mix(ba_noise(vec2(u * 3.2, h * 0.35)), 0.5, far);
	float below = fract((1.0 + 0.3 * seed - h) / 3.0) * 3.0;
	float streak = column * exp(-below * 0.7) * (0.45 + thread) * smoothstep(0.8, 2.0, h);
	streak = mix(streak, 0.12, far) * vertical * grime;
	col *= 1.0 - clamp(streak * 0.4 * age, 0.0, 0.45);
	// Crasse diffuse (plaques).
	col *= 1.0 - 0.14 * age * smoothstep(0.45, 0.85, macro) * grime;
	// Faces au ciel : toits assombris, mousse et lichen par plaques.
	float up = smoothstep(0.25, 0.75, n.y);
	float blotch = mix(ba_noise(p_m.xz * 0.6 + seed * 19.0), 0.5, far * 0.7);
	float moss = up * moss_k * smoothstep(0.42, 0.78, blotch * 0.7 + macro * 0.3 + 0.12 * (seed - 0.5)) * age;
	col *= 1.0 - 0.12 * up * moss_k * age;
	col = mix(col, vec3(0.10, 0.12, 0.05) * (0.8 + 0.4 * macro), clamp(moss * 0.5, 0.0, 0.6));
	return col;
}
