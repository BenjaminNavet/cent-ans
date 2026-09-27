class_name CameraFeel
extends RefCounted

## Chantier PO5 (ADR 0097) : ressenti des caméras, lu dans `data/ui/camera_feel.json` (schéma
## `data/schemas/camera_feel.schema.json`). `get_value(block, key)` rend la valeur du bloc
## `campaign` ou `battle`, remplacée par celle de `reduce_motion` quand « Réduire les
## animations » est actif. Les valeurs de repli ne servent que si le fichier manque.

const DATA_FILE := "ui/camera_feel.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const FALLBACK := {
	"campaign": {
		"pan_accel": 14.0, "pan_friction": 5.5, "follow_damping": 12.0, "zoom_damping": 9.0,
		"rotate_damping": 10.0, "focus_glide_s": 0.4, "drag_release_inertia": 0.6,
	},
	"battle": {"zoom_damping": 9.0, "focus_glide_s": 0.4},
	"reduce_motion": {},
}

static var _cache: Dictionary = {}
## Vrai si les valeurs viennent du fichier de données (tests).
static var loaded_from_data: bool = false


## Valeur `key` du bloc `block` (`"campaign"` ou `"battle"`), selon « Réduire les animations ».
static func get_value(block: String, key: String) -> float:
	var feel := settings()
	if Accessibility.reduce_motion():
		var reduced: Dictionary = feel.get("reduce_motion", {})
		if reduced.has(key):
			return float(reduced[key])
	var values: Dictionary = feel.get(block, {})
	return float(values.get(key, (FALLBACK.get(block, {}) as Dictionary).get(key, 0.0)))


## Réglages complets (lus une fois).
static func settings() -> Dictionary:
	if _cache.is_empty():
		_cache = FALLBACK.duplicate(true)
		var path := _data_dir().path_join(DATA_FILE)
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_FILE)
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path)) if FileAccess.file_exists(path) else null
		if parsed is Dictionary:
			for block: String in ["campaign", "battle", "reduce_motion"]:
				if (parsed as Dictionary).get(block) is Dictionary:
					(_cache[block] as Dictionary).merge(parsed[block], true)
			loaded_from_data = true
		else:
			push_warning("CameraFeel : %s illisible, valeurs de repli." % DATA_FILE)
	return _cache


## Relit le fichier au prochain accès (tests).
static func reload() -> void:
	_cache = {}
	loaded_from_data = false


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.project_root().path_join("data")
