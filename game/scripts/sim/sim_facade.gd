extends Node

## Autoload `SimFacade` : point d'accès unique de l'UI à la simulation.
##
## `sim` est la vraie `CampaignSim` (GDExtension Rust) quand elle expose l'API M2
## (`get_army_ids`), sinon `CampaignSimMock` (même API, données factices). `store`
## est un `GameDataStore` chargé sur `MapPaths.data_dir` (null si la GDExtension
## manque ou si le dossier n'est pas un jeu de données complet, ex. fixtures).
##
## Transporte aussi la requête de démarrage entre `start_menu.tscn` et
## `campaign_map.tscn` (`pending_faction`, `pending_seed`, `pending_load_path`).

const SAVES_DIR := "user://saves"
const SAVE_VERSION := 1
const DEFAULT_FACTION := "fac_france"
const DEFAULT_SEED := 1337

var store: Object = null
var sim: Object = null
var is_real: bool = false

var pending_faction: String = DEFAULT_FACTION
var pending_seed: int = DEFAULT_SEED
var pending_load_path: String = ""


func _ready() -> void:
	_init_store()
	_init_sim()


func _init_store() -> void:
	if not ClassDB.class_exists("GameDataStore"):
		push_warning("SimFacade: GameDataStore not available (run core/build.sh)")
		return
	var candidate: Object = ClassDB.instantiate("GameDataStore")
	if candidate.call("load", MapPaths.data_dir):
		store = candidate
	else:
		push_warning("SimFacade: GameDataStore could not load %s; faction data unavailable" % MapPaths.data_dir)


func _init_sim() -> void:
	if ClassDB.class_exists("CampaignSim"):
		var candidate: Object = ClassDB.instantiate("CampaignSim")
		if candidate.has_method("get_army_ids"):
			sim = candidate
			is_real = true
			print("SimFacade: using real CampaignSim")
			return
	sim = CampaignSimMock.new()
	is_real = false
	print("SimFacade: using CampaignSimMock")


func store_loaded() -> bool:
	return store != null


func engine_label() -> String:
	return "réelle" if is_real else "factice"


## Démarre une nouvelle campagne ; l'échec d'une vraie sim retombe sur le mock.
func new_campaign(faction: String, seed: int) -> bool:
	if sim.call("new_campaign", MapPaths.data_dir, faction, seed):
		return true
	if is_real:
		push_warning("SimFacade: real CampaignSim refused %s; falling back to mock" % MapPaths.data_dir)
		sim = CampaignSimMock.new()
		is_real = false
		return sim.new_campaign(MapPaths.data_dir, faction, seed)
	return false


# --- Données de faction ----------------------------------------------------------


func faction_info(id: String) -> Dictionary:
	if store != null:
		var info: Dictionary = store.call("get_faction", id)
		if not info.is_empty():
			return info
	return {"id": id, "name": id, "short_name": id.trim_prefix("fac_").capitalize(), "color": Color(0.5, 0.5, 0.5), "blazon": "", "capital": ""}


func faction_color(id: String) -> Color:
	return faction_info(id).get("color", Color(0.5, 0.5, 0.5))


func faction_short_name(id: String) -> String:
	return str(faction_info(id).get("short_name", id))


# --- Sauvegardes ----------------------------------------------------------------


func save_path(save_name: String) -> String:
	return SAVES_DIR.path_join(save_name.validate_filename() + ".json")


## Écrit `user://saves/<nom>.json` : état de la sim + métadonnées.
func save_game(save_name: String) -> bool:
	DirAccess.make_dir_recursive_absolute(SAVES_DIR)
	var wrapper := {
		"version": SAVE_VERSION,
		"engine": "real" if is_real else "mock",
		"faction": str(sim.call("get_player_faction")),
		"date": str(sim.call("get_date_label")),
		"turn": int(sim.call("get_turn")),
		"timestamp": Time.get_datetime_string_from_system(),
		"state": str(sim.call("save_to_string")),
	}
	var file := FileAccess.open(save_path(save_name), FileAccess.WRITE)
	if file == null:
		push_error("SimFacade: cannot write %s" % save_path(save_name))
		return false
	file.store_string(JSON.stringify(wrapper))
	file.close()
	return true


func read_save(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if parsed == null or not (parsed is Dictionary) or not parsed.has("state"):
		return {}
	return parsed


## Restaure l'état de la sim depuis un fichier ; le moteur (réel/factice) doit correspondre.
func load_game(path: String) -> bool:
	var wrapper := read_save(path)
	if wrapper.is_empty():
		push_error("SimFacade: invalid save %s" % path)
		return false
	var engine: String = str(wrapper.get("engine", "mock"))
	if engine == "mock" and is_real:
		push_warning("SimFacade: save made with the mock; loading with mock")
		sim = CampaignSimMock.new()
		is_real = false
	elif engine == "real" and not is_real:
		push_error("SimFacade: save made with the real simulation, unavailable here")
		return false
	return bool(sim.call("load_from_string", str(wrapper["state"])))


## Liste des sauvegardes : [{path, name, faction, date, timestamp}], plus récentes d'abord.
func list_saves() -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	var dir := DirAccess.open(SAVES_DIR)
	if dir == null:
		return result
	for file_name in dir.get_files():
		if not file_name.ends_with(".json"):
			continue
		var path := SAVES_DIR.path_join(file_name)
		var wrapper := read_save(path)
		if wrapper.is_empty():
			continue
		result.append({
			"path": path,
			"name": file_name.get_basename(),
			"faction": str(wrapper.get("faction", "")),
			"date": str(wrapper.get("date", "")),
			"timestamp": str(wrapper.get("timestamp", "")),
			"engine": str(wrapper.get("engine", "mock")),
		})
	result.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a["timestamp"] > b["timestamp"])
	return result
