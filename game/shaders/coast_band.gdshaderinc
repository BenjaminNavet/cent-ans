// Lot TB5 (mer et côtes) : bande côtière du terrain. Un seul crochet dans `terrain.gdshader`
// (`coast_band`, après l'eau peinte sur le terrain : la bande n'est plus voilée par elle, et le
// relief qui émerge côté mer de la côte raster — pied de falaise, écueil, banc de sable — est
// peint en roche ou en sable au lieu de rester une tache d'eau pâle). Falaise (craie, granite, roche) là où le relief
// domine le rivage, plage (sable, galets) ailleurs ; la roche et la plage viennent de la région
// (`coast_common.gdshaderinc`). Matières procédurales : aucun échantillonneur de plus que
// `coast_types`. Réglages : `data/map/coast_types.json`, posés par `CoastLook.apply`.
#include "res://shaders/coast_common.gdshaderinc"

uniform float coast_cliff_px = 0.85;   // largeur de la falaise vue de dessus (px carte)
uniform float coast_sea_reach_px = 3.0; // relief émergé peint jusqu'à cette distance côté mer
uniform float coast_beach_px = 0.55;   // largeur de la plage
uniform float coast_wet_px = 0.16;     // estran mouillé
uniform float coast_strength = 1.0;
uniform float coast_relief = 0.9;      // inclinaison feinte de la paroi vers la mer
uniform float coast_far_keep = 0.55;   // part gardée au dézoom
// Indices : 0 roche, 1 craie, 2 granite, 3 sable, 4 galets.
uniform vec3 coast_color[5];
uniform vec3 coast_shade[5];
uniform float coast_streak[5];

void coast_band(vec2 p, vec2 uv, float footprint, float coast, float hc, float world_y, float lake_shore, float snow,
		inout vec3 col, inout float roughness, inout vec3 n) {
	float reach = max(coast_cliff_px * 1.2, coast_beach_px * 3.2);
	if (!coast_enabled || !has_coast_dist || lake_shore > 0.5 || coast > reach) {
		return;
	}
	// Relief émergé côté mer de la côte raster (la carte d'altitude y dépasse le niveau de la mer).
	float emerged = coast > -coast_sea_reach_px && world_y >= 0.0 ? smoothstep(0.0, 2.0, hc) : 0.0;
	if (coast < -0.3 && emerged <= 0.0) {
		return;
	}
	vec2 to_land = coast_to_land(uv);
	float h = height_m_lod(coast_probe_uv(uv, coast, to_land), 0.0);
	float cliff_f = max(coast_cliff_factor(h), emerged * smoothstep(6.0, 15.0, hc));
	vec4 geo = coast_geology(p);
	float w_rock = max(1.0 - geo.r - geo.g, 0.0);
	float fine = 1.0 - smoothstep(0.15, 0.7, footprint);  // grain visible de près seulement
	float shore = max(smoothstep(-0.25, 0.05, coast), emerged);
	float k = coast_strength * mix(1.0, coast_far_keep, smoothstep(1.5, 5.0, footprint));

	// Plage : largeur irrégulière, grain, estran mouillé au ras de l'eau.
	vec3 beach_c = mix(coast_color[3], coast_color[4], geo.b);
	vec3 beach_s = mix(coast_shade[3], coast_shade[4], geo.b);
	float beach_k = mix(coast_streak[3], coast_streak[4], geo.b);
	float bw = coast_beach_px * mix(0.5, 2.0, geo.a) * (0.55 + 0.9 * value_noise(p * 0.9 + vec2(11.0, 3.0)));
	float speck = mix(0.5, value_noise(p * 17.0) * 0.6 + value_noise(p * 41.0) * 0.4, fine);
	vec3 beach_col = mix(beach_s, beach_c, clamp(1.0 - beach_k + beach_k * speck * 1.6, 0.0, 1.0));
	float wet = 1.0 - smoothstep(0.0, coast_wet_px, coast);
	beach_col *= mix(1.0, 0.7, wet);
	// TB5 : ressac sur la plage (même lame que la mer) : eau de la lame, puis son écume.
	vec2 swash = coast_swash(p, coast, cliff_f, footprint) * shore;
	beach_col = mix(beach_col, beach_col * vec3(0.55, 0.7, 0.72), swash.y * 0.5);
	beach_col = mix(beach_col, swash_foam_color, swash.x * 0.8);
	float beach = shore * (1.0 - smoothstep(bw * 0.6, bw, coast)) * (1.0 - cliff_f) * (1.0 - 0.7 * snow);
	col = mix(col, beach_col, beach * k);
	roughness = mix(roughness, mix(0.9, 0.5, wet), beach * k);

	// Falaise : paroi claire ou sombre selon la roche, pied dans l'ombre et les galets, strates
	// et grain ; la normale penche vers la mer (la paroi prend la lumière selon son exposition).
	vec3 rock_c = coast_color[0] * w_rock + coast_color[1] * geo.r + coast_color[2] * geo.g;
	vec3 rock_s = coast_shade[0] * w_rock + coast_shade[1] * geo.r + coast_shade[2] * geo.g;
	float rock_k = coast_streak[0] * w_rock + coast_streak[1] * geo.r + coast_streak[2] * geo.g;
	float cw = coast_cliff_px * (0.65 + 0.5 * value_noise(p * 1.7)) * mix(0.7, 1.0, smoothstep(coast_cliff_m.y, coast_cliff_m.y * 3.0, h));
	float t = max(clamp(coast / max(cw, 1e-3), 0.0, 1.0), emerged * smoothstep(5.0, 40.0, hc));  // 0 pied, 1 crête
	float grain = value_noise(p * 5.0) * 0.6 + value_noise(p * 11.0) * 0.4;
	float strata = value_noise(vec2(t * 6.0, value_noise(p * 0.9) * 3.0));
	float mottle = mix(0.5, grain * 0.65 + strata * 0.35, fine);
	float light = clamp((1.0 - rock_k + rock_k * mottle * 1.7) * (0.55 + 0.45 * smoothstep(0.0, 0.45, t)), 0.0, 1.0);
	vec3 cliff_col = mix(rock_s, rock_c, light);
	float foot = (1.0 - smoothstep(0.05, 0.22, coast)) * 0.7 * (1.0 - emerged);  // estran de galets ou de blocs au pied
	cliff_col = mix(cliff_col, mix(beach_col, rock_s, 0.5), foot);
	cliff_col = mix(cliff_col, swash_foam_color, swash.x * 0.8);  // la lame bat le pied de la falaise
	float face = shore * (1.0 - smoothstep(cw * 0.7, cw, coast)) * cliff_f;
	col = mix(col, cliff_col, face * k);
	roughness = mix(roughness, 0.95, face * k);
	n = normalize(n + vec3(-to_land.x, 0.0, -to_land.y) * coast_relief * face * k);
}
