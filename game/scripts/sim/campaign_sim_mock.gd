class_name CampaignSimMock
extends RefCounted

## Simulation factice implémentant l'API `CampaignSim` (docs/design/m2-campaign-loop.md § 2)
## pour développer et tester l'interface tant que `core/` n'expose pas la vraie.
## Données plausibles mais fausses : 3 armées, voisinage tiré de `provinces.geojson`
## (ou de `GameDataStore`), fin de tour = date + 1 saison et 2 événements factices.
## Aucune règle de jeu réelle ici : tout est remplacé par `core/` (Rust) à terme.

const SEASONS := ["Printemps", "Été", "Automne", "Hiver"]
const START_YEAR := 1337
const MAX_MOVEMENT := 3
const STANCES := ["normal", "raid", "siege"]

const FAKE_UNIT_TYPES := [
	{"unit_type": "unit_men_at_arms_foot", "name": "Hommes d'armes à pied", "cost": 600, "upkeep": 50, "strength": 120},
	{"unit_type": "unit_crossbowmen", "name": "Arbalétriers", "cost": 550, "upkeep": 50, "strength": 100},
	{"unit_type": "unit_knights", "name": "Chevaliers", "cost": 1800, "upkeep": 150, "strength": 60},
	{"unit_type": "unit_longbowmen", "name": "Archers longs", "cost": 500, "upkeep": 45, "strength": 100},
]
const FAKE_TREASURY := {"fac_france": 60000, "fac_england": 40000, "fac_burgundy": 15000}
const FAKE_GENERALS := {"fac_france": "Philippe VI", "fac_england": "Édouard III", "fac_burgundy": "Eudes IV"}

var _turn: int = -1
var _seed: int = 0
var _player: String = ""
var _rng := RandomNumberGenerator.new()

## province_id → {name, owner, controller, neighbors: Array[String], garrison: Array, unrest, devastation, population_total}
var _provinces: Dictionary = {}
## faction_id → {treasury, income, at_war_with: Array, allies: Array, alive}
var _factions: Dictionary = {}
## army_id → {faction, general, general_name, location, units: Array, movement_points, supply, stance, path: Array}
var _armies: Dictionary = {}
var _events: Array[Dictionary] = []
var _pending_recruits: Array[Dictionary] = []
var _next_army_number: int = 1


# --- Cycle de vie -------------------------------------------------------------


func new_campaign(data_dir: String, player: String, seed: int) -> bool:
	_turn = 0
	_seed = seed
	_player = player
	_rng.seed = seed
	_provinces.clear()
	_factions.clear()
	_armies.clear()
	_events.clear()
	_pending_recruits.clear()
	_next_army_number = 1
	if not _load_provinces(data_dir):
		push_error("CampaignSimMock: cannot read provinces from %s" % data_dir)
		return false
	_setup_factions()
	_setup_armies()
	return true


func save_to_string() -> String:
	return JSON.stringify({
		"mock_version": 1,
		"turn": _turn,
		"seed": _seed,
		"player": _player,
		"rng_state": _rng.state,
		"provinces": _provinces,
		"factions": _factions,
		"armies": _armies,
		"events": _events,
		"pending_recruits": _pending_recruits,
		"next_army_number": _next_army_number,
	})


func load_from_string(json: String) -> bool:
	var parsed: Variant = JSON.parse_string(json)
	if parsed == null or not (parsed is Dictionary) or not parsed.has("mock_version"):
		return false
	_turn = int(parsed.get("turn", 0))
	_seed = int(parsed.get("seed", 0))
	_player = str(parsed.get("player", ""))
	_rng.seed = _seed
	_rng.state = int(parsed.get("rng_state", 0))
	_provinces = parsed.get("provinces", {})
	_factions = parsed.get("factions", {})
	_armies = parsed.get("armies", {})
	_events.assign(parsed.get("events", []))
	_pending_recruits.assign(parsed.get("pending_recruits", []))
	_next_army_number = int(parsed.get("next_army_number", 1))
	return true


# --- Lecture ------------------------------------------------------------------


func get_turn() -> int:
	return _turn


func get_date_label() -> String:
	if _turn < 0:
		return ""
	return "%s %d" % [SEASONS[_turn % 4], START_YEAR + _turn / 4]


func get_player_faction() -> String:
	return _player


func get_faction_summary(id: String) -> Dictionary:
	var faction: Dictionary = _factions.get(id, {})
	if faction.is_empty():
		return {}
	var province_count := 0
	for province_id in _provinces:
		if _provinces[province_id]["owner"] == id:
			province_count += 1
	var army_count := 0
	for army_id in _armies:
		if _armies[army_id]["faction"] == id:
			army_count += 1
	return {
		"treasury": faction["treasury"],
		"income": faction["income"],
		"at_war_with": faction["at_war_with"],
		"allies": faction["allies"],
		"provinces_count": province_count,
		"armies_count": army_count,
		"alive": faction["alive"],
	}


func get_province_state(id: String) -> Dictionary:
	var province: Dictionary = _provinces.get(id, {})
	if province.is_empty():
		return {}
	var state := {
		"owner": province["owner"],
		"controller": province["controller"],
		"garrison": province["garrison"].duplicate(true),
		"unrest": province["unrest"],
		"devastation": province["devastation"],
		"population_total": province["population_total"],
	}
	if province.has("siege"):
		state["siege"] = province["siege"]
	return state


func get_army_ids() -> PackedStringArray:
	var ids := PackedStringArray(_armies.keys())
	ids.sort()
	return ids


func get_army(id: String) -> Dictionary:
	var army: Dictionary = _armies.get(id, {})
	return army.duplicate(true)


## Provinces atteignables ce tour : parcours en largeur borné par les points de mouvement
## (1 point par lien terrestre). Ne contient pas la province de départ.
func get_reachable(army_id: String) -> Dictionary:
	var army: Dictionary = _armies.get(army_id, {})
	if army.is_empty():
		return {}
	var budget: int = army["movement_points"]
	var costs := {army["location"]: 0}
	var frontier: Array = [army["location"]]
	while not frontier.is_empty():
		var current: String = frontier.pop_front()
		var cost: int = costs[current]
		if cost >= budget:
			continue
		for neighbor in _neighbors_of(current):
			if not costs.has(neighbor):
				costs[neighbor] = cost + 1
				frontier.append(neighbor)
	costs.erase(army["location"])
	return costs


## Plus court chemin (en liens) de l'armée vers `target`, sans la province de départ.
## Vide si inaccessible. Le chemin peut dépasser les points de mouvement (spec § 1.2).
func find_path(army_id: String, target: String) -> PackedStringArray:
	var army: Dictionary = _armies.get(army_id, {})
	if army.is_empty() or not _provinces.has(target):
		return PackedStringArray()
	var start: String = army["location"]
	if start == target:
		return PackedStringArray()
	var previous := {start: ""}
	var frontier: Array = [start]
	while not frontier.is_empty():
		var current: String = frontier.pop_front()
		if current == target:
			break
		for neighbor in _neighbors_of(current):
			if not previous.has(neighbor):
				previous[neighbor] = current
				frontier.append(neighbor)
	if not previous.has(target):
		return PackedStringArray()
	var path := PackedStringArray()
	var node := target
	while node != start:
		path.insert(0, node)
		node = previous[node]
	return path


func get_recruitable(province_id: String) -> Array:
	var province: Dictionary = _provinces.get(province_id, {})
	if province.is_empty():
		return []
	var result: Array = []
	var treasury: int = _factions.get(_player, {}).get("treasury", 0)
	for i in FAKE_UNIT_TYPES.size():
		var unit_type: Dictionary = FAKE_UNIT_TYPES[i]
		var row := {
			"unit_type": unit_type["unit_type"],
			"name": unit_type["name"],
			"cost": unit_type["cost"],
			"upkeep": unit_type["upkeep"],
			"available": true,
			"reason": "",
		}
		if province["owner"] != _player:
			row["available"] = false
			row["reason"] = "Province étrangère"
		elif province["controller"] != _player:
			row["available"] = false
			row["reason"] = "Province occupée"
		elif treasury < unit_type["cost"]:
			row["available"] = false
			row["reason"] = "Trésor insuffisant"
		elif i == FAKE_UNIT_TYPES.size() - 1:
			row["available"] = false
			row["reason"] = "Bâtiment requis : champ de tir"
		result.append(row)
	return result


func get_events() -> Array:
	return _events.duplicate(true)


# --- Ordres -------------------------------------------------------------------


func submit_order(order: Dictionary) -> Dictionary:
	var kind: String = str(order.get("type", ""))
	match kind:
		"move_army":
			return _order_move(order)
		"set_stance":
			return _order_stance(order)
		"recruit":
			return _order_recruit(order)
		"create_army":
			return _order_create_army(order)
		"end_turn":
			end_turn()
			return {"ok": true, "error": ""}
	return {"ok": false, "error": "Ordre inconnu : %s" % kind}


func _order_move(order: Dictionary) -> Dictionary:
	var army_id: String = str(order.get("army", ""))
	if not _armies.has(army_id):
		return {"ok": false, "error": "Armée inconnue"}
	var army: Dictionary = _armies[army_id]
	if army["faction"] != _player:
		return {"ok": false, "error": "Cette armée n'est pas la vôtre"}
	var path: Array = []
	for step in order.get("path", []):
		path.append(str(step))
	var previous: String = army["location"]
	for step in path:
		if not _neighbors_of(previous).has(step):
			return {"ok": false, "error": "%s n'est pas voisine de %s" % [step, previous]}
		previous = step
	army["path"] = path
	return {"ok": true, "error": ""}


func _order_stance(order: Dictionary) -> Dictionary:
	var army_id: String = str(order.get("army", ""))
	var stance: String = str(order.get("stance", "normal"))
	if not _armies.has(army_id):
		return {"ok": false, "error": "Armée inconnue"}
	if not STANCES.has(stance):
		return {"ok": false, "error": "Posture inconnue : %s" % stance}
	_armies[army_id]["stance"] = stance
	return {"ok": true, "error": ""}


func _order_recruit(order: Dictionary) -> Dictionary:
	var province_id: String = str(order.get("province", ""))
	var unit_type: String = str(order.get("unit_type", ""))
	for row in get_recruitable(province_id):
		if row["unit_type"] != unit_type:
			continue
		if not row["available"]:
			return {"ok": false, "error": row["reason"]}
		_factions[_player]["treasury"] -= int(row["cost"])
		_pending_recruits.append({"province": province_id, "unit_type": unit_type})
		return {"ok": true, "error": ""}
	return {"ok": false, "error": "Unité non recrutable ici"}


func _order_create_army(order: Dictionary) -> Dictionary:
	var province_id: String = str(order.get("province", ""))
	var province: Dictionary = _provinces.get(province_id, {})
	if province.is_empty():
		return {"ok": false, "error": "Province inconnue"}
	if province["owner"] != _player:
		return {"ok": false, "error": "Province étrangère"}
	var indices: Array = []
	for value in order.get("units_from_garrison", []):
		indices.append(int(value))
	indices.sort()
	indices.reverse()
	if indices.is_empty():
		return {"ok": false, "error": "Aucune unité choisie"}
	var garrison: Array = province["garrison"]
	var units: Array = []
	for index in indices:
		if index < 0 or index >= garrison.size():
			return {"ok": false, "error": "Unité de garnison invalide"}
	for index in indices:
		units.insert(0, garrison[index])
		garrison.remove_at(index)
	var army_id := "army_%s_%d" % [_player.trim_prefix("fac_"), _next_army_number]
	_next_army_number += 1
	_armies[army_id] = _make_army(_player, province_id, units, "", "")
	return {"ok": true, "error": "", "army": army_id}


# --- Fin de tour --------------------------------------------------------------


func end_turn() -> Array:
	_events.clear()
	if _turn < 0:
		return []
	for army_id in get_army_ids():
		if not _armies.has(army_id):
			continue
		var army: Dictionary = _armies[army_id]
		if army["faction"] != _player and army["path"].is_empty():
			_plan_ai_move(army)
		_advance_army(army_id, army)
	_apply_recruits()
	_apply_economy()
	_turn += 1
	_fake_events()
	return _events.duplicate(true)


func _plan_ai_move(army: Dictionary) -> void:
	var neighbors := _neighbors_of(army["location"])
	if neighbors.is_empty() or _rng.randf() < 0.4:
		return
	army["path"] = [neighbors[_rng.randi_range(0, neighbors.size() - 1)]]


func _advance_army(army_id: String, army: Dictionary) -> void:
	var path: Array = army["path"]
	var budget: int = MAX_MOVEMENT
	while not path.is_empty() and budget > 0:
		var next: String = path.pop_front()
		var enemy := _enemy_army_in(next, army["faction"])
		if enemy != "":
			_resolve_battle(army_id, enemy, next)
			path.clear()
			break
		army["location"] = next
		budget -= 1
	army["movement_points"] = MAX_MOVEMENT
	var province: Dictionary = _provinces.get(army["location"], {})
	if not province.is_empty() and army["stance"] == "siege" and province["owner"] != army["faction"]:
		province["controller"] = army["faction"]
		province["owner"] = army["faction"]
		province.erase("siege")
		_events.append({"kind": "province_taken", "text_fr": "%s tombe aux mains de %s." % [province["name"], _faction_label(army["faction"])], "province": army["location"], "faction": army["faction"]})
	elif not province.is_empty() and army["stance"] == "raid" and province["owner"] != army["faction"]:
		province["devastation"] = mini(int(province["devastation"]) + 30, 100)
		_events.append({"kind": "raid", "text_fr": "Chevauchée en %s : dévastation %d." % [province["name"], province["devastation"]], "province": army["location"], "army": army_id})


func _enemy_army_in(province_id: String, faction: String) -> String:
	for other_id in _armies:
		var other: Dictionary = _armies[other_id]
		if other["location"] == province_id and other["faction"] != faction and _at_war(faction, other["faction"]):
			return other_id
	return ""


func _at_war(a: String, b: String) -> bool:
	return _factions.get(a, {}).get("at_war_with", []).has(b)


func _resolve_battle(attacker_id: String, defender_id: String, province_id: String) -> void:
	var attacker: Dictionary = _armies[attacker_id]
	var defender: Dictionary = _armies[defender_id]
	var attacker_wins := _rng.randf() < 0.5
	var loser: Dictionary = defender if attacker_wins else attacker
	var winner: Dictionary = attacker if attacker_wins else defender
	for unit in loser["units"]:
		unit["strength"] = maxi(int(unit["strength"] * 0.7), 1)
		unit["morale"] = maxf(unit["morale"] - 20.0, 10.0)
	for unit in winner["units"]:
		unit["strength"] = maxi(int(unit["strength"] * 0.9), 1)
	var name: String = _provinces.get(province_id, {}).get("name", province_id)
	_events.append({
		"kind": "battle",
		"text_fr": "Bataille de %s : %s contre %s. Vainqueur : %s (pertes : 30 % pour le vaincu, 10 % pour le vainqueur)." % [
			name, _faction_label(attacker["faction"]), _faction_label(defender["faction"]), _faction_label(winner["faction"])],
		"province": province_id,
		"army": attacker_id,
		"faction": winner["faction"],
	})


func _apply_recruits() -> void:
	for recruit in _pending_recruits:
		var province: Dictionary = _provinces.get(recruit["province"], {})
		if province.is_empty():
			continue
		for unit_type in FAKE_UNIT_TYPES:
			if unit_type["unit_type"] == recruit["unit_type"]:
				province["garrison"].append(_make_unit(unit_type))
				_events.append({"kind": "recruit", "text_fr": "%s : %s rejoignent la garnison." % [province["name"], unit_type["name"]], "province": recruit["province"]})
	_pending_recruits.clear()


func _apply_economy() -> void:
	for faction_id in _factions:
		var income := 0
		for province_id in _provinces:
			var province: Dictionary = _provinces[province_id]
			if province["owner"] == faction_id:
				income += int(120.0 * (1.0 - float(province["devastation"]) / 100.0))
			province["devastation"] = maxi(int(province["devastation"]) - 5, 0)
		var upkeep := 0
		for army_id in _armies:
			if _armies[army_id]["faction"] == faction_id:
				upkeep += 50 * _armies[army_id]["units"].size()
		var faction: Dictionary = _factions[faction_id]
		faction["income"] = income - upkeep
		faction["treasury"] = int(faction["treasury"]) + income - upkeep


func _fake_events() -> void:
	var player_summary := get_faction_summary(_player)
	_events.append({
		"kind": "income",
		"text_fr": "%s : revenus du trésor %+d livres (trésor : %d)." % [get_date_label(), player_summary.get("income", 0), player_summary.get("treasury", 0)],
		"faction": _player,
	})
	var ids := get_army_ids()
	if not ids.is_empty():
		var army: Dictionary = _armies[ids[_turn % ids.size()]]
		var province_name: String = _provinces.get(army["location"], {}).get("name", army["location"])
		_events.append({
			"kind": "movement",
			"text_fr": "L'armée de %s campe en %s." % [_faction_label(army["faction"]), province_name],
			"province": army["location"],
			"army": ids[_turn % ids.size()],
			"faction": army["faction"],
		})


# --- Construction de l'état initial ---------------------------------------------


func _load_provinces(data_dir: String) -> bool:
	var path := data_dir.path_join("map").path_join("provinces.geojson")
	if not FileAccess.file_exists(path):
		return false
	var geo: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if geo == null or not (geo is Dictionary):
		return false
	var store: Object = _data_store()
	var position := 0
	for feature in geo.get("features", []):
		position += 1
		var props: Dictionary = feature.get("properties", {})
		var id: String = str(props.get("id", feature.get("id", "prov_%d" % position)))
		var owner: String = str(props.get("owner", ""))
		var name: String = str(props.get("name", id))
		var neighbors: Array = []
		for neighbor in props.get("neighbors", []):
			neighbors.append(str(neighbor))
		var population := 50000 + (position * 7919) % 400000
		if store != null:
			var info: Dictionary = store.call("get_province", id)
			if not info.is_empty():
				if owner == "":
					owner = str(info.get("owner", ""))
				if neighbors.is_empty():
					for neighbor in info.get("neighbors", PackedStringArray()):
						neighbors.append(str(neighbor))
				population = int(info.get("population_total", population))
		_provinces[id] = {
			"name": name,
			"owner": owner,
			"controller": owner,
			"neighbors": neighbors,
			"garrison": [],
			"unrest": (position * 13) % 20,
			"devastation": 0,
			"population_total": population,
		}
	# Symétrise le voisinage (les GeoJSON partiels ne listent pas toujours les deux sens).
	for id in _provinces:
		for neighbor in _provinces[id]["neighbors"]:
			if _provinces.has(neighbor) and not _provinces[neighbor]["neighbors"].has(id):
				_provinces[neighbor]["neighbors"].append(id)
	return not _provinces.is_empty()


func _setup_factions() -> void:
	var owners: Array = []
	for id in _provinces:
		var owner: String = _provinces[id]["owner"]
		if owner != "" and not owners.has(owner):
			owners.append(owner)
	if not owners.has(_player):
		owners.append(_player)
	for owner in owners:
		_factions[owner] = {
			"treasury": FAKE_TREASURY.get(owner, 8000),
			"income": 0,
			"at_war_with": [],
			"allies": [],
			"alive": true,
		}
	if _factions.has("fac_france") and _factions.has("fac_england"):
		_factions["fac_france"]["at_war_with"].append("fac_england")
		_factions["fac_england"]["at_war_with"].append("fac_france")
	for id in _provinces:
		var province: Dictionary = _provinces[id]
		if province["owner"] == "":
			continue
		var count := 1 + (id.hash() % 3)
		for i in count:
			province["garrison"].append(_make_unit(FAKE_UNIT_TYPES[(i + id.hash()) % 2]))


func _setup_armies() -> void:
	var order: Array = [_player]
	for faction_id in _factions:
		if faction_id != _player:
			order.append(faction_id)
	var created := 0
	for faction_id in order:
		if created >= 3:
			break
		var location := _home_province(faction_id)
		if location == "":
			continue
		var units: Array = []
		var count: int = 8 if faction_id == _player else 4 + created
		for i in count:
			units.append(_make_unit(FAKE_UNIT_TYPES[i % 3]))
		var army_id := "army_%s_%d" % [faction_id.trim_prefix("fac_"), _next_army_number]
		_next_army_number += 1
		var general_name: String = FAKE_GENERALS.get(faction_id, "")
		_armies[army_id] = _make_army(faction_id, location, units, "chr_" + faction_id.trim_prefix("fac_") if general_name != "" else "", general_name)
		created += 1


func _home_province(faction_id: String) -> String:
	var store: Object = _data_store()
	if store != null:
		var capital: String = str(store.call("get_faction", faction_id).get("capital", ""))
		if _provinces.has(capital) and _provinces[capital]["owner"] == faction_id:
			return capital
	for id in _provinces:
		if _provinces[id]["owner"] == faction_id:
			return id
	return ""


func _make_army(faction: String, location: String, units: Array, general: String, general_name: String) -> Dictionary:
	return {
		"faction": faction,
		"general": general,
		"general_name": general_name,
		"location": location,
		"units": units,
		"movement_points": MAX_MOVEMENT,
		"supply": 100,
		"stance": "normal",
		"path": [],
	}


func _make_unit(unit_type: Dictionary) -> Dictionary:
	return {
		"unit_type": unit_type["unit_type"],
		"name": unit_type["name"],
		"strength": unit_type["strength"],
		"max_strength": unit_type["strength"],
		"morale": 80.0,
		"experience": 0,
	}


func _neighbors_of(province_id: String) -> Array:
	return _provinces.get(province_id, {}).get("neighbors", [])


func _faction_label(faction_id: String) -> String:
	var store: Object = _data_store()
	if store != null:
		var short_name: String = str(store.call("get_faction", faction_id).get("short_name", ""))
		if short_name != "":
			return short_name
	return faction_id.trim_prefix("fac_").capitalize()


func _data_store() -> Object:
	var tree: SceneTree = Engine.get_main_loop() as SceneTree
	if tree == null or tree.root == null:
		return null
	var facade: Node = tree.root.get_node_or_null("SimFacade")
	if facade == null:
		return null
	return facade.get("store")
