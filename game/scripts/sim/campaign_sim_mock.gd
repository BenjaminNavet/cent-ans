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

const CLASS_IDS := ["peasants", "burghers", "clergy", "nobility"]
const CLASS_SHARE := {"peasants": 0.60, "burghers": 0.25, "clergy": 0.05, "nobility": 0.10}
const CLASS_WEALTH_BASE := {"peasants": 30.0, "burghers": 55.0, "clergy": 50.0, "nobility": 70.0}
const TAX_RATES := ["low", "normal", "high"]
const TAX_MULTIPLIER := {"low": 0.7, "normal": 1.0, "high": 1.4}
const TAX_UNREST_PCT := {"low": 0.0, "normal": 15.0, "high": 30.0}

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
## DF1 : niveau de difficulté (le mock ne l'applique pas, il le mémorise seulement).
var _difficulty: String = "normal"

## province_id → {name, owner, controller, neighbors: Array[String], garrison: Array, unrest, devastation, population_total}
var _provinces: Dictionary = {}
## faction_id → {treasury, income, at_war_with: Array, allies: Array, alive}
var _factions: Dictionary = {}
## army_id → {faction, general, general_name, location, units: Array, movement_points, supply, stance, path: Array}
var _armies: Dictionary = {}
var _events: Array[Dictionary] = []
var _pending_recruits: Array[Dictionary] = []
var _next_army_number: int = 1
## building_id → {name, category, cost, turns, upkeep, required_building, required_resource,
## requires_coastal, upgrades_from, effects} — chargé depuis `data/buildings/*.json`.
var _buildings: Dictionary = {}
## resource_id → {name, category} — chargé depuis `data/resources/*.json`.
var _resources: Dictionary = {}
## trait_id → {name, category, description} — chargé depuis `data/traits/*.json` si présent.
var _traits: Dictionary = {}
## character_id → {name, epithet, sex, house, faction, role, birth_year, death_year, alive,
## experience, skill_points, skills_learned: Array[String], traits: Array[String],
## spouse, children: Array[String], father, mother, location, army, governor_of, captive,
## piety, prestige, titles: Array[String]}.
var _characters: Dictionary = {}
var _next_generated_id: int = 1
## [{id, name, branch, tier, prerequisites[], cost, description}] — arbre de compétences,
## chargé depuis `data/skills/*.json` si le dossier existe, sinon `HARDCODED_SKILL_TREE`.
var _skill_tree: Array = []


# --- Cycle de vie -------------------------------------------------------------


func new_campaign(data_dir: String, player: String, seed: int) -> bool:
	_turn = 0
	_difficulty = "normal"
	_seed = seed
	_player = player
	_rng.seed = seed
	_provinces.clear()
	_factions.clear()
	_armies.clear()
	_events.clear()
	_pending_recruits.clear()
	_next_army_number = 1
	_characters.clear()
	_next_generated_id = 1
	_load_catalogs(data_dir)
	if not _load_provinces(data_dir):
		push_error("CampaignSimMock: cannot read provinces from %s" % data_dir)
		return false
	_setup_factions()
	_load_characters(data_dir)
	_load_skill_tree(data_dir)
	_setup_armies()
	_setup_city(data_dir)
	return true


func save_to_string() -> String:
	return JSON.stringify({
		"mock_version": 1,
		"turn": _turn,
		"seed": _seed,
		"player": _player,
		"difficulty": _difficulty,
		"rng_state": _rng.state,
		"provinces": _provinces,
		"factions": _factions,
		"armies": _armies,
		"events": _events,
		"pending_recruits": _pending_recruits,
		"next_army_number": _next_army_number,
		"characters": _characters,
		"next_generated_id": _next_generated_id,
	})


func load_from_string(json: String) -> bool:
	var parsed: Variant = JSON.parse_string(json)
	if parsed == null or not (parsed is Dictionary) or not parsed.has("mock_version"):
		return false
	_turn = int(parsed.get("turn", 0))
	_seed = int(parsed.get("seed", 0))
	_player = str(parsed.get("player", ""))
	_difficulty = str(parsed.get("difficulty", "normal"))
	_rng.seed = _seed
	_rng.state = int(parsed.get("rng_state", 0))
	_provinces = parsed.get("provinces", {})
	_factions = parsed.get("factions", {})
	_armies = parsed.get("armies", {})
	_events.assign(parsed.get("events", []))
	_pending_recruits.assign(parsed.get("pending_recruits", []))
	_next_army_number = int(parsed.get("next_army_number", 1))
	_characters = parsed.get("characters", {})
	_next_generated_id = int(parsed.get("next_generated_id", 1))
	var data_dir := _map_paths_data_dir()
	if data_dir != "":
		_load_catalogs(data_dir)
		_load_skill_tree(data_dir)
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
	var army_upkeep := _army_upkeep_of(id)
	var building_upkeep := _building_upkeep_of(id)
	return {
		"treasury": faction["treasury"],
		"income": faction["income"],
		"at_war_with": faction["at_war_with"],
		"allies": faction["allies"],
		"provinces_count": province_count,
		"armies_count": army_count,
		"alive": faction["alive"],
		"projected_income": _projected_income(id, building_upkeep, army_upkeep),
		"army_upkeep": army_upkeep,
		"building_upkeep": building_upkeep,
		"tax_rate": str(faction.get("tax_rate", "normal")),
	}


func get_province_state(id: String) -> Dictionary:
	var province: Dictionary = _provinces.get(id, {})
	if province.is_empty():
		return {}
	var state := {
		"owner": province["owner"],
		"controller": province["controller"],
		"garrison": province["garrison"].duplicate(true),
		# B7a : comme le pont, le mécontentement qui déclenche les révoltes (moyenne pondérée
		# des classes) ; la jauge propre de la province est le désordre.
		"unrest": roundi(weighted_unrest(id)) if not (province.get("classes", {}) as Dictionary).is_empty() else province["unrest"],
		"disorder": province["unrest"],
		"devastation": province["devastation"],
		"population_total": province["population_total"],
	}
	if province.has("siege"):
		state["siege"] = province["siege"]
	var governor: String = str(province.get("governor", ""))
	if governor != "" and _characters.has(governor) and bool(_characters[governor]["alive"]) and str(_characters[governor]["governor_of"]) == id:
		state["governor"] = governor
		state["governor_name"] = str(_characters[governor]["name"])
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


# --- Ville et économie (docs/design/m3-cities-economy.md § 2) -----------------


## `{classes: {peasants: {count, unrest, health, wealth, goods_satisfaction}, ...},
## buildings: [{id, name, category, upkeep}], construction: {building, name, turns_left}
## (absent si aucune), fortification_level, capacity, buildable: [{building, name, category,
## cost, turns, available, reason}], resources: [ids], effects: {..: valeur}}`.
func get_province_city(id: String) -> Dictionary:
	var province: Dictionary = _provinces.get(id, {})
	if province.is_empty():
		return {}
	var classes := {}
	for class_id in CLASS_IDS:
		classes[class_id] = (province.get("classes", {}) as Dictionary).get(class_id, {}).duplicate()
	var buildings: Array = []
	for bld_id in province.get("buildings", []):
		var info: Dictionary = _buildings.get(bld_id, {})
		buildings.append({
			"id": bld_id, "name": info.get("name", bld_id),
			"category": info.get("category", ""), "upkeep": int(info.get("upkeep", 0)),
		})
	var effects: Dictionary = _province_effects(province)
	var fortification_bonus: float = float((effects.get("fortification_level", {}) as Dictionary).get("flat", 0.0))
	var result := {
		"classes": classes,
		"buildings": buildings,
		"fortification_level": int(province.get("fortification_level", 0)) + int(fortification_bonus),
		"capacity": _capacity_of(province),
		"buildable": _buildable_list(province),
		"resources": (province.get("resources", []) as Array).duplicate(),
		"effects": effects,
	}
	var construction: Variant = province.get("construction")
	if construction != null:
		var info: Dictionary = _buildings.get((construction as Dictionary)["building"], {})
		result["construction"] = {
			"building": construction["building"],
			"name": info.get("name", construction["building"]),
			"turns_left": int(construction["turns_left"]),
		}
	return result


## `{treasury, income, projected_income, army_upkeep, building_upkeep, tax_rate,
## goods: {resource_id: count}, goods_categories: [..]}`.
func get_faction_economy(id: String) -> Dictionary:
	var faction: Dictionary = _factions.get(id, {})
	if faction.is_empty():
		return {}
	var army_upkeep := _army_upkeep_of(id)
	var building_upkeep := _building_upkeep_of(id)
	var goods := {}
	var categories: Array = []
	for province_id in _provinces:
		var province: Dictionary = _provinces[province_id]
		if province["owner"] != id:
			continue
		for res_id in province.get("resources", []):
			goods[res_id] = int(goods.get(res_id, 0)) + 1
			var category: String = str(_resources.get(res_id, {}).get("category", ""))
			if category != "" and not categories.has(category):
				categories.append(category)
	return {
		"treasury": int(faction.get("treasury", 0)),
		"income": int(faction.get("income", 0)),
		"projected_income": _projected_income(id, building_upkeep, army_upkeep),
		# Le mock ajoute ce montant tel quel au trésor : c'est déjà un solde net.
		"net_income": _projected_income(id, building_upkeep, army_upkeep),
		"net_income_last_turn": int(faction.get("income", 0)),
		"army_upkeep": army_upkeep,
		"building_upkeep": building_upkeep,
		"tax_rate": str(faction.get("tax_rate", "normal")),
		"goods": goods,
		"goods_categories": categories,
	}


func _capacity_of(province: Dictionary) -> int:
	var production_levels := 0
	for bld_id in province.get("buildings", []):
		if str(_buildings.get(bld_id, {}).get("category", "")) == "production":
			production_levels += 1
	return 40000 * (1 + production_levels)


## Somme des `value` d'`effects` de tous les bâtiments, par type d'effet : `{flat, percent}`
## (docs/design/data-model.md § 7.4, même forme que `CampaignSim.get_province_city`).
func _province_effects(province: Dictionary) -> Dictionary:
	var totals := {}
	for bld_id in province.get("buildings", []):
		for effect in _buildings.get(bld_id, {}).get("effects", []):
			var key: String = str(effect.get("effect", ""))
			if not totals.has(key):
				totals[key] = {"flat": 0.0, "percent": 0.0}
			var bucket: String = "percent" if str(effect.get("mode", "flat")) == "percent" else "flat"
			totals[key][bucket] = float(totals[key][bucket]) + float(effect.get("value", 0))
	return totals


func _buildable_list(province: Dictionary) -> Array:
	var result: Array = []
	var treasury: int = int(_factions.get(_player, {}).get("treasury", 0))
	var owner_ok: bool = province["owner"] == _player and province["controller"] == _player
	var built: Array = province.get("buildings", [])
	var ids := _buildings.keys()
	ids.sort()
	for bld_id in ids:
		if _provides(built, bld_id):
			continue
		var info: Dictionary = _buildings[bld_id]
		var cost := int(info.get("cost", 0))
		var reasons: Array[String] = []
		if not owner_ok:
			reasons.append("Province non contrôlée")
		if province.get("construction") != null:
			reasons.append("Construction déjà en cours")
		var required_building: String = str(info.get("required_building", ""))
		if required_building != "" and not _provides(built, required_building):
			reasons.append("Bâtiment requis : %s" % _buildings.get(required_building, {}).get("name", required_building))
		var upgrades_from: String = str(info.get("upgrades_from", ""))
		if upgrades_from != "" and not built.has(upgrades_from):
			reasons.append("Bâtiment requis : %s" % _buildings.get(upgrades_from, {}).get("name", upgrades_from))
		var required_resource: String = str(info.get("required_resource", ""))
		if required_resource != "" and not (province.get("resources", []) as Array).has(required_resource):
			reasons.append("Ressource requise : %s" % _resources.get(required_resource, {}).get("name", required_resource))
		if bool(info.get("requires_coastal", false)) and not bool(province.get("coastal", false)):
			reasons.append("Province côtière requise")
		if reasons.is_empty() and treasury < cost:
			reasons.append("Trésor insuffisant")
		result.append({
			"building": bld_id, "name": info.get("name", bld_id), "category": info.get("category", ""),
			"cost": cost, "turns": int(info.get("turns", 1)),
			"available": reasons.is_empty(), "reason": reasons[0] if not reasons.is_empty() else "",
		})
	return result


func _army_upkeep_of(faction_id: String) -> int:
	var upkeep := 0
	for army_id in _armies:
		if _armies[army_id]["faction"] == faction_id:
			upkeep += 50 * _armies[army_id]["units"].size()
	return upkeep


func _building_upkeep_of(faction_id: String) -> int:
	var upkeep := 0
	for province_id in _provinces:
		var province: Dictionary = _provinces[province_id]
		if province["owner"] != faction_id:
			continue
		for bld_id in province.get("buildings", []):
			upkeep += int(_buildings.get(bld_id, {}).get("upkeep", 0))
	return upkeep


func _projected_income(faction_id: String, building_upkeep: int, army_upkeep: int) -> int:
	var rate: String = str(_factions.get(faction_id, {}).get("tax_rate", "normal"))
	var mult: float = TAX_MULTIPLIER.get(rate, 1.0)
	var gross := 0
	for province_id in _provinces:
		var province: Dictionary = _provinces[province_id]
		if province["owner"] == faction_id:
			gross += _province_tax_income(province, mult)
	return gross - building_upkeep - army_upkeep


func _province_tax_income(province: Dictionary, mult: float) -> int:
	var base := 0.0
	for class_id in CLASS_IDS:
		var c: Dictionary = (province.get("classes", {}) as Dictionary).get(class_id, {})
		base += float(c.get("count", 0)) * 0.0006 * float(c.get("wealth", 50)) / 50.0
	var devastation_factor := 1.0 - float(province.get("devastation", 0)) / 100.0
	var effects: Dictionary = _province_effects(province)
	var trade_bonus := 1.0 + float((effects.get("trade_income", {}) as Dictionary).get("percent", 0.0)) / 100.0
	var tax_bonus := 1.0 + float((effects.get("tax_income", {}) as Dictionary).get("percent", 0.0)) / 100.0
	return int(base * devastation_factor * mult * tax_bonus * trade_bonus)


## Moyenne du mécontentement des classes, pondérée par effectif ; 0 si population nulle.
func weighted_unrest(province_id: String) -> float:
	var province: Dictionary = _provinces.get(province_id, {})
	if province.is_empty():
		return 0.0
	var total := 0.0
	var weighted := 0.0
	for class_id in CLASS_IDS:
		var c: Dictionary = (province.get("classes", {}) as Dictionary).get(class_id, {})
		var count := float(c.get("count", 0))
		total += count
		weighted += count * float(c.get("unrest", 0))
	return weighted / total if total > 0.0 else 0.0


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
		"build":
			return _order_build(order)
		"cancel_build":
			return _order_cancel_build(order)
		"set_tax_rate":
			return _order_set_tax_rate(order)
		"learn_skill":
			return _order_learn_skill(order)
		"assign_governor":
			return _order_assign_governor(order)
		"assign_general":
			return _order_assign_general(order)
		"propose_marriage":
			return _order_propose_marriage(order)
		"debug_grant_xp":
			return _order_debug_grant_xp(order)
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


func _order_build(order: Dictionary) -> Dictionary:
	var province_id: String = str(order.get("province", ""))
	var building: String = str(order.get("building", ""))
	var province: Dictionary = _provinces.get(province_id, {})
	if province.is_empty():
		return {"ok": false, "error": "Province inconnue"}
	if not _buildings.has(building):
		return {"ok": false, "error": "Bâtiment inconnu"}
	for row in _buildable_list(province):
		if row["building"] != building:
			continue
		if not row["available"]:
			return {"ok": false, "error": row["reason"]}
		_factions[_player]["treasury"] = int(_factions[_player]["treasury"]) - int(row["cost"])
		province["construction"] = {"building": building, "turns_left": int(row["turns"])}
		return {"ok": true, "error": ""}
	return {"ok": false, "error": "Bâtiment non constructible ici"}


func _order_cancel_build(order: Dictionary) -> Dictionary:
	var province_id: String = str(order.get("province", ""))
	var province: Dictionary = _provinces.get(province_id, {})
	if province.is_empty() or province.get("construction") == null:
		return {"ok": false, "error": "Aucune construction en cours"}
	if province["owner"] != _player:
		return {"ok": false, "error": "Province étrangère"}
	var construction: Dictionary = province["construction"]
	var info: Dictionary = _buildings.get(str(construction["building"]), {})
	var refund := int(round(float(info.get("cost", 0)) * 0.5))
	_factions[_player]["treasury"] = int(_factions[_player]["treasury"]) + refund
	province["construction"] = null
	return {"ok": true, "error": ""}


func _order_set_tax_rate(order: Dictionary) -> Dictionary:
	var rate: String = str(order.get("rate", "normal"))
	if not TAX_RATES.has(rate):
		return {"ok": false, "error": "Taux d'imposition inconnu : %s" % rate}
	_factions[_player]["tax_rate"] = rate
	return {"ok": true, "error": ""}


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
	_apply_construction()
	_apply_population()
	_apply_economy()
	_apply_unrest_events()
	_apply_character_events()
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
		_events.append({"kind": "province_captured", "text_fr": "%s tombe aux mains de %s." % [province["name"], _faction_label(army["faction"])], "province": army["location"], "faction": army["faction"]})
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
		"text_fr": "Bataille de %s : %s contre %s. Vainqueur : %s (pertes : 30 %% pour le vaincu, 10 %% pour le vainqueur)." % [
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
				_events.append({"kind": "recruited", "text_fr": "%s : %s rejoignent la garnison." % [province["name"], unit_type["name"]], "province": recruit["province"]})
	_pending_recruits.clear()


func _apply_construction() -> void:
	for province_id in _provinces:
		var province: Dictionary = _provinces[province_id]
		var construction: Variant = province.get("construction")
		if construction == null:
			continue
		var c: Dictionary = construction
		c["turns_left"] = int(c["turns_left"]) - 1
		if int(c["turns_left"]) > 0:
			continue
		var bld_id: String = str(c["building"])
		province["buildings"].append(bld_id)
		province["construction"] = null
		var name: String = str(_buildings.get(bld_id, {}).get("name", bld_id))
		_events.append({"kind": "building_completed", "text_fr": "%s : %s est achevé." % [province["name"], name], "province": province_id})


## Dérive légère des jauges de classe (§ 1.1, version simplifiée pour le mock).
func _apply_population() -> void:
	for province_id in _provinces:
		var province: Dictionary = _provinces[province_id]
		var classes: Dictionary = province.get("classes", {})
		if classes.is_empty():
			continue
		var rate: String = str(_factions.get(province["owner"], {}).get("tax_rate", "normal"))
		var tax_pct: float = TAX_UNREST_PCT.get(rate, 15.0)
		for class_id in CLASS_IDS:
			if not classes.has(class_id):
				continue
			var c: Dictionary = classes[class_id]
			var goods: float = float(c.get("goods_satisfaction", 50))
			var health: float = float(c.get("health", 50))
			var target_unrest: float = tax_pct + float(province.get("devastation", 0)) / 2.0 + (50.0 - goods) / 3.0 + (50.0 - health) / 4.0
			c["unrest"] = int(clampf(lerpf(float(c.get("unrest", 0)), target_unrest, 0.2), 0.0, 100.0))
			var target_health: float = 50.0 + (goods - 50.0) / 4.0
			c["health"] = int(clampf(lerpf(health, target_health, 0.2), 0.0, 100.0))
			# Ancré sur la richesse courante (pas une base absolue) : seuls la fiscalité et
			# l'effet Wealth des bâtiments la font dériver, pas de décroissance systémique.
			var wealth_effect: float = float((_province_effects(province).get("wealth", {}) as Dictionary).get("flat", 0.0))
			var target_wealth: float = float(c.get("wealth", CLASS_WEALTH_BASE.get(class_id, 40.0))) - (tax_pct - 15.0) * 0.3 + wealth_effect
			c["wealth"] = int(clampf(lerpf(float(c.get("wealth", 40)), target_wealth, 0.15), 0.0, 100.0))


func _apply_economy() -> void:
	for faction_id in _factions:
		var army_upkeep := _army_upkeep_of(faction_id)
		var building_upkeep := _building_upkeep_of(faction_id)
		var income := _projected_income(faction_id, building_upkeep, army_upkeep)
		var faction: Dictionary = _factions[faction_id]
		faction["income"] = income
		faction["treasury"] = int(faction["treasury"]) + income
		for province_id in _provinces:
			var province: Dictionary = _provinces[province_id]
			if province["owner"] == faction_id:
				province["devastation"] = maxi(int(province["devastation"]) - 5, 0)


## Révolte (mécontentement pondéré > 75 pendant 2 saisons), peste (santé moyenne < 30),
## famine (dévastation > 70 en hiver) : versions simplifiées pour le mock (§ 1.5).
func _apply_unrest_events() -> void:
	var is_winter := _turn % 4 == 3
	for province_id in _provinces:
		var province: Dictionary = _provinces[province_id]
		var classes: Dictionary = province.get("classes", {})
		if classes.is_empty() or province["owner"] == "":
			continue
		if weighted_unrest(province_id) > 75.0:
			province["unrest_streak"] = int(province.get("unrest_streak", 0)) + 1
		else:
			province["unrest_streak"] = 0
		if int(province.get("unrest_streak", 0)) >= 2:
			province["unrest_streak"] = 0
			var garrison: Array = province["garrison"]
			province["garrison"] = garrison.slice(0, int(ceil(garrison.size() * 0.75)))
			_events.append({"kind": "revolt", "text_fr": "Révolte en %s : la garnison est réduite." % province["name"], "province": province_id})
		var avg_health := 0.0
		for class_id in CLASS_IDS:
			avg_health += float(classes.get(class_id, {}).get("health", 50))
		avg_health /= CLASS_IDS.size()
		if avg_health < 30.0:
			for class_id in CLASS_IDS:
				if not classes.has(class_id):
					continue
				var c: Dictionary = classes[class_id]
				c["count"] = int(int(c.get("count", 0)) * 0.9)
				c["unrest"] = mini(int(c.get("unrest", 0)) + 20, 100)
			_events.append({"kind": "plague", "text_fr": "Peste en %s : la population décline." % province["name"], "province": province_id})
		if is_winter and int(province.get("devastation", 0)) > 70 and classes.has("peasants"):
			var peasants: Dictionary = classes["peasants"]
			peasants["count"] = int(int(peasants.get("count", 0)) * 0.95)
			_events.append({"kind": "famine", "text_fr": "Famine en %s." % province["name"], "province": province_id})


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


## Catalogues `data/buildings/*.json` et `data/resources/*.json` : vides sur les fixtures
## de test (pas de ces dossiers), la ville reste alors sans bâtiments constructibles.
func _load_catalogs(data_dir: String) -> void:
	_buildings.clear()
	_resources.clear()
	_load_building_dir(data_dir.path_join("buildings"))
	_load_resource_dir(data_dir.path_join("resources"))


func _load_building_dir(path: String) -> void:
	var dir := DirAccess.open(path)
	if dir == null:
		return
	for file_name in dir.get_files():
		if not file_name.ends_with(".json"):
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path.path_join(file_name)))
		if parsed == null or not (parsed is Dictionary):
			continue
		var data: Dictionary = parsed
		var id: String = str(data.get("id", file_name.get_basename()))
		_buildings[id] = {
			"name": str((data.get("name", {}) as Dictionary).get("display", id)),
			"category": str(data.get("category", "")),
			"cost": int((data.get("cost", {}) as Dictionary).get("money", 0)),
			"turns": int(data.get("build_time_turns", 1)),
			"upkeep": int(data.get("upkeep", 0)),
			"required_building": str(data.get("required_building", "")),
			"required_resource": str(data.get("required_resource", "")),
			"requires_coastal": bool(data.get("requires_coastal", false)),
			"upgrades_from": str(data.get("upgrades_from", "")),
			"effects": data.get("effects", []),
		}


func _load_resource_dir(path: String) -> void:
	var dir := DirAccess.open(path)
	if dir == null:
		return
	for file_name in dir.get_files():
		if not file_name.ends_with(".json"):
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path.path_join(file_name)))
		if parsed == null or not (parsed is Dictionary):
			continue
		var data: Dictionary = parsed
		var id: String = str(data.get("id", file_name.get_basename()))
		_resources[id] = {
			"name": str((data.get("name", {}) as Dictionary).get("display", id)),
			"category": str(data.get("category", "")),
		}


## Peuple `classes`, `buildings`, `resources`, `coastal`, `fortification_level` par province
## depuis `data/provinces/<id>.json` (non exposé par `GameDataStore.get_province`). Absent sur
## les fixtures de test : la province garde des classes synthétiques à partir de sa population.
func _setup_city(data_dir: String) -> void:
	var dir_path := data_dir.path_join("provinces")
	for province_id in _provinces:
		var province: Dictionary = _provinces[province_id]
		var file_path := dir_path.path_join(province_id + ".json")
		var data: Dictionary = {}
		if FileAccess.file_exists(file_path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(file_path))
			if parsed is Dictionary:
				data = parsed
		var classes: Dictionary = (data.get("population", {}) as Dictionary).get("classes", {})
		if classes.is_empty():
			classes = _synthetic_classes(int(province.get("population_total", 0)))
		province["classes"] = classes
		province["buildings"] = (data.get("buildings", []) as Array).duplicate()
		province["construction"] = null
		province["resources"] = (data.get("resources", []) as Array).duplicate()
		province["coastal"] = bool(data.get("coastal", false))
		province["fortification_level"] = int(data.get("fortification_level", 0))


## Classes plausibles à partir de la population totale, pour les provinces sans fiche JSON
## (fixtures de test).
func _synthetic_classes(population_total: int) -> Dictionary:
	var classes := {}
	for class_id in CLASS_IDS:
		classes[class_id] = {
			"count": int(population_total * float(CLASS_SHARE[class_id])),
			"unrest": 15, "health": 55,
			"wealth": int(CLASS_WEALTH_BASE[class_id]), "goods_satisfaction": 50,
		}
	return classes


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
		var count: int = 1 + (str(id).hash() % 3)
		for i in count:
			province["garrison"].append(_make_unit(FAKE_UNIT_TYPES[(i + str(id).hash()) % 2]))


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
		var general_id := _pick_general(faction_id)
		var general_name: String = ""
		if general_id != "":
			general_name = str(_characters[general_id]["name"])
			_characters[general_id]["army"] = army_id
		else:
			general_name = FAKE_GENERALS.get(faction_id, "")
			general_id = "chr_" + faction_id.trim_prefix("fac_") if general_name != "" else ""
		_armies[army_id] = _make_army(faction_id, location, units, general_id, general_name)
		created += 1


## Choisit un personnage vivant, non captif, adulte de la faction pour commander l'armée
## initiale : le dirigeant en priorité (plausible en 1337), sinon un commandant connu.
func _pick_general(faction_id: String) -> String:
	var ruler: String = str(_faction_info(faction_id).get("ruler", ""))
	var candidates: Array = []
	for id in _characters:
		var c: Dictionary = _characters[id]
		if str(c["faction"]) != faction_id or not bool(c["alive"]) or bool(c["captive"]):
			continue
		if _age_of(c) < 15:
			continue
		if str(c["role"]) == "commander":
			candidates.append(id)
		elif str(c["role"]) == "ruler":
			ruler = id
	if not candidates.is_empty():
		return candidates[0]
	return ruler


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


func _map_paths_data_dir() -> String:
	var tree: SceneTree = Engine.get_main_loop() as SceneTree
	if tree == null or tree.root == null:
		return ""
	var paths: Node = tree.root.get_node_or_null("MapPaths")
	if paths == null:
		return ""
	return str(paths.get("data_dir"))


# --- Personnages et dynasties (docs/design/m4-characters-dynasties.md § 3) ----------------


const XP_PER_SKILL_POINT := 100
const MAJORITY_AGE := 15
const MARRIAGE_MIN_AGE := 14
const FERTILE_MIN_AGE := 16
const FERTILE_MAX_AGE := 45
const BIRTH_PROBABILITY := 0.25
const ROLE_LABELS_FR := {
	"ruler": "Souverain(e)", "consort": "Conjoint(e) royal(e)", "heir": "Héritier(ère)",
	"prince": "Prince/Princesse", "commander": "Commandant(e)", "noble": "Noble",
	"prelate": "Prélat", "burgher": "Bourgeois(e)", "exile": "Exilé(e)",
	"claimant": "Prétendant(e)", "regent": "Régent(e)",
}
const MALE_NAMES := ["Guillaume", "Jean", "Pierre", "Louis", "Charles", "Robert", "Henri", "Thibaut", "Geoffroy", "Aymeric"]
const FEMALE_NAMES := ["Marguerite", "Jeanne", "Isabelle", "Blanche", "Agnès", "Alix", "Mahaut", "Catherine", "Yolande", "Béatrice"]
const PERSONALITY_TRAITS := ["trait_ambitious", "trait_brave", "trait_pious", "trait_cautious", "trait_generous", "trait_energetic", "trait_resolute", "trait_just"]

## Arbre à trois branches (command/governance/court), tiers 1-3, ≈30 nœuds : repli utilisé
## tant que `data/skills/*.json` n'existe pas (agent `data/` en parallèle, § 1 de la spec).
const HARDCODED_SKILL_TREE := [
	{"id": "cmd_hardiesse", "name": "Hardiesse", "branch": "command", "tier": 1, "prerequisites": [], "cost": 1, "description": "Charge +10 %."},
	{"id": "cmd_ordre_bataille", "name": "Ordre de bataille", "branch": "command", "tier": 1, "prerequisites": [], "cost": 1, "description": "Moral d'armée +5."},
	{"id": "cmd_chevauchee", "name": "Chevauchée", "branch": "command", "tier": 1, "prerequisites": [], "cost": 1, "description": "Mouvement +1."},
	{"id": "cmd_tir_droit", "name": "Tir droit", "branch": "command", "tier": 1, "prerequisites": [], "cost": 1, "description": "Tir à distance +5 %."},
	{"id": "cmd_maitre_sieges", "name": "Maître des sièges", "branch": "command", "tier": 2, "prerequisites": ["cmd_hardiesse"], "cost": 2, "description": "Durée de siège -1 tour."},
	{"id": "cmd_charge_lourde", "name": "Charge lourde", "branch": "command", "tier": 2, "prerequisites": ["cmd_hardiesse"], "cost": 2, "description": "Charge +15 %."},
	{"id": "cmd_discipline", "name": "Discipline", "branch": "command", "tier": 2, "prerequisites": ["cmd_ordre_bataille"], "cost": 2, "description": "Moral +10, défense +5 %."},
	{"id": "cmd_genie_tactique", "name": "Génie tactique", "branch": "command", "tier": 3, "prerequisites": ["cmd_maitre_sieges", "cmd_discipline"], "cost": 3, "description": "Toutes statistiques de commandement +10 %."},
	{"id": "cmd_terreur_des_champs", "name": "Terreur des champs", "branch": "command", "tier": 3, "prerequisites": ["cmd_charge_lourde"], "cost": 3, "description": "Charge +25 %, moral ennemi -10."},
	{"id": "cmd_grand_capitaine", "name": "Grand capitaine", "branch": "command", "tier": 3, "prerequisites": ["cmd_discipline"], "cost": 3, "description": "Défense +15 %."},
	{"id": "gov_bon_justicier", "name": "Bon justicier", "branch": "governance", "tier": 1, "prerequisites": [], "cost": 1, "description": "Mécontentement -5."},
	{"id": "gov_intendant", "name": "Intendant", "branch": "governance", "tier": 1, "prerequisites": [], "cost": 1, "description": "Impôts +5 %."},
	{"id": "gov_batisseur", "name": "Bâtisseur", "branch": "governance", "tier": 1, "prerequisites": [], "cost": 1, "description": "Construction -1 tour."},
	{"id": "gov_agronome", "name": "Agronome", "branch": "governance", "tier": 1, "prerequisites": [], "cost": 1, "description": "Richesse paysanne +5."},
	{"id": "gov_grand_intendant", "name": "Grand intendant", "branch": "governance", "tier": 2, "prerequisites": ["gov_intendant"], "cost": 2, "description": "Impôts +10 %."},
	{"id": "gov_urbaniste", "name": "Urbaniste", "branch": "governance", "tier": 2, "prerequisites": ["gov_batisseur"], "cost": 2, "description": "Construction -2 tours."},
	{"id": "gov_juge_equitable", "name": "Juge équitable", "branch": "governance", "tier": 2, "prerequisites": ["gov_bon_justicier"], "cost": 2, "description": "Mécontentement -10."},
	{"id": "gov_reformateur", "name": "Réformateur", "branch": "governance", "tier": 3, "prerequisites": ["gov_grand_intendant", "gov_juge_equitable"], "cost": 3, "description": "Impôts +15 %, mécontentement -15."},
	{"id": "gov_maitre_des_travaux", "name": "Maître des travaux", "branch": "governance", "tier": 3, "prerequisites": ["gov_urbaniste"], "cost": 3, "description": "Construction -3 tours, coût -10 %."},
	{"id": "gov_pere_du_peuple", "name": "Père du peuple", "branch": "governance", "tier": 3, "prerequisites": ["gov_juge_equitable"], "cost": 3, "description": "Mécontentement -20."},
	{"id": "court_beau_parleur", "name": "Beau parleur", "branch": "court", "tier": 1, "prerequisites": [], "cost": 1, "description": "Diplomatie +2 (M5)."},
	{"id": "court_entremetteur", "name": "Entremetteur", "branch": "court", "tier": 1, "prerequisites": [], "cost": 1, "description": "Facilite les mariages."},
	{"id": "court_piete", "name": "Piété", "branch": "court", "tier": 1, "prerequisites": [], "cost": 1, "description": "Piété +10."},
	{"id": "court_charme", "name": "Charme", "branch": "court", "tier": 1, "prerequisites": [], "cost": 1, "description": "Prestige +5."},
	{"id": "court_diplomate_ne", "name": "Diplomate-né", "branch": "court", "tier": 2, "prerequisites": ["court_beau_parleur"], "cost": 2, "description": "Diplomatie +5 (M5)."},
	{"id": "court_faiseur_de_rois", "name": "Faiseur de rois", "branch": "court", "tier": 2, "prerequisites": ["court_entremetteur"], "cost": 2, "description": "Mariages : prestige supplémentaire."},
	{"id": "court_devot", "name": "Dévot", "branch": "court", "tier": 2, "prerequisites": ["court_piete"], "cost": 2, "description": "Piété +20."},
	{"id": "court_cardinal_gris", "name": "Cardinal gris", "branch": "court", "tier": 3, "prerequisites": ["court_diplomate_ne", "court_devot"], "cost": 3, "description": "Diplomatie +10, piété +10 (M5)."},
	{"id": "court_grand_chambellan", "name": "Grand chambellan", "branch": "court", "tier": 3, "prerequisites": ["court_faiseur_de_rois"], "cost": 3, "description": "Prestige +15."},
	{"id": "court_ame_de_la_cour", "name": "Âme de la cour", "branch": "court", "tier": 3, "prerequisites": ["court_devot"], "cost": 3, "description": "Piété +30."},
]


func _current_year() -> int:
	return START_YEAR + _turn / 4


func _age_of(character: Dictionary) -> int:
	if not bool(character.get("alive", true)):
		var death_year := int(character.get("death_year", character.get("birth_year", _current_year())))
		return maxi(death_year - int(character.get("birth_year", death_year)), 0)
	return maxi(_current_year() - int(character.get("birth_year", _current_year())), 0)


func _faction_info(faction_id: String) -> Dictionary:
	var store: Object = _data_store()
	if store == null:
		return {}
	return store.call("get_faction", faction_id)


## `data/characters/*.json` (docs/design/m4-characters-dynasties.md § 1) : famille royale
## complète pour les factions jouables les mieux documentées (France, Angleterre : ~8
## personnages chacune), effectif variable ailleurs — reflète les données historiques
## réelles plutôt qu'une composition artificielle à 8 partout.
func _load_characters(data_dir: String) -> void:
	var dir := DirAccess.open(data_dir.path_join("characters"))
	if dir == null:
		return
	for file_name in dir.get_files():
		if not file_name.ends_with(".json"):
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join("characters").path_join(file_name)))
		if parsed == null or not (parsed is Dictionary):
			continue
		var data: Dictionary = parsed
		var id: String = str(data.get("id", file_name.get_basename()))
		var birth: String = str((data.get("birth", {}) as Dictionary).get("value", "1300"))
		var death_info: Dictionary = data.get("death", {})
		var titles: Array = []
		for entry in data.get("titles", []):
			titles.append(str(entry.get("title", "")))
		var family: Dictionary = data.get("family", {})
		var spouses: Array = family.get("spouses", [])
		var skills: Dictionary = data.get("skills", {})
		_characters[id] = {
			"id": id,
			"name": str((data.get("name", {}) as Dictionary).get("display", id)),
			"epithet": str(data.get("epithet", "")),
			"sex": str(data.get("sex", "male")),
			"house": str(data.get("house", "")),
			"faction": str(data.get("faction", "")),
			"role": str(data.get("role", "noble")),
			"birth_year": int(birth.substr(0, 4)) if birth.length() >= 4 and birth.substr(0, 4).is_valid_int() else 1300,
			"death_year": int(str(death_info.get("value", "")).substr(0, 4)) if not death_info.is_empty() and str(death_info.get("value", "")).length() >= 4 else -1,
			"alive": true,
			"experience": int(skills.get("command", 0)) * 60 + int(skills.get("governance", 0)) * 60 + int(skills.get("court", 0)) * 60,
			"skill_points": 0,
			"skills_learned": [],
			"traits": (data.get("traits", []) as Array).duplicate(),
			"spouse": str(spouses[0]) if not spouses.is_empty() else "",
			"children": (family.get("children", []) as Array).duplicate(),
			"father": str(family.get("father", "")),
			"mother": str(family.get("mother", "")),
			"location": str(data.get("starting_location", "")),
			"army": "",
			"governor_of": "",
			"captive": false,
			"piety": int(data.get("piety", 50)),
			"prestige": 0,
			"titles": titles,
			"base_skills": {
				"command": int(skills.get("command", 0)),
				"governance": int(skills.get("governance", 0)),
				"court": int(skills.get("court", 0)),
			},
		}
	_load_trait_dir(data_dir.path_join("traits"))


func _load_trait_dir(path: String) -> void:
	_traits.clear()
	var dir := DirAccess.open(path)
	if dir == null:
		return
	for file_name in dir.get_files():
		if not file_name.ends_with(".json"):
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path.path_join(file_name)))
		if parsed == null or not (parsed is Dictionary):
			continue
		var data: Dictionary = parsed
		var id: String = str(data.get("id", file_name.get_basename()))
		_traits[id] = {
			"name": str((data.get("name", {}) as Dictionary).get("display", data.get("name", id))) if data.get("name") is Dictionary else str(data.get("name", id)),
			"category": str(data.get("category", "")),
			"description": str(data.get("description", "")),
		}


## `data/skills/*.json` si présent (agent `data/` en parallèle), sinon `HARDCODED_SKILL_TREE`.
func _load_skill_tree(data_dir: String) -> void:
	var dir := DirAccess.open(data_dir.path_join("skills"))
	if dir == null:
		_skill_tree = HARDCODED_SKILL_TREE.duplicate(true)
		return
	var loaded: Array = []
	for file_name in dir.get_files():
		if not file_name.ends_with(".json"):
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join("skills").path_join(file_name)))
		if parsed == null or not (parsed is Dictionary):
			continue
		var data: Dictionary = parsed
		loaded.append({
			"id": str(data.get("id", file_name.get_basename())),
			"name": str((data.get("name", {}) as Dictionary).get("display", data.get("name", data.get("id", "")))) if data.get("name") is Dictionary else str(data.get("name", data.get("id", ""))),
			"branch": str(data.get("branch", "command")),
			"tier": int(data.get("tier", 1)),
			"prerequisites": (data.get("prerequisites", []) as Array).duplicate(),
			"cost": int(data.get("cost", 1)),
			"description": str(data.get("description", "")),
		})
	_skill_tree = loaded if not loaded.is_empty() else HARDCODED_SKILL_TREE.duplicate(true)


# --- Lecture personnages (docs/design/m4-characters-dynasties.md § 3) ---------------------


func get_character(id: String) -> Dictionary:
	if not _characters.has(id):
		return {}
	var c: Dictionary = _characters[id]
	var age := _age_of(c)
	var traits_out: Array = []
	for trait_id in c["traits"]:
		var info: Dictionary = _traits.get(trait_id, {})
		traits_out.append({
			"id": trait_id,
			"name": str(info.get("name", str(trait_id).trim_prefix("trait_").capitalize())),
			"category": str(info.get("category", "")),
		})
	var children_out: Array = []
	for child_id in c["children"]:
		if _characters.has(child_id):
			var child: Dictionary = _characters[child_id]
			children_out.append({"id": child_id, "name": str(child["name"]), "age": _age_of(child)})
	var spouse_id: String = str(c["spouse"])
	return {
		"id": id,
		"name": str(c["name"]),
		"epithet": str(c["epithet"]),
		"sex": str(c["sex"]),
		"age": age,
		"alive": bool(c["alive"]),
		"faction": str(c["faction"]),
		"house": str(c["house"]),
		"title": _title_of(c),
		"role": _current_role(c),
		"skills": {
			"command": _branch_level(c, "command"),
			"governance": _branch_level(c, "governance"),
			"court": _branch_level(c, "court"),
		},
		"experience": int(c["experience"]),
		"skill_points": int(c["skill_points"]),
		"skills_learned": (c["skills_learned"] as Array).duplicate(),
		"traits": traits_out,
		"spouse": spouse_id,
		"spouse_name": str(_characters.get(spouse_id, {}).get("name", "")),
		"children": children_out,
		"father": str(c["father"]),
		"mother": str(c["mother"]),
		"location": str(c["location"]),
		"army": str(c["army"]),
		"governor_of": str(c["governor_of"]),
		"captive": bool(c["captive"]),
		"piety": int(c["piety"]),
		"prestige": int(c["prestige"]),
	}


## Niveau agrégé d'une branche : base des données (`data/characters` § skills 0-10) plus un
## point par compétence apprise de cette branche (` _skill_tree`).
func _branch_level(c: Dictionary, branch: String) -> int:
	var base := int((c.get("base_skills", {}) as Dictionary).get(branch, 0))
	var learned := 0
	for skill_id in c["skills_learned"]:
		for node in _skill_tree:
			if str(node["id"]) == skill_id and str(node["branch"]) == branch:
				learned += 1
	return mini(base + learned, 10)


func _title_of(c: Dictionary) -> String:
	var titles: Array = c.get("titles", [])
	if not titles.is_empty():
		return str(titles[0])
	if str(c["role"]) == "ruler":
		var info := _faction_info(str(c["faction"]))
		var faction_titles: Array = info.get("titles", [])
		if not faction_titles.is_empty():
			return str(faction_titles[0])
	return str(ROLE_LABELS_FR.get(c["role"], str(c["role"]).capitalize()))


func _current_role(c: Dictionary) -> String:
	if bool(c["captive"]):
		return "Captif(ve)"
	var governor_of: String = str(c["governor_of"])
	if governor_of != "":
		return "gouverneur de %s" % _province_display_name(governor_of)
	var army_id: String = str(c["army"])
	if army_id != "" and _armies.has(army_id):
		return "général de l'armée en %s" % _province_display_name(str(_armies[army_id]["location"]))
	return "à la cour"


func _province_display_name(province_id: String) -> String:
	var store: Object = _data_store()
	if store != null:
		var info: Dictionary = store.call("get_province", province_id)
		if not info.is_empty():
			return str(info.get("display_name", province_id))
	return str(_provinces.get(province_id, {}).get("name", province_id))


## Vivants de la faction, dirigeant puis héritier en tête, puis par âge décroissant.
func get_faction_characters(faction: String) -> Array:
	var info := _faction_info(faction)
	var ruler: String = str(info.get("ruler", ""))
	var heir: String = str(info.get("heir", ""))
	var ids: Array = []
	for id in _characters:
		if str(_characters[id]["faction"]) == faction and bool(_characters[id]["alive"]):
			ids.append(id)
	ids.sort_custom(func(a: String, b: String) -> bool:
		var rank_a := 0 if a == ruler else (1 if a == heir else 2)
		var rank_b := 0 if b == ruler else (1 if b == heir else 2)
		if rank_a != rank_b:
			return rank_a < rank_b
		return _age_of(_characters[a]) > _age_of(_characters[b]))
	return ids


func get_skill_tree() -> Array:
	return _skill_tree.duplicate(true)


func get_learnable(character: String) -> Array:
	if not _characters.has(character):
		return []
	var c: Dictionary = _characters[character]
	var learned: Array = c["skills_learned"]
	var points := int(c["skill_points"])
	var result: Array = []
	for node in _skill_tree:
		var id: String = str(node["id"])
		if learned.has(id):
			continue
		if int(node["cost"]) > points:
			continue
		var prereqs_ok := true
		for prereq in node["prerequisites"]:
			if not learned.has(str(prereq)):
				prereqs_ok = false
				break
		if prereqs_ok:
			result.append(id)
	return result


## Candidats de mariage : même faction (interfactions = diplomatie M5, hors mock), vivants,
## sexe opposé, ≥ 14 ans, non mariés, ni parent/enfant/frère-sœur.
func get_marriage_candidates(character: String) -> Array:
	if not _characters.has(character):
		return []
	var c: Dictionary = _characters[character]
	var result: Array = []
	for id in _characters:
		if id == character:
			continue
		var other: Dictionary = _characters[id]
		if not _can_marry(c, other):
			continue
		result.append({"id": id, "name": str(other["name"]), "age": _age_of(other), "faction": str(other["faction"])})
	return result


func _can_marry(a: Dictionary, b: Dictionary) -> bool:
	if not bool(a["alive"]) or not bool(b["alive"]):
		return false
	if str(a["faction"]) != str(b["faction"]):
		return false
	if str(a["sex"]) == str(b["sex"]):
		return false
	if str(a["spouse"]) != "" or str(b["spouse"]) != "":
		return false
	if _age_of(a) < MARRIAGE_MIN_AGE or _age_of(b) < MARRIAGE_MIN_AGE:
		return false
	if _are_related(a, b):
		return false
	return true


func _are_related(a: Dictionary, b: Dictionary) -> bool:
	var a_id: String = str(a["id"])
	var b_id: String = str(b["id"])
	if str(a["father"]) == b_id or str(a["mother"]) == b_id or str(b["father"]) == a_id or str(b["mother"]) == a_id:
		return true
	if (a["children"] as Array).has(b_id) or (b["children"] as Array).has(a_id):
		return true
	var a_parents := [str(a["father"]), str(a["mother"])]
	var b_parents := [str(b["father"]), str(b["mother"])]
	for parent in a_parents:
		if parent != "" and b_parents.has(parent):
			return true
	return false


# --- Ordres personnages --------------------------------------------------------------------


func _order_learn_skill(order: Dictionary) -> Dictionary:
	var character: String = str(order.get("character", ""))
	var skill: String = str(order.get("skill", ""))
	if not _characters.has(character):
		return {"ok": false, "error": "Personnage inconnu"}
	if not get_learnable(character).has(skill):
		return {"ok": false, "error": "Compétence non disponible pour ce personnage"}
	var node: Dictionary = {}
	for candidate in _skill_tree:
		if str(candidate["id"]) == skill:
			node = candidate
			break
	var c: Dictionary = _characters[character]
	c["skill_points"] = int(c["skill_points"]) - int(node["cost"])
	(c["skills_learned"] as Array).append(skill)
	_events.append({
		"kind": "skill_learned",
		"text_fr": "%s apprend « %s »." % [str(c["name"]), str(node["name"])],
		"character": character,
	})
	return {"ok": true, "error": ""}


func _order_assign_governor(order: Dictionary) -> Dictionary:
	var province_id: String = str(order.get("province", ""))
	var character: String = str(order.get("character", ""))
	if not _characters.has(character):
		return {"ok": false, "error": "Personnage inconnu"}
	if not _provinces.has(province_id):
		return {"ok": false, "error": "Province inconnue"}
	var c: Dictionary = _characters[character]
	var province: Dictionary = _provinces[province_id]
	if str(province["owner"]) != str(c["faction"]):
		return {"ok": false, "error": "Province hors de la faction du personnage"}
	if not bool(c["alive"]) or bool(c["captive"]):
		return {"ok": false, "error": "Personnage indisponible"}
	if _age_of(c) < MAJORITY_AGE:
		return {"ok": false, "error": "Personnage mineur"}
	if str(c["army"]) != "":
		return {"ok": false, "error": "Ce personnage commande déjà une armée"}
	# Un seul gouverneur par province : retire l'ancien.
	for id in _characters:
		if str(_characters[id]["governor_of"]) == province_id:
			_characters[id]["governor_of"] = ""
	c["governor_of"] = province_id
	province["governor"] = character
	_events.append({"kind": "appointment", "text_fr": "%s est nommé(e) gouverneur(e) de %s." % [str(c["name"]), _province_display_name(province_id)], "character": character, "province": province_id})
	return {"ok": true, "error": ""}


func _order_assign_general(order: Dictionary) -> Dictionary:
	var army_id: String = str(order.get("army", ""))
	var character: String = str(order.get("character", ""))
	if not _characters.has(character):
		return {"ok": false, "error": "Personnage inconnu"}
	if not _armies.has(army_id):
		return {"ok": false, "error": "Armée inconnue"}
	var c: Dictionary = _characters[character]
	var army: Dictionary = _armies[army_id]
	if str(army["faction"]) != str(c["faction"]):
		return {"ok": false, "error": "Armée hors de la faction du personnage"}
	if not bool(c["alive"]) or bool(c["captive"]):
		return {"ok": false, "error": "Personnage indisponible"}
	if _age_of(c) < MAJORITY_AGE:
		return {"ok": false, "error": "Personnage mineur"}
	if str(c["governor_of"]) != "":
		return {"ok": false, "error": "Ce personnage gouverne déjà une province"}
	# Un général par armée ; retire ce personnage d'une éventuelle armée précédente.
	for id in _armies:
		if str(_armies[id]["general"]) == character:
			_armies[id]["general"] = ""
			_armies[id]["general_name"] = ""
	army["general"] = character
	army["general_name"] = str(c["name"])
	c["army"] = army_id
	_events.append({"kind": "appointment", "text_fr": "%s prend le commandement d'une armée." % str(c["name"]), "character": character, "army": army_id})
	return {"ok": true, "error": ""}


func _order_propose_marriage(order: Dictionary) -> Dictionary:
	var character: String = str(order.get("character", ""))
	var spouse: String = str(order.get("spouse", ""))
	if not _characters.has(character) or not _characters.has(spouse):
		return {"ok": false, "error": "Personnage inconnu"}
	var a: Dictionary = _characters[character]
	var b: Dictionary = _characters[spouse]
	if not _can_marry(a, b):
		return {"ok": false, "error": "Ce mariage n'est pas possible"}
	a["spouse"] = spouse
	b["spouse"] = character
	a["prestige"] = int(a["prestige"]) + 10
	b["prestige"] = int(b["prestige"]) + 10
	_events.append({
		"kind": "marriage",
		"text_fr": "Mariage de %s et %s." % [str(a["name"]), str(b["name"])],
		"character": character, "spouse": spouse,
	})
	return {"ok": true, "error": ""}


## Réservé au mode headless (tests) : force l'expérience d'un personnage pour valider
## `learn_skill` sans jouer les batailles/tours de gouvernance qui l'accorderaient normalement.
func _order_debug_grant_xp(order: Dictionary) -> Dictionary:
	var character: String = str(order.get("character", ""))
	var amount: int = int(order.get("amount", 100))
	if not _characters.has(character):
		return {"ok": false, "error": "Personnage inconnu"}
	var c: Dictionary = _characters[character]
	c["experience"] = int(c["experience"]) + amount
	c["skill_points"] = int(c["skill_points"]) + amount / XP_PER_SKILL_POINT
	return {"ok": true, "error": ""}


# --- Fin de tour : naissances, morts, succession ------------------------------------------


## Naissances (hiver, couples mariés, femme 16-45 ans, 25 % de probabilité), morts
## (probabilité croissant avec l'âge) et succession simplifiée sur la mort d'un dirigeant.
func _apply_character_events() -> void:
	var is_winter := _turn % 4 == 3
	if is_winter:
		_apply_births()
	_apply_deaths()


func _apply_births() -> void:
	var processed: Array = []
	for id in _characters.keys():
		if processed.has(id):
			continue
		var c: Dictionary = _characters[id]
		var spouse_id: String = str(c["spouse"])
		if spouse_id == "" or not _characters.has(spouse_id) or processed.has(spouse_id):
			continue
		processed.append(id)
		processed.append(spouse_id)
		var mother: Dictionary = c if str(c["sex"]) == "female" else _characters[spouse_id]
		var father: Dictionary = _characters[spouse_id] if str(c["sex"]) == "female" else c
		if not bool(mother["alive"]) or not bool(father["alive"]):
			continue
		var age := _age_of(mother)
		if age < FERTILE_MIN_AGE or age > FERTILE_MAX_AGE:
			continue
		if _rng.randf() >= BIRTH_PROBABILITY:
			continue
		_generate_child(str(father["id"]), str(mother["id"]))


func _generate_child(father_id: String, mother_id: String) -> void:
	var father: Dictionary = _characters[father_id]
	var mother: Dictionary = _characters[mother_id]
	var sex: String = "male" if _rng.randf() < 0.5 else "female"
	var names: Array = MALE_NAMES if sex == "male" else FEMALE_NAMES
	var name: String = names[_rng.randi_range(0, names.size() - 1)]
	var id := "chr_gen_%d" % _next_generated_id
	_next_generated_id += 1
	var trait_id: String = PERSONALITY_TRAITS[_rng.randi_range(0, PERSONALITY_TRAITS.size() - 1)]
	_characters[id] = {
		"id": id, "name": name, "epithet": "", "sex": sex, "house": str(father["house"]),
		"faction": str(father["faction"]), "role": "noble", "birth_year": _current_year(),
		"death_year": -1, "alive": true,
		"experience": 0, "skill_points": 0, "skills_learned": [],
		"traits": [trait_id],
		"spouse": "", "children": [], "father": father_id, "mother": mother_id,
		"location": str(father["location"]), "army": "", "governor_of": "", "captive": false,
		"piety": 50, "prestige": 0, "titles": [],
		"base_skills": {
			"command": _rng.randi_range(0, 3), "governance": _rng.randi_range(0, 3), "court": _rng.randi_range(0, 3),
		},
	}
	(father["children"] as Array).append(id)
	(mother["children"] as Array).append(id)
	_events.append({
		"kind": "birth",
		"text_fr": "Naissance de %s, enfant de %s et %s." % [name, str(father["name"]), str(mother["name"])],
		"character": id, "father": father_id, "mother": mother_id,
	})
	if trait_id != "":
		_events.append({
			"kind": "trait_acquired",
			"text_fr": "%s acquiert le trait « %s »." % [name, str(trait_id).trim_prefix("trait_").capitalize()],
			"character": id,
		})


## Mortalité simplifiée (probabilité croissante avec l'âge) : suffit à garantir des morts sur
## un horizon de plusieurs dizaines de tours pour le smoke test, sans prétendre à une table
## actuarielle réaliste (réservé à `core/`).
func _apply_deaths() -> void:
	for id in _characters.keys():
		var c: Dictionary = _characters[id]
		if not bool(c["alive"]):
			continue
		var age := _age_of(c)
		if age < 20:
			continue
		var chance: float = 0.0015 + maxf(0.0, float(age - 40)) * 0.0006
		if _rng.randf() >= chance:
			continue
		_kill_character(id)


func _kill_character(id: String) -> void:
	var c: Dictionary = _characters[id]
	c["alive"] = false
	c["death_year"] = _current_year()
	if str(c["governor_of"]) != "":
		c["governor_of"] = ""
	if str(c["army"]) != "":
		if _armies.has(str(c["army"])):
			_armies[str(c["army"])]["general"] = ""
			_armies[str(c["army"])]["general_name"] = ""
		c["army"] = ""
	_events.append({"kind": "death", "text_fr": "Mort de %s." % str(c["name"]), "character": id})
	if str(c["role"]) == "ruler":
		_handle_succession(str(c["faction"]), id)


## Succession simplifiée : héritier désigné par `GameDataStore.get_faction().heir` s'il est
## vivant et adulte, sinon le personnage vivant le plus âgé de la faction ; régence si mineur ;
## la loi de succession détaillée (salique, etc.) est traitée par `core/` (§ 2 de la spec).
func _handle_succession(faction_id: String, dead_id: String) -> void:
	var info := _faction_info(faction_id)
	var heir: String = str(info.get("heir", ""))
	var successor := ""
	if heir != "" and _characters.has(heir) and bool(_characters[heir]["alive"]):
		successor = heir
	else:
		var best_age := -1
		for id in _characters:
			var c: Dictionary = _characters[id]
			if str(c["faction"]) == faction_id and bool(c["alive"]):
				var age := _age_of(c)
				if age > best_age:
					best_age = age
					successor = id
	if successor == "":
		_events.append({"kind": "succession", "text_fr": "%s s'éteint sans héritier vivant." % _faction_info(faction_id).get("short_name", faction_id), "faction": faction_id})
		return
	_characters[successor]["role"] = "ruler"
	_events.append({
		"kind": "succession",
		"text_fr": "%s succède à %s sur le trône." % [str(_characters[successor]["name"]), str(_characters[dead_id]["name"])],
		"character": successor, "faction": faction_id,
	})
	if _age_of(_characters[successor]) < MAJORITY_AGE:
		_events.append({
			"kind": "regency",
			"text_fr": "%s est mineur(e) : une régence est instaurée." % str(_characters[successor]["name"]),
			"character": successor, "faction": faction_id,
		})


# --- Difficulté (DF1) ---------------------------------------------------------------------------


## Niveaux lus dans `data/rules/difficulty.json` (sans le résumé chiffré, calculé par le cœur).
func get_difficulty_levels() -> Array:
	var path := _map_paths_data_dir().path_join("rules/difficulty.json")
	if not FileAccess.file_exists(path):
		return []
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		return []
	var default_id := str((parsed as Dictionary).get("default", "normal"))
	var result: Array = []
	for entry in (parsed as Dictionary).get("levels", []):
		var level: Dictionary = entry
		var id := str(level.get("id", ""))
		result.append({
			"id": id,
			"label": str(level.get("label", id)),
			"description": str(level.get("description", "")),
			"effects": PackedStringArray(),
			"summary": "",
			"default": id == default_id,
		})
	return result


func set_difficulty(id: String) -> bool:
	if not id in ["easy", "normal", "hard", "very_hard"] or _turn < 0:
		return false
	if _turn > 0:
		return id == _difficulty
	_difficulty = id
	return true


func get_difficulty() -> String:
	return _difficulty


## B7c (miroir du core `GameData::has_building`) : `required` est bâti, ou l'une de ses améliorations.
func _provides(built: Array, required: String) -> bool:
	for bld_id in built:
		var current := str(bld_id)
		for _step in 16:
			if current == required:
				return true
			current = str(_buildings.get(current, {}).get("upgrades_from", ""))
			if current == "":
				break
	return false
