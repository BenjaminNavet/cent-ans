// Lot ZG8 (ADR 0036) — hauteur affichée de la carte de campagne, SOURCE UNIQUE côté GPU.
// Double GDScript : `MapData.display_height` (même formule, même échantillonnage des champs) ;
// double Rust : `vegetation::ScatterRequest::display_height` (semis d'arbres natif).
//
//     y = s · (h − K · max(h − base(x, z), 0) + g · (1 − K) · max(h − fond(x, z), 0)),  K = c · k(x, z)
// s : échelle verticale dynamique (ZG4, unités monde par mètre) ; g : gain de relief local (ZG8,
// 0 = comportement ZG4) ; fond : fond de vallée lissé en mètres (≥ 0 : la côte et la mer ne
// bougent pas) ; lot SZ1 : base = fond non plafonné (≤ fond), k = facteur d'écrasement des
// montagnes (amplitude régionale, 0 pour collines et plaines ; le gain local y est écrasé
// d'autant), c = poids de l'écrasement selon l'échelle (0 en vue stratégique, 1 au palier vallée). Grille basse résolution RGBF (`ReliefFloor` :
// R fond, G base, B k), cellule de `info.x` pixels carte, centre de la cellule c en x = c·info.x + info.y.
// Tout shader qui pose une hauteur en MÈTRES sur la carte inclut ce fichier (et ne redéclare pas
// `campaign_vertical_scale`).

global uniform float campaign_vertical_scale;
global uniform float campaign_relief_gain;
global uniform float campaign_relief_squash;
global uniform sampler2D campaign_relief_floor;
global uniform vec4 campaign_relief_floor_info;

// Vrai quand la hauteur affichée dépend des champs (sinon y = s·h, aucune lecture de texture).
bool campaign_relief_active() {
	return campaign_relief_gain != 0.0 || campaign_relief_squash != 0.0;
}

// Champs (fond, base, k) au point monde xz : bilinéaire manuel (texelFetch) entre centres de
// cellules, bords répliqués ; `grad_*` = dérivées (par unité monde) du fond, de la base et de k.
vec3 campaign_relief_fields_grad(vec2 xz, out vec2 grad_floor, out vec2 grad_base, out vec2 grad_k) {
	grad_floor = vec2(0.0);
	grad_base = vec2(0.0);
	grad_k = vec2(0.0);
	if (!campaign_relief_active()) {
		return vec3(0.0);
	}
	ivec2 sz = textureSize(campaign_relief_floor, 0);
	float cell = max(campaign_relief_floor_info.x, 1.0);
	vec2 f = clamp((xz - vec2(campaign_relief_floor_info.y)) / cell, vec2(0.0), vec2(sz - ivec2(1)));
	ivec2 i = min(ivec2(f), sz - ivec2(2));
	vec2 t = f - vec2(i);
	vec3 a = texelFetch(campaign_relief_floor, i, 0).rgb;
	vec3 b = texelFetch(campaign_relief_floor, i + ivec2(1, 0), 0).rgb;
	vec3 c = texelFetch(campaign_relief_floor, i + ivec2(0, 1), 0).rgb;
	vec3 d = texelFetch(campaign_relief_floor, i + ivec2(1, 1), 0).rgb;
	vec3 gx = mix(b - a, d - c, t.y) / cell;
	vec3 gz = mix(c - a, d - b, t.x) / cell;
	grad_floor = vec2(gx.x, gz.x);
	grad_base = vec2(gx.y, gz.y);
	grad_k = vec2(gx.z, gz.z);
	return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
}

vec3 campaign_relief_fields(vec2 xz) {
	vec2 g0;
	vec2 g1;
	vec2 g2;
	return campaign_relief_fields_grad(xz, g0, g1, g2);
}

// Fond de vallée (m) au point monde xz (ZG8).
float campaign_floor_m(vec2 xz) {
	return campaign_relief_fields(xz).x;
}

// `campaign_display_height` avec les champs déjà lus au même point (même expression, même résultat).
float campaign_display_height_at_fields(float h_m, vec3 fields) {
	if (!campaign_relief_active()) {
		return h_m * campaign_vertical_scale;
	}
	float k = campaign_relief_squash * fields.z;
	return campaign_vertical_scale * (h_m - k * max(h_m - fields.y, 0.0)
		+ campaign_relief_gain * (1.0 - k) * max(h_m - fields.x, 0.0));
}

// Hauteur affichée (unités monde) de l'altitude h_m (m) au point monde xz.
float campaign_display_height(float h_m, vec2 xz) {
	return campaign_display_height_at_fields(h_m, campaign_relief_fields(xz));
}

// Gradient de la surface affichée (unités monde par unité monde) à partir du gradient d'altitude
// `grad_m` (m par unité monde) : ombrage et pente des falaises.
vec2 campaign_display_gradient(float h_m, vec2 grad_m, vec2 xz) {
	if (!campaign_relief_active()) {
		return grad_m * campaign_vertical_scale;
	}
	vec2 gf;
	vec2 gb;
	vec2 gk;
	vec3 fields = campaign_relief_fields_grad(xz, gf, gb, gk);
	float above_floor = step(fields.x, h_m);
	float above_base = step(fields.y, h_m);
	float k = campaign_relief_squash * fields.z;
	float g = campaign_relief_gain * (1.0 - k);
	vec2 d = grad_m * (1.0 - k * above_base + g * above_floor)
		+ above_base * (k * gb - campaign_relief_squash * gk * (h_m - fields.y))
		- above_floor * (g * gf + campaign_relief_gain * campaign_relief_squash * gk * (h_m - fields.x));
	return campaign_vertical_scale * d;
}
