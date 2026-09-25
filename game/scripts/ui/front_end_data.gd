class_name FrontEndData
extends RefCounted

## Lot MM1 — textes et réglages des écrans d'accueil (`data/ui/front_end.json`, schéma
## `data/schemas/front_end.schema.json`) : décor 3D du menu, présentation des factions, dates de
## départ, introduction, citations, conseils et illustrations des écrans de chargement.

const DATA_PATH := "ui/front_end.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var _data: Dictionary = {}


static func data() -> Dictionary:
	if _data.is_empty():
		var path := _data_dir().path_join(DATA_PATH)
		# Jeux de données réduits (fixtures des tests) : textes d'accueil du jeu complet.
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
		if _data.is_empty():
			push_warning("FrontEndData: %s missing or invalid" % path)
	return _data


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


static func backdrop() -> Dictionary:
	return data().get("backdrop", {})


## Présentations des factions jouables, dans l'ordre du fichier.
static func factions() -> Array:
	return data().get("factions", [])


static func faction(faction_id: String) -> Dictionary:
	for entry in factions():
		if str((entry as Dictionary).get("id", "")) == faction_id:
			return entry
	return {}


static func difficulty_label(level: int) -> String:
	var labels: Array = data().get("difficulty_labels", [])
	return str(labels[clampi(level, 0, labels.size() - 1)]) if not labels.is_empty() else ""


static func start_dates() -> Array:
	return data().get("start_dates", [])


static func intro() -> Dictionary:
	return data().get("intro", {})


static func loading() -> Dictionary:
	return data().get("loading", {})


## Tirage d'une citation {text, author, source, date} ({} si aucune).
static func random_quote(rng: RandomNumberGenerator = null) -> Dictionary:
	var quotes: Array = loading().get("quotes", [])
	if quotes.is_empty():
		return {}
	return quotes[_pick(quotes.size(), rng)]


static func random_tip(rng: RandomNumberGenerator = null) -> String:
	var tips: Array = loading().get("tips", [])
	return str(tips[_pick(tips.size(), rng)]) if not tips.is_empty() else ""


## Illustration d'écran de chargement : de préférence une propre à la faction (une fois sur
## deux), sinon une du fonds commun.
static func random_illustration(faction_id: String = "", rng: RandomNumberGenerator = null) -> String:
	var info := loading()
	var own: Array = (info.get("faction_illustrations", {}) as Dictionary).get(faction_id, [])
	var common: Array = info.get("illustrations", [])
	if not own.is_empty() and (common.is_empty() or _pick(2, rng) == 0):
		return str(own[_pick(own.size(), rng)])
	return str(common[_pick(common.size(), rng)]) if not common.is_empty() else ""


static func _pick(count: int, rng: RandomNumberGenerator) -> int:
	if count <= 1:
		return 0
	return rng.randi_range(0, count - 1) if rng != null else randi() % count
