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

uniform float opacity = 0.6;
uniform float wet = 0.0;               // ciel chargé (0-1)
uniform float soft = 0.2;              // transition de bord, en part du champ des bancs (30-40 % du rayon)
uniform float fray = 0.05;             // amplitude du bruit fin de bord (basse fréquence)
uniform float fray_scale = 0.08;
uniform float layer_step = 0.06;
uniform float gain = 5.0;
uniform float bank_scale = 0.45;       // fréquence des bancs, en part de celle des ombres RV-C
uniform vec2 bank_band = vec2(0.66, 0.8);  // seuil du champ des bancs : haut = peu de bancs
uniform float sea_penalty = 0.14;      // seuil relevé en pleine mer
uniform vec3 lit_color : source_color = vec3(1.0, 0.97, 0.92);
uniform vec3 shade_color : source_color = vec3(0.5, 0.56, 0.67);
uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform bool have_provinces = false;

float land_at(vec2 p) {
	if (!have_provinces) {
		return 1.0;
	}
	ivec2 sz = textureSize(province_ids, 0);
	float land = 0.0;
	for (int i = 0; i < 8; i++) {
		float a = float(i) * 0.785398;
		vec2 q = p + vec2(cos(a), sin(a)) * 40.0;
		vec4 c = texelFetch(province_ids, clamp(ivec2(floor(q)), ivec2(0), sz - ivec2(1)), 0);
		land += (c.r + c.g) > 0.001 ? 0.125 : 0.0;
	}
	return land;
}

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
	// Bancs : champ RV-C à basse fréquence (même dérive, même domaine déformé, première octave
	// seulement : pas de petites cellules), seuil haut -> quelques grands bancs, ciel clair entre.
	vec2 drift = TIME * cloud_shadow_wind * cloud_shadow_drift;
	vec2 q = (p - drift) * cloud_shadow_scale * bank_scale;
	vec2 warp = vec2(cs_noise(q * 0.7 + vec2(1.7, 9.2)), cs_noise(q * 0.7 + vec2(8.3, 2.8))) - 0.5;
	float field = cs_noise(q + warp * 1.2) * 0.7 + cs_noise(q * 2.1 + 5.2) * 0.3;
	field += (atm_noise(p * fray_scale + vec2(7.3, 1.9)) - 0.5) * fray * 2.0;
	float region = cs_fbm((p - drift * 0.5) * 0.0035 + vec2(3.3, 7.1));  // champ régional RV-C
	field += (region - 0.5) * 0.25;
	float threshold = bank_band.x + (1.0 - land_at(p)) * sea_penalty - wet * 0.12;
	float above = field - threshold - layer * layer_step;
	float width = max(bank_band.y - bank_band.x, 0.02) * (1.0 + soft * 3.0);
	float thick = smoothstep(0.0, width, above);
	if (thick <= 0.003) {
		discard;
	}
	// Éclairage : le champ baisse du côté du soleil sur les flancs éclairés.
	vec2 toward_sun = normalize(atm_sun_dir.xz + vec2(1e-4)) * 14.0;
	vec2 qs = (p + toward_sun - drift) * cloud_shadow_scale * bank_scale;
	float field_sun = cs_noise(qs + warp * 1.2) * 0.7 + cs_noise(qs * 2.1 + 5.2) * 0.3;
	float base_field = cs_noise(q + warp * 1.2) * 0.7 + cs_noise(q * 2.1 + 5.2) * 0.3;
	float lit = clamp(0.55 + (base_field - field_sun) * gain * 4.0, 0.0, 1.0);
	lit = clamp(lit * (0.55 + 0.5 * layer) + 0.2 * smoothstep(0.0, 0.3, above), 0.0, 1.0);
	vec3 col = mix(shade_color, lit_color * atm_sun_color, smoothstep(0.1, 0.9, lit));
	col = mix(shade_color, col, smoothstep(0.0, 0.35, thick));
	float depth_fade = smoothstep(40.0, 160.0, length(VERTEX));
	float a = thick * opacity * atm_fade * depth_fade * atm_edge_fade(p)
			* atm_focus_clear(world_pos, CAMERA_POSITION_WORLD);
	ALBEDO = col;
	ALPHA = clamp(a, 0.0, 0.7);
}
