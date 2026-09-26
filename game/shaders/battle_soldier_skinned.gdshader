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
// droit, 3 gauche, 4 jambe droite, 5 gauche, 6 cheval qui s'enfuit ; + 8 : désarmé, lot EP12)
// + sang 0-0,99 en fraction.
// Changement d'état : fondu de `blend_time` secondes depuis le jeu précédent (`prev_*`).
// Matières selon COLOR.a × 16 (code) : livrée, bordure, plates, mailles, tissu, couleur exacte,
// armoiries (UV), peau, cuir, cheveux, robe du cheval, bois, gambison matelassé.
// Lot FG3 : variante `FG3_BAKED` (posée par BattleSkinned pour les figurines fines cuites
// seulement ; le rendu par défaut est compilé sans ces lignes) : atlas par figurine (normale de
// forme, occlusion, masque) et tuiles de détail partagées par matière, lus au LOD0/LOD1 (UV
// d'atlas empaquetée dans UV2.y ; LOD2 : aucune lecture).
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
// DA1 : atlas d'armoiries de la bataille (Texture2DArray : factions, maisons des généraux,
// bannerets) et couches du régiment : x = seigneur (ou faction), y/z/w = bannerets (-1 : aucun),
// portés par `vassal_share` des figurines (tirage par hachage de l'instance, sans donnée CPU).
uniform sampler2DArray arms_atlas : source_color, filter_linear_mipmap, repeat_disable;
uniform bool use_atlas = false;
uniform ivec4 arms_layers = ivec4(0, -1, -1, -1);
uniform float vassal_share = 0.3;
// DA1 : buste en pose de repos (centre x/y, largeur/hauteur) et profondeur z : armoiries sur la
// poitrine et le dos du surcot (nobles, `chest_arms`) ou croix de livrée du commun (`badge_*`,
// alpha > 0,5 = actif ; pièce d'étoffe sous la croix).
uniform bool chest_arms = false;
uniform vec4 chest_box = vec4(0.0, 1.22, 0.3, 0.38);
uniform vec2 chest_depth = vec2(-0.1, 0.3);
uniform vec4 badge_cross : source_color = vec4(0.0);
uniform vec4 badge_patch : source_color = vec4(0.0);
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
// Lot EP12 : régiment en déroute qui a jeté armes et boucliers (faces marquées du bit 6 du
// masque de variante, `HELD_MASK` de tools/blender_scripts/battle_skinned.py). En mode 3, le
// drapeau 8 du code de INSTANCE_CUSTOM.w fait de même pour un cadavre ou un blessé.
uniform bool drop_arms = false;
const int HELD_MASK = 64;
const int CODE_UNARMED = 8;
// Lisibilité lointaine (A1-01) : la livrée s'éclaire au-delà de `far_start` mètres.
uniform float far_start = 70.0;
uniform float far_boost = 0.35;

#ifdef FG3_BAKED
// Atlas des figurines fines (couche = figurine) : RG normale de forme (espace tangent de l'UV
// d'atlas), B occlusion cuite, A masque selon la matière (barbe/cheveux : densité ; peau :
// sourcils, lèvres, barbe naissante ; plates : martelage ; étoffe teinte : zone de livrée ;
// autres : 1). Tuiles de détail (couches FG3_TILE_*) : RG normale, B relief (albédo), A rugosité.
// Pelage du cheval (CC0 réduit, UV d'origine) : RG normale, B relief de la robe, A occlusion.
uniform sampler2DArray fine_atlas0 : filter_linear_mipmap_anisotropic, repeat_disable;
uniform sampler2DArray fine_atlas1 : filter_linear_mipmap, repeat_disable;
uniform sampler2DArray fine_detail : filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D fine_horse : filter_linear_mipmap_anisotropic, repeat_enable;
uniform int fine_layer = -1;
// Au-delà (m), plus aucune lecture de carte (LOD2 et imposteurs lointains).
uniform float fine_distance = 80.0;
// Tuiles : couche et taille (m) par matière.
const int FG3_TILE_MAIL = 0;
const int FG3_TILE_WEAVE = 1;
const int FG3_TILE_FELT = 2;
const int FG3_TILE_LEATHER = 3;
const int FG3_TILE_HAMMERED = 4;
const int FG3_TILE_WOOD = 5;
const int FG3_TILE_SKIN = 6;
const int FG3_TILE_HAIR = 7;
const float FG3_TILE_SIZE[8] = float[](0.072, 0.024, 0.08, 0.12, 0.16, 0.3, 0.06, 0.05);
// Teints (multipliés par la couleur du sommet) : pâle, clair rosé, hâlé, olivâtre, buriné,
// brun (bible : variation par soldat).
const vec3 FG3_SKIN[6] = vec3[](
	vec3(1.12, 1.04, 0.98), vec3(1.08, 0.94, 0.86), vec3(0.96, 0.82, 0.66),
	vec3(0.88, 0.80, 0.62), vec3(0.84, 0.66, 0.52), vec3(0.62, 0.48, 0.36)
);
// Cheveux et barbes (linéaires) : châtain, brun, blond cendré, roux, noir, grisonnant, blanc.
const vec3 FG3_HAIR[7] = vec3[](
	vec3(0.11, 0.065, 0.035), vec3(0.065, 0.04, 0.022), vec3(0.30, 0.22, 0.12),
	vec3(0.26, 0.10, 0.04), vec3(0.035, 0.03, 0.028), vec3(0.20, 0.18, 0.16),
	vec3(0.42, 0.40, 0.37)
);
varying vec2 v_atlas;
varying flat int v_src;
#endif

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
	// Bits 0-5 : variantes ; bit 6 : arme tenue (EP12) ; bit 7 : pavois du dos (BV3).
	int variant_bits = mask & 63;
	bool shown = variant_bits == 0 || ((variant_bits >> variant) & 1) == 1;
	if (hide_pavise && (mask & 128) != 0) {
		shown = false;
	}
	bool held = (mask & HELD_MASK) != 0;
	if (drop_arms && held) {
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
		if (code >= CODE_UNARMED) {
			// EP12 : blessé ou fuyard tombé, désarmé.
			code -= CODE_UNARMED;
			shown = shown && !held;
		}
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
#ifdef FG3_BAKED
	// UV d'atlas : u, v sur 11 bits, source sur 2 (1 atlas LOD0, 3 atlas LOD1, 2 cheval).
	int packed = int(UV2.y + 0.5);
	v_src = fine_layer >= 0 ? (packed >> 22) & 3 : 0;
	v_atlas = vec2(float(packed & 2047), float((packed >> 11) & 2047)) / 2047.0;
#endif
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

// DA1 : couche d'armoiries de la figurine (seigneur, ou l'un des bannerets).
float arms_layer() {
	int layer = arms_layers.x;
	int count = int(arms_layers.y >= 0) + int(arms_layers.z >= 0) + int(arms_layers.w >= 0);
	float r = fract(v_var.x * 17.23 + v_var.w * 7.91);
	if (count > 0 && r < vassal_share) {
		int k = min(int(r / vassal_share * float(count)), count - 1);
		layer = k == 0 ? arms_layers.y : (k == 1 ? arms_layers.z : arms_layers.w);
	}
	return float(layer);
}

vec4 arms_sample(vec2 uv) {
	if (use_atlas) {
		return texture(arms_atlas, vec3(uv, arms_layer()));
	}
	return has_heraldry ? texture(heraldry, uv) : vec4(0.0);
}

#ifdef FG3_BAKED
// Normale perturbée par une carte d'espace tangent sans tangentes stockées : repère
// cotangent tiré des dérivées écran de la position et de l'UV (v vers le haut de la carte).
vec3 fg3_perturb(vec3 n, vec3 dp1, vec3 dp2, vec2 duv1, vec2 duv2, vec2 nxy) {
	vec3 dp2perp = cross(dp2, n);
	vec3 dp1perp = cross(n, dp1);
	vec3 t = dp2perp * duv1.x + dp1perp * duv2.x;
	vec3 b = dp2perp * duv1.y + dp1perp * duv2.y;
	float m = max(dot(t, t), dot(b, b));
	if (m < 1e-20) {
		return n;
	}
	float inv = inversesqrt(m);
	return normalize(t * (inv * nxy.x) + b * (inv * nxy.y) + n);
}
#endif

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
		// DA1 : armoiries (nobles) ou croix de livrée (commun) sur la poitrine et le dos.
		vec2 cq = vec2((v_rest.x - chest_box.x) / chest_box.z + 0.5, (chest_box.y + 0.5 * chest_box.w - v_rest.y) / chest_box.w);
		bool torso = abs(v_rest_n.z) > 0.35 && v_rest.z > chest_depth.x && v_rest.z < chest_depth.y
			&& cq.x > 0.0 && cq.x < 1.0 && cq.y > 0.0 && cq.y < 1.0 && dist < 60.0;
		if (torso && chest_arms && wears) {
			// Le buste regarde -Z en pose de repos (le squelette cuit le demi-tour) : vu de face
			// ou de dos, l'écu se lit dans le même sens (dextre à gauche du spectateur).
			vec2 uv = vec2(v_rest_n.z > 0.0 ? 1.0 - cq.x : cq.x, cq.y);
			vec4 arms = arms_sample(mix(vec2(0.1, 0.06), vec2(0.9, 0.94), uv));
			albedo = mix(albedo, arms.rgb * mix(0.86, 0.98, v_var.y) * v_color, arms.a);
		} else if (torso && badge_cross.a > 0.5) {
			vec2 d = abs(cq - vec2(0.5, 0.56));
			if (badge_patch.a > 0.5 && d.x < 0.3 && d.y < 0.28) {
				albedo = badge_patch.rgb * mix(0.8, 0.95, v_var.y) * v_color;
			}
			if ((d.x < 0.07 && d.y < 0.26) || (abs(cq.y - 0.5) < 0.06 && d.x < 0.26)) {
				albedo = badge_cross.rgb * mix(0.8, 0.95, v_var.y) * v_color;
			}
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
		vec4 arms = inside ? arms_sample(vec2(mix(0.08, 0.92, UV.x), mix(0.06, 0.94, UV.y))) : vec4(0.0);
		// DA1 (bible § 2, 3.1) : armoiries saturées, étoffe teinte (±6 % de luminosité).
		vec3 painted = arms.rgb * mix(0.84, 0.96, v_var.y);
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
#ifdef FG3_BAKED
	// Lot FG3 : cartes cuites (LOD0/LOD1 des figurines fines ; LOD2 : v_src = 0, rien de
	// plus). v_src est constant sur tout un maillage LOD0/LOD1 (1, 3 ou 2 pour le cheval, tous
	// non nuls) : les dérivées restent en flux uniforme ; lectures en textureGrad.
	float fg_ao = 1.0;
	if (v_src != 0) {
		vec3 fg_dp1 = dFdx(VERTEX);
		vec3 fg_dp2 = dFdy(VERTEX);
		vec2 fg_da1 = dFdx(v_atlas);
		vec2 fg_da2 = dFdy(v_atlas);
		vec2 fg_dq1 = dFdx(q);
		vec2 fg_dq2 = dFdy(q);
		if (dist < fine_distance) {
			vec4 at;
			if (v_src == 2) {
				at = textureGrad(fine_horse, v_atlas, fg_da1, fg_da2);
			} else if (v_src == 1) {
				at = textureGrad(fine_atlas0, vec3(v_atlas, float(fine_layer)), fg_da1, fg_da2);
			} else {
				at = textureGrad(fine_atlas1, vec3(v_atlas, float(fine_layer)), fg_da1, fg_da2);
			}
			// Le v de la carte descend : le repère prend -v.
			vec3 n = fg3_perturb(NORMAL, fg_dp1, fg_dp2, vec2(fg_da1.x, -fg_da1.y), vec2(fg_da2.x, -fg_da2.y), at.rg * 2.0 - 1.0);
			float mask = at.a;
			if (v_src == 2) {
				// Cheval : relief de la robe (passe-haut de la couleur CC0) et occlusion fine.
				albedo *= mix(0.6, 1.4, at.b);
				fg_ao = at.a;
				mask = 1.0;
			} else {
				fg_ao = at.b;
			}
			int tl = -1;
			float amt = 0.0;
			float relief = 0.5;
			vec3 skin_tone = vec3(0.50, 0.33, 0.24) * FG3_SKIN[int(h6 * 5.99)];
			if (code == C_MAIL) {
				tl = FG3_TILE_MAIL;
				amt = 1.0;
				relief = 0.0; // albédo refait ci-dessous
			} else if (code == C_PLATE) {
				// Martelage : fort sur les casques (masque), léger sur les autres plates.
				tl = FG3_TILE_HAMMERED;
				amt = mix(0.12, 0.45, mask);
				relief = 0.06;
			} else if (code == C_TRIM) {
				tl = FG3_TILE_HAMMERED;
				amt = 0.2;
				relief = 0.06;
			} else if (code == C_LIVERY || code == C_ARMS || code == C_CLOTH) {
				tl = FG3_TILE_WEAVE;
				amt = 0.55;
				relief = 0.25;
			} else if (code == C_QUILT) {
				tl = FG3_TILE_FELT;
				amt = 0.7;
				relief = 0.35;
			} else if (code == C_LEATHER) {
				tl = FG3_TILE_LEATHER;
				amt = 0.8;
				relief = 0.45;
			} else if (code == C_WOOD) {
				tl = FG3_TILE_WOOD;
				amt = 0.45;
				relief = 0.22;
			} else if (code == C_SKIN) {
				tl = FG3_TILE_SKIN;
				amt = 0.35;
				relief = 0.12;
				// Teints variés ; le masque assombrit sourcils et barbe naissante vers le poil.
				albedo = v_color * FG3_SKIN[int(h6 * 5.99)] * mix(0.94, 1.06, v_var.z);
				albedo = mix(FG3_HAIR[int(h5 * 6.99)] * 1.4, albedo, mix(0.35, 1.0, mask));
			} else if (code == C_HAIR || code == C_COAT) {
				tl = FG3_TILE_HAIR;
				amt = code == C_HAIR ? 0.9 : 0.45;
				relief = code == C_HAIR ? 0.0 : 0.3;
			}
			if (tl >= 0) {
				float size = FG3_TILE_SIZE[tl];
				vec4 d = textureGrad(fine_detail, vec3(vec2(q.x, -q.y) / size, float(tl)), vec2(fg_dq1.x, -fg_dq1.y) / size, vec2(fg_dq2.x, -fg_dq2.y) / size);
				n = fg3_perturb(n, fg_dp1, fg_dp2, fg_dq1, fg_dq2, (d.rg * 2.0 - 1.0) * amt);
				albedo *= 1.0 + relief * (d.b * 2.0 - 1.0);
				if (code == C_MAIL) {
					// Mailles lisibles sans être noires : anneaux clairs, creux sombres, moins
					// métalliques que l'acier plein (pas de reflet du ciel dans la maille serrée).
					albedo = v_color * mix(0.85, 1.1, v_var.z) * mix(0.38, 1.15, d.b);
					metallic = 0.6;
					roughness = mix(0.7, 0.32, d.a);
					fg_ao = mix(0.55, 1.0, fg_ao);
				} else if (code == C_HAIR) {
					// Barbes et cheveux : couleur plus claire et variée ; la densité (masque) fond
					// les bords et la bouche dans la peau, les mèches cassent la lisière.
					vec3 hair = FG3_HAIR[int(h6 * 6.99)] * (v_color.r > 0.2 ? 1.2 : 1.0);
					float cover = smoothstep(0.25, 0.75, mask * (0.55 + 0.9 * d.b));
					albedo = mix(skin_tone, hair * mix(0.75, 1.2, d.b), cover);
					roughness = mix(0.6, 0.75, cover);
				} else {
					roughness = mix(roughness, roughness * (0.6 + 0.8 * d.a), min(amt, 1.0));
				}
			}
			albedo *= mix(1.0, fg_ao, 0.55);
			NORMAL = n;
		}
	}
	AO = fg_ao;
#endif
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
