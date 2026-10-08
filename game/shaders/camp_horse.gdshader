// Lot AS1 : chevaux au piquet des camps de bataille (`BattleDecor._horse_line`, modèles
// `horse_0..2` du kit : tête basse en train de brouter, museau vers -X, queue vers +X, pieds à
// y = 0). Mouvement en déplacement de sommets, masques par position locale et par matière
// (`ch_hair` = 1 sur les crins : la queue est le crin qui pend vers +X). Réglages :
// `camp_horse` de `data/fx/animal_motion.json`, posés par `AnimalMotion.camp_horse_material`.
// Respiration, mâchonnement, coups de tête, queue qui chasse les mouches, poids qui passe d'un
// sabot arrière à l'autre.
shader_type spatial;
render_mode diffuse_burley;

uniform vec4 albedo : source_color = vec4(1.0);
uniform float rough = 0.85;
uniform float ch_hair = 0.0;
uniform float ch_on = 1.0;
// (pivot x, pivot y, début du cou x, fin de la tête x)
uniform vec4 ch_head = vec4(-0.75, 1.3, -0.75, -1.15);
// (tête relevée, durée du cycle, part du cycle tête haute, hochement)
uniform vec4 ch_head2 = vec4(0.4, 15.0, 0.22, 0.04);
// (fréquence du hochement, fréquence de mâchonnement, amplitude de mâchonnement)
uniform vec4 ch_head3 = vec4(0.7, 2.2, 0.025, 0.0);
// (début x, fin x, attache y, longueur)
uniform vec4 ch_tail = vec4(0.6, 0.9, 1.45, 0.75);
// (battement, fréquence, période des chasses-mouches)
uniform vec4 ch_tail2 = vec4(0.1, 0.9, 20.0, 0.0);
// (période, levée du sabot, recul du sabot, déhanchement)
uniform vec4 ch_shift = vec4(22.0, 0.045, 0.04, 0.018);
// (souffle vertical, souffle latéral, fréquence)
uniform vec4 ch_breath = vec4(0.008, 0.012, 0.3, 0.0);

float ch_hash(vec2 p) {
	return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

void vertex() {
	if (ch_on > 0.5) {
		float seed = ch_hash(MODEL_MATRIX[3].xz * 0.37);
		float t = TIME + seed * 97.0;
		vec3 v = VERTEX;
		float side = v.z >= 0.0 ? 1.0 : -1.0;

		// Respiration : flancs et dos.
		float barrel = (1.0 - smoothstep(0.5, 0.8, abs(v.x))) * smoothstep(0.6, 0.95, v.y);
		float breath = sin(t * TAU * ch_breath.z);
		v.z *= 1.0 + ch_breath.y * breath * barrel;
		v.y += ch_breath.x * breath * smoothstep(0.8, 1.5, v.y) * (1.0 - smoothstep(0.7, 1.1, abs(v.x)));

		// Poids qui passe d'un sabot arrière à l'autre.
		float sv = sin(t * TAU / ch_shift.x);
		float cock = side > 0.0 ? smoothstep(0.55, 0.95, sv) : smoothstep(0.55, 0.95, -sv);
		float leg_w = clamp((0.8 - v.y) / 0.8, 0.0, 1.0) * (1.0 - ch_hair) * smoothstep(0.35, 0.5, v.x);
		v.y += ch_shift.y * cock * leg_w;
		v.x -= ch_shift.z * cock * leg_w;
		float lean = smoothstep(0.55, 0.95, sv) - smoothstep(0.55, 0.95, -sv);
		v.z -= ch_shift.w * lean * smoothstep(0.3, 1.2, v.y);

		// Tête : relevée de temps en temps, mâchonne quand elle est basse, léger hochement.
		float hw = clamp((ch_head.z - v.x) / (ch_head.z - ch_head.w), 0.0, 1.0);
		hw = hw * hw * (3.0 - 2.0 * hw);
		if (hw > 0.0) {
			float u = fract(t / ch_head2.y);
			float up = smoothstep(0.0, 0.05, u) * (1.0 - smoothstep(ch_head2.z - 0.05, ch_head2.z, u));
			float angle = -ch_head2.x * up;
			angle += sin(t * TAU * ch_head3.y) * ch_head3.z * (1.0 - up);
			angle += sin(t * TAU * ch_head3.x) * ch_head2.w;
			angle *= hw;
			vec2 d = v.xy - ch_head.xy;
			float c = cos(angle);
			float s = sin(angle);
			v.xy = ch_head.xy + vec2(d.x * c - d.y * s, d.y * c + d.x * s);
		}

		// Queue : chasse les mouches par salves.
		float tw = ch_hair * smoothstep(ch_tail.x, ch_tail.y, v.x) * clamp((ch_tail.z - v.y) / ch_tail.w, 0.0, 1.0);
		if (tw > 0.0) {
			float swish = 0.3 + 0.7 * pow(max(0.0, sin(t * TAU / ch_tail2.z)), 3.0);
			v.z += tw * ch_tail2.x * swish * sin(t * TAU * ch_tail2.y);
		}
		VERTEX = v;
	}
}

void fragment() {
	ALBEDO = albedo.rgb;
	ROUGHNESS = rough;
}
