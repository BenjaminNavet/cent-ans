shader_type spatial;
render_mode cull_disabled, depth_draw_opaque, diffuse_lambert_wrap, specular_disabled;

// Lot FC3 : touffes d'herbe et broussailles proches de la carte de campagne (`GroundClutter`).
// Cartes croisées (UV.y = 0 en haut), masque procédural (brins ou feuillage), coupe alpha, vent
// léger, teinte saisonnière (poids `campaign_season` du lot CV1), fondu dithered par distance
// caméra (`fade`) et au bord du disque visible (`focus`, `radius`). Aucune ombre (réglée par le
// script). Données par instance (`INSTANCE_CUSTOM`) : x = sorte (0 herbe, 1 broussaille),
// y = variation de teinte, z = phase du vent, w = rang (coupe du fondu).

global uniform vec4 campaign_season;
global uniform float campaign_prop_scale;

uniform float fade : hint_range(0.0, 1.0) = 1.0;
// Lot FC5 : réduction avec `campaign_prop_scale`^puissance (0,5 : plus lente que les arbres).
uniform float prop_scale_power = 0.5;
uniform vec2 focus = vec2(0.0);
uniform float radius = 40.0;
uniform float wind_strength = 0.12;
uniform float wind_speed = 1.4;
// Lot FC5 : teintes proches de la prairie du terrain (`terrain.gdshader`, tint_grass linéaire
// 0,105 / 0,165 / 0,045), pointes un peu plus claires pour détacher les touffes du sol.
uniform vec3 grass_color : source_color = vec3(0.34, 0.44, 0.21);
uniform vec3 grass_dry : source_color = vec3(0.46, 0.46, 0.27);
uniform vec3 bush_color : source_color = vec3(0.28, 0.37, 0.18);
// Lot FC5 : touffe en éventail (lot DA6, CC0) : alpha = brins, luminance = relief des brins.
// Mipmaps + compensation de l'alpha : les brins restent lisibles quand la touffe fait 10 px.
uniform sampler2D grass_texture : source_color, filter_linear_mipmap, repeat_disable;
uniform bool has_grass_texture = false;
uniform vec3 autumn_color : source_color = vec3(0.45, 0.30, 0.12);
uniform vec3 winter_color : source_color = vec3(0.36, 0.32, 0.24);

varying float v_kind;
varying float v_tint;
varying float v_seed;

float hash1(float n) {
	return fract(sin(n * 12.9898) * 43758.5453);
}

float hash2(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float value_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), u.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x), u.y);
}

void vertex() {
	v_kind = INSTANCE_CUSTOM.x;
	v_tint = INSTANCE_CUSTOM.y;
	v_seed = INSTANCE_CUSTOM.z;
	float up = 1.0 - UV.y;
	float prop = pow(max(campaign_prop_scale, 1e-4), prop_scale_power);
	VERTEX *= prop;
	vec3 world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	// Vent : pointe des brins seulement (buisson plus raide).
	float sway = sin(TIME * wind_speed + INSTANCE_CUSTOM.z * 6.283 + world.x * 0.35 + world.z * 0.21);
	float bend = up * up * wind_strength * (v_kind > 0.5 ? 0.35 : 1.0) * prop;
	VERTEX.x += sway * bend;
	VERTEX.z += sway * bend * 0.4;
	// Fondu : les rangs au-dessus du seuil (distance caméra, bord du disque) sont retirés.
	float edge = 1.0 - smoothstep(radius * 0.75, radius, distance(world.xz, focus));
	if (INSTANCE_CUSTOM.w > fade * edge) {
		VERTEX = vec3(0.0);  // triangle dégénéré : ni fragment, ni coût de remplissage
	}
}

// Brins : sept lames effilées, penchées au hasard.
float blades(vec2 uv, float seed) {
	float up = 1.0 - uv.y;
	float a = 0.0;
	for (int i = 0; i < 7; i++) {
		float fi = float(i) + seed * 17.0;
		float cx = 0.12 + 0.76 * hash1(fi + 1.3);
		float h = 0.5 + 0.5 * hash1(fi + 7.1);
		float lean = (hash1(fi + 3.7) - 0.5) * 0.5;
		float x = cx + lean * up * up;
		float w = 0.055 * (1.0 - up / h);
		a = max(a, step(abs(uv.x - x), w) * step(up, h));
	}
	return a;
}

// Broussaille : masse arrondie au bord déchiqueté.
float bush(vec2 uv, float seed) {
	vec2 p = (uv - vec2(0.5, 0.55)) * vec2(2.0, 2.3);
	float body = 1.0 - length(p);
	float leaves = value_noise(uv * 11.0 + seed * 31.0) * 0.55 + value_noise(uv * 23.0 - seed * 13.0) * 0.3;
	return step(0.35, body + leaves * 0.6);
}

void fragment() {
	bool is_bush = v_kind > 0.5;
	float mask;
	float shade = 1.0;
	if (is_bush) {
		mask = bush(UV, v_seed);
	} else if (has_grass_texture) {
		// Une touffe sur deux retournée : moins de répétition.
		vec2 uv = vec2(v_seed > 0.5 ? 1.0 - UV.x : UV.x, UV.y);
		vec4 t = texture(grass_texture, uv);
		float lod = max(textureQueryLod(grass_texture, uv).x, 0.0);
		mask = t.a * (1.0 + lod * 0.4);
		shade = mix(0.8, 1.1, dot(t.rgb, vec3(0.3, 0.59, 0.11)));
	} else {
		mask = blades(UV, v_seed);
	}
	if (mask < 0.5) {
		discard;
	}
	float up = 1.0 - UV.y;
	vec3 base = is_bush ? bush_color : mix(grass_color, grass_dry, v_tint * v_tint * 0.5);
	base *= mix(0.9, 1.12, v_tint) * shade;
	vec4 w = campaign_season;
	// Printemps tendre, été sec, automne roux, hiver brun-gris.
	vec3 color = base * (w.x * vec3(1.03, 1.12, 0.92) + w.y * vec3(1.05, 1.0, 0.85));
	color += w.z * mix(base, autumn_color * (is_bush ? 0.8 : 1.0), 0.6);
	color += w.w * mix(base, winter_color, 0.75);
	float total = w.x + w.y + w.z + w.w;
	if (total < 0.01) {
		color = base;
	}
	// Pied des touffes assombri (occlusion au sol).
	color *= mix(0.6, 1.06, smoothstep(0.0, 0.8, up));
	ALBEDO = color;
	ROUGHNESS = 0.95;
	BACKLIGHT = color * 0.3;
}
