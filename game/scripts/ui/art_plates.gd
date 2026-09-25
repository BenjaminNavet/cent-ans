class_name ArtPlates
extends RefCounted

## Lot AR1 — habillage illustré (`data/ui/illustrations.json`, schéma
## `data/schemas/illustrations.schema.json`) : écrans de chargement par contexte (bataille,
## siège, bataille navale, campagne) avec citation de chroniqueur, vignettes des événements
## (par genre d'événement du cœur ; l'ordre du fichier fait la priorité) et écrans de fin.
## Aucune règle : lecture et choix d'illustration seulement.

const DATA_PATH := "ui/illustrations.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var _data: Dictionary = {}


static func data() -> Dictionary:
	if _data.is_empty():
		var path := _data_dir().path_join(DATA_PATH)
		# Jeux de données réduits (fixtures des tests) : illustrations du jeu complet.
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
		if _data.is_empty():
			push_warning("ArtPlates: %s missing or invalid" % path)
	return _data


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


## Durée d'affichage minimale d'un écran de chargement (secondes).
static func min_seconds() -> float:
	return float((data().get("loading", {}) as Dictionary).get("min_seconds", 3.0))


static func max_seconds() -> float:
	return float((data().get("loading", {}) as Dictionary).get("max_seconds", 4.0))


## Écrans de chargement d'un contexte (`battle`, `siege`, `naval`, `campaign`).
static func loading_screens(context: String) -> Array:
	var result: Array = []
	for screen in (data().get("loading", {}) as Dictionary).get("screens", []):
		if context in (screen as Dictionary).get("contexts", []):
			result.append(screen)
	return result


## Un écran tiré au hasard pour `context` ({} si aucun).
static func random_loading_screen(context: String) -> Dictionary:
	var screens := loading_screens(context)
	return screens.pick_random() if not screens.is_empty() else {}


## Vignette d'un genre d'événement ({} si aucune).
static func vignette_for_kind(kind: String) -> Dictionary:
	for vignette in data().get("vignettes", []):
		if kind in (vignette as Dictionary).get("kinds", []):
			return vignette
	return {}


## Vignette du genre le plus prioritaire parmi `events` (dictionnaires avec `kind`).
static func vignette_for_events(events: Array) -> Dictionary:
	var kinds := {}
	for event in events:
		if event is Dictionary:
			kinds[str(event.get("kind", ""))] = true
	for vignette in data().get("vignettes", []):
		for kind in (vignette as Dictionary).get("kinds", []):
			if kinds.has(kind):
				return vignette
	return {}


## Écran de fin (`battle_victory`, `battle_defeat`, `campaign_victory`, `campaign_defeat`).
static func ending(outcome: String) -> Dictionary:
	for entry in data().get("endings", []):
		if str((entry as Dictionary).get("outcome", "")) == outcome:
			return entry
	return {}


static func texture(entry: Dictionary) -> Texture2D:
	var path := str(entry.get("image", ""))
	return PortraitLoader.load_texture(path) if path != "" else null
