// RC5 (ADR 0117) : détail d'eau tuilable (texture Nano Banana 2) à inclure dans les shaders d'eau
// de la carte (mer, fleuves, rivières) : #include "res://shaders/water_detail.gdshaderinc"
// Réglé par le script `WaterDetail.apply()` (data/fx/water_detail.json). À `water_detail_strength`
// = 0 (défaut, ou texture absente) les deux fonctions rendent des valeurs neutres : le rendu
// procédural du shader hôte est inchangé.

uniform sampler2D water_detail_normal : hint_normal, repeat_enable, filter_linear_mipmap;
uniform sampler2D water_detail_albedo : source_color, repeat_enable, filter_linear_mipmap;
// 0 = désactivé ; 1 = détail plein.
uniform float water_detail_strength = 0.0;
// Répétitions de la tuile par unité monde.
uniform float water_detail_scale = 1.0;

// Normale de détail en espace monde (Y vers le haut), (0, 1, 0) quand le détail est éteint.
// `flow` : dérive de la texture par seconde en coordonnées de tuile (écoulement, 0 pour la mer).
// Deux couches à échelles et directions différentes cassent la répétition visible.
vec3 water_detail_normal_ws(vec2 world_xz, float time, vec2 flow) {
	if (water_detail_strength <= 0.0) {
		return vec3(0.0, 1.0, 0.0);
	}
	vec2 uv = world_xz * water_detail_scale;
	vec3 n1 = texture(water_detail_normal, uv + flow * time).xyz * 2.0 - 1.0;
	vec3 n2 = texture(water_detail_normal, uv.yx * 0.37 + vec2(0.31, 0.57) - flow * time * 0.6).xyz * 2.0 - 1.0;
	// Espace tangent (x, y, z=haut) -> monde (x, haut, z) ; le vert OpenGL pointe vers -Z monde.
	vec3 n = vec3(n1.x + n2.x, 0.0, -(n1.y + n2.y)) * 0.5 * water_detail_strength;
	return normalize(vec3(n.x, 1.0, n.z));
}

// Teinte `base` par le relief de couleur de la texture (albédo divisé par sa moyenne, pour
// garder la couleur du shader hôte) ; `uv` en coordonnées de tuile. `base` si le détail est éteint.
vec3 water_detail_tint(vec3 base, vec2 uv) {
	if (water_detail_strength <= 0.0) {
		return base;
	}
	vec3 mean_color = max(textureLod(water_detail_albedo, vec2(0.5), 9.0).rgb, vec3(0.02));
	vec3 ratio = texture(water_detail_albedo, uv).rgb / mean_color;
	return base * mix(vec3(1.0), clamp(ratio, vec3(0.5), vec3(1.6)), water_detail_strength);
}
