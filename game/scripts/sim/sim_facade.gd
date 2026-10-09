extends Node

## Autoload `SimFacade` : point d'accès unique de l'UI à la simulation.
##
## `sim` est la `CampaignSim` native (GDExtension Rust), null si la GDExtension manque
## (`is_real` faux). `store` est un `GameDataStore` chargé sur `MapPaths.data_dir` (null si la GDExtension
## manque ou si le dossier n'est pas un jeu de données complet, ex. fixtures).
##
## Transporte aussi la requête de démarrage entre `start_menu.tscn` et
## `campaign_map.tscn` (`pending_faction`, `pending_seed`, `pending_load_path`).

## T2 : redirigeable par `use_test_saves_dir` (smoke test, bancs de perf) pour ne pas partager
## `user://saves` entre plusieurs exécutions en parallèle (plusieurs worktrees d'agents pointent
## vers le même `user://` Godot, dérivé du nom du projet et non du chemin sur disque).
var SAVES_DIR := "user://saves"
const SAVE_VERSION := 1
const DEFAULT_FACTION := "fac_france"
const DEFAULT_SEED := 1337
const DEFAULT_DIFFICULTY := "normal"

var store: Object = null
var sim: Object = null
var is_real: bool = false

var pending_faction: String = DEFAULT_FACTION
var pending_seed: int = DEFAULT_SEED
## DF1 : niveau de difficulté choisi sur l'écran de faction, appliqué juste après
## `new_campaign` (`easy`, `normal`, `hard`, `very_hard`).
var pending_difficulty: String = DEFAULT_DIFFICULTY
var pending_load_path: String = ""


func _ready() -> void:
	_init_store()
	_init_sim()
	print("SimFacade: %s" % ("CampaignSim ready" if is_real else "CampaignSim unavailable (run core/build.sh)"))


## Vrai si `data_dir` contient les données de jeu complètes (pas seulement `map/`).
static func has_game_data(data_dir: String) -> bool:
	return DirAccess.dir_exists_absolute(data_dir.path_join("factions"))


func _init_store() -> void:
	if not ClassDB.class_exists("GameDataStore"):
		push_warning("SimFacade: GameDataStore not available (run core/build.sh)")
		return
	if not has_game_data(MapPaths.data_dir):
		push_warning("SimFacade: %s has no game data (factions/); store unavailable" % MapPaths.data_dir)
		return
	var candidate: Object = ClassDB.instantiate("GameDataStore")
	if candidate.call("load", MapPaths.data_dir):
		store = candidate
	else:
		push_warning("SimFacade: GameDataStore could not load %s; faction data unavailable" % MapPaths.data_dir)


func _init_sim() -> void:
	is_real = ClassDB.class_exists("CampaignSim")
	sim = ClassDB.instantiate("CampaignSim") if is_real else null


func store_loaded() -> bool:
	return store != null


## T2 : isole les sauvegardes (smoke test) dans un dossier dédié à cette exécution.
func use_test_saves_dir(dir: String) -> void:
	SAVES_DIR = dir


## Repointe `data/` (tests) : recharge le store et une simulation neuve.
func set_data_dir(data_dir: String) -> void:
	MapPaths.data_dir = data_dir
	store = null
	_init_store()
	_init_sim()


## Démarre une nouvelle campagne sur une simulation neuve ; faux si la sim est absente
## ou refuse le dossier de données.
func new_campaign(faction: String, seed: int) -> bool:
	_init_sim()
	if sim == null:
		push_error("SimFacade: CampaignSim unavailable (run core/build.sh)")
		return false
	if not sim.call("new_campaign", MapPaths.data_dir, faction, seed):
		push_warning("SimFacade: CampaignSim refused %s" % MapPaths.data_dir)
		return false
	_apply_pending_difficulty()
	return true


# --- Difficulté (DF1) -----------------------------------------------------------


## Fixe le niveau choisi sur la campagne qui vient d'être créée (figé ensuite par la sim).
func _apply_pending_difficulty() -> void:
	if sim != null and sim.has_method("set_difficulty"):
		if not sim.call("set_difficulty", pending_difficulty):
			push_warning("SimFacade: difficulty '%s' refused" % pending_difficulty)


## Les quatre niveaux, du plus facile au plus difficile : `[{id, label, description,
## effects, summary, default}]` (vide si la sim ne les expose pas).
func difficulty_levels() -> Array:
	var levels: Array = []
	if sim != null and sim.has_method("get_difficulty_levels"):
		levels = sim.call("get_difficulty_levels")
	# Sim absente ou sans campagne (tests) : le cœur connaît les niveaux par défaut.
	if levels.is_empty() and ClassDB.class_exists("CampaignSim"):
		var core: Object = ClassDB.instantiate("CampaignSim")
		if core.has_method("get_difficulty_levels"):
			levels = core.call("get_difficulty_levels")
	return levels


## Niveau de la campagne en cours (`normal` si la sim ne le connaît pas).
func current_difficulty() -> String:
	if sim != null and sim.has_method("get_difficulty"):
		return str(sim.call("get_difficulty"))
	return DEFAULT_DIFFICULTY


## Libellé français du niveau `id` (« Difficile »), l'id lui-même à défaut.
func difficulty_label(id: String) -> String:
	for level in difficulty_levels():
		if str((level as Dictionary).get("id", "")) == id:
			return str((level as Dictionary).get("label", id))
	return id


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


## NT5 (N6) : unités par armée au plus (`data/rules/armies.json`, règle du cœur).
func army_unit_cap() -> int:
	if sim != null and sim.has_method("army_unit_cap"):
		return int(sim.call("army_unit_cap"))
	return 20


# --- Sauvegardes ----------------------------------------------------------------


func save_path(save_name: String) -> String:
	return SAVES_DIR.path_join(save_name.validate_filename() + ".json")


## Écrit `user://saves/<nom>.json` : état de la sim + métadonnées.
func save_game(save_name: String) -> bool:
	if sim == null:
		return false
	var state := str(sim.call("save_to_string"))
	if state.is_empty():
		push_error("SimFacade: no campaign state, refusing to overwrite %s" % save_name)
		return false
	DirAccess.make_dir_recursive_absolute(SAVES_DIR)
	var wrapper := {
		"version": SAVE_VERSION,
		"faction": str(sim.call("get_player_faction")),
		"date": str(sim.call("get_date_label")),
		"turn": int(sim.call("get_turn")),
		"difficulty": current_difficulty(),
		"timestamp": Time.get_datetime_string_from_system(),
		"state": state,
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


## Raison (en français) du dernier refus de `load_game`, pour l'affichage.
var last_load_error: String = ""


## Restaure l'état de la sim depuis un fichier ; le moteur (réel/factice) doit correspondre.
func load_game(path: String) -> bool:
	last_load_error = ""
	var wrapper := read_save(path)
	if wrapper.is_empty():
		push_error("SimFacade: invalid save %s" % path)
		return false
	if sim == null or str(wrapper.get("engine", "real")) == "mock":
		push_error("SimFacade: save %s cannot be loaded (no CampaignSim, or legacy mock save)" % path)
		return false
	# The running campaign is only replaced once the save has loaded: a refused
	# save must not leave an empty sim that the next autosave would write out.
	var previous_sim: Object = sim
	_init_sim()
	if not bool(sim.call("load_from_string", str(wrapper["state"]))):
		last_load_error = str(sim.call("get_load_error"))
		sim = previous_sim
		return false
	return true


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
		})
	result.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a["timestamp"] > b["timestamp"])
	return result
