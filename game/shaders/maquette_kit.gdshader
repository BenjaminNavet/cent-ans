shader_type spatial;
render_mode cull_disabled;

// Lot GC6-perf (ADR 0158) : matériau unique des maquettes de lieux du kit (`TownMaquetteLayer`).
// Chaque modèle des six familles n'a qu'une surface (`Kit`) : la teinte de chaque face est cuite
// dans la couleur de sommet par Blender (`settlements_east.bake_kit`), au lieu d'une surface et
// d'un matériau par teinte. Un seul matériau pour tous les modèles et toutes les tuiles : un
// appel de dessin par MultiMesh. Opaque, ombres reçues et portées. Rendu seulement.
//
// Espace de couleur : `COLOR_0` du glTF est linéaire et Godot l'importe tel quel ; `ALBEDO`
// attend du linéaire. Aucune conversion : c'est le même produit que l'ancien matériau importé
// (`baseColorFactor` linéaire × couleur de sommet, `vertex_color_is_srgb` faux). Convertir ici
// (sRGB → linéaire) assombrirait les teintes, l'inverse les délaverait.
//
// Bannières : les faces de l'ancien matériau `Banner` portent alpha = 0 (1 partout ailleurs) ;
// elles prennent la couleur du contrôleur du lieu, donnée d'instance du MultiMesh
// (`INSTANCE_CUSTOM.rgb`, déjà linéaire : `Color.srgb_to_linear()` côté script), même convention
// que `maquette_banner.gdshader`.

uniform float roughness : hint_range(0.0, 1.0) = 0.9;

varying vec3 tint;

void vertex() {
	tint = COLOR.a < 0.5 ? INSTANCE_CUSTOM.rgb : COLOR.rgb;
}

void fragment() {
	ALBEDO = tint;
	ROUGHNESS = roughness;
}
