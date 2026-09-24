// Figurines des batailles (lots V4, B1, B4) : animation par shader de sommets, compatible
// MultiMesh. Chaque sommet porte son membre et le pivot de ce membre (CUSTOM0, cf.
// `battle_meshes.gd`) ; le shader fait tourner jambes, bras, arme, encolure et jambes du cheval
// selon l'état du régiment (uniforme `anim_state`) et une phase propre à chaque soldat
// (hachage de INSTANCE_ID). Les cadavres (MultiMesh à données perso) lisent leur instant de
// mort dans INSTANCE_CUSTOM et tombent au sol. Matières selon COLOR.a : livrée, bordure,
// métal, tissu varié, couleur exacte, armoiries (texture du blason de la faction).
// Lot B1 : CUSTOM0.w = poids de flexion du genou (homme) ou du genou/jarret (cheval), dont le
// pivot (y, z) est dans UV2 ; allures du cheval au pas (quatre temps) et au galop.
// Lot B4 : bras à trois articulations (coude : poids CUSTOM0.w et pivot UV2 ; épaule en flexion
// et en abduction), torsion du buste ; archer qui bande (corde à la main droite, flèche encochée,
// décoche synchronisée sur la volée du régiment, `volley_time`), arbalétrier qui vise et
// réarme au pied de biche ; cavalier : jambes liées à la selle, rênes à la main gauche, buste
// penché et lance couchée à la charge ; caparaçon soulevé au galop ; mêlée : coups tirés au
// sort à chaque passe, gardes, reculs, choc de la charge (`state_time`) ; chutes variées.
shader_type spatial;
render_mode cull_back, diffuse_burley, specular_schlick_ggx;

const int S_IDLE = 0;
const int S_WALK = 1;
const int S_RUN = 2;
const int S_MELEE = 3;
const int S_SHOOT = 4;
const int S_CLIMB = 5;
const int S_ROUT = 6;

const int W_SWORD = 0;
const int W_POLE = 1;
const int W_BOW = 2;
const int W_CROSSBOW = 3;
const int W_LANCE = 4;
const int W_ENGINE = 5;

// Membres (cf. `battle_meshes.gd`).
const int P_ARM_R = 3;
const int P_ARM_L = 4;
const int P_HEAD = 5;
const int P_RIDER = 12;
const int P_WEAPON = 13;
const int P_BOW = 17;
const int P_RIDER_LEG = 18;
const int P_REIN = 19;
const int P_ARROW = 20;

uniform vec3 livery : source_color = vec3(0.2, 0.3, 0.75);
uniform vec3 trim : source_color = vec3(0.85, 0.7, 0.25);
uniform sampler2D heraldry : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_heraldry = false;
uniform float anim_time = 0.0;
uniform int anim_state = 0;
uniform int weapon_mode = 0;
uniform bool mounted = false;
uniform bool corpse = false;
uniform vec2 hip = vec2(0.93, 0.0);       // pivot (y, z) du buste
uniform vec2 shoulder = vec2(1.4, 0.0);   // épaules (hiérarchie de l'arme)
uniform vec2 elbow = vec2(1.13, -0.02);   // coudes (lot B4)
uniform float highlight = 0.0;            // régiment sélectionné : léger éclaircissement
// A1-01 : lisibilité à distance (0 de près, 1 au loin ; fixé par `battle_soldiers.gd` selon la
// distance caméra → régiment). Teinte de camp renforcée, liseré lumineux (rim) de la livrée,
// figurines un peu grossies, sélection soulignée d'or : comme les unités « colorées » de TW.
uniform float far_blend = 0.0;
uniform float far_scale = 0.18;
uniform vec3 select_glow : source_color = vec3(1.0, 0.85, 0.35);
// Part des soldats portant le surcot de livrée (lot V4b) ; les autres portent un vêtement de
// teinte naturelle (écru, brun, roux, gris…) marqué d'une croix de livrée sur la poitrine.
uniform float livery_share = 0.55;
uniform float torso_y = 0.0;              // décalage du buste (cavaliers : assise sur la selle)
uniform float torso_z = 0.0;
// Lot B4 : instant (anim_time) de la dernière volée du régiment et durée de rechargement ;
// temps écoulé depuis l'entrée dans l'état courant (choc de la charge au contact).
uniform float volley_time = -1000.0;
uniform float reload_time = 6.0;
uniform float state_time = 100.0;

varying vec3 v_color;
varying float v_code;
varying vec3 v_var;
varying float v_height;
varying vec3 v_local;    // position au repos (avant animation), pour la croix de poitrine
varying vec3 v_local_n;

const float SHOULDER_X = 0.19;

float hash1(float n) {
	return fract(sin(n * 12.9898 + 4.1414) * 43758.5453);
}

vec3 rot_x(vec3 v, float a, vec2 pivot) {
	vec2 p = v.yz - pivot;
	float c = cos(a);
	float s = sin(a);
	return vec3(v.x, p.x * c - p.y * s + pivot.x, p.x * s + p.y * c + pivot.y);
}

vec3 rot_x_dir(vec3 n, float a) {
	float c = cos(a);
	float s = sin(a);
	return vec3(n.x, n.y * c - n.z * s, n.y * s + n.z * c);
}

vec3 rot_z(vec3 v, float a, vec2 pivot_xy) {
	vec2 p = v.xy - pivot_xy;
	float c = cos(a);
	float s = sin(a);
	return vec3(p.x * c - p.y * s + pivot_xy.x, p.x * s + p.y * c + pivot_xy.y, v.z);
}

vec3 rot_z_dir(vec3 n, float a) {
	float c = cos(a);
	float s = sin(a);
	return vec3(n.x * c - n.y * s, n.x * s + n.y * c, n.z);
}

// Rotation autour de l'axe vertical passant par (x, z) = pivot_xz.
vec3 rot_y(vec3 v, float a, vec2 pivot_xz) {
	vec2 p = v.xz - pivot_xz;
	float c = cos(a);
	float s = sin(a);
	return vec3(p.x * c + p.y * s + pivot_xz.x, v.y, -p.x * s + p.y * c + pivot_xz.y);
}

vec3 rot_y_dir(vec3 n, float a) {
	float c = cos(a);
	float s = sin(a);
	return vec3(n.x * c + n.z * s, n.y, -n.x * s + n.z * c);
}

// Bras (lot B4) : angles (flexion à l'épaule, abduction, flexion du coude) ; `side` = -1 pour
// le bras droit (côté -X), +1 pour le gauche. Le coude plie d'abord (poids `w`), puis l'épaule.
vec3 arm_point(vec3 p, float side, vec3 a, float w) {
	p = rot_x(p, a.z * w, elbow);
	p = rot_x(p, a.x, shoulder);
	return rot_z(p, a.y * side, vec2(SHOULDER_X * side, shoulder.x));
}

vec3 arm_dir(vec3 n, float side, vec3 a, float w) {
	n = rot_x_dir(n, a.z * w);
	n = rot_x_dir(n, a.x);
	return rot_z_dir(n, a.y * side);
}

// Buste : torsion autour de la colonne puis inclinaison aux hanches.
vec3 body_point(vec3 p, float twist, float lean) {
	p = rot_y(p, twist, vec2(0.0, hip.y));
	return rot_x(p, lean, hip);
}

vec3 body_dir(vec3 n, float twist, float lean) {
	return rot_x_dir(rot_y_dir(n, twist), lean);
}

// Pose d'arc (lot B4) : torsion, inclinaison, bras gauche, bras droit (cf. `tools/` : angles
// trouvés par optimisation des positions des mains, `docs/wip/b4-effets-animations.md`).
struct Pose {
	float twist;
	float lean;
	vec3 l;
	vec3 r;
};

Pose pose_mix(Pose a, Pose b, float k) {
	return Pose(mix(a.twist, b.twist, k), mix(a.lean, b.lean, k), mix(a.l, b.l, k), mix(a.r, b.r, k));
}

// Repère d'un objet tenu en visée (arc, arbalète) : tangage puis roulis, en local.
vec3 aim_rot(vec3 p, float pitch, float roll) {
	p = rot_z(p, roll, vec2(0.0));
	return rot_x(p, pitch, vec2(0.0));
}

vec3 aim_rot_dir(vec3 n, float pitch, float roll) {
	return rot_x_dir(rot_z_dir(n, roll), pitch);
}

void vertex() {
	float id = float(INSTANCE_ID);
	float h1 = hash1(id * 0.731 + 1.3);
	float h2 = hash1(id * 1.917 + 7.1);
	float h3 = hash1(id * 3.113 + 2.9);
	int part = int(CUSTOM0.x + 0.5);
	vec2 pivot = CUSTOM0.yz;
	vec3 v = VERTEX;
	vec3 n = NORMAL;
	v_local = VERTEX;
	v_local_n = NORMAL;
	float code = COLOR.a * 5.0;

	int state = corpse ? -1 : anim_state;
	float t = anim_time;
	float ph = h1 * 6.2832;

	// Angles des membres (radians ; négatif = vers l'avant pour un membre pendant).
	float leg = 0.0;
	vec3 arm_r = vec3(0.0);   // (flexion, abduction, coude)
	vec3 arm_l = vec3(0.0);
	float weapon_total = 0.0; // orientation absolue souhaitée de l'arme (0 = droite)
	bool weapon_abs = true;
	float bow = 0.15;          // arc : orientation absolue
	float lean = 0.0;
	float twist = 0.0;
	float head_turn = 0.0;
	float bob = 0.0;
	float recoil = 0.0;        // recul du soldat (m, vers l'arrière)
	float engine = 0.0;
	// Tir (lot B4) : arc ou arbalète tenus en visée, corde tirée, flèche encochée.
	float aim = 0.0;
	float aim_pitch = 0.0;
	float aim_roll = 0.0;
	float draw = 0.0;
	float nock = 0.0;
	// Cheval.
	float hl_fl = 0.0;
	float hl_fr = 0.0;
	float hl_bl = 0.0;
	float hl_br = 0.0;
	float neck = 0.0;
	float horse_pitch = 0.0;
	float horse_bob = 0.0;
	float gallop = 0.0;
	// Flexion des genoux (homme) et des genoux/jarrets (cheval), appliquée avant la hanche.
	float knee_l = 0.03;
	float knee_r = 0.03;
	float hk_fl = 0.0;
	float hk_fr = 0.0;
	float hk_bl = 0.0;
	float hk_br = 0.0;

	if (weapon_mode == W_SWORD) weapon_total = 0.35;
	if (weapon_mode == W_CROSSBOW) weapon_total = 0.0;

	if (state == S_IDLE) {
		float br = sin(t * 1.6 + ph);
		bob = br * 0.006;
		lean = sin(t * 0.37 + ph) * 0.025;
		arm_r = vec3(-0.08 + br * 0.03, 0.05, -0.1);
		arm_l = vec3(-0.05 - br * 0.02, 0.05, -0.1);
		neck = sin(t * 0.5 + ph) * 0.12 - 0.1;
		head_turn = sin(t * 0.23 + ph * 2.0) * 0.25;
		if (weapon_mode == W_BOW) {
			bow = 0.1;
		}
		if (mounted) {
			arm_l = vec3(-0.3, 0.0, -0.6); // rênes
		}
		// Cheval au repos : une jambe arrière soulagée, pince au sol.
		if (h3 < 0.5) { hk_br = -0.35; hl_br = 0.08; } else { hk_bl = -0.35; hl_bl = 0.08; }
	} else if (state == S_WALK || state == S_CLIMB) {
		float c = t * 6.2832 * (0.95 + h2 * 0.1) + ph;
		leg = sin(c) * 0.42;
		arm_l = vec3(-sin(c) * 0.22 - 0.05, 0.05, -0.15);
		arm_r = vec3(sin(c) * 0.18 - 0.1, 0.05, -0.15);
		bob = abs(cos(c)) * 0.035;
		lean = 0.04;
		if (weapon_mode == W_POLE) weapon_total = 0.08 + sin(c) * 0.03;
		knee_l = max(-cos(c), 0.0) * 0.75 + 0.05;
		knee_r = max(cos(c), 0.0) * 0.75 + 0.05;
		// Pas du cheval, quatre temps : postérieur gauche, antérieur gauche, postérieur droit,
		// antérieur droit ; le genou (jarret) plie quand la jambe se lève et avance.
		float g = t * 6.2832 * 0.9 + ph;
		float p_bl = g;
		float p_fl = g - 1.571;
		float p_br = g - 3.1416;
		float p_fr = g - 4.712;
		hl_bl = sin(p_bl) * 0.3;
		hl_fl = sin(p_fl) * 0.32;
		hl_br = sin(p_br) * 0.3;
		hl_fr = sin(p_fr) * 0.32;
		hk_fl = max(cos(p_fl), 0.0) * 0.75;
		hk_fr = max(cos(p_fr), 0.0) * 0.75;
		hk_bl = -max(cos(p_bl), 0.0) * 0.45;
		hk_br = -max(cos(p_br), 0.0) * 0.45;
		neck = sin(g * 2.0) * 0.08;
		horse_bob = abs(sin(g)) * 0.03;
		gallop = 0.25;
		if (mounted) {
			arm_l = vec3(-0.35, 0.0, -0.6 + sin(g * 2.0) * 0.05);
			arm_r = vec3(-0.15, 0.05, -0.5);
		}
		if (state == S_CLIMB) {
			arm_r = vec3(-2.6 + sin(c) * 0.35, 0.1, -0.3);
			arm_l = vec3(-2.6 - sin(c) * 0.35, 0.1, -0.3);
			lean = -0.1;
		}
	} else if (state == S_RUN || state == S_ROUT) {
		float c = t * 6.2832 * (1.45 + h2 * 0.15) + ph;
		leg = sin(c) * 0.75;
		arm_l = vec3(-sin(c) * 0.5 - 0.2, 0.1, -0.9);
		arm_r = vec3(sin(c) * 0.45 - 0.2, 0.1, -0.9);
		bob = abs(cos(c)) * 0.07;
		lean = state == S_ROUT ? 0.3 : 0.2;
		if (state == S_RUN) {
			if (weapon_mode == W_POLE) {
				weapon_total = 1.2;
				arm_r = vec3(-0.45 + sin(c) * 0.08, 0.1, -0.6);
			}
			if (weapon_mode == W_SWORD) {
				arm_r = vec3(-1.3 + sin(c) * 0.15, 0.2, -0.9);
			}
		} else {
			weapon_total = 0.9; // arme portée n'importe comment dans la déroute
			head_turn = sin(t * 1.3 + ph) * 0.6; // regards en arrière
		}
		// Galop : jambes par paires décalées, tangage et rebond du corps.
		knee_l = max(-cos(c), 0.0) * 1.4 + 0.15;
		knee_r = max(cos(c), 0.0) * 1.4 + 0.15;
		float g = t * 6.2832 * (1.9 + h2 * 0.2) + ph;
		hl_fl = sin(g) * 0.7;
		hl_fr = sin(g + 0.5) * 0.7;
		hl_bl = sin(g + 3.1) * 0.65;
		hl_br = sin(g + 3.6) * 0.65;
		// Au galop les antérieurs se replient très haut sous le poitrail.
		hk_fl = max(cos(g), 0.0) * 1.5;
		hk_fr = max(cos(g + 0.5), 0.0) * 1.5;
		hk_bl = -max(cos(g + 3.1), 0.0) * 0.7;
		hk_br = -max(cos(g + 3.6), 0.0) * 0.7;
		horse_pitch = sin(g + 1.2) * 0.07;
		horse_bob = (sin(g + 1.6) * 0.5 + 0.5) * 0.14;
		neck = sin(g + 1.2) * 0.15;
		gallop = 1.0;
		if (mounted) {
			// Charge : buste penché sur l'encolure, lance couchée sous le bras, rênes courtes.
			lean = state == S_ROUT ? 0.15 : 0.32;
			arm_l = vec3(-0.55, 0.0, -0.9 + sin(g) * 0.08);
			arm_r = vec3(-0.25, 0.15, -1.25);
			if (weapon_mode == W_LANCE && state == S_RUN) {
				weapon_total = 1.5;
			}
		}
	} else if (state == S_MELEE) {
		float speed = 0.85 + h2 * 0.35;
		float cyc = t * speed + h1 * 7.0;
		float s = fract(cyc);
		float k = floor(cyc);
		// Chaque passe tire au sort son coup (et parfois une garde) : aucun soldat ne frappe en
		// cadence avec ses voisins.
		float r = hash1(id * 0.37 + k * 1.91);
		float hit = hash1(id * 1.13 + k * 2.7);
		leg = 0.22 + sin(t * 1.3 + ph) * 0.05;
		lean = 0.1 + sin(t * speed * 6.2832 + ph) * 0.05;
		arm_l = vec3(-0.85 + sin(t * 2.0 + ph) * 0.1, 0.25, -1.0);
		if (weapon_mode == W_SWORD) {
			weapon_abs = false;
			weapon_total = 0.25;
			if (r < 0.2) {
				// Garde : écu levé, lame prête, un pas en arrière.
				arm_r = vec3(-0.9, 0.3, -1.1);
				arm_l = vec3(-1.05, 0.35, -1.4);
				lean = -0.02;
				recoil = smoothstep(0.0, 0.3, s) * (1.0 - smoothstep(0.7, 1.0, s)) * 0.12;
			} else if (r < 0.55) {
				// Taille verticale : armer au-dessus de l'épaule, frapper vite, revenir.
				float up = smoothstep(0.0, 0.55, s);
				float down = smoothstep(0.55, 0.72, s);
				arm_r = mix(mix(vec3(-0.3, 0.15, -0.6), vec3(-2.7, 0.3, -1.3), up), vec3(-0.5, 0.1, -0.1), down);
				lean += down * (1.0 - smoothstep(0.72, 1.0, s)) * 0.12;
			} else if (r < 0.8) {
				// Coup de revers : bras écarté, balayage horizontal, rotation du buste.
				float sw = smoothstep(0.45, 0.68, s);
				float back = smoothstep(0.75, 1.0, s);
				arm_r = mix(mix(vec3(-1.2, 1.4, -1.4), vec3(-1.35, -0.3, -0.2), sw), vec3(-0.5, 0.2, -0.6), back);
				twist = mix(mix(0.25, 0.1, smoothstep(0.0, 0.45, s)), -0.4, sw) * (1.0 - back);
				weapon_total = 1.3;
			} else {
				// Estoc : lame couchée, bras qui se détend.
				float th = smoothstep(0.5, 0.65, s) * (1.0 - smoothstep(0.75, 1.0, s));
				arm_r = mix(vec3(-1.0, 0.25, -1.7), vec3(-1.5, 0.05, 0.1), th);
				weapon_total = 1.45;
				lean += th * 0.15;
			}
		} else if (weapon_mode == W_POLE) {
			// Piques et lances abaissées : estocades hautes ou basses, parfois une pause.
			float amp = r < 0.2 ? 0.0 : 1.0;
			float thrust = pow(max(sin(s * 6.2832), 0.0), 3.0) * amp;
			float high = hash1(id * 2.3 + k) * 0.25 - 0.1;
			arm_r = vec3(-0.5 - thrust * 0.45 - high, 0.1, -0.8 + thrust * 0.5);
			arm_l = vec3(-0.7 - thrust * 0.3 - high, 0.2, -0.9 + thrust * 0.3);
			weapon_total = 1.5 - thrust * 0.05 + high * 0.3;
			lean = 0.12 + thrust * 0.12;
		} else if (weapon_mode == W_LANCE) {
			arm_r = vec3(-0.35 - pow(max(sin(s * 6.2832), 0.0), 3.0) * 0.5, 0.15, -0.9);
			arm_l = vec3(-0.5, 0.0, -0.9);
			weapon_total = 1.35;
		} else {
			// Archers et arbalétriers au corps à corps (dague, arme courte).
			arm_r = s < 0.5 ? vec3(-1.9, 0.3, -1.2) : vec3(-0.5, 0.1, -0.3);
			arm_l = vec3(-0.9, 0.2, -1.0);
			weapon_abs = false;
			weapon_total = 0.3;
		}
		// Coup reçu : recul d'un pas, buste rejeté en arrière.
		if (hit < 0.3) {
			float pulse = smoothstep(0.62, 0.7, s) * (1.0 - smoothstep(0.72, 0.95, s));
			recoil += pulse * 0.2;
			lean -= pulse * 0.25;
		}
		// Chevaux piétinant.
		knee_l = 0.3;
		knee_r = 0.15;
		float g = t * 6.2832 * 1.1 + ph;
		hl_fl = max(sin(g), 0.0) * 0.6;
		hl_fr = max(sin(g + 2.0), 0.0) * 0.5;
		hk_fl = max(sin(g), 0.0) * 1.2;
		hk_fr = max(sin(g + 2.0), 0.0) * 1.0;
		hl_bl = sin(g + 1.0) * 0.1;
		hl_br = sin(g + 3.0) * 0.1;
		horse_pitch = -max(sin(g), 0.0) * 0.08;
		neck = sin(g * 1.3) * 0.2 - 0.1;
		gallop = 0.3;
		// Choc du contact (charge) : les premiers rangs encaissent et reculent, les chevaux se
		// cabrent un peu ; décroît en une seconde environ.
		float shock = exp(-state_time * 2.5) * mix(0.5, 1.2, h3);
		recoil += shock * 0.4;
		lean -= shock * 0.3;
		horse_pitch -= shock * 0.18;
	} else if (state == S_SHOOT) {
		neck = sin(t * 0.5 + ph) * 0.1 - 0.1;
		// Décoche calée sur la volée du régiment (±0,4 s selon le soldat), à s = 0,55.
		float jitter = (h1 - 0.5) * 0.8;
		float s = fract((t - volley_time - jitter) / max(reload_time, 1.0) + 0.55);
		if (weapon_mode == W_BOW) {
			// Encocher (arc bas), bander en levant l'arc, viser, décocher (la main droite part
			// en arrière), puis chercher une flèche au carquois.
			Pose nock_p = Pose(-0.5, 0.0, vec3(-0.3, 0.1, -0.65), vec3(-2.3, 2.35, -0.15));
			Pose draw_p = Pose(-0.9, -0.05, vec3(-0.9, 2.1, 0.25), vec3(-1.5, 1.25, -1.6));
			Pose loose_p = Pose(-0.9, -0.05, vec3(-0.85, 2.1, 0.15), vec3(-1.2, 0.7, -1.8));
			Pose quiver_p = Pose(-0.3, 0.0, vec3(-0.45, -0.05, -0.6), vec3(1.45, -0.1, -1.85));
			Pose p;
			if (s < 0.12) {
				p = pose_mix(quiver_p, nock_p, smoothstep(0.0, 0.12, s));
			} else if (s < 0.55) {
				float k = smoothstep(0.12, 0.3, s);
				p = pose_mix(nock_p, draw_p, k);
				draw = k;
				// Tremblement de l'effort, bras tendu.
				p.l.x += sin(t * 17.0 + ph) * 0.006 * k;
			} else if (s < 0.72) {
				p = pose_mix(draw_p, loose_p, smoothstep(0.55, 0.6, s));
			} else {
				p = pose_mix(loose_p, quiver_p, smoothstep(0.72, 0.88, s));
			}
			twist = p.twist;
			lean = p.lean;
			arm_l = p.l;
			arm_r = p.r;
			nock = s < 0.55 ? 1.0 : 0.0;
			aim = 1.0;
			aim_pitch = -mix(0.05, 0.3, smoothstep(0.1, 0.3, s)) - h2 * 0.1; // tir en cloche
			aim_roll = 0.12;
			head_turn = -twist * 0.85;
			leg = 0.2;
			knee_l = 0.12;
			knee_r = 0.12;
		} else if (weapon_mode == W_CROSSBOW) {
			// Viser (arbalète à l'épaule), tirer, abaisser le nez au sol, bander à deux mains
			// en se redressant, relever.
			Pose aim_p = Pose(-0.35, -0.02, vec3(-1.2, 2.15, -0.6), vec3(-1.1, 1.7, -1.7));
			Pose low_p = Pose(-0.1, 0.55, vec3(-0.85, -0.4, 0.45), vec3(-0.9, -0.6, 0.65));
			Pose high_p = Pose(-0.1, 0.25, vec3(-0.05, -0.5, -0.8), vec3(-0.15, -0.6, -0.55));
			Pose p = aim_p;
			float down = smoothstep(0.62, 0.7, s);
			if (s >= 0.62 && s < 0.92) {
				float pull = smoothstep(0.7, 0.88, s);
				p = pose_mix(pose_mix(aim_p, low_p, down), high_p, pull);
				aim_pitch = mix(-0.04, 1.4, down) - pull * 0.1;
			} else if (s >= 0.92) {
				p = pose_mix(high_p, aim_p, smoothstep(0.92, 1.0, s));
				aim_pitch = mix(1.3, -0.04, smoothstep(0.92, 1.0, s));
			} else {
				aim_pitch = -0.04;
				// Recul du tir.
				aim_pitch -= smoothstep(0.55, 0.57, s) * (1.0 - smoothstep(0.57, 0.64, s)) * 0.15;
			}
			twist = p.twist;
			lean = p.lean;
			arm_l = p.l;
			arm_r = p.r;
			aim = 1.0;
			head_turn = -twist * 0.8;
			leg = 0.15;
			knee_l = 0.1 + down * (1.0 - smoothstep(0.7, 0.88, s)) * 0.5;
			knee_r = knee_l;
		} else if (weapon_mode == W_ENGINE) {
			// Verge de l'engin : lente remontée, décoche brusque.
			float se = fract(t * (0.22 + h2 * 0.06) + h1);
			engine = se < 0.8 ? mix(0.0, -0.9, smoothstep(0.0, 0.8, se)) : mix(-0.9, 1.3, smoothstep(0.8, 0.86, se));
		} else {
			arm_r = vec3(-0.3, 0.1, -0.4);
		}
	} else if (corpse) {
		// Membres figés dans une pose aléatoire.
		leg = (h2 - 0.5) * 0.8;
		arm_r = vec3(-h3 * 2.2, h1 * 0.8, -h2 * 1.2);
		arm_l = vec3(-(1.0 - h3) * 1.6, h2 * 0.9, -h1 * 1.0);
		weapon_total = 1.4;
		hl_fl = (h2 - 0.5) * 1.2;
		hl_fr = (h3 - 0.5) * 1.2;
		hl_bl = (h1 - 0.5) * 0.8;
		hl_br = (h2 - 0.3) * 0.8;
		neck = 0.9;
		knee_l = h3 * 1.2;
		knee_r = h1 * 0.8;
		hk_fl = h3 * 1.4;
		hk_fr = h2 * 0.9;
		hk_bl = -h1 * 0.6;
		hk_br = -h3 * 0.5;
	}
	// Le cavalier compense en partie le tangage du cheval (buste stable).
	if (mounted) {
		lean -= horse_pitch * 0.7;
	}

	// 0. Flexion du genou, du jarret ou du coude (poids CUSTOM0.w, pivot UV2).
	float bend_w = CUSTOM0.w;
	if (bend_w > 0.0) {
		float k = 0.0;
		if (part == 1) k = knee_l;
		else if (part == 2) k = knee_r;
		else if (part == 7) k = hk_fl;
		else if (part == 8) k = hk_fr;
		else if (part == 9) k = hk_bl;
		else if (part == 10) k = hk_br;
		k *= bend_w;
		v = rot_x(v, k, UV2);
		n = rot_x_dir(n, k);
	}

	// 1. Membres.
	bool upper = part == 0 || part == P_ARM_R || part == P_ARM_L || part == P_HEAD || part == P_RIDER || part == P_WEAPON || part == P_BOW || part == P_ARROW;
	bool placed = false; // position finale déjà calculée (objets tenus en visée, rênes)
	if (part == 1) { v = rot_x(v, leg, pivot); n = rot_x_dir(n, leg); }
	else if (part == 2) { v = rot_x(v, -leg, pivot); n = rot_x_dir(n, -leg); }
	else if (part == P_ARM_R) { v = arm_point(v, -1.0, arm_r, bend_w); n = arm_dir(n, -1.0, arm_r, bend_w); }
	else if (part == P_ARM_L) { v = arm_point(v, 1.0, arm_l, bend_w); n = arm_dir(n, 1.0, arm_l, bend_w); }
	else if (part == P_HEAD) { v = rot_y(v, head_turn, vec2(0.0, hip.y)); n = rot_y_dir(n, head_turn); v = rot_x(v, lean * -0.4, vec2(1.5, 0.0)); }
	else if (part == 7) { v = rot_x(v, -hl_fl, pivot); n = rot_x_dir(n, -hl_fl); }
	else if (part == 8) { v = rot_x(v, -hl_fr, pivot); n = rot_x_dir(n, -hl_fr); }
	else if (part == 9) { v = rot_x(v, -hl_bl, pivot); n = rot_x_dir(n, -hl_bl); }
	else if (part == 10) { v = rot_x(v, -hl_br, pivot); n = rot_x_dir(n, -hl_br); }
	else if (part == 11) { v = rot_x(v, neck, pivot); n = rot_x_dir(n, neck); }
	else if (part == 15) { v = rot_x(v, engine, pivot); n = rot_x_dir(n, engine); }
	else if (part == 16) {
		// Jupe du surcot : suit un peu les jambes.
		float k = clamp((hip.x - v.y) / 0.4, 0.0, 1.0);
		v.z += sin(leg * 1.5) * 0.04 * k - lean * 0.1 * k;
	}
	else if (part == P_WEAPON || part == P_BOW || part == P_ARROW) {
		// Objet tenu : rotation propre (orientation absolue), coude, épaule.
		bool right = part == P_WEAPON;
		vec3 a = right ? arm_r : arm_l;
		float side = right ? -1.0 : 1.0;
		vec3 chain_v = v;
		vec3 chain_n = n;
		float w = right ? (weapon_abs ? weapon_total - a.x - a.z : weapon_total) : bow - a.x - a.z;
		if (right && weapon_mode == W_LANCE) {
			// Lot B7 : fanion de lance (en arrière de la hampe, près de la pointe). Lance droite,
			// il flotte à l'horizontale ; lance couchée, le vent de la course le rabat le long de
			// la hampe vers le cavalier (au lieu de se dresser à la verticale) ; il ondule.
			float d = v.z - pivot.y;
			float up = v.y - pivot.x;
			if (d < -0.035 && up > 2.7) {
				float len = -d;
				float fold = weapon_total * 0.8 + gallop * 0.12 * sin(t * 17.0 - len * 14.0 + ph);
				chain_v.y = v.y - len * sin(fold);
				chain_v.z = pivot.y - len * cos(fold);
				chain_v.x += len * (0.05 + gallop * 0.08) * sin(t * 11.0 - len * 22.0 + ph);
				chain_n = rot_x_dir(n, -fold);
			}
		}
		chain_v = rot_x(chain_v, w, pivot);
		chain_n = rot_x_dir(chain_n, w);
		chain_v = body_point(arm_point(chain_v, side, a, 1.0), twist, lean);
		chain_n = body_dir(arm_dir(chain_n, side, a, 1.0), twist, lean);
		if (aim > 0.0 && (weapon_mode == W_BOW || weapon_mode == W_CROSSBOW)) {
			// En visée : l'objet est posé dans la main (poing calculé par la chaîne du bras),
			// dans un repère fixe (tangage du tir, léger roulis de l'arc).
			vec3 grip = vec3(0.25 * side, pivot);
			vec3 fist = body_point(arm_point(grip, side, a, 1.0), twist, lean);
			vec3 local = v - grip;
			if (weapon_mode == W_BOW && !right) {
				local.z -= 0.16; // la poignée (pas la corde) dans le poing
			}
			vec3 aimed = fist + aim_rot(local, aim_pitch, weapon_mode == W_BOW ? aim_roll : 0.0);
			vec3 aimed_n = aim_rot_dir(n, aim_pitch, weapon_mode == W_BOW ? aim_roll : 0.0);
			if (weapon_mode == W_BOW && !right) {
				// Corde et flèche : le point d'encoche suit la main droite.
				vec3 hand_r = body_point(arm_point(vec3(-0.25, 0.9 + (shoulder.x - 1.4), 0.12 + (shoulder.y)), -1.0, arm_r, 1.0), twist, lean);
				vec3 nock_w = fist + aim_rot(vec3(0.0, 0.0, -0.16), aim_pitch, aim_roll);
				vec3 pull = hand_r - nock_w;
				if (part == P_BOW) {
					aimed += pull * bend_w * draw;
				} else {
					aimed += pull * draw;
					if (nock < 0.5) aimed = fist;
				}
			}
			v = mix(chain_v, aimed, aim);
			n = mix(chain_n, aimed_n, aim);
		} else {
			v = chain_v;
			n = chain_n;
			if (part == P_ARROW) v = body_point(arm_point(vec3(0.25, pivot), 1.0, arm_l, 1.0), twist, lean);
		}
		placed = true;
	}
	else if (part == P_REIN) {
		// Rênes : l'avant suit l'encolure, l'arrière le poing gauche (poids CUSTOM0.w).
		vec3 front = rot_x(v, neck, pivot);
		vec3 fist0 = vec3(0.25, UV2);
		vec3 fist = body_point(arm_point(fist0, 1.0, arm_l, 1.0), twist, lean);
		v = mix(front, v + (fist - fist0), bend_w);
		placed = true;
	}
	else if (part == 6 && code > 4.5 && v.y < 1.25) {
		// Caparaçon : l'ourlet se soulève et ondule au galop, l'avant s'écarte des antérieurs.
		float k = clamp((1.2 - v.y) / 0.46, 0.0, 1.0);
		float front = smoothstep(0.25, 0.75, v.z);
		float swing = max(max(hl_fl, hl_fr), 0.0);
		float fold = max(hk_fl, hk_fr);
		v.y += k * gallop * (0.1 + 0.05 * sin(v.z * 7.0 - t * 14.0 + ph));
		v.x *= 1.0 + k * gallop * 0.12;
		v.z += k * front * (swing * 0.45 + gallop * 0.06);
		v.y += k * front * fold * 0.1;
	}

	// 2. Buste (tout sauf jambes, cheval, jupe) : torsion, inclinaison aux hanches.
	if (upper && !placed && part != P_ARM_R && part != P_ARM_L) {
		v = body_point(v, part == P_HEAD ? twist * 0.15 : twist, lean);
		n = body_dir(n, part == P_HEAD ? twist * 0.15 : twist, lean);
	} else if (part == P_ARM_R || part == P_ARM_L) {
		v = body_point(v, twist, lean);
		n = body_dir(n, twist, lean);
	}
	if (part != 1 && part != 2 && part != 14 && part != 15 && part != P_RIDER_LEG) {
		v.y += bob;
	}
	// Recul (coups reçus, choc de la charge) : tout le soldat, cheval compris.
	if (weapon_mode != W_ENGINE) {
		v.z -= recoil;
	}
	// 3. Cheval : tangage et rebond de tout ce qui est porté.
	if (mounted && part != 7 && part != 8 && part != 9 && part != 10) {
		v = rot_x(v, horse_pitch, vec2(1.3, 0.0));
		n = rot_x_dir(n, horse_pitch);
		v.y += horse_bob;
	}
	if (mounted && part >= 7 && part <= 10) {
		v.y += horse_bob * 0.3;
	}

	// 4. Variation de taille (±7 %) et de carrure (±5 %), servants et engins exceptés.
	float scale = mix(0.93, 1.07, h2);
	float girth = mix(0.95, 1.06, hash1(id * 5.37 + 0.7));
	if (weapon_mode == W_ENGINE) {
		scale = 1.0;
		girth = 1.0;
	}
	v *= scale * (1.0 + far_scale * far_blend);
	v.x *= girth;

	// 5. Cadavres : chute variée après l'instant de mort (INSTANCE_CUSTOM.x) — en arrière, en
	// avant, sur le côté, ou affaissement à genoux avant de basculer ; durée propre au soldat.
	if (corpse) {
		float since = anim_time - INSTANCE_CUSTOM.x;
		float dur = mix(0.5, 1.1, h3);
		float dir = INSTANCE_CUSTOM.z >= 0.0 ? 1.0 : -1.0;
		if (mounted) {
			float fall = smoothstep(0.0, dur + 0.3, since);
			float a = fall * mix(1.3, 1.55, h1) * dir;
			v = rot_z(v, a, vec2(0.0, 0.0));
			n = rot_z_dir(n, a);
			v.y += fall * 0.28;
		} else {
			int mode = int(h2 * 3.99);
			if (mode == 3) {
				// Affaissement : les genoux cèdent, puis le corps bascule en avant.
				float sink = smoothstep(0.0, 0.35, since);
				float fall = smoothstep(0.3, 0.3 + dur, since);
				v.y *= mix(1.0, 0.6, sink * (1.0 - fall));
				v = rot_x(v, fall * 1.5, vec2(0.0, 0.0));
				n = rot_x_dir(n, fall * 1.5);
				v.y += fall * 0.12;
			} else if (mode == 2) {
				float fall = smoothstep(0.0, dur, since);
				float a = fall * 1.5 * dir;
				v = rot_z(v, a, vec2(0.0, 0.0));
				n = rot_z_dir(n, a);
				v.y += fall * 0.14;
			} else {
				float fall = smoothstep(0.0, dur, since);
				float a = fall * mix(1.4, 1.6, h1) * dir;
				v = rot_x(v, a, vec2(0.0, 0.0));
				n = rot_x_dir(n, a);
				v.y += fall * 0.12;
			}
		}
	}

	VERTEX = v;
	NORMAL = normalize(n);
	// Hauteur relative (occlusion d'ambiance approchée : bas des jambes et dessous plus sombres).
	v_height = corpse ? 1.0 : clamp(v.y / (mounted ? 1.9 : 1.1), 0.0, 1.0);
	v_color = COLOR.rgb;
	v_code = code;
	// Variation par soldat : teinte des vêtements, du teint, des lots de teinture.
	v_var = vec3(h1, h2, h3);
}

// Couleurs naturelles (sRGB) des vêtements non teints aux couleurs de la livrée.
const vec3 PLAIN[8] = vec3[](
	vec3(0.74, 0.68, 0.55), // écru
	vec3(0.45, 0.35, 0.25), // brun
	vec3(0.55, 0.36, 0.25), // roux
	vec3(0.55, 0.53, 0.49), // gris
	vec3(0.30, 0.28, 0.25), // laine sombre
	vec3(0.42, 0.44, 0.31), // vert passé
	vec3(0.40, 0.44, 0.52), // bleu passé
	vec3(0.66, 0.56, 0.34)  // ocre
);

void fragment() {
	int code = int(v_code + 0.5);
	vec3 albedo = v_color;
	float metallic = 0.0;
	float roughness = 0.85;
	float h4 = fract(v_var.x * 7.31 + v_var.z * 3.17);
	float h5 = fract(v_var.y * 5.13 + v_var.x * 11.7);
	bool wears_livery = h4 < livery_share || v_local.y > 2.8; // fanions de lance toujours en livrée
	// Livrée désaturée et assombrie (teintures végétales), lot de teinture propre au soldat.
	float lum = dot(livery, vec3(0.3, 0.59, 0.11));
	vec3 dyed = mix(livery, vec3(lum), 0.42) * mix(0.6, 0.88, v_var.x);
	dyed = mix(dyed, dyed * vec3(1.0, 0.9, 0.75), v_var.z * 0.4);
	if (code == 0) {
		if (wears_livery) {
			albedo = dyed * v_color;
		} else {
			vec3 plain = pow(PLAIN[int(h5 * 7.99)], vec3(2.2)) * mix(0.85, 1.1, v_var.z);
			albedo = plain * v_color;
			// Croix de livrée sur la poitrine et le dos du buste (bande verticale et traverse).
			vec3 q = v_local - vec3(0.0, torso_y, torso_z);
			bool chest = q.y > 1.0 && q.y < 1.42 && abs(v_local_n.z) > 0.45 && abs(v_local_n.y) < 0.6;
			if (chest && (abs(q.x) < 0.045 || abs(q.y - 1.25) < 0.04)) {
				albedo = dyed;
			}
		}
	} else if (code == 1) {
		albedo = trim * v_color;
		metallic = 0.3;
		roughness = 0.5;
	} else if (code == 2) {
		// Acier poli, terni ou bruni selon le soldat.
		albedo = v_color * mix(0.72, 1.08, v_var.z) * mix(vec3(1.0), vec3(0.95, 0.88, 0.8), step(0.75, h5));
		metallic = 0.8;
		roughness = mix(0.3, 0.55, h4);
	} else if (code == 3) {
		// Tissu : six teintes naturelles autour de la couleur de base.
		vec3 dyes[6] = vec3[](vec3(1.0), vec3(0.85, 0.88, 0.95), vec3(0.78, 0.86, 0.7), vec3(1.1, 0.85, 0.65), vec3(1.15, 1.08, 0.95), vec3(0.62, 0.58, 0.55));
		albedo = v_color * dyes[int(v_var.y * 5.99)] * mix(0.8, 1.1, v_var.x);
	} else if (code == 5) {
		// UV hors de [0, 1] (caparaçon hors du blason) : livrée unie.
		bool inside = UV.x >= 0.0 && UV.x <= 1.0 && UV.y >= 0.0 && UV.y <= 1.0;
		vec4 arms = has_heraldry && inside ? texture(heraldry, vec2(mix(0.08, 0.92, UV.x), mix(0.06, 0.94, UV.y))) : vec4(0.0);
		vec3 painted = mix(arms.rgb, vec3(dot(arms.rgb, vec3(0.3, 0.59, 0.11))), 0.25) * mix(0.7, 0.9, v_var.y);
		albedo = mix(dyed, painted, arms.a);
		roughness = 0.7;
	} else if (code == 4) {
		albedo = v_color * mix(0.92, 1.06, v_var.z);
		// Teint : du clair au hâlé (couleur SKIN de `battle_meshes.gd`, linéaire).
		if (distance(v_color, vec3(0.604, 0.319, 0.187)) < 0.05) {
			albedo = v_color * mix(vec3(1.08, 1.05, 1.0), vec3(0.72, 0.62, 0.52), h5);
		}
	}
	float ao = mix(0.55, 1.0, smoothstep(0.0, 1.0, v_height));
	// A1-01 : au loin, vêtements et caparaçons virent à la couleur franche du camp (la peau et
	// l'acier restent lisibles de près seulement), liseré de livrée et halo doré si sélectionné.
	vec3 side_col = livery * 1.15;
	float tint = far_blend * (code == 2 ? 0.35 : code == 4 ? 0.2 : 0.7);
	albedo = mix(albedo, side_col, tint);
	ao = mix(ao, 1.0, far_blend * 0.6);
	float rim = pow(1.0 - clamp(dot(NORMAL, VIEW), 0.0, 1.0), 2.0);
	vec3 glow = livery * (0.18 + 0.55 * rim) * far_blend;
	glow += select_glow * highlight * (0.08 + 0.5 * rim) * (0.3 + 0.7 * far_blend);
	EMISSION = glow;
	ALBEDO = albedo * ao * (1.0 + highlight * 0.25);
	METALLIC = metallic;
	ROUGHNESS = roughness;
	SPECULAR = 0.4;
}
