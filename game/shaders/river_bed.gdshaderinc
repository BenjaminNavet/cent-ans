// Lit des fleuves de la carte de campagne (lot V4, A1-11), partagé par terrain.gdshader et
// river_water.gdshader.
// - `river_bed` : distance signée à la berge (px carte, négative dans le lit), L8,
//   (valeur − 128) / river_bed_scale (`cent-ans geo rivers-render`).
// - `carved_rects` : tuiles de relief fin (x0, z0, x1, z1) où le lit est creusé dans le maillage
//   (sommets tous les 0,5 px) ; ailleurs (LOD proche / lointain, sommets tous les 4-8 px) on ne
//   creuse pas, l'eau flotte légèrement au-dessus du relief. Réglées chaque image par
//   `RiversRenderer` ; fondu de `carve_edge_px` au bord des tuiles (pas de fente entre niveaux).

uniform sampler2D river_bed : filter_linear, repeat_disable, hint_default_white;
uniform bool has_river_bed = false;
uniform float river_bed_scale = 16.0;
uniform float river_bank_px = 1.1;
// Profondeur du lit creusé (unités monde ; 1 m = 0,006).
uniform float river_carve_depth = 0.075;
uniform vec4 carved_rects[4];
uniform int carved_rect_count = 0;
uniform float carve_edge_px = 3.0;
// Lot ZG5b : disque où l'hydrographie fine (FineGeoLayer) remplace ce rendu : centre x, z ;
// rayon couvert par les tuiles maillées ; poids du palier près.
uniform vec4 fine_zone = vec4(0.0, 0.0, -1.0, 0.0);

// 1 là où le rendu fin a pris le relais, 0 ailleurs (fondu sur les 15 % extérieurs du rayon).
float fine_zone_mask(vec2 p) {
	if (fine_zone.z <= 0.0 || fine_zone.w <= 0.0) {
		return 0.0;
	}
	return fine_zone.w * (1.0 - smoothstep(fine_zone.z * 0.85, fine_zone.z, distance(p, fine_zone.xy)));
}

float river_sd_at(vec2 p, vec2 size) {
	return (textureLod(river_bed, (p + vec2(0.5)) / size, 0.0).r * 255.0 - 128.0) / river_bed_scale;
}

// 1 au cœur d'une tuile creusée, 0 hors des tuiles creusées.
float carve_weight(vec2 p) {
	float w = 0.0;
	for (int i = 0; i < 4; i++) {
		if (i >= carved_rect_count) {
			break;
		}
		vec4 r = carved_rects[i];
		float inside = min(min(p.x - r.x, r.z - p.x), min(p.y - r.y, r.w - p.y));
		w = max(w, clamp(inside / carve_edge_px, 0.0, 1.0));
	}
	return w;
}

// Profil du lit : 1 au fond, 0 au-delà de la berge.
float river_profile(float sd) {
	return 1.0 - smoothstep(-0.3, river_bank_px, sd);
}
