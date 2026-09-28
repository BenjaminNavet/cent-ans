// Chantier GA (ADR 0105) : fonctions partagées par le sol de bataille (GA2,
// `battle_ground.gdshader`), le terrain de campagne et la mer (GA4, `terrain.gdshader`,
// `water.gdshader`). Aucune dépendance : les appelants fournissent leur bruit (texture de bruit
// en bataille, bruit de valeur procédural sur la carte), les échelles restent propres à chacun.

// Macro-variation de teinte et de luminance, faible amplitude, multipliée à l'albédo.
// `ga_h` : bruit macro (0..1) de la teinte ; `ga_lum_h` : bruit (0..1) de la luminance.
// `ga_tint_amp` = 1 et `ga_lum_amp` = 0 : réglage d'origine de GA2 (teinte seule, ±3 %).
vec3 ga_macro_variation(float ga_h, float ga_lum_h, float ga_tint_amp, float ga_lum_amp) {
	vec3 ga_tint = vec3(
		1.0 + (ga_h - 0.5) * 0.06 * ga_tint_amp,
		1.0 + (ga_h - 0.5) * 0.04 * ga_tint_amp,
		1.0 - (ga_h - 0.5) * 0.05 * ga_tint_amp
	);
	return ga_tint * (1.0 + (ga_lum_h - 0.5) * 2.0 * ga_lum_amp);
}

// Couleur de l'eau selon la profondeur (m) : haut-fond → palier intermédiaire (`ga_mid_depth`)
// → grand fond (décroissance exponentielle `ga_falloff`). `ga_mid_amount` = 0 : deux paliers
// (réglage d'origine du lot V2).
vec3 ga_depth_color(float ga_d, vec3 ga_shallow, vec3 ga_mid, vec3 ga_deep, float ga_mid_depth,
		float ga_falloff, float ga_mid_amount) {
	vec3 ga_two = mix(ga_shallow, ga_deep, 1.0 - exp(-ga_d / ga_falloff));
	vec3 ga_near = mix(ga_shallow, ga_mid, smoothstep(0.0, ga_mid_depth, ga_d));
	vec3 ga_three = mix(ga_near, ga_deep, smoothstep(ga_mid_depth * 0.5, ga_mid_depth + ga_falloff * 2.0, ga_d));
	return mix(ga_two, ga_three, ga_mid_amount);
}
