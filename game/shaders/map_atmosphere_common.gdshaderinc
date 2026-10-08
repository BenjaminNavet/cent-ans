// Lot ME5 : outils communs de l'atmosphère de la carte (MapAtmosphere). Aucune règle de jeu.
// Bruit de valeur, fbm, effacement autour du point visé (aucune carte ni rideau devant ce que le
// joueur regarde) et cartes (billboards) posées sans nœud par instance (MultiMesh).

uniform vec2 atm_focus = vec2(0.0);       // point visé (px carte)
uniform float atm_clear_radius = 0.0;     // rayon d'effacement (unités monde), 0 : aucun
uniform float atm_clear_feather = 1.0;
uniform float atm_ground_y = 2.0;         // altitude moyenne du sol, pour retrouver le point vu
uniform float atm_fade = 1.0;             // fondu global posé par le script (distance, parchemin)
uniform vec3 atm_sun_dir = vec3(0.3, 0.8, 0.5);  // vers le soleil, repère monde
uniform vec3 atm_sun_color : source_color = vec3(1.0, 0.97, 0.9);
uniform vec2 atm_wind = vec2(3.0, 0.6);
uniform vec2 atm_map_size = vec2(1.0);

float atm_hash(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float atm_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(atm_hash(i), atm_hash(i + vec2(1.0, 0.0)), f.x),
			mix(atm_hash(i + vec2(0.0, 1.0)), atm_hash(i + vec2(1.0, 1.0)), f.x), f.y);
}

float atm_fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	for (int i = 0; i < 4; i++) {
		v += a * atm_noise(p);
		p = p * 2.03 + vec2(17.1, 9.7);
		a *= 0.5;
	}
	return v;
}

// 0 devant le point visé, 1 au-delà du rayon + fondu. Le point du sol vu à travers `world` (rayon
// caméra prolongé jusqu'au sol) décide, pas l'aplomb du fragment.
float atm_focus_clear(vec3 world, vec3 cam) {
	if (atm_clear_radius <= 0.0) {
		return 1.0;
	}
	vec2 ground = world.xz;
	if (cam.y > world.y) {
		float t = (cam.y - atm_ground_y) / max(cam.y - world.y, 1e-3);
		ground = cam.xz + (world.xz - cam.xz) * t;
	}
	return smoothstep(atm_clear_radius, atm_clear_radius + atm_clear_feather, distance(ground, atm_focus));
}

// Fondu aux bords de la carte (les cartes dérivantes y reviennent par l'autre côté).
float atm_edge_fade(vec2 p) {
	vec2 e = min(p, atm_map_size - p);
	return smoothstep(0.0, 0.06 * min(atm_map_size.x, atm_map_size.y), min(e.x, e.y));
}
