shader_type spatial;
render_mode blend_mix, depth_draw_never, cull_disabled, unshaded, shadows_disabled;

// Lot ME5 : champ de cumulus du ciel, LE MÊME champ que l'ombre des cumulus RV-C
// (campaign_cloud_shadow.gdshaderinc : même bruit, même dérive, même soleil) : chaque ombre au sol
// a son nuage au-dessus. Trois plans empilés (INSTANCE_CUSTOM.x = 0, 0.5, 1) : le plan bas
// (altitude de l'ombre) s'étend jusqu'au seuil, les plans hauts ne gardent que le cœur des bancs,
// d'où des amas bombés à bord effiloché. Base plate plus sombre (plan bas), sommet éclairé du côté
// du soleil (pente du champ vers le soleil), bord transparent sans halo.

#include "res://shaders/map_atmosphere_common.gdshaderinc"
#include "res://shaders/campaign_cloud_shadow.gdshaderinc"

uniform float opacity = 0.7;
uniform float wet = 0.0;               // ciel chargé (0-1), comme `wet` des ombres
uniform float soft = 0.16;             // largeur de la transition de bord (champ brut)
uniform float fray = 0.1;              // amplitude du bruit fin qui effiloche le bord
uniform float fray_scale = 0.28;       // fréquence du bruit fin (1 / px carte)
uniform float layer_step = 0.075;      // retrait du seuil par plan (champ brut)
uniform float gain = 5.0;
uniform vec2 bank_band = vec2(0.42, 0.62);  // seuil du champ régional (RV-C, ~200 km) : bancs et grands ciels clairs
uniform float bank_drift = 0.3;              // contraste de l'éclairage par la pente du champ
uniform vec3 lit_color : source_color = vec3(1.0, 0.97, 0.92);
uniform vec3 shade_color : source_color = vec3(0.56, 0.62, 0.72);

varying vec3 world_pos;
varying float layer;

void vertex() {
	layer = INSTANCE_CUSTOM.x;
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	if (atm_fade <= 0.001) {
		discard;
	}
	vec2 p = world_pos.xz;
	float footprint = sqrt(length(dFdx(p)) * length(dFdy(p)));
	float threshold;
	float edge;
	float c = cs_cumulus_field(p, wet, footprint, threshold, edge);
	// Bord effiloché : bruit fin, retiré quand il passerait sous le pixel.
	float fine = 1.0 - smoothstep(0.3, 0.9, footprint * fray_scale * 2.0);
	c += (atm_fbm(p * fray_scale + vec2(7.3, 1.9)) - 0.5) * fray * 2.0 * fine;
	float above = c - threshold - layer * layer_step * 3.0;
	// Bancs : seul le champ régional de RV-C (même expression, même dérive) laisse passer les nuages.
	vec2 drift = TIME * cloud_shadow_wind * cloud_shadow_drift;
	float region = cs_fbm((p - drift * 0.5) * 0.0035 + vec2(3.3, 7.1));
	float bank = smoothstep(bank_band.x, bank_band.y, region + wet * 0.2);
	float thick = smoothstep(0.0, soft + edge, above) * bank;
	if (thick <= 0.003) {
		discard;
	}
	// Éclairage : le champ baisse du côté du soleil sur les flancs éclairés.
	vec2 toward_sun = normalize(atm_sun_dir.xz + vec2(1e-4)) * 7.0;
	float thr2;
	float edge2;
	float c_sun = cs_cumulus_field(p + toward_sun, wet, footprint, thr2, edge2);
	float lit = clamp(0.55 + (c - c_sun) * gain, 0.0, 1.0);
	// Sommet plus clair que la base ; bord et plan bas plus sombres.
	lit = clamp(lit * (0.55 + 0.5 * layer) + 0.2 * smoothstep(0.0, 0.3, above), 0.0, 1.0);
	vec3 col = mix(shade_color, lit_color * atm_sun_color, smoothstep(0.1, 0.9, lit));
	col = mix(shade_color, col, smoothstep(0.0, 0.35, thick));  // pas de halo clair au bord
	float depth_fade = smoothstep(40.0, 160.0, length(VERTEX));
	float a = thick * opacity * atm_fade * depth_fade * atm_edge_fade(p)
			* atm_focus_clear(world_pos, CAMERA_POSITION_WORLD);
	ALBEDO = col;
	ALPHA = clamp(a, 0.0, 0.8);
}
