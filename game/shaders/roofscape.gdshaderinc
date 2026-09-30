// Lot SZ4 (extrait au lot VT-C, ADR 0138) : teinte « masse de toits » partagée entre les villes
// 1:1 (`town_building.gdshader`, sol bâti vu de loin) et le maillage lointain des villes
// (`town_far.gdshader`). Couleur moyenne (dernier niveau de mip) d'une couche de toit de l'atlas,
// tirée par cellule de `roofscape_cell_m`, bord sombre de chaque cellule (ruelles et cours).
// Le shader qui inclut ce fichier déclare avant l'inclusion : `albedo_array` (sampler2DArray),
// `layer_tint[16]`, `roof_first`, `roof_last`, `snow`.

uniform float roofscape = 0.0;
uniform float roofscape_near = 1.2;
uniform float roofscape_far = 3.5;
uniform float roofscape_cell_m = 9.0;
uniform float roofscape_gain = 1.0;

float roof_hash(vec2 p) {
	p = fract(p * vec2(127.1, 311.7));
	p += dot(p, p + 19.19);
	return fract(p.x * p.y);
}

// Teinte de toit (et sa rugosité) de la cellule contenant le point `world_m` (mètres monde).
vec3 roofscape_color(vec3 world_m, out float roof_roughness) {
	float h = roof_hash(floor(world_m.xz / roofscape_cell_m));
	int roof = roof_first + int(floor(h * float(roof_last - roof_first + 1)));
	roof = clamp(roof, roof_first, roof_last);
	vec3 mean = textureLod(albedo_array, vec3(0.5, 0.5, float(roof)), 12.0).rgb * layer_tint[roof].rgb;
	// Ruelles et cours entre les toits : bord sombre de chaque cellule.
	vec2 f = abs(fract(world_m.xz / roofscape_cell_m) - 0.5) * 2.0;
	float gap = smoothstep(0.72, 0.95, max(f.x, f.y));
	mean *= roofscape_gain * mix(0.8, 1.15, fract(h * 7.31)) * mix(1.0, 0.55, gap);
	if (snow > 0.0) {
		mean = mix(mean, vec3(0.9, 0.92, 0.97), snow);
	}
	roof_roughness = layer_tint[roof].a;
	return mean;
}

// Teinte moyenne de toutes les couches de toit (sans variation par cellule) : repli quand une
// cellule devient plus petite qu'un pixel (le tirage par cellule scintillerait).
vec3 roofscape_average(out float roof_roughness) {
	vec3 sum = vec3(0.0);
	float rough = 0.0;
	int n = 0;
	for (int roof = roof_first; roof <= roof_last; roof++) {
		sum += textureLod(albedo_array, vec3(0.5, 0.5, float(roof)), 12.0).rgb * layer_tint[roof].rgb;
		rough += layer_tint[roof].a;
		n++;
	}
	float k = 1.0 / float(max(n, 1));
	vec3 mean = sum * k * roofscape_gain * 0.95;
	if (snow > 0.0) {
		mean = mix(mean, vec3(0.9, 0.92, 0.97), snow);
	}
	roof_roughness = rough * k;
	return mean;
}
