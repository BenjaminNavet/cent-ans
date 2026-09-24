shader_type canvas_item;

// Minicarte de campagne (lot C1) : fond pré-calculé (relief/mer, index de province réduits),
// couleurs de faction et brouillard en textures 1D indexées par province, frontières
// détectées sur les texels voisins. Aucun calcul de règle : couleurs et masque viennent du jeu.

uniform sampler2D province_ids : filter_nearest, repeat_disable;
uniform sampler2D relief : filter_linear, repeat_disable;
uniform sampler2D faction_colors : filter_nearest, repeat_disable, hint_default_transparent;
uniform sampler2D fog_mask : filter_nearest, repeat_disable, hint_default_white;
uniform bool fog_enabled = false;
uniform bool political = true;
uniform vec3 parchment : source_color = vec3(0.90, 0.84, 0.68);
uniform vec3 ink : source_color = vec3(0.22, 0.14, 0.07);
uniform vec3 veil : source_color = vec3(0.55, 0.50, 0.42);

int id_at(vec2 uv) {
	vec4 c = texture(province_ids, uv);
	return int(round(c.r * 255.0)) | (int(round(c.g * 255.0)) << 8);
}

vec4 faction_of(int id) {
	if (id <= 0) {
		return vec4(0.0);
	}
	return texelFetch(faction_colors, ivec2(clamp(id, 0, textureSize(faction_colors, 0).x - 1), 0), 0);
}

float seen(int id) {
	if (!fog_enabled || id <= 0) {
		return 1.0;
	}
	return texelFetch(fog_mask, ivec2(clamp(id, 0, textureSize(fog_mask, 0).x - 1), 0), 0).r;
}

void fragment() {
	vec2 texel = 1.0 / vec2(textureSize(province_ids, 0));
	int id = id_at(UV);
	vec3 base = texture(relief, UV).rgb;
	vec3 col = base;
	if (id > 0) {
		vec4 fc = faction_of(id);
		if (political) {
			// Aplat de couleur politique sur parchemin, relief à peine suggéré.
			float shade = dot(base, vec3(0.333));
			vec3 paper = parchment * (0.82 + 0.25 * shade);
			col = mix(paper, fc.rgb * (0.85 + 0.2 * shade), 0.78 * fc.a);
		} else {
			col = mix(base, fc.rgb, 0.18 * fc.a);
		}
		// Frontières : province (trait fin) et royaume (couleur de faction différente).
		float province_edge = 0.0;
		float realm_edge = 0.0;
		for (int k = 0; k < 4; k++) {
			vec2 dir = k == 0 ? vec2(1.0, 0.0) : k == 1 ? vec2(-1.0, 0.0) : k == 2 ? vec2(0.0, 1.0) : vec2(0.0, -1.0);
			int other = id_at(UV + dir * texel);
			if (other != id) {
				province_edge = 1.0;
				vec4 fo = faction_of(other);
				if (other == 0 || distance(fo, fc) > 0.02) {
					realm_edge = 1.0;
				}
			}
		}
		col = mix(col, ink, political ? 0.22 * province_edge : 0.12 * province_edge);
		col = mix(col, ink, 0.75 * realm_edge);
		// Brouillard : province hors de vue désaturée, voilée, hachurée.
		if (seen(id) < 0.5) {
			float lum = dot(col, vec3(0.299, 0.587, 0.114));
			col = mix(vec3(lum), veil, 0.6) * 0.62;
			float hatch = step(3.0, mod(FRAGCOORD.x + FRAGCOORD.y, 5.0));
			col = mix(col, ink, 0.18 * hatch);
		}
	}
	COLOR = vec4(col, 1.0);
}
