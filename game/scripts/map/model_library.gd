class_name ModelLibrary
extends RefCounted

## M10 assets : modèles 3D low-poly de `res://assets/models/*.glb` (générés par
## `tools/blender_scripts/models.py`). Tout est facultatif : sans fichier importé, les
## fonctions renvoient null / false et les marqueurs gardent leurs placeholders.
##
## Villes : castle (capitale de faction), cathedral (bâtiment `bld_cathedral`), town
## (murailles ou fortification ≥ 2), village sinon. Armées : porte-étendard, plus camp de
## siège (posture `siege`) ou cogue (`embarked`/`at_sea` vrai). Le matériau `Banner`
## est teinté à la couleur de la faction.

const MODELS_DIR := "res://assets/models/"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
## Échelle monde des modèles de ville (≈ 2,5 unités Blender → ≈ 10 px de carte).
const CITY_SCALE := 4.0
## Échelle du porte-étendard dans l'espace local du marqueur d'armée (hampe placeholder ≈ 7).
const ARMY_SCALE := 2.3
const MODEL_NODE := "M10Model"

static var _scenes: Dictionary = {}  # nom → PackedScene ou null
static var _city_kinds: Dictionary = {}  # province_id → nom de modèle
static var _capitals: Dictionary = {}  # province_id → true
static var _capitals_loaded := false


static func has_model(model_name: String) -> bool:
	return get_scene(model_name) != null


static func get_scene(model_name: String) -> PackedScene:
	if _scenes.has(model_name):
		return _scenes[model_name]
	var path := MODELS_DIR + model_name + ".glb"
	var scene: PackedScene = null
	if ResourceLoader.exists(path):
		scene = load(path) as PackedScene
	_scenes[model_name] = scene
	return scene


static func instantiate(model_name: String, model_scale: float = 1.0) -> Node3D:
	var scene := get_scene(model_name)
	if scene == null:
		return null
	var node := scene.instantiate() as Node3D
	if node == null:
		return null
	node.name = MODEL_NODE
	node.scale = Vector3.ONE * model_scale
	return node


## Teinte les surfaces dont le matériau s'appelle `Banner` (sous-arbre de `root`).
static func tint_banner(root: Node, color: Color) -> void:
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		if mesh_instance.mesh == null:
			continue
		for surface in mesh_instance.mesh.get_surface_count():
			var material := mesh_instance.mesh.surface_get_material(surface)
			if material != null and material.resource_name == "Banner" and material is BaseMaterial3D:
				var tinted := (material as BaseMaterial3D).duplicate() as BaseMaterial3D
				tinted.albedo_color = color
				mesh_instance.set_surface_override_material(surface, tinted)


# --- Villes ------------------------------------------------------------------------


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


static func _read_json(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


static func _load_capitals() -> void:
	_capitals_loaded = true
	var factions_dir := _data_dir().path_join("factions")
	var dir := DirAccess.open(factions_dir)
	if dir == null:
		return
	for file_name in dir.get_files():
		if file_name.ends_with(".json"):
			var capital := str(_read_json(factions_dir.path_join(file_name)).get("capital", ""))
			if capital != "":
				_capitals[capital] = true


## Nom du modèle de ville pour une province (lecture des données, rendu seulement).
static func city_kind(province_id: String) -> String:
	if _city_kinds.has(province_id):
		return _city_kinds[province_id]
	if not _capitals_loaded:
		_load_capitals()
	var kind := "village"
	var province := _read_json(_data_dir().path_join("provinces").path_join(province_id + ".json"))
	var buildings: Array = province.get("buildings", [])
	if _capitals.has(province_id):
		kind = "castle"
	elif buildings.has("bld_cathedral"):
		kind = "cathedral"
	elif buildings.has("bld_stone_walls") or int(province.get("fortification_level", 0)) >= 2:
		kind = "town"
	_city_kinds[province_id] = kind
	return kind


## Modèle de ville prêt à poser, ou null (repli sur le cylindre).
static func city_model(province_id: String) -> Node3D:
	var kind := city_kind(province_id)
	var node := instantiate(kind, CITY_SCALE)
	if node == null and kind != "village":
		node = instantiate("village", CITY_SCALE)
	return node


# --- Armées ------------------------------------------------------------------------


static func army_kind(army: Dictionary) -> String:
	if bool(army.get("embarked", false)) or bool(army.get("at_sea", false)):
		return "ship"
	return "army"


## Habille un `ArmyMarker` : modèle 3D (+ camp de siège) teinté, placeholders masqués.
## Renvoie false (placeholders conservés) si le modèle n'est pas disponible.
static func dress_army_marker(marker: Node3D, army: Dictionary, color: Color) -> bool:
	var previous := marker.get_node_or_null(MODEL_NODE)
	if previous != null:
		marker.remove_child(previous)
		previous.queue_free()
	var model := instantiate(army_kind(army), ARMY_SCALE)
	if model == null:
		return false
	if str(army.get("stance", "")) == "siege":
		var camp := instantiate("siege_camp", 1.0)
		if camp != null:
			camp.name = "SiegeCamp"
			camp.position = Vector3(1.6, 0.0, 0.8)
			model.add_child(camp)
	tint_banner(model, color)
	marker.add_child(model)
	for placeholder in ["Pole", "Finial", "Banner"]:
		var node := marker.get_node_or_null(placeholder) as Node3D
		if node != null:
			node.visible = false
	return true
