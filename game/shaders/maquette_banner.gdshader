shader_type spatial;
render_mode cull_disabled;

// Lot GC2 (ADR 0158) : bannières des maquettes de lieux (`TownMaquetteLayer`). Les maquettes sont
// dessinées par MultiMesh : la couleur du contrôleur du lieu arrive par la donnée d'instance
// (`INSTANCE_CUSTOM.rgb`, linéaire), sans toucher à la couleur de sommet des bâtiments (couche
// de l'atlas et patine). Rendu seulement.

uniform float roughness : hint_range(0.0, 1.0) = 0.85;

varying vec3 tint;

void vertex() {
	tint = INSTANCE_CUSTOM.rgb;
}

void fragment() {
	ALBEDO = tint;
	ROUGHNESS = roughness;
}
