// Lot TB5 (mer et côtes) : géologie de la côte partagée par le terrain (`coast_band.gdshaderinc`)
// et la mer (`water.gdshader`). Inclus après `coast_dist`, `map_size` et `value_noise`.
// `coast_types` : texture basse définition cuite par `CoastLook` depuis `data/map/coast_types.json`
// (R craie, V granite, B galets, A largeur de plage) ; règle de pente : altitude lue à
// `coast_probe_px` de la côte vers l'intérieur, plage sous `coast_cliff_m.x`, falaise au-dessus
// de `coast_cliff_m.y`. `coast_enabled` = false (pas de données) : rendu inchangé.
uniform sampler2D coast_types : filter_linear, repeat_disable, hint_default_black;
uniform bool coast_enabled = false;
uniform float coast_probe_px = 1.6;
uniform vec2 coast_cliff_m = vec2(36.0, 52.0);

// Géologie au point p (px carte) ; le bord des régions est déformé par un bruit lent (pas de
// limite droite le long du rivage) et resserré.
vec4 coast_geology(vec2 p) {
	vec2 warp = (vec2(value_noise(p * 0.21), value_noise(p * 0.21 + vec2(37.0, 11.0))) - 0.5) * 10.0;
	vec4 g = textureLod(coast_types, (p + warp) / map_size, 0.0);
	return vec4(smoothstep(vec3(0.3), vec3(0.7), g.rgb), g.a);
}

// Direction de la terre (gradient de la distance à la côte), unitaire, en px carte.
vec2 coast_to_land(vec2 uv) {
	vec2 ex = vec2(1.0 / map_size.x, 0.0);
	vec2 ey = vec2(0.0, 1.0 / map_size.y);
	vec2 cg = vec2(textureLod(coast_dist, uv + ex, 0.0).r - textureLod(coast_dist, uv - ex, 0.0).r,
		textureLod(coast_dist, uv + ey, 0.0).r - textureLod(coast_dist, uv - ey, 0.0).r);
	return cg / max(length(cg), 1e-5);
}

// Point (uv) où lire l'altitude pour la règle de pente : à `coast_probe_px` de la côte, côté
// terre, quel que soit le point de la bande d'où l'on part (`coast` : distance signée en px).
vec2 coast_probe_uv(vec2 uv, float coast, vec2 to_land) {
	return uv + to_land * (coast_probe_px - coast) / map_size;
}

// 0 plage, 1 falaise, pour l'altitude (m) lue au point de sonde.
float coast_cliff_factor(float h) {
	return smoothstep(coast_cliff_m.x, coast_cliff_m.y, h);
}

// Ressac (TB5) : une lame d'écume avance sur le rivage puis se retire, partagée par la mer (côté
// eau) et la bande côtière du terrain (côté plage) pour que la lame soit continue au trait de
// côte. Réglages : bloc `swash` de `data/map/coast_types.json`. `swash_amount` = 0 : pas de ressac.
uniform float swash_amount = 0.0;
uniform float swash_period_s = 7.0;
uniform float swash_out_px = 0.45;    // recul de la lame vers le large (px carte)
uniform float swash_in_px = 0.22;     // montée sur la plage
uniform float swash_edge_px = 0.07;   // épaisseur du liseré d'écume
uniform float swash_film_px = 0.3;    // dentelle d'écume derrière le liseré
uniform float swash_cliff = 0.35;     // part de la montée au pied d'une falaise
uniform vec3 swash_foam_color = vec3(0.62, 0.66, 0.66);

// Lame au point p. `coast` : distance signée à la côte (px carte, > 0 sur terre) ; `cliff_f` :
// 0 plage, 1 falaise (la lame y monte peu). Rend x = écume [0, 1], y = eau de la lame [0, 1]
// (1 côté large du front). La phase varie lentement le long du rivage (la lame n'arrive pas
// partout au même instant) ; au dézoom, le va-et-vient se fige en un liseré (pas de scintillement).
vec2 coast_swash(vec2 p, float coast, float cliff_f, float footprint) {
	if (swash_amount <= 0.0) {
		return vec2(0.0);
	}
	float reach_in = swash_in_px * mix(1.0, swash_cliff, cliff_f);
	if (coast > reach_in + swash_edge_px + 0.1 || coast < -swash_out_px - swash_film_px - swash_edge_px - 0.1) {
		return vec2(0.0);
	}
	float far = smoothstep(0.3, 1.1, footprint);
	float cyc = fract(TIME / swash_period_s + value_noise(p * 0.45) + value_noise(p * 0.11) * 2.0);
	float up = smoothstep(0.0, 0.3, cyc);          // montée rapide
	float down = 1.0 - smoothstep(0.3, 1.0, cyc);  // retrait lent
	float adv = mix(up * down, 0.5, far);
	float front = mix(-swash_out_px, reach_in, adv) + (value_noise(p * 3.1) - 0.5) * 0.08 * (1.0 - far);
	float behind = front - coast;                  // > 0 côté large du front
	float edge = 1.0 - smoothstep(0.0, swash_edge_px, abs(behind));
	float lace = smoothstep(0.35, 0.75, value_noise(p * 6.0 + vec2(TIME * 0.07, 0.0)));
	float film = step(0.0, behind) * (1.0 - smoothstep(0.0, swash_film_px, behind)) * lace * mix(0.35, 0.8, down);
	float foam = max(edge * mix(0.55, 1.0, down), film) * mix(1.0, 0.6, far);
	return vec2(foam * swash_amount, smoothstep(-0.02, 0.02, behind) * swash_amount);
}
