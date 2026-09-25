// Trajectoire d'une flèche d'une volée (lot BV1). Reproduit à l'identique par
// `BattleVolleys.arrow_landing()` (GDScript) : mêmes hachages entiers, mêmes constantes.

// Vitesse (m/s), flèche relative de l'arc, étalement du tir (s) : flèche, carreau.
const float VOLLEY_SPEED[2] = float[](48.0, 62.0);
const float VOLLEY_ARC[2] = float[](0.16, 0.06);
const float VOLLEY_STAGGER[2] = float[](1.5, 0.8);

struct VolleyArrow {
	vec3 start;
	vec3 end_p;
	float launch;
	float flight;
	float arc;
	int count;
	bool bolt;
	bool fire;
	bool handed_off;
	float seed_f;
};

uint volley_pcg(uint v) {
	uint state = v * 747796405u + 2891336453u;
	uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
	return (word >> 22u) ^ word;
}

float volley_rand(uint seed_hash, int i, int k) {
	uint h = volley_pcg(seed_hash + uint(i) * 16u + uint(k));
	return float(h >> 8u) / 16777216.0;
}

VolleyArrow volley_arrow(mat4 m, vec4 c, int i) {
	VolleyArrow a;
	uint sh = volley_pcg(uint(c.x + 0.5));
	int code = int(c.y + 0.5);
	int kind = code & 1;
	a.bolt = kind == 1;
	int cover = (code >> 2) & 3;
	int stride = code >> 4;
	a.count = int(c.z + 0.5);
	a.seed_f = c.x;
	vec3 src = m[0].xyz;
	vec3 tgt = m[1].xyz;
	float shw = m[2].y;
	float shd = m[2].z;
	float thw = m[3].x;
	float thd = m[3].y;
	vec2 d2 = tgt.xz - src.xz;
	float len2 = max(length(d2), 1.0);
	vec2 dir = d2 / len2;
	vec2 perp = vec2(dir.y, -dir.x);
	vec2 s_off = perp * ((volley_rand(sh, i, 0) * 2.0 - 1.0) * shw) + dir * ((volley_rand(sh, i, 1) * 2.0 - 1.0) * shd);
	float across = (volley_rand(sh, i, 2) + volley_rand(sh, i, 3) - 1.0) * (thw + 2.0);
	float along = (volley_rand(sh, i, 4) + volley_rand(sh, i, 5) - 1.0) * (thd * 1.6 + 3.0);
	bool in_cover = cover > 0 && along < -0.35 * thd;
	if (in_cover) {
		along = -(thd + 0.9);
	}
	vec2 t_off = perp * across + dir * along;
	a.start = vec3(src.x + s_off.x, src.y + 1.5, src.z + s_off.y);
	a.end_p = vec3(tgt.x + t_off.x, tgt.y + c.w * t_off.x + m[3].z * t_off.y, tgt.z + t_off.y);
	if (in_cover) {
		a.end_p.y += 0.35 + volley_rand(sh, i, 8) * 0.8;
	}
	a.launch = m[2].x + volley_rand(sh, i, 6) * VOLLEY_STAGGER[kind];
	float dist = distance(a.start, a.end_p);
	a.flight = max(dist / VOLLEY_SPEED[kind] * (1.0 + VOLLEY_ARC[kind]), 0.05);
	a.arc = dist * VOLLEY_ARC[kind] * (0.85 + 0.3 * volley_rand(sh, i, 7));
	a.fire = (code & 2) != 0 && volley_rand(sh, i, 9) < 0.5;
	a.handed_off = stride > 0 && (i % max(stride, 1)) == 0;
	return a;
}

// Pointe et direction de vol à la fraction `u` du vol.
void arrow_state(VolleyArrow a, float u, out vec3 pos, out vec3 dir) {
	pos = mix(a.start, a.end_p, u) + vec3(0.0, a.arc * 4.0 * u * (1.0 - u), 0.0);
	dir = normalize(a.end_p - a.start + vec3(0.0, a.arc * 4.0 * (1.0 - 2.0 * u), 0.0));
}
