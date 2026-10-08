// Lot DN-ME4 : névés et glaciers posés sur le sol de la carte (permanents, toutes saisons).
// Inclus par terrain.gdshader après les fonctions utilitaires (value_noise, fbm).
uniform vec4 fw_glacier = vec4(2600.0, 3100.0, 0.62, 1.0);  // limite basse (m), pleine couverture (m), pente max (m/m), force
uniform vec3 fw_ice_color = vec3(0.70, 0.80, 0.88);
uniform vec3 fw_crevasse_color = vec3(0.38, 0.50, 0.62);
uniform vec3 fw_neve_color = vec3(0.90, 0.93, 0.97);

// `snow` : neige saisonnière déjà calculée (0-1) ; renvoie le poids glace/névé posé sur `col`.
void fw_glacier_apply(vec2 p, float hc, float slope, float footprint, float macro, float land_f, inout vec3 col) {
	if (fw_glacier.w <= 0.0 || hc < fw_glacier.x - 600.0) {
		return;
	}
	float alt = hc + (macro - 0.5) * 420.0;
	float flat_k = 1.0 - smoothstep(fw_glacier.z - 0.12, fw_glacier.z + 0.25, slope);
	// Névé : plaques blanches sous la limite des glaciers, découpées par le bruit.
	float patch = value_noise(p * 0.35) * 0.6 + value_noise(p * 1.3) * 0.4;
	float neve = smoothstep(fw_glacier.x - 500.0, fw_glacier.x + 150.0, alt) * smoothstep(0.35, 0.6, patch + 0.12) * flat_k;
	col = mix(col, fw_neve_color * (0.92 + 0.1 * patch), neve * 0.8 * fw_glacier.w * land_f);
	// Glacier : langue blanc-bleu à crevasses, sur pente douce au-dessus de la limite.
	float ice = smoothstep(fw_glacier.x, fw_glacier.y, alt) * flat_k;
	if (ice > 0.01) {
		float fade_detail = 1.0 - smoothstep(0.5, 2.5, footprint);
		float seams = abs(value_noise(p * vec2(2.2, 0.7)) - 0.5) * 2.0;
		float crevasse = (1.0 - smoothstep(0.0, 0.1, seams)) * fade_detail;
		vec3 ice_col = mix(fw_ice_color * (0.9 + 0.15 * patch), fw_crevasse_color, crevasse * 0.7);
		col = mix(col, ice_col, ice * 0.85 * fw_glacier.w * land_f);
	}
}
