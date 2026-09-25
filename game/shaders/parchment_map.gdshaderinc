// Lot CM2 : terrain en carte enluminée sur parchemin (inclus par terrain.gdshader, crochets
// d'une ligne marqués `// CM2`). Relief en hachures et montagnes « taupinières », forêts en
// petits arbres dessinés, lavis de couleur des factions (pigment accumulé au bord), frontières
// à l'encre, côtes et lacs cernés. Dépend des uniformes et fonctions de terrain.gdshader.
#include "res://shaders/parchment_common.gdshaderinc"

// Hachures : pas (pixels carte) et direction de la lumière (nord-ouest, repère carte).
uniform float pm_hatch_px = 5.0;
uniform vec2 pm_light_dir = vec2(-0.7, -0.7);
// Montagnes dessinées : cellule (px carte), altitude de départ (m).
uniform float pm_mount_cell_px = 26.0;
uniform float pm_mount_min_m = 750.0;
// Arbres dessinés : cellule (px carte), part de forêt minimale.
uniform float pm_tree_cell_px = 12.0;
uniform float pm_tree_min = 0.56;
uniform float pm_wash_strength = 0.55;

float pm_height(vec2 p, float lod) {
	return height_m_lod((p + vec2(0.5)) / map_size, lod);
}

// Montagne d'une cellule : forme en cloche, versant est hachuré ; renvoie (couverture, encre, ombre).
vec3 pm_mountain(vec2 p, vec2 cell, float footprint) {
	vec2 jitter = pm_hash2(cell) - 0.5;
	vec2 base = (cell + vec2(0.5, 0.85) + jitter * vec2(0.5, 0.2)) * pm_mount_cell_px;
	float h = pm_height(base + vec2(0.0, -pm_mount_cell_px * 0.3), 4.0);
	if (h < pm_mount_min_m) {
		return vec3(0.0);
	}
	float size = pm_mount_cell_px * mix(0.55, 1.05, smoothstep(pm_mount_min_m, 2600.0, h)) * (0.85 + 0.3 * pm_hash(cell + 3.1));
	vec2 q = (p - base) / size;  // x : -0.5..0.5, y : -1 (sommet) .. 0 (pied)
	float x = q.x * 2.0;
	float peak = 1.0 - pow(abs(x), 1.25);
	peak *= 1.0 + 0.12 * sin(x * 9.0 + pm_hash(cell) * 6.0);
	float top = -peak * mix(0.75, 1.05, pm_hash(cell + 7.7));
	float inside = step(abs(x), 1.0) * step(top, q.y) * step(q.y, 0.0);
	if (inside <= 0.0 && abs(x) > 1.1) {
		return vec3(0.0);
	}
	float aa = footprint / size * 1.2;
	// Contour : distance approximative au profil (verticale) et au pied.
	float d_top = abs(q.y - top) * size;
	float outline = inside > 0.0 || d_top < footprint * 1.5 ? pm_line(d_top, 1.0, footprint) * step(abs(x), 1.0) : 0.0;
	// Versant à l'ombre (est) hachuré de traits verticaux.
	float shade_side = smoothstep(-0.05, 0.1, x) * inside;
	float hatch = smoothstep(0.55, 0.8, abs(fract(p.x / max(footprint * 3.0, 1.2)) - 0.5) * 2.0);
	float ridge = pm_line((x - 0.05 * sin(q.y * 7.0)) * size * 0.5, 0.5, footprint) * inside * step(-0.85 * peak, q.y);
	return vec3(inside, max(outline, ridge * 0.6), shade_side * (0.25 + 0.75 * hatch));
}

// Arbre d'une cellule (houppier rond + fût) : renvoie (couverture, encre, ombre).
vec3 pm_tree(vec2 p, vec2 cell, float footprint) {
	vec2 jitter = pm_hash2(cell + 11.0) - 0.5;
	vec2 base = (cell + vec2(0.5, 0.8) + jitter * 0.35) * pm_tree_cell_px;
	vec4 sp = texture(splat, (base + vec2(0.5)) / map_size);
	float forest = sp.b / max(sp.r + sp.g + sp.b + sp.a, 1e-3);
	if (forest < pm_tree_min + (pm_hash(cell) - 0.5) * 0.25 || pm_hash(cell + 2.7) < 0.35 || pm_height(base, 0.0) <= 0.0) {
		return vec3(0.0);
	}
	float r = pm_tree_cell_px * (0.3 + 0.08 * pm_hash(cell + 5.0));
	vec2 crown = base + vec2(0.0, -r * 1.35);
	float dc = length(p - crown) - r;
	float trunk_d = max(abs(p.x - base.x), abs(p.y - (base.y - r * 0.35)) - r * 0.45);
	float cover = max(step(dc, 0.0), step(trunk_d, footprint * 0.5));
	float ink = max(pm_line(dc, 0.7, footprint), pm_line(trunk_d, 0.55, footprint));
	float shade = step(dc, 0.0) * smoothstep(-0.2, 0.6, dot(normalize(p - crown + 1e-4), vec2(0.7, 0.7)));
	return vec3(cover, ink, shade);
}

// Couleur parchemin du terrain. `coast` : distance signée à la côte (px carte, > 0 à terre).
vec3 parchment_terrain(vec2 p, vec2 uv, float footprint, float coast, float lake, int id, int other_id, float d) {
	vec3 col = pm_paper_at(p, uv, footprint);
	float land = smoothstep(-0.3, 0.3, coast) * (1.0 - lake);

	// --- Lavis de faction (aquarelle : pigment accumulé au bord, granulation) ---
	vec4 fc = faction_of(id);
	vec4 fo = faction_of(other_id);
	bool realm = other_id != id && (distance(fo.rgb, fc.rgb) > 0.02 || abs(fo.a - fc.a) > 0.1);
	float seen = (fog_enabled && id > 0) ? step(0.5, fog_seen(id)) : 1.0;
	if (id > 0 && fc.a > 0.0) {
		vec3 wash = pow(fc.rgb, vec3(2.2));
		wash = wash / max(max(wash.r, max(wash.g, wash.b)), 0.05);
		float gran = 0.8 + 0.4 * pm_noise(p * 0.35) * pm_noise(p * 0.07 + 4.0);
		float pool = realm ? 1.0 - smoothstep(0.0, 7.0, d) : 0.0;
		float strength = pm_wash_strength * gran * (1.0 + 0.9 * pool) * mix(0.3, 1.0, seen);
		col *= mix(vec3(1.0), wash, clamp(strength, 0.0, 0.95) * land);
	}
	if (id == selected_id && id > 0) {
		col = mix(col, vec3(0.95, 0.72, 0.25), 0.25 * land);
	} else if (id == hovered_id && id > 0) {
		col *= 1.12;
	}
	if (mask_enabled && id > 0) {
		float m = texelFetch(province_mask, ivec2(clamp(id, 0, textureSize(province_mask, 0).x - 1), 0), 0).r;
		col = m > 0.25 ? col : col * 0.72;
	}

	// --- Relief : hachures (lumière du nord-ouest) hors des massifs ---
	float lod = 3.0;
	float k = 8.0;
	vec2 g = vec2(pm_height(p + vec2(k, 0.0), lod) - pm_height(p - vec2(k, 0.0), lod),
		pm_height(p + vec2(0.0, k), lod) - pm_height(p - vec2(0.0, k), lod)) / (2.0 * k * meters_per_px);
	float hc = pm_height(p, lod);
	vec3 nrm = normalize(vec3(-g.x * 9.0, 1.0, -g.y * 9.0));
	float shade = clamp(1.0 - dot(nrm, normalize(vec3(pm_light_dir.x, 1.1, pm_light_dir.y))) * 1.05, 0.0, 1.0);
	float slope = length(g);
	float mountain_zone = smoothstep(pm_mount_min_m - 150.0, pm_mount_min_m + 250.0, hc);
	float hatch_amount = smoothstep(0.012, 0.06, slope) * smoothstep(0.05, 0.35, shade) * (1.0 - mountain_zone * 0.85);
	float spacing = pm_hatch_px * exp2(max(floor(log2(max(footprint, 0.5) * 1.6)), 0.0));
	float s1 = abs(fract(dot(p, vec2(0.7071, 0.7071)) / spacing) - 0.5) * 2.0;
	float s2 = abs(fract(dot(p, vec2(0.7071, -0.7071)) / spacing) - 0.5) * 2.0;
	float wline = clamp(hatch_amount, 0.0, 1.0) * 0.55;
	float aa = footprint / spacing * 2.0;
	float h1 = 1.0 - smoothstep(wline - aa, wline + aa, s1);
	float h2 = (1.0 - smoothstep(wline * 0.8 - aa, wline * 0.8 + aa, s2)) * step(0.55, shade);
	float hatch = max(h1, h2 * 0.8) * step(0.001, hatch_amount);
	col = mix(col, pm_ink * 3.0, hatch * 0.55 * land);

	// --- Montagnes dessinées (cellule et voisines du sud, qui passent devant) ---
	vec2 cm = floor(p / pm_mount_cell_px);
	bool mountains_near = hc > pm_mount_min_m - 500.0;
	for (int j = -1; j <= (mountains_near ? 1 : -2); j++) {
		for (int i = -1; i <= 1; i++) {
			vec3 m = pm_mountain(p, cm + vec2(float(i), float(j)), footprint);
			if (m.x > 0.0 || m.y > 0.0) {
				vec3 fill = mix(pm_paper * vec3(1.04, 1.0, 0.92), pm_paper_dark * 0.55, m.z * 0.8);
				col = mix(col, fill, m.x * land);
				col = mix(col, pm_ink, m.y * land);
			}
		}
	}

	// --- Forêts : petits arbres (cellule et voisine du sud) ---
	vec4 sp = texture(splat, uv);
	float forest_here = sp.b / max(sp.r + sp.g + sp.b + sp.a, 1e-3);
	col = mix(col, col * vec3(0.86, 0.95, 0.78), smoothstep(pm_tree_min - 0.15, pm_tree_min + 0.2, forest_here) * land * 0.6);
	if (mountain_zone < 0.6 && forest_here > pm_tree_min - 0.3) {
		vec2 ct = floor(p / pm_tree_cell_px);
		for (int j = 0; j <= 1; j++) {
			for (int i = -1; i <= 1; i++) {
				vec3 t = pm_tree(p, ct + vec2(float(i), float(j)), footprint);
				if (t.x > 0.0 || t.y > 0.0) {
					vec3 leaf = mix(pm_green_ink * 3.2 + pm_paper * 0.25, pm_green_ink * 1.4, t.z);
					col = mix(col, leaf, t.x * land);
					col = mix(col, pm_ink * 1.5, t.y * land);
				}
			}
		}
	}

	// --- Frontières : provinces en pointillé fin, royaumes à l'encre ---
	if (has_border_dist && id > 0 && d < 12.0) {
		float dots = step(0.45, fract((p.x + p.y) * 0.18 + pm_noise(p * 0.2)));
		float prov = pm_line(d, 0.45, footprint) * dots * 0.6;
		float rl = realm ? pm_line(d, 1.3, footprint) : 0.0;
		col = mix(col, realm ? mix(pm_ink, pm_red_ink, 0.35) : pm_ink, max(prov, rl) * land);
	}

	// --- Fleuves : trait d'encre bleue (distance signée du lit, lot V4) ---
	if (has_river_bed) {
		float sd = river_sd_at(p, map_size);
		col = mix(col, vec3(0.05, 0.13, 0.16), pm_line(max(sd, 0.0), 0.7, footprint) * 0.85 * land);
	}

	// --- Eaux : lacs au lavis bleu-vert, côtes et rives cernées d'un trait d'encre ---
	vec3 water = mix(pm_paper_at(p, uv, footprint), pm_sea_wash, 0.8);
	col = mix(col, water, clamp(lake + (1.0 - smoothstep(-0.3, 0.3, coast)), 0.0, 1.0));
	col = mix(col, pm_ink, pm_line(coast, 1.0, footprint) * 0.9);
	col = mix(col, pm_ink, pm_frame(uv, footprint, map_size) * 0.85);
	return col;
}

// Crochet unique du terrain : mélange selon `campaign_parchment`, aplat non ombré.
void parchment_apply(vec2 p, vec2 uv, float footprint, float coast, float lake, int id, int other_id,
		float d, inout vec3 col, inout vec3 n, inout float roughness, out vec3 emission) {
	emission = vec3(0.0);
	float w = pm_weight();
	if (w <= 0.001) {
		return;
	}
	vec3 parch = parchment_terrain(p, uv, footprint, coast, lake, id, other_id, d);
	vec3 albedo;
	pm_output(col, parch, w, albedo, emission);
	col = albedo;
	n = normalize(mix(n, vec3(0.0, 1.0, 0.0), w));
	roughness = mix(roughness, 1.0, w);
}

// Parchemin seul (poids 1) : le terrain saute tout le rendu 3D. Recalcule côte, lacs et
// province de part et d'autre de la frontière la plus proche (même méthode que le terrain).
void parchment_only(vec2 p, vec2 uv, float footprint, out vec3 albedo, out vec3 emission) {
	float hc = height_m_lod(uv, 0.0);
	float coast = has_coast_dist ? (texture(coast_dist, uv).r * 255.0 - 128.0) / coast_dist_scale : (hc > 0.0 ? 8.0 : -8.0);
	float land_f = smoothstep(-0.4, 0.6, coast);
	float lake = (1.0 - land_f) * step(0.0, hc);
	int id = province_at(p);
	int other_id = id;
	float d = 1e3;
	if (has_border_dist) {
		d = border_d(uv);
		if (d < 12.0) {
			vec2 e = vec2(1.0 / map_size.x, 0.0);
			vec2 f = vec2(0.0, 1.0 / map_size.y);
			vec2 gd = vec2(border_d(uv + e) - border_d(uv - e), border_d(uv + f) - border_d(uv - f));
			vec2 away = gd / max(length(gd), 1e-4);
			id = province_at(p + away * max(0.0, 1.3 - d));
			other_id = province_at(p - away * (d + 1.3));
		}
	}
	vec3 parch = parchment_terrain(p, uv, footprint, coast, lake, id, other_id, d);
	pm_output(vec3(0.0), parch, 1.0, albedo, emission);
}
