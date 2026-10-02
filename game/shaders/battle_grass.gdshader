// Herbe des batailles (lot V4) : touffes en cartes croisées d'un MultiMesh en grille qui suit le
// regard (nœud déplacé par pas entiers de la grille). La position réelle de chaque touffe est
// dérivée de sa cellule monde (hachage) : l'herbe ne « glisse » pas quand la grille se déplace.
// Hauteur lue dans la carte de hauteurs du terrain ; densité selon la splatmap (pas d'herbe sur
// les chemins, la boue, les galets, les labours, sous l'eau), les parcelles (blé doré, jachère
// haute) et la neige. Balancement au vent dans le sommet, fondu au bord du disque.
shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_disabled, depth_draw_opaque, alpha_to_coverage;

#include "res://shaders/battle_common.gdshaderinc"

uniform sampler2D grass_texture : source_color, filter_linear_mipmap, repeat_disable;
uniform sampler2DArray albedo_array : source_color, filter_linear_mipmap, repeat_enable;
uniform sampler2D macro_noise : filter_linear_mipmap, repeat_enable;
uniform sampler2D height_map : filter_linear, repeat_disable;
uniform vec4 height_rect = vec4(-400.0, -400.0, 2010.0, 1610.0);
uniform sampler2D splat_a : filter_linear, repeat_disable;
uniform sampler2D splat_b : filter_linear, repeat_disable;
uniform vec4 splat_rect = vec4(-400.0, -400.0, 2000.0, 1600.0);
uniform vec4 calm_rect = vec4(150.0, 60.0, 1050.0, 740.0);
// EP6 : parcelles du décor (voir `bt_decor_field`).
uniform sampler2D decor_fields : filter_nearest, repeat_disable;
uniform float decor_on = 0.0;
uniform float spacing = 0.9;
uniform float radius = 80.0;
uniform float inner_radius = 0.0; // couche lointaine : vide au centre (couvert par la proche)
uniform float blade_scale = 1.0;
uniform float snow = 0.0;
uniform vec3 base_tint : source_color = vec3(1.0);
uniform vec2 wind_dir = vec2(0.8, 0.6);
uniform float wind_strength = 1.0;
uniform vec3 focus = vec3(0.0);
// BV3 : herbe couchée (R) et tachée de sang (G), carte `BattleGrassFlatten`.
uniform sampler2D flatten_map : filter_linear, repeat_disable;
uniform vec4 flatten_rect = vec4(0.0, -100.0, 1200.0, 1000.0);
uniform float flatten_on = 0.0;
uniform vec3 grass_blood_color : source_color = vec3(0.2, 0.02, 0.02);
// DA6 : lisières douces (touffes mêlées au bord des parcelles), touffes en volume (cartes cintrées,
// normales arrondies portées par le maillage, normale de carte dans COLOR), cartes vues par la
// tranche amincies, pied assombri. `--no-da6` : 0 (ancien rendu).
uniform float da6_on = 0.0;
uniform float decor_saturation = 0.6; // DA6 : même désaturation que le sol
uniform float tex_lum = 0.14; // luminance moyenne (linéaire) de la carte, ramenée à ~0,6
// FA7 : atlas de touffes faites de vrais brins (`grass_tufts.png`, `data/art/battle_grass.json`).
// Chaque carte de la touffe tire sa case (hachage de la cellule monde) ; l'herbe rase comble les
// vides entre les bouquets ; clarté et teinte tenues près de celles du sol. `--no-fa-grass` : 0.
const int FA_MAX_VARIANTS = 16;
uniform float fa_on = 0.0;
uniform vec2 fa_grid = vec2(4.0, 2.0); // cases de l'atlas
uniform vec2 fa_pad = vec2(0.0); // marge d'une case (fraction de case)
uniform int fa_count = 8;
uniform float fa_height[FA_MAX_VARIANTS]; // hauteur de la touffe (m)
uniform float fa_cum_open[FA_MAX_VARIANTS]; // poids cumulés hors des bouquets
uniform float fa_cum_clump[FA_MAX_VARIANTS]; // poids cumulés au cœur des bouquets
uniform ivec2 fa_wheat = ivec2(0); // cases des parcelles de blé
uniform ivec2 fa_stubble = ivec2(0); // cases du chaume et des semis
uniform vec4 fa_field_height = vec4(1.3, 1.2, 1.0, 0.6); // blé, jachère, chaume, semis
uniform vec2 fa_size = vec2(0.8, 1.25); // taille d'une touffe (tirage)
uniform float fa_height_var = 0.5; // part gardée des variations de hauteur par plaques
uniform float fa_gap_fill = 0.9; // densité gardée entre les bouquets
uniform float fa_patch_fill = 0.5; // densité gardée dans les trouées à ~25 m
uniform vec3 fa_tint_gain = vec3(1.0); // couleur du sol → couleur de l'herbe
uniform float fa_tint_var = 0.5; // part gardée des écarts de teinte entre touffes
uniform float fa_hue_mix = 0.6; // part de la teinte propre des brins (0 = luminance seule)
uniform vec2 fa_luma_clamp = vec2(0.55, 2.0);
uniform float fa_contrast = 0.8; // contraste clair/sombre des brins autour de la moyenne
uniform float fa_foot_shade = 0.85; // pied de touffe (1 = pas d'assombrissement)
uniform float fa_foot_height = 0.4; // part de la carte où le pied s'éclaircit
uniform float fa_up_normal = 0.6; // normale ramenée vers la verticale (éclairage du sol)
uniform float fa_far_luma = 1.0; // clarté vers laquelle tendent les touffes lointaines
uniform float fa_mip_boost = 0.25; // alpha relevé par niveau de mipmap
uniform float fa_backlight = 1.0;

varying float v_shade;
varying vec3 v_tint;
varying vec3 v_normal;
varying float v_facing;
varying flat vec2 v_cell;

float terrain_h(vec2 p) {
	vec2 uv = (p - height_rect.xy) / height_rect.zw;
	return texture(height_map, uv).r;
}

void vertex() {
	vec3 mesh_normal = NORMAL;
	vec3 origin = MODEL_MATRIX[3].xyz;
	vec2 cell = floor(origin.xz / spacing + 0.5);
	vec2 rnd = bt_hash22(cell);
	float r3 = bt_hash21(cell + 31.7);
	vec2 wp = (cell + rnd - 0.5) * spacing;
	float dist = length(wp - focus.xz);
	float fade = (1.0 - smoothstep(radius * 0.6, radius, dist)) * smoothstep(inner_radius * 0.75, inner_radius, dist);
	// Touffes clairsemées par endroits (bruit) : test bon marché avant les lectures de textures.
	float density = smoothstep(0.15, 0.45, bt_value_noise(wp * 0.05)) * (1.0 - snow * 0.93);
	// PO4 : touffes groupées (DA6) : plaques à ~25 m avec des trouées franches, et dans les
	// plaques des bouquets de ~2 m plus ou moins serrés, au lieu de taches régulières.
	float clump = 0.5;
	if (da6_on > 0.5) {
		float patches = bt_value_noise(wp * 0.04 + 5.3) * 0.7 + bt_value_noise(wp * 0.11 + 1.9) * 0.3;
		clump = bt_value_noise(wp * 0.55 + 7.1);
		density = smoothstep(0.08, 0.38, patches) * mix(0.62, 1.0, smoothstep(0.3, 0.72, clump)) * (1.0 - snow * 0.93);
		if (fa_on > 0.5) {
			// FA7 : nappe continue, les vides deviennent de l'herbe rase (tirage des cases).
			density = mix(fa_patch_fill, 1.0, smoothstep(0.08, 0.38, patches)) * mix(fa_gap_fill, 1.0, smoothstep(0.3, 0.72, clump)) * (1.0 - snow * 0.93);
		}
	}
	// VN : dans les parcelles de blé et de chaume du décor, tiges serrées partout (sinon ils ne poussaient que dans
	// les plaques d'herbe : blé en ovales, Crécy, Poitiers).
	// Seulement pour les touffes qu'on allait écarter (une lecture de texture, sans déformation).
	if (decor_on > 0.5 && fade > 0.0 && r3 >= density) {
		vec2 duv0 = (wp - splat_rect.xy) / splat_rect.zw;
		vec4 df0 = texture(decor_fields, duv0);
		float kind0 = floor(df0.r * 8.0 + 0.5);
		if (df0.a > 0.5 && (kind0 == 2.0 || kind0 == 6.0)) {
			density = mix(density, 0.92, smoothstep(0.0, 0.5, df0.b));
		}
	}
	v_tint = vec3(0.0);
	v_shade = 0.0;
	v_normal = vec3(0.0, 1.0, 0.0);
	v_facing = 1.0;
	v_cell = vec2(0.0);
	bool fa = fa_on > 0.5;
	if (fade <= 0.0 || r3 >= density) {
		// Touffe absente : triangle dégénéré sous le sol (aucun pixel).
		VERTEX = vec3(0.0, -500.0, 0.0);
	} else {
		// Densité : zones de sol nu et parcelles.
		vec2 suv = (wp - splat_rect.xy) / splat_rect.zw;
		vec4 sa = texture(splat_a, suv);
		sa.b = texture(splat_a, suv + bt_mud_warp(wp) / splat_rect.zw).b; // VN2 : comme le sol
		vec4 sb = texture(splat_b, suv);
		density *= 1.0 - smoothstep(0.2, 0.5, max(sa.g, sa.a));
		// VN2 : boue en partie herbue, plus encore au bord de l'eau (vue de haut : tache sombre sinon).
		density *= 1.0 - smoothstep(0.2, 0.6, sa.b) * mix(0.72, 0.45, sb.r);
		density *= 1.0 - smoothstep(0.3, 0.7, sb.r) * 0.35; // VN2 : berges herbues
		density *= mix(1.0, 0.45, smoothstep(0.3, 0.8, sa.r)); // sous-bois : clairsemé
		vec2 calm_d = max(calm_rect.xy - wp, wp - calm_rect.zw);
		float outside_calm = smoothstep(0.0, 60.0, max(calm_d.x, calm_d.y));
		bool soft = da6_on > 0.5;
		vec3 field = soft ? bt_field_info_soft(wp) : bt_field_info(wp);
		float height_mul = 1.0;
		int fa_kind = 0; // 1 : blé, 2 : chaume et semis
		// Couleur de base : celle du sol dessous (couches herbe et prairie, très floues), pour que
		// l'herbe se fonde dans la texture du terrain.
		float macro2 = texture(macro_noise, wp * 0.00043 + vec2(0.3, 0.7)).r;
		vec3 ground = mix(textureLod(albedo_array, vec3(wp / 5.0, 0.0), 7.0).rgb, textureLod(albedo_array, vec3(wp / 7.0, 1.0), 7.0).rgb, smoothstep(0.25, 0.55, macro2));
		v_tint = mix(base_tint * ground * vec3(1.12, 1.18, 1.0), vec3(0.45, 0.45, 0.4), snow * 0.6);
		// Taches de prairie (mêmes formules que `battle_ground.gdshader`, lot V4b).
		float patch = texture(macro_noise, wp * 0.0013 + vec2(0.61, 0.13)).r;
		float patch2 = texture(macro_noise, wp * 0.0047 + vec2(0.27, 0.83)).r;
		float dry = smoothstep(0.42, 0.66, patch * 0.7 + patch2 * 0.45);
		float lush = smoothstep(0.45, 0.7, (1.0 - patch) * 0.75 + patch2 * 0.35);
		v_tint = mix(v_tint, v_tint * (soft ? vec3(0.9, 0.85, 0.6) : vec3(1.35, 1.12, 0.62)), dry * 0.85);
		// VN1 : prairie grasse plus verte mais pas plus sombre que le sol (touffes « détritus »).
		v_tint = mix(v_tint, v_tint * (soft ? vec3(0.86, 1.0, 0.8) : vec3(0.7, 0.86, 0.66)), lush * 0.8);
		height_mul *= mix(1.0, 1.25, lush) * mix(1.0, 0.8, dry);
		float in_field = outside_calm * (1.0 - sb.g) * field.z;
		if (decor_on > 0.5) {
			vec2 duv = soft ? (wp + bt_edge_warp(wp) - splat_rect.xy) / splat_rect.zw : suv;
			vec4 df = texture(decor_fields, duv);
			if (df.a > 0.5) {
				field = bt_decor_field(df, wp);
				in_field = soft ? bt_decor_edge_soft(field.z, wp) : field.z;
				if (field.x == 4.0) {
					// Semis : herbe courte et claire en rangs.
					density *= 0.55;
					height_mul *= fa ? fa_field_height.w : 0.45;
					fa_kind = 2;
					field.x = 0.0;
				} else if (field.x == 5.0) {
					// Vigne : herbe rase entre les rangs.
					density *= 1.0 - in_field * 0.7;
					field.x = 0.0;
				} else if (field.x == 6.0) {
					// Chaume : tiges courtes et blondes.
					height_mul *= fa ? fa_field_height.z : 0.5;
					fa_kind = in_field > 0.5 ? 2 : 0;
					v_tint = mix(v_tint, soft ? vec3(0.6, 0.5, 0.3) : vec3(1.2, 1.02, 0.62), in_field * 0.8);
					field.x = 0.0;
				}
			}
		}
		if (soft) {
			// Chaque touffe bascule à son propre seuil : au bord, les touffes des deux côtés se
			// mêlent sur quelques mètres au lieu d'un fondu uniforme (ou d'une ligne).
			float r_edge = bt_hash21(cell + 71.3) * 0.8 + 0.1;
			in_field = mix(in_field, smoothstep(r_edge - 0.18, r_edge + 0.18, in_field), 0.8);
		}
		if (field.x == 1.0) {
			density *= 1.0 - in_field;
		} else if (field.x == 2.0) {
			height_mul = mix(1.0, fa ? fa_field_height.x : 2.1, in_field);
			fa_kind = in_field > 0.5 ? 1 : 0;
			v_tint = mix(v_tint, soft ? vec3(0.4, 0.33, 0.17) : vec3(0.5, 0.36, 0.11), in_field);
		} else if (field.x == 3.0) {
			height_mul = mix(1.0, fa ? fa_field_height.y : 1.4, in_field);
			// DA6 : jachère herbeuse (vert passé, fleurs rares) et non plus crème vif.
			v_tint = soft ? mix(v_tint, v_tint * vec3(1.02, 1.0, 0.82), in_field) : mix(v_tint, vec3(1.25, 1.1, 0.7), in_field * 0.7);
		}
		height_mul *= 1.0 - snow * 0.5;
		// BV3 : herbe couchée par les troupes et sous les corps, sang sur les brins.
		float flat_k = 0.0;
		float blood_k = 0.0;
		if (flatten_on > 0.0) {
			vec2 fuv = (wp - flatten_rect.xy) / flatten_rect.zw;
			// NT10 : rien hors du rectangle couvert (sans cela, le texel du bord s'étire).
			vec2 inside = step(vec2(0.0), fuv) * step(fuv, vec2(1.0));
			vec2 fl = texture(flatten_map, fuv).rg * inside.x * inside.y;
			// Bords irréguliers (bruit à ~1,5 m) et touffes plus ou moins couchées.
			float ragged = bt_value_noise(wp * 0.7) - 0.5;
			flat_k = smoothstep(0.08, 0.9, fl.r * (1.0 + ragged * 0.6) + (rnd.x - 0.5) * 0.25 * fl.r);
			blood_k = smoothstep(0.05, 0.6, fl.g) * smoothstep(-0.45, 0.1, ragged + fl.g * 0.5);
			density *= 1.0 - flat_k * 0.3;
			height_mul *= mix(1.0, 0.22, flat_k);
			v_tint = mix(v_tint, v_tint * vec3(1.08, 1.0, 0.62), flat_k * 0.6);
			v_tint = mix(v_tint, grass_blood_color, blood_k * mix(0.55, 0.9, r3));
		}
		float keep = step(r3, density);
		float s = (0.6 + 0.8 * rnd.x) * blade_scale * fade * keep;
		if (soft) {
			// Hauteur : variation propre à la touffe et par plaques (~6 m).
			float h_var = mix(0.7, 1.3, bt_value_noise(wp * 0.17 + 2.3)) * mix(0.85, 1.15, r3);
			// PO4 : chaque bouquet a sa hauteur et sa teinte (vert frais au cœur des bouquets
			// serrés, brins plus secs et plus courts aux bords clairsemés).
			h_var *= mix(0.8, 1.2, clump);
			vec3 t_var = mix(vec3(1.07, 1.03, 0.86), vec3(0.93, 1.0, 0.92), clump) * mix(0.94, 1.06, bt_hash21(floor(wp * 0.55) + 13.7));
			height_mul *= fa ? mix(1.0, h_var, fa_height_var) : h_var;
			v_tint *= fa ? mix(vec3(1.0), t_var, fa_tint_var) : t_var;
		}
		float h_units = 1.0; // hauteur de la carte en unités de l'ancienne touffe (0,44 m)
		if (fa) {
			// FA7 : case de l'atlas propre à chaque carte de la touffe (UV2.x = rang de la carte),
			// tirée selon le bouquet (herbe rase dans les vides, hautes herbes au cœur) ; cartes
			// retournées une fois sur deux.
			float pick = bt_hash21(cell * 1.31 + 91.7 + UV2.x * 17.3);
			float flip = bt_hash21(cell + 47.9 + UV2.x * 5.1);
			float in_clump = smoothstep(0.3, 0.72, clump);
			int idx = fa_count - 1;
			for (int i = 0; i < fa_count; i++) {
				if (pick < mix(fa_cum_open[i], fa_cum_clump[i], in_clump)) {
					idx = i;
					break;
				}
			}
			if (fa_kind == 1) {
				idx = flip < 0.5 ? fa_wheat.x : fa_wheat.y;
			} else if (fa_kind == 2) {
				idx = flip < 0.5 ? fa_stubble.x : fa_stubble.y;
			}
			v_cell = vec2(float(idx % int(fa_grid.x)), float(idx / int(fa_grid.x)));
			if (fract(flip * 7.0) < 0.5) {
				UV.x = 1.0 - UV.x;
			}
			s = mix(fa_size.x, fa_size.y, rnd.x) * blade_scale * fade * keep;
			h_units = fa_height[idx] / 0.44;
			v_tint *= fa_tint_gain;
		}

		float angle = rnd.y * 6.2832;
		mat2 rot = mat2(vec2(cos(angle), sin(angle)), vec2(-sin(angle), cos(angle)));
		vec3 v = VERTEX;
		v.xz = rot * v.xz * s;
		v.y *= s * height_mul * (fa ? h_units * 0.44 : 1.0);
		// Vent : le haut de la touffe (UV.y = 0) plie, rafales qui traversent le champ.
		float top = 1.0 - UV.y;
		float gust = sin(dot(wp, wind_dir) * 0.08 - TIME * 1.7) * 0.5 + 0.5;
		float sway = sin(TIME * 2.3 + rnd.x * 6.2832 + wp.x * 0.3) * 0.12 + gust * 0.28;
		v.xz += wind_dir * sway * top * top * wind_strength * s * height_mul * min(h_units, 1.6) * (1.0 - flat_k * 0.8);
		// Brins couchés : le haut de la touffe s'étale au ras du sol (sens propre à la touffe).
		v.xz += vec2(cos(angle * 1.7), sin(angle * 1.7)) * top * 0.42 * s * flat_k;
		vec3 world = vec3(wp.x, terrain_h(wp) - 0.05, wp.y) + v;
		VERTEX = world - origin;
		NORMAL = normalize(vec3(0.0, 1.0, 0.0) + vec3(wind_dir.x, 0.0, wind_dir.y) * sway * 0.3);
		if (soft) {
			// Normale arrondie du maillage tournée avec la touffe ; carte vue par la tranche
			// (vue rasante) : amincie par le seuil d'alpha dans `fragment`.
			vec3 n = mesh_normal;
			n.xz = rot * n.xz;
			v_normal = normalize(n + vec3(wind_dir.x, 0.0, wind_dir.y) * sway * 0.3);
			if (fa) {
				v_normal = normalize(mix(v_normal, vec3(0.0, 1.0, 0.0), fa_up_normal));
			}
			vec3 card = COLOR.rgb * 2.0 - 1.0;
			card.xz = rot * card.xz;
			vec3 to_cam = CAMERA_POSITION_WORLD - vec3(wp.x, 0.0, wp.y);
			to_cam.y = CAMERA_POSITION_WORLD.y - terrain_h(wp);
			vec3 vdir = normalize(to_cam);
			float face_h = abs(dot(normalize(card.xz + 1e-4), normalize(vdir.xz + 1e-4)));
			v_facing = max(face_h, abs(vdir.y) * 1.3);
		}
		v_shade = mix(0.85, 1.12, rnd.x) * mix(0.9, 1.05, gust);
		vec3 r_tint = mix(vec3(0.95, 1.0, 0.9), vec3(1.08, 1.03, 0.9), r3);
		if (fa) {
			v_shade = mix(1.0, v_shade, fa_tint_var);
			r_tint = mix(vec3(1.0), r_tint, fa_tint_var);
		}
		v_tint *= r_tint;
	}
}

void fragment() {
	// Normale « verticale » pour toutes les faces (pas d'assombrissement des faces arrière).
	NORMAL = normalize((VIEW_MATRIX * vec4(da6_on > 0.5 ? v_normal : vec3(0.0, 1.0, 0.0), 0.0)).xyz);
	bool fa = fa_on > 0.5;
	vec2 uv = fa ? (v_cell + fa_pad + clamp(UV, 0.0, 1.0) * (1.0 - 2.0 * fa_pad)) / fa_grid : UV;
	vec4 tex = texture(grass_texture, uv);
	// La carte ne donne que la découpe et les nuances claires/sombres des brins ; la couleur
	// vient du sol (v_tint) pour que l'herbe prolonge la texture du terrain.
	float l = dot(tex.rgb, vec3(0.3, 0.59, 0.11)) / tex_lum;
	// VN1 : au-delà de ~30 m, contraste des brins et pied sombre atténués : vue de haut, les
	// touffes lointaines faisaient un moucheté sombre (« détritus ») sur le sol plus clair.
	float far_k = smoothstep(22.0, 70.0, length(VERTEX)) * da6_on;
	l = mix(l, 1.05, far_k * 0.65);
	vec3 hue = tex.rgb / max(dot(tex.rgb, vec3(0.333)), 0.001) * 0.333;
	ALBEDO = v_tint * clamp(l, 0.45, 1.5) * mix(vec3(1.0), hue * 3.0, 0.2) * v_shade;
	if (fa) {
		// FA7 : la carte est neutre en moyenne ; elle porte les écarts de clarté et de teinte des
		// brins (brin vert, épi paille, feuille jaunie), appliqués à la couleur du sol.
		vec3 rel = tex.rgb / tex_lum;
		float rl = max(dot(rel, vec3(0.3, 0.59, 0.11)), 0.001);
		float lum = clamp(mix(1.0, rl, fa_contrast), fa_luma_clamp.x, fa_luma_clamp.y);
		lum = mix(lum, fa_far_luma, far_k * 0.65);
		ALBEDO = v_tint * lum * mix(vec3(1.0), rel / rl, fa_hue_mix) * v_shade;
	}
	// Mipmaps : l'alpha moyenné des petits niveaux amincit les brins au loin ; on le relève
	// selon le niveau lu pour garder leur couverture (sans mipmaps : fourmillement).
	vec2 texel = uv * vec2(textureSize(grass_texture, 0));
	float mip = max(0.0, 0.5 * log2(max(dot(dFdx(texel), dFdx(texel)), dot(dFdy(texel), dFdy(texel)))));
	ALPHA = clamp(tex.a * (1.0 + mip * (fa ? fa_mip_boost : 0.25)), 0.0, 1.0);
	if (fa) {
		float top = 1.0 - UV.y;
		ALBEDO *= mix(mix(fa_foot_shade, 1.0, far_k), 1.0, smoothstep(0.0, fa_foot_height, top));
		ALPHA *= mix(0.45, 1.0, smoothstep(0.12, 0.5, v_facing));
		ALBEDO = mix(vec3(dot(ALBEDO, vec3(0.3, 0.59, 0.11))), ALBEDO, min(decor_saturation * 1.25, 1.0));
	} else if (da6_on > 0.5) {
		// Pied de touffe dans l'ombre des brins, pointes un peu plus sèches ; cartes vues par la
		// tranche amincies (le seuil d'alpha mange d'abord les bords des brins).
		float top = 1.0 - UV.y;
		ALBEDO *= mix(mix(0.8, 1.0, far_k), 1.0, smoothstep(0.0, 0.55, top)) * mix(1.0, 1.1, far_k);
		ALBEDO = mix(ALBEDO, ALBEDO * vec3(1.08, 1.04, 0.9), top * top * 0.5);
		ALPHA *= mix(0.45, 1.0, smoothstep(0.12, 0.5, v_facing));
		// VN1 : brins un peu moins désaturés que le sol (la prairie restait olive terne).
		ALBEDO = mix(vec3(dot(ALBEDO, vec3(0.3, 0.59, 0.11))), ALBEDO, min(decor_saturation * 1.25, 1.0));
	}
	// Alpha-to-coverage : bord des brins lissé par le MSAA (bord à 0,4 comme l'ancienne coupe),
	// sinon la coupe franche scintille dès que la caméra bouge.
	ALPHA_SCISSOR_THRESHOLD = 0.05;
	ALPHA_ANTIALIASING_EDGE = 0.35;
	ALPHA_TEXTURE_COORDINATE = texel;
	ROUGHNESS = 0.9;
	BACKLIGHT = vec3(0.18, 0.22, 0.08) * (fa ? fa_backlight : 1.0);
}
