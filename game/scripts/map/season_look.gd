class_name SeasonLook
extends RefCounted

## Lot TB1 : réglages visuels des saisons de la carte de campagne, lus dans
## `data/ui/campaign_seasons.json` (schéma `data/schemas/campaign_seasons_ui.schema.json`) : mer
## selon la saison, mer sous la tempête, étalonnage de saison de la carte, neige des toits des villes 1:1 (ADR 0150). Sans fichier (données de test), valeurs neutres : rendu
## inchangé. Purement visuel : aucune règle de jeu.

const DATA_PATH := "ui/campaign_seasons.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
## Paramètre de shader global lu par `roofscape.gdshaderinc` (villes 1:1) : x quantité de neige,
## y / z début et fin du fondu vers le sud (px carte).
const ROOF_SNOW_PARAM := &"campaign_roof_snow"

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
## {tint: Vector3, grey: Vector3, grey_amount: float, foam: float}, mêlés selon les poids.
static func sea(weights: Vector4) -> Dictionary:
	var tint := Vector3.ZERO
	var grey := Vector3.ZERO
	var grey_amount := 0.0
	var foam := 0.0
	var total := 0.0
	for index in SeasonVisuals.SEASONS.size():
		var weight: float = weights[index]
		if weight <= 0.0:
			continue
		var block := _season_block(SeasonVisuals.SEASONS[index], "sea")
		tint += _vec3(block.get("tint"), Vector3.ONE) * weight
		grey += _vec3(block.get("grey"), Vector3.ZERO) * weight
		grey_amount += float(block.get("grey_amount", 0.0)) * weight
		foam += float(block.get("foam", 1.0)) * weight
		total += weight
	if total <= 0.0:
		return {"tint": Vector3.ONE, "grey": Vector3.ZERO, "grey_amount": 0.0, "foam": 1.0}
	return {"tint": tint / total, "grey": grey / total, "grey_amount": grey_amount / total, "foam": foam / total}


static func _vec3(values: Variant, fallback: Vector3) -> Vector3:
	if values is Array and (values as Array).size() >= 3:
		return Vector3(float(values[0]), float(values[1]), float(values[2]))
	return fallback


## Mer sous la tempête : {foam_gain, foam_width, whitecaps, darken, reach_px, rain_share}
## (tout à 0 sans données : pas d'effet).
static func storm() -> Dictionary:
	var block: Dictionary = data().get("storm", {})
	var result := {}
	for key: String in ["foam_gain", "foam_width", "whitecaps", "darken", "reach_px", "rain_share"]:
		result[key] = float(block.get(key, 0.0))
	return result


## Étalonnage de saison de la carte (bloc `grade`, format de `AtmosphereLibrary.grade_lut`) ;
## {} sans données.
static func grade(season: String) -> Dictionary:
	return _season_block(season, "grade")


## Limite sud de la neige (début, fin du fondu, en fraction de la hauteur de la carte) ;
## `Vector2(-1, -1)` sans données.
static func snow_south_fade() -> Vector2:
	var fade: Variant = (data().get("snow", {}) as Dictionary).get("south_fade", [])
	if fade is Array and (fade as Array).size() >= 2:
		return Vector2(float(fade[0]), float(fade[1]))
	return Vector2(-1, -1)


## Valeur du paramètre global `campaign_roof_snow` pour des poids de saison et une carte haute de
## `map_height` px : neige des toits mêlée selon les poids, fondu vers le sud.
static func roof_snow(weights: Vector4, map_height: float) -> Vector4:
	var amount := 0.0
	for index in SeasonVisuals.SEASONS.size():
		var season: Dictionary = (data().get("seasons", {}) as Dictionary).get(SeasonVisuals.SEASONS[index], {})
		amount += float(season.get("roof_snow", 0.0)) * weights[index]
	var fade := snow_south_fade()
	if fade.x < 0.0:
		return Vector4(clampf(amount, 0.0, 1.0), 0.0, 0.0, 0.0)
	return Vector4(clampf(amount, 0.0, 1.0), fade.x * map_height, fade.y * map_height, 0.0)
