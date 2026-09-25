// Lot PF1 : pluie et neige de la carte de campagne (CM2), en shader de particules maison au lieu
// d'un ParticleProcessMaterial : celui-ci partage ses shaders dans un cache statique du moteur,
// dont une entrée restait allouée à la fermeture du jeu exporté (« 1 shaders of type
// ParticlesShaderRD were never freed »). Un `Shader` ordinaire est libéré avec sa ressource.
// Émission dans une boîte centrée sur le nœud (coordonnées monde), vitesse initiale dans un cône
// autour de `direction`, sans gravité ; la neige ondule (`sway`).
shader_type particles;

uniform vec3 emission_box_extents = vec3(60.0, 10.0, 60.0);
uniform vec3 direction = vec3(0.12, -1.0, 0.05);
uniform float spread_deg = 4.0;
uniform float velocity_min = 60.0;
uniform float velocity_max = 75.0;
// Amplitude de l'ondulation horizontale (unités par seconde), 0 pour la pluie.
uniform float sway = 0.0;

float hash(uint x) {
	x = ((x >> 16u) ^ x) * 0x45d9f3bu;
	x = ((x >> 16u) ^ x) * 0x45d9f3bu;
	x = (x >> 16u) ^ x;
	return float(x) / 4294967295.0;
}

void start() {
	uint seed = RANDOM_SEED + uint(INDEX) * 7919u;
	if (RESTART_POSITION) {
		vec3 local = (vec3(hash(seed), hash(seed + 1u), hash(seed + 2u)) * 2.0 - 1.0) * emission_box_extents;
		TRANSFORM = mat4(vec4(1.0, 0.0, 0.0, 0.0), vec4(0.0, 1.0, 0.0, 0.0), vec4(0.0, 0.0, 1.0, 0.0),
			EMISSION_TRANSFORM * vec4(local, 1.0));
	}
	if (RESTART_VELOCITY) {
		vec3 dir = normalize(direction);
		float angle = radians(spread_deg) * sqrt(hash(seed + 3u));
		float turn = 6.2831853 * hash(seed + 4u);
		vec3 side = normalize(abs(dir.y) < 0.99 ? cross(dir, vec3(0.0, 1.0, 0.0)) : cross(dir, vec3(1.0, 0.0, 0.0)));
		vec3 up = cross(side, dir);
		vec3 v = dir * cos(angle) + (side * cos(turn) + up * sin(turn)) * sin(angle);
		VELOCITY = v * mix(velocity_min, velocity_max, hash(seed + 5u));
	}
}

void process() {
	if (sway > 0.0) {
		float phase = float(INDEX) * 1.618;
		VELOCITY.x += sin(TIME * 1.3 + phase) * sway * DELTA;
		VELOCITY.z += cos(TIME * 1.1 + phase * 0.7) * sway * DELTA;
	}
}
