// Fonctions communes aux shaders de bataille (lot V4) : hachage, bruit, parcelles cultivées.
// Les parcelles sont purement décoratives et calculées ici pour que le sol et l'herbe
// (qui s'en écarte) soient d'accord sans texture supplémentaire.

float bt_hash21(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

vec2 bt_hash22(vec2 p) {
	float n = bt_hash21(p);
	return vec2(n, bt_hash21(p + n * 17.0));
}

float bt_value_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	float a = bt_hash21(i);
	float b = bt_hash21(i + vec2(1.0, 0.0));
	float c = bt_hash21(i + vec2(0.0, 1.0));
	float d = bt_hash21(i + vec2(1.0, 1.0));
	return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// EP6 : parcelle du décor du cœur lue dans `decor_fields` (texel `df`) : x = nature (1 labour,
// 2 blé, 3 pré, 4 semis, 5 vigne, 6 chaume), y = coordonnée à travers les sillons (qui suivent la
// longueur de la parcelle), z = distance normalisée au bord.
vec3 bt_decor_field(vec4 df, vec2 p) {
	float kind = floor(df.r * 8.0 + 0.5);
	float yaw = df.g * 3.14159265;
	return vec3(kind, dot(p, vec2(-sin(yaw), cos(yaw))), df.b);
}

// Parcelle cultivée au point `p` (m) : x = nature (0 aucune, 1 labour, 2 blé, 3 jachère),
// y = coordonnée le long des sillons, z = distance normalisée au bord (0 bord, 1 centre).
vec3 bt_field_info(vec2 p) {
	const float ANGLE = 0.31;
	mat2 rot = mat2(vec2(cos(ANGLE), sin(ANGLE)), vec2(-sin(ANGLE), cos(ANGLE)));
	vec2 q = rot * p;
	// Bandes de largeur variable (lanières médiévales) découpées en parcelles.
	vec2 size = vec2(130.0, 46.0);
	vec2 cell = floor(q / size);
	float h = bt_hash21(cell + 7.0);
	vec2 local = fract(q / size);
	float edge = min(min(local.x, 1.0 - local.x) * size.x, min(local.y, 1.0 - local.y) * size.y);
	float kind = 0.0;
	if (h < 0.22) {
		kind = 1.0;
	} else if (h < 0.40) {
		kind = 2.0;
	} else if (h < 0.52) {
		kind = 3.0;
	}
	// Des massifs entiers sans parcelles (prés, landes).
	if (bt_value_noise(p * 0.0012 + 3.1) < 0.38) {
		kind = 0.0;
	}
	return vec3(kind, q.y, smoothstep(0.0, 4.0, edge));
}
