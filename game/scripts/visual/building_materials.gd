class_name BuildingMaterials
extends RefCounted

## Lot BR1 (ADR 0021) : matériaux PBR partagés des bâtiments du kit Blender
## (`tools/blender_scripts/building_kit.py`). Les GLB du kit ne portent que des matériaux
## *nommés* (`Plaster`, `Rubble`, `RoofTile`…), des UV en mètres réels et des couleurs de sommet
## (teinte du bâtiment × occlusion cuite × patine) ; ce script remplace chaque surface par un
## matériau texturé commun (textures Poly Haven CC0), albédo × couleur de sommet. Variantes :
## `snow` (toits enneigés) et `far` (campagne : sans carte normale, moins de lectures de texture).
## Purement visuel. Les teintes par matériau reprennent `MATERIAL_TINT` de `kit_export.py`.
##
## Lot GA5 : `SPECS`/`PLAIN`/`ROOFS`/`ATLAS_LAYERS` viennent de `data/art/building_materials.json`
## (schéma `art_building_materials.schema.json`), jamais codés en dur ici — même repli que
## `BattleTerrain.ground_layers()`. `TimberFrame` (torchis/colombage, lot GA5) y figure avec
## `wired = false` : la matière est prête (texture, entrée `SPECS`) mais aucune surface du kit
## Blender ne porte ce nom pour l'instant (voir docs/wip/ga.md, section GA5, pour la raison :
## l'indice de couche de l'atlas `Building` est baké dans les `.glb` exportés, donc l'y ajouter
## demande un changement + réexport côté `tools/blender_scripts/kit_export.py`, hors périmètre
## de ce lot). `TimberFrame` reste donc utilisable uniquement via `material("TimberFrame")`
## (matériau individuel, pas l'atlas).

const TEX := "res://assets/textures/"
const DATA_FILE := "art/building_materials.json"

## nom → [albédo, normale, rugosité ("" = scalaire), tuile (m), teinte, rugosité]
static var SPECS: Dictionary = {}
## Matériaux unis (sans texture) : couleur sRGB, rugosité.
static var PLAIN: Dictionary = {}
static var ROOFS: Array = []
## Toutes les matières du kit fusionnées en un matériau `Building` (`building_atlas.gdshader`),
## couche = alpha de la couleur de sommet : maquettes de campagne (variante `far`, sans normales)
## et bâtiments de bataille. Même ordre que `kit_export.ATLAS_LAYERS` et que les tranches de
## `building_albedo_array.jpg` / `building_normal_array.jpg`.
static var ATLAS_LAYERS: Array = []
const ATLAS_SHADER := preload("res://shaders/building_atlas.gdshader")

static var _materials: Dictionary = {}  # "variante|nom" → Material
static var _meshes: Dictionary = {}  # "variante|id du maillage" → Mesh
static var _data_loaded: bool = false


## Lot GA5 : charge `SPECS`/`PLAIN`/`ROOFS`/`ATLAS_LAYERS` depuis les données (dossier de données
## du jeu, puis `data/` du dépôt). Sans repli chiffré : si le fichier est introuvable ou invalide,
## les tables restent vides et `material()`/`_atlas()` ne trouvent aucune matière (avertissement).
static func _ensure_data() -> void:
	if _data_loaded:
		return
	_data_loaded = true
	var candidates: Array[String] = []
	var tree := Engine.get_main_loop() as SceneTree
	var paths: Node = tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	if paths != null:
		candidates.append(str(paths.get("data_dir")))
	candidates.append(ProjectSettings.globalize_path("res://").path_join("../data").simplify_path())
	for dir in candidates:
		var path := dir.path_join(DATA_FILE)
		if not FileAccess.file_exists(path):
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
		if not (parsed is Dictionary):
			continue
		var doc := parsed as Dictionary
		for entry_v in doc.get("textured", []):
			var entry := entry_v as Dictionary
			var name := str(entry["name"])
			var rough_map := str(entry.get("roughness_map", ""))
			var tint: Array = entry["tint"]
			SPECS[name] = [
				str(entry["diffuse"]),
				str(entry["normal"]),
				rough_map,
				float(entry["tile_size_m"]),
				Color(tint[0], tint[1], tint[2]),
				float(entry["roughness"]),
			]
			if bool(entry.get("roof", false)):
				ROOFS.append(name)
		for entry_v in doc.get("plain", []):
			var entry := entry_v as Dictionary
			var color: Array = entry["color"]
			PLAIN[str(entry["name"])] = [Color(color[0], color[1], color[2]), float(entry["roughness"])]
		var layers: Array = doc.get("atlas_layers", [])
		ATLAS_LAYERS = layers.duplicate()
		return
	push_warning("BuildingMaterials: %s introuvable, aucune matière chargée" % DATA_FILE)


static func clear_cache() -> void:
	_materials.clear()
	_meshes.clear()


## Lot GA5 : ordre des couches de l'atlas `Building` (`ATLAS_LAYERS`, chargé depuis les données).
static func atlas_layers() -> Array:
	_ensure_data()
	return ATLAS_LAYERS


## Matériau partagé `name` (variante "", "snow" ou "far") ; null si le nom est inconnu.
static func material(name: String, variant: String = "") -> Material:
	_ensure_data()
	var key := variant + "|" + name
	if _materials.has(key):
		return _materials[key]
	if name == "Building":
		var atlas := _atlas(variant)
		_materials[key] = atlas
		return atlas
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


static func _atlas(variant: String) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = ATLAS_SHADER
	mat.resource_name = "Building"
	mat.set_shader_parameter("albedo_array", load(TEX + "buildings/building_albedo_array.jpg"))
	var tints := PackedVector4Array()
	var tiles := PackedFloat32Array()
	for i in 16:
		var layer_name: String = ATLAS_LAYERS[i] if i < ATLAS_LAYERS.size() else ""
		if SPECS.has(layer_name):
			var spec: Array = SPECS[layer_name]
			var tint: Color = spec[4]
			tints.append(Vector4(tint.r, tint.g, tint.b, float(spec[5])))
			tiles.append(1.0 / float(spec[3]))
		elif PLAIN.has(layer_name):
			var color: Color = (PLAIN[layer_name][0] as Color).srgb_to_linear()
			tints.append(Vector4(color.r, color.g, color.b, float(PLAIN[layer_name][1])))
			tiles.append(1.0)
		else:
			tints.append(Vector4(1, 1, 1, 0.9))
			tiles.append(1.0)
	mat.set_shader_parameter("layer_tint", tints)
	mat.set_shader_parameter("layer_tile", tiles)
	mat.set_shader_parameter("first_plain", ATLAS_LAYERS.find("Window"))
	mat.set_shader_parameter("roof_first", ATLAS_LAYERS.find("RoofTile"))
	mat.set_shader_parameter("roof_last", ATLAS_LAYERS.find("Thatch"))
	mat.set_shader_parameter("snow", 1.0 if variant == "snow" else 0.0)
	if variant != "far":
		mat.set_shader_parameter("normal_array", load(TEX + "buildings/building_normal_array.jpg"))
		mat.set_shader_parameter("use_normals", true)
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
