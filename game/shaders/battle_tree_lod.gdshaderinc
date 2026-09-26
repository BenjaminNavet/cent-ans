// DA6 : niveaux de détail des arbres par instance. La distance est prise à la caméra principale
// (`MAIN_CAM_INV_VIEW_MATRIX`, aussi dans la passe d'ombre) depuis l'origine de l'instance :
// chaque arbre passe d'un niveau à l'autre à sa propre distance (les tuiles de MultiMesh ne
// servent qu'au tri grossier), avec un fondu tramé complémentaire sur `lod_band` mètres.

uniform float lod_near = 0.0; // visible au-delà (0 : dès la caméra)
uniform float lod_far = 100000.0; // visible en deçà
uniform float lod_band = 16.0;

// x = entrée (fondu depuis `lod_near`), y = sortie (fondu vers `lod_far`).
varying vec2 v_lod;

// Fondus d'entrée et de sortie de l'instance ; hors plage si l'un est nul (sommets à écraser).
vec2 tree_lod(vec3 origin, vec3 camera) {
	float d = length(origin - camera);
	float fin = lod_near <= 0.0 ? 1.0 : smoothstep(lod_near - lod_band * 0.5, lod_near + lod_band * 0.5, d);
	float fout = 1.0 - smoothstep(lod_far - lod_band * 0.5, lod_far + lod_band * 0.5, d);
	return vec2(fin, fout);
}

// Tramage de Bayer 4 × 4 : le niveau qui entre occupe les pixels que celui qui sort libère.
bool tree_lod_discard(vec2 frag, vec2 lod) {
	int x = int(mod(frag.x, 4.0));
	int y = int(mod(frag.y, 4.0));
	const float bayer[16] = float[](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
	float t = (bayer[y * 4 + x] + 0.5) / 16.0;
	return t < 1.0 - lod.x || t >= lod.y;
}
