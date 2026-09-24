shader_type spatial;
render_mode cull_back, depth_draw_opaque;

// Terrain de campagne semi-réaliste (lot V2, ADR 0004).
// - Relief : normale dérivée de la heightmap (R16 filtrée + mipmaps), pas proportionnel à
//   l'empreinte du pixel à l'écran (ombrage doux, sans bruit au dézoom).
// - Matériaux : splatmap hors ligne (R prairie, G cultures, B forêt, A roche/lande) + neige
//   (altitude) + sable (distance à la côte), textures PBR Poly Haven en Texture2DArray,
//   teintées vers des albédos réalistes ; parcellaire procédural sur les cultures.
// - Politique : teinte de faction discrète de près, plus lisible au dézoom ; frontières par
//   champs de distance signés (trait antialiasé d'épaisseur constante à l'écran), frontières
//   de royaume plus marquées avec liseré de la couleur de faction.
// - Surbrillance survol/sélection et masque d'atteignabilité inchangés.

uniform sampler2D heightmap : filter_linear_mipmap, repeat_disable, hint_default_black;
uniform int height_bpp = 1;               // 1 = canal R normalisé (R16/L8), 2 = LA8 big-endian
uniform bool height_little_endian = false; // LA8 seulement
uniform float height_min_m = -200.0;
uniform float height_max_m = 4800.0;
uniform float height_scale = 0.006;       // unités monde par mètre (MapData.HEIGHT_SCALE)
uniform vec2 map_size = vec2(4096.0, 4096.0);
uniform float meters_per_px = 719.0;

uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform sampler2D faction_colors : filter_nearest, repeat_disable, hint_default_transparent;
uniform int hovered_id = 0;
uniform int selected_id = 0;
// Masque 1D par province (R8) : ~0,5 = atteignable par l'armée sélectionnée, 1 = sur le chemin.
uniform sampler2D province_mask : filter_nearest, repeat_disable, hint_default_black;
uniform bool mask_enabled = false;
uniform vec3 reachable_color : source_color = vec3(0.98, 0.93, 0.55);
uniform vec3 path_color : source_color = vec3(1.0, 0.55, 0.15);

uniform sampler2D splat : filter_linear, repeat_disable, hint_default_black;
uniform bool has_splat = false;
uniform sampler2D border_dist : filter_linear, repeat_disable, hint_default_white;
uniform bool has_border_dist = false;
uniform float border_dist_scale = 6.0;    // niveaux 8 bits par pixel carte (tools/geo/splat.py)
uniform sampler2D coast_dist : filter_linear, repeat_disable, hint_default_white;
uniform bool has_coast_dist = false;
uniform float coast_dist_scale = 2.0;

uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray normal_rough_array : filter_linear_mipmap_anisotropic, repeat_enable;
uniform bool has_textures = false;
// Moyenne linéaire de chaque albédo (calculée par TerrainBuilder) et teinte réaliste visée.
uniform vec3 layer_mean[7];
// Teintes linéaires : prairie, cultures, forêt, roche, lande, neige, sable.
uniform vec3 tint_grass = vec3(0.105, 0.165, 0.045);
uniform vec3 tint_farmland = vec3(0.21, 0.20, 0.075);
uniform vec3 tint_forest = vec3(0.030, 0.062, 0.022);
uniform vec3 tint_rock = vec3(0.19, 0.17, 0.15);
uniform vec3 tint_heath = vec3(0.15, 0.11, 0.08);
uniform vec3 tint_snow = vec3(0.78, 0.80, 0.84);
uniform vec3 tint_sand = vec3(0.48, 0.40, 0.26);
// Parcellaire (lot V2b) : couleurs linéaires des cultures et des prés.
uniform vec3 crop_wheat = vec3(0.36, 0.265, 0.075);
uniform vec3 crop_green = vec3(0.12, 0.175, 0.045);
uniform vec3 crop_ploughed = vec3(0.15, 0.10, 0.055);
uniform vec3 crop_fallow = vec3(0.19, 0.175, 0.075);
uniform vec3 meadow_color = vec3(0.095, 0.16, 0.042);
uniform vec3 hay_color = vec3(0.24, 0.215, 0.085);
uniform vec3 vine_soil = vec3(0.19, 0.135, 0.075);
uniform vec3 vine_leaf = vec3(0.07, 0.10, 0.03);
uniform vec3 dry_grass = vec3(0.23, 0.19, 0.085);
uniform vec3 track_color = vec3(0.26, 0.20, 0.12);
uniform vec3 hedge_color = vec3(0.035, 0.05, 0.02);
// Occupation du sol (VegetationFields.landuse) : R vigne, G sécheresse, B bocage.
uniform sampler2D landuse : filter_linear, repeat_disable, hint_default_black;
uniform bool has_landuse = false;
uniform vec3 lake_color = vec3(0.035, 0.10, 0.115);
uniform vec3 seabed_color = vec3(0.03, 0.07, 0.07);
// Couleurs de water.gdshader (mer peinte là où le maillage dépasse le plan d'eau).
uniform vec3 sea_shallow_color = vec3(0.045, 0.14, 0.14);
uniform vec3 sea_deep_color = vec3(0.006, 0.028, 0.050);

// Détail : taille d'une tuile de texture (pixels carte), part du détail, force des normales.
uniform float tile_px = 3.0;
uniform float farm_tile_px = 0.6;
uniform float tile_screen_px = 72.0;
uniform float detail_strength = 0.85;
uniform float detail_normal_strength = 0.55;
// Parcellaire : demi-largeur (px carte) du pied de haie et des chemins.
uniform float hedge_width = 0.16;
uniform float track_width = 0.10;
// Neige : altitude de base (m) ; roche au-dessus de rock_line.
uniform float snow_line_m = 2750.0;
uniform float rock_line_m = 1700.0;
// Exagération de l'ombrage par rapport à la géométrie (lisibilité du relief).
uniform float shading_relief = 1.8;

// Politique : empreinte (pixels carte par pixel écran) où la couleur de faction prend le dessus.
uniform float political_start = 0.9;
uniform float political_full = 2.8;
uniform float faction_alpha_near = 0.10;
uniform float faction_alpha_far = 0.50;
uniform float faction_saturation = 0.55;
uniform float political_flat_gain = 0.45;
uniform vec3 border_color = vec3(0.012, 0.009, 0.006);
uniform float province_border_px = 0.55;  // demi-largeur (pixels écran)
uniform float province_border_alpha = 0.55;
uniform float realm_border_px = 1.25;
uniform float realm_border_alpha = 0.9;
uniform float realm_band_px = 7.0;        // liseré intérieur de couleur de faction
uniform float realm_band_alpha = 0.55;

varying vec3 world_pos;

float height01_lod(vec2 uv, float lod) {
	if (height_bpp == 2) {
		vec4 t = texture(heightmap, uv);
		if (height_little_endian) {
			return (t.a * 255.0 * 256.0 + t.r * 255.0) / 65535.0;
		}
		return (t.r * 255.0 * 256.0 + t.a * 255.0) / 65535.0;
	}
	return textureLod(heightmap, uv, lod).r;
}

float height_m_lod(vec2 uv, float lod) {
	return mix(height_min_m, height_max_m, height01_lod(uv, lod));
}

int province_at(vec2 p) {
	ivec2 sz = textureSize(province_ids, 0);
	ivec2 texel = clamp(ivec2(floor(p)), ivec2(0), sz - ivec2(1));
	vec4 c = texelFetch(province_ids, texel, 0);
	return int(round(c.r * 255.0)) | (int(round(c.g * 255.0)) << 8);
}

vec4 faction_of(int id) {
	if (id <= 0) {
		return vec4(0.0);
	}
	return texelFetch(faction_colors, ivec2(clamp(id, 0, textureSize(faction_colors, 0).x - 1), 0), 0);
}

float hash12(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float value_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	float a = hash12(i);
	float b = hash12(i + vec2(1.0, 0.0));
	float c = hash12(i + vec2(0.0, 1.0));
	float d = hash12(i + vec2(1.0, 1.0));
	return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	for (int i = 0; i < 4; i++) {
		v += a * value_noise(p);
		p = p * 2.03 + vec2(17.1, 9.7);
		a *= 0.5;
	}
	return v;
}

// Distance (pixels carte) au trait de frontière le plus proche : min des |canaux signés|.
float border_d(vec2 uv) {
	vec3 s = (texture(border_dist, uv).rgb * 255.0 - 128.0) / border_dist_scale;
	return min(abs(s.r), min(abs(s.g), abs(s.b)));
}

// Échantillon d'une couche à deux échelles (anti-répétition) : albédo teinté + normale/rugosité.
void sample_layer(int layer, vec2 uv_a, vec2 uv_b, float mix_b, vec3 tint, float detail,
		inout vec3 albedo, inout vec3 nrm, float w) {
	vec3 ta = texture(albedo_array, vec3(uv_a, float(layer))).rgb;
	vec3 tb = texture(albedo_array, vec3(uv_b, float(layer))).rgb;
	vec3 t = mix(ta, tb, mix_b);
	vec3 na = texture(normal_rough_array, vec3(uv_a, float(layer))).rgb;
	vec3 nb = texture(normal_rough_array, vec3(uv_b, float(layer))).rgb;
	vec3 n = mix(na, nb, mix_b);
	vec3 ratio = t / max(layer_mean[layer], vec3(0.01));
	// Détail surtout en luminance : pas de reflets colorés (bleutés) des photos.
	ratio = mix(vec3(dot(ratio, vec3(0.2126, 0.7152, 0.0722))), ratio, 0.35);
	albedo += w * tint * mix(vec3(1.0), ratio, detail);
	nrm += w * vec3(n.rg * 2.0 - 1.0, n.b);
}

// --- Parcellaire partagé avec vegetation_fields.gd (mêmes constantes, même hachage) ---
float hash01i(int n) {
	uint h = (uint(n) * 1103515245u + 12345u) & 0x7fffffffu;
	h = ((h ^ (h >> 13u)) * 1274126177u) & 0x7fffffffu;
	return float(h % 100000u) / 100000.0;
}

float region_value(vec2 p) {
	return sin(0.0131 * p.x + 1.3 * sin(0.0093 * p.y)) + sin(0.0117 * p.y + 1.1 * sin(0.0171 * p.x)) - 0.25;
}

vec2 field_warp(vec2 q, float ph) {
	float ox = 1.7 * sin(0.071 * q.y + 1.9 * sin(0.027 * q.x) + ph) + 0.55 * sin(0.23 * q.y + 1.3 * sin(0.061 * q.x));
	float oy = 1.7 * sin(0.063 * q.x + 1.7 * sin(0.023 * q.y) + ph * 1.3) + 0.55 * sin(0.19 * q.x + 1.1 * sin(0.047 * q.y));
	return vec2(ox, oy);
}

// Parcelle au point p (trame `lay`). Sorties : tirages d'enclos et de lanière, distances (px carte)
// au bord à u constant / de rangée et tirages de ces bords, distance au bord de lanière, direction
// des lanières (repère carte) et coordonnée transverse (rangs de vigne).
void field_at(vec2 p, int lay, out float cell_h, out float strip_h, out float du, out float dv,
		out float roll_u, out float roll_v, out float strip_d, out vec2 strip_dir, out float rows) {
	float k = lay == 0 ? 0.35 : -0.8;
	float fu = lay == 0 ? 5.0 : 4.6;
	float fv = lay == 0 ? 4.2 : 3.8;
	float ph = lay == 0 ? 0.0 : 2.1;
	vec2 q = p;
	for (int i = 0; i < 4; i++) {
		q = p - field_warp(q, ph);
	}
	float u = (q.x + k * q.y) / fu;
	float v = (q.y - k * q.x) / fv;
	float inv = inversesqrt(1.0 + k * k);
	int cu = int(floor(u));
	float vs = v + hash01i(cu * 5023 + 17 + lay * 101);
	int cv = int(floor(vs));
	float fu_f = fract(u);
	float fv_f = fract(vs);
	du = min(fu_f, 1.0 - fu_f) * fu * inv;
	dv = min(fv_f, 1.0 - fv_f) * fv * inv;
	int uline = fu_f < 0.5 ? cu : cu + 1;
	int vline = fv_f < 0.5 ? cv : cv + 1;
	roll_u = hash01i(uline * 7919 + int(floor(v)) * 104729 + lay * 7 + 1);
	roll_v = hash01i(vline * 7919 + cu * 104729 + lay * 7 + 31);
	int cell_key = cu * 7919 + cv * 104729 + lay * 7 + 3;
	cell_h = hash01i(cell_key);
	float h2 = hash01i(cell_key + 11);
	float h3 = hash01i(cell_key + 23);
	// Lanières : 1 à 4 par enclos, dans l'un ou l'autre sens, bords légèrement ondulés.
	float n = 1.0 + floor(h2 * 4.0);
	bool along_u = h3 < 0.5;
	float t = along_u ? fu_f : fv_f;
	t += 0.035 * sin((along_u ? v : u) * 9.0 + h2 * 6.0);
	float ts = clamp(t, 0.0, 0.999) * n;
	float strip = floor(ts);
	strip_h = hash01i(cell_key + int(strip) * 13 + 5);
	float sf = fract(ts);
	strip_d = n > 1.0 ? min(sf, 1.0 - sf) / n * (along_u ? fu : fv) * inv : 1e3;
	strip_dir = along_u ? normalize(vec2(-k, 1.0)) : normalize(vec2(1.0, k));
	rows = ts;
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 p = world_pos.xz;
	vec2 uv = (p + vec2(0.5)) / map_size;
	// Empreinte d'un pixel écran en pixels carte (≈ 0,3 de près, ≈ 2,5 France entière).
	float footprint = max(length(dFdx(p)), length(dFdy(p)));

	// --- Relief ---
	float k = max(1.0, footprint * 1.5);
	float lod = log2(k);
	vec2 du = vec2(k / map_size.x, 0.0);
	vec2 dv = vec2(0.0, k / map_size.y);
	float hc = height_m_lod(uv, max(lod - 1.0, 0.0));
	float h_l = height_m_lod(uv - du, lod);
	float h_r = height_m_lod(uv + du, lod);
	float h_u = height_m_lod(uv - dv, lod);
	float h_d = height_m_lod(uv + dv, lod);
	vec2 grad_m = vec2(h_r - h_l, h_d - h_u) / (2.0 * k);     // mètres par pixel carte
	float slope = length(grad_m) / meters_per_px;              // pente réelle (m/m)
	vec2 g = grad_m * height_scale * shading_relief;
	vec3 n = normalize(vec3(-g.x, 1.0, -g.y));

	float coast = has_coast_dist ? (texture(coast_dist, uv).r * 255.0 - 128.0) / coast_dist_scale : (hc > 0.0 ? 8.0 : -8.0);
	float land_f = smoothstep(-0.4, 0.6, coast);

	// --- Poids des matériaux ---
	vec4 s = has_splat ? texture(splat, uv) : vec4(0.5, 0.3, 0.2, 0.0);
	s /= max(s.r + s.g + s.b + s.a, 1e-3);
	float macro = fbm(p * 0.035);
	float rock_frac = max(smoothstep(rock_line_m - 700.0, rock_line_m, hc), smoothstep(0.35, 0.6, slope));
	float w_grass = s.r;
	float w_farm = s.g;
	float w_forest = s.b;
	float w_rock = s.a * rock_frac;
	float w_heath = s.a * (1.0 - rock_frac);
	float snow = smoothstep(snow_line_m - 150.0, snow_line_m + 250.0, hc + (macro - 0.5) * 500.0)
		* (1.0 - smoothstep(0.55, 0.9, slope));
	float sand = (1.0 - smoothstep(0.3, 1.6, coast)) * (1.0 - smoothstep(4.0, 30.0, hc)) * land_f;
	float keep = (1.0 - snow) * (1.0 - sand);
	w_grass *= keep; w_farm *= keep; w_forest *= keep; w_rock *= keep; w_heath *= keep;

	// Variation macro : légères nuances de teinte pour casser l'uniformité.
	float tone = 0.85 + 0.3 * macro;

	// --- Parcellaire (lot V2b) : enclos, lanières de cultures, prés, vignes, haies, chemins ---
	vec3 lu = has_landuse ? texture(landuse, uv).rgb : vec3(0.0);
	float dry = lu.g;
	float open_land = clamp(s.g + 0.6 * s.r, 0.0, 1.0);
	float grass_share = s.r / max(s.r + s.g, 1e-3);
	// Teintes de base (lointain) : même moyenne que les parcelles, sèches au sud.
	vec3 grass_base = mix(tint_grass, dry_grass, dry * 0.75);
	vec3 farm_base = mix(tint_farmland, mix(tint_farmland, crop_wheat, 0.35) * vec3(1.05, 0.98, 0.9), dry * 0.6);
	vec3 grass_tint = grass_base;
	vec3 farm_tint = farm_base;
	float hedge = 0.0;
	float track = 0.0;
	float balk = 0.0;
	float vine_rows = 0.0;
	vec2 furrow_dir = vec2(1.0, 0.0);
	float cell_px = 4.4 / max(footprint, 1e-4);  // taille d'un enclos à l'écran
	float field_vis = smoothstep(4.0, 12.0, cell_px);
	if (field_vis > 0.0 && (s.g + s.r) > 0.05) {
		float rv = region_value(p);
		int lay = rv > 0.0 ? 1 : 0;
		float cell_h; float strip_h; float du; float dv; float roll_u; float roll_v; float strip_d; float rows;
		field_at(p, lay, cell_h, strip_h, du, dv, roll_u, roll_v, strip_d, furrow_dir, rows);
		// Lanières visibles de plus près que les enclos : sinon, couleur de l'enclos.
		float strip_vis = smoothstep(3.0, 8.0, cell_px * 0.4);
		float h = strip_vis > 0.5 ? strip_h : cell_h;
		float h_b = hash01i(int(h * 99991.0) + 7);
		// Prés plus fréquents dans les zones herbagères, vigne selon l'occupation du sol.
		float p_meadow = mix(0.15, 0.85, grass_share);
		vec3 crop;
		bool vine = h_b < lu.r * 0.32 && grass_share < 0.8;
		if (vine) {
			crop = vine_soil;
		} else if (h < p_meadow) {
			float m = h / max(p_meadow, 1e-3);
			crop = m < 0.6 ? meadow_color * (0.85 + 0.35 * h_b) : (m < 0.85 ? mix(meadow_color, hay_color, 0.55) : hay_color);
			crop = mix(crop, dry_grass, dry * 0.7);
		} else {
			float c = (h - p_meadow) / max(1.0 - p_meadow, 1e-3);
			crop = c < 0.36 ? crop_wheat : (c < 0.6 ? crop_green : (c < 0.76 ? crop_ploughed : crop_fallow));
			crop *= 0.9 + 0.2 * h_b;
			crop = mix(crop, crop * vec3(1.08, 0.96, 0.82), dry);
		}
		// Couleurs des parcelles plus tard que leur tracé : pas de damier bruité au zoom moyen.
		float crop_fade = smoothstep(6.0, 24.0, cell_px);
		grass_tint = mix(grass_base, crop, crop_fade * 0.8);
		farm_tint = mix(farm_base, crop, crop_fade * 0.85);
		// Rangs de vigne (de près seulement ; au-delà, teinte moyenne des rangs).
		if (vine) {
			float rw = fwidth(rows * 5.0);
			float r = abs(fract(rows * 5.0) - 0.5);
			float fine = 1.0 - smoothstep(0.25, 0.6, rw);
			vine_rows = mix(0.45, 1.0 - smoothstep(0.22 - rw, 0.22 + rw, r), fine) * field_vis;
		}
		// Bords : haie (tirage < probabilité), chemin de terre (autre tirage), sinon talus herbeux.
		float p_hedge = open_land * mix(0.1, 0.75, lu.b);
		float aa = footprint * 0.75;
		float hedge_u = step(roll_u, p_hedge) * (1.0 - smoothstep(hedge_width - aa, hedge_width + aa, du));
		float hedge_v = step(roll_v, p_hedge) * (1.0 - smoothstep(hedge_width - aa, hedge_width + aa, dv));
		// Au-delà d'un pixel, la ligne s'estompe (couverture) au lieu de disparaître.
		float cover = clamp(2.0 * hedge_width / max(footprint, 1e-4), 0.0, 1.0);
		hedge = max(hedge_u, hedge_v) * mix(cover, 1.0, cover) * field_vis;
		float tr_u = step(fract(roll_u * 7.31), 0.07) * (1.0 - smoothstep(track_width - aa, track_width + aa, du));
		float tr_v = step(fract(roll_v * 7.31), 0.07) * (1.0 - smoothstep(track_width - aa, track_width + aa, dv));
		// Limite entre les deux trames : chemin continu.
		float rgrad = length(vec2(region_value(p + vec2(1.0, 0.0)) - rv, region_value(p + vec2(0.0, 1.0)) - rv));
		float rd = abs(rv) / max(rgrad, 1e-4);
		float tr_r = 1.0 - smoothstep(track_width * 1.3 - aa, track_width * 1.3 + aa, rd);
		float tcover = clamp(2.0 * track_width / max(footprint, 1e-4), 0.0, 1.0);
		track = max(max(tr_u, tr_v), tr_r) * tcover * field_vis;
		float edge = min(min(du, dv), strip_d);
		balk = (1.0 - smoothstep(0.03, 0.03 + aa * 2.0, edge)) * smoothstep(8.0, 20.0, cell_px) * 0.5;
	}

	// --- Textures ---
	float detail_fade = 1.0 - smoothstep(0.7, 1.8, footprint);
	vec3 albedo = vec3(0.0);
	vec3 dn = vec3(0.0, 0.0, 0.0);
	float wsum = w_grass + w_farm + w_forest + w_rock + w_heath + snow + sand;
	if (has_textures && detail_fade > 0.0) {
		// Échelle de tuilage adaptée au zoom : une tuile garde ~tile_screen_px pixels à l'écran
		// (pas de trame de points répétés au zoom moyen) ; fondu entre deux octaves d'échelle,
		// la seconde tournée pour casser l'alignement.
		float lvl = max(log2(tile_screen_px * footprint / tile_px), 0.0);
		float tile_a = tile_px * exp2(floor(lvl));
		float ang = 0.9;
		vec2 uv_a = p / tile_a + 0.21 * floor(lvl);
		vec2 uv_b = mat2(vec2(cos(ang), sin(ang)), vec2(-sin(ang), cos(ang))) * p / (tile_a * 2.0) + 0.37;
		float mix_b = fract(lvl);
		float det = detail_strength * detail_fade;
		if (w_grass > 0.01) { sample_layer(0, uv_a, uv_b, mix_b, grass_tint, det, albedo, dn, w_grass); }
		if (w_farm > 0.01) {
			// Sillons dans le sens des lanières.
			mat2 rot = mat2(vec2(furrow_dir.x, -furrow_dir.y), vec2(furrow_dir.y, furrow_dir.x));
			// Tuile plus fine que les parcelles (sillons) : sous le pixel dès le zoom moyen, donc
			// moyennée par les mipmaps (pas de trame régulière visible).
			vec2 uv_f = rot * p / farm_tile_px;
			vec3 tf = textureGrad(albedo_array, vec3(uv_f, 1.0), rot * dFdx(p) / farm_tile_px, rot * dFdy(p) / farm_tile_px).rgb;
			vec3 nf = textureGrad(normal_rough_array, vec3(uv_f, 1.0), rot * dFdx(p) / farm_tile_px, rot * dFdy(p) / farm_tile_px).rgb;
			vec3 ratio = tf / max(layer_mean[1], vec3(0.01));
			ratio = mix(vec3(dot(ratio, vec3(0.2126, 0.7152, 0.0722))), ratio, 0.35);
			albedo += w_farm * farm_tint * mix(vec3(1.0), ratio, det * field_vis * 0.7);
			vec2 nxy = transpose(rot) * (nf.rg * 2.0 - 1.0);
			dn += w_farm * vec3(nxy, nf.b);
		}
		if (w_forest > 0.01) { sample_layer(2, uv_a, uv_b, mix_b, tint_forest, det, albedo, dn, w_forest); }
		if (w_rock > 0.01) { sample_layer(3, uv_a, uv_b, mix_b, tint_rock, det, albedo, dn, w_rock); }
		if (w_heath > 0.01) { sample_layer(4, uv_a, uv_b, mix_b, tint_heath, det, albedo, dn, w_heath); }
		if (snow > 0.01) { sample_layer(5, uv_a, uv_b, mix_b, tint_snow, det, albedo, dn, snow); }
		if (sand > 0.01) { sample_layer(6, uv_a, uv_b, mix_b, tint_sand, det, albedo, dn, sand); }
		albedo /= max(wsum, 1e-3);
		dn /= max(wsum, 1e-3);
	} else {
		albedo = (w_grass * grass_tint + w_farm * farm_tint + w_forest * tint_forest + w_rock * tint_rock
			+ w_heath * tint_heath + snow * tint_snow + sand * tint_sand) / max(wsum, 1e-3);
		dn = vec3(0.0, 0.0, 0.85);
	}
	vec3 col = albedo * mix(1.0, tone, 1.0 - snow);
	// Canopée : ombrage de volume léger (les forêts paraissent plus sombres et granuleuses).
	col *= 1.0 - 0.25 * w_forest / max(wsum, 1e-3) * (1.0 - value_noise(p * 1.7)) * detail_fade;
	float open_w = (w_grass + w_farm) / max(wsum, 1e-3);
	col = mix(col, col * vec3(0.93, 0.95, 0.9), balk * open_w);
	col = mix(col, vine_leaf, vine_rows * open_w * 0.8);
	col = mix(col, track_color * (0.9 + 0.2 * macro), track * open_w * 0.85);
	col = mix(col, hedge_color, hedge * open_w * 0.55);

	// Normales de détail combinées au relief (blend « whiteout » dans le repère carte).
	float dstrength = detail_normal_strength * detail_fade;
	n = normalize(vec3(n.x + dn.x * dstrength, n.y, n.z - dn.y * dstrength));
	float roughness = has_textures && detail_fade > 0.0 ? clamp(dn.z, 0.55, 1.0) : 0.9;

	// Fond marin (sous le plan d'eau) et lacs.
	if (hc <= 0.0) {
		col = mix(tint_sand * 0.5, seabed_color, smoothstep(-2.0, -30.0, hc));
	}
	// Eau peinte sur le terrain : lacs (au-dessus du niveau de la mer) et mer là où le maillage
	// LOD dépasse le plan d'eau alors que la côte réelle (raster) est plus loin. Pour la mer, même
	// rendu que water.gdshader (couleur selon la profondeur composée sur le fond marin).
	float lake = (1.0 - land_f) * max(step(0.0, hc), step(-0.02, world_pos.y));
	vec3 water_col = lake_color;
	if (hc <= 0.0) {
		float depth = -hc;
		vec3 wc = mix(sea_shallow_color, sea_deep_color, 1.0 - exp(-depth / 70.0));
		water_col = mix(col, wc, mix(0.7, 0.97, smoothstep(0.0, 45.0, depth)));
	}
	col = mix(col, water_col, lake);
	roughness = mix(roughness, 0.3, lake);
	n = normalize(mix(n, vec3(0.0, 1.0, 0.0), lake));

	// --- Couche politique ---
	float political = smoothstep(political_start, political_full, footprint);
	// Au dézoom maximal : terrain un peu désaturé, façon carte.
	float lum = dot(col, vec3(0.2126, 0.7152, 0.0722));
	col = mix(col, vec3(lum) * vec3(1.02, 1.0, 0.95), 0.3 * political);

	int id = province_at(p);
	float d = 1e3;
	vec2 away = vec2(0.0);
	int other_id = id;
	if (has_border_dist) {
		d = border_d(uv);
		if (d < 3.0 + realm_band_px * footprint) {
			vec2 e = vec2(1.0 / map_size.x, 0.0);
			vec2 f = vec2(0.0, 1.0 / map_size.y);
			vec2 gd = vec2(border_d(uv + e) - border_d(uv - e), border_d(uv + f) - border_d(uv - f));
			away = gd / max(length(gd), 1e-4);
			// Côté déterminé par le trait lissé (et non par le texel) : pas d'escalier de près.
			id = province_at(p + away * max(0.0, 1.3 - d));
			other_id = province_at(p - away * (d + 1.3));
		}
	}
	vec4 fc = faction_of(id);
	vec3 fc_lin = pow(fc.rgb, vec3(2.2));
	float faction_amount = mix(faction_alpha_near, faction_alpha_far, political);
	if (id > 0) {
		// Teinte de faction à luminance constante (colore sans délaver), puis au dézoom
		// glissement vers un aplat de couleur politique.
		float lc = dot(col, vec3(0.2126, 0.7152, 0.0722));
		float lf = max(dot(fc_lin, vec3(0.2126, 0.7152, 0.0722)), 0.02);
		vec3 colorized = mix(vec3(1.0), fc_lin / lf, faction_saturation) * lc;
		vec3 target = mix(colorized, fc_lin * political_flat_gain, political * 0.55);
		col = mix(col, target, fc.a * faction_amount * land_f);
		if (mask_enabled) {
			float m = texelFetch(province_mask, ivec2(clamp(id, 0, textureSize(province_mask, 0).x - 1), 0), 0).r;
			float lcol = dot(col, vec3(0.2126, 0.7152, 0.0722));
			if (m > 0.75) {
				col = mix(col, pow(path_color, vec3(2.2)) * max(lcol * 2.2, 0.08), 0.6);
			} else if (m > 0.25) {
				col = mix(col, pow(reachable_color, vec3(2.2)) * max(lcol * 1.8, 0.06), 0.45);
			} else {
				// Hors de portée : assombri pour faire ressortir les provinces atteignables.
				col *= 0.65;
			}
		}
		if (id == selected_id) {
			col = col * 1.3 + vec3(0.06, 0.045, 0.01);
		} else if (id == hovered_id) {
			col = col * 1.18 + vec3(0.02);
		}
	}

	// --- Frontières ---
	if (has_border_dist && d < 3.0 + realm_band_px * footprint) {
		float px = clamp(fwidth(d), footprint * 0.5, footprint * 2.0);
		float d_scr = d / max(px, 1e-4);
		vec4 fo = faction_of(other_id);
		bool realm = other_id != id && (distance(fo.rgb, fc.rgb) > 0.02 || abs(fo.a - fc.a) > 0.1);
		float w = realm ? realm_border_px : province_border_px;
		// Épaisseur minimale en pixels carte de très près (sinon le trait devient un cheveu).
		w = max(w, 0.12 / max(px, 1e-4));
		float line = 1.0 - smoothstep(w - 0.6, w + 0.6, d_scr);
		float alpha = realm ? realm_border_alpha : mix(province_border_alpha * 0.8, province_border_alpha, political);
		if (realm && fc.a > 0.0) {
			float band = 1.0 - smoothstep(0.0, realm_band_px, d_scr);
			col = mix(col, fc_lin * 0.5, band * band * realm_band_alpha * fc.a * land_f);
		}
		vec3 bcol = realm ? mix(border_color, fc_lin * 0.2, 0.4) : border_color;
		col = mix(col, bcol, line * alpha * land_f);
	}

	ALBEDO = col;
	ROUGHNESS = roughness;
	SPECULAR = mix(0.3, 0.1, lake);
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}
