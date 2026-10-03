// Lot RJ-d (ADR 0175) — lavis translucide par position diplomatique du contrôleur de chaque
// province (or : nous, vert : amis, rouge : ennemis, gris léger : autres).
// Inclus par terrain.gdshader et terrain_parchment.gdshader APRÈS parchment_map.gdshaderinc
// (s'appuie sur pm_weight) ; crochet d'une ligne juste avant `fr1_borders` :
// `sf_fill(id, albedo, emission)` — les frontières et hachures d'occupation passent par-dessus.
// Uniformes posés par StanceFill (game/scripts/map/stance_fill.gd) ; `sf_alpha` vaut 0 par
// défaut (sans StanceFill : aucun effet, un seul test uniforme).

// Par province (index raster) : rgb = couleur de position (sRGB), a = opacité de la catégorie.
uniform sampler2D sf_colors : filter_nearest, repeat_disable, hint_default_transparent;
uniform float sf_alpha = 0.0;            // option × mode de carte × atténuation de près
uniform float sf_saturation = 0.6;
uniform float sf_parchment_scale = 0.55;

// Teinte à luminance conservée (colore sans délaver ni assombrir le relief), comme la teinte de
// faction du terrain ; même opération sur l'émission (part du papier en vue parchemin).
vec3 sf_tint(vec3 c, vec3 ratio, float a) {
	float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
	return mix(c, l * ratio, a);
}

void sf_fill(int id, inout vec3 albedo, inout vec3 emission) {
	if (sf_alpha <= 0.0 || id <= 0) {
		return;
	}
	vec4 c = texelFetch(sf_colors, ivec2(clamp(id, 0, textureSize(sf_colors, 0).x - 1), 0), 0);
	if (c.a <= 0.0) {
		return;
	}
	vec3 lin = pow(c.rgb, vec3(2.2));
	float lf = max(dot(lin, vec3(0.2126, 0.7152, 0.0722)), 0.02);
	vec3 ratio = mix(vec3(1.0), lin / lf, sf_saturation);
	float a = c.a * sf_alpha * mix(1.0, sf_parchment_scale, pm_weight());
	albedo = sf_tint(albedo, ratio, a);
	emission = sf_tint(emission, ratio, a);
}
