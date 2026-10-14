// CS2 : brume de bord de carte pour les objets instanciés (rochers, arbres) : même fondu que le sol et la mer
// (terrain.gdshader, water.gdshader). Global `campaign_edge` = (largeur carte, hauteur carte, largeur de brume
// en px carte, 0), posé par CampaignWeatherView ; couleur de l'horizon du ciel.
global uniform vec4 campaign_edge;

float ce_haze(vec2 xz) {
	if (campaign_edge.z <= 0.0) {
		return 0.0;
	}
	vec2 q = min(xz, campaign_edge.xy - xz);
	return 1.0 - smoothstep(0.0, campaign_edge.z, min(q.x, q.y));
}

const vec3 CE_HAZE_LINEAR = vec3(0.4968, 0.5402, 0.5402);  // (0,74 ; 0,76 ; 0,76) sRGB en linéaire
