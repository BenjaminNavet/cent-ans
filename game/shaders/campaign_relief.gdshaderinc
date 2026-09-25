// Lot ZG8 (ADR 0036) — hauteur affichée de la carte de campagne, SOURCE UNIQUE côté GPU.
// Double GDScript : `MapData.display_height` (même formule, même échantillonnage du fond).
//
//     y = s · (h + g · max(h − fond(x, z), 0))
// s : échelle verticale dynamique (ZG4, unités monde par mètre) ; g : gain de relief local (ZG8,
// 0 = comportement ZG4) ; fond : fond de vallée lissé en mètres (≥ 0 : la côte et la mer ne
// bougent pas), grille basse résolution (`ReliefFloor`, cellule de `info.x` pixels carte, centre
// de la cellule c en x = c·info.x + info.y).
// Tout shader qui pose une hauteur en MÈTRES sur la carte inclut ce fichier (et ne redéclare pas
// `campaign_vertical_scale`).

global uniform float campaign_vertical_scale;
global uniform float campaign_relief_gain;
global uniform sampler2D campaign_relief_floor;
global uniform vec4 campaign_relief_floor_info;

// Fond (m) au point monde xz : bilinéaire manuel (texelFetch) entre centres de cellules, bords
// répliqués ; `grad` = dérivées (m par unité monde).
float campaign_floor_m_grad(vec2 xz, out vec2 grad) {
	grad = vec2(0.0);
	if (campaign_relief_gain == 0.0) {
		return 0.0;
	}
	ivec2 sz = textureSize(campaign_relief_floor, 0);
	float cell = max(campaign_relief_floor_info.x, 1.0);
	vec2 f = clamp((xz - vec2(campaign_relief_floor_info.y)) / cell, vec2(0.0), vec2(sz - ivec2(1)));
	ivec2 i = min(ivec2(f), sz - ivec2(2));
	vec2 t = f - vec2(i);
	float a = texelFetch(campaign_relief_floor, i, 0).r;
	float b = texelFetch(campaign_relief_floor, i + ivec2(1, 0), 0).r;
	float c = texelFetch(campaign_relief_floor, i + ivec2(0, 1), 0).r;
	float d = texelFetch(campaign_relief_floor, i + ivec2(1, 1), 0).r;
	grad = vec2(mix(b - a, d - c, t.y), mix(c - a, d - b, t.x)) / cell;
	return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
}

float campaign_floor_m(vec2 xz) {
	vec2 g;
	return campaign_floor_m_grad(xz, g);
}

// Hauteur affichée (unités monde) de l'altitude h_m (m) au point monde xz.
float campaign_display_height(float h_m, vec2 xz) {
	if (campaign_relief_gain == 0.0) {
		return h_m * campaign_vertical_scale;
	}
	return campaign_vertical_scale * (h_m + campaign_relief_gain * max(h_m - campaign_floor_m(xz), 0.0));
}

// Même chose à une échelle et un gain donnés (rubans cuits).
// `campaign_display_height` avec le fond déjà lu au même point (même expression, même résultat).
float campaign_display_height_at_floor(float h_m, float floor_m) {
	if (campaign_relief_gain == 0.0) {
		return h_m * campaign_vertical_scale;
	}
	return campaign_vertical_scale * (h_m + campaign_relief_gain * max(h_m - floor_m, 0.0));
}

float campaign_display_height_with(float h_m, vec2 xz, float scale, float gain) {
	if (gain == 0.0) {
		return h_m * scale;
	}
	return scale * (h_m + gain * max(h_m - campaign_floor_m(xz), 0.0));
}

// Inverse : altitude (m) dont la hauteur affichée vaut y à cette échelle et ce gain.
float campaign_height_from_display_with(float y, vec2 xz, float scale, float gain) {
	float v = y / max(scale, 1e-9);
	if (gain == 0.0) {
		return v;
	}
	float f = campaign_floor_m(xz);
	return v <= f ? v : (v + gain * f) / (1.0 + gain);
}

// Gradient de la surface affichée (unités monde par unité monde) à partir du gradient d'altitude
// `grad_m` (m par unité monde) : ombrage et pente des falaises.
vec2 campaign_display_gradient(float h_m, vec2 grad_m, vec2 xz) {
	if (campaign_relief_gain == 0.0) {
		return grad_m * campaign_vertical_scale;
	}
	vec2 fg;
	float f = campaign_floor_m_grad(xz, fg);
	float above = step(f, h_m);
	return campaign_vertical_scale * (grad_m * (1.0 + campaign_relief_gain * above) - campaign_relief_gain * above * fg);
}
