class_name NextHint
extends RefCounted

## Lot UX2 — choix du conseil « que faire maintenant » (encart parchemin de la carte). Textes,
## actions et seuils dans `data/ui/next_hints.json` (schéma `data/schemas/next_hints.schema.json`),
## par ordre de priorité ; ici seulement les conditions, qui lisent un état déjà exposé par la
## simulation et rassemblé par `NextHintController` :
##
## `{alerts: Array (celles de la cloche, `CampaignAlerts.collect`, avec `province_name`),
##   idle_army: String, idle_army_name: String, treasury: int, income: int,
##   constructions: int, can_build: bool, tutorial_step: int (-1 : pas de guide en attente),
##   dismissed: Array (identifiants masqués par le joueur jusqu'à la saison suivante)}`.
##
## Aucune règle de jeu : un seul conseil, le plus prioritaire, expliqué en une phrase.

const DATA_PATH := "ui/next_hints.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
## Conditions servies par une alerte de la cloche du même type (`kind`).
const ALERT_KINDS := ["chronicle_decision", "siege", "enemy_army", "debt", "research_idle"]

static var _data: Dictionary = {}


static func data() -> Dictionary:
	if _data.is_empty():
		var path := _data_dir().path_join(DATA_PATH)
		# Jeux de données réduits (fixtures des tests) : textes du jeu complet.
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
		if _data.is_empty():
			push_warning("NextHint: %s missing or invalid" % path)
	return _data


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


## Conseil à afficher pour `state` : `{id, title, text, action, alert?, army_id?}`, ou `{}`.
static func choose(state: Dictionary, config: Dictionary = {}) -> Dictionary:
	if config.is_empty():
		config = data()
	var dismissed: Array = state.get("dismissed", [])
	for hint: Dictionary in config.get("hints", []):
		var id := str(hint.get("id", ""))
		if id in dismissed:
			continue
		var found := _match(id, state, config)
		if found.is_empty():
			continue
		var values: Dictionary = found.get("values", {})
		var result := {
			"id": id,
			"title": str(hint.get("title", "")).format(values),
			"text": str(hint.get("text", "")).format(values),
			"action": str(hint.get("action", "")),
		}
		for key in found:
			if key != "values":
				result[key] = found[key]
		return result
	return {}


## `{values, alert?, army_id?}` si la condition `id` tient, `{}` sinon.
static func _match(id: String, state: Dictionary, config: Dictionary) -> Dictionary:
	if id in ALERT_KINDS:
		for alert: Dictionary in state.get("alerts", []):
			if str(alert.get("kind", "")) == id:
				var province := str(alert.get("province_name", ""))
				return {"values": {"province": province if province != "" else "une de vos provinces"}, "alert": alert}
		return {}
	match id:
		"army_idle":
			var army_id := str(state.get("idle_army", ""))
			if army_id == "":
				return {}
			var army_name := str(state.get("idle_army_name", ""))
			return {"values": {"army": army_name if army_name != "" else "votre armée"}, "army_id": army_id}
		"idle_treasury":
			if not idle_treasury(state, config):
				return {}
			return {"values": {"treasury": Money.amount(int(state.get("treasury", 0)))}}
		"tutorial_postponed":
			var step := int(state.get("tutorial_step", -1))
			return {"values": {"step": step + 1}} if step >= 0 else {}
		"end_turn":
			return {"values": {}}
	return {}


## Vrai si le trésor dépasse le seuil et qu'aucun chantier n'est ouvert alors qu'on peut bâtir.
static func idle_treasury(state: Dictionary, config: Dictionary) -> bool:
	if not bool(state.get("can_build", true)) or int(state.get("constructions", 0)) > 0:
		return false
	var treasury := int(state.get("treasury", 0))
	var threshold := maxf(float(config.get("idle_treasury_min", 2000)),
		float(state.get("income", 0)) * float(config.get("idle_treasury_income_seasons", 3)))
	return treasury >= threshold


## Vrai si l'armée (dictionnaire `get_army`) n'a reçu aucun ordre cette saison : ni chemin, ni
## pas fait, ni siège, ni embarquement.
static func army_is_idle(army: Dictionary) -> bool:
	if army.is_empty() or bool(army.get("embarked", false)) or str(army.get("stance", "")) == "siege":
		return false
	if not (army.get("path", []) as Array).is_empty():
		return false
	var planned: Variant = army.get("planned_path", [])
	if (planned is Array or planned is PackedVector2Array) and planned.size() > 0:
		return false
	var left := int(army.get("movement_left", army.get("movement_points", 0)))
	var allowance := int(army.get("movement_max", left))
	return left > 0 and left >= allowance
