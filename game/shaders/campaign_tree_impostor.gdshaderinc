// Corps des imposteurs d'arbres de la carte (lots FC2, FC6) : panneau face à la caméra, vue cuite
// choisie par azimut local, tramage de Bayer entre deux vues, découpe alpha, hiver ajouré. Inclus
// par `campaign_tree_impostor.gdshader` (tuiles lointaines) et, via `foliage.gdshaderinc`
// (FOLIAGE_CARDS), par `foliage_cards.gdshader` : les arbres proches y portent aussi un
// quadrilatère d'imposteur, pour un fondu tramé cartes ↔ imposteur sans appel de dessin de plus.
// À inclure après `foliage_common.gdshaderinc`.

uniform sampler2D albedo_atlas : source_color, filter_linear_mipmap, repeat_disable;
// RVB : normale dans le repère de la vue cuite (x droite, y haut, z vers la caméra), A : occlusion.
uniform sampler2D normal_atlas : hint_default_white, filter_linear_mipmap, repeat_disable;
uniform int views = 8;
uniform int rows = 3;
// Cadrage de la cuisson (constantes ORTHO et FOOT du générateur) : côté d'une cellule pour un
// arbre de hauteur 1, et hauteur du pied dans la cellule (fraction depuis le bas).
uniform float ortho = 1.7;
uniform float foot = 0.2588;
uniform float alpha_cut = 0.5;
uniform float max_lod = 2.5;

struct ImpostorVertex {
	vec3 world;
	vec2 uv0;
	vec2 uv1;
	float blend;
	vec3 right;
	vec3 up;
	vec3 face;
	vec3 tint;
	vec2 local;
};

// Sommet monde d'un coin (`corner`, UV du quadrilatère, y = 0 en haut) pour une instance.
ImpostorVertex impostor_vertex(mat4 model, vec3 cam, vec2 corner, float essence, float seed, vec3 custom_tint) {
	ImpostorVertex o;
	vec3 origin = model[3].xyz;
	float keep = foliage_keep(origin, seed);
	vec3 tone = vec3(1.0);
	vec3 variety = foliage_variety(origin, seed, tone);
	vec3 bx = model[0].xyz;
	vec3 by = model[1].xyz;
	vec3 bz = model[2].xyz;
	float width = length(bx);
	float height = length(by);
	vec3 to_cam = cam - origin;
	// Azimut de la caméra dans le repère de l'arbre (Blender (x, y) = Godot (x, −z)).
	float ax = dot(to_cam, bx / max(width, 1e-6));
	float az = dot(to_cam, bz / max(length(bz), 1e-6));
	float f = fract(atan(-az, ax) / TAU + 1.0) * float(views);
	int k0 = int(floor(f)) % views;
	int k1 = (k0 + 1) % views;
	o.blend = fract(f);
	// Panneau face à la caméra, incliné vers elle ; pied à l'origine de l'instance.
	vec3 face = normalize(to_cam + vec3(0.0, 1e-4, 0.0));
	vec3 right = normalize(cross(vec3(0.0, 1.0, 0.0), face) + vec3(1e-5, 0.0, 0.0));
	vec3 up = cross(face, right);
	float s = keep * campaign_prop_scale * ortho;
	vec2 local = vec2(corner.x - 0.5, 1.0 - corner.y);
	o.world = origin + right * local.x * width * variety.x * s + up * (local.y - foot) * height * variety.y * s;
	// Lot SZ4b : pied enfoncé comme le feuillage maillé (`FOLIAGE_GROUND_SINK`).
	o.world += by * FOLIAGE_GROUND_SINK * (1.0 - campaign_prop_scale);
	float row = clamp(floor(essence + 0.5), 0.0, float(rows - 1));
	vec2 cell = vec2(float(views), float(rows));
	o.uv0 = (vec2(float(k0), row) + corner) / cell;
	o.uv1 = (vec2(float(k1), row) + corner) / cell;
	o.right = right;
	o.up = up;
	o.face = face;
	o.tint = custom_tint * tone;
	o.local = local;
	return o;
}

// Seuil de Bayer 4 × 4 (0..1) du pixel.
float bayer4(vec2 frag) {
	int x = int(mod(frag.x, 4.0));
	int y = int(mod(frag.y, 4.0));
	const float bayer[16] = float[](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
	return (bayer[y * 4 + x] + 0.5) / 16.0;
}

struct ImpostorFragment {
	vec3 albedo;
	vec3 world_normal;
	float alpha;
	float ao;
	bool cut;  // hiver : pixel ajouré (discard chez l'appelant)
};

ImpostorFragment impostor_fragment(vec2 frag, vec2 uv0, vec2 uv1, float blend, vec3 right, vec3 up, vec3 face, vec3 tint, vec2 local, float seed, float essence, float world_y) {
	ImpostorFragment o;
	vec2 uv = blend > bayer4(frag) ? uv1 : uv0;
	// Niveau de mipmap borné : très loin (arbres de quelques pixels), l'alpha moyen de la cellule
	// entière, même compensé, remplirait tout le quadrilatère (carrés pleins).
	float lod = clamp(textureQueryLod(albedo_atlas, uv).x, 0.0, max_lod);
	vec4 t = textureLod(albedo_atlas, uv, lod);
	vec4 nt = textureLod(normal_atlas, uv, lod);
	float leaves = value_noise(vec3(local * 24.0, seed * 17.0));
	bool broadleaf = essence < 1.5;
	// Hiver : feuillus ajourés (on voit le sol à travers), comme `foliage_winter.gdshader`.
	o.cut = broadleaf && campaign_season.w > 0.5 && value_noise(vec3(local * 40.0, seed * 29.0)) < 0.45;
	vec3 n = nt.rgb * 2.0 - 1.0;
	o.world_normal = normalize(right * n.x + up * n.y + face * max(n.z, 0.05));
	vec3 albedo = t.rgb * tint * mix(0.85, 1.12, leaves);
	o.albedo = foliage_season(albedo, essence, seed, leaves, o.world_normal.y, world_y);
	// Mipmaps : l'alpha moyen baisse avec le niveau, compensé pour garder la silhouette.
	o.alpha = t.a * (1.0 + lod * 0.35);
	o.ao = mix(0.55, 1.0, nt.a);
	return o;
}
