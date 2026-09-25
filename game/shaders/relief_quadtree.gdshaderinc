// Lot ZG2 (ADR 0036) — relief streamé en quadtree : patchs fixes déplacés dans le vertex shader
// depuis des pages de hauteurs (tuiles 512² de la pyramide, Texture2DArray R16 + mipmaps).
// Inclus par terrain.gdshader après height_m_lod ; trois crochets d'une ligne :
//   qt_vertex (déplacement + morphing), qt_relief (normale et altitude depuis la page).
// Les morceaux E0 (TerrainBuilder) gardent les valeurs par défaut des `instance uniform` : aucun
// effet. Paramètres posés par ReliefQuadtree (game/scripts/map/relief_quadtree.gd).
//
// Morphing géomorphe (CDLOD) : un sommet d'un nœud de niveau n glisse vers la grille de son parent
// (sommets impairs sur les pairs) et mélange sa hauteur vers la page du parent quand sa distance à
// la caméra approche la limite du niveau : aucune fissure ni saut entre niveaux voisins. Jupes en
// filet de sécurité (pages voisines non chargées, fondu d'une page qui arrive).

uniform sampler2DArray qt_pages : filter_linear_mipmap, repeat_disable;
// Caméra principale (et non celle de la passe d'ombre) : même morphing dans toutes les passes.
uniform vec3 qt_camera = vec3(0.0, 1e6, 0.0);
uniform float qt_page_h_min = -200.0;
uniform float qt_page_h_range = 5000.0;
// Débogage (`--qt-debug=N`) : 1 = teinte par profondeur de nœud, 2 = par étage de la page fine.
uniform int qt_debug = 0;

// Nœud : origine x, z (monde) ; espacement des sommets s (unités) ; drapeau (≥ 0 : nœud du quadtree).
instance uniform vec4 qt_node = vec4(0.0, 0.0, 1.0, -1.0);
// Pages fine (niveau du nœud) et grossière (niveau du parent) : couche du Texture2DArray
// (-1 = heightmap 4096), origine x, z et côté (unités) de la tuile.
instance uniform vec4 qt_fine = vec4(-1.0, 0.0, 0.0, 1.0);
instance uniform vec4 qt_coarse = vec4(-1.0, 0.0, 0.0, 1.0);
// Début du morphing (distance), 1 / longueur du morphing, poids de la page fine (fondu d'arrivée),
// profondeur de jupe (unités monde).
instance uniform vec4 qt_morph = vec4(1e9, 1.0, 1.0, 0.0);
// Couches des tuiles voisines de même étage (O, E, N, S puis NO, NE, SO, SE ; -1 = absente) :
// interpolation bilinéaire continue d'une page à l'autre (pas de marche aux bords de tuiles).
instance uniform vec4 qt_fine_nbr = vec4(-1.0);
instance uniform vec4 qt_fine_diag = vec4(-1.0);
instance uniform vec4 qt_coarse_nbr = vec4(-1.0);
instance uniform vec4 qt_coarse_diag = vec4(-1.0);

const int QT_PAGE_PX = 512;

float qt_texel(ivec2 t, float layer, vec4 nbr, vec4 diag) {
	int dx = t.x < 0 ? -1 : (t.x >= QT_PAGE_PX ? 1 : 0);
	int dy = t.y < 0 ? -1 : (t.y >= QT_PAGE_PX ? 1 : 0);
	float l = layer;
	if (dx != 0 || dy != 0) {
		float cand;
		if (dy == 0) {
			cand = dx < 0 ? nbr.x : nbr.y;
		} else if (dx == 0) {
			cand = dy < 0 ? nbr.z : nbr.w;
		} else if (dy < 0) {
			cand = dx < 0 ? diag.x : diag.y;
		} else {
			cand = dx < 0 ? diag.z : diag.w;
		}
		if (cand >= 0.0) {
			l = cand;
			t -= ivec2(dx, dy) * QT_PAGE_PX;
		} else {
			t = clamp(t, ivec2(0), ivec2(QT_PAGE_PX - 1));
		}
	}
	return texelFetch(qt_pages, ivec3(t, int(l)), 0).r;
}

// Altitude (m) au point monde p : bilinéaire dans la page (et ses voisines), ou heightmap 4096.
float qt_height_m(vec2 p, vec4 page, vec4 nbr, vec4 diag, float hm_lod) {
	if (page.x < 0.0) {
		return height_m_lod((p + vec2(0.5)) / map_size, hm_lod);
	}
	vec2 t = (p - page.yz) / page.w * float(QT_PAGE_PX) - 0.5;
	vec2 f = floor(t);
	vec2 w = t - f;
	ivec2 i = ivec2(f);
	float a = qt_texel(i, page.x, nbr, diag);
	float b = qt_texel(i + ivec2(1, 0), page.x, nbr, diag);
	float c = qt_texel(i + ivec2(0, 1), page.x, nbr, diag);
	float d = qt_texel(i + ivec2(1, 1), page.x, nbr, diag);
	return qt_page_h_min + mix(mix(a, b, w.x), mix(c, d, w.x), w.y) * qt_page_h_range;
}

// Crochet vertex : sommet local du patch (grille entière, y = -1 pour la jupe) → sommet déplacé
// (x, z en unités de grille, y en unités monde ; la transformation d'instance met x, z à l'échelle).
vec3 qt_vertex(vec3 v) {
	if (qt_node.w < 0.0) {
		return v;
	}
	float s = qt_node.z;
	vec2 g = v.xz;
	// Heightmap 4096 (1 px = 1 unité) : mipmap assorti à l'espacement des sommets.
	float lod_fine = max(log2(s) - 1.0, 0.0);
	float lod_coarse = max(log2(2.0 * s) - 1.0, 0.0);
	vec2 p = qt_node.xy + g * s;
	float h0 = qt_height_m(p, qt_coarse, qt_coarse_nbr, qt_coarse_diag, lod_coarse) * height_scale;
	float dist = distance(qt_camera, vec3(p.x, h0, p.y));
	float k = clamp((dist - qt_morph.x) * qt_morph.y, 0.0, 1.0);
	vec2 gm = g - fract(g * 0.5) * 2.0 * k;
	vec2 pm = qt_node.xy + gm * s;
	float hc = qt_height_m(pm, qt_coarse, qt_coarse_nbr, qt_coarse_diag, lod_coarse);
	float hf = qt_fine.x >= 0.0 && qt_fine.x == qt_coarse.x ? hc
		: qt_height_m(pm, qt_fine, qt_fine_nbr, qt_fine_diag, lod_fine);
	float h = mix(hc, hf, qt_morph.z * (1.0 - k)) * height_scale;
	if (v.y < -0.5) {
		h -= qt_morph.w;
	}
	return vec3(gm.x, h, gm.y);
}

// Altitude (m) et gradient (m par unité monde) dans une page, différences finies au pas `k` pixels
// de page (≥ 1, suit l'empreinte du pixel écran), décentrées au bord de la tuile.
void qt_page_relief(vec4 page, vec2 p, float footprint, out float h, out vec2 grad) {
	float px = page.w / float(QT_PAGE_PX);
	float k = max(1.0, footprint / px * 1.5);
	float lod = log2(k);
	vec2 uv = (p - page.yz) / page.w;
	float e = k / float(QT_PAGE_PX);
	float lo = 0.5 / float(QT_PAGE_PX);
	float hi = 1.0 - lo;
	float xl = clamp(uv.x - e, lo, hi);
	float xr = clamp(uv.x + e, lo, hi);
	float yu = clamp(uv.y - e, lo, hi);
	float yd = clamp(uv.y + e, lo, hi);
	float vy = clamp(uv.y, lo, hi);
	float vx = clamp(uv.x, lo, hi);
	float hl = textureLod(qt_pages, vec3(xl, vy, page.x), lod).r;
	float hr = textureLod(qt_pages, vec3(xr, vy, page.x), lod).r;
	float hu = textureLod(qt_pages, vec3(vx, yu, page.x), lod).r;
	float hd = textureLod(qt_pages, vec3(vx, yd, page.x), lod).r;
	grad = vec2((hr - hl) / max(xr - xl, 1e-6), (hd - hu) / max(yd - yu, 1e-6)) / page.w * qt_page_h_range;
	h = qt_page_h_min + textureLod(qt_pages, vec3(vx, vy, page.x), max(lod - 1.0, 0.0)).r * qt_page_h_range;
}

// Crochet fragment : normale d'ombrage et altitude depuis la page du nœud (vallées et crêtes au
// pixel de l'étage), au lieu de la heightmap 4096 + détail 8192 ; sans effet hors quadtree ou
// quand le nœud n'a pas de page (niveaux grossiers).
void qt_relief(vec2 p, float footprint, inout float hc, inout vec2 grad_m) {
	if (qt_node.w < 0.0 || qt_fine.x < 0.0) {
		return;
	}
	float h;
	vec2 g;
	qt_page_relief(qt_fine, p, footprint, h, g);
	if (qt_morph.z < 0.999 && qt_coarse.x >= 0.0) {
		float h2;
		vec2 g2;
		qt_page_relief(qt_coarse, p, footprint, h2, g2);
		h = mix(h2, h, qt_morph.z);
		g = mix(g2, g, qt_morph.z);
	}
	hc = h;
	grad_m = g;
}

// Crochet de débogage (fin du fragment) : teintes par niveau pour vérifier sélection et pages.
vec3 qt_debug_color(vec3 col) {
	if (qt_debug == 0 || qt_node.w < 0.0) {
		return col;
	}
	float level = qt_debug == 1 ? qt_node.w : (qt_fine.x < 0.0 ? -1.0 : log2(256.0 / qt_fine.w));
	vec3 tint = 0.5 + 0.5 * cos(6.2832 * (level * 0.13 + vec3(0.0, 0.33, 0.67)));
	return mix(col, tint * 0.35, 0.55);
}
