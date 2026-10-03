// Lot RV-F : paillettes du soleil sur l'eau, lisibles en vue régionale (caméra haute).
// Les facettes des vagues sont sous le pixel en vue moyenne : chaque paillette est une cellule de
// taille fixe à l'écran (`glint_cell_px` pixels), tirée au hasard et qui s'allume puis s'éteint
// lentement. Deux tailles de grille voisines (puissances de 2 en unités monde) sont fondues selon
// l'empreinte du pixel, comme des mipmaps : la grille suit le zoom sans saut ni moiré, et une
// paillette ne passe jamais sous le pixel (pas de scintillement au dézoom).
// L'intensité suit un lobe large autour du reflet du soleil (facettes très inclinées d'une mer
// agitée) plus une traîne faible : le soleil de la carte est souvent derrière la caméra, la mer
// garde quelques éclats ailleurs que sur le chemin du soleil.
// `glint_mask` au fragment (masque spatial 0-1), `glint_light` dans light().

// Force des paillettes (0 : désactivées) et part de la traîne hors du chemin du soleil.
uniform float glint_amount = 0.55;
uniform float glint_tail = 0.16;
// Taille d'une cellule à l'écran (pixels) et part des cellules qui portent une paillette.
uniform float glint_cell_px = 4.0;
uniform float glint_coverage = 0.16;
// Échelle des nappes de mer agitée qui regroupent les paillettes (1 / unités monde).
uniform float glint_patch_scale = 0.02;
// Cadence du clignement (cycles par seconde, moyenne) et dérive de la grille avec la houle
// (cellules par seconde : vitesse fixe à l'écran, jamais un défilement rapide de cellules).
uniform float glint_rate = 0.35;
uniform vec2 glint_drift = vec2(0.06, 0.025);
// Pente des facettes (écart-type) : largeur du chemin du soleil.
uniform float glint_slope = 0.32;
// Empreinte du pixel (unités monde) de début et de plein effet : de près, le reflet ordinaire des
// vagues suffit ; les paillettes prennent le relais en vue moyenne et régionale.
uniform vec2 glint_footprint = vec2(0.35, 1.2);
uniform vec3 glint_color : source_color = vec3(1.0, 0.94, 0.80);

float glint_hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

// Paillettes d'une grille de cellules de côté `cell` (unités monde).
float glint_grid(vec2 p, float cell, float time) {
	vec2 q = p / cell + time * glint_drift;
	vec2 id = floor(q);
	vec2 f = fract(q) - 0.5;
	float h = glint_hash(id);
	// Nappes de mer plus ou moins agitée (bruit large, en monde) : paillettes groupées, pas en semis.
	vec2 g = p * glint_patch_scale + time * vec2(0.004, 0.002);
	vec2 gi = floor(g);
	vec2 gf = fract(g);
	gf = gf * gf * (3.0 - 2.0 * gf);
	float patchy = mix(mix(glint_hash(gi), glint_hash(gi + vec2(1.0, 0.0)), gf.x), mix(glint_hash(gi + vec2(0.0, 1.0)), glint_hash(gi + vec2(1.0, 1.0)), gf.x), gf.y);
	if (h > glint_coverage * mix(0.25, 1.75, patchy)) {
		return 0.0;
	}
	float h2 = glint_hash(id + 17.3);
	// Position dans la cellule et clignement (montée et descente douces, pause entre deux éclats).
	vec2 c = (vec2(h2, glint_hash(id + 5.1)) - 0.5) * 0.3;
	float blob = 1.0 - smoothstep(0.08, 0.42, length(f - c));
	float phase = time * glint_rate * mix(0.6, 1.4, h2) + h * 37.0;
	float on = pow(max(sin(phase * 6.2832), 0.0), 3.0);
	return blob * on;
}

// Masque des paillettes au point `p` (unités monde) ; `footprint` : unités monde par pixel.
float glint_mask(vec2 p, float footprint, float time) {
	if (glint_amount <= 0.0) {
		return 0.0;
	}
	float w = smoothstep(glint_footprint.x, glint_footprint.y, footprint);
	if (w <= 0.0) {
		return 0.0;
	}
	float lod = log2(max(footprint * glint_cell_px, 1e-4));
	float l0 = floor(lod);
	float k = lod - l0;
	float a = glint_grid(p, exp2(l0), time);
	float b = glint_grid(p + 3.7, exp2(l0 + 1.0), time);
	return mix(a, b, k) * w;
}

// Lumière des paillettes (à ajouter à SPECULAR_LIGHT). `n_flat` : normale de la surface au repos
// (repère vue), `view`, `light` : VIEW et LIGHT de light().
float glint_light(float mask, vec3 n_flat, vec3 view, vec3 light) {
	if (mask <= 0.0) {
		return 0.0;
	}
	vec3 h = normalize(view + light);
	float ndh = clamp(dot(n_flat, h), 1e-3, 1.0);
	float ndh2 = ndh * ndh;
	// Distribution de Beckmann des pentes (Cox-Munk simplifié) ramenée à 1 au sommet.
	float lobe = exp(-(1.0 - ndh2) / (ndh2 * glint_slope * glint_slope));
	float sun_up = clamp(dot(n_flat, light), 0.0, 1.0);
	return mask * glint_amount * (lobe + glint_tail * sun_up);
}
