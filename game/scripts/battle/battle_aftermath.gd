class_name BattleAftermath
extends RefCounted

## Suites d'une bataille du joueur pour l'écran de fin (UB1) : on photographie l'armée, le
## général et les captifs **avant** `CampaignSim.resolve_battle`, puis **après**, et on affiche
## la différence (expérience gagnée, régiments aguerris, captifs faits ou perdus et leurs rançons).
## Aucune règle : tout est calculé par le cœur, ici on ne fait que comparer deux lectures.


## Lecture de l'état utile avant / après la résolution.
static func snapshot(sim: Object, army_id: String, general_id: String) -> Dictionary:
	var snap := {"units": [], "general": {}, "held": {}, "ours": {}}
	if sim == null:
		return snap
	if army_id != "" and sim.has_method("get_army"):
		snap["units"] = (sim.call("get_army", army_id) as Dictionary).get("units", [])
	if general_id != "" and sim.has_method("get_character"):
		snap["general"] = sim.call("get_character", general_id)
	if sim.has_method("get_ransoms"):
		var ransoms: Dictionary = sim.call("get_ransoms")
		for key in ["held", "ours"]:
			var by_id := {}
			for captive in ransoms.get(key, []):
				by_id[str(captive.get("character", ""))] = captive
			snap[key] = by_id
	return snap


## Différence entre deux lectures : `{general_name, general_xp, skill_points, general_fate,
## veterans, captives[{name, faction, ransom}], lost[{name, captor, ransom}], ransom_total}`.
static func diff(before: Dictionary, after: Dictionary) -> Dictionary:
	var result := {"general_name": "", "general_xp": 0, "skill_points": 0, "general_fate": "", "veterans": 0,
		"captives": [], "lost": [], "ransom_total": 0}
	var g0: Dictionary = before.get("general", {})
	var g1: Dictionary = after.get("general", {})
	if not g0.is_empty():
		result["general_name"] = str(g0.get("name", ""))
		if not g1.is_empty():
			result["general_xp"] = maxi(int(g1.get("experience", 0)) - int(g0.get("experience", 0)), 0)
			result["skill_points"] = maxi(int(g1.get("skill_points", 0)) - int(g0.get("skill_points", 0)), 0)
			if not bool(g1.get("alive", true)):
				result["general_fate"] = "tombé au combat"
			elif bool(g1.get("captive", false)) and not bool(g0.get("captive", false)):
				result["general_fate"] = "prisonnier"
	result["veterans"] = veterans(before.get("units", []), after.get("units", []))
	var held_before: Dictionary = before.get("held", {})
	var total := 0
	for id in (after.get("held", {}) as Dictionary):
		if not held_before.has(id):
			var captive: Dictionary = after["held"][id]
			(result["captives"] as Array).append({"name": str(captive.get("name", "")), "faction": str(captive.get("faction", "")), "ransom": int(captive.get("ransom", 0)), "rank": str(captive.get("rank_label", ""))})
			total += int(captive.get("ransom", 0))
	result["ransom_total"] = total
	var ours_before: Dictionary = before.get("ours", {})
	for id in (after.get("ours", {}) as Dictionary):
		if not ours_before.has(id):
			var captive: Dictionary = after["ours"][id]
			(result["lost"] as Array).append({"name": str(captive.get("name", "")), "captor": str(captive.get("captor", "")), "ransom": int(captive.get("ransom", 0))})
	return result


## Régiments dont l'expérience a monté. Les régiments anéantis disparaissent de l'armée : on
## apparie les listes dans l'ordre, par type d'unité, en sautant ceux qui manquent après.
static func veterans(units_before: Array, units_after: Array) -> int:
	var count := 0
	var i := 0
	for unit in units_after:
		while i < units_before.size() and str(units_before[i].get("unit_type", "")) != str(unit.get("unit_type", "")):
			i += 1
		if i >= units_before.size():
			break
		if int(unit.get("experience", 0)) > int(units_before[i].get("experience", 0)):
			count += 1
		i += 1
	return count
