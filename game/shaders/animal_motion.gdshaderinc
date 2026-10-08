// Lot AS1 : mouvement procédural des bêtes de la carte (bœuf, vache, mouton, cheval, et la bête
// attelée d'une charrette), en déplacement de sommets. Aucun os, aucune donnée cuite : les
// masques viennent de la position locale (modèles FK2 : avance +Z, haut +Y, côtés ±X, pieds à
// y = 0) et des mesures de `data/fx/animal_motion.json`, posées en uniformes par `AnimalMotion`.
//
// - Pas : quatre temps (arrière gauche, avant gauche, arrière droite, avant droite), cadence =
//   vitesse réelle de l'instance / longueur de foulée ; rebond du corps à deux fois la cadence.
//   Lot AS8c : le débattement et la levée du sabot suivent des courbes en table (16 échantillons)
//   mesurées sur la vidéo d'un troupeau (data/fx/animal_motion_measured.json), plus un sinus.
// - Hochement de tête au pas ; à l'arrêt : tête basse qui broute (mâchonnement) puis relevée,
//   cycle décalé par instance.
// - Queue qui chasse les mouches (plus vive au pas), souffle des flancs, déhanchement lent.
// - Roues : rotation à la vitesse de l'instance ; cahot de la caisse = trois raies mesurées
//   (périodes proportionnelles à la foulée de la bête attelée) et roulis de même provenance.

uniform float am_on = 0.0;
// (centre de la bête sur l'avance, demi-longueur, hauteur du ventre, hauteur du garrot)
uniform vec4 am_body = vec4(0.0, 0.8, 0.77, 1.4);
// (début du cou, longueur du cou, hauteur du pivot, longueur de la queue)
uniform vec4 am_neck = vec4(0.64, 0.4, 1.19, 0.08);
// (foulée, débattement, levée, rebond)
uniform vec4 am_gait = vec4(1.5, 0.2, 0.07, 0.012);
// (tête basse, hochement, durée du cycle, part du cycle tête basse)
uniform vec4 am_head = vec4(0.8, 0.05, 12.0, 0.7);
// (battement de queue, fréquence, gain au pas, déhanchement)
uniform vec4 am_tail = vec4(0.08, 0.5, 2.0, 0.012);
// (souffle, fréquence du souffle, fréquence de mâchonnement, amplitude de mâchonnement)
uniform vec4 am_breath = vec4(0.008, 0.28, 2.4, 0.035);
// (rayon des roues, écart latéral minimal, gain du cahot (1 = mesuré), foulée de référence du cahot)
uniform vec4 am_cart = vec4(0.0, 0.65, 1.0, 1.26);
// Lot AS8c : (durée de la descente de la tête, roulis de la caisse en rad, 0, 0)
uniform vec4 am_extra = vec4(1.0, 0.0093, 0.0, 0.0);
// Cahot : amplitudes (m) et périodes en fraction de la foulée des trois raies mesurées.
uniform vec3 am_jolt_amp = vec3(0.0127, 0.0096, 0.0066);
uniform vec3 am_jolt_ratio = vec3(1.35, 0.96, 0.75);
// Courbes de pas mesurées, 16 échantillons par cycle, phase 0 = mi-balancé montant :
// déplacement avant (amplitude 1) et levée du sabot (0 à 1).
uniform float am_swing[16];
uniform float am_lift[16];

float am_hash(vec2 p) {
	return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

// Table de pas à la phase `u` (cycles), interpolée linéairement et bouclée.
float am_swing_at(float u) {
	float x = fract(u) * 16.0;
	int i = int(floor(x));
	return mix(am_swing[i], am_swing[(i + 1) % 16], x - float(i));
}

float am_lift_at(float u) {
	float x = fract(u) * 16.0;
	int i = int(floor(x));
	return mix(am_lift[i], am_lift[(i + 1) % 16], x - float(i));
}

// Rotation de pas (a > 0 : l'avant descend) autour de l'axe X au point (y0, z0).
vec2 am_pitch(vec2 yz, vec2 pivot, float a) {
	vec2 d = yz - pivot;
	float c = cos(a);
	float s = sin(a);
	return pivot + vec2(d.x * c - d.y * s, d.y * c + d.x * s);
}

// `t` : heure d'animation, `speed` : vitesse de l'instance (m/s, 0 = immobile), `seed` : [0, 1[
// propre à l'instance. Modifie `v` (sommet local, avant le déplacement de trajet).
void animal_pose(inout vec3 v, float t, float speed, float seed) {
	float moving = smoothstep(0.05, 0.3, speed);
	float turn = TAU * (t * speed / max(am_gait.x, 0.05) + seed);

	// Roues (charrettes) : roulent à la vitesse d'avance.
	bool is_wheel = am_cart.x > 0.0 && abs(v.x) > am_cart.y && length(vec2(v.y - am_cart.x, v.z)) < am_cart.x + 0.03;
	if (is_wheel) {
		float a = t * speed / am_cart.x;
		vec2 r = am_pitch(v.yz, vec2(am_cart.x, 0.0), a);
		v.yz = r;
	}
	// Cahot et roulis de la caisse (hors roues et hors bête attelée) : trois raies mesurées dont la
	// période est un multiple de la foulée de la bête attelée (`am_cart.w`).
	if (speed > 0.0 && !is_wheel) {
		float dist = t * speed;
		float jolt = 0.0;
		for (int k = 0; k < 3; k++) {
			jolt += am_jolt_amp[k] * sin(TAU * (dist / (am_jolt_ratio[k] * am_cart.w) + seed * (1.0 + 0.37 * float(k))));
		}
		v.y += am_cart.z * moving * jolt;
		if (am_cart.x > 0.0 && abs(v.z - am_body.x) > am_body.y + 0.2) {
			float roll = am_extra.y * moving * sin(TAU * (dist / (am_jolt_ratio.x * am_cart.w) + seed * 1.7));
			v.xy = am_pitch(v.xy, vec2(0.0, am_cart.x), roll);
		}
	}

	if (am_on < 0.5) {
		return;
	}
	float zr = v.z - am_body.x;
	float half_len = am_body.y;
	if (zr < -half_len - am_neck.w - 0.05 || zr > half_len + 1.4) {
		return;
	}
	float belly = am_body.z;
	float withers = am_body.w;
	float side = v.x >= 0.0 ? 1.0 : -1.0;
	float leg_w = clamp((belly - v.y) / belly, 0.0, 1.0);
	float body_w = 1.0 - leg_w;
	float cyc_t = t + seed * 40.0;

	// Pattes : quatre temps, débattement croissant vers le sabot.
	if (abs(zr) < half_len * 0.95) {
		float front = zr >= 0.0 ? 0.25 : 0.0;
		float leg_u = turn / TAU + (side > 0.0 ? 0.0 : 0.5) + front;
		v.z += am_swing_at(leg_u) * am_gait.y * leg_w * moving;
		v.y += am_lift_at(leg_u) * am_gait.z * leg_w * leg_w * moving;
	}
	// Rebond du corps et déhanchement (à l'arrêt, le poids passe d'un côté à l'autre).
	v.y += am_gait.w * sin(2.0 * turn + 1.0) * body_w * moving;
	v.x += am_tail.w * sin(cyc_t * 0.7) * smoothstep(0.0, withers, v.y) * (1.0 - moving);
	// Souffle : flancs et dos.
	v.y += am_breath.x * sin(cyc_t * TAU * am_breath.y) * smoothstep(belly, withers, v.y);

	// Tête : broute (tête basse, mâchonne) puis relève ; au pas, hochement.
	float head_w = smoothstep(am_neck.x, am_neck.x + am_neck.y, zr) * smoothstep(belly * 0.75, belly * 1.1, v.y);
	if (head_w > 0.0) {
		float u = fract(cyc_t / am_head.z);
		// Descente et remontée de la tête : durée mesurée (`am_extra.x`, secondes).
		float ramp = clamp(am_extra.x / max(am_head.z, 1.0), 0.01, 0.3);
		float graze = smoothstep(0.0, ramp, u) * (1.0 - smoothstep(am_head.w - ramp, am_head.w, u));
		graze *= 1.0 - moving;
		float chew = sin(cyc_t * TAU * am_breath.z) * am_breath.w * graze;
		float nod = sin(turn) * am_head.y * moving + sin(cyc_t * 0.9) * am_head.y * 0.4 * (1.0 - moving);
		float angle = (graze * am_head.x + chew + nod) * head_w;
		v.yz = am_pitch(v.yz, vec2(am_neck.z, am_body.x + am_neck.x), angle);
	}
	// Queue : chasse les mouches.
	float tail_w = clamp((-zr - half_len) / am_neck.w, 0.0, 1.0) * step(abs(v.x), 0.1) * step(belly * 0.6, v.y);
	if (tail_w > 0.0) {
		float swish = 0.5 + 0.5 * pow(max(0.0, sin(cyc_t * 0.37)), 3.0);
		float rate = am_tail.y * (1.0 + (am_tail.z - 1.0) * moving);
		v.x += tail_w * am_tail.x * swish * sin(cyc_t * TAU * rate);
	}
}
