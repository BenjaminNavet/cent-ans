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

// DA6 : déformation des lisières (m) — les bords de parcelles ne sont plus rectilignes : ondulation
// de ±5 m sur ~28 m et frange de ±1,2 m sur ~5 m. Même formule au sol et dans l'herbe.
vec2 bt_edge_warp(vec2 p) {
	vec2 w = vec2(bt_value_noise(p * 0.035 + 11.3), bt_value_noise(p * 0.035 + 47.9)) - 0.5;
	vec2 f = vec2(bt_value_noise(p * 0.21 + 5.1), bt_value_noise(p * 0.21 + 91.7)) - 0.5;
	return w * 10.0 + f * 2.4;
}

// Parcelle cultivée au point `p` (m) : x = nature (0 aucune, 1 labour, 2 blé, 3 jachère),
// y = coordonnée le long des sillons, z = distance normalisée au bord (0 bord, 1 centre).
vec3 bt_field_info_ex(vec2 p, float ramp, float soft) {
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
	// Des massifs entiers sans parcelles (prés, landes) ; DA6 : fondu sur le bord du massif.
	float massif = bt_value_noise(p * 0.0012 + 3.1);
	float z = smoothstep(0.0, ramp, edge);
	if (soft > 0.5) {
		z *= smoothstep(0.37, 0.41, massif);
		if (massif < 0.37) {
			kind = 0.0;
		}
	} else if (massif < 0.38) {
		kind = 0.0;
	}
	return vec3(kind, q.y, z);
}

vec3 bt_field_info(vec2 p) {
	return bt_field_info_ex(p, 4.0, 0.0);
}

// DA6 : parcelle à lisière douce (bords déformés par `bt_edge_warp`, rampe de 9 m).
vec3 bt_field_info_soft(vec2 p) {
	vec3 f = bt_field_info_ex(p + bt_edge_warp(p), 9.0, 1.0);
	// Sillons droits (coordonnée non déformée) : seules les lisières ondulent.
	f.y = sin(0.31) * p.x + cos(0.31) * p.y;
	return f;
}

// DA6 : lisière d'une parcelle du décor (texel `df` lu au point déformé) : rampe à bord bruité.
float bt_decor_edge_soft(float z, vec2 p) {
	return clamp(z + (bt_value_noise(p * 0.3 + 3.7) - 0.5) * 0.35, 0.0, 1.0);
}
