class_name SeasonLook
extends RefCounted

## Lot TB1 : réglages visuels des saisons de la carte de campagne, lus dans
## `data/ui/campaign_seasons.json` (schéma `data/schemas/campaign_seasons_ui.schema.json`) : mer
## selon la saison, mer sous la tempête. Sans fichier (données de test), valeurs neutres : rendu
## inchangé. Purement visuel : aucune règle de jeu.

const DATA_PATH := "ui/campaign_seasons.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var _data: Dictionary = {}
static var _loaded: bool = false


static func data() -> Dictionary:
	if not _loaded:
		_loaded = true
		var path := _data_dir().path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
	return _data


## Oublie le fichier lu (tests : autre dossier de données).
static func reload() -> void:
	_loaded = false
	_data = {}


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


static func _season_block(season: String, key: String) -> Dictionary:
	var block: Variant = ((data().get("seasons", {}) as Dictionary).get(season, {}) as Dictionary).get(key, {})
	return block if block is Dictionary else {}


## Mer pour des poids de saison (x printemps, y été, z automne, w hiver) :
## {tint: Vector3, desaturate: float, foam: float}, mêlés selon les poids.
static func sea(weights: Vector4) -> Dictionary:
	var tint := Vector3.ZERO
	var desaturate := 0.0
	var foam := 0.0
	var total := 0.0
	for index in SeasonVisuals.SEASONS.size():
		var weight: float = weights[index]
		if weight <= 0.0:
			continue
		var block := _season_block(SeasonVisuals.SEASONS[index], "sea")
		var rgb: Variant = block.get("tint", [1.0, 1.0, 1.0])
		var season_tint := Vector3.ONE
		if rgb is Array and (rgb as Array).size() >= 3:
			season_tint = Vector3(float(rgb[0]), float(rgb[1]), float(rgb[2]))
		tint += season_tint * weight
		desaturate += float(block.get("desaturate", 0.0)) * weight
		foam += float(block.get("foam", 1.0)) * weight
		total += weight
	if total <= 0.0:
		return {"tint": Vector3.ONE, "desaturate": 0.0, "foam": 1.0}
	return {"tint": tint / total, "desaturate": desaturate / total, "foam": foam / total}


## Mer sous la tempête : {foam_gain, foam_width, whitecaps, darken, reach_px, rain_share}
## (tout à 0 sans données : pas d'effet).
static func storm() -> Dictionary:
	var block: Dictionary = data().get("storm", {})
	var result := {}
	for key: String in ["foam_gain", "foam_width", "whitecaps", "darken", "reach_px", "rain_share"]:
		result[key] = float(block.get(key, 0.0))
	return result
