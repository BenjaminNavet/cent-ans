class_name MapFireWind
extends RefCounted

## Lot AS5 : paramètres des restes statiques passés en shader (flammes de la carte, vent des
## bannières de maquette, balancement des imposteurs d'arbres de bataille), lus dans
## `data/fx/map_fire_wind.json` (schéma `fx_map_fire_wind.schema.json`). Rendu seulement. Fichier
## absent (fixtures de test) : effets éteints. Éteint aussi par `enabled` et par `--no-as5` (A/B).

const DATA_PATH := "fx/map_fire_wind.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var _data: Dictionary = {}
static var _loaded := false


static func data() -> Dictionary:
	if not _loaded:
		_loaded = true
		var path := _data_dir().path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
	return _data


static func enabled() -> bool:
	return bool(data().get("enabled", false)) and not OS.get_cmdline_user_args().has("--no-as5")


## Section `name` (`fire`, `maquette_banner`, `battle_tree_impostor`), {} si éteint ou absente.
static func section(name: String) -> Dictionary:
	if not enabled():
		return {}
	return data().get(name, {})


static func color(value: Variant) -> Color:
	var rgb: Array = value
	return Color(float(rgb[0]), float(rgb[1]), float(rgb[2]))


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()
