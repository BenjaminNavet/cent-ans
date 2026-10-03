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
// ZG4 : échelle verticale (MapData.vertical_scale()) ; ZG8 : relief local exagéré, hauteur affichée
// `campaign_display_height` (source unique, double GDScript MapData.display_height).
#include "res://shaders/campaign_relief.gdshaderinc"
#define height_scale campaign_vertical_scale
uniform vec2 map_size = vec2(1.0, 1.0);  // taille du monde (map.json size_px), fournie par le script
uniform float meters_per_px = 719.0;

uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform sampler2D faction_colors : filter_nearest, repeat_disable, hint_default_transparent;
uniform int hovered_id = 0;
uniform int selected_id = 0;
// Masque 1D par province (R8) : ~0,5 = atteignable par l'armée sélectionnée, 1 = sur le chemin.
uniform sampler2D province_mask : filter_nearest, repeat_disable, hint_default_black;
uniform bool mask_enabled = false;
// FE6 : filtre « Féodalité » — couleur du tenant direct par province (alpha 0 = pas de hachure),
// peinte en hachures obliques par-dessus le fond du royaume.
uniform sampler2D hatch_colors : filter_nearest, repeat_disable, hint_default_transparent;
uniform bool hatch_enabled = false;
uniform float hatch_period_px = 9.0;
uniform vec3 reachable_color : source_color = vec3(0.98, 0.93, 0.55);
uniform vec3 path_color : source_color = vec3(1.0, 0.55, 0.15);
// Brouillard de guerre (lot C1, A1-03 ; BR 2026-09-30 ; refait TB2 2026-10-02) : 1 = province vue,
// 0 = voilée. Voile de parchemin : le sol hors de vue est tiré vers un camaïeu sépia, un peu
// assombri, sous un lavis léger ; l'ombrage du relief reste entier (plus de nappe blanche qui
// l'efface). Un reste de brume très fin dérive par-dessus. Réglages : bloc `fog_of_war` de
// `data/ui/campaign_map.json` (posés par `MapReadability`), mêmes valeurs par défaut ici.
uniform sampler2D fog_mask : filter_nearest, repeat_disable, hint_default_white;
uniform bool fog_enabled = false;
uniform float fog_desaturation = 0.62;      // part de camaïeu sépia du sol hors de vue
uniform float fog_dim = 0.72;               // assombrissement (« pas vu en ce moment »)
uniform vec3 fog_veil_color : source_color = vec3(0.74, 0.65, 0.48);  // ton parchemin
uniform float fog_veil_strength = 0.10;     // lavis uni par-dessus (0 = aucun)
uniform vec3 fog_mist_color : source_color = vec3(0.80, 0.74, 0.62);
uniform float fog_mist_max = 0.10;          // opacité du reste de brume le plus épais
uniform float fog_mist_min = 0.0;           // opacité dans les trouées
uniform float fog_mist_glow = 0.0;          // plus de brume auto-éclairée (elle blanchissait le sol)
uniform float fog_cloud_scale = 0.030;      // par pixel carte (≈ 33 px de période)
uniform vec2 fog_cloud_wind = vec2(0.020, 0.008);
uniform float fog_feather_px = 3.0;         // fondu de bord, pixels carte
// M5a : brouillard par case (texture de vue R8 de la simulation, filtrage linéaire).
uniform sampler2D fog_cells : filter_linear, repeat_disable, hint_default_white;
uniform bool fog_by_cell = false;
uniform vec2 fog_cells_size = vec2(1.0, 1.0);   // pixels carte couverts par la texture (fournis par le script)
uniform float fog_edge_jitter_px = 7.0;

uniform sampler2D splat : filter_linear, repeat_disable, hint_default_black;
uniform bool has_splat = false;
uniform sampler2D border_dist : filter_linear, repeat_disable, hint_default_white;
uniform bool has_border_dist = false;
uniform float border_dist_scale = 6.0;    // niveaux 8 bits par pixel carte (tools/geo/splat.py)
uniform sampler2D coast_dist : filter_linear, repeat_disable, hint_default_white;
uniform bool has_coast_dist = false;
uniform float coast_dist_scale = 2.0;
// Lot V4 (A1-11) : lit creusé et berges des fleuves.
#include "res://shaders/river_banks.gdshaderinc"

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
// HC2 (ADR 0161 §3) : eau intérieure éclaircie, bleu-gris, distincte des forêts à hauteur de jeu
// (avant : 0.035, 0.10, 0.115, confondue avec les massifs) ; reflet du ciel `lake_sky_reflect`.
uniform vec3 lake_color = vec3(0.07, 0.23, 0.38);
uniform vec3 lake_sky_color = vec3(0.38, 0.55, 0.72);
uniform float lake_sky_reflect = 0.35;
// A1-04 : berge de lac (vase, roseaux) à la place du sable et de l'écume marins. HC2 : berge claire.
uniform vec3 lake_bank_color = vec3(0.15, 0.14, 0.085);
uniform vec3 lake_shallow_color = vec3(0.12, 0.28, 0.36);
uniform float lake_bank_px = 1.6;
// HB7 : niveau (m) au-dessus duquel une eau est un lac ; en dessous, c'est la mer (même seuil que
// water.gdshader). À 0 m, les hauts-fonds de la mer d'Azov (MNT 0-1 m) devenaient des « lacs »
// sombres à berge de vase, en taches et en escalier contre le plan d'eau de la mer.
uniform float lake_level_m = 2.0;
uniform float pond_max_px = 3.0;
uniform float pond_probe_px = 6.0;
uniform vec3 seabed_color = vec3(0.03, 0.07, 0.07);
// Couleurs de water.gdshader (mer peinte là où le maillage dépasse le plan d'eau).
uniform vec3 sea_shallow_color = vec3(0.045, 0.14, 0.14);
uniform vec3 sea_deep_color = vec3(0.006, 0.028, 0.050);
// GA4 (ADR 0105) : macro-variation à l'échelle de la carte (périodes en pixels carte) et mer
// peinte à trois paliers de profondeur, réglées depuis `data/fx/campaign_terrain_textures.json`
// (`CampaignTextures.apply_terrain`) ; `ga4_on` = 0 (`--no-ga4`) : rendu d'origine.
uniform float ga4_on = 0.0;
uniform vec4 ga4_macro_periods = vec4(9.0, 24.0, 60.0, 150.0);
uniform vec4 ga4_macro_weights = vec4(0.4, 0.3, 0.2, 0.1);
uniform float ga4_tint_amp = 1.0;
uniform float ga4_lum_amp = 0.08;
uniform vec3 sea_mid_color = vec3(0.018, 0.075, 0.095);
uniform float sea_mid_depth_m = 25.0;
uniform float sea_depth_falloff_m = 70.0;

// Détail : taille d'une tuile de texture (pixels carte), part du détail, force des normales.
uniform float tile_px = 3.0;
uniform float farm_tile_px = 0.6;
uniform float tile_screen_px = 72.0;
uniform float detail_strength = 0.85;
uniform float detail_normal_strength = 0.55;
// VT3 : canopée des forêts (terrain) : période de la plus fine octave (unités, ~36 m), octaves
// ×4 ; octave éteinte sous `canopy_min_px` pixels de période ; poids des octaves grossières ;
// relief (normales) et contraste de teinte (houppiers au soleil / trouées sombres).
uniform float canopy_period = 0.05;
uniform float canopy_min_px = 6.0;
uniform float canopy_octave_gain = 0.8;
uniform float canopy_bump = 0.45;
uniform float canopy_contrast = 0.35;
// VT3 : éclaircissement de la teinte des forêts vers celle des houppiers 1:1 (vue d'avion, une
// forêt est surtout faite de cimes au soleil) : le passage terrain → arbres ne montre plus des
// houppiers clairs sur un sol presque noir ; les massifs lointains se rapprochent de la teinte des
// arbres dessinés avant VT3.
uniform vec3 canopy_lift = vec3(1.8, 1.65, 1.25);
// Parcellaire : demi-largeur (px carte) du pied de haie et des chemins.
uniform float hedge_width = 0.16;
uniform float track_width = 0.10;
// GC (ADR 0158) : taille des enclos du parcellaire lointain par rapport à la trame d'origine
// (1 = enclos de ≈ 4,4 px carte, soit ≈ 3 km) ; haies et chemins suivent. 0,3 : enclos de ≈ 1 km,
// lanières de 250 m à 1 km, nettement plus petits qu'une maquette de village (≈ 3 unités). Le semis
// des haies (`VegetationFields.LAYOUTS`, `core/crates/vegetation`) garde la trame d'origine.
uniform float field_scale = 0.3;
// Neige : altitude de base (m) ; roche au-dessus de rock_line.
uniform float snow_line_m = 2750.0;
uniform float rock_line_m = 1700.0;
// Exagération de l'ombrage par rapport à la géométrie (lisibilité du relief).
// CV3-0 (#5) : 1.8 -> 2.1, la plaine entre Paris et Orléans restait peu lisible en vue régionale
// (l'écrasement des montagnes, indépendant, reste inchangé).
uniform float shading_relief = 2.1;
// SS2 (ADR 0142) : de près (empreinte < `shading_relief_near_footprint`), l'exagération retombe à
// `shading_relief_near` : la carte de couleur porte la lecture du paysage, le relief ne doit plus
// dominer (effet « carte IGN »).
uniform float shading_relief_near = 1.3;
uniform float shading_relief_near_footprint = 0.25;
// ZG8 : roche sur les pentes raides du relief EXAGÉRÉ localement (pente affichée ramenée à l'échelle
// ×1 : pente vraie × (1 + gain local)), m/m ; 0 / 0 : désactivé (posés par TerrainBuilder).
uniform float cliff_slope_start = 0.0;
uniform float cliff_slope_full = 0.0;

// Politique : empreinte (pixels carte par pixel écran) où la couleur de faction prend le dessus.
uniform float political_start = 0.9;
uniform float political_full = 2.8;
// Lot DV (ADR 0124) : la couche « politique » (provinces aplaties en couleur de faction,
// désaturation, frontières renforcées) est neutralisée en vue normale : jusqu'au fondu vers le
// parchemin (d ≈ 1100-1300), le terrain garde le rendu « near » (teinte de faction légère,
// frontières DZ) ; seul le parchemin assure la lecture politique au-delà. Un multiplicateur plutôt
// que des seuils d'empreinte repoussés : l'empreinte à une distance donnée dépend de la résolution
// (HiDPI, ADR 0123). Remettre 1.0 pour retrouver l'ancien palier « loin » (A/B).
uniform float political_amount = 0.0;
uniform float faction_alpha_near = 0.04;  // HB : 0,10 → 0,04, la teinte de faction grisait le sol de près (les frontières suffisent)
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

// VT3 : bruit de valeur avec dérivées analytiques (x : valeur [0, 1], yz : gradient par unité de p).
vec3 value_noised(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	vec2 du = 6.0 * f * (1.0 - f);
	float a = hash12(i);
	float b = hash12(i + vec2(1.0, 0.0));
	float c = hash12(i + vec2(0.0, 1.0));
	float d = hash12(i + vec2(1.0, 1.0));
	float k = a - b - c + d;
	return vec3(a + (b - a) * u.x + (c - a) * u.y + k * u.x * u.y,
		du * vec2(b - a + k * u.y, c - a + k * u.x));
}

// VT3 : canopée des forêts portée par le terrain (les arbres 1:1 ne sont plus dessinés au-delà de
// d ≈ 30). Quatre octaves de houppiers (`canopy_period` : bouquets de houppiers, puis ×4 :
// bouquets, peuplements, massifs), chacune éteinte quand sa période passe sous
// `canopy_min_px` pixels (pas de scintillement). Rend (relief centré [-1, 1], pente sans dimension).
vec3 canopy_field(vec2 p, float footprint) {
	vec3 acc = vec3(0.0);
	float period = canopy_period;
	float amp = 1.0;
	float wsum = 0.0;
	for (int i = 0; i < 4; i++) {
		float w = 1.0 - smoothstep(period / (canopy_min_px * 2.0), period / canopy_min_px, footprint);
		if (w > 0.0) {
			vec3 nd = value_noised(p / period + vec2(float(i) * 17.3, float(i) * 5.1));
			acc += w * amp * vec3(nd.x * 2.0 - 1.0, nd.yz * 2.0);  // pente sans dimension (relief ∝ période)
		}
		wsum += amp;
		period *= 4.0;
		amp *= canopy_octave_gain;
	}
	return acc / max(wsum, 1e-3);
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

float fog_seen(int id) {
	return texelFetch(fog_mask, ivec2(clamp(id, 0, textureSize(fog_mask, 0).x - 1), 0), 0).r;
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
	// Inversion de la déformation par point fixe (3 itérations suffisent : |∇warp| ≈ 0,3).
	for (int i = 0; i < 3; i++) {
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

#include "res://shaders/relief_landcover.gdshaderinc"
// Lot CV1 : saisons, terroirs, dévastation, ombres de nuages (fonctions séparées).
#include "res://shaders/campaign_life.gdshaderinc"
// Lot CM2 : vue stratégique parchemin (fondu au zoom maximal) et météo de campagne.
#include "res://shaders/parchment_map.gdshaderinc"
#include "res://shaders/campaign_weather.gdshaderinc"
#define CS_WITH_WEATHER  // RV-C : ombres de nuages liées à la météo par province
#include "res://shaders/campaign_cloud_shadow.gdshaderinc"  // RV-C : ombres de nuages
#include "res://shaders/relief_quadtree.gdshaderinc"  // ZG2 : relief streamé en quadtree
#include "res://shaders/fine_parcels.gdshaderinc"  // ZG5b : parcellaire et détail de près
#include "res://shaders/faction_borders.gdshaderinc"  // FR1 : frontières de faction (crochet fr1_borders)
#include "res://shaders/ga_macro.gdshaderinc"  // GA4 : macro-variation, couleur de profondeur
#include "res://shaders/satellite_ground.gdshaderinc"  // SS2 : carte de couleur (ADR 0142)
#include "res://shaders/hb_ground.gdshaderinc"  // HB3 : habillage par biome (ADR 0143)
#include "res://shaders/coast_band.gdshaderinc"  // TB5 : falaises et plages (crochet coast_band)

void vertex() {
	VERTEX = qt_vertex(VERTEX);  // ZG2 : patch du quadtree déplacé (sans effet sur les morceaux E0)
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	float carve = river_carve(world_pos.xz, map_size);  // lot V4
	VERTEX.y -= carve;
	world_pos.y -= carve;
}

// HB7 : eau raster (coast_dist) dont le MNT dépasse `lake_level_m` : vrai lac seulement si aucune
// eau sous le niveau de la mer à `lake_sea_probe_px` alentour (8 directions). Sinon c'est un défaut
// de calage MNT / trait de côte au bord de la mer (delta du Don, flèches d'Azov) : peint comme la mer.
uniform float lake_sea_probe_px = 4.0;
float sea_nearby(vec2 uv) {
	float lowest = 1e6;
	for (int k = 0; k < 8; k++) {
		float a = float(k) * 0.785398;
		lowest = min(lowest, height_m_lod(uv + vec2(cos(a), sin(a)) * lake_sea_probe_px / map_size, 0.0));
	}
	return 1.0 - step(0.0, lowest);
}

void fragment() {
	vec2 p = world_pos.xz;
	vec2 uv = p / map_size;  // SZ2b : pixel carte i centré en x = i + 0,5, comme les outils (ADR 0086)
	// Empreinte d'un pixel écran en pixels carte (≈ 0,3 de près, ≈ 2,5 France entière).
	float footprint = max(length(dFdx(p)), length(dFdy(p)));

	// --- Relief ---
	float k = max(1.0, footprint * 1.5);
	float lod = log2(k);
	vec2 du = vec2(k / map_size.x, 0.0);
	vec2 dv = vec2(0.0, k / map_size.y);
	float hc = 0.0;
	vec2 grad_m = vec2(0.0);
	// PB2 : sur une page du quadtree, `qt_relief` remplace altitude et gradient (9 lectures de
	// heightmap et de relief fin évitées, même résultat).
	if (!qt_has_page()) {
		hc = height_m_lod(uv, max(lod - 1.0, 0.0));
		float h_l = height_m_lod(uv - du, lod);
		float h_r = height_m_lod(uv + du, lod);
		float h_u = height_m_lod(uv - dv, lod);
		float h_d = height_m_lod(uv + dv, lod);
		grad_m = vec2(h_r - h_l, h_d - h_u) / (2.0 * k);     // mètres par pixel carte
		rl_relief(uv, footprint, grad_m);  // lot R1 : relief fin
	}
	qt_relief(p, footprint, hc, grad_m);  // ZG2 : relief de la page du quadtree
	float slope = length(grad_m) / meters_per_px;              // pente réelle (m/m)
	vec2 grad_disp = campaign_display_gradient(hc, grad_m, p);  // ZG8 : pente de la surface affichée
	float cliff = cliff_slope_full > cliff_slope_start ? smoothstep(cliff_slope_start, cliff_slope_full, length(grad_disp) / (height_scale * meters_per_px)) : 0.0;
	vec2 g = grad_disp * mix(shading_relief_near, shading_relief, smoothstep(0.0, shading_relief_near_footprint, footprint));
	vec3 n = normalize(vec3(-g.x, 1.0, -g.y));

	float coast = has_coast_dist ? (texture(coast_dist, uv).r * 255.0 - 128.0) / coast_dist_scale : (hc > 0.0 ? 8.0 : -8.0);
	float land_f = smoothstep(-0.4, 0.6, coast);
	// A1-04 : côté eau de la côte la plus proche au-dessus du niveau de la mer = lac/étang (berge
	// fondue, pas de sable) ; sinon mer. Sonde le long du gradient de distance à la côte.
	float lake_shore = 0.0;
	// Étang sous le niveau de la mer (marais côtiers) : à `pond_probe_px` plus loin vers l'eau,
	// on reste à moins de `pond_max_px` d'une rive (même test que water.gdshader).
	if (has_coast_dist && coast < lake_bank_px + 1.5 && coast > -pond_max_px - 1.0) {
		vec2 ex = vec2(1.0 / map_size.x, 0.0);
		vec2 ey = vec2(0.0, 1.0 / map_size.y);
		vec2 cg = vec2(texture(coast_dist, uv + ex).r - texture(coast_dist, uv - ex).r,
			texture(coast_dist, uv + ey).r - texture(coast_dist, uv - ey).r);
		vec2 to_water = -cg / max(length(cg), 1e-5);
		vec2 probe = uv + to_water * (max(coast, 0.0) + 1.5) / map_size;
		vec2 far_probe = uv + to_water * (max(coast, 0.0) + pond_probe_px) / map_size;
		float far_coast = (texture(coast_dist, far_probe).r * 255.0 - 128.0) / coast_dist_scale;
		lake_shore = max(step(lake_level_m, height_m_lod(probe, 0.0)) * (1.0 - sea_nearby(probe)), step(-pond_max_px, far_coast));
	}

	// --- Poids des matériaux ---
	vec4 s = has_splat ? texture(splat, uv) : vec4(0.5, 0.3, 0.2, 0.0);
	s = fp_splat(s, p, footprint);  // ZG5b : bords des texels de splat cassés de près
	s /= max(s.r + s.g + s.b + s.a, 1e-3);
	s = rl_landcover(s, p, footprint);  // lot R1 : lisières
	// TB4 : le masque de terroir est carré, à la même échelle sur les deux axes (`TerroirMask` :
	// 1024 / plus grand côté de la carte) ; avec `uv` (carte 7168 × 6144 depuis OM), finages et
	// brûlis étaient décalés vers le nord d'un sixième de l'ordonnée.
	vec4 terroir = terroir_at(p / max(map_size.x, map_size.y));  // CV1 : finage des colonies, vigne, brûlis, pâtures
	s = terroir_splat(s, terroir);
	float macro = fbm(p * 0.035);
	float rock_frac = max(smoothstep(rock_line_m - 700.0, rock_line_m, hc), smoothstep(0.35, 0.6, slope));
	float w_grass = s.r;
	float w_farm = s.g;
	float w_forest = s.b;
	float w_rock = s.a * rock_frac;
	float w_heath = s.a * (1.0 - rock_frac);
	// ZG8 : falaises (pente affichée) en roche, la forêt s'accroche un peu plus longtemps.
	float before_cliff = w_grass + w_farm + w_forest + w_heath;
	w_grass *= 1.0 - cliff;
	w_farm *= 1.0 - cliff;
	w_heath *= 1.0 - cliff;
	w_forest *= 1.0 - 0.6 * cliff;
	w_rock += before_cliff - (w_grass + w_farm + w_forest + w_heath);
	float snow = smoothstep(snow_line_m - 150.0, snow_line_m + 250.0, hc + (macro - 0.5) * 500.0)
		* (1.0 - smoothstep(0.55, 0.9, slope));
	snow = season_snow(snow, hc, uv, p, slope, macro, footprint, w_forest / max(w_grass + w_farm + w_forest + w_rock + w_heath, 1e-3));  // CV1, TB1
	float sand = (1.0 - smoothstep(0.3, 1.6, coast)) * (1.0 - smoothstep(4.0, 30.0, hc)) * land_f * (1.0 - lake_shore);
	float keep = (1.0 - snow) * (1.0 - sand);
	w_grass *= keep; w_farm *= keep; w_forest *= keep; w_rock *= keep; w_heath *= keep;

	// Variation macro : légères nuances de teinte pour casser l'uniformité.
	float tone = 0.85 + 0.3 * macro;

	// --- Parcellaire (lot V2b) : enclos, lanières de cultures, prés, vignes, haies, chemins ---
	vec3 lu = has_landuse ? texture(landuse, uv).rgb : vec3(0.0);
	lu = terroir_landuse(lu, terroir);  // CV1
	float dry = lu.g;
	float open_land = clamp(s.g + 0.6 * s.r, 0.0, 1.0);
	float grass_share = s.r / max(s.r + s.g, 1e-3);
	// Teintes de base (lointain) : même moyenne que les parcelles, sèches au sud.
	vec3 grass_base = mix(tint_grass, dry_grass, dry * 0.75);
	vec3 farm_base = mix(tint_farmland, mix(tint_farmland, crop_wheat, 0.35) * vec3(1.05, 0.98, 0.9), dry * 0.6);
	vec3 grass_summer = grass_base;  // TB1 : références d'été de l'écart saisonnier
	vec3 farm_summer = farm_base;
	grass_base = season_grass(grass_base);  // CV1
	farm_base = season_farm(farm_base);
	vec3 grass_tint = grass_base;
	vec3 farm_tint = farm_base;
	float hedge = 0.0;
	float track = 0.0;
	float balk = 0.0;
	float vine_rows = 0.0;
	vec2 furrow_dir = vec2(1.0, 0.0);
	float field_s = max(field_scale, 0.05);
	float field_fp = footprint / field_s;  // empreinte du pixel dans le repère du parcellaire
	float cell_px = 4.4 / max(field_fp, 1e-4);  // taille d'un enclos à l'écran
	float field_vis = smoothstep(5.0, 14.0, cell_px);
	if (field_vis > 0.0 && (s.g + s.r) > 0.05) {
		float rv = region_value(p);
		int lay = rv > 0.0 ? 1 : 0;
		float cell_h; float strip_h; float du; float dv; float roll_u; float roll_v; float strip_d; float rows;
		field_at(p / field_s, lay, cell_h, strip_h, du, dv, roll_u, roll_v, strip_d, furrow_dir, rows);
		// Lanières visibles de plus près que les enclos : sinon, couleur de l'enclos.
		float strip_vis = smoothstep(3.0, 8.0, cell_px * 0.4);
		float h = strip_vis > 0.5 ? strip_h : cell_h;
		float h_b = hash01i(int(h * 99991.0) + 7);
		// Prés plus fréquents dans les zones herbagères, vigne selon l'occupation du sol.
		float p_meadow = mix(0.15, 0.85, grass_share);
		vec3 crop;
		bool vine = h_b < lu.r * 0.32 && grass_share < 0.8;
		if (vine) {
			crop = season_field(vine_soil, 6, h_b);  // CV1
		} else if (h < p_meadow) {
			float m = h / max(p_meadow, 1e-3);
			crop = m < 0.6 ? meadow_color * (0.85 + 0.35 * h_b) : (m < 0.85 ? mix(meadow_color, hay_color, 0.55) : hay_color);
			crop = season_field(crop, m < 0.6 ? 4 : 5, h_b);  // CV1
			crop = mix(crop, dry_grass, dry * 0.7);
		} else {
			float c = (h - p_meadow) / max(1.0 - p_meadow, 1e-3);
			crop = c < 0.36 ? crop_wheat : (c < 0.6 ? crop_green : (c < 0.76 ? crop_ploughed : crop_fallow));
			crop = season_field(crop, c < 0.36 ? 0 : (c < 0.6 ? 1 : (c < 0.76 ? 2 : 3)), h_b);  // CV1
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
		float aa = field_fp * 0.75;
		float hedge_u = step(roll_u, p_hedge) * (1.0 - smoothstep(hedge_width - aa, hedge_width + aa, du));
		float hedge_v = step(roll_v, p_hedge) * (1.0 - smoothstep(hedge_width - aa, hedge_width + aa, dv));
		// Au-delà d'un pixel, la ligne s'estompe (couverture) au lieu de disparaître.
		float cover = clamp(2.0 * hedge_width / max(field_fp, 1e-4), 0.0, 1.0);
		hedge = max(hedge_u, hedge_v) * mix(cover, 1.0, cover) * field_vis;
		float tr_u = step(fract(roll_u * 7.31), 0.07) * (1.0 - smoothstep(track_width - aa, track_width + aa, du));
		float tr_v = step(fract(roll_v * 7.31), 0.07) * (1.0 - smoothstep(track_width - aa, track_width + aa, dv));
		// Limite entre les deux trames : chemin continu.
		float tcover = clamp(2.0 * track_width / max(field_fp, 1e-4), 0.0, 1.0);
		float tr_r = 0.0;
		if (tcover > 0.15 && abs(rv) < 0.05) {
			float rgrad = length(vec2(region_value(p + vec2(1.0, 0.0)) - rv, region_value(p + vec2(0.0, 1.0)) - rv));
			float rd = abs(rv) / max(rgrad, 1e-4);
			tr_r = 1.0 - smoothstep(track_width * 1.3 - aa, track_width * 1.3 + aa, rd);
		}
		track = max(max(tr_u, tr_v), tr_r) * tcover * field_vis;
		float edge = min(min(du, dv), strip_d);
		balk = (1.0 - smoothstep(0.03, 0.03 + aa * 2.0, edge)) * smoothstep(8.0, 20.0, cell_px) * 0.5;
	}

	// --- Textures ---
	vec3 forest_tint = season_forest(tint_forest, value_noise(p * 0.45)) * canopy_lift;  // CV1, VT3
	float detail_fade = 1.0 - smoothstep(0.7, 1.8, footprint);
	vec3 albedo = vec3(0.0);
	vec3 dn = vec3(0.0, 0.0, 0.0);
	float wsum = w_grass + w_farm + w_forest + w_rock + w_heath + snow + sand;
	if (has_textures && detail_fade > 0.0) {
		// Échelle de tuilage adaptée au zoom : une tuile garde ~tile_screen_px pixels à l'écran
		// (pas de trame de points répétés au zoom moyen) ; fondu entre deux octaves d'échelle,
		// la seconde tournée pour casser l'alignement.
		float lvl = fp_tile_level(log2(tile_screen_px * footprint / tile_px));  // ZG5b : détail de près
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
		if (w_forest > 0.01) { sample_layer(2, uv_a, uv_b, mix_b, forest_tint, det, albedo, dn, w_forest); }
		if (w_rock > 0.01) { sample_layer(3, uv_a, uv_b, mix_b, tint_rock, det, albedo, dn, w_rock); }
		if (w_heath > 0.01) { sample_layer(4, uv_a, uv_b, mix_b, tint_heath, det, albedo, dn, w_heath); }
		if (snow > 0.01) { sample_layer(5, uv_a, uv_b, mix_b, tint_snow, det, albedo, dn, snow); }
		if (sand > 0.01) { sample_layer(6, uv_a, uv_b, mix_b, tint_sand, det, albedo, dn, sand); }
		albedo /= max(wsum, 1e-3);
		dn /= max(wsum, 1e-3);
	} else {
		albedo = (w_grass * grass_tint + w_farm * farm_tint + w_forest * forest_tint + w_rock * tint_rock
			+ w_heath * tint_heath + snow * tint_snow + sand * tint_sand) / max(wsum, 1e-3);
		dn = vec3(0.0, 0.0, 0.85);
	}
	vec3 col = albedo * mix(1.0, tone, 1.0 - snow);
	// GA4 : macro-variation de teinte (octaves fines d'abord) et de luminance (octaves larges
	// d'abord), mêmes 4 échantillons de bruit ; visible aussi au dézoom (textures coupées).
	if (ga4_on > 0.5) {
		vec4 mo = vec4(value_noise(p / ga4_macro_periods.x + vec2(3.1, 7.7)),
			value_noise(p / ga4_macro_periods.y + vec2(11.3, 1.9)),
			value_noise(p / ga4_macro_periods.z + vec2(5.3, 17.2)),
			value_noise(p / ga4_macro_periods.w + vec2(23.9, 13.4)));
		float wsum4 = max(dot(ga4_macro_weights, vec4(1.0)), 1e-3);
		float mh = dot(mo, ga4_macro_weights) / wsum4;
		float ml = dot(mo, ga4_macro_weights.wzyx) / wsum4;
		col *= ga_macro_variation(mh, ml, ga4_tint_amp, ga4_lum_amp);
	}
	// Canopée : ombrage de volume léger (les forêts paraissent plus sombres et granuleuses).
	col *= 1.0 - 0.25 * w_forest / max(wsum, 1e-3) * (1.0 - value_noise(p * 1.7)) * detail_fade;
	float open_w = (w_grass + w_farm) / max(wsum, 1e-3);
	col = mix(col, col * vec3(0.93, 0.95, 0.9), balk * open_w);
	col = mix(col, season_vine_leaf(vine_leaf), vine_rows * open_w * 0.8);  // CV1
	col = mix(col, track_color * (0.9 + 0.2 * macro), track * open_w * 0.85);
	col = mix(col, hedge_color, hedge * open_w * 0.55);

	// Normales de détail combinées au relief (blend « whiteout » dans le repère carte).
	float dstrength = detail_normal_strength * detail_fade;
	n = normalize(vec3(n.x + dn.x * dstrength, n.y, n.z - dn.y * dstrength));
	// VT3 : canopée (houppiers en relief et contraste) sur les forêts, à toute distance.
	float forest_f = w_forest / max(wsum, 1e-3);
	if (forest_f > 0.01 && (canopy_bump > 0.0 || canopy_contrast > 0.0)) {
		vec3 cf = canopy_field(p, footprint);
		n = normalize(vec3(n.x - cf.y * canopy_bump * forest_f, n.y, n.z - cf.z * canopy_bump * forest_f));
		col *= 1.0 + canopy_contrast * forest_f * cf.x;
	}
	float roughness = has_textures && detail_fade > 0.0 ? clamp(dn.z, 0.55, 1.0) : 0.9;
	fp_parcels(p, uv, footprint, grad_m, slope, hc, s, lu, terroir, open_w, w_forest / max(wsum, 1e-3), col, roughness, n);  // ZG5b
	// TB1 : écart saisonnier (saison / été) de l'occupation du sol, posé par-dessus la carte de
	// couleur et les matières (qui sont d'une saison moyenne) au lieu d'être effacé par elles.
	vec3 k_grass = season_ratio(grass_base, grass_summer);
	vec3 k_farm = season_ratio(farm_base * mix(vec3(1.0), summer_gold, life_season().y * float(life_enabled)), farm_summer);
	vec3 forest_far = season_forest_mean(tint_forest);
	vec3 k_forest = season_ratio(mix(season_forest(tint_forest, value_noise(p * 0.45)), forest_far, smoothstep(0.3, 1.0, footprint)), tint_forest);
	vec3 russet = mix(vec3(1.0), autumn_russet, life_season().z * float(life_enabled));
	k_grass *= russet;
	k_forest *= russet;
	k_farm *= mix(vec3(1.0), russet, 0.5);
	vec3 k_heath = mix(vec3(1.0), k_grass, 0.6);
	vec3 season_num = w_grass * grass_summer * k_grass + w_farm * farm_summer * k_farm + w_forest * tint_forest * k_forest
		+ w_rock * tint_rock + w_heath * tint_heath * k_heath;
	vec3 season_den = w_grass * grass_summer + w_farm * farm_summer + w_forest * tint_forest + w_rock * tint_rock + w_heath * tint_heath;
	vec3 season_k = dot(season_den, vec3(1.0)) > 1e-4 ? season_ratio(season_num, season_den) : vec3(1.0);
	sg_apply(uv, footprint, snow, land_f, season_k, col);  // SS2
	vec4 burn_parcel;  // TB4 : parcelle HB du pixel (brûlis par parcelles)
	hb_apply(p, map_size, footprint, snow, land_f, (w_grass + w_farm + w_heath) / max(wsum, 1e-3), w_forest / max(wsum, 1e-3),
		w_rock / max(wsum, 1e-3), w_farm / max(w_grass + w_farm, 1e-3), slope, k_grass, k_forest,
		mat4(vec4(crop_wheat, 0.0), vec4(crop_green, 0.0), vec4(crop_ploughed, 0.0), vec4(crop_fallow, 0.0)), col, burn_parcel);  // HB3
	col = mix(season_winter_ground(col, land_f), col, snow);  // TB1 : sol d'hiver hors neige
	vec3 hb_dbg = col;  // HB3 : diagnostic (`hb_debug` > 0 : sortie directe de l'habillage)
	// TB4 : brûlis (CV1) posé par-dessus la carte de couleur (SS2) et les matières (HB3), qui le
	// recouvraient. Par parcelles (celles de l'habillage HB) : une part carbonisée, une part
	// roussie, le reste intact, selon la dévastation lue au centre de la parcelle ; au loin, la
	// moyenne des trois, découpée par un bruit large.
	col = terroir_burn(col, terroir.b, 1.0 - snow * 0.7, p, footprint, burn_parcel, 1.0 / max(map_size.x, map_size.y));

	if (hc > 0.0) {
		river_banks(p, map_size, footprint, col, roughness, n);  // lot V4
	}

	// Fond marin (sous le plan d'eau) et lacs.
	if (hc <= 0.0 || (land_f < 0.5 && lake_shore < 0.5 && (hc < lake_level_m || sea_nearby(uv) > 0.5))) {  // HB7 : hauts-fonds marins
		col = mix(tint_sand * 0.5, seabed_color, smoothstep(-2.0, -30.0, hc));
	}
	// Eau peinte sur le terrain : lacs (au-dessus du niveau de la mer) et mer là où le maillage
	// LOD dépasse le plan d'eau alors que la côte réelle (raster) est plus loin. Pour la mer, même
	// rendu que water.gdshader (couleur selon la profondeur composée sur le fond marin).
	float lake = (1.0 - land_f) * max(step(lake_level_m, hc), step(-0.02, world_pos.y));
	vec3 water_col = lake_color;
	float sea_water = hc < lake_level_m ? 1.0 : 0.0;
	if (sea_water < 0.5 && land_f < 0.99) {
		sea_water = sea_nearby(uv);
	}
	if (sea_water > 0.5) {  // HB7 : mer jusqu'au niveau des lacs (hauts-fonds à 0-2 m compris)
		float depth = max(-hc, 0.0);
		vec3 wc = ga_depth_color(depth, sea_shallow_color, sea_mid_color, sea_deep_color, sea_mid_depth_m, sea_depth_falloff_m, ga4_on);  // GA4
		water_col = mix(col, wc, mix(0.7, 0.97, smoothstep(0.0, 45.0, depth)));
	}
	if (lake_shore > 0.5) {
		// Berge de lac : vase et roseaux sur la rive, eau peu profonde teintée près du bord,
		// transition large et bruitée (pas de liseré net).
		float reeds = value_noise(p * 3.1) * 0.6 + value_noise(p * 7.3) * 0.4;
		float bank = (1.0 - smoothstep(0.0, lake_bank_px * (0.7 + 0.6 * reeds), coast)) * land_f;
		col = mix(col, lake_bank_color * (0.8 + 0.4 * reeds), bank * 0.85);
		lake = 1.0 - smoothstep(-0.7, 0.3 + 0.3 * (reeds - 0.5), coast);
		water_col = mix(lake_color, lake_shallow_color, 1.0 - smoothstep(-1.6, 0.0, coast));
	}
	rl_surface(p, uv, footprint, VIEWPORT_SIZE.y, land_f, col, lake, water_col, roughness);  // lot R1
	if (lake_shore > 0.5 || sea_water < 0.5) {
		// HC2 : reflet du ciel sur l'eau intérieure (lacs, étangs, mares ; pas la mer peinte), plus
		// marqué en vue rasante.
		float graze = pow(1.0 - clamp(dot(normalize((VIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz), VIEW), 0.0, 1.0), 4.0);
		water_col = mix(water_col, lake_sky_color, clamp(lake_sky_reflect + 0.5 * graze, 0.0, 0.8));
	}
	if (hb_debug > 8.5 && hb_debug < 9.5) { hb_dbg = col; }  // HB3 : sonde
	col = mix(col, water_col, lake);
	roughness = mix(roughness, 0.3, lake);
	n = normalize(mix(n, vec3(0.0, 1.0, 0.0), lake));
	coast_band(p, uv, footprint, coast, hc, world_pos.y, lake_shore, snow, col, roughness, n);  // TB5 : après l'eau peinte

	// --- Couche politique ---
	if (hb_debug > 9.5 && hb_debug < 10.5) { hb_dbg = col; }  // HB3 : sonde
	float political = smoothstep(political_start, political_full, footprint) * political_amount;
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
		if (hatch_enabled) {
			vec4 hatch_c = texelFetch(hatch_colors, ivec2(clamp(id, 0, textureSize(hatch_colors, 0).x - 1), 0), 0);
			if (hatch_c.a > 0.0) {
				// Hachures obliques antialiasées (largeur d'un demi-pas), à la teinte du tenant.
				// Pas élargi au dézoom (au moins six pixels d'écran) : pas de moiré.
				float hatch_period = max(hatch_period_px, footprint * 6.0);
				float hatch_s = fract((p.x + p.y) / hatch_period);
				float hatch_aa = clamp(footprint / hatch_period, 0.02, 0.25);
				float hatch_stripe = smoothstep(0.5 - hatch_aa, 0.5 + hatch_aa, hatch_s) * (1.0 - smoothstep(1.0 - hatch_aa, 1.0, hatch_s));
				vec3 hatch_lin = pow(hatch_c.rgb, vec3(2.2));
				float hatch_l = max(dot(hatch_lin, vec3(0.2126, 0.7152, 0.0722)), 0.02);
				vec3 hatch_target = mix(mix(vec3(1.0), hatch_lin / hatch_l, faction_saturation) * lc, hatch_lin * political_flat_gain, political * 0.55);
				col = mix(col, hatch_target, hatch_stripe * hatch_c.a * faction_amount * land_f);
			}
		}
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

	float fog_mist = 0.0;  // BR : opacité de la brume, réutilisée pour l'émission et la normale
	if (fog_enabled && id > 0) {
		float fog;
		if (fog_by_cell) {
			// M5a : vue par case (texture 512² de la simulation, 255 = vu, seuil à 128).
			// Bord effrangé : la prise est décalée de quelques pixels par un bruit lent.
			vec2 jitter = vec2(fbm(p * 0.035), fbm(p * 0.035 + vec2(31.7, 5.3))) - 0.5;
			float seen_r = texture(fog_cells, (p + jitter * fog_edge_jitter_px) / fog_cells_size).r;
			fog = 1.0 - smoothstep(0.32, 0.68, seen_r);
		} else {
			fog = 1.0 - step(0.5, fog_seen(id));
			if (other_id != id && other_id > 0) {
				float fog_other = 1.0 - step(0.5, fog_seen(other_id));
				if (fog_other != fog) {
					fog = mix(0.5, fog, smoothstep(0.0, fog_feather_px, d));
				}
			}
		}
		if (fog > 0.0) {
			// TB2 : voile de parchemin. Camaïeu sépia à luminance conservée (le modelé du sol reste
			// lisible), assombri, puis un lavis uni très léger du ton du papier.
			const vec3 lum_w = vec3(0.2126, 0.7152, 0.0722);
			float lum_f = dot(col, lum_w);
			vec3 sepia = fog_veil_color / max(dot(fog_veil_color, lum_w), 1e-3);
			vec3 ground = mix(col, lum_f * sepia, fog_desaturation) * fog_dim;
			ground = mix(ground, fog_veil_color * fog_dim, fog_veil_strength);
			// Reste de brume : bruit à domaine déformé, deux couches qui dérivent à des vitesses
			// différentes ; la couche fine s'efface au dézoom (pas de scintillement).
			vec2 q = p * fog_cloud_scale;
			vec2 warp = vec2(fbm(q * 0.5 + TIME * fog_cloud_wind * 0.6), fbm(q * 0.5 + vec2(7.3, 2.9) - TIME * fog_cloud_wind * 0.4));
			float banks = fbm(q + warp * 1.6 + TIME * fog_cloud_wind);
			float fine_w = 1.0 - smoothstep(0.4, 2.0, footprint * fog_cloud_scale * 12.0);
			float wisps = fbm(q * 3.1 - TIME * fog_cloud_wind * 2.2);
			float density = smoothstep(0.26, 0.78, banks + (wisps - 0.5) * 0.35 * fine_w);
			// À la lisière de la zone vue, le voile s'effiloche au lieu de faire un mur.
			float edge = smoothstep(0.0, 1.0, fog);
			fog_mist = mix(fog_mist_min, fog_mist_max, density) * edge;
			col = mix(mix(col, ground, fog), fog_mist_color * fog_dim, fog_mist);
		}
	}

	col *= cloud_shadow_light(p, footprint, float(life_enabled) * (1.0 - political) * (1.0 - pm_weight()));  // RV-C
	if (hb_debug > 10.5 && hb_debug < 11.5) { hb_dbg = col; }  // HB3 : sonde
	vec3 wx_emit = vec3(0.0);
	// TB6 : brume du matin (relief lissé à grande échelle, moyenne régionale, rasance du regard).
	bool wx_valley = weather_enabled && weather_valley_mist > 0.0;
	float wx_h_mid = wx_valley ? height_m_lod(uv, weather_valley_lods.x) : hc;
	float wx_h_wide = wx_valley ? height_m_lod(uv, weather_valley_lods.y) : hc;
	float wx_grazing = 1.0 - abs(normalize(CAMERA_POSITION_WORLD - world_pos).y);
	float wx_mist = 0.0;
	col = weather_ground(col, p, hc, slope, footprint, wx_h_mid, wx_h_wide, wx_grazing, roughness, wx_emit, wx_mist);  // CM2 : météo
	if (hb_debug > 11.5 && hb_debug < 12.5) { hb_dbg = col; }  // HB3 : sonde
	vec3 pm_emit;
	parchment_apply(p, uv, footprint, coast, lake, id, other_id, d, col, n, roughness, pm_emit);  // CM2
	if (hb_debug > 12.5 && hb_debug < 13.5) { hb_dbg = col; }  // HB3 : sonde
	fr1_borders(p, uv, footprint, col, pm_emit);  // FR1 : frontières de faction lumineuses
	col = qt_debug_color(col);  // ZG2 : teintes de débogage du quadtree (désactivées par défaut)
	ALBEDO = col;
	EMISSION = pm_emit + wx_emit * (1.0 - pm_weight());  // CM2
	EMISSION += col * fog_mist * fog_mist_glow * (1.0 - pm_weight());  // BR : brume auto-éclairée
	ROUGHNESS = roughness;
	if (hb_debug > 0.5) {
		ALBEDO = hb_dbg;
	}
	SPECULAR = mix(0.3, 0.1, lake);
	n = normalize(mix(n, vec3(0.0, 1.0, 0.0), max(fog_mist * 0.7, wx_mist) * (1.0 - pm_weight())));  // BR, TB6 : la brume nappe le relief
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}
