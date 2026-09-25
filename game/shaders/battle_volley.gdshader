// Volées massives (lot BV1) : chaque instance du MultiMesh est un « paquet » de 256 flèches
// (ou carreaux) d'une même volée ; le maillage contient les 256 flèches, chacune repérée par son
// numéro (UV2.x). Tout le vol est calculé ici à partir de l'instant de tir : le processeur n'écrit
// que 16 flottants par paquet, jamais une flèche.
//
// Données d'instance (cf. `BattleVolleys._write_chunk`, qui reproduit exactement ce calcul pour
// les flèches confiées à la couche des flèches plantées) :
//   MODEL_MATRIX[0].xyz = centre du régiment tireur (au sol)
//   MODEL_MATRIX[1].xyz = centre de la cible (au sol)
//   MODEL_MATRIX[2].xyz = (instant de tir, demi-front du tireur, demi-profondeur du tireur)
//   MODEL_MATRIX[3].xyz = (demi-front de la cible, demi-profondeur de la cible, pente z du sol)
//   INSTANCE_CUSTOM     = (graine, code, nombre de flèches du paquet, pente x du sol)
//   code = sorte (0 flèche, 1 carreau) + 2 × feu + 4 × couvert (0, 1 pavois, 2 pieux)
//          + 16 × pas des flèches confiées à la couche plantée.
shader_type spatial;
render_mode skip_vertex_transform, cull_disabled, specular_disabled, shadows_disabled;

#include "res://shaders/battle_volley.gdshaderinc"

uniform float time_now = 0.0;
// Durée (s) où une flèche non confiée à la couche plantée reste fichée avant de s'effacer.
uniform float stick_hold = 8.0;
// Durée (s) de la flamme d'une flèche enflammée une fois fichée.
uniform float burn_after = 5.0;

varying float v_part;
varying vec2 v_quad;
varying float v_flicker;

void vertex() {
	int i = int(UV2.x + 0.5);
	VolleyArrow a = volley_arrow(MODEL_MATRIX, INSTANCE_CUSTOM, i);
	float age = time_now - a.launch;
	bool flying = age >= 0.0 && age < a.flight;
	bool stuck = age >= a.flight && age < a.flight + stick_hold && !a.handed_off;
	vec3 pos;
	vec3 dir;
	arrow_state(a, clamp(age / a.flight, 0.0, 1.0), pos, dir);
	float scale = (i < a.count && (flying || stuck)) ? 1.0 : 0.0;
	if (stuck) {
		pos -= dir * 0.28;
		scale *= 1.0 - smoothstep(a.flight + stick_hold - 1.5, a.flight + stick_hold, age);
	}
	vec3 cam = CAMERA_POSITION_WORLD;
	float d = distance(cam, pos);
	vec3 world;
	v_part = UV.x;
	v_quad = VERTEX.xy;
	v_flicker = fract(float(i) * 0.618 + a.seed_f * 0.013);
	if (UV.x > 0.5) {
		// Flamme d'une flèche enflammée : panneau face caméra à la pointe.
		bool burning = a.fire && age >= 0.0 && age < a.flight + burn_after;
		float s = (burning && i < a.count) ? 0.2 * clamp(d / 40.0, 1.0, 4.0) : 0.0;
		vec3 right = INV_VIEW_MATRIX[0].xyz;
		vec3 up = INV_VIEW_MATRIX[1].xyz;
		world = pos + (right * VERTEX.x + up * (VERTEX.y + 0.35) * 1.5) * s;
	} else {
		// Ruban tourné vers la caméra autour de l'axe du trait : épaissi de loin (lisible comme un
		// trait d'un pixel), allongé en vol (flou de bougé : la pluie de flèches se lit en traînées).
		vec3 to_cam = normalize(cam - pos);
		vec3 across = normalize(cross(dir, to_cam) + vec3(0.0, 1e-4, 0.0));
		// Fichées : comme la couche plantée (fines, invisibles au-delà de 240 m).
		float widen = (flying ? clamp(d / 30.0, 1.0, 6.0) : clamp(d / 60.0, 1.0, 1.5)) * (a.bolt ? 1.6 : 1.0);
		if (stuck && d > 240.0) {
			scale = 0.0;
		}
		float stretch = (a.bolt ? 0.5 : 1.0) * (flying ? 1.0 + clamp(d / 70.0, 0.0, 2.5) : 1.0);
		world = pos + across * VERTEX.x * widen * scale + dir * VERTEX.z * stretch * scale;
	}
	VERTEX = (VIEW_MATRIX * vec4(world, 1.0)).xyz;
	NORMAL = normalize((VIEW_MATRIX * vec4(normalize(cam - pos), 0.0)).xyz);
}

void fragment() {
	if (v_part > 0.5) {
		// Goutte de flamme : large en bas, effilée en haut, qui vacille.
		vec2 q = v_quad;
		float flick = 0.85 + 0.15 * sin(time_now * 23.0 + v_flicker * 40.0);
		float w = mix(0.55, 0.08, clamp(q.y + 0.5, 0.0, 1.0)) * flick;
		float shape = 1.0 - smoothstep(w * 0.7, w, abs(q.x));
		shape *= step(-0.5, q.y) * step(q.y, 0.5 * flick);
		vec3 hot = mix(vec3(1.0, 0.22, 0.02), vec3(1.0, 0.62, 0.12), clamp(0.4 - q.y - abs(q.x) * 1.5, 0.0, 1.0));
		ALBEDO = hot * 0.3;
		EMISSION = hot * 1.3;
		ALPHA = shape;
		ALPHA_SCISSOR_THRESHOLD = 0.5;
	} else {
		ALBEDO = COLOR.rgb;
		ROUGHNESS = 0.85;
		ALPHA = 1.0;
		ALPHA_SCISSOR_THRESHOLD = 0.5;
	}
}
