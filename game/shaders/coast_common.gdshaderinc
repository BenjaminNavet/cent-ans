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
