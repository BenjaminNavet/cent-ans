// Figurines de bataille skinnées (lot V2, audit A1-18 / A1-07 / A1-01) : maillages Blender
// (personnages et chevaux Quaternius CC0 habillés en XIVe siècle) dont chaque sommet porte
// quatre os et leurs poids (CUSTOM0 = indices, CUSTOM1 = poids) ; les animations sont cuites
// dans une texture d'os (`bone_tex` : trois texels RGBA32F = lignes d'une matrice 3×4 par os,
// une ligne de texture par image, 24 i/s). Rendu par MultiMesh : aucun squelette côté moteur.
//
// Choix du clip : l'état du régiment fixe un jeu de clips (`clip_set`, jusqu'à 4) et un mode
// (`clip_mode`) ; chaque soldat en tire un au sort (hachage de INSTANCE_ID, stable tant que
// son rang dans la formation l'est) avec sa phase propre. Mode 3 (cadavres, renversés, poses
// figées) : INSTANCE_CUSTOM = (instant de la chute, clip dans le jeu, vitesse de projection en
// arrière m/s, partie tranchée + sang) — lot BV2 : w = code entier (0 aucune, 1 tête, 2 avant-bras
// droit, 3 gauche, 4 jambe droite, 5 gauche, 6 cheval qui s'enfuit) + sang 0-0,99 en fraction.
// Changement d'état : fondu de `blend_time` secondes depuis le jeu précédent (`prev_*`).
// Matières selon COLOR.a × 16 (code) : livrée, bordure, plates, mailles, tissu, couleur exacte,
// armoiries (UV), peau, cuir, cheveux, robe du cheval, bois, gambison matelassé.
shader_type spatial;
render_mode cull_back, diffuse_burley, specular_schlick_ggx;

const int C_LIVERY = 0;
const int C_TRIM = 1;
const int C_PLATE = 2;
const int C_MAIL = 3;
const int C_CLOTH = 4;
const int C_EXACT = 5;
const int C_ARMS = 6;
const int C_SKIN = 7;
const int C_LEATHER = 8;
const int C_HAIR = 9;
const int C_COAT = 10;
const int C_WOOD = 11;
const int C_QUILT = 12;

const int M_LOOP = 0;
const int M_CYCLE = 1;
const int M_VOLLEY = 2;
const int M_CUSTOM = 3;
// SG1 : les `split_count` premiers soldats jouent set.x (escalade sur l'échelle), les autres
// tirent parmi le reste du jeu (au pied du mur).
const int M_SPLIT = 4;

uniform sampler2D bone_tex : filter_nearest, repeat_disable;
uniform float bone_fps = 24.0;
// Table des clips du rig : (ligne de départ, images, boucle 0/1, 0).
uniform vec4 clips[48];

uniform float anim_time = 0.0;
uniform ivec4 clip_set = ivec4(0);
uniform int clip_set_size = 1;
uniform int clip_mode = 0;
uniform float anim_speed = 1.0;
uniform ivec4 prev_set = ivec4(0);
uniform int prev_set_size = 1;
uniform int prev_mode = 0;
uniform float prev_speed = 1.0;
uniform float blend_since = -1000.0;
uniform float blend_time = 0.35;
// Mode CYCLE (mêlée) : durée d'une passe ; mode VOLLEY (tir) : instant de la dernière volée,
// durée du rechargement, instant de la décoche dans le clip (s).
uniform float cycle_len = 1.5;
uniform float volley_time = -1000.0;
uniform float reload_time = 6.0;
uniform float release_at = 1.0;
uniform int split_count = 0;

// Variantes (casques, armes...) : bit v de UV2.x = pièce visible pour la variante v.
uniform int variant_count = 1;
// BV3 : bit 7 du masque = pavois porté au dos (génois), masqué quand la rangée est plantée.
uniform bool hide_pavise = false;
uniform float size_jitter = 0.05;
// Distance (m) au-delà de laquelle les images ne sont plus interpolées.
uniform float interp_distance = 45.0;

uniform vec3 livery : source_color = vec3(0.2, 0.3, 0.75);
uniform vec3 trim : source_color = vec3(0.85, 0.7, 0.25);
uniform sampler2D heraldry : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_heraldry = false;
uniform float livery_share = 0.7;
uniform float highlight = 0.0;
// Sang du régiment 0-1 (taches progressives, lot BV2) ; par soldat, fraction de
// INSTANCE_CUSTOM.w (mode 3). Sans effet à 0.
uniform float blood = 0.0;
// Lot BV2 : os des parties tranchées (x..y, plus z et w si >= 0), indice = code de
// INSTANCE_CUSTOM.w ; entrée 0 = os du cheval (code 6 : il s'enfuit sans son cavalier).
uniform ivec4 sever_bones[6];
// Couche des renversés : la projection s'annule à la fin du clip (le soldat regagne sa place).
uniform bool tumble = false;
uniform float gravity = 9.8;
// Lisibilité lointaine (A1-01) : la livrée s'éclaire au-delà de `far_start` mètres.
uniform float far_start = 70.0;
uniform float far_boost = 0.35;

varying vec3 v_color;
varying float v_code;
varying vec4 v_var;
varying vec3 v_rest;
varying vec3 v_rest_n;
varying float v_blood;
varying float v_stump;

float hash1(float n) {
	return fract(sin(n * 12.9898 + 4.1414) * 43758.5453);
}

int pick(ivec4 s, int i) {
	return i == 0 ? s.x : (i == 1 ? s.y : (i == 2 ? s.z : s.w));
}

float pickf(vec4 s, int i) {
	return i == 0 ? s.x : (i == 1 ? s.y : (i == 2 ? s.z : s.w));
}

// Lignes de la matrice de skinning (somme pondérée des os) à la ligne `row` de la texture.
void skin_rows(int row, ivec4 idx, vec4 w, out vec4 a, out vec4 b, out vec4 c) {
	a = vec4(0.0);
	b = vec4(0.0);
	c = vec4(0.0);
	for (int k = 0; k < 4; k++) {
		float wk = pickf(w, k);
		if (wk > 0.0) {
			int x = pick(idx, k) * 3;
			a += texelFetch(bone_tex, ivec2(x, row), 0) * wk;
			b += texelFetch(bone_tex, ivec2(x + 1, row), 0) * wk;
			c += texelFetch(bone_tex, ivec2(x + 2, row), 0) * wk;
		}
	}
}

// Poids des os de la partie `part` (sever_bones) portés par un sommet.
float part_weight(int part, ivec4 idx, vec4 w) {
	ivec4 r = sever_bones[part];
	float total = 0.0;
	for (int k = 0; k < 4; k++) {
		int b = pick(idx, k);
		if ((b >= r.x && b <= r.y) || b == r.z || b == r.w) {
			total += pickf(w, k);
		}
	}
	return total;
}

// Clip et temps local d'un soldat pour un jeu de clips et un mode.
void choose(int mode, ivec4 set, int size, float speed, vec4 h, vec4 custom, float id, out int clip, out float t) {
	int n = max(size, 1);
	if (mode == M_SPLIT) {
		clip = id < float(split_count) || n < 2 ? set.x : pick(set, 1 + min(int(h.x * float(n - 1)), n - 2));
		t = anim_time * speed * mix(0.93, 1.07, h.y) + h.z * 11.0;
	} else if (mode == M_CUSTOM) {
		clip = pick(set, int(mod(custom.y, float(n))));
		t = anim_time - custom.x;
	} else if (mode == M_CYCLE) {
		float local = anim_time * mix(0.9, 1.1, h.y) + h.z * cycle_len;
		float cyc = floor(local / cycle_len);
		clip = pick(set, min(int(hash1(h.x * 91.7 + cyc * 13.1) * float(n)), n - 1));
		t = mod(local, cycle_len);
	} else if (mode == M_VOLLEY) {
		clip = pick(set, min(int(h.x * float(n)), n - 1));
		t = mod(anim_time - volley_time + release_at + (h.z - 0.5) * 0.4, max(reload_time, 0.5));
	} else {
		clip = pick(set, min(int(h.x * float(n)), n - 1));
		t = anim_time * speed * mix(0.93, 1.07, h.y) + h.z * 11.0;
	}
}

// Lignes de texture encadrant l'instant `t` du clip, et fraction entre les deux.
void clip_rows(int clip, float t, out int r0, out int r1, out float f) {
	vec4 c = clips[clip];
	float frames = max(c.y, 1.0);
	float x = max(t, 0.0) * bone_fps;
	float f0;
	float f1;
	if (c.z > 0.5) {
		x = mod(x, frames);
		f0 = floor(x);
		f1 = mod(f0 + 1.0, frames);
	} else {
		x = min(x, frames - 1.0);
		f0 = floor(x);
		f1 = min(f0 + 1.0, frames - 1.0);
	}
	f = x - f0;
	r0 = int(c.x + f0);
	r1 = int(c.x + f1);
}

void pose_rows(int clip, float t, ivec4 idx, vec4 w, bool smooth_frames, out vec4 a, out vec4 b, out vec4 c) {
	int r0;
	int r1;
	float f;
	clip_rows(clip, t, r0, r1, f);
	vec4 a0; vec4 b0; vec4 c0;
	skin_rows(r0, idx, w, a0, b0, c0);
	if (smooth_frames && f > 0.01 && r1 != r0) {
		vec4 a1; vec4 b1; vec4 c1;
		skin_rows(r1, idx, w, a1, b1, c1);
		a0 = mix(a0, a1, f);
		b0 = mix(b0, b1, f);
		c0 = mix(c0, c1, f);
	}
	a = a0;
	b = b0;
	c = c0;
}

void vertex() {
	float id = float(INSTANCE_ID);
	vec4 h = vec4(hash1(id * 1.37 + 0.11), hash1(id * 2.71 + 0.53), hash1(id * 0.73 + 1.9), hash1(id * 3.91 + 2.3));
	vec4 custom = INSTANCE_CUSTOM;
	v_rest = VERTEX;
	v_rest_n = NORMAL;

	// Variante du soldat : pièces absentes repliées en un point (triangles dégénérés).
	int mask = int(UV2.x + 0.5);
	int variant = min(int(h.w * float(variant_count)), variant_count - 1);
	bool shown = mask == 0 || ((mask >> variant) & 1) == 1;
	if (hide_pavise && (mask & 128) != 0) {
		shown = false;
	}

	ivec4 idx = ivec4(CUSTOM0 + 0.5);
	vec4 w = CUSTOM1;
	int clip;
	float t;
	choose(clip_mode, clip_set, clip_set_size, anim_speed, h, custom, id, clip, t);
	vec4 a; vec4 b; vec4 c;
	// Au loin, image la plus proche sans interpolation (moitié moins de lectures).
	bool smooth_frames = length((MODELVIEW_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz) < interp_distance;
	pose_rows(clip, t, idx, w, smooth_frames, a, b, c);
	float blend = clamp((anim_time - blend_since) / max(blend_time, 0.01), 0.0, 1.0);
	if (blend < 1.0 && clip_mode != M_CUSTOM) {
		int pclip;
		float pt;
		choose(prev_mode, prev_set, prev_set_size, prev_speed, h, custom, id, pclip, pt);
		vec4 pa; vec4 pb; vec4 pc;
		pose_rows(pclip, pt, idx, w, smooth_frames, pa, pb, pc);
		float s = smoothstep(0.0, 1.0, blend);
		a = mix(pa, a, s);
		b = mix(pb, b, s);
		c = mix(pc, c, s);
	}
	vec4 p = vec4(VERTEX, 1.0);
	vec3 v = vec3(dot(a, p), dot(b, p), dot(c, p));
	vec3 n = vec3(dot(a.xyz, NORMAL), dot(b.xyz, NORMAL), dot(c.xyz, NORMAL));
	v *= mix(1.0 - size_jitter, 1.0 + size_jitter, h.y);
	float own_blood = 0.0;
	v_stump = 0.0;
	if (clip_mode == M_CUSTOM) {
		// Lot BV2 : projection balistique en arrière (-Z du modèle), partie tranchée masquée.
		float part = floor(custom.w + 0.001);
		own_blood = clamp((custom.w - part) / 0.99, 0.0, 1.0);
		float imp = custom.z;
		if (abs(imp) > 0.05) {
			float vy = 0.35 * abs(imp) + 0.6;
			float fly = 2.0 * vy / gravity;
			float tt = clamp(t, 0.0, fly);
			vec3 offset = vec3(0.0, vy * tt - 0.5 * gravity * tt * tt, -imp * tt);
			if (tumble) {
				float len = clips[clip].y / bone_fps;
				offset *= 1.0 - smoothstep(len - 0.9, len, t);
			}
			v += offset;
		}
		int code = int(part + 0.5);
		if (code >= 1 && code <= 5) {
			// Poids de la partie tranchée, interpolé : le fragment coupe à 0,5 (variante
			// BV2_CORPSE du shader), le moignon rougit en deçà.
			v_stump = part_weight(code, idx, w);
		} else if (code == 6) {
			// Cavalier désarçonné : le cheval détale puis disparaît.
			float horse = part_weight(0, idx, w);
			if (horse > 0.5) {
				float run = max(t - 0.4, 0.0);
				v.z += run * run * 1.5 + run * 4.0;
				shown = shown && t < 4.5;
			}
		}
	}
	VERTEX = shown ? v : vec3(0.0);
	NORMAL = normalize(n);
	v_color = COLOR.rgb;
	v_code = COLOR.a;
	v_var = h;
	v_blood = max(own_blood, blood);
}

// Couleurs naturelles (linéaires) des vêtements non teints aux couleurs de la livrée.
const vec3 PLAIN[8] = vec3[](
	vec3(0.50, 0.42, 0.27), // écru
	vec3(0.17, 0.10, 0.05), // brun
	vec3(0.26, 0.11, 0.05), // roux
	vec3(0.26, 0.25, 0.21), // gris
	vec3(0.07, 0.065, 0.055), // laine sombre
	vec3(0.15, 0.16, 0.08), // vert passé
	vec3(0.13, 0.16, 0.23), // bleu passé
	vec3(0.40, 0.28, 0.10)  // ocre
);
const vec3 HAIR[5] = vec3[](
	vec3(0.035, 0.022, 0.012), vec3(0.09, 0.05, 0.02), vec3(0.20, 0.12, 0.05),
	vec3(0.012, 0.010, 0.009), vec3(0.28, 0.26, 0.23)
);
// Robes du cheval : bai, alezan, noir, gris, isabelle (multipliées par la couleur du sommet).
const vec3 COATS[5] = vec3[](
	vec3(0.16, 0.07, 0.03), vec3(0.26, 0.10, 0.03), vec3(0.03, 0.025, 0.022),
	vec3(0.34, 0.33, 0.31), vec3(0.36, 0.26, 0.13)
);

// Coordonnées de motif (triplanaire approchée sur la position de repos).
vec2 plane_uv(vec3 p, vec3 n) {
	vec3 an = abs(n);
	if (an.x > an.y && an.x > an.z) {
		return p.zy;
	} else if (an.z > an.y) {
		return p.xy;
	}
	return p.xz;
}

float noise2(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	float a = hash1(i.x + i.y * 57.0);
	float b = hash1(i.x + 1.0 + i.y * 57.0);
	float c = hash1(i.x + (i.y + 1.0) * 57.0);
	float d = hash1(i.x + 1.0 + (i.y + 1.0) * 57.0);
	return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

void fragment() {
	int code = int(v_code * 16.0 + 0.5);
	vec3 albedo = v_color;
	float metallic = 0.0;
	float roughness = 0.85;
	float specular = 0.4;
	vec2 q = plane_uv(v_rest, v_rest_n);
	float dist = length(VERTEX);
	float h5 = fract(v_var.x * 7.31 + v_var.z * 3.17);
	float h6 = fract(v_var.y * 5.13 + v_var.x * 11.7);
	// Livrée désaturée et assombrie (teintures végétales), lot de teinture propre au soldat.
	float lum = dot(livery, vec3(0.3, 0.59, 0.11));
	vec3 dyed = mix(livery, vec3(lum), 0.3) * mix(0.62, 0.9, v_var.x);
	float far = smoothstep(far_start, far_start * 2.5, dist);
	bool livery_surface = false;
	if (code == C_LIVERY) {
		bool wears = h5 < livery_share;
		if (wears) {
			albedo = dyed * v_color;
			livery_surface = true;
		} else {
			albedo = PLAIN[int(h6 * 7.99)] * mix(0.85, 1.15, v_var.z) * v_color * 1.6;
		}
		// Tissage : fin quadrillage, effacé quand il devient plus petit que le pixel.
		vec2 g = q * 260.0;
		float fade = 1.0 - smoothstep(0.3, 0.8, length(fwidth(g)));
		albedo *= 1.0 - 0.12 * fade * (0.5 + 0.5 * sin(g.x) * sin(g.y));
	} else if (code == C_TRIM) {
		albedo = trim * v_color;
		metallic = 0.5;
		roughness = 0.45;
	} else if (code == C_PLATE) {
		albedo = v_color * mix(0.8, 1.1, v_var.z) * mix(vec3(1.0), vec3(0.95, 0.88, 0.8), step(0.8, h6));
		float brushed = noise2(q * vec2(8.0, 90.0));
		metallic = 0.85;
		roughness = mix(0.28, 0.5, h5) + brushed * 0.1;
	} else if (code == C_MAIL) {
		// Mailles : anneaux en quinconce (≈ 1,6 cm), effacés au loin en gris moyen.
		vec2 g = q * 62.0;
		g.x += 0.5 * mod(floor(g.y), 2.0);
		float r = length(fract(g) - 0.5);
		float ringv = smoothstep(0.14, 0.24, r) * (1.0 - smoothstep(0.36, 0.47, r));
		float fade = 1.0 - smoothstep(0.35, 0.9, length(fwidth(g)));
		float m = mix(0.62, mix(0.35, 1.25, ringv), fade);
		albedo = v_color * m * mix(0.85, 1.1, v_var.z);
		metallic = 0.8;
		roughness = mix(0.55, mix(0.75, 0.35, ringv), fade);
	} else if (code == C_CLOTH || code == C_QUILT) {
		vec3 dyes[6] = vec3[](vec3(1.0), vec3(0.85, 0.88, 0.95), vec3(0.78, 0.86, 0.7), vec3(1.1, 0.85, 0.65), vec3(1.15, 1.08, 0.95), vec3(0.62, 0.58, 0.55));
		albedo = v_color * dyes[int(v_var.y * 5.99)] * mix(0.8, 1.12, v_var.x);
		vec2 g = q * 240.0;
		float fade = 1.0 - smoothstep(0.3, 0.8, length(fwidth(g)));
		albedo *= 1.0 - 0.1 * fade * (0.5 + 0.5 * sin(g.x + sin(g.y)));
		if (code == C_QUILT) {
			// Gambison : piqûres horizontales tous les 5 cm.
			float line = abs(fract(v_rest.y * 20.0) - 0.5);
			float fq = 1.0 - smoothstep(0.2, 0.45, fwidth(v_rest.y * 20.0));
			albedo *= 1.0 - 0.3 * fq * (1.0 - smoothstep(0.0, 0.12, line));
		}
	} else if (code == C_ARMS) {
		bool inside = UV.x >= 0.0 && UV.x <= 1.0 && UV.y >= 0.0 && UV.y <= 1.0;
		vec4 arms = has_heraldry && inside ? texture(heraldry, vec2(mix(0.08, 0.92, UV.x), mix(0.06, 0.94, UV.y))) : vec4(0.0);
		vec3 painted = mix(arms.rgb, vec3(dot(arms.rgb, vec3(0.3, 0.59, 0.11))), 0.2) * mix(0.72, 0.92, v_var.y);
		albedo = mix(dyed, painted, arms.a);
		roughness = 0.65;
		livery_surface = true;
	} else if (code == C_SKIN) {
		albedo = v_color * mix(vec3(1.1, 1.06, 1.0), vec3(0.62, 0.5, 0.4), h6);
		roughness = 0.6;
		specular = 0.3;
	} else if (code == C_LEATHER || code == C_WOOD) {
		float grain = code == C_WOOD ? noise2(q * vec2(6.0, 70.0)) : noise2(q * 40.0);
		albedo = v_color * mix(0.8, 1.15, grain) * mix(0.85, 1.12, v_var.z);
		roughness = code == C_WOOD ? 0.75 : 0.6;
	} else if (code == C_HAIR) {
		albedo = HAIR[int(h6 * 4.99)] * (v_color.r > 0.2 ? 1.2 : 1.0);
		roughness = 0.7;
	} else if (code == C_COAT) {
		albedo = COATS[int(v_var.w * 4.99)] * v_color;
		roughness = 0.6;
	}
	// Taches de sang : invisibles au loin, pas calculées au-delà de 90 m (coût des vivants).
	if (v_blood > 0.0 && code != C_PLATE && dist < 90.0) {
		// Deux octaves : taches arrondies et éclaboussures fines (lot BV2).
		float spots = noise2(q * 7.0 + v_var.xy * 40.0) * 0.7 + noise2(q * 26.0 + v_var.zw * 17.0) * 0.3;
		float amount = smoothstep(1.0 - v_blood * 0.6, 1.0, spots);
		albedo = mix(albedo, vec3(0.09, 0.005, 0.004), amount);
		roughness = mix(roughness, 0.3, amount);
	}
#ifdef BV2_CORPSE
	if (v_stump > 0.5) {
		discard;
	}
	if (v_stump > 0.08) {
		// Moignon : chair et sang autour de la coupure.
		float raw = smoothstep(0.08, 0.4, v_stump);
		albedo = mix(albedo, vec3(0.16, 0.012, 0.01), raw);
		roughness = mix(roughness, 0.35, raw);
		metallic = mix(metallic, 0.0, raw);
	}
#endif
	float ao = mix(0.6, 1.0, smoothstep(0.0, 1.2, v_rest.y));
	ALBEDO = albedo * ao * (1.0 + highlight * 0.25);
	METALLIC = metallic;
	ROUGHNESS = clamp(roughness, 0.05, 1.0);
	SPECULAR = specular;
	if (livery_surface && far > 0.0) {
		EMISSION = dyed * far * far_boost;
	}
}
