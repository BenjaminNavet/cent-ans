// Trajectoire des traits (lot B4), partagée par `battle_projectile.gdshader` (le trait) et
// `battle_projectile_trail.gdshader` (sa traînée). Chaque instance du MultiMesh code sa
// trajectoire dans sa transformée (MODEL_MATRIX, nœud à l'origine) : colonne 0 = départ,
// colonne 1 = arrivée, colonne 2 = (instant de tir, durée du vol, flèche de l'arc),
// origine = (durée où le trait reste fiché au sol, graine, 0).

uniform float time_now = 0.0;

struct Flight {
	vec3 pos;    // position de la pointe
	vec3 dir;    // direction du vol
	vec3 side;
	vec3 up;
	float scale; // 0 : invisible
	bool flying;
};

Flight projectile(mat4 m) {
	vec3 start = m[0].xyz;
	vec3 end_p = m[1].xyz;
	float launch = m[2].x;
	float flight = max(m[2].y, 0.05);
	float arc = m[2].z;
	float stick = m[3].x;
	float age = time_now - launch;
	bool flying = age >= 0.0 && age < flight;
	bool stuck = age >= flight && age < flight + stick;
	float u = clamp(age / flight, 0.0, 1.0);
	Flight f;
	f.pos = mix(start, end_p, u) + vec3(0.0, arc * 4.0 * u * (1.0 - u), 0.0);
	f.dir = normalize(end_p - start + vec3(0.0, arc * 4.0 * (1.0 - 2.0 * u), 0.0));
	f.side = normalize(cross(vec3(0.0, 1.0, 0.0), f.dir) + vec3(1e-4, 0.0, 0.0));
	f.up = cross(f.dir, f.side);
	f.scale = flying ? 1.0 : 0.0;
	f.flying = flying;
	if (stuck) {
		// Fiché au sol, pointe enfoncée ; s'efface à la fin.
		f.pos -= f.dir * 0.28;
		f.scale = 1.0 - smoothstep(flight + stick - 1.0, flight + stick, age);
	}
	return f;
}
