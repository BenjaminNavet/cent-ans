// Lot CV1 : campagne vivante, fonctions incluses par terrain.gdshader (crochets localisés).
// - Saisons : poids globaux `campaign_season` (printemps, été, automne, hiver ; somme 1),
//   publiés par season_visuals.gd, glissant d'un tour à l'autre. La palette d'origine du
//   terrain (lot V2b) est celle de l'été ; les autres saisons la transforment.
// - Terroirs : masque terroir_mask.gd (R cultures, G vigne, B brûlis, A pâtures).
// - Ombres de nuages qui passent.
// Inclus après les fonctions utilitaires de terrain.gdshader (value_noise, fbm).

global uniform vec4 campaign_season;
uniform sampler2D terroir_mask : filter_linear, repeat_disable, hint_default_black;
uniform bool has_terroir = false;
uniform bool life_enabled = true;
// Neige : altitude (m) de la limite par saison (printemps, été, automne, hiver).
uniform vec4 season_snow_line = vec4(1500.0, 2750.0, 1900.0, 520.0);
// Couverture neigeuse de plaine en hiver : base, nord, est (0-1).
uniform vec3 winter_cover = vec3(0.10, 0.42, 0.22);
// TB1 : limite sud de la neige de plaine (v de carte : début, fin du fondu) et retrait au sud.
uniform vec3 winter_south = vec3(0.58, 0.72, 0.35);
// TB1 : bord des plaques de neige (demi-largeur du fondu : vue large, vue moyenne et proche),
// découpe du bord par le bruit fin, part de neige retirée sous les bois, opacité de près.
uniform vec2 snow_edge_soft = vec2(0.07, 0.24);
uniform float snow_edge_noise = 0.34;
uniform float snow_forest_shed = 0.7;
uniform float snow_near_opacity = 0.8;
// TB1 : sol d'hiver hors neige : part de gris et teinte brun-gris (luminance gardée × teinte).
uniform float winter_ground_desat = 0.55;
uniform vec3 winter_ground_tint = vec3(1.10, 1.0, 0.86);
// TB1 : roux d'automne des prés et des forêts posé sur la carte de couleur (vert baissé, pas
// d'orange vif) ; les cultures en reçoivent la moitié.
uniform vec3 autumn_russet = vec3(1.03, 0.80, 0.86);
// TB1 : dorure des cultures en été, appliquée par-dessus la carte de couleur (saison moyenne).
uniform vec3 summer_gold = vec3(1.12, 1.04, 0.84);
uniform vec3 life_snow_color = vec3(0.80, 0.82, 0.86);
// Brûlis (dévastation) : cendres et chaume noirci.
uniform vec3 burnt_color = vec3(0.022, 0.014, 0.009);  // TB4 : brun-noir chaud (linéaire ; ≈ 40 31 24 à l'écran, plus de gris neutre)
uniform vec3 ash_color = vec3(0.060, 0.050, 0.040);
// TB4 : brûlis par parcelles. À 100 % de dévastation, part des parcelles carbonisées et part des
// parcelles roussies (le reste est intact) ; teinte des champs roussis ; part du sol d'origine
// gardée sous le noir (relief, chemins et sillons restent lisibles).
uniform bool burn_active = false;  // posé par CampaignLife : une province au moins est dévastée
uniform float burn_charred_share = 0.36;
uniform float burn_singed_share = 0.33;
uniform vec3 burn_singe_tint = vec3(0.92, 0.50, 0.20);  // × luminance du sol : roux sombre
uniform float burn_ground_keep = 0.22;
// Ombres de nuages : force, échelle (par pixel carte), vent.
uniform float cloud_shadow_amount = 0.18;
uniform float cloud_shadow_scale = 0.010;
uniform vec2 cloud_shadow_wind = vec2(0.012, 0.005);

vec4 life_season() {
	return life_enabled ? campaign_season : vec4(0.0, 1.0, 0.0, 0.0);
}

// Parcelle selon la saison. cls : 0 blé, 1 culture verte, 2 labour, 3 jachère, 4 pré, 5 foin,
// 6 sol de vigne. h : tirage [0, 1] de la parcelle (variété d'une parcelle à l'autre).
vec3 season_field(vec3 summer, int cls, float h) {
	if (!life_enabled) {
		return summer;
	}
	vec4 w = life_season();
	vec3 young = vec3(0.10, 0.19, 0.045);    // blé en herbe, pousses de printemps
	vec3 plough = vec3(0.13, 0.085, 0.048);  // labour frais
	vec3 wet_plough = vec3(0.085, 0.058, 0.036);
	vec3 stubble = vec3(0.25, 0.20, 0.105);  // chaume
	vec3 meadow_spring = vec3(0.085, 0.19, 0.035);
	vec3 meadow_autumn = vec3(0.11, 0.12, 0.045);
	vec3 meadow_winter = vec3(0.075, 0.085, 0.048);
	vec3 spring = summer;
	vec3 autumn = summer;
	vec3 winter = summer;
	vec3 ripe = summer;
	if (cls == 0) {
		// Blé mûr doré l'été : en herbe au printemps (quelques labours tardifs), chaume puis labours à l'automne,
		// semis d'hiver clairsemés sur la terre nue en hiver.
		ripe = summer * vec3(1.15, 1.06, 0.78);
		spring = h < 0.8 ? young : plough;
		autumn = h < 0.45 ? stubble : plough;
		winter = mix(wet_plough, young, 0.3);
	} else if (cls == 1) {
		spring = mix(young, summer, 0.25);
		autumn = h < 0.5 ? plough : mix(stubble, plough, 0.5);
		winter = wet_plough;
	} else if (cls == 2) {
		// Labours : printemps = labours et semis (terre fraîche), été = chaume.
		spring = plough;
		autumn = plough * 0.9;
		winter = wet_plough;
	} else if (cls == 3) {
		spring = h < 0.5 ? plough : mix(summer, young, 0.5);
		autumn = mix(summer, stubble, 0.4);
		winter = mix(wet_plough, meadow_winter, 0.5);
	} else if (cls == 4) {
		spring = meadow_spring;
		autumn = meadow_autumn;
		winter = meadow_winter;
	} else if (cls == 5) {
		spring = meadow_spring * 1.05;
		autumn = mix(meadow_autumn, stubble, 0.35);
		winter = mix(meadow_winter, stubble * 0.6, 0.4);
	} else {
		spring = summer;
		autumn = summer * vec3(1.05, 0.95, 0.9);
		winter = summer * 0.85;
	}
	return spring * w.x + ripe * w.y + autumn * w.z + winter * w.w;
}

// Teinte de fond des prés et des cultures (lointain) : même moyenne que les parcelles.
vec3 season_grass(vec3 summer) {
	vec4 w = life_season();
	vec3 spring = summer * vec3(0.9, 1.18, 0.85);
	vec3 autumn = mix(summer, vec3(0.13, 0.12, 0.05), 0.5);
	vec3 winter = mix(summer, vec3(0.08, 0.085, 0.055), 0.65);
	return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

vec3 season_farm(vec3 summer) {
	vec4 w = life_season();
	vec3 spring = mix(summer, vec3(0.11, 0.15, 0.05), 0.6);
	vec3 autumn = mix(summer, vec3(0.15, 0.11, 0.06), 0.6);
	vec3 winter = mix(summer, vec3(0.095, 0.075, 0.05), 0.7);
	return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

// Feuilles de vigne : vert tendre, puis rousses et pourpres à l'automne, taille nue l'hiver.
vec3 season_vine_leaf(vec3 summer) {
	vec4 w = life_season();
	vec3 autumn = vec3(0.24, 0.085, 0.02);
	vec3 winter = vec3(0.10, 0.07, 0.045);
	return summer * 1.15 * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

// Forêts vues comme terrain (lointain) : roux, or et verts persistants à l'automne ;
// bois nus gris-brun l'hiver, résineux gardés (tirage par bosquet).
vec3 season_forest(vec3 summer, float grove) {
	vec4 w = life_season();
	if (w.y > 0.999) {
		return summer;
	}
	vec3 spring = summer * vec3(1.05, 1.3, 0.9);
	vec3 russet = grove < 0.35 ? vec3(0.115, 0.042, 0.010) : (grove < 0.7 ? vec3(0.12, 0.08, 0.015) : summer);
	vec3 autumn = russet;
	vec3 bare = vec3(0.050, 0.043, 0.036);
	vec3 winter = grove < 0.72 ? bare : summer * 0.85;
	return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

// TB1 : moyenne de `season_forest` sur les bosquets (vue large : un pixel couvre plusieurs bosquets).
vec3 season_forest_mean(vec3 summer) {
	vec4 w = life_season();
	vec3 spring = summer * vec3(1.05, 1.3, 0.9);
	vec3 autumn = 0.35 * vec3(0.115, 0.042, 0.010) + 0.35 * vec3(0.12, 0.08, 0.015) + 0.30 * summer;
	vec3 winter = 0.72 * vec3(0.050, 0.043, 0.036) + 0.28 * summer * 0.85;
	return spring * w.x + summer * w.y + autumn * w.z + winter * w.w;
}

// TB1 : écart saisonnier multiplicatif (saison / été), borné, à appliquer sur une couleur de
// saison moyenne (carte de couleur SS2, matières HB3) qui ne connaît pas la saison.
vec3 season_ratio(vec3 season_col, vec3 summer_col) {
	return clamp(season_col / max(summer_col, vec3(1e-3)), vec3(0.35), vec3(2.6));
}

// Neige selon la saison : limite d'altitude qui descend en hiver (plus bas au nord et à l'est),
// et couverture de plaine en plaques l'hiver. `base` : neige d'altitude d'origine ; `forest_f` :
// part de forêt au pixel (la neige de plaine tient moins sous les bois).
float season_snow(float base, float hc, vec2 uv, vec2 p, float slope, float macro, float footprint, float forest_f) {
	if (!life_enabled) {
		return base;
	}
	vec4 w = campaign_season;
	float line = dot(w, season_snow_line);
	float north = smoothstep(0.62, 0.36, uv.y);
	float east = smoothstep(0.50, 0.66, uv.x);
	line -= (north * 0.6 + east * 0.4) * (w.w * 420.0 + w.z * 300.0 + w.x * 300.0);
	float mountain = smoothstep(line - 150.0, line + 250.0, hc + (macro - 0.5) * 500.0);
	float low = 0.0;
	if (w.w > 0.01) {
		// TB1 : pas de neige de plaine au sud (Méditerranée), sauf en altitude.
		float south = smoothstep(winter_south.x, winter_south.y, uv.y);
		float cover = w.w * clamp(winter_cover.x * (1.0 - south) + winter_cover.y * north + winter_cover.z * east
			+ hc / 900.0 * (1.0 - 0.5 * south) - winter_south.z * south, 0.0, 1.0);
		// TB1 : plaques régionales (nappes de ~40 km, contour à ~12 km) lisibles en vue large ; le
		// bruit est étalé pour que `cover` soit la part couverte. En vue moyenne et proche, le bord
		// s'élargit et se découpe sur deux octaves plus fines (chacune s'efface avant de passer sous
		// le pixel : pas de taches), la neige se pose moins sous les bois et reste un voile.
		float mid = 1.0 - smoothstep(0.8, 2.5, footprint);
		float near = 1.0 - smoothstep(0.55, 1.1, footprint);
		float patches = fbm(p * 0.018) * 0.78 + value_noise(p * 0.071 + 5.3) * 0.22 * mid + 0.11 * (1.0 - mid);
		patches = smoothstep(0.24, 0.76, patches);
		float oct_a = (value_noise(p * 0.19 + 2.1) - 0.5) * (1.0 - smoothstep(0.45, 1.3, footprint));
		float oct_b = (value_noise(p * 0.47 + 8.4) - 0.5) * (1.0 - smoothstep(0.18, 0.6, footprint));
		patches += (oct_a * 0.65 + oct_b * 0.35) * snow_edge_noise;
		float soft = mix(snow_edge_soft.x, snow_edge_soft.y, near);
		low = smoothstep(1.0 - cover - soft, 1.0 - cover + soft, patches) * step(0.02, cover);
		low *= 1.0 - snow_forest_shed * forest_f * mix(0.35, 1.0, near);
		low *= mix(1.0, snow_near_opacity, near);
		// Poudrage : la neige mince laisse deviner sillons et chaumes.
		low *= mix(0.72, 0.95, smoothstep(0.3, 0.9, cover));
		low *= step(1.0, hc);
	}
	return max(max(base * w.y, mountain), low) * (1.0 - smoothstep(0.55, 0.9, slope));
}

// TB1 : sol d'hiver hors neige, brun-gris et désaturé (la neige ne tranche plus sur un sol vert).
vec3 season_winter_ground(vec3 col, float land_f) {
	if (!life_enabled) {
		return col;
	}
	float lum = dot(col, vec3(0.2126, 0.7152, 0.0722));
	return mix(col, lum * winter_ground_tint, winter_ground_desat * campaign_season.w * land_f);
}

// Ombres de nuages qui passent (facteur multiplicatif de l'albédo, 1 = pas d'ombre).
float cloud_shadow(vec2 p) {
	if (!life_enabled || cloud_shadow_amount <= 0.0) {
		return 1.0;
	}
	vec2 q = p * cloud_shadow_scale + TIME * cloud_shadow_wind;
	float c = value_noise(q) * 0.62 + value_noise(q * 2.7 + vec2(3.1, 7.7)) * 0.38;
	return 1.0 - cloud_shadow_amount * smoothstep(0.45, 0.78, c);
}

// --- Terroirs (masque terroir_mask.gd) ---
vec4 terroir_at(vec2 uv) {
	return (life_enabled && has_terroir) ? texture(terroir_mask, uv) : vec4(0.0);
}

// Finage : les prés et friches proches des colonies deviennent des champs (R), les pâtures
// gagnent en couronne (A). Poids du splat (R prairie, G cultures, B forêt, A roche/lande).
vec4 terroir_splat(vec4 s, vec4 tr) {
	float to_farm = s.r * tr.r * 0.75 + s.a * tr.r * 0.4;
	s.r -= s.r * tr.r * 0.75;
	s.a -= s.a * tr.r * 0.4;
	s.g += to_farm;
	float to_grass = s.g * tr.a * 0.35 * (1.0 - tr.r);
	s.g -= to_grass;
	s.r += to_grass;
	return s;
}

// Vigne autour des colonies des régions viticoles (occupation du sol : R vigne).
vec3 terroir_landuse(vec3 lu, vec4 tr) {
	lu.r = max(lu.r, tr.g * 1.6);
	return lu;
}

// Brûlis (TB4) : dévastation par parcelles. `burn` : masque au pixel ; `keep` : atténuation (neige) ;
// `parcel` : parcelle de l'habillage HB (`hb_apply` : centre, tirage, poids) ; `mask_scale` :
// 1 / plus grand côté de la carte (échelle du masque de terroir).
vec3 terroir_burn(vec3 col, float burn, float keep, vec2 p, float footprint, vec4 parcel, float mask_scale) {
	if (!burn_active || !life_enabled || !has_terroir) {
		return col;
	}
	const vec3 lum_w = vec3(0.2126, 0.7152, 0.0722);
	// Découpe large : la zone ravagée n'est pas un disque net (bruit éteint quand il passe sous le pixel).
	float coarse = 1.0 - smoothstep(1.5, 5.0, footprint);
	float patch_px = mix(0.5, value_noise(p * 0.13 + 5.0) * 0.65 + value_noise(p * 0.41 + 9.0) * 0.35, coarse);
	float burn_px = clamp(burn * (0.45 + 1.1 * patch_px), 0.0, 1.0) * keep;
	float lum = dot(col, lum_w);
	// Lointain : moyenne des trois états, sans dessin de parcelle.
	vec3 char_far = mix(mix(burnt_color, ash_color, 0.15), col, burn_ground_keep);
	vec3 singe_far = mix(col, lum * burn_singe_tint, 0.7);
	float c_far = burn_charred_share * burn_px;
	float s_far = burn_singed_share * burn_px;
	vec3 far = col * (1.0 - c_far - s_far) + char_far * c_far + singe_far * s_far;
	if (parcel.w <= 0.001) {
		return far;
	}
	// Près : la parcelle entière prend un état, tiré contre la dévastation lue en son centre (le
	// bord de la zone s'effrange parcelle par parcelle).
	float patch_c = value_noise(parcel.xy * 0.13 + 5.0) * 0.65 + value_noise(parcel.xy * 0.41 + 9.0) * 0.35;
	float burn_c = clamp(texture(terroir_mask, parcel.xy * mask_scale).b * (0.45 + 1.1 * patch_c), 0.0, 1.0) * keep;
	float h = parcel.z;
	float charred = step(h, burn_charred_share * burn_c);
	float singed = step(h, (burn_charred_share + burn_singed_share) * burn_c) - charred;
	float fine = 1.0 - smoothstep(0.8, 2.5, footprint);
	float ash = value_noise(p * 1.1 + 11.0);
	float stubble = value_noise(p * 6.3 + 3.0);
	vec3 scorched = mix(burnt_color, ash_color, ash * ash * 0.6 * fine + 0.1);
	vec3 char_col = mix(scorched, col, burn_ground_keep * mix(1.0, 0.6 + 0.8 * stubble, fine));
	vec3 singe_col = mix(col, lum * burn_singe_tint, 0.55 + 0.3 * fract(h * 7.31));
	vec3 near = mix(mix(col, char_col, charred), singe_col, singed);
	return mix(far, near, parcel.w);
}
