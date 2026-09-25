shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

// Coques des batailles navales (lot NV2) : bordé à clin procédural en espace objet (aucune UV
// ni texture requise, aucun triangle ajouté). Virures de chêne à recouvrement, joints d'about
// décalés, rivets de fer (clous rivés) le long des recouvrements avec coulures de rouille,
// goudron (brai) sous la flottaison, bande mouillée et brillante au-dessus (clapot animé),
// usure claire au plat-bord, crasse et coulures sous les dalots. `deck` : bordé de pont (planches
// le long du navire, joints calfatés). Mâts et vergues (près de l'axe, verticaux) : fil du bois
// seul. Modèles : 1 unité = 1 m, flottaison à y = 0, +X vers l'avant.

uniform vec3 oak_dark : source_color = vec3(0.25, 0.17, 0.10);
uniform vec3 oak_light : source_color = vec3(0.54, 0.40, 0.25);
uniform vec3 tar_color : source_color = vec3(0.03, 0.026, 0.022);
uniform vec3 iron_color : source_color = vec3(0.13, 0.12, 0.115);
uniform vec3 rust_color : source_color = vec3(0.36, 0.17, 0.07);
uniform float strake_height = 0.34;
uniform float plank_length = 5.2;
uniform float waterline = 0.55;
uniform float wet_band = 0.8;
uniform float wear = 0.5;
uniform float tone = 1.0;
uniform float seed = 0.0;
uniform float charred = 0.0;
uniform bool deck = false;

varying vec3 obj_pos;
varying vec3 obj_normal;

void vertex() {
	obj_pos = VERTEX;
	obj_normal = NORMAL;
}

float hash1(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash1(i), hash1(i + vec2(1.0, 0.0)), u.x),
		mix(hash1(i + vec2(0.0, 1.0)), hash1(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	for (int k = 0; k < 4; k++) {
		v += a * noise(p);
		p = p * 2.03 + vec2(17.1, 9.7);
		a *= 0.5;
	}
	return v;
}

void fragment() {
	vec3 p = obj_pos;
	vec3 n = normalize(obj_normal);
	float s = seed * 7.13;
	// Mâts, vergues, hampes : bois nu, fil vertical, sans virures.
	bool spar = !deck && abs(p.z) < 0.45 && abs(n.y) < 0.6 && p.y > 1.0;
	// Coordonnées du bordé : le long du navire (x) et en hauteur (y) ; pont : x et travers (z).
	vec2 q = deck ? vec2(p.x, p.z) : vec2(p.x, p.y);
	float band = deck ? 0.24 : strake_height;
	float row_f = q.y / band;
	float row = floor(row_f);
	float across = fract(row_f);
	float along_f = (q.x + hash1(vec2(row, s)) * plank_length) / plank_length;
	float plank = floor(along_f);
	float along = fract(along_f);
	vec2 plank_id = vec2(plank, row) + s;

	// Fil du bois : bruit étiré le long des planches, veines, nœuds rares.
	vec2 grain_uv = spar ? vec2(p.y * 0.8, (p.x + p.z) * 30.0) : vec2(q.x * 0.9, q.y * 26.0);
	float grain = fbm(grain_uv + plank_id * 3.7);
	float veins = smoothstep(0.35, 0.65, fbm(vec2(q.x * 0.35, q.y * 60.0) + plank_id));
	float knot = smoothstep(0.035, 0.0, length(vec2((along - hash1(plank_id + 3.1)) * plank_length, (across - 0.5) * band)) - 0.01);
	float plank_tone = hash1(plank_id);
	vec3 wood = mix(oak_dark, oak_light, clamp(0.25 + 0.35 * plank_tone + 0.4 * grain - 0.15 * veins, 0.0, 1.0));
	wood = mix(wood, oak_dark * 0.6, knot * 0.8);
	wood *= tone;

	float rough = 0.78 + 0.12 * grain;
	float metal = 0.0;
	vec3 albedo = wood;
	if (!spar) {
		// Recouvrement du clin : ombre sous l'arête de chaque virure, arête claire au-dessus.
		float lap_shadow = deck ? smoothstep(0.0, 0.05, across) * smoothstep(1.0, 0.95, across)
			: smoothstep(0.0, 0.12, across);
		float lap_edge = deck ? 0.0 : smoothstep(0.88, 1.0, across);
		float butt = smoothstep(0.0, 0.007, along) * smoothstep(1.0, 0.993, along);
		albedo *= mix(0.55, 1.0, lap_shadow) * mix(0.72, 1.0, butt);
		albedo += lap_edge * 0.05;
		if (deck) {
			// Calfatage (étoupe et brai) entre les planches ; usure claire au milieu du pont.
			float worn = smoothstep(0.55, 0.2, abs(p.z) / 3.0) * (0.6 + 0.4 * fbm(q * 0.8));
			albedo = mix(albedo, oak_light * 1.15, worn * 0.35 * wear);
		} else {
			// Clous rivés le long des recouvrements (tous les 18 cm) et coulures de rouille.
			float rx = fract(q.x / 0.18 + hash1(vec2(row, 1.0 + s)) * 0.5) - 0.5;
			float ry = (across - 0.14) * band;
			float rivet = smoothstep(0.016, 0.009, length(vec2(rx * 0.18, ry)));
			float streak = smoothstep(0.012, 0.0, abs(rx * 0.18)) * smoothstep(0.0, -0.22, ry)
				* smoothstep(-0.34, -0.1, ry) * step(0.45, hash1(vec2(floor(q.x / 0.18), row) + s));
			albedo = mix(albedo, rust_color * 0.8, streak * 0.35);
			albedo = mix(albedo, iron_color, rivet);
			metal = rivet * 0.6;
			rough = mix(rough, 0.45, rivet);
			// Crasse : coulures verticales sous les dalots, plus denses vers le plat-bord.
			float grime = smoothstep(0.55, 0.85, fbm(vec2(q.x * 3.0, q.y * 0.35) + s)) * smoothstep(waterline, 1.6, p.y);
			albedo *= 1.0 - grime * 0.22;
			// Usure : bois délavé et frotté au plat-bord et aux angles.
			float scuff = smoothstep(0.6, 0.9, fbm(q * 2.3 + s * 1.7)) * smoothstep(1.2, 2.4, p.y);
			albedo = mix(albedo, oak_light * 1.2, scuff * 0.3 * wear);
		}
	}

	// Sous la flottaison : goudron noir, luisant ; au-dessus, bande mouillée (clapot animé).
	if (!deck && !spar) {
		float lap = sin(TIME * 1.3 + p.x * 0.8 + s) * 0.06 + sin(TIME * 2.1 - p.x * 1.7) * 0.03;
		float tar_line = waterline + (noise(vec2(p.x * 1.5, s)) - 0.5) * 0.08;
		float tar = 1.0 - smoothstep(tar_line - 0.02, tar_line + 0.02, p.y);
		// Le brai déborde un peu en coulures au-dessus de la ligne.
		tar = max(tar, (1.0 - smoothstep(tar_line, tar_line + 0.25, p.y)) * step(0.72, noise(vec2(p.x * 9.0, 3.0 + s))));
		vec3 tarred = tar_color * (0.8 + 0.4 * grain);
		albedo = mix(albedo, tarred, tar);
		rough = mix(rough, 0.28, tar);
		float wet = 1.0 - smoothstep(tar_line, tar_line + wet_band + lap, p.y);
		wet *= 1.0 - tar;
		albedo *= 1.0 - 0.38 * wet;
		rough = mix(rough, 0.18, wet);
		SPECULAR = mix(0.5, 0.75, max(wet, tar));
		// Algues et sel séché juste au-dessus de la ligne.
		float weed = (1.0 - smoothstep(tar_line + 0.02, tar_line + 0.12, p.y)) * smoothstep(0.5, 0.8, fbm(vec2(p.x * 4.0, p.y * 20.0) + s));
		albedo = mix(albedo, vec3(0.09, 0.12, 0.06), weed * 0.6 * (1.0 - tar));
	}

	// Brûlures : plaques noircies qui gagnent avec le feu subi.
	float burn = charred * smoothstep(1.0 - charred, 1.15 - charred, fbm(p.xy * 0.9 + p.zy * 0.7 + seed));
	albedo = mix(albedo, vec3(0.025, 0.02, 0.018), clamp(burn, 0.0, 0.95));
	ALBEDO = albedo;
	ROUGHNESS = clamp(rough, 0.05, 1.0);
	METALLIC = metal;
}
