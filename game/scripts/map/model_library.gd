class_name ModelLibrary
extends RefCounted

## M10 assets : modèles 3D low-poly de `res://assets/models/*.glb` (générés par
## `tools/blender_scripts/models.py`). Tout est facultatif : sans fichier importé, les
## fonctions renvoient null / false et les marqueurs gardent leurs placeholders.
##
## Villes : castle (capitale de faction), cathedral (bâtiment `bld_cathedral`), town
## (murailles ou fortification ≥ 2), village sinon ; lot V3 : villes fortifiées entières
## (`city_cathedral` pour les cathédrales). Armées : chef monté et escorte (fantassins,
## arbalétriers, cavaliers) selon la composition, plus camp de
## siège (posture `siege`) ou cogue (`embarked`/`at_sea` vrai). Le matériau `Banner`
## est teinté à la couleur de la faction.

const MODELS_DIR := "res://assets/models/"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
## Échelle monde des modèles de ville (lot V3 : une ville fortifiée ≈ 2,1 unités Blender
## → ≈ 10 px de carte ; maisons de la taille des arbres, cohérentes avec le relief).
const CITY_SCALE := 4.8
## Échelle du porte-étendard dans l'espace local du marqueur d'armée (hampe placeholder ≈ 7).
const ARMY_SCALE := 3.8
const MODEL_NODE := "M10Model"

static var _scenes: Dictionary = {}  # nom → PackedScene ou null
static var _city_kinds: Dictionary = {}  # province_id → nom de modèle
static var _capitals: Dictionary = {}  # province_id → true
static var _capitals_loaded := false
static var _tinted_meshes: Dictionary = {}  # "mesh|couleur" → Mesh teinté
static var _unit_categories: Dictionary = {}  # unit_type → catégorie
static var _rulers: Dictionary = {}  # faction_id → personnage


## Vide les caches statiques (appelé en fin de smoke test pour éviter les fuites signalées).
static func clear_cache() -> void:
	_scenes.clear()
	_city_kinds.clear()
	_capitals.clear()
	_capitals_loaded = false
	_tinted_meshes.clear()
	_unit_categories.clear()
	_rulers.clear()


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
## Le maillage est dupliqué (un par couleur, mis en cache) plutôt que d'utiliser
## `set_surface_override_material`, qui produit des erreurs avec le rendu factice headless.
static func tint_banner(root: Node, color: Color) -> void:
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		if mesh_instance.mesh == null:
			continue
		var key := "%s|%s" % [mesh_instance.mesh.resource_path + str(mesh_instance.mesh.get_rid().get_id()), color.to_html()]
		if _tinted_meshes.has(key):
			mesh_instance.mesh = _tinted_meshes[key]
			continue
		var tinted_mesh: Mesh = null
		for surface in mesh_instance.mesh.get_surface_count():
			var material := mesh_instance.mesh.surface_get_material(surface)
			if material != null and material.resource_name == "Banner" and material is BaseMaterial3D:
				if tinted_mesh == null:
					tinted_mesh = mesh_instance.mesh.duplicate() as Mesh
				var tinted := (material as BaseMaterial3D).duplicate() as BaseMaterial3D
				tinted.albedo_color = color
				tinted_mesh.surface_set_material(surface, tinted)
		if tinted_mesh != null:
			_tinted_meshes[key] = tinted_mesh
			mesh_instance.mesh = tinted_mesh


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
	# Lot V3 : la ville épiscopale entière (`city_cathedral`) ; `cathedral` seul reste le
	# bâtiment isolé (décor des batailles de siège).
	var node := instantiate("city_cathedral", CITY_SCALE) if kind == "cathedral" else null
	if node == null:
		node = instantiate(kind, CITY_SCALE)
	if node == null and kind != "village":
		node = instantiate("village", CITY_SCALE)
	return node


# --- Armées ------------------------------------------------------------------------

## Figurines d'escorte (en plus du chef) selon l'effectif : [seuil d'hommes, nombre].
const ESCORT_BY_MEN := [[400, 2], [1200, 3], [1000000000, 4]]
## Places des figurines d'escorte dans le repère du marqueur (le groupe regarde +X).
const ESCORT_SLOTS := [
	Vector3(-1.5, 0.0, 1.2), Vector3(-1.1, 0.0, -1.5), Vector3(-2.9, 0.0, 0.2), Vector3(-2.7, 0.0, 1.9),
]
const SIEGE_CAMP_OFFSET := Vector3(3.2, 0.0, 2.2)


static func army_kind(army: Dictionary) -> String:
	if bool(army.get("embarked", false)) or bool(army.get("at_sea", false)):
		return "ship"
	return "army"


## Catégorie d'un type d'unité (`data/unit_types/<id>.json` : cavalry, infantry, ranged, siege).
static func unit_category(unit_type: String) -> String:
	if _unit_categories.has(unit_type):
		return _unit_categories[unit_type]
	var data := _read_json(_data_dir().path_join("unit_types").path_join(unit_type + ".json"))
	var category := str(data.get("category", "infantry"))
	_unit_categories[unit_type] = category
	return category


## Modèles des figurines d'escorte, proportionnels à la composition de l'armée.
static func escort_models(army: Dictionary) -> PackedStringArray:
	var units: Array = army.get("units", [])
	var men := 0
	var weights := {"army_foot": 0, "army_archer": 0, "army": 0}
	for unit in units:
		men += int(unit.get("strength", 0))
		match unit_category(str(unit.get("unit_type", ""))):
			"ranged":
				weights["army_archer"] += 1
			"cavalry":
				weights["army"] += 1
			_:
				weights["army_foot"] += 1
	var count := 0
	for step in ESCORT_BY_MEN:
		if men < int(step[0]):
			count = int(step[1])
			break
	var total: int = weights["army_foot"] + weights["army_archer"] + weights["army"]
	var result := PackedStringArray()
	if total == 0:
		for i in count:
			result.append("army_foot")
		return result
	# Répartition au plus fort reste, fantassins d'abord.
	var remaining := count
	var shares := {}
	for model_name in ["army_foot", "army_archer", "army"]:
		shares[model_name] = int(floor(float(weights[model_name]) * count / total))
		remaining -= shares[model_name]
	for model_name in ["army_foot", "army_archer", "army"]:
		if remaining <= 0:
			break
		if weights[model_name] > 0 and float(weights[model_name]) * count / total - shares[model_name] > 0.0:
			shares[model_name] += 1
			remaining -= 1
	for model_name in ["army_foot", "army_archer", "army"]:
		for i in shares[model_name]:
			result.append(model_name)
	while result.size() < count:
		result.append("army_foot")
	return result


## Habille un `ArmyMarker` : groupe de figurines teintées (chef monté + escorte selon la
## composition) ou cogue, plus camp de siège ; le placeholder `Banner` est masqué.
## Renvoie false (placeholders conservés) si le modèle n'est pas disponible.
static func dress_army_marker(marker: Node3D, army: Dictionary, color: Color) -> bool:
	var previous := marker.get_node_or_null(MODEL_NODE)
	if previous != null:
		marker.remove_child(previous)
		previous.queue_free()
	var kind := army_kind(army)
	var leader := instantiate(kind, ARMY_SCALE if kind == "army" else ARMY_SCALE * 1.5)
	if leader == null:
		return false
	var group := Node3D.new()
	group.name = MODEL_NODE
	leader.name = "Leader"
	leader.position = Vector3(0.6, 0.0, 0.0)
	group.add_child(leader)
	if kind == "army":
		var escort := escort_models(army)
		for i in mini(escort.size(), ESCORT_SLOTS.size()):
			var figure := instantiate(escort[i], ARMY_SCALE * (0.8 if escort[i] == "army" else 1.0))
			if figure == null:
				continue
			figure.name = "Escort_%d" % i
			figure.position = ESCORT_SLOTS[i]
			figure.rotation.y = randf_range(-0.12, 0.12)
			group.add_child(figure)
	if str(army.get("stance", "")) == "siege":
		var camp := instantiate("siege_camp", ARMY_SCALE * 0.9)
		if camp != null:
			camp.name = "SiegeCamp"
			camp.position = SIEGE_CAMP_OFFSET
			group.add_child(camp)
	tint_banner(group, color)
	for figure in group.find_children("*", "GeometryInstance3D", true, false):
		(figure as GeometryInstance3D).layers = 2
	marker.add_child(group)
	var placeholder := marker.get_node_or_null("Banner") as Node3D
	if placeholder != null:
		placeholder.visible = false
	return true


## Dirigeant d'une faction d'après les données (`data/factions/<id>.json`, champ `ruler`), "" sinon.
static func faction_ruler(faction_id: String) -> String:
	if not _rulers.has(faction_id):
		_rulers[faction_id] = str(_read_json(_data_dir().path_join("factions").path_join(faction_id + ".json")).get("ruler", ""))
	return _rulers[faction_id]
