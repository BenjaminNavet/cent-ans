// Lot ZG5b (ADR 0036) — parcellaire procédural de près (paliers vallée / site) et détail du sol.
// Inclus par terrain.gdshader après relief_quadtree.gdshaderinc (utilise hash12, value_noise,
// season_field, rl_occ_at, splat, crop_* ...). Trois crochets d'une ligne dans terrain.gdshader :
//   fp_splat          : bords des texels de `splat.png` (719 m) cassés de près (déformation de
//                       domaine multi-échelle) — plus de damier de grands carrés ;
//   fp_tile_level     : échelle des textures de détail qui continue de diminuer de près (au lieu de
//                       plafonner à une tuile de 2 km : sol flou aux paliers vallée / site) ;
//   fp_parcels        : parcellaire à l'échelle réelle, en mètres :
//     - openfield (nord, est : `landuse.b` faible) : quartiers (Voronoï ~400 m) de lanières
//       de 15 à 35 m orientées par quartier, assolement triennal par grandes soles (blé d'hiver,
//       céréales de printemps, jachère), tournières enherbées, raies entre lanières ;
//     - bocage (ouest : `landuse.b` fort) : enclos irréguliers (Voronoï ~150 m) bordés de haies
//       arborées, prés et champs ;
//     - vignes sur les coteaux exposés au sud (`landuse.r`, pente 4-45 %, exposition), rangs
//       suivant les courbes de niveau, terrasses sur les pentes fortes ;
//     - prés de fauche en fond de vallée (occlusion du relief < 0, pente faible, prés humides de
//       `wetlands`), andains ;
//     - essarts et lisières près des bois (poids de forêt partiel) ;
//     - loin des villages et hameaux (`terroir_mask` : finage R et pâtures A faibles) : friches et
//       pâtures maigres.
// Pente et exposition viennent du gradient du relief fin (`grad_m`, pages du quadtree), la
// tendance régionale de `splat` / `landuse`, la proximité des villages du masque des terroirs.
// Réglé par `fp_quality` (0 éteint, 1 simple, 2 complet ; préréglages de qualité PF1).

uniform int fp_quality = 2;
// Empreinte (unités monde par pixel écran) où le parcellaire apparaît (début, plein).
uniform float fp_start = 0.05;
uniform float fp_full = 0.015;
uniform float fp_meters = 719.0;
uniform vec3 fp_hedge_color = vec3(0.028, 0.045, 0.016);
uniform vec3 fp_headland_color = vec3(0.13, 0.17, 0.06);
uniform vec3 fp_waste_color = vec3(0.13, 0.13, 0.06);
uniform vec3 fp_assart_color = vec3(0.12, 0.12, 0.05);
uniform vec3 fp_terrace_color = vec3(0.16, 0.14, 0.11);

// Bloc de 8 unités monde (5 752 m) : coordonnées locales en mètres (précision flottante) et
// cellules alignées sur le bloc (pas de couture) ; `per` cellules par bloc.
const float FP_BLOCK = 8.0;

// Lot ZG6 : finages des villes 1:1 proches (`TownLayer._push_finage`) : x, y (unités monde),
// rayon du finage, rayon bâti (unités). Terres cultivées jusqu'au rayon du finage (plus de
// friches), jardins et vergers entre le bâti et 1,3 × le bâti.
const int FP_MAX_TOWNS = 16;
uniform vec4 fp_towns[16];
uniform int fp_town_count = 0;

// 1 dans le finage d'une ville proche, fondu au bord ; `garden` : couronne des jardins.
float fp_finage(vec2 p, out float garden) {
	float f = 0.0;
	garden = 0.0;
	// ZG7a : boucle de longueur fixe (déroulée ; emplacements vides à rayon nul, `TownLayer`),
	// test sur la distance au carré avant tout fondu : la plupart des villes sont trop loin.
	for (int i = 0; i < FP_MAX_TOWNS; i++) {
		vec4 t = fp_towns[i];
		vec2 dp = p - t.xy;
		float reach = max(t.z, t.w * 1.4);
		if (dot(dp, dp) >= reach * reach) {
			continue;
		}
		float d = length(dp);
		f = max(f, 1.0 - smoothstep(t.z * 0.75, t.z, d));
		garden = max(garden, smoothstep(t.w * 0.9, t.w, d) * (1.0 - smoothstep(t.w * 1.2, t.w * 1.4, d)));
	}
	return f;
}

float fp_hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

// ZG7a : deux valeurs d'un seul brassage (hash22 de D. Hoskins) au lieu de deux `fp_hash` :
// Voronoï deux fois moins cher, même loi (motif tiré différemment).
vec2 fp_hash2(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.xx + p3.yz) * p3.zy);
}

// 1 aux paliers vallée / site, 0 au-delà (empreinte d'un pixel en unités monde).
float fp_near(float footprint) {
	return fp_quality <= 0 ? 0.0 : 1.0 - smoothstep(fp_full, fp_start, footprint);
}

// Voronoï sur une grille jitterée de `per` cellules par bloc : id global de la cellule, centre
// (m, repère local), distance approchée au bord (m, F2 - F1 / 2).
void fp_voronoi(vec2 q, vec2 blk, float per, float jitter, out vec2 cell_id, out vec2 center, out float border) {
	float size = FP_BLOCK * fp_meters / per;
	vec2 g = floor(q / size);
	vec2 f = q / size - g;
	float d1 = 1e9;
	float d2 = 1e9;
	vec2 best = vec2(0.0);
	vec2 best_c = vec2(0.0);
	for (int j = -1; j <= 1; j++) {
		for (int i = -1; i <= 1; i++) {
			vec2 o = vec2(float(i), float(j));
			vec2 gid = g + o;
			// Id global (cellule du bloc voisin pour les bords) : pas de couture entre blocs.
			vec2 wrapped = mod(gid, per);
			vec2 id = (blk + floor(gid / per)) * per + wrapped;
			vec2 c = o + 0.5 + (fp_hash2(id) - 0.5) * jitter;
			float d = length(c - f);
			if (d < d1) {
				d2 = d1;
				d1 = d;
				best = id;
				best_c = (g + c) * size;
			} else if (d < d2) {
				d2 = d;
			}
		}
	}
	cell_id = best;
	center = best_c;
	border = (d2 - d1) * 0.5 * size;
}

// Crochet 1 : texel de splat lu en coordonnées déformées de près (bords des carrés de 719 m cassés
// par du bruit à 3 échelles), avant la normalisation des poids.
vec4 fp_splat(vec4 s, vec2 p, float footprint) {
	float vis = 1.0 - smoothstep(0.03, 0.12, footprint);
	if (vis <= 0.0 || fp_quality <= 0 || !has_splat) {
		return s;
	}
	vec2 w = vec2(value_noise(p * 0.9), value_noise(p * 0.9 + vec2(31.7, 11.3))) - 0.5;
	w += (vec2(value_noise(p * 3.7 + 5.1), value_noise(p * 3.7 + 17.9)) - 0.5) * 0.45;
	w += (vec2(value_noise(p * 14.0 + 2.3), value_noise(p * 14.0 + 8.1)) - 0.5) * 0.18;
	vec4 warped = texture(splat, (p + w * 1.1 + vec2(0.5)) / map_size);
	return mix(s, warped, vis);
}

// Crochet 2 : niveau d'échelle des textures de détail (log2 du nombre de tuiles de base) ;
// l'original s'arrête à 0 (une tuile de `tile_px` = 2 km), de près on descend jusqu'à -7 (≈ 17 m).
float fp_tile_level(float lvl) {
	return fp_quality <= 0 ? max(lvl, 0.0) : max(lvl, -7.0);
}

// Crochet 3 : parcellaire de près. Modifie `col` (albédo linéaire), `roughness` et la normale `n`
// (repère monde) ; `open_w` : part des prés et cultures ; `forest_w` : part de forêt.
void fp_parcels(vec2 p, vec2 uv, float footprint, vec2 grad_m, float slope, float hc, vec4 s, vec3 lu,
		vec4 terroir, float open_w, float forest_w, inout vec3 col, inout float roughness, inout vec3 n) {
	float vis = fp_near(footprint);
	// ZG4b : les terres basses sont relevées au plancher `MIN_LAND_M` = 0,5 m (ZG3b, rives de la
	// Tamise, polders) ; le test `hc <= 0.5` les privait de parcellaire (sol nu de près).
	if (vis <= 0.0 || open_w + forest_w < 0.05 || hc < 0.25) {
		return;
	}
	vec2 blk = floor(p / FP_BLOCK);
	vec2 q = (p - blk * FP_BLOCK) * fp_meters;          // m, local au bloc
	float px_m = footprint * fp_meters;                   // m par pixel écran
	float aa = max(px_m, 0.05);

	// --- Tendances : bocage / openfield, village, fond de vallée, coteau sud, lisière ---
	float bocage = smoothstep(0.25, 0.6, lu.b + (value_noise(p * 0.25) - 0.5) * 0.25);
	float village = clamp(terroir.r * 1.3 + terroir.a * 0.6, 0.0, 1.0);
	float town_garden = 0.0;
	village = max(village, fp_finage(p, town_garden));  // ZG6 : finage des villes 1:1
	float grass_share = s.r / max(s.r + s.g, 1e-3);
	float occ = has_relief_shade ? rl_occ_at(uv) : 0.0;
	float wet_meadow = has_wetlands ? texture(wetlands, uv).b : 0.0;
	float valley = smoothstep(-0.05, -0.3, occ) * (1.0 - smoothstep(0.03, 0.08, slope));
	valley = max(valley, wet_meadow * 0.8);
	vec2 g = grad_m / fp_meters;                          // pente (m/m) par axe
	float g_len = length(g);
	// Exposition au sud : le terrain descend vers le sud (y croissant) → g.y < 0.
	float south = g_len > 1e-4 ? clamp(-g.y / g_len, -1.0, 1.0) : 0.0;
	float vine_site = smoothstep(0.04, 0.10, slope) * (1.0 - smoothstep(0.45, 0.7, slope)) * smoothstep(0.1, 0.6, south);
	float vine_w = vine_site * clamp(lu.r * 2.2 + 0.12, 0.0, 1.0) * (1.0 - bocage * 0.6);
	float assart = smoothstep(0.08, 0.3, forest_w) * (1.0 - smoothstep(0.5, 0.75, forest_w));
	float waste = (1.0 - smoothstep(0.05, 0.35, village)) * smoothstep(0.3, 0.8, grass_share + slope * 2.0);

	vec3 pc;
	float edge_line = 0.0;   // haie (bocage) ou tournière (openfield)
	float hedge = 0.0;
	vec2 furrow = vec2(1.0, 0.0);
	float furrow_on = 0.0;
	float h_cell = 0.0;

	// --- Openfield : quartiers de lanières ---
	vec2 fid;
	vec2 fcen;
	float fborder;
	fp_voronoi(q, blk, 14.0, 0.8, fid, fcen, fborder);
	float fh = fp_hash(fid + 3.1);
	float ang = fh * 3.14159;
	vec2 dir = vec2(cos(ang), sin(ang));
	vec2 across = vec2(-dir.y, dir.x);
	float strip_w = mix(15.0, 35.0, fp_hash(fid + 7.7));
	float t = dot(q - fcen, across) / strip_w;
	float strip = floor(t);
	float sh = fp_hash(fid * 1.37 + vec2(strip, strip * 0.37));
	// Soles de l'assolement triennal : grandes zones (≈ 1,5 km) partagées par les quartiers voisins.
	float sole = floor(value_noise(p * 0.45 + 3.3) * 2.999 + fh * 0.4);
	// Même sole = même culture dans le quartier ; une lanière sur quatre fait exception.
	float c_pick = fract(sole / 3.0 + (sh < 0.75 ? 0.0 : sh));
	vec3 crop;
	if (c_pick < 0.34) {
		crop = season_field(crop_wheat, 0, sh);
	} else if (c_pick < 0.67) {
		crop = season_field(mix(crop_green, crop_wheat, sh * 0.4), 1, sh);
	} else {
		crop = season_field(crop_fallow, 3, sh);
	}
	crop *= 0.8 + 0.4 * fp_hash(vec2(strip, fh * 97.0));  // un tenancier par lanière
	float balk_d = (0.5 - abs(fract(t) - 0.5)) * strip_w;  // m jusqu'à la raie la plus proche
	float balk = (1.0 - smoothstep(0.6, 0.6 + aa, balk_d)) * (1.0 - smoothstep(1.5, 6.0, px_m));
	float headland = (1.0 - smoothstep(5.0, 5.0 + aa * 1.5, fborder)) * (0.6 + 0.4 * value_noise(q * 0.08));
	vec3 openfield = mix(crop, fp_headland_color, max(balk * 0.45, headland * 0.45));
	furrow = dir;
	furrow_on = (1.0 - headland) * (c_pick < 0.67 ? 1.0 : 0.3);

	// --- Bocage : enclos irréguliers et haies arborées ---
	vec3 bocage_col = openfield;
	if (bocage > 0.01 && fp_quality >= 2) {
		vec2 bid;
		vec2 bcen;
		float bborder;
		vec2 warp = vec2(value_noise(p * 6.0), value_noise(p * 6.0 + 4.0)) * 55.0 + vec2(value_noise(p * 25.0), value_noise(p * 25.0 + 2.0)) * 14.0;
		fp_voronoi(q + warp, blk, 38.0, 0.95, bid, bcen, bborder);
		float bh = fp_hash(bid + 11.3);
		vec3 field = bh < 0.55 ? season_field(meadow_color * (0.85 + 0.3 * bh), 4, bh)
			: (bh < 0.8 ? season_field(crop_wheat, 0, bh) : season_field(crop_ploughed, 2, bh));
		// Haie arborée : talus et houppiers de 5 à 10 m, bosselés, cœur plus sombre.
		float crowns = value_noise(q * 0.18) * 0.6 + value_noise(q * 0.55) * 0.4;
		float hw = 4.0 + 5.0 * crowns;
		hedge = (1.0 - smoothstep(hw - aa, hw + aa, bborder)) * step(0.1, fp_hash(bid + 5.5));
		float core = 1.0 - smoothstep(0.0, hw, bborder);
		bocage_col = mix(field, fp_hedge_color * (0.6 + 0.7 * crowns) * mix(1.15, 0.7, core), hedge);
	} else if (bocage > 0.01) {
		bocage_col = mix(openfield, meadow_color, 0.5);
	}
	pc = mix(openfield, bocage_col, bocage);
	furrow_on *= 1.0 - bocage;

	// --- Prés de fauche en fond de vallée (andains) ---
	if (valley > 0.01) {
		float swath = abs(fract(dot(q, across) / 3.0) - 0.5);
		vec3 hay = season_field(mix(meadow_color, hay_color, 0.25), 4, fh);
		hay *= 0.92 + 0.12 * smoothstep(0.1, 0.35, swath) * (1.0 - smoothstep(0.5, 1.5, px_m));
		hay = mix(hay, meadow_color * 0.8, wet_meadow * 0.4);
		pc = mix(pc, hay, valley * (1.0 - hedge));
		furrow_on *= 1.0 - valley;
	}

	// --- Vignes sur les coteaux exposés au sud (rangs le long des courbes de niveau) ---
	if (vine_w > 0.01) {
		vec2 contour = g_len > 1e-4 ? vec2(-g.y, g.x) / g_len : dir;
		vec2 downhill = g_len > 1e-4 ? -g / g_len : across;
		float row = dot(q, downhill) / 1.8;
		float rw = fwidth(row);
		float r = abs(fract(row) - 0.5);
		float fine = 1.0 - smoothstep(0.25, 0.6, rw);
		float leaf = mix(0.45, 1.0 - smoothstep(0.2 - rw, 0.2 + rw, r), fine);
		vec3 vine = mix(season_field(vine_soil, 6, fh), season_vine_leaf(vine_leaf), leaf);
		// Terrasses (murets tous les 8-12 m de dénivelé apparent) sur les pentes fortes.
		float terrace = smoothstep(0.2, 0.35, slope) * (1.0 - smoothstep(0.08, 0.08 + aa / 10.0, abs(fract(dot(q, downhill) / 10.0) - 0.5) - 0.42));
		vine = mix(vine, fp_terrace_color, terrace * 0.7);
		pc = mix(pc, vine, vine_w);
		furrow = contour;
		furrow_on *= 1.0 - vine_w;
	}

	// --- Essarts (lisières) et friches loin des villages ---
	if (assart > 0.01) {
		float stumps = smoothstep(0.78, 0.82, value_noise(q * 0.4)) * (1.0 - smoothstep(0.3, 1.0, px_m));
		vec3 cleared = mix(fp_assart_color * (0.85 + 0.3 * value_noise(q * 0.05)), fp_hedge_color, stumps);
		pc = mix(pc, cleared, assart * 0.7);
	}
	if (waste > 0.01) {
		float bush = smoothstep(0.7, 0.76, value_noise(q * 0.12) * 0.7 + value_noise(q * 0.5) * 0.3);
		vec3 rough = mix(season_field(fp_waste_color, 4, fh), fp_hedge_color * 1.3, bush * 0.8);
		pc = mix(pc, rough, waste * 0.75);
	}

	// ZG6 : jardins et vergers en couronne autour du bâti (petits carrés de 12 m, arbres ronds).
	if (town_garden > 0.01) {
		vec2 plot = floor(q / 12.0);
		float kind = fp_hash(plot + blk * 131.0);
		vec3 garden_col = mix(vec3(0.10, 0.15, 0.05), vec3(0.17, 0.18, 0.08), kind);
		vec2 f = fract(q / 12.0) - 0.5;
		float tree = (1.0 - smoothstep(0.18, 0.26, length(f))) * step(0.55, kind);
		float border = smoothstep(0.44, 0.48, max(abs(f.x), abs(f.y)));
		garden_col = mix(garden_col, fp_hedge_color * 1.2, max(tree, border * 0.6));
		pc = mix(pc, garden_col, town_garden * 0.85);
		furrow_on *= 1.0 - town_garden;
	}

	// Détail haute fréquence (touffes, mottes) : visible de près seulement.
	// ZG7a : bruit calculé seulement là où il se voit (moins de 3 m par pixel).
	if (px_m < 3.0) {
		float hf = value_noise(q * 0.9) * 0.5 + value_noise(q * 3.1) * 0.3 + value_noise(q * 9.7) * 0.2;
		pc *= 0.9 + 0.2 * mix(0.5, hf, 1.0 - smoothstep(0.4, 3.0, px_m));
	}

	float w = vis * clamp(open_w + assart * forest_w, 0.0, 1.0);
	col = mix(col, pc, w * 0.92);
	roughness = mix(roughness, mix(0.85, 0.95, hedge), w);
	// Sillons : légère ondulation de la normale en travers des lanières.
	if (furrow_on > 0.01) {
		float k = dot(q, vec2(-furrow.y, furrow.x)) / 0.9;
		float fade = 1.0 - smoothstep(0.2, 0.8, px_m);
		vec2 bump = vec2(-furrow.y, furrow.x) * cos(k * 6.2832) * 0.12 * fade * furrow_on * w;
		n = normalize(n + vec3(bump.x, 0.0, bump.y));
	}
	// Haies en relief (ombrage).
	if (hedge > 0.001) {
		n = normalize(n + vec3((value_noise(q * 0.5) - 0.5), 0.0, (value_noise(q * 0.5 + 9.0) - 0.5)) * hedge * 0.6 * w);
	}
}
