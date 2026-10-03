// Lot RV-C : ombres de nuages qui dérivent sur la carte de campagne (remplace l'ombre CV1 de
// campaign_life.gdshaderinc). Employé par le sol (terrain.gdshader), la mer (water.gdshader) et
// les imposteurs d'arbres (campaign_tree_impostor.gdshader) : même champ partout.
// - Cumulus : bruit fbm à domaine déformé, cellules de 5 à 15 km (1 px carte ≈ 0,72 km)
//   séparées par du ciel clair, bord net mais doux ; couverture variable d'une région à l'autre
//   (champ très lent), plus forte là où la météo du cœur donne pluie, neige ou orage.
// - Sous les nuées visibles (campaign_clouds.gdshader) : même champ (`wx_cloud_shape`), seulement
//   dans le sol (`CS_WITH_WEATHER` défini avant l'inclusion, après campaign_weather.gdshaderinc).
// - L'ombre est lue au point du plan de nuées d'où part le rayon du soleil (direction du nœud Sun).
// - Dérive avec le vent météo (`weather_wind`, même valeur par défaut que `cloud_shadow_wind`),
//   plus lente pour les cumulus (`cloud_shadow_drift`).
// - S'efface quand un cumulus ne couvre plus que quelques pixels d'écran (pas de scintillement).
// Valeurs du bloc `clouds` de data/ui/campaign_map.json, posées par CampaignWeatherView sur chaque
// matériau ; repli : valeurs ci-dessous (identiques aux données).

// Force de l'ombre sous les nuées de la météo (pluie, neige, orage), posée par CampaignWeatherView
// (`weather_shadow` des données) ; 0 par temps clair.
uniform float cloud_shadow_amount = 0.05;
// Force de l'ombre au cœur d'un cumulus (part de lumière retirée, 0-1).
uniform float cloud_shadow_cumulus = 0.26;
// Fréquence des cumulus (1 / px carte) : 0,07 ≈ cellules de 14 px, soit ≈ 10 km.
uniform float cloud_shadow_scale = 0.07;
// Couverture de beau temps (part du ciel couverte, 0-1) : minimum et maximum selon les régions,
// et couverture sous un ciel chargé (pluie, neige, orage).
uniform vec2 cloud_shadow_cover = vec2(0.12, 0.38);
uniform float cloud_shadow_cover_wet = 0.7;
// Largeur du bord des cumulus (en part du seuil de bruit) : petit = bord net.
uniform float cloud_shadow_edge = 0.12;
// Vent (px carte / s ; = `weather_wind` de campaign_weather.gdshaderinc) et part de ce vent prise
// par les cumulus.
uniform vec2 cloud_shadow_wind = vec2(3.0, 0.6);
uniform float cloud_shadow_drift = 0.3;
// Teinte de l'ombre (lumière du ciel seule : légère dominante froide), multipliée au sol.
uniform vec3 cloud_shadow_tint = vec3(0.94, 0.97, 1.06);
// Direction vers le soleil (monde), posée depuis le nœud Sun par CampaignWeatherView.
uniform vec3 cloud_shadow_sun = vec3(0.35, 0.71, 0.61);
// Altitude des nuées au-dessus du sol (unités monde = px carte) pour le décalage solaire.
uniform float cloud_shadow_height = 34.0;
// Taille apparente (px d'écran) d'un cumulus où l'ombre commence à s'effacer, puis disparaît.
uniform vec2 cloud_shadow_fade_px = vec2(16.0, 5.0);  // ≈ fondu de la vue politique (footprint 0,9 à 2,8)

float cs_hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

float cs_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(cs_hash(i), cs_hash(i + vec2(1.0, 0.0)), u.x),
		mix(cs_hash(i + vec2(0.0, 1.0)), cs_hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float cs_fbm(vec2 p) {
	return cs_noise(p) * 0.55 + cs_noise(p * 2.1 + 5.2) * 0.3 + cs_noise(p * 4.3 + 1.7) * 0.15;
}

// Point du plan de nuées traversé par le rayon du soleil qui arrive au sol en p.
vec2 cs_source(vec2 p) {
	vec3 sun = normalize(cloud_shadow_sun);
	return p + sun.xz / max(sun.y, 0.2) * cloud_shadow_height;
}

// Fondu selon la taille d'un cumulus à l'écran (footprint : px carte par px d'écran).
float cs_footprint_fade(float footprint) {
	float cell_px = 1.0 / max(cloud_shadow_scale * footprint, 1e-4);
	return smoothstep(cloud_shadow_fade_px.y, cloud_shadow_fade_px.x, cell_px);
}

// Densité [0, 1] des cumulus au point src du plan de nuées ; `wet` = ciel chargé (0-1).
float cs_cumulus(vec2 src, float wet, float footprint) {
	vec2 drift = TIME * cloud_shadow_wind * cloud_shadow_drift;
	vec2 q = (src - drift) * cloud_shadow_scale;
	// Domaine déformé : contours de cumulus bourgeonnants plutôt que taches rondes.
	vec2 warp = vec2(cs_fbm(q * 0.55 + vec2(1.7, 9.2)), cs_fbm(q * 0.55 + vec2(8.3, 2.8))) - 0.5;
	float c = cs_fbm(q + warp * 1.4);
	// Détail fin du bord, retiré quand il passerait sous le pixel.
	float fine_w = 1.0 - smoothstep(0.25, 0.6, footprint * cloud_shadow_scale * 3.0);
	c += (cs_noise(q * 3.7 + warp * 2.0 + vec2(4.1, 0.6)) - 0.5) * 0.16 * fine_w;
	// Couverture régionale : champ très lent (≈ 200 km).
	float region = cs_fbm((src - drift * 0.5) * 0.0035 + vec2(3.3, 7.1));
	float cover = mix(cloud_shadow_cover.x, cloud_shadow_cover.y, smoothstep(0.3, 0.7, region));
	cover = mix(cover, cloud_shadow_cover_wet, wet);
	// cs_fbm vaut ≈ 0,5 ± 0,2 : seuil tel qu'environ `cover` du ciel soit au-dessus.
	float threshold = mix(0.74, 0.36, cover);
	float edge = cloud_shadow_edge + footprint * cloud_shadow_scale * 0.25;
	return smoothstep(threshold, threshold + edge, c);
}

// Part de lumière retirée [0, 1] → facteur RVB (1 = plein soleil), teinte froide au cœur.
vec3 cs_light_factor(float shade) {
	float cool = clamp(shade / max(cloud_shadow_cumulus, 1e-3), 0.0, 1.0);
	return mix(vec3(1.0), cloud_shadow_tint, cool) * (1.0 - shade);
}

// Part de lumière retirée par les cumulus seuls (mer, arbres : pas de météo par province).
// `fade` : 0 en vue politique, parchemin ou désactivé.
float cloud_shadow_fair(vec2 p, float footprint, float fade) {
	fade *= cs_footprint_fade(footprint);
	if (fade <= 0.001 || cloud_shadow_cumulus <= 0.0) {
		return 0.0;
	}
	return cs_cumulus(cs_source(p), 0.0, footprint) * cloud_shadow_cumulus * fade;
}

#ifdef CS_WITH_WEATHER
// Sol : cumulus plus nombreux sous la météo chargée et ombre des nuées visibles. Facteur de
// lumière au sol (1 = plein soleil).
vec3 cloud_shadow_light(vec2 p, float footprint, float fade) {
	fade *= cs_footprint_fade(footprint);
	if (fade <= 0.001 || (cloud_shadow_cumulus <= 0.0 && cloud_shadow_amount <= 0.0)) {
		return vec3(1.0);
	}
	vec2 src = cs_source(p);
	vec4 w = wx_sample(src);
	float wet = clamp(max(max(w.r, w.g), w.a), 0.0, 1.0);
	float shade = cs_cumulus(src, wet, footprint) * cloud_shadow_cumulus;
	if (wet > 0.0 && cloud_shadow_amount > 0.0) {
		shade = max(shade, wx_cloud_shape(src, w, 1.0, 0.22) * cloud_shadow_amount);
	}
	return cs_light_factor(shade * fade);
}
#endif
