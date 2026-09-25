class_name LandmarkLibrary
extends RefCounted

## Villes emblématiques (lot L1) : lecture de `data/landmarks/*.json`, index par colonie.
## Rendu seulement ; sans dossier ou sans modèle importé, rien ne change (maquette générique).

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var _by_settlement: Dictionary = {}
static var _loaded := false


static func clear_cache() -> void:
	_by_settlement.clear()
	_loaded = false


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


static func _load() -> void:
	_loaded = true
	var dir_path := _data_dir().path_join("landmarks")
	var dir := DirAccess.open(dir_path)
	if dir == null:
		return
	for file_name in dir.get_files():
		if not file_name.ends_with(".json"):
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(dir_path.path_join(file_name)))
		if parsed is Dictionary and parsed.has("settlement"):
			_by_settlement[str(parsed["settlement"])] = parsed


## Plan de la ville emblématique d'une colonie, {} sinon.
static func for_settlement(settlement_id: String) -> Dictionary:
	if not _loaded:
		_load()
	return _by_settlement.get(settlement_id, {})


static func all() -> Array:
	if not _loaded:
		_load()
	return _by_settlement.values()
