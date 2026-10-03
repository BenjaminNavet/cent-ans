shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_disabled;

// Mer (lot V2) : plan à Y = 0. Couleur et opacité selon la profondeur (heightmap), vagues par
// normales animées (estompées au dézoom pour éviter le moiré), fresnel, reflet du soleil
// borné (pas de tache blanche au dézoom), écume douce le long des côtes marines (distance à la
// côte) ; les lacs au-dessus du niveau de la mer sont laissés au shader terrain (A1-04).

uniform sampler2D heightmap : filter_linear_mipmap, repeat_disable, hint_default_black;
uniform int height_bpp = 1;
uniform bool height_little_endian = false;
uniform float height_min_m = -200.0;
uniform float height_max_m = 4800.0;
uniform vec2 map_size = vec2(1.0, 1.0);  // taille du monde (map.json size_px), fournie par le script
uniform bool has_heightmap = false;
uniform sampler2D coast_dist : filter_linear, repeat_disable, hint_default_black;
uniform bool has_coast_dist = false;
uniform float coast_dist_scale = 2.0;

// Couleurs linéaires.
uniform vec3 shallow_color = vec3(0.045, 0.14, 0.14);
uniform vec3 deep_color = vec3(0.006, 0.028, 0.050);
uniform vec3 foam_color = vec3(0.62, 0.66, 0.66);
uniform float depth_falloff_m = 70.0;
uniform float alpha_shallow = 0.7;
uniform float alpha_deep = 0.97;
uniform float wave_scale = 0.6;      // vagues par pixel carte
uniform float wave_speed = 0.35;
uniform float wave_strength = 0.08;
uniform float foam_px = 1.4;          // largeur de la bande d'écume (pixels carte)
uniform float lake_level_m = 2.0;     // au-dessus : plan d'eau intérieur (lac), pas la mer (HB7 : 0,5 → 2, hauts-fonds d'Azov)
uniform float pond_max_px = 3.0;      // étendue plus étroite : étang, pas la mer
uniform float pond_probe_px = 6.0;
uniform float roughness_near = 0.3;
uniform float roughness_far = 0.55;
uniform float specular_amount = 0.06;
// Reflet du soleil (modèle de Blinn-Phong borné, remplace le spéculaire par défaut).
uniform float sun_spec_power = 180.0;
uniform float sun_spec_intensity = 0.1;
uniform float sun_spec_max = 0.1;
// GA4 (ADR 0105) : normale d'eau tuilable (CC0, `water_normal.png`) en deux couches défilantes
// (échelles en pixels carte, vitesses en pixels carte par seconde) et couleur de profondeur à
// trois paliers, réglées depuis `data/fx/campaign_terrain_textures.json`
// (`CampaignTextures.apply_water`) ; `ga4_on` = 0 (`--no-ga4`) : houle procédurale d'origine.
uniform float ga4_on = 0.0;
uniform sampler2D water_normal : filter_linear_mipmap_anisotropic, repeat_enable, hint_default_black;
uniform float ga4_scale_a = 1.7;
uniform float ga4_scale_b = 4.3;
uniform vec2 ga4_velocity_a = vec2(0.05, 0.02);
uniform vec2 ga4_velocity_b = vec2(-0.03, 0.045);
uniform float ga4_normal_strength = 0.35;
uniform vec3 mid_color = vec3(0.018, 0.075, 0.095);
// RC5 (ADR 0141) : détail tuilable Nano Banana 2 — `sea` (water_detail_*, eaux côtières) et
// `ocean` (ocean_detail_*, large), fondus selon la profondeur ; force 0 = rendu inchangé.
#include "res://shaders/water_detail.gdshaderinc"
uniform sampler2D ocean_detail_normal : hint_normal, repeat_enable, filter_linear_mipmap;
uniform sampler2D ocean_detail_albedo : source_color, repeat_enable, filter_linear_mipmap;
uniform float ocean_detail_strength = 0.0;
uniform float ocean_detail_scale = 1.0;
// Profondeurs (m) du passage de la mer côtière au large.
uniform vec2 ocean_detail_depth = vec2(30.0, 120.0);
uniform float mid_depth_m = 25.0;
// Lot TB1 : mer selon la saison (valeurs de `data/ui/campaign_seasons.json`, mêlées par
// `SeasonLook.sea` et posées par `Sea.apply_season`) : teinte multiplicative, gris visé (acier
// l'hiver : une mer profonde seulement assombrie devenait noire) et sa part, écume.
uniform vec3 season_tint = vec3(1.0);
uniform vec3 season_grey = vec3(0.0);
uniform float season_grey_amount = 0.0;
uniform float season_foam = 1.0;
// Lot TB1 : mer sous la tempête. Masque météo par province du lot CM2 (R pluie, A orage), repris
// du terrain par `Sea.sync_weather` ; la tempête d'une province côtière porte à `storm_reach_px`.
uniform sampler2D province_ids : filter_nearest, repeat_disable, hint_default_black;
uniform sampler2D weather_mask : filter_nearest, repeat_disable, hint_default_black;
uniform bool weather_enabled = false;
uniform float storm_foam_gain = 0.0;
uniform float storm_foam_width = 0.0;
uniform float storm_whitecaps = 0.0;
uniform float storm_darken = 0.0;
uniform float storm_reach_px = 12.0;
uniform float storm_rain_share = 0.4;
// Lot CM2 : mer des portulans (vue stratégique parchemin).
#include "res://shaders/parchment_sea.gdshaderinc"
#include "res://shaders/ga_macro.gdshaderinc"  // GA4 : couleur de profondeur
#include "res://shaders/campaign_cloud_shadow.gdshaderinc"  // RV-C : ombres de cumulus
#include "res://shaders/water_glint.gdshaderinc"  // RV-F : paillettes du soleil

varying float v_far;
// RV-C : lumière directe gardée sous les cumulus (1 = plein soleil), calculée dans fragment().
varying vec3 v_cloud_sun;
// RV-F : masque des paillettes et normale au repos (repère vue), du fragment vers light().
varying float v_glint;
varying vec3 v_flat_n;

varying vec3 world_pos;

float height_m(vec2 uv) {
	if (height_bpp == 2) {
		vec4 t = texture(heightmap, uv);
		float v = height_little_endian ? (t.a * 255.0 * 256.0 + t.r * 255.0) : (t.r * 255.0 * 256.0 + t.a * 255.0);
		return mix(height_min_m, height_max_m, v / 65535.0);
	}
	return mix(height_min_m, height_max_m, texture(heightmap, uv).r);
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
	return mix(mix(hash12(i), hash12(i + vec2(1.0, 0.0)), u.x),
		mix(hash12(i + vec2(0.0, 1.0)), hash12(i + vec2(1.0, 1.0)), u.x), u.y);
}

#include "res://shaders/sea_basins.gdshaderinc"  // TB5 : mers par bassin
#include "res://shaders/coast_common.gdshaderinc"  // TB5 : géologie de la côte, ressac

// TB1 : force de la tempête [0, 1] au point p (px carte) : province lue ici et à `storm_reach_px`
// autour (frange déformée par le bruit : pas de bord de province net sur l'eau).
float sea_storm(vec2 p) {
	if (!weather_enabled) {
		return 0.0;
	}
	ivec2 sz = textureSize(province_ids, 0);
	int width = textureSize(weather_mask, 0).x - 1;
	float warp = value_noise(p * 0.05) * 6.2832;
	float sum = 0.0;
	for (int i = 0; i < 5; i++) {
		float ang = warp + float(i) * 1.5708;
		vec2 q = p + (i == 0 ? vec2(0.0) : vec2(cos(ang), sin(ang)) * storm_reach_px * (0.5 + value_noise(p * 0.11 + float(i) * 3.7)));
		vec4 c = texelFetch(province_ids, clamp(ivec2(floor(q)), ivec2(0), sz - ivec2(1)), 0);
		int id = int(round(c.r * 255.0)) | (int(round(c.g * 255.0)) << 8);
		if (id > 0) {
			vec4 w = texelFetch(weather_mask, ivec2(clamp(id, 0, width), 0), 0);
			sum += max(w.a, w.r * storm_rain_share);
		}
	}
	return clamp(sum * 0.4, 0.0, 1.0);
}

// Hauteur de houle (somme de trains d'ondes orientés + bruit), en unités arbitraires.
float swell(vec2 p, float t) {
	float h = 0.0;
	h += sin(dot(p, vec2(0.82, 0.57)) * 1.0 + t * 1.1) * 0.5;
	h += sin(dot(p, vec2(-0.35, 0.94)) * 1.7 - t * 1.4) * 0.3;
	h += sin(dot(p, vec2(0.99, -0.12)) * 2.9 + t * 2.1) * 0.15;
	h += (value_noise(p * 1.3 + t * 0.3) - 0.5) * 0.6;
	return h;
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 p = world_pos.xz;
	vec2 uv = p / map_size;  // SZ2b : pixel carte i centré en x = i + 0,5, comme les outils (ADR 0086)
	float footprint = max(length(dFdx(p)), length(dFdy(p)));
	float far_t = smoothstep(0.6, 2.5, footprint);
	v_far = far_t;
	v_cloud_sun = cs_light_factor(cloud_shadow_fair(p, footprint, 1.0 - pm_weight()));  // RV-C

	bool inside = all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)));
	float h_here = (has_heightmap && inside) ? height_m(uv) : -200.0;
	float depth = max(-h_here, 0.0);
	// TB5 : mer du bassin (teinte, clarté, houle, écume) ; la clarté étire ou resserre la
	// profondeur vue (Méditerranée : hauts-fonds clairs visibles plus loin ; Manche : sombre).
	vec3 basin_tint_here;
	vec4 basin_wave_here;
	vec2 basin_water_here;
	basin_look(uv, basin_tint_here, basin_wave_here, basin_water_here);
	// Retouche TB5 : profondeur exacte sur les petits fonds, lissée au large (mip de la carte
	// d'altitude) : le talus du plateau ne dessine plus de bord net. Eau claire (> 1) : la
	// profondeur vue est divisée ; eau trouble (< 1) : une épaisseur d'eau s'ajoute (la diviser
	// durcissait chaque marche du fond en tache claire).
	float depth_soft = has_heightmap && inside && height_bpp == 1
		? max(-mix(height_min_m, height_max_m, textureLod(heightmap, uv, basin_depth_lod).r), 0.0) : depth;
	float depth_col = mix(depth, depth_soft, smoothstep(basin_soft_depth_m.x, basin_soft_depth_m.y, depth));
	float clarity = max(basin_water_here.x, 0.1);
	float seen_depth = clarity >= 1.0 ? depth_col / clarity : depth_col + (1.0 / clarity - 1.0) * basin_murk_m;
	vec3 col = ga_depth_color(seen_depth, shallow_color, mid_color, deep_color, mid_depth_m, depth_falloff_m, ga4_on);  // GA4
	float alpha = mix(alpha_shallow, alpha_deep, smoothstep(0.0, 45.0, seen_depth));

	// Vagues : normale par différences finies de la houle, estompée au dézoom.
	float t = TIME * wave_speed;
	vec2 q = p * wave_scale;
	float h0 = swell(q, t);
	vec3 n;
	if (ga4_on > 0.5) {
		// GA4 : deux couches de normale défilant en sens différents (la seconde tournée), mêlées
		// en « whiteout » ; les mipmaps lissent le lointain, estompe plus tardive que la houle.
		vec2 ua = (p + TIME * ga4_velocity_a) / ga4_scale_a;
		vec2 ub = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8)) * (p + TIME * ga4_velocity_b) / ga4_scale_b;
		vec2 na = texture(water_normal, ua).rg * 2.0 - 1.0;
		vec2 nb = texture(water_normal, ub).rg * 2.0 - 1.0;
		float gs = ga4_normal_strength * basin_wave_here.x * (1.0 - smoothstep(0.6, 2.5, footprint) * 0.7);
		vec2 nxy = (na + nb) * gs;
		n = normalize(vec3(nxy.x, 1.0, nxy.y));
	} else {
		float e = 0.15;
		float hx = swell(q + vec2(e, 0.0), t) - h0;
		float hz = swell(q + vec2(0.0, e), t) - h0;
		float ws = wave_strength * basin_wave_here.x * (1.0 - smoothstep(0.15, 0.8, footprint));
		n = normalize(vec3(-hx / e * ws, 1.0, -hz / e * ws));
	}
	// RC5 : détail côtier puis large (normale ajoutée, teinte relative), estompé au dézoom.
	float rc_near = 1.0 - smoothstep(0.6, 2.5, footprint) * 0.8;
	float rc_ocean = smoothstep(ocean_detail_depth.x, ocean_detail_depth.y, depth);
	vec3 rc_sea_n = water_detail_normal_ws(p, TIME, vec2(0.0)) - vec3(0.0, 1.0, 0.0);
	vec3 rc_ocean_n = vec3(0.0);
	vec3 rc_ocean_col = col;
	if (ocean_detail_strength > 0.0) {
		vec2 ou = p * ocean_detail_scale;
		vec3 on = texture(ocean_detail_normal, ou + vec2(TIME * 0.004, 0.0)).xyz * 2.0 - 1.0;
		rc_ocean_n = vec3(on.x, 0.0, -on.y) * ocean_detail_strength;
		vec3 mean_o = max(textureLod(ocean_detail_albedo, vec2(0.5), 9.0).rgb, vec3(0.02));
		vec3 ratio_o = clamp(texture(ocean_detail_albedo, ou).rgb / mean_o, vec3(0.5), vec3(1.6));
		rc_ocean_col = col * mix(vec3(1.0), ratio_o, ocean_detail_strength);
	}
	vec3 rc_sea_col = water_detail_tint(col, p * water_detail_scale);
	col = mix(col, mix(rc_sea_col, rc_ocean_col, rc_ocean), rc_near);
	n = normalize(n + mix(rc_sea_n, rc_ocean_n, rc_ocean) * rc_near);
	// Creux et crêtes légèrement contrastés dans la couleur (lisibles sans reflet du soleil).
	col *= 1.0 + h0 * 0.06 * basin_wave_here.x * (1.0 - smoothstep(0.25, 1.2, footprint));
	// Retouche TB5 : relief fin des vagues lisible dans la couleur sur toutes les mers (la
	// Méditerranée claire sortait en aplat), plus ou moins marqué selon le clapot du bassin.
	col *= 1.0 + (n.x * 0.6 + n.z * 0.8) * sea_ripple_contrast * (1.0 - smoothstep(0.25, 1.2, footprint));
	// TB5 : longue houle du large (Atlantique), lisible dans la couleur et dans le reflet.
	vec3 ls = long_swell(p, footprint, basin_wave_here.y);
	col *= 1.0 + ls.z * long_swell_contrast;
	n = normalize(n - vec3(ls.x, 0.0, ls.y) * long_swell_normal);
	// TB1 : mer de la saison (gris acier l'hiver), assombrie sous la tempête.
	float storm = sea_storm(p);
	// Retouche TB5 : teinte entière au large, adoucie (racine) sur les petits fonds déjà clairs.
	vec3 basin_tint_seen = mix(sqrt(basin_tint_here), basin_tint_here, smoothstep(0.0, 40.0, depth_col));
	col = mix(col * basin_tint_seen, season_grey, clamp(season_grey_amount * basin_water_here.y, 0.0, 1.0)) * season_tint;  // TB5 : bassin
	col *= 1.0 - storm_darken * storm;

	// Écume : bande près du rivage, lignes qui avancent vers la côte, cassées par le bruit.
	float coast = has_coast_dist && inside ? (texture(coast_dist, uv).r * 255.0 - 128.0) / coast_dist_scale : -50.0;
	float off = max(-coast, 0.0);
	float foam_w = foam_px * (1.0 + storm_foam_width * storm);  // TB1 : bande plus large par tempête
	float band = 1.0 - smoothstep(0.0, foam_w, off);
	float lines = smoothstep(0.55, 0.95, sin(off * 5.5 + TIME * 1.6 + value_noise(p * 0.8) * 4.0) * 0.5 + 0.5);
	float broken = smoothstep(0.35, 0.75, value_noise(p * 1.6 + vec2(TIME * 0.05, 0.0)));
	float foam = band * mix(0.35 + 0.65 * lines * broken, 0.6, far_t);
	foam *= 1.0 - smoothstep(foam_w * 0.2, foam_w, off) * 0.5;
	// TB5 : ressac au trait de côte. La lame va et vient (elle monte peu au pied des falaises) ;
	// la bande d'écume d'origine s'efface d'autant pour rester sobre.
	if (swash_amount > 0.0 && has_coast_dist && inside && coast > -(swash_out_px + swash_film_px + 0.3)) {
		float cliff_f = coast_enabled && has_heightmap ? coast_cliff_factor(height_m(coast_probe_uv(uv, coast, coast_to_land(uv)))) : 0.0;
		foam = max(foam * (1.0 - 0.45 * swash_amount), coast_swash(p, coast, cliff_f, footprint).x);
	}
	foam = clamp(foam * season_foam * basin_wave_here.z * (1.0 + storm_foam_gain * storm), 0.0, 1.6);  // TB1, TB5
	// TB1, TB5 : moutons au large (tempête, bassin houleux) : crêtes rares et irrégulières de
	// près, simple voile de teinte au dézoom (`sea_whitecaps`).
	vec2 caps = sea_whitecaps(p, footprint, max(storm * storm_whitecaps, basin_wave_here.w));
	foam = max(foam, caps.x);
	col = mix(col, foam_color, min(foam * 0.55, 0.9) + caps.y);
	alpha = mix(alpha, 0.9, foam * 0.5);
	// Côte précise (raster) plutôt que l'intersection avec le maillage (LOD grossier) : l'eau
	// s'efface sur la terre même si la géométrie y passe sous le niveau de la mer.
	alpha *= 1.0 - smoothstep(-0.2, 0.6, coast);
	// A1-04 : lacs et étangs (eau au-dessus du niveau de la mer) : ni écume ni plan de mer ; le
	// shader terrain les peint avec une berge fondue. L'écume reste réservée à la mer.
	// Étang côtier sous le niveau de la mer (marais) : petite étendue d'eau reconnue parce qu'à
	// `pond_probe_px` vers le large, on est encore à moins de `pond_max_px` de la rive.
	float lake_f = smoothstep(lake_level_m - 0.5, lake_level_m + 0.5, h_here);
	if (has_coast_dist && inside && coast < 0.5 && coast > -pond_max_px) {
		vec2 ex = vec2(1.0 / map_size.x, 0.0);
		vec2 ey = vec2(0.0, 1.0 / map_size.y);
		vec2 cg = vec2(texture(coast_dist, uv + ex).r - texture(coast_dist, uv - ex).r,
			texture(coast_dist, uv + ey).r - texture(coast_dist, uv - ey).r);
		vec2 to_water = -cg / max(length(cg), 1e-5);
		float probe = (texture(coast_dist, uv + to_water * pond_probe_px / map_size).r * 255.0 - 128.0) / coast_dist_scale;
		lake_f = max(lake_f, smoothstep(-pond_max_px - 0.5, -pond_max_px + 0.5, probe));
	}
	alpha *= 1.0 - lake_f;

	// Fresnel discret : le large reflète un peu plus le ciel en vue rasante.
	float fres = pow(1.0 - clamp(dot(normalize(VIEW), NORMAL), 0.0, 1.0), 5.0);
	col += vec3(0.02, 0.035, 0.045) * fres;
	vec3 pm_emit;
	parchment_sea(p, uv, footprint, coast, lake_f, col, alpha, n, pm_emit);  // CM2

	// RV-F : paillettes du soleil (vue moyenne et régionale), éteintes sur l'écume, les lacs et
	// la terre (alpha) ; le parchemin les efface dans light().
	v_glint = glint_mask(p, footprint, TIME) * (1.0 - clamp(foam, 0.0, 1.0)) * alpha * (1.0 - 0.6 * storm);
	v_flat_n = normalize((VIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz);
	ALBEDO = col;
	EMISSION = pm_emit;  // CM2
	ALPHA = alpha;
	ROUGHNESS = mix(mix(roughness_near, roughness_far, far_t), 0.8, foam);
	SPECULAR = specular_amount * (1.0 - foam);
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}

void light() {
	float ndl = max(dot(NORMAL, LIGHT), 0.0);
	DIFFUSE_LIGHT += ndl * ATTENUATION * LIGHT_COLOR / PI * v_cloud_sun;  // RV-C : ombre des cumulus
	vec3 h = normalize(VIEW + LIGHT);
	float power = mix(sun_spec_power, sun_spec_power * 0.25, v_far);
	float spec = pow(max(dot(NORMAL, h), 0.0), power) * sun_spec_intensity * (1.0 - 0.7 * v_far);
	SPECULAR_LIGHT += min(spec, sun_spec_max) * ATTENUATION * LIGHT_COLOR / PI * (1.0 - pm_weight()) * v_cloud_sun;  // CM2, RV-C
	SPECULAR_LIGHT += glint_light(v_glint, v_flat_n, VIEW, LIGHT) * glint_color * ATTENUATION * LIGHT_COLOR * (1.0 - pm_weight()) * v_cloud_sun;  // RV-F, RV-C : pas de paillettes sous les cumulus
}
