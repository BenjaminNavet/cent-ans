shader_type spatial;
render_mode cull_disabled;

// Lot GC2 (ADR 0158) : bannières des maquettes de lieux (`TownMaquetteLayer`). Les maquettes sont
// dessinées par MultiMesh : la couleur du contrôleur du lieu arrive par la donnée d'instance
// (`INSTANCE_CUSTOM.rgb`, linéaire), sans toucher à la couleur de sommet des bâtiments (couche
// de l'atlas et patine). Rendu seulement.
// Lot AS5 : onde de vent. La toile ondule le long de sa normale (phase = position dans le plan de
// la toile, avancée par le temps) ; l'amplitude (part de la taille de la bannière, `banner_size`)
// croît avec la rafale partagée de la carte (`campaign_wind.gdshaderinc`, la même que pour les
// forêts et les imposteurs d'arbres). `amplitude` = 0 : bannière immobile (`--no-as5`).

#include "res://shaders/campaign_wind.gdshaderinc"

uniform float roughness : hint_range(0.0, 1.0) = 0.85;
uniform float amplitude = 0.0;
uniform float banner_size = 1.0;
uniform float wave_speed = 5.0;
uniform float wave_length = 0.9;
uniform float gust_gain = 0.9;
uniform float calm = 0.45;

varying vec3 tint;

void vertex() {
	tint = INSTANCE_CUSTOM.rgb;
	if (amplitude > 0.0) {
		vec3 anchor = MODEL_MATRIX[3].xyz;
		float gust = wind_gust(anchor.xz);
		float phase = dot(VERTEX, vec3(1.0, 0.6, 1.0)) / max(wave_length * banner_size, 1e-4)
			- TIME * wave_speed + wind_hash(anchor.xz) * 6.2831;
		float wave = sin(phase) * 0.7 + sin(phase * 2.1 + 1.3) * 0.3;
		VERTEX += NORMAL * wave * amplitude * banner_size * (calm + gust_gain * gust);
	}
}

void fragment() {
	ALBEDO = tint;
	ROUGHNESS = roughness;
}
