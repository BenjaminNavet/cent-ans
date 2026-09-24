// Fleuves dans le terrain de campagne (lot V4, A1-11), inclus par terrain.gdshader seulement :
// lit creusé (sommets) et berges (fragment). S'appuie sur river_bed.gdshaderinc.

#include "res://shaders/river_bed.gdshaderinc"

uniform vec3 river_mud_color = vec3(0.085, 0.072, 0.048);
uniform vec3 river_bed_color = vec3(0.075, 0.080, 0.060);
uniform vec3 river_grass_color = vec3(0.070, 0.115, 0.035);
// Inclinaison des berges dans l'ombrage (lisible même là où le maillage n'est pas creusé).
uniform float river_bank_slope = 0.4;

float rb_hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float rb_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(rb_hash(i), rb_hash(i + vec2(1.0, 0.0)), u.x), mix(rb_hash(i + vec2(0.0, 1.0)), rb_hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

// Abaissement (unités monde) du sommet en `p` : lit creusé dans les tuiles de relief fin.
float river_carve(vec2 p, vec2 size) {
	if (!has_river_bed || carved_rect_count <= 0) {
		return 0.0;
	}
	float cw = carve_weight(p);
	if (cw <= 0.0) {
		return 0.0;
	}
	return river_carve_depth * river_profile(river_sd_at(p, size)) * cw;
}

// Berges : fond vaseux dans le lit (vu sous l'eau peu profonde), bande de vase et de roseaux au
// bord, herbe grasse sur la berge ; berges inclinées vers l'eau dans l'ombrage (normale `n` en
// repère monde). `footprint` : pixels carte par pixel écran.
void river_banks(vec2 p, vec2 size, float footprint, inout vec3 col, inout float roughness, inout vec3 n) {
	if (!has_river_bed) {
		return;
	}
	float rsd = river_sd_at(p, size);
	if (rsd >= river_bank_px + 0.6) {
		return;
	}
	float vis = 1.0 - smoothstep(river_bank_px * 0.6, river_bank_px * 2.5, footprint);
	float reeds = rb_noise(p * 5.3) * 0.6 + rb_noise(p * 13.1) * 0.4;
	float in_bed = 1.0 - smoothstep(-0.15, 0.05, rsd);
	float mud = (1.0 - smoothstep(0.0, 0.18 + 0.25 * reeds, rsd)) * (1.0 - in_bed);
	float lush = 1.0 - smoothstep(0.1, river_bank_px * (0.8 + 0.5 * reeds), rsd);
	col = mix(col, river_grass_color * (0.8 + 0.4 * reeds), lush * 0.35 * vis);
	col = mix(col, river_mud_color * (0.8 + 0.4 * reeds), mud * 0.85 * vis);
	col = mix(col, river_bed_color, in_bed * vis);
	roughness = mix(roughness, 0.5, mud * vis);
	// Pente de la berge : gradient de la distance (vers l'extérieur du lit).
	vec2 e = vec2(0.35, 0.0);
	vec2 gsd = vec2(river_sd_at(p + e.xy, size) - river_sd_at(p - e.xy, size),
		river_sd_at(p + e.yx, size) - river_sd_at(p - e.yx, size)) / 0.7;
	float slope_w = smoothstep(-0.3, 0.2, rsd) * (1.0 - smoothstep(0.2, river_bank_px, rsd));
	vec2 outward = gsd / max(length(gsd), 1e-4);
	n = normalize(n + vec3(-outward.x, 0.0, -outward.y) * river_bank_slope * slope_w * vis);
}
