// Lot CV-B (ADR 0354) : apparition et disparition sans pop des figurines et accessoires de la
// carte vivante (`FolkPool`). Rendu seulement.
//
// - Chaque instance porte un aléa fixe h ∈ [0, 1) codé dans son échelle : échelle = base ×
//   (1 + FK_H_RANGE × h) (`fk_instance_h`) ; il ne change pas d'un placement à l'autre.
// - Éclaircie stable : l'instance est visible si h < `fk_thin.x` (seuil continu fonction de la
//   distance caméra, posé par le réservoir) ; bande de fondu proportionnelle (`fk_thin.y`).
// - Fondu au bord de la zone de vue (`fk_zone` = x, z du point visé, rayon intérieur, extérieur)
//   et à la portée maximale (`fk_thin.z`, 1 en deçà).
// - Le fondu est tramé (matrice de Bayer 4×4) : pas de transparence, donc pas de tri.
// Le vertex shader appelle `fk_fade_at` (varying `v_fk_fade`), le fragment `fk_dither`.
uniform float fk_base_scale = 1.0;
uniform vec4 fk_zone = vec4(0.0, 0.0, 1.0e9, 2.0e9);
uniform vec4 fk_thin = vec4(2.0, 0.1, 1.0, 0.0);
varying float v_fk_fade;

const float FK_H_RANGE = 0.004;

float fk_instance_h(mat4 model) {
	return clamp((length(model[0].xyz) / max(fk_base_scale, 1e-9) - 1.0) / FK_H_RANGE, 0.0, 0.9999);
}

float fk_fade_at(vec3 world, float h) {
	float edge = 1.0 - smoothstep(fk_zone.z, fk_zone.w, distance(world.xz, fk_zone.xy));
	float thin = clamp((fk_thin.x - h) / max(fk_thin.x * fk_thin.y, 1.0e-4), 0.0, 1.0);
	return edge * thin * fk_thin.z;
}

bool fk_dither(float fade, vec2 frag) {
	const float M[16] = float[16](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
	ivec2 q = ivec2(mod(frag, 4.0));
	return fade < (M[q.x + q.y * 4] + 0.5) / 16.0;
}
