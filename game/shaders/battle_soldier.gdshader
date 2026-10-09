// Engins de siège et servants de repli (maillages rigides de `battle_meshes.gd`). Lot SC bt3
// (ADR 0238) : les soldats sont tous des figurines skinnées (`battle_skinned_*.gdshader`) ; ce
// shader ne garde que ce qu'utilisent les engins : verge ou bras de lancement (membre 15,
// CUSTOM0.x) qui remonte lentement puis décoche pendant l'état « tir », chute des cadavres
// (MultiMesh à données perso, instant de mort dans INSTANCE_CUSTOM.x), lisibilité à distance
// (teinte de camp, liseré, sélection) et matières selon COLOR.a : livrée, bordure, métal,
// tissu varié, couleur exacte.
shader_type spatial;
render_mode cull_back, diffuse_burley, specular_schlick_ggx;

const int S_SHOOT = 4;
const int P_ENGINE_ARM = 15;

// Part des servants portant le surcot de livrée (les autres : teinte naturelle + croix).
const float LIVERY_SHARE = 0.4;

uniform vec3 livery : source_color = vec3(0.2, 0.3, 0.75);
uniform vec3 trim : source_color = vec3(0.85, 0.7, 0.25);
uniform float anim_time = 0.0;
uniform int anim_state = 0;
uniform bool corpse = false;
uniform float highlight = 0.0;            // régiment sélectionné : léger éclaircissement
// Lisibilité à distance (0 de près, 1 au loin ; fixé par `battle_soldiers.gd`) : teinte de camp
// renforcée, liseré lumineux de la livrée, engin un peu grossi, sélection soulignée d'or.
uniform float far_blend = 0.0;
uniform float far_scale = 0.18;
uniform vec3 select_glow : source_color = vec3(1.0, 0.85, 0.35);

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

vec3 rot_z_dir(vec3 n, float a) {
	float c = cos(a);
	float s = sin(a);
	return vec3(n.x * c - n.y * s, n.x * s + n.y * c, n.z);
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

	// Verge de l'engin : lente remontée, décoche brusque (état « tir »).
	float engine = 0.0;
	if (!corpse && anim_state == S_SHOOT) {
		float se = fract(anim_time * (0.22 + h2 * 0.06) + h1);
		engine = se < 0.8 ? mix(0.0, -0.9, smoothstep(0.0, 0.8, se)) : mix(-0.9, 1.3, smoothstep(0.8, 0.86, se));
	}
	if (part == P_ENGINE_ARM) {
		v = rot_x(v, engine, pivot);
		n = rot_x_dir(n, engine);
	}
	v *= 1.0 + far_scale * far_blend;

	// Cadavres : chute variée après l'instant de mort (INSTANCE_CUSTOM.x) — en arrière, en
	// avant, sur le côté, ou affaissement avant de basculer ; durée propre à chaque instance.
	if (corpse) {
		float since = anim_time - INSTANCE_CUSTOM.x;
		float dur = mix(0.5, 1.1, h3);
		float dir = INSTANCE_CUSTOM.z >= 0.0 ? 1.0 : -1.0;
		int mode = int(h2 * 3.99);
		if (mode == 3) {
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

	VERTEX = v;
	NORMAL = normalize(n);
	// Hauteur relative (occlusion d'ambiance approchée : dessous plus sombres).
	v_height = corpse ? 1.0 : clamp(v.y / 1.1, 0.0, 1.0);
	v_color = COLOR.rgb;
	v_code = code;
	// Variation par instance : teinte des vêtements, du teint, des lots de teinture.
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
	bool wears_livery = h4 < LIVERY_SHARE || v_local.y > 2.8; // fanions de lance toujours en livrée
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
			vec3 q = v_local - vec3(0.0);
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
