// Lot BV2 : gouttes de sang (mode 0, quads tournés vers la caméra) et morceaux tranchés
// (mode 1 : tête, avant-bras, jambe). Trajectoire balistique calculée ici depuis les données
// d'instance (`battle_gore.gd`) : origine = point au sol ; gouttes : base.y.y = hauteur de
// départ, base.z.z = taille (quads recalculés en POSITION) ; morceaux : base identité, couleur
// d'instance = (hauteur de départ / 4, teinte) ; INSTANCE_CUSTOM = (vitesse initiale, instant).
shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;

uniform int mode = 0;
uniform int piece = 0;
uniform float anim_time = 0.0;
uniform float lifetime = 1.4;
uniform float rest_seconds = 40.0;
uniform float gravity = 9.8;
// Gouttes posées au sol : restent quelques secondes en s'étalant.
uniform float ground_seconds = 5.0;

varying float v_fade;
varying float v_cut;
varying float v_tint;

void vertex() {
	vec3 origin = MODEL_MATRIX[3].xyz;
	float h0 = mode == 0 ? MODEL_MATRIX[1].y : COLOR.r * 4.0;
	vec3 vel = INSTANCE_CUSTOM.xyz;
	float t = anim_time - INSTANCE_CUSTOM.w;
	// Temps d'arrivée au sol : h0 + vy t - g t² / 2 = 0.
	float land = (vel.y + sqrt(vel.y * vel.y + 2.0 * gravity * max(h0, 0.0))) / gravity;
	float tt = clamp(t, 0.0, land);
	vec3 p = origin + vec3(vel.x * tt, h0 + vel.y * tt - 0.5 * gravity * tt * tt, vel.z * tt);
	v_fade = 1.0;
	v_cut = 0.0;
	v_tint = COLOR.g;
	if (mode == 0) {
		float size = MODEL_MATRIX[2].z;
		bool alive = t >= 0.0 && t < land + ground_seconds;
		v_fade = 1.0 - smoothstep(land + ground_seconds * 0.4, land + ground_seconds, t);
		// En vol : goutte étirée le long de la vitesse ; au sol : tache aplatie.
		vec3 view_pos = (VIEW_MATRIX * vec4(p + vec3(0.0, 0.02, 0.0), 1.0)).xyz;
		float grown = t > land ? mix(1.0, 3.0, smoothstep(land, land + 0.4, t)) : 1.0;
		vec2 corner = VERTEX.xy * size * grown;
		if (!alive) {
			corner = vec2(0.0);
			view_pos = vec3(0.0, 0.0, 10.0);
		}
		POSITION = PROJECTION_MATRIX * vec4(view_pos + vec3(corner, 0.0), 1.0);
	} else {
		float id = float(INSTANCE_ID);
		vec3 axis = normalize(vec3(fract(sin(id * 12.99) * 437.5), fract(sin(id * 78.23) * 853.1), fract(sin(id * 3.71) * 127.9)) - 0.5 + vec3(1e-3));
		float angle = tt * 9.0;
		// Rodrigues : rotation du sommet autour de l'axe tiré au sort.
		vec3 v = VERTEX;
		float c = cos(angle);
		float s = sin(angle);
		vec3 r = v * c + cross(axis, v) * s + axis * dot(axis, v) * (1.0 - c);
		vec3 n = NORMAL * c + cross(axis, NORMAL) * s + axis * dot(axis, NORMAL) * (1.0 - c);
		// Posé au sol (rayon ~0,1 m), puis s'enfonce à la fin.
		float sink = smoothstep(rest_seconds * 0.8, rest_seconds, t) * 0.4;
		vec3 world = p + r + vec3(0.0, 0.06 - sink, 0.0);
		if (t < 0.0 || t > rest_seconds) {
			world = vec3(0.0, -1000.0, 0.0);
		}
		// Coupure : extrémité -Y du morceau (cou, coude, genou).
		v_cut = 1.0 - smoothstep(-0.12, -0.02, VERTEX.y);
		VERTEX = world - origin;
		NORMAL = normalize(n);
	}
}

void fragment() {
	if (mode == 0) {
		vec2 d = UV - 0.5;
		float disc = 1.0 - smoothstep(0.3, 0.5, length(d));
		if (disc * v_fade < 0.3) {
			discard;
		}
		ALBEDO = vec3(0.16, 0.005, 0.004);
		ROUGHNESS = 0.25;
		SPECULAR = 0.6;
	} else {
		vec3 skin = mix(vec3(0.55, 0.38, 0.28), vec3(0.3, 0.19, 0.12), v_tint);
		vec3 steel = vec3(0.42, 0.42, 0.44);
		vec3 cloth = mix(vec3(0.3, 0.22, 0.12), vec3(0.16, 0.16, 0.2), v_tint);
		vec3 base = piece == 0 ? (v_tint > 0.55 ? steel : skin) : cloth;
		float metal = piece == 0 && v_tint > 0.55 ? 0.8 : 0.0;
		ALBEDO = mix(base, vec3(0.2, 0.01, 0.008), v_cut);
		METALLIC = metal * (1.0 - v_cut);
		ROUGHNESS = mix(0.6, 0.3, v_cut);
	}
}
