// Lot RV-C : ombres de nuages qui dérivent sur la carte de campagne (remplace l'ombre CV1 de
// campaign_life.gdshaderinc). Inclus après campaign_weather.gdshaderinc (wx_noise, wx_fbm,
// wx_sample, wx_cloud_shape, weather_wind).
// - Cumulus de beau temps : bruit fbm à domaine déformé, cellules de 5 à 15 km (1 px carte ≈
//   0,72 km) séparées par du ciel clair, bord net mais doux ; couverture variable d'une région à
//   l'autre, plus forte là où la météo du cœur donne pluie, neige ou orage (ciel chargé).
// - Sous les nuées visibles (campaign_clouds.gdshader) : même champ (`wx_cloud_shape`), lu au point
//   d'où part le rayon du soleil vers le sol (décalage selon la direction du soleil).
// - Dérive avec le vent météo (`weather_wind`), plus lente pour les cumulus (`cloud_shadow_drift`).
// - S'efface quand un cumulus ne couvre plus que quelques pixels d'écran (pas de scintillement).
// Valeurs du bloc `clouds` de data/ui/campaign_map.json (CampaignWeatherView) ; repli : ci-dessous.

// Force de l'ombre sous les nuées de la météo (pluie, neige, orage), posée par CampaignWeatherView
// (`weather_shadow` des données) ; 0 par temps clair.
uniform float cloud_shadow_amount = 0.05;
// Force de l'ombre au cœur d'un cumulus de beau temps (part de lumière retirée, 0-1).
uniform float cloud_shadow_cumulus = 0.26;
// Fréquence des cumulus (1 / px carte) : 0,07 ≈ cellules de 14 px, soit ≈ 10 km.
uniform float cloud_shadow_scale = 0.07;
// Couverture de beau temps (part du ciel couverte, 0-1) : minimum et maximum selon les régions,
// et couverture sous un ciel chargé (pluie, neige, orage).
uniform vec2 cloud_shadow_cover = vec2(0.12, 0.38);
uniform float cloud_shadow_cover_wet = 0.7;
// Largeur du bord des cumulus (en part du seuil de bruit) : petit = bord net.
uniform float cloud_shadow_edge = 0.12;
// Dérive des cumulus en part du vent météo (`weather_wind`, px carte / s).
uniform float cloud_shadow_drift = 0.3;
// Teinte de l'ombre (lumière du ciel seule : légère dominante froide), multipliée au sol.
uniform vec3 cloud_shadow_tint = vec3(0.94, 0.97, 1.06);
// Direction vers le soleil (monde), posée depuis le nœud Sun par CampaignWeatherView.
uniform vec3 cloud_shadow_sun = vec3(0.35, 0.71, 0.61);
// Altitude des nuées au-dessus du sol (unités monde = px carte) pour le décalage solaire.
uniform float cloud_shadow_height = 34.0;
// Taille apparente (px d'écran) d'un cumulus en deçà de laquelle l'ombre s'efface : début, fin.
uniform vec2 cloud_shadow_fade_px = vec2(10.0, 4.0);

// Densité [0, 1] des cumulus de beau temps au point p (px carte) ; `wet` = ciel chargé (0-1).
float cs_cumulus(vec2 p, float wet, float footprint) {
	vec2 drift = TIME * weather_wind * cloud_shadow_drift;
	vec2 q = (p - drift) * cloud_shadow_scale;
	// Domaine déformé : contours de cumulus bourgeonnants plutôt que taches rondes.
	vec2 warp = vec2(wx_fbm(q * 0.55 + vec2(1.7, 9.2)), wx_fbm(q * 0.55 + vec2(8.3, 2.8))) - 0.5;
	float c = wx_fbm(q + warp * 1.4);
	// Détail fin du bord, retiré quand il passerait sous le pixel.
	float fine_w = 1.0 - smoothstep(0.25, 0.6, footprint * cloud_shadow_scale * 3.0);
	c += (wx_noise(q * 3.7 + warp * 2.0 + vec2(4.1, 0.6)) - 0.5) * 0.16 * fine_w;
	// Couverture régionale : champ très lent (≈ 200 km), plus quelques nappes de ciel chargé.
	float region = wx_fbm((p - drift * 0.5) * 0.0035 + vec2(3.3, 7.1));
	float cover = mix(cloud_shadow_cover.x, cloud_shadow_cover.y, smoothstep(0.3, 0.7, region));
	cover = mix(cover, cloud_shadow_cover_wet, wet);
	// wx_fbm vaut ≈ 0,5 ± 0,2 : seuil tel que `cover` du ciel soit au-dessus.
	float threshold = mix(0.74, 0.36, cover);
	float edge = cloud_shadow_edge + footprint * cloud_shadow_scale * 0.25;
	return smoothstep(threshold, threshold + edge, c);
}

// Facteur de lumière au sol (1 = plein soleil) au point p ; `fade` = 0 en vue politique,
// parchemin ou désactivé.
vec3 cloud_shadow_light(vec2 p, float footprint, float fade) {
	if (fade <= 0.001 || (cloud_shadow_cumulus <= 0.0 && cloud_shadow_amount <= 0.0)) {
		return vec3(1.0);
	}
	// Taille d'un cumulus à l'écran (px) : s'efface au dézoom avant de scintiller.
	float cell_px = 1.0 / max(cloud_shadow_scale * footprint, 1e-4);
	fade *= smoothstep(cloud_shadow_fade_px.y, cloud_shadow_fade_px.x, cell_px);
	if (fade <= 0.001) {
		return vec3(1.0);
	}
	// Point du plan de nuées traversé par le rayon du soleil qui arrive en p.
	vec3 sun = normalize(cloud_shadow_sun);
	vec2 src = p + sun.xz / max(sun.y, 0.2) * cloud_shadow_height;
	vec4 w = wx_sample(src);
	float wet = clamp(max(max(w.r, w.g), w.a), 0.0, 1.0);
	float shade = cs_cumulus(src, wet, footprint) * cloud_shadow_cumulus;
	// Nuées visibles de la météo : même champ que campaign_clouds.gdshader.
	if (wet > 0.0 && cloud_shadow_amount > 0.0) {
		shade = max(shade, wx_cloud_shape(src, w, 1.0, 0.22) * cloud_shadow_amount);
	}
	shade *= fade;
	float cool = clamp(shade / max(cloud_shadow_cumulus, 1e-3), 0.0, 1.0);
	return mix(vec3(1.0), cloud_shadow_tint, cool) * (1.0 - shade);
}
