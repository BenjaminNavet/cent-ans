// Chantier GA (ADR 0105) : fonctions partagées par le sol de bataille (GA2,
// `battle_ground.gdshader`), le terrain de campagne et la mer (GA4, `terrain.gdshader`,
// `water.gdshader`). Aucune dépendance : les appelants fournissent leur bruit (texture de bruit
// en bataille, bruit de valeur procédural sur la carte), les échelles restent propres à chacun.

// Macro-variation de teinte et de luminance, faible amplitude, multipliée à l'albédo.
// `h` : bruit macro (0..1) de la teinte ; `lum_h` : bruit (0..1) de la luminance.
// `tint_amp` = 1 et `lum_amp` = 0 : réglage d'origine de GA2 (teinte seule, ±3 %).
vec3 ga_macro_variation(float h, float lum_h, float tint_amp, float lum_amp) {
	vec3 tint = vec3(
		1.0 + (h - 0.5) * 0.06 * tint_amp,
		1.0 + (h - 0.5) * 0.04 * tint_amp,
		1.0 - (h - 0.5) * 0.05 * tint_amp
	);
	return tint * (1.0 + (lum_h - 0.5) * 2.0 * lum_amp);
}

// Couleur de l'eau selon la profondeur (m) : haut-fond → palier intermédiaire (`mid_depth_m`)
// → grand fond (décroissance exponentielle `falloff_m`). `mid_amount` = 0 : deux paliers
// (réglage d'origine du lot V2).
vec3 ga_depth_color(float depth, vec3 shallow, vec3 mid, vec3 deep, float mid_depth_m,
		float falloff_m, float mid_amount) {
	vec3 two = mix(shallow, deep, 1.0 - exp(-depth / falloff_m));
	vec3 near = mix(shallow, mid, smoothstep(0.0, mid_depth_m, depth));
	vec3 three = mix(near, deep, smoothstep(mid_depth_m * 0.5, mid_depth_m + falloff_m * 2.0, depth));
	return mix(two, three, mid_amount);
}
