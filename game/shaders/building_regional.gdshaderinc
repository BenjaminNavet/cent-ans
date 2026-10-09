// TX T4 (ADR 0241) : matières régionales des bâtiments. Inclus par `building_atlas.gdshader` et
// `town_building.gdshader`. Un tableau de couches générées (`tx_building_albedo_array.jpg`, normales
// `tx_building_normal_array.jpg`) remplace, pour une région, la couche par défaut de l'atlas d'un
// rôle (Plaster, Masonry, RoofTile…) : `region_table[région * 16 + couche de l'atlas]` = couche du
// tableau régional, -1 = couche par défaut. `BuildingMaterials` remplit les tables depuis
// `data/art/building_regions.json` (`materials`) et `data/art/tx_building_pack.json`. Tout est
// inactif (`use_regional` faux) sans le paquet ou avec `--legacy-textures`.

uniform sampler2DArray regional_albedo : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2DArray regional_normal : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform bool use_regional = false;
uniform int region_table[640];
// Répétitions par mètre de chaque couche régionale (1 / taille de tuile).
uniform float regional_tile[96];
// Micro-détail de maçonnerie (TX) : grain fin fondu au-dessous de `micro_fade.y` mètres de la
// caméra (`micro_fade.x` : force, 0 = inactif).
uniform sampler2D micro_masonry : filter_linear_mipmap_anisotropic, repeat_enable, hint_default_white;
uniform vec3 micro_fade = vec3(0.0, 2.0, 12.0);  // force, distance pleine (m), distance nulle (m)

int regional_layer(int region, int layer) {
	if (!use_regional || region < 0 || region >= 40) {
		return -1;
	}
	return region_table[region * 16 + layer];
}

// Grain fin multiplicatif autour de 1 (0,5 = neutre dans la texture), fondu avec la distance.
// Le test de `micro_fade.x` est uniforme : la lecture reste dans un flux de contrôle uniforme.
float micro_grain(vec2 uv_m, float distance_m) {
	if (micro_fade.x <= 0.0) {
		return 1.0;
	}
	float g = texture(micro_masonry, uv_m * 0.9).r;
	float k = 1.0 - smoothstep(micro_fade.y, micro_fade.z, distance_m);
	return 1.0 + (g - 0.5) * 1.6 * micro_fade.x * k;
}
