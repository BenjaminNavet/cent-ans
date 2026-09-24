// Figurines des batailles (lot V4) : animation par shader de sommets, compatible MultiMesh.
// Chaque sommet porte son membre et le pivot de ce membre (CUSTOM0, cf. `battle_meshes.gd`) ;
// le shader fait tourner jambes, bras, arme, encolure et jambes du cheval autour de l'axe X
// selon l'état du régiment (uniforme `anim_state`) et une phase propre à chaque soldat
// (hachage de INSTANCE_ID). Les cadavres (MultiMesh à données perso) lisent leur instant de
// mort dans INSTANCE_CUSTOM et basculent au sol. Matières selon COLOR.a : livrée, bordure,
// métal, tissu varié, couleur exacte, armoiries (texture du blason de la faction).
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
uniform float highlight = 0.0;            // régiment sélectionné : léger éclaircissement
// Part des soldats portant le surcot de livrée (lot V4b) ; les autres portent un vêtement de
// teinte naturelle (écru, brun, roux, gris…) marqué d'une croix de livrée sur la poitrine.
uniform float livery_share = 0.55;
uniform float torso_y = 0.0;              // décalage du buste (cavaliers : assise sur la selle)
uniform float torso_z = 0.0;

varying vec3 v_color;
varying float v_code;
varying vec3 v_var;
varying float v_height;
varying vec3 v_local;    // position au repos (avant animation), pour la croix de poitrine
varying vec3 v_local_n;

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

	int state = corpse ? -1 : anim_state;
	float t = anim_time;
	float ph = h1 * 6.2832;

	// Angles des membres (radians ; négatif = vers l'avant pour un membre pendant).
	float leg = 0.0;
	float arm_r = 0.0;
	float arm_l = 0.0;
	float weapon_total = 0.0; // orientation absolue souhaitée de l'arme (0 = droite)
	bool weapon_abs = true;
	float bow = 0.15;          // arc : orientation absolue
	float draw = 0.0;          // corde tirée : recul du bras droit (m)
	float lean = 0.0;
	float bob = 0.0;
	float engine = 0.0;
	// Cheval.
	float hl_fl = 0.0;
	float hl_fr = 0.0;
	float hl_bl = 0.0;
	float hl_br = 0.0;
	float neck = 0.0;
	float horse_pitch = 0.0;
	float horse_bob = 0.0;

	if (weapon_mode == W_SWORD) weapon_total = 0.35;
	if (weapon_mode == W_CROSSBOW) weapon_total = 0.0;

	if (state == S_IDLE) {
		float br = sin(t * 1.6 + ph);
		bob = br * 0.006;
		lean = sin(t * 0.37 + ph) * 0.025;
		arm_r = -0.08 + br * 0.03;
		arm_l = -0.05 - br * 0.02;
		neck = sin(t * 0.5 + ph) * 0.12 - 0.1;
		if (weapon_mode == W_BOW) {
			bow = 0.1;
		}
	} else if (state == S_WALK || state == S_CLIMB) {
		float c = t * 6.2832 * (0.95 + h2 * 0.1) + ph;
		leg = sin(c) * 0.42;
		arm_l = -sin(c) * 0.22 - 0.05;
		arm_r = sin(c) * 0.18 - 0.1;
		bob = abs(cos(c)) * 0.035;
		lean = 0.04;
		if (weapon_mode == W_POLE) weapon_total = 0.08 + sin(c) * 0.03;
		float g = t * 6.2832 * 0.9 + ph;
		hl_fl = sin(g) * 0.35;
		hl_br = sin(g) * 0.35;
		hl_fr = -sin(g) * 0.35;
		hl_bl = -sin(g) * 0.35;
		neck = sin(g * 2.0) * 0.08;
		horse_bob = abs(sin(g)) * 0.04;
		if (state == S_CLIMB) {
			arm_r = -2.6 + sin(c) * 0.35;
			arm_l = -2.6 - sin(c) * 0.35;
			lean = -0.1;
		}
	} else if (state == S_RUN || state == S_ROUT) {
		float c = t * 6.2832 * (1.45 + h2 * 0.15) + ph;
		leg = sin(c) * 0.75;
		arm_l = -sin(c) * 0.5 - 0.2;
		arm_r = sin(c) * 0.45 - 0.2;
		bob = abs(cos(c)) * 0.07;
		lean = state == S_ROUT ? 0.3 : 0.2;
		if (state == S_RUN) {
			if (weapon_mode == W_POLE) {
				weapon_total = 1.2;
				arm_r = -0.45 + sin(c) * 0.08;
			}
			if (weapon_mode == W_LANCE) {
				weapon_total = 1.42;
				arm_r = -0.3;
			}
			if (weapon_mode == W_SWORD) {
				arm_r = -1.3 + sin(c) * 0.15;
			}
		} else {
			weapon_total = 0.9; // arme portée n'importe comment dans la déroute
		}
		// Galop : jambes par paires décalées, tangage et rebond du corps.
		float g = t * 6.2832 * (1.9 + h2 * 0.2) + ph;
		hl_fl = sin(g) * 0.7;
		hl_fr = sin(g + 0.5) * 0.7;
		hl_bl = sin(g + 3.1) * 0.65;
		hl_br = sin(g + 3.6) * 0.65;
		horse_pitch = sin(g + 1.2) * 0.07;
		horse_bob = (sin(g + 1.6) * 0.5 + 0.5) * 0.14;
		neck = sin(g + 1.2) * 0.15;
	} else if (state == S_MELEE) {
		float speed = 0.85 + h2 * 0.35;
		float s = fract(t * speed + h1);
		leg = 0.22 + sin(t * 1.3 + ph) * 0.05;
		lean = 0.1 + sin(t * speed * 6.2832 + ph) * 0.05;
		if (weapon_mode == W_SWORD) {
			// Armer lentement au-dessus de l'épaule, frapper vite, revenir.
			arm_r = s < 0.55 ? mix(-0.3, -2.7, smoothstep(0.0, 0.55, s)) : mix(-2.7, -0.4, smoothstep(0.55, 0.72, s));
			weapon_abs = false;
			weapon_total = 0.25;
			arm_l = -0.85 + sin(t * 2.0 + ph) * 0.1;
		} else if (weapon_mode == W_POLE) {
			// Piques et lances abaissées, estocades.
			float thrust = pow(max(sin(s * 6.2832), 0.0), 3.0);
			arm_r = -0.5 - thrust * 0.45;
			arm_l = -0.7 - thrust * 0.3;
			weapon_total = 1.5 - thrust * 0.05;
			lean = 0.12 + thrust * 0.12;
		} else if (weapon_mode == W_LANCE) {
			arm_r = -0.35 - pow(max(sin(s * 6.2832), 0.0), 3.0) * 0.5;
			weapon_total = 1.35;
		} else {
			arm_r = s < 0.5 ? -1.9 : -0.4;
			arm_l = -0.9;
			weapon_abs = false;
			weapon_total = 0.3;
		}
		// Chevaux piétinant.
		float g = t * 6.2832 * 1.1 + ph;
		hl_fl = max(sin(g), 0.0) * 0.6;
		hl_fr = max(sin(g + 2.0), 0.0) * 0.5;
		hl_bl = sin(g + 1.0) * 0.1;
		hl_br = sin(g + 3.0) * 0.1;
		horse_pitch = -max(sin(g), 0.0) * 0.08;
		neck = sin(g * 1.3) * 0.2 - 0.1;
	} else if (state == S_SHOOT) {
		float speed = 0.22 + h2 * 0.06;
		float s = fract(t * speed + h1);
		neck = sin(t * 0.5 + ph) * 0.1 - 0.1;
		if (weapon_mode == W_BOW) {
			// Arc levé vers l'avant et un peu vers le haut ; corde tirée puis lâchée.
			arm_l = -1.78;
			bow = 0.12;
			arm_r = -1.72;
			draw = s < 0.7 ? smoothstep(0.1, 0.65, s) * 0.42 : 0.0;
			if (s >= 0.72) {
				arm_r = mix(-1.72, -0.2, smoothstep(0.72, 0.85, s)); // flèche au carquois
				arm_r = mix(arm_r, -1.72, smoothstep(0.88, 1.0, s));
			}
			lean = -0.08;
			leg = 0.2;
		} else if (weapon_mode == W_CROSSBOW) {
			// Viser, tirer, puis réarmer (arbalète pointée au sol, buste penché).
			if (s < 0.35) {
				arm_r = -1.35;
				arm_l = -1.35;
				weapon_total = -0.08;
				leg = 0.15;
			} else {
				float k = smoothstep(0.35, 0.45, s) * (1.0 - smoothstep(0.9, 1.0, s));
				arm_r = mix(-1.35, -0.4, k);
				arm_l = mix(-1.35, -0.5, k);
				weapon_total = mix(-0.08, 1.2, k);
				lean = k * 0.55;
			}
		} else if (weapon_mode == W_ENGINE) {
			// Verge de l'engin : lente remontée, décoche brusque.
			engine = s < 0.8 ? mix(0.0, -0.9, smoothstep(0.0, 0.8, s)) : mix(-0.9, 1.3, smoothstep(0.8, 0.86, s));
		} else {
			arm_r = -0.3;
		}
	} else if (corpse) {
		// Membres figés dans une pose aléatoire.
		leg = (h2 - 0.5) * 0.8;
		arm_r = -h3 * 2.2;
		arm_l = -(1.0 - h3) * 1.6;
		weapon_total = 1.4;
		hl_fl = (h2 - 0.5) * 1.2;
		hl_fr = (h3 - 0.5) * 1.2;
		hl_bl = (h1 - 0.5) * 0.8;
		hl_br = (h2 - 0.3) * 0.8;
		neck = 0.9;
	}

	// 1. Rotation propre du membre.
	float arm_for_weapon = arm_r;
	if (part == 1) { v = rot_x(v, leg, pivot); n = rot_x_dir(n, leg); }
	else if (part == 2) { v = rot_x(v, -leg, pivot); n = rot_x_dir(n, -leg); }
	else if (part == 13) {
		float w = weapon_abs ? weapon_total - arm_for_weapon : weapon_total;
		v = rot_x(v, w, pivot);
		n = rot_x_dir(n, w);
		v.z -= draw * 0.0;
		v = rot_x(v, arm_r, shoulder);
		n = rot_x_dir(n, arm_r);
	}
	else if (part == 17) {
		float b = bow - arm_l;
		v = rot_x(v, b, pivot);
		n = rot_x_dir(n, b);
		v = rot_x(v, arm_l, shoulder);
		n = rot_x_dir(n, arm_l);
	}
	else if (part == 3) { v = rot_x(v, arm_r, pivot); n = rot_x_dir(n, arm_r); v.z -= draw; }
	else if (part == 4) { v = rot_x(v, arm_l, pivot); n = rot_x_dir(n, arm_l); }
	else if (part == 5) { v = rot_x(v, lean * -0.4, vec2(1.5, 0.0)); }
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

	// 2. Buste (tout sauf jambes, cheval, jupe) : inclinaison aux hanches et rebond.
	bool upper = part == 0 || part == 3 || part == 4 || part == 5 || part == 12 || part == 13 || part == 17;
	if (upper) {
		v = rot_x(v, lean, hip);
		n = rot_x_dir(n, lean);
	}
	if (part != 1 && part != 2 && part != 14 && part != 15) {
		v.y += bob;
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
	v *= scale;
	v.x *= girth;

	// 5. Cadavres : bascule au sol après l'instant de mort (INSTANCE_CUSTOM.x).
	if (corpse) {
		float fall = smoothstep(0.0, 0.7, anim_time - INSTANCE_CUSTOM.x);
		float dir = INSTANCE_CUSTOM.z >= 0.0 ? 1.0 : -1.0;
		if (mounted) {
			float a = fall * 1.45 * dir;
			v = rot_z(v, a, vec2(0.0, 0.0));
			float c = cos(a);
			float s = sin(a);
			n = vec3(n.x * c - n.y * s, n.x * s + n.y * c, n.z);
			v.y += fall * 0.28;
		} else {
			float a = fall * 1.5 * dir;
			v = rot_x(v, a, vec2(0.0, 0.0));
			n = rot_x_dir(n, a);
			v.y += fall * 0.12;
		}
	}

	VERTEX = v;
	NORMAL = normalize(n);
	// Hauteur relative (occlusion d'ambiance approchée : bas des jambes et dessous plus sombres).
	v_height = corpse ? 1.0 : clamp(v.y / (mounted ? 1.9 : 1.1), 0.0, 1.0);
	v_color = COLOR.rgb;
	v_code = COLOR.a * 5.0;
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
		vec4 arms = has_heraldry ? texture(heraldry, vec2(mix(0.08, 0.92, UV.x), mix(0.06, 0.94, UV.y))) : vec4(0.0);
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
	ALBEDO = albedo * ao * (1.0 + highlight * 0.25);
	METALLIC = metallic;
	ROUGHNESS = roughness;
	SPECULAR = 0.4;
}
