class_name BuildingMaterials
extends RefCounted

## Lot BR1 (ADR 0021) : matériaux PBR partagés des bâtiments du kit Blender
## (`tools/blender_scripts/building_kit.py`). Les GLB du kit ne portent que des matériaux
## *nommés* (`Plaster`, `Rubble`, `RoofTile`…), des UV en mètres réels et des couleurs de sommet
## (teinte du bâtiment × occlusion cuite × patine) ; ce script remplace chaque surface par un
## matériau texturé commun (textures Poly Haven CC0), albédo × couleur de sommet. Variantes :
## `snow` (toits enneigés) et `far` (campagne : sans carte normale, moins de lectures de texture).
## Purement visuel. Les teintes par matériau reprennent `MATERIAL_TINT` de `kit_export.py`.

const TEX := "res://assets/textures/"
## nom → [albédo, normale, rugosité ("" = scalaire), tuile (m), teinte, rugosité]
const SPECS := {
	"Plaster": ["buildings/lime_plaster_diff", "buildings/medieval_wall_01_nor", "buildings/medieval_wall_01_rough", 2.2, Color(1.05, 1.02, 0.97), 0.95],
	"Rubble": ["buildings/stone_wall_diff", "buildings/stone_wall_nor", "buildings/stone_wall_rough", 2.4, Color(1.0, 1.0, 1.0), 0.95],
	"Ashlar": ["buildings/rustic_stone_wall_diff", "buildings/rustic_stone_wall_nor", "buildings/rustic_stone_wall_rough", 2.1, Color(1.0, 1.0, 1.0), 0.9],
	"Timber": ["buildings/rough_wood_diff", "buildings/rough_wood_nor", "buildings/rough_wood_rough", 1.6, Color(1.0, 1.0, 1.0), 0.9],
	"Planks": ["buildings/weathered_brown_planks_diff", "buildings/weathered_brown_planks_nor", "buildings/weathered_brown_planks_rough", 2.2, Color(1.0, 1.0, 1.0), 0.92],
	"Door": ["buildings/weathered_brown_planks_diff", "buildings/weathered_brown_planks_nor", "", 1.6, Color(0.7, 0.62, 0.55), 0.9],
	"RoofTile": ["buildings/clay_roof_tiles_03_diff", "buildings/clay_roof_tiles_03_nor", "buildings/clay_roof_tiles_03_rough", 2.4, Color(0.85, 0.78, 0.76), 0.85],
	"RoofFlat": ["buildings/roof_tiles_14_diff", "buildings/roof_tiles_14_nor", "buildings/roof_tiles_14_rough", 2.2, Color(1.0, 1.0, 1.0), 0.88],
	"RoofSlate": ["battle/roof_slates_02_diff", "battle/roof_slates_02_nor", "", 2.4, Color(0.64, 0.68, 0.76), 0.6],
	"Thatch": ["battle/thatch_roof_angled_diff", "battle/thatch_roof_angled_nor", "", 2.2, Color(1.45, 1.2, 0.85), 1.0],
}
## Matériaux unis (sans texture) : couleur sRGB, rugosité.
const PLAIN := {
	"Window": [Color(0.035, 0.032, 0.03), 0.35],
	"Iron": [Color(0.09, 0.09, 0.1), 0.5],
	"Canvas": [Color(0.78, 0.74, 0.66), 0.95],
	"Banner": [Color(0.7, 0.1, 0.1), 0.8],
}
const ROOFS := ["RoofTile", "RoofFlat", "RoofSlate", "Thatch"]

static var _materials: Dictionary = {}  # "variante|nom" → Material
static var _meshes: Dictionary = {}  # "variante|id du maillage" → Mesh


static func clear_cache() -> void:
	_materials.clear()
	_meshes.clear()


## Matériau partagé `name` (variante "", "snow" ou "far") ; null si le nom est inconnu.
static func material(name: String, variant: String = "") -> Material:
	var key := variant + "|" + name
	if _materials.has(key):
		return _materials[key]
	var mat: StandardMaterial3D = null
	if SPECS.has(name):
		mat = _textured(name, variant)
	elif PLAIN.has(name):
		mat = StandardMaterial3D.new()
		mat.albedo_color = PLAIN[name][0]
		mat.roughness = PLAIN[name][1]
		mat.vertex_color_use_as_albedo = name != "Window"
		if name == "Canvas":
			mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	if mat != null:
		mat.resource_name = name
	_materials[key] = mat
	return mat


static func _textured(name: String, variant: String) -> StandardMaterial3D:
	var spec: Array = SPECS[name]
	var mat := StandardMaterial3D.new()
	var snow := variant == "snow" and name in ROOFS
	mat.albedo_texture = load(TEX + str(spec[0]) + ".jpg")
	mat.albedo_color = Color(0.93, 0.95, 1.0) if snow else spec[4]
	if snow:
		# Manteau de neige : l'albédo du toit ne sert plus qu'à moduler légèrement le blanc.
		mat.albedo_texture = null
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.8 if snow else float(spec[5])
	if variant != "far":
		mat.normal_enabled = true
		mat.normal_texture = load(TEX + str(spec[1]) + ".jpg")
		mat.normal_scale = 0.6 if snow else 1.0
		if str(spec[2]) != "":
			mat.roughness_texture = load(TEX + str(spec[2]) + ".jpg")
			mat.roughness_texture_channel = BaseMaterial3D.TEXTURE_CHANNEL_GREEN
			mat.roughness = 1.0
	var tile := 1.0 / float(spec[3])
	mat.uv1_scale = Vector3(tile, tile, 1.0)
	mat.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	return mat


## Copie de `mesh` dont chaque surface reçoit le matériau partagé de même nom (mise en cache).
## Les surfaces au nom inconnu gardent leur matériau d'origine.
static func remap_mesh(mesh: Mesh, variant: String = "") -> Mesh:
	if mesh == null:
		return null
	var key := variant + "|" + str(mesh.get_instance_id())
	if _meshes.has(key):
		return _meshes[key]
	var copy := mesh.duplicate() as Mesh
	for i in copy.get_surface_count():
		var original := copy.surface_get_material(i)
		if original == null:
			continue
		var shared := material(_base_name(original.resource_name), variant)
		if shared != null:
			copy.surface_set_material(i, shared)
	_meshes[key] = copy
	return copy


## Remplace les maillages de toutes les `MeshInstance3D` de `root` (modèle instancié).
static func remap_node(root: Node, variant: String = "") -> void:
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		mesh_instance.mesh = remap_mesh(mesh_instance.mesh, variant)


## Blender suffixe les doublons (« Plaster.001 ») ; on ne garde que la racine.
static func _base_name(name: String) -> String:
	var dot := name.find(".")
	return name if dot < 0 else name.substr(0, dot)
