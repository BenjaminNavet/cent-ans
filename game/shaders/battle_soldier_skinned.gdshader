// Figurines de bataille skinnées (lot V2, audit A1-18 / A1-07 / A1-01) : maillages Blender
// (personnages et chevaux Quaternius CC0 habillés en XIVe siècle) dont chaque sommet porte
// quatre os et leurs poids (CUSTOM0 = indices, CUSTOM1 = poids) ; les animations sont cuites
// dans une texture d'os (`bone_tex` : trois texels RGBA32F = lignes d'une matrice 3×4 par os,
// une ligne de texture par image, 24 i/s). Rendu par MultiMesh : aucun squelette côté moteur.
//
// Choix du clip : l'état du régiment fixe un jeu de clips (`clip_set`, jusqu'à 8) et un mode
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

#include "res://shaders/battle_bone_anim.gdshaderinc"

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
// NT7 : fondu entre deux clips successifs d'un cycle (mode CYCLE : mêlée, charge des lanciers),
// en secondes (`data/fx/battle_animation.json`, `cycle_blend_s`) ; 0 = changement sec. Le clip
// précédent continue au-delà de la fin du cycle (tenu sur sa dernière image s'il ne boucle pas)
// pendant que le suivant monte ; double lecture seulement dans la fenêtre et si le clip change.
uniform float cycle_blend = 0.0;
// Mode CYCLE (mêlée) : durée d'une passe ; mode VOLLEY (tir) : instant de la dernière volée,
// durée du rechargement, instant de la décoche dans le clip (s).
uniform float cycle_len = 1.5;
// NT10 : fondu au changement de clip du mode CUSTOM (porte-étendards, musiciens, servants).
// 0 : format historique. Sinon INSTANCE_CUSTOM.y = emplacement (0-31) + 32 × (indice du clip
// précédent dans `clips[]` + 1, 0 = aucun) + 4096 × q, q = instant du changement en 1/32 s
// modulo 64 s (le CPU efface le clip précédent une fois le fondu passé). Clip précédent lu à
// anim_time − x (1 : même phase, rôles) ou anim_time − z (2 : début du geste précédent,
// servants). z et w ne portent alors ni projection ni sang.
uniform int custom_fade = 0;
// NT10 : cuisson des imposteurs (`BattleImpostors`) : x ≥ 0 remplace l'identifiant haché (habit,
// teintes, visage) de toutes les figurines du matériau ; y ≥ 0 force la variante (casque...).
uniform vec2 bake_id = vec2(-1.0);
uniform float role_blend = 0.25;
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
// Lot FG5 : bande de distance (m, caméra → soldat) où ce calque dessine ses soldats. Hors de la
// bande, le soldat est replié en un point avant tout skinning : le calque principal d'une
// figurine fine dessine le LOD1 sur [r, ∞[, son calque LOD0 les soldats proches sur [0, r[.
instance uniform vec2 lod_band = vec2(0.0, 1.0e9);
// FG5 : calque LOD0 compacté (seulement les soldats proches) : le rang du soldat dans le régiment
// (hachage des variantes, visages, clips) est lu dans INSTANCE_CUSTOM.x au lieu d'INSTANCE_ID.
instance uniform bool id_in_custom = false;

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
// Lot OMR R5 (`data/fx/unit_looks.json`) : étoffes et robes propres au type d'unité, à la
// place des palettes communes PLAIN / COATS quand le compte est non nul.
uniform int plain_count = 0;
uniform vec3 plain_colors[4];
uniform int coat_count = 0;
uniform vec3 coat_colors[4];
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
// CR2 : occlusion des reflets de l'acier (0 : rendu SR2). Le ciel est le seul environnement
// reflété : sous ~20° de hauteur, où l'acier réel reflète sol, troupe et lisières, le reflet
// du voile d'horizon (clair) est assombri, et les creux cuits ne reflètent presque rien.
uniform float steel_occlusion = 1.0;
// CR4 : lance tenue (os `Prop` de l'index `prop_bone`, -1 : aucune) ; repère de repos de la
// prise (manifeste `prop_grip`/`prop_axis`/`prop_side`) : longueur ±10 % et angle ±3° par
// cavalier, flamme ondulante montrée pour `pennon_share` des cavaliers, fût veiné, prise usée.
uniform int prop_bone = -1;
uniform vec3 prop_grip = vec3(0.0);
uniform vec3 prop_axis = vec3(0.0, 0.0, 1.0);
uniform vec3 prop_side = vec3(1.0, 0.0, 0.0);
uniform float pennon_share = 0.6;

// Lot AN1a (ADR 0096) : mouvement secondaire (bas des surcots et jaques, caparaçons, crinières,
// queues), ajouté après le skinning, dans le repère posé de la figurine (+Z = avant). Poids par
// sommet sans donnée cuite : part des os des jambes (étoffes), hauteur de repos (caparaçon),
// rang dans la chaîne `Tail1-7` (queue), teinte exacte des crins (crinière). Moteurs : vitesse
// du régiment, vent de la bataille et rafales. Uniformes posés par BattleSecondaryMotion
// (`data/fx/atmosphere.json`, `secondary_motion`) ; éteint par défaut.
uniform bool sm_enabled = false;
uniform float sm_distance = 100.0;
uniform float move_speed = 0.0;
uniform float sm_speed_ref = 5.0;
uniform float sm_wind_scale = 1.0;
uniform vec2 wind_dir = vec2(1.0, 0.0);
uniform float wind_strength = 0.5;
uniform float wind_gust = 0.2;
// Par pièce (0 étoffe, 1 caparaçon, 2 crinière, 3 queue) : recul (m, à la vitesse de
// référence), vent (m, force 1), flottement (m), fréquence (Hz) ; évasement par pièce.
uniform vec4 sm_amp[4];
uniform vec4 sm_lift = vec4(0.0);
uniform vec2 sm_cap_band = vec2(0.48, 1.45);
uniform ivec2 sm_leg_bones = ivec2(-1, -2);
uniform ivec2 sm_tail_bones = ivec2(-1, -2);
uniform float sm_mane_tint = 0.3294;

#ifdef GA3_TEX
// GA3-L3 : albédo de la figurine générée (RGB) et masque teintable (A), partagé par ses LOD ;
// luminance linéaire moyenne des zones livrée (x) et étoffe (y) de la texture.
uniform sampler2D ga3_albedo : source_color, filter_linear_mipmap_anisotropic, repeat_disable;
uniform vec2 ga3_lum = vec2(0.2);
// GA3-L4 : hauteur de repos (m) du cou : au-dessus, les texels couleur chair prennent le teint
// du soldat (et perdent la rougeur de l'étalonnage).
uniform float ga3_head_y = 99.0;
#endif
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
const float FG3_TILE_SIZE[8] = float[](0.1, 0.024, 0.08, 0.12, 0.16, 0.3, 0.06, 0.05);
// Lot GA1 (ADR 0104) : matières générées (`data/art/materials.yaml`, même ordre, `tile_m` =
// GA1_TILE_SIZE). `fine_detail` reçoit alors `fine_detail_ga1.png` (RG normale, B relief,
// A rugosité absolue) et `fine_detail_albedo` un albédo de détail centré en luminance 0,5 qui
// multiplie (x2) la couleur de sommet : livrée et armoiries restent pilotées par les données.
// Sans les tableaux GA1 : tuiles FG3 d'origine (ga1_detail = false).
uniform sampler2DArray fine_detail_albedo : filter_linear_mipmap_anisotropic, repeat_enable;
uniform bool ga1_detail = false;
const int GA1_TILE_WOOL = 0;
const int GA1_TILE_LINEN = 1;
const int GA1_TILE_FUSTIAN = 2;
const int GA1_TILE_GAMBESON = 3;
const int GA1_TILE_MAIL = 4;
const int GA1_TILE_LEATHER = 5;
const int GA1_TILE_PLATE = 6;
const int GA1_TILE_WOOD = 7;
const int GA1_TILE_SKIN = 8;
const int GA1_TILE_HAIR = 9;
const int GA1_TILE_COAT_LIGHT = 10;
const int GA1_TILE_COAT_DARK = 11;
// SR1 : couches 0-7 = scans ambientCG CC0 à l'échelle physique (mailles ~9 mm, fils de 0,6-2 mm,
// piqûres de 4 cm, veinage de planche) ; tests tools/tests/test_material_gen.py, test_sr1_scans.py.
const float GA1_TILE_SIZE[12] = float[](0.15, 0.13, 0.16, 0.32, 0.51, 0.3, 0.3, 0.8, 0.06, 0.05, 0.1, 0.1);
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
#if defined(FG3_BAKED) || defined(GA3_TEX)
// Lot SR2 : usure (0 : rendu SR1 exact) posée par BattleSkinned ; hauteur de
// repos (m) jusqu'où monte la boue (piétons ~ genoux, cavaliers : jambes du cheval, caparaçon).
// GA3-L4 : aussi pour les figurines générées (variante GA3_TEX, sans atlas).
uniform float weathering = 0.85;
uniform float sr2_mud_height = 0.45;
#endif

varying vec3 v_color;
varying float v_code;
varying vec4 v_var;
varying vec3 v_rest;
varying vec3 v_rest_n;
varying float v_blood;
varying float v_stump;
// CR4 : sommet de la lance tenue (1) ; 0 ailleurs.
varying float v_prop;

float hash1(float n) {
	return fract(sin(n * 12.9898 + 4.1414) * 43758.5453);
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
		clip = id < float(split_count) || n < 2 ? pick_clip(set, 0) : pick_clip(set, 1 + min(int(h.x * float(n - 1)), n - 2));
		t = anim_time * speed * mix(0.93, 1.07, h.y) + h.z * 11.0;
	} else if (mode == M_CUSTOM) {
		clip = pick_clip(set, int(mod(mod(custom.y, 32.0), float(n))));
		t = anim_time - custom.x;
	} else if (mode == M_CYCLE) {
		float local = anim_time * mix(0.9, 1.1, h.y) + h.z * cycle_len;
		float cyc = floor(local / cycle_len);
		clip = pick_clip(set, min(int(hash1(h.x * 91.7 + cyc * 13.1) * float(n)), n - 1));
		t = mod(local, cycle_len);
	} else if (mode == M_VOLLEY) {
		clip = pick_clip(set, min(int(h.x * float(n)), n - 1));
		t = mod(anim_time - volley_time + release_at + (h.z - 0.5) * 0.4, max(reload_time, 0.5));
	} else {
		clip = pick_clip(set, min(int(h.x * float(n)), n - 1));
		t = anim_time * speed * mix(0.93, 1.07, h.y) + h.z * 11.0;
	}
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

// NT7 : clip du cycle précédent et poids du fondu (0 au début du cycle, 1 après `cycle_blend`).
// Retourne 1 (pas de fondu) hors fenêtre ou quand le tirage redonne le même clip.
float cycle_prev(ivec4 set, int size, vec4 h, int clip, out int pclip, out float pt) {
	pclip = clip;
	pt = 0.0;
	int n = max(size, 1);
	float local = anim_time * mix(0.9, 1.1, h.y) + h.z * cycle_len;
	float t = mod(local, cycle_len);
	if (cycle_blend <= 0.001 || n < 2 || t >= cycle_blend) {
		return 1.0;
	}
	float cyc = floor(local / cycle_len) - 1.0;
	pclip = pick_clip(set, min(int(hash1(h.x * 91.7 + cyc * 13.1) * float(n)), n - 1));
	if (pclip == clip) {
		return 1.0;
	}
	pt = cycle_len + t;
	return smoothstep(0.0, cycle_blend, t);
}

// Pose (lignes de la matrice de skinning) d'un soldat pour un jeu de clips et un mode, avec le
// fondu intra-cycle de NT7.
void anim_pose(int mode, ivec4 set, int size, float speed, vec4 h, vec4 custom, float id, ivec4 idx, vec4 w, bool smooth_frames, out int clip, out float t, out vec4 a, out vec4 b, out vec4 c) {
	choose(mode, set, size, speed, h, custom, id, clip, t);
	pose_rows(clip, t, idx, w, smooth_frames, a, b, c);
	if (mode == M_CYCLE) {
		int pclip;
		float pt;
		float k = cycle_prev(set, size, h, clip, pclip, pt);
		if (k < 1.0) {
			vec4 pa; vec4 pb; vec4 pc;
			pose_rows(pclip, pt, idx, w, smooth_frames, pa, pb, pc);
			a = mix(pa, a, k);
			b = mix(pb, b, k);
			c = mix(pc, c, k);
		}
	} else if (mode == M_CUSTOM && custom_fade > 0 && custom.y >= 32.0 && role_blend > 0.001) {
		// NT10 : fondu depuis le clip précédent (voir `custom_fade`).
		float packed = floor(custom.y / 32.0);
		int pclip = int(mod(packed, 128.0) + 0.5) - 1;
		float since = mod(anim_time - floor(custom.y / 4096.0) / 32.0, 64.0);
		float k = smoothstep(0.0, role_blend, since);
		if (pclip >= 0 && k < 1.0) {
			float pt = custom_fade == 2 ? anim_time - custom.z : t;
			vec4 pa; vec4 pb; vec4 pc;
			pose_rows(pclip, pt, idx, w, smooth_frames, pa, pb, pc);
			a = mix(pa, a, k);
			b = mix(pb, b, k);
			c = mix(pc, c, k);
		}
	}
}

// AN1a : poids (0-1, 1 au bord libre) et pièce souple d'un sommet ; -1 : pièce rigide.
float sm_weight(int code, ivec4 idx, vec4 w, vec3 rest, float tint, out int part) {
	part = -1;
	// Calculs vectoriels (quatre os d'un coup) : le sommet reste l'étape la plus chargée.
	if (code == C_LIVERY || code == C_CLOTH || code == C_QUILT) {
		// Bas du vêtement : part des os des jambes (0 à la taille, ~0,85 à l'ourlet) ; les chausses
		// (étoffe entièrement sur les jambes) restent rigides. Au LOD2, l'ourlet du surcot est
		// entièrement sur les jambes : gardé pour la livrée et le gambison.
		vec4 on = vec4(greaterThanEqual(idx, ivec4(sm_leg_bones.x))) * vec4(lessThanEqual(idx, ivec4(sm_leg_bones.y)));
		float legs = dot(w, on);
		part = 0;
		float hose = code == C_CLOTH ? smoothstep(0.93, 0.99, legs) : 0.0;
		return smoothstep(0.02, 0.85, legs) * (1.0 - hose);
	}
	if (sm_tail_bones.y < 0) {
		return 0.0;
	}
	if (code == C_ARMS) {
		// Caparaçon : pièce armoriée portée par les os du cheval (0 .. sever_bones[0].y).
		if (dot(w, vec4(lessThanEqual(idx, ivec4(sever_bones[0].y)))) > 0.5) {
			part = 1;
			float k = 1.0 - smoothstep(sm_cap_band.x, sm_cap_band.y, rest.y);
			return k * k;
		}
	} else if (code == C_COAT) {
		vec4 on = vec4(greaterThanEqual(idx, ivec4(sm_tail_bones.x))) * vec4(lessThanEqual(idx, ivec4(sm_tail_bones.y)));
		vec4 wt = w * on;
		float tail = wt.x + wt.y + wt.z + wt.w;
		if (tail > 0.5) {
			part = 3;
			float rank = dot(wt, vec4(idx - ivec4(sm_tail_bones.x)));
			float span = max(float(sm_tail_bones.y - sm_tail_bones.x), 1.0);
			return 0.25 + 0.75 * clamp(rank / (tail * span), 0.0, 1.0);
		}
		if (abs(tint - sm_mane_tint) < 0.002) {
			part = 2;
			return 1.0;
		}
	}
	return 0.0;
}

// AN1a : déplacement (repère posé) d'une pièce souple : recul dû à la marche (-Z), poussée du
// vent, flottement ; les faces tournées vers la poussée s'évasent, celles qui lui font face
// restent plaquées (pas de pénétration dans les jambes ou les flancs).
vec3 sm_offset(int part, float wt, vec3 rest, vec3 n, vec3 wind_local, float time, vec4 h) {
	vec4 a = sm_amp[part];
	float gust = sin(time * 0.9 + h.x * 6.283) * 0.6;
	float ws = clamp(wind_strength + wind_gust * gust, 0.0, 1.5) * sm_wind_scale;
	float sp = clamp(move_speed / max(sm_speed_ref, 0.01), 0.0, 1.5);
	vec3 push = vec3(0.0, 0.0, -a.x * sp) + wind_local * (a.y * ws);
	float plen = length(push);
	vec3 pdir = plen > 1e-4 ? push / plen : vec3(0.0, 0.0, -1.0);
	float phase = time * 6.283 * a.w * (0.8 + 0.5 * sp) + dot(rest, vec3(4.1, 5.3, 3.7)) + h.y * 6.283;
	float flut = sin(phase + 0.6 * sin(phase * 2.3 + 1.7)); // onde irrégulière, deux sinus
	float energy = clamp(0.25 + sp + ws, 0.0, 1.5);
	vec3 off;
	if (part <= 1) {
		// Étoffe, caparaçon : poussée selon l'orientation de la face, gonflement le long de la normale.
		float face = clamp(dot(n, pdir) * 0.5 + 0.5, 0.0, 1.0);
		face *= face;
		off = pdir * (plen * face) + n * (flut * a.z * energy * mix(0.4, 1.0, face));
		off.y += sm_lift[part] * plen * face;
	} else {
		// Crins : poussée pleine, flottement latéral.
		vec3 side = vec3(pdir.z, 0.0, -pdir.x); // cross(haut, pdir), pdir horizontal
		off = pdir * plen + side * (flut * a.z * energy);
		off.y += sm_lift[part] * plen;
	}
	return off * wt;
}

void vertex() {
	float inst_dist = length((MODELVIEW_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz);
	if (inst_dist < lod_band.x || inst_dist >= lod_band.y) {
		// FG5 : soldat hors de la bande de ce calque, replié en un point sans skinning.
		VERTEX = vec3(0.0);
		v_prop = 0.0;
	} else {
		float id = bake_id.x >= 0.0 ? bake_id.x : (id_in_custom ? INSTANCE_CUSTOM.x : float(INSTANCE_ID));
		vec4 h = vec4(hash1(id * 1.37 + 0.11), hash1(id * 2.71 + 0.53), hash1(id * 0.73 + 1.9), hash1(id * 3.91 + 2.3));
		vec4 custom = id_in_custom ? vec4(0.0) : INSTANCE_CUSTOM;
		v_rest = VERTEX;
		v_rest_n = NORMAL;

		// Variante du soldat : pièces absentes repliées en un point (triangles dégénérés).
		int mask = int(UV2.x + 0.5);
		int variant = bake_id.y >= 0.0 ? int(bake_id.y + 0.5) % max(variant_count, 1) : min(int(h.w * float(variant_count)), variant_count - 1);
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
		vec4 a; vec4 b; vec4 c;
		// Au loin, image la plus proche sans interpolation (moitié moins de lectures).
		bool smooth_frames = inst_dist < interp_distance;
		anim_pose(clip_mode, clip_set, clip_set_size, anim_speed, h, custom, id, idx, w, smooth_frames, clip, t, a, b, c);
		float blend = clamp((anim_time - blend_since) / max(blend_time, 0.01), 0.0, 1.0);
		if (blend < 1.0 && clip_mode != M_CUSTOM) {
			int pclip;
			float pt;
			vec4 pa; vec4 pb; vec4 pc;
			anim_pose(prev_mode, prev_set, prev_set_size, prev_speed, h, custom, id, idx, w, smooth_frames, pclip, pt, pa, pb, pc);
			float s = smoothstep(0.0, 1.0, blend);
			a = mix(pa, a, s);
			b = mix(pb, b, s);
			c = mix(pc, c, s);
		}
		v_prop = 0.0;
		if (prop_bone >= 0 && idx.x == prop_bone && w.x > 0.99) {
			// CR4 : lance propre au cavalier (repère de repos de la prise, avant le skinning).
			v_prop = 1.0;
			vec3 out_dir = normalize(cross(prop_axis, prop_side));
			vec3 r = VERTEX - prop_grip;
			float s_ax = dot(r, prop_axis);
			if (int(COLOR.a * 16.0 + 0.5) == C_LIVERY) {
				if (fract(h.z * 13.1 + h.x * 3.7) > pennon_share) {
					r = vec3(0.0); // pas de flamme pour ce cavalier
				} else {
					// Flamme : onde qui court vers la pointe libre, amplitude croissante.
					float d = length(r - prop_axis * s_ax);
					r += prop_side * (0.07 * d * sin(TIME * 7.0 - d * 11.0 + h.y * 6.283));
				}
			}
			float k_len = mix(0.9, 1.1, fract(h.x * 7.31 + h.z * 3.17));
			r += prop_axis * (max(s_ax - 0.3, 0.0) * (k_len - 1.0));
			// Angle : ±3° (baissée ou relevée), ±2° de côté, petits angles.
			float t1 = radians(3.0) * (fract(h.y * 5.7 + h.w * 1.3) * 2.0 - 1.0);
			float t2 = radians(2.0) * (fract(h.w * 9.1 + h.x * 2.3) * 2.0 - 1.0);
			r += t1 * cross(prop_side, r) + t2 * cross(out_dir, r);
			NORMAL = normalize(NORMAL + t1 * cross(prop_side, NORMAL) + t2 * cross(out_dir, NORMAL));
			VERTEX = prop_grip + r;
		}
		vec4 p = vec4(VERTEX, 1.0);
		vec3 v = vec3(dot(a, p), dot(b, p), dot(c, p));
		vec3 n = vec3(dot(a.xyz, NORMAL), dot(b.xyz, NORMAL), dot(c.xyz, NORMAL));
		if (sm_enabled && inst_dist < sm_distance) {
			// AN1a : mouvement secondaire (éteint sur les couches de poses figées : cadavres, renversés).
			int part;
			float wt = sm_weight(int(COLOR.a * 16.0 + 0.5), idx, w, VERTEX, COLOR.r, part);
			if (part >= 0 && wt > 0.001) {
				wt *= 1.0 - smoothstep(sm_distance * 0.7, sm_distance, inst_dist);
				// Vent dans le repère de la figurine (instance : rotation et échelle uniforme).
				vec3 wl = transpose(mat3(MODEL_MATRIX)) * vec3(wind_dir.x, 0.0, wind_dir.y);
				wl.y = 0.0;
				float wlen = length(wl);
				wl = wlen > 1e-4 ? wl / wlen : vec3(0.0);
				v += sm_offset(part, wt, VERTEX, n, wl, TIME, h);
			}
		}
		v *= mix(1.0 - size_jitter, 1.0 + size_jitter, h.y);
		float own_blood = 0.0;
		v_stump = 0.0;
		if (clip_mode == M_CUSTOM && custom_fade == 0) {
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
	#ifdef FK_TRAVEL
		// Lot FK3 (carte vivante) : marche le long d'un segment droit en +Z du modèle, entièrement
		// en shader. INSTANCE_CUSTOM = (longueur, vitesse, phase, dénivelé), unités du modèle ;
		// longueur nulle = sur place. Au bout, retour au départ, masqué par un rétrécissement.
		if (custom.x > 0.0) {
			float s = mod(anim_time * custom.y + custom.z, custom.x);
			float u = s / custom.x;
			v *= smoothstep(0.0, 0.08, u) * (1.0 - smoothstep(0.92, 1.0, u));
			v += vec3(0.0, custom.w * u, s);
		}
	#endif
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
	float lance_s = 0.0;
	if (v_prop > 0.5) {
		// CR4 : coordonnées du fût (tour, long de l'axe) : veine du bois le long de la lance.
		vec3 r = v_rest - prop_grip;
		lance_s = dot(r, prop_axis);
		vec3 rr = r - prop_axis * lance_s;
		q = vec2(atan(dot(rr, prop_side), dot(rr, normalize(cross(prop_axis, prop_side)))) * 0.03, lance_s);
	}
#ifdef FG3_BAKED
	if (code == C_COAT && v_src == 2) {
		// CR4 : poil couché vers l'arrière et le bas sur le tronc (tuile : poil le long de v),
		// droit vers le bas sur les jambes.
		float ang = 1.15 * smoothstep(0.75, 1.0, v_rest.y);
		float ca = cos(ang);
		float sa = sin(ang);
		q = mat2(vec2(ca, sa), vec2(-sa, ca)) * q;
	}
#endif
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
			vec3 plain = plain_count > 0 ? plain_colors[min(int(h6 * float(plain_count)), plain_count - 1)] : PLAIN[int(h6 * 7.99)];
			albedo = plain * mix(0.85, 1.15, v_var.z) * v_color * 1.6;
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
		vec3 coat = coat_count > 0 ? coat_colors[min(int(v_var.w * float(coat_count)), coat_count - 1)] : COATS[int(v_var.w * 4.99)];
		albedo = coat * v_color;
		roughness = 0.6;
		// CR4 : crins et extrémités noirs des bais et des isabelles ; bas des jambes plus foncé
		// pour toutes les robes (paturons), balzane blanche postérieure gauche sur un cheval sur
		// six. Le crin (hors robe) garde sa teinte de cuisson.
		bool black_points = (coat.r < 0.2 && coat.r > 0.06 && coat.g < 0.5 * coat.r) || (coat.g > 0.2 && coat.b < 0.55 * coat.g);
		float lower = 1.0 - smoothstep(0.3, 0.62, v_rest.y);
		bool is_body = abs(v_color.r - sm_mane_tint) > 0.003; // crins : teinte exacte 0,33
#ifdef FG3_BAKED
		if (v_src != 0) {
			is_body = v_src == 2;
		}
#endif
		if (!is_body && black_points) {
			albedo = vec3(0.028, 0.022, 0.018) * mix(0.8, 1.2, v_var.z);
		} else if (is_body) {
			albedo = mix(albedo, black_points ? vec3(0.03, 0.024, 0.02) : albedo * 0.55, lower * (black_points ? 0.9 : 0.6));
			if (fract(v_var.w * 37.3) > 0.83 && v_rest.x > 0.0 && v_rest.z < 0.0 && v_rest.y < 0.22) {
				albedo = mix(albedo, vec3(0.62, 0.6, 0.55), smoothstep(0.22, 0.16, v_rest.y));
			}
		}
	}
#ifdef GA3_TEX
	// GA3-L3 : figurine générée (albédo sur l'UV du maillage ; équipement procédural en u < 0,
	// non texturé). Alpha = zone teintable : livrée (C_LIVERY : livrée, armoiries, croix du jeu)
	// ou étoffe (C_CLOTH : couleur naturelle par soldat), modulées par la luminance de la
	// texture sur sa moyenne (plis, piqûres, crasse) ; ailleurs la couleur générée. Les faces
	// peintes de l'équipement (C_ARMS : écu, pavois) gardent l'UV d'armoiries (L3b).
	if (UV.x >= 0.0 && code != C_ARMS) {
		// UV exportées en convention Blender (v vers le haut, `ga3_figures.cam_uvs`) et PNG
		// enregistré de haut en bas : Godot lit la ligne 1 - v.
		vec4 g3 = texture(ga3_albedo, vec2(UV.x, 1.0 - UV.y));
		float g3_l = dot(g3.rgb, vec3(0.3, 0.59, 0.11));
		if (code == C_LIVERY) {
			albedo = mix(g3.rgb, albedo * clamp(g3_l / ga3_lum.x, 0.2, 2.2), g3.a);
		} else if (code == C_CLOTH) {
			vec3 hose = PLAIN[int(fract(h6 * 3.7 + v_var.w) * 7.99)] * mix(0.85, 1.15, v_var.z) * 1.5;
			albedo = mix(g3.rgb, hose * clamp(g3_l / ga3_lum.y, 0.2, 2.2), g3.a);
		} else {
			albedo = g3.rgb;
			// L4 : texels couleur chair de la tête (clairs, rouge > vert > bleu, moins saturés
			// que le cuir) : rougeur de l'étalonnage réduite, teint du soldat (même tirage que
			// les mains fines C_SKIN greffées, ramené autour du teint généré).
			if (v_rest.y > ga3_head_y) {
				float sat = (g3.r - g3.b) / max(g3.r, 1e-3);
				float skin = smoothstep(0.08, 0.16, g3_l) * smoothstep(0.03, 0.08, g3.r - g3.g)
					* smoothstep(0.0, 0.02, g3.g - g3.b) * (1.0 - smoothstep(0.8, 0.9, sat));
				vec3 calm = mix(g3.rgb, vec3(g3_l) * vec3(1.3, 0.93, 0.74), 0.4);
				vec3 tone = mix(vec3(1.1, 1.06, 1.0), vec3(0.62, 0.5, 0.4), h6) / vec3(0.93, 0.86, 0.79);
				albedo = mix(albedo, calm * tone, skin);
			}
		}
		// L4 : usure SR2 (bruit procédural seulement, rien à 0) : teintes passées, crasse par
		// taches en quantité propre au soldat, acier patiné (C_PLATE), boue montant des pieds.
		float sr_w = weathering * (1.0 - smoothstep(60.0, 80.0, dist));
		if (sr_w > 0.0) {
			bool dyed_cloth = code == C_LIVERY || code == C_CLOTH;
			float sr_l = dot(albedo, vec3(0.3, 0.59, 0.11));
			if (dyed_cloth) {
				albedo = mix(albedo, vec3(sr_l) * 1.08, 0.2 * sr_w);
			}
			float dirty = fract(v_var.x * 5.7 + v_var.w * 9.1);
			float blot = smoothstep(0.35, 0.8, noise2(q * 4.0 + v_var.xy * 23.0));
			float grime = (code == C_PLATE ? 0.5 : 1.0) * blot * mix(0.1, 0.55, dirty) * sr_w;
			albedo = mix(albedo, vec3(sr_l) * vec3(0.6, 0.54, 0.45), 0.6 * grime);
			roughness = mix(roughness, 0.9, grime * 0.5);
			if (code == C_PLATE) {
				// Acier forgé : rugosité par taches, gris sourd, film de crasse brune.
				float lf = noise2(q * 3.0 + v_var.xy * 17.0);
				roughness = mix(roughness, mix(0.3, 0.7, lf), sr_w);
				vec3 steel = vec3(0.46, 0.46, 0.46) * mix(0.85, 1.1, lf);
				albedo = mix(albedo, steel * (0.6 + 0.4 * dot(albedo, vec3(0.33))), 0.4 * sr_w);
				float gb = smoothstep(0.5, 0.85, noise2(q * 7.0 + v_var.zw * 29.0)) * mix(0.3, 0.8, dirty) * sr_w;
				albedo = mix(albedo, vec3(0.13, 0.11, 0.085), 0.6 * gb);
				roughness = mix(roughness, 0.85, gb);
				metallic = mix(metallic, 0.2, gb);
			}
			float sr_k = code == C_PLATE ? 0.45 : 1.0;
			float edge_n = noise2(v_rest.xz * 9.0 + v_var.xy * 13.0) * 0.7 + noise2(v_rest.xz * 31.0) * 0.3;
			float mud_top = sr2_mud_height * mix(0.75, 1.1, v_var.w);
			float rise = 1.0 - smoothstep(mud_top * 0.45, mud_top, v_rest.y + (edge_n - 0.5) * 0.14);
			float mud_blotch = mix(0.55, 1.0, smoothstep(0.25, 0.7, noise2(q * 9.0 + v_var.yz * 31.0)));
			float mud = rise * mud_blotch * sr_k * mix(0.35, 1.0, fract(v_var.z * 13.7 + v_var.w * 5.3)) * 0.85 * sr_w;
			vec3 mud_c = mix(vec3(0.16, 0.12, 0.08), vec3(0.30, 0.25, 0.18), smoothstep(0.0, 0.12, v_rest.y))
				* mix(0.8, 1.1, edge_n);
			albedo = mix(albedo, mud_c, mud);
			roughness = mix(roughness, 0.9, mud);
			metallic = mix(metallic, 0.0, mud);
		}
	}
#endif
#ifdef FG3_BAKED
	// Lot FG3 : cartes cuites (LOD0/LOD1 des figurines fines ; LOD2 : v_src = 0, rien de
	// plus). v_src est constant sur tout un maillage LOD0/LOD1 (1, 3 ou 2 pour le cheval, tous
	// non nuls) : les dérivées restent en flux uniforme ; lectures en textureGrad.
	float fg_ao = 1.0;
	float cr2_grime = 0.0;
	if (v_src != 0) {
		vec3 fg_dp1 = dFdx(VERTEX);
		vec3 fg_dp2 = dFdy(VERTEX);
		vec2 fg_da1 = dFdx(v_atlas);
		vec2 fg_da2 = dFdy(v_atlas);
		vec2 fg_dq1 = dFdx(q);
		vec2 fg_dq2 = dFdy(q);
		// SR2 : dérivées de la normale interpolée (courbure des arêtes d'acier), en flux uniforme.
		vec3 fg_dn1 = dFdx(NORMAL);
		vec3 fg_dn2 = dFdy(NORMAL);
		if (dist < fine_distance) {
			vec4 at;
			if (v_src == 2) {
				at = textureGrad(fine_horse, v_atlas, fg_da1, fg_da2);
			} else if (v_src == 1) {
				at = textureGrad(fine_atlas0, vec3(v_atlas, float(fine_layer)), fg_da1, fg_da2);
			} else {
				at = textureGrad(fine_atlas1, vec3(v_atlas, float(fine_layer)), fg_da1, fg_da2);
			}
			// Le v de la carte descend : le repère prend -v. Plates et garnitures : normale de
			// forme atténuée (le bruit de cuisson des pièces lisses ressortait en bosses).
			float shape = (code == C_PLATE || code == C_TRIM) ? 0.35 : 1.0;
			vec3 n = fg3_perturb(NORMAL, fg_dp1, fg_dp2, vec2(fg_da1.x, -fg_da1.y), vec2(fg_da2.x, -fg_da2.y), (at.rg * 2.0 - 1.0) * shape);
			float mask = at.a;
			if (v_src == 2) {
				// Cheval : relief de la robe (passe-haut de la couleur CC0) et occlusion fine.
				albedo *= mix(0.6, 1.4, at.b);
				fg_ao = at.a;
				mask = 1.0;
			} else {
				fg_ao = at.b;
			}
			if (code == C_PLATE || code == C_TRIM) {
				// Acier lisse : l'occlusion cuite (bruitée sur les grandes pièces) garde les creux
				// profonds seulement.
				fg_ao = mix(0.7, 1.0, smoothstep(0.25, 0.85, fg_ao));
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
				// CR2 : martelage lisible (reflets ondulés) ; plus fort sur les casques (masque).
				amt = mix(0.05, 0.22, mask) + 0.3 * steel_occlusion;
				relief = 0.06;
			} else if (code == C_TRIM) {
				tl = FG3_TILE_HAMMERED;
				amt = 0.1;
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
				// CR4 : crins du cheval (robe hors carte du cheval) traités comme des cheveux.
				bool mane = code == C_COAT && v_src != 2;
				amt = code == C_HAIR || mane ? 0.9 : 0.6;
				relief = code == C_HAIR ? 0.0 : (mane ? 0.35 : 0.3);
			}
			// GA1 : couche générée de la matière et part de l'albédo de détail (uniforme : flux
			// uniforme, aucune divergence entre figurines).
			int gl = -1;
			float ga_mix = 0.0;
			if (ga1_detail && tl >= 0) {
				if (code == C_MAIL) {
					gl = GA1_TILE_MAIL;
					ga_mix = 1.0;
				} else if (code == C_PLATE || code == C_TRIM) {
					gl = GA1_TILE_PLATE;
					// SR1 : 0,6 (GA1 0,45) pour que les taches et traces de lime du scan se voient.
					ga_mix = code == C_PLATE ? 0.6 : 0.3;
				} else if (code == C_LIVERY || code == C_ARMS) {
					gl = GA1_TILE_FUSTIAN;
					ga_mix = 0.6;
				} else if (code == C_CLOTH) {
					gl = h5 < 0.5 ? GA1_TILE_WOOL : GA1_TILE_LINEN;
					ga_mix = 0.8;
				} else if (code == C_QUILT) {
					gl = GA1_TILE_GAMBESON;
					ga_mix = 0.8;
				} else if (code == C_LEATHER) {
					gl = GA1_TILE_LEATHER;
					ga_mix = 0.85;
				} else if (code == C_WOOD) {
					gl = GA1_TILE_WOOD;
					ga_mix = 0.8;
				} else if (code == C_SKIN) {
					gl = GA1_TILE_SKIN;
					ga_mix = 0.45;
				} else if (code == C_HAIR) {
					gl = GA1_TILE_HAIR;
					ga_mix = 0.6;
				} else {
					// CR4 : tuile de poil couché pour toutes les robes (la claire est unie, sans
					// sens du poil) ; crins : tuile de cheveux.
					gl = v_src != 2 ? GA1_TILE_HAIR : GA1_TILE_COAT_DARK;
					ga_mix = v_src != 2 ? 0.7 : 0.45;
				}
			}
			if (tl >= 0) {
				int layer = gl >= 0 ? gl : tl;
				float size = gl >= 0 ? GA1_TILE_SIZE[gl] : FG3_TILE_SIZE[tl];
				vec3 tq = vec3(vec2(q.x, -q.y) / size, float(layer));
				vec2 tg1 = vec2(fg_dq1.x, -fg_dq1.y) / size;
				vec2 tg2 = vec2(fg_dq2.x, -fg_dq2.y) / size;
				vec4 d = textureGrad(fine_detail, tq, tg1, tg2);
				n = fg3_perturb(n, fg_dp1, fg_dp2, fg_dq1, fg_dq2, (d.rg * 2.0 - 1.0) * amt);
				vec3 ga_albedo = vec3(1.0);
				if (gl >= 0) {
					ga_albedo = mix(vec3(1.0), textureGrad(fine_detail_albedo, tq, tg1, tg2).rgb * 2.0, ga_mix);
					albedo *= ga_albedo;
				} else {
					albedo *= 1.0 + relief * (d.b * 2.0 - 1.0);
				}
				if (gl >= 0 && code == C_MAIL) {
					// GA1 : anneaux et creux portés par l'albédo généré ; rugosité de la tuile
					// (anneaux lisses ~0,35, creux plus rugueux).
					// CR2 : ×0,55 : l'ombre portée d'un anneau sur l'autre (le soleil, que l'AO ne
					// touche pas, blanchissait la maille comme un tricot).
					albedo = v_color * mix(0.85, 1.1, v_var.z) * ga_albedo * mix(1.0, 0.55, steel_occlusion);
					// CR4 : anneaux lisibles : creux (relief bas du scan) presque noirs, anneaux gris
					// fer ; la tuile seule, centrée en luminance, donnait un voile gris uniforme.
					albedo *= mix(1.0, mix(0.08, 0.95, smoothstep(0.38, 0.66, d.b)), steel_occlusion);
					metallic = 0.6;
					// CR2 : pas de reflet plus lisse que 0,5 : les anneaux se masquent l'un l'autre
					// (sinon voile blanc à contre-jour, halo du glow).
					roughness = max(d.a, mix(0.0, 0.5, steel_occlusion));
					fg_ao = mix(0.55, 1.0, fg_ao);
				} else if (code == C_MAIL) {
					// Mailles lisibles sans être noires : anneaux clairs, creux sombres, moins
					// métalliques que l'acier plein (pas de reflet du ciel dans la maille serrée).
					albedo = v_color * mix(0.85, 1.1, v_var.z) * mix(0.18, 1.2, d.b * d.b);
					metallic = 0.6;
					roughness = mix(0.7, 0.32, d.a);
					fg_ao = mix(0.55, 1.0, fg_ao);
				} else if (code == C_HAIR) {
					// Barbes et cheveux : couleur plus claire et variée ; la densité (masque) fond
					// les bords et la bouche dans la peau, les mèches cassent la lisière.
					vec3 hair = FG3_HAIR[int(h6 * 6.99)] * (v_color.r > 0.2 ? 1.2 : 1.0);
					float cover = smoothstep(0.25, 0.75, mask * (0.55 + 0.9 * d.b));
					albedo = mix(skin_tone, hair * mix(0.75, 1.2, d.b) * ga_albedo, cover);
					roughness = mix(0.6, 0.75, cover);
				} else if (gl >= 0) {
					// Rugosité absolue de la tuile générée.
					roughness = mix(roughness, d.a, min(amt, 1.0));
				} else {
					roughness = mix(roughness, roughness * (0.6 + 0.8 * d.a), min(amt, 1.0));
				}
			}
			// Lot SR2 : usure, sans aucune lecture de texture (bruit procédural seulement).
			// Fondue avant la limite des cartes (pas de saut au passage LOD1 → LOD2) ; tout est
			// multiplié par `sr_w` : weathering = 0 redonne exactement le rendu SR1.
			float sr_w = weathering * (1.0 - smoothstep(fine_distance * 0.75, fine_distance, dist));
			if (sr_w > 0.0) {
				// Étoffes teintes : livrée, tissu, armoiries, caparaçon (robe du cheval exclue).
				bool dyed_cloth = code == C_LIVERY || code == C_CLOTH || code == C_ARMS || (code == C_COAT && v_src != 2);
				bool fabric = dyed_cloth || code == C_QUILT;
				// 4. Teintes passées (teintures végétales, −15 %) éclaircies vers le gris de l'étoffe ;
				// la livrée lointaine (`dyed`, far_boost) n'est pas touchée.
				if (dyed_cloth) {
					float sr_l = dot(albedo, vec3(0.3, 0.59, 0.11));
					albedo = mix(albedo, vec3(sr_l) * 1.08, 0.25 * sr_w);
				}
				// 2. Crasse des creux : l'occlusion cuite assombrit et désature étoffe et cuir.
				if (fabric || code == C_LEATHER) {
					float cav = (1.0 - fg_ao) * sr_w;
					float sr_l = dot(albedo, vec3(0.3, 0.59, 0.11));
					albedo = mix(albedo, vec3(sr_l) * vec3(0.9, 0.85, 0.75), 0.6 * cav);
					albedo *= 1.0 - 0.35 * cav;
				}
				// 3. Acier vivant : rugosité des plates par taches (0,25-0,6) ; usure plus claire et
				// plus lisse sur les arêtes (courbure : dérivées de la normale / de la position).
				if (code == C_PLATE) {
					float lf = noise2(q * 3.0 + v_var.xy * 17.0);
					// CR2 : acier forgé mat mais pas plâtreux (0,3-0,5) : une rugosité de 0,45-0,75
					// fondait tout le ciel en un voile uniforme (le « plastique blanc »).
					float rough = clamp(mix(mix(0.45, 0.2, steel_occlusion), mix(0.75, 0.42, steel_occlusion), lf)
						+ (roughness - 0.4) * mix(0.4, 0.2, steel_occlusion), mix(0.4, 0.16, steel_occlusion), 0.8);
					roughness = mix(roughness, rough, sr_w);
					// Acier forgé : gris sourd (F0 réel ≈ 0,55), pas le blanc chromé du rendu FG.
					vec3 steel = mix(vec3(0.50, 0.50, 0.52), vec3(0.42, 0.415, 0.40), steel_occlusion) * mix(0.85, 1.1, lf);
					albedo = mix(albedo, steel * (0.6 + 0.4 * dot(albedo, vec3(0.33))), 0.7 * sr_w);
					// Crasse et graisse dans les creux de l'armure.
					albedo *= mix(1.0, mix(0.55, 1.0, fg_ao), sr_w);
					// CR2 : film de crasse (graisse, poussière brune) dans les creux et par plaques :
					// ni métallique ni lisse, il casse le reflet uniforme.
					float gb = smoothstep(0.5, 0.85, noise2(q * 7.0 + v_var.zw * 29.0));
					cr2_grime = clamp((1.0 - smoothstep(0.5, 0.95, fg_ao)) * 0.8 + gb * 0.35, 0.0, 1.0) * sr_w * steel_occlusion;
					albedo = mix(albedo, vec3(0.13, 0.11, 0.085), 0.75 * cr2_grime);
					roughness = mix(roughness, 0.85, cr2_grime);
					metallic = mix(metallic, 0.2, cr2_grime);
					float curv = (length(fg_dn1) + length(fg_dn2)) / max(length(fg_dp1) + length(fg_dp2), 1e-5);
					float wear = smoothstep(0.55, 0.85, noise2(q * 55.0 + 3.1)) * smoothstep(12.0, 50.0, curv);
					albedo *= 1.0 + 0.25 * wear * sr_w;
					roughness -= 0.12 * wear * sr_w;
				} else if (code == C_MAIL) {
					// Légère rouille brune dans le bas des mailles (sous la taille).
					float low = 1.0 - smoothstep(sr2_mud_height * 1.6, sr2_mud_height * 2.4, v_rest.y);
					float ox = low * smoothstep(0.45, 0.8, noise2(q * 6.0 + v_var.zw * 23.0)) * 0.5 * sr_w;
					albedo = mix(albedo, vec3(0.20, 0.10, 0.045), ox);
					roughness = mix(roughness, 0.8, ox);
					metallic = mix(metallic, 0.3, ox);
				}
				// 1. Boue et poussière montant des pieds (bord bruité), quantité propre au soldat ;
				// forte sur étoffe et cuir, légère sur l'acier, aucune sur les cheveux.
				float sr_k = fabric || code == C_LEATHER ? 1.0
					: (code == C_COAT ? 0.9 : (code == C_WOOD || code == C_SKIN ? 0.7
					: (code == C_PLATE || code == C_MAIL || code == C_TRIM ? 0.45 : 0.0)));
				float edge_n = noise2(v_rest.xz * 9.0 + v_var.xy * 13.0) * 0.7 + noise2(v_rest.xz * 31.0) * 0.3;
				float mud_top = sr2_mud_height * mix(0.75, 1.1, v_var.w);
				float rise = 1.0 - smoothstep(mud_top * 0.45, mud_top, v_rest.y + (edge_n - 0.5) * 0.14);
				float blotch = mix(0.55, 1.0, smoothstep(0.25, 0.7, noise2(q * 9.0 + v_var.yz * 31.0)));
				float mud = rise * blotch * sr_k * mix(0.35, 1.0, fract(v_var.z * 13.7 + v_var.w * 5.3)) * 0.85 * sr_w;
				// Terre sèche, plus sombre (humide) au ras du sol.
				vec3 mud_c = mix(vec3(0.16, 0.12, 0.08), vec3(0.30, 0.25, 0.18), smoothstep(0.0, 0.12, v_rest.y))
					* mix(0.8, 1.1, edge_n);
				albedo = mix(albedo, mud_c, mud);
				roughness = mix(roughness, 0.9, mud);
				metallic = mix(metallic, 0.0, mud);
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
	if (v_prop > 0.5 && code == C_WOOD) {
		// CR4 : prise usée (bois noirci et lustré par les gantelets) de la poignée à la rondelle.
		float grip = smoothstep(-0.34, -0.22, lance_s) * (1.0 - smoothstep(0.06, 0.14, lance_s));
		albedo = mix(albedo, albedo * vec3(0.42, 0.36, 0.3), grip);
		roughness = mix(roughness, 0.42, grip);
	}
	float ao = mix(0.6, 1.0, smoothstep(0.0, 1.2, v_rest.y));
	ALBEDO = albedo * ao * (1.0 + highlight * 0.25);
	METALLIC = metallic;
	ROUGHNESS = clamp(roughness, 0.05, 1.0);
	SPECULAR = specular;
	if (steel_occlusion > 0.0 && code == C_MAIL) {
		// CR4 : maille rasante (contre-jour) : les anneaux se masquent, plus de reflet de
		// Fresnel (f90 de Godot suit f0 quand le métal et la réflectance tombent) : anneaux
		// sombres au lieu d'un voile blanc « fourrure ».
		float graz = 1.0 - smoothstep(0.12, 0.55, abs(dot(NORMAL, VIEW)));
		METALLIC = mix(METALLIC, 0.0, graz * steel_occlusion);
		SPECULAR = mix(SPECULAR, 0.08, graz * steel_occlusion);
	}
	if (steel_occlusion > 0.0 && (code == C_PLATE || code == C_TRIM || code == C_MAIL)) {
		// CR2 : occlusion spéculaire. RADIANCE (a) mêle le reflet du ciel au noir : le reflet
		// n'atteint le ciel libre qu'en montant (troupe, sol, lisières et bannières, absents du
		// ciel, bouchent l'horizon), et presque pas dans les creux cuits. Transition élargie
		// avec la rugosité.
		vec3 r_view = reflect(-VIEW, NORMAL);
		float ry = (INV_VIEW_MATRIX * vec4(r_view, 0.0)).y;
		float w = 0.3 * ROUGHNESS;
		float sky_seen = smoothstep(0.02 - w, 0.62 + w, ry);
		float occ = mix(0.12, 1.0, sky_seen * sky_seen);
		if (code == C_MAIL) {
			// Anneaux entrelacés : chacun cache une bonne part du ciel à ses voisins.
			occ *= 0.5;
		}
#ifdef FG3_BAKED
		occ *= mix(0.35, 1.0, smoothstep(0.35, 0.95, AO));
#endif
		RADIANCE = vec4(vec3(0.0), (1.0 - occ) * steel_occlusion);
	}
	if (livery_surface && far > 0.0) {
		EMISSION = dyed * far * far_boost;
	}
}
