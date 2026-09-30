// Lot HB3 (ADR 0143) — habillage du sol par biome en vue moyenne et proche : parcellaire texturé
// (matières fal.ai de `data/art/ground_materials.yaml`), sol sauvage (garrigue, steppe, lande,
// tourbière, alpage), canopée sur les forêts et roche sur les pentes, choisis par le biome de
// `data/map/biomes.png` et le mélange de `data/art/ground_biome_mix.json`.
// Inclus par terrain.gdshader ; un crochet d'une ligne après la carte de couleur (sg_apply).
// Posé par HbGround.apply (`has_hb` faux sans tableaux : rendu SS inchangé).
//
// Table `hb_table` (17 × 8, ligne = indice de biome), valeurs dans [0, 1] (échelles de HbGround) :
//   x 0..11 : cultures (R couche / 255, G poids cumulé × 0,5 (1 : case vide), B tuile / 1000 m) ;
//   x 12 : canopée (R, B ; A = taille de parcelle / 5000 m) ; x 13 : roche (R, B ; A = haies / 2) ;
//   x 14 / 15 : sols sauvages a / b (R, B ; A = allongement / 10 / présence agricole) ;
//   x 16 : teinte multiplicative du biome (RVB / 2).

uniform sampler2DArray hb_albedo : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D hb_biomes : filter_nearest, repeat_disable;
uniform sampler2D hb_table : filter_nearest, repeat_disable;
uniform bool has_hb = false;
uniform float hb_strength = 0.92;       // part des matières là où elles sont pleinement visibles
uniform float hb_fade_start = 0.14;     // empreinte (px carte / px écran) : début du fondu vers la carte
uniform float hb_fade_end = 0.38;       // au-delà : carte de couleur seule
uniform float hb_hue_keep = 0.15;       // part de la teinte de la carte de couleur gardée
uniform float hb_meters_px = 719.0;     // mètres par pixel carte
uniform float hb_hedge_m = 9.0;         // demi-largeur d'une haie (m) à force 1
uniform vec3 hb_hedge_color = vec3(0.03, 0.05, 0.02);
uniform float hb_region_period = 60.0;  // période (px carte) de l'orientation régionale des lanières
uniform float hb_cell_scale = 3.0;      // grossissement des parcelles (lisibilité, comme Total War)
uniform float hb_cell_jitter = 0.16;    // variation de luminance d'une parcelle à l'autre (±)
uniform float hb_cell_hue = 0.1;        // variation de teinte (jaune ↔ vert) d'une parcelle à l'autre
uniform float hb_debug = 0.0;           // diagnostic : 1 biome, 2 parcelles (R hasard, G culture), 3 part agricole

float hb_hash(vec2 q) {
	return fract(sin(dot(q, vec2(127.1, 311.7))) * 43758.5453);
}

mat2 hb_rot(float a) {
	return mat2(vec2(cos(a), sin(a)), vec2(-sin(a), cos(a)));
}

// `layer_n`, `tile_n` : valeurs normalisées de la table.
vec3 hb_sample(float layer_n, float tile_n, vec2 p, vec2 dpx, vec2 dpy, float ang) {
	float layer = floor(layer_n * 255.0 + 0.5);
	float tile_m = tile_n * 1000.0;
	mat2 r = hb_rot(ang);
	float k = hb_meters_px / max(tile_m, 1.0);
	return textureGrad(hb_albedo, vec3(r * p * k, layer), r * dpx * k, r * dpy * k).rgb;
}

// `open_w`, `forest_w`, `rock_w` : parts normalisées des matériaux au pixel ; `farm_share` : part
// des cultures dans les terres ouvertes (splat) ; `slope` en m/m.
void hb_apply(vec2 p, vec2 map_sz, float footprint, float snow, float land_f, float open_w, float forest_w,
		float rock_w, float farm_share, float slope, inout vec3 col) {
	vec2 dpx = dFdx(p);
	vec2 dpy = dFdy(p);
	if (!has_hb) {
		return;
	}
	float w = hb_strength * (1.0 - smoothstep(hb_fade_start, hb_fade_end, footprint)) * (1.0 - snow) * land_f;
	if (w <= 0.001) {
		return;
	}
	// Orientation régionale des lanières (champs parallèles par terroir) : constante par région et
	// rotation autour de l'origine de la région (tourner les coordonnées absolues avec un angle
	// variable cisaillait tout en tourbillons).
	vec2 region = floor(p / hb_region_period);
	vec2 origin = region * hb_region_period;
	float ang_r = hb_hash(region + 41.0) * 6.2832;
	// Biome au centre de la parcelle (une parcelle = un biome, pas de coupure).
	int b0 = int(texture(hb_biomes, p / map_sz).r * 255.0 + 0.5);
	b0 = clamp(b0, 1, 7);
	vec4 prm0 = texelFetch(hb_table, ivec2(12, b0), 0);
	vec4 prm2 = texelFetch(hb_table, ivec2(14, b0), 0);
	float cell = max(prm0.a * 5000.0, 50.0) * hb_cell_scale / hb_meters_px;
	float strip = max(prm2.a * 10.0, 1.0);
	vec2 pr = hb_rot(ang_r) * (p - origin);
	pr.x /= strip;
	pr += (vec2(value_noise(pr / cell * 0.35), value_noise(pr / cell * 0.35 + 17.3)) - 0.5) * cell * 0.5;
	vec2 cid = floor(pr / cell) + region * 131.0;
	vec2 f = fract(pr / cell);
	vec2 pc = (floor(pr / cell) + 0.5) * cell;
	pc.x *= strip;
	pc = transpose(hb_rot(ang_r)) * pc + origin;
	int b = int(texture(hb_biomes, pc / map_sz).r * 255.0 + 0.5);
	b = b >= 1 && b <= 7 ? b : b0;
	vec4 canopy = texelFetch(hb_table, ivec2(12, b), 0);
	vec4 rock = texelFetch(hb_table, ivec2(13, b), 0);
	vec4 wild_a = texelFetch(hb_table, ivec2(14, b), 0);
	vec4 wild_b = texelFetch(hb_table, ivec2(15, b), 0);
	float hedge_k = rock.a * 2.0;
	float presence = wild_b.a;

	// Parcelle : culture tirée par poids, ou sol sauvage selon la part agricole.
	float h = hb_hash(cid + float(b) * 13.1);
	float h_farm = hb_hash(cid + 71.7);
	float h_turn = hb_hash(cid + 5.3);
	float field_ang = ang_r + (h_turn < 0.7 ? 0.0 : 1.5708);
	float wild_mix = smoothstep(0.35, 0.65, value_noise(p * 0.35 + 3.7));
	vec3 wild = mix(hb_sample(wild_a.r, wild_a.b, p, dpx, dpy, ang_r * 0.3),
		hb_sample(wild_b.r, wild_b.b, p, dpx, dpy, 1.1), wild_mix);
	vec3 open_col = wild;
	bool farmed = h_farm < farm_share * presence;
	if (farmed) {
		float layer = -1.0;
		float tile = 120.0;
		for (int i = 0; i < 12; i++) {
			vec4 slot = texelFetch(hb_table, ivec2(i, b), 0);
			if (slot.g > 0.75) {
				break;
			}
			if (h <= slot.g * 2.0) {
				layer = slot.r;
				tile = slot.b;
				break;
			}
		}
		if (layer >= 0.0) {
			open_col = hb_sample(layer, tile, p, dpx, dpy, field_ang);
			// Chaque parcelle a sa nuance (maturité, sol, exposition) : patchwork lisible même quand
			// la texture n'est plus que sa couleur moyenne.
			float j1 = hb_hash(cid + 3.3) - 0.5;
			float j2 = hb_hash(cid + 9.1) - 0.5;
			open_col *= (1.0 + 2.0 * hb_cell_jitter * j1) * vec3(1.0 + hb_cell_hue * j2, 1.0, 1.0 - hb_cell_hue * j2);
			// Haies et talus sur les bords des parcelles cultivées.
			float edge_m = min(min(f.x, 1.0 - f.x) * strip, min(f.y, 1.0 - f.y)) * cell * hb_meters_px / hb_cell_scale;
			float px_m = footprint * hb_meters_px;
			float hedge_w = hb_hedge_m * hedge_k;
			// Couverture de la haie dans le pixel : reste lisible (plus pâle) quand elle est plus fine
			// qu'un pixel au lieu de disparaître.
			float hedge = hedge_k > 0.01 ? (1.0 - smoothstep(0.0, max(px_m, hedge_w), edge_m - hedge_w)) * clamp(2.0 * hedge_w / max(px_m, 1e-3), 0.35, 1.0) : 0.0;
			float balk = 1.0 - smoothstep(1.5, 1.5 + px_m, edge_m);
			open_col = mix(open_col, open_col * 0.82, balk * 0.6);
			open_col = mix(open_col, hb_hedge_color, hedge * 0.8);
		}
	}
	if (hb_debug > 7.5 && hb_debug < 8.5) {
		col = vec3(fract(p), footprint * 10.0);
		return;
	}
	if (hb_debug > 6.5 && hb_debug < 8.5) {
		col = vec3(prm0.a * 10.0, rock.a, prm2.a * 3.0);
		return;
	}
	if (hb_debug > 5.5 && hb_debug < 8.5) {
		col = vec3(f, 0.0);
		return;
	}
	if (hb_debug > 4.5 && hb_debug < 8.5) {
		col = texture(hb_albedo, vec3(fract(p * 3.0), floor(fract(p.x * 0.05) * 27.0))).rgb;
		return;
	}
	if (hb_debug > 3.5 && hb_debug < 8.5) {
		col = open_col;
		return;
	}
	if (hb_debug > 0.5 && hb_debug < 3.5) {
		col = hb_debug < 1.5 ? vec3(float(b) / 7.0, float(b0) / 7.0, 0.0)
			: (hb_debug < 2.5 ? vec3(h, farmed ? 1.0 : 0.0, 0.0) : vec3(farm_share, presence, open_w));
		return;
	}
	vec3 canopy_col = hb_sample(canopy.r, canopy.b, p, dpx, dpy, 0.4);
	// Roche : part de roche du splat, renforcée sur les pentes fortes.
	float rock_share = clamp(rock_w + smoothstep(0.35, 0.8, slope) * (1.0 - forest_w), 0.0, 1.0);
	vec3 rock_col = hb_sample(rock.r, rock.b, p, dpx, dpy, 0.0);
	float total = max(open_w + forest_w + rock_share, 1e-3);
	vec3 hb_col = (open_col * open_w + canopy_col * forest_w + rock_col * rock_share) / total;
	hb_col *= texelFetch(hb_table, ivec2(16, b), 0).rgb * 2.0;
	// Teinte : une part de celle de la carte de couleur (cohérence avec la vue lointaine).
	float lum_hb = dot(hb_col, vec3(0.2126, 0.7152, 0.0722));
	float lum_c = max(dot(col, vec3(0.2126, 0.7152, 0.0722)), 1e-3);
	hb_col = mix(hb_col, col * (lum_hb / lum_c), hb_hue_keep);
	col = mix(col, hb_col, w);
}
