extends SceneTree

## Vérification headless de CampaignSim (GDExtension `core/crates/godot-bridge`, spec M2 § 2).
## Usage : godot --headless --path game --script <chemin absolu>/core/checks/campaign_sim_check.gd
## Le dossier `data/` est résolu relativement au projet Godot (`res://../data`).
## Scénario : nouvelle campagne France, armées, provinces atteignables de la première
## armée, ordre move_army, 4 fins de tour, sauvegarde/rechargement, égalité des dates.
## Code de sortie 0 si tout passe, 1 sinon.

const PLAYER := "fac_france"
const TURNS := 4
const FACTION_SUMMARY_KEYS := ["treasury", "income", "at_war_with", "allies", "provinces_count", "armies_count", "alive", "projected_income", "army_upkeep", "building_upkeep", "tax_rate"]
const PROVINCE_STATE_KEYS := ["owner", "controller", "garrison", "unrest", "devastation", "population_total"]
const ARMY_KEYS := ["faction", "general", "general_name", "location", "units", "movement_points", "supply", "stance", "path"]
const UNIT_KEYS := ["unit_type", "name", "strength", "max_strength", "morale"]
const RECRUIT_KEYS := ["unit_type", "name", "cost", "upkeep", "available", "reason"]
const EVENT_KEYS := ["kind", "text_fr", "province", "army", "faction"]
const PROVINCE_CITY_KEYS := ["classes", "buildings", "fortification_level", "capacity", "buildable", "resources", "effects"]
const CLASS_KEYS := ["count", "unrest", "health", "wealth", "goods_satisfaction"]
const BUILDING_SUMMARY_KEYS := ["id", "name", "category", "upkeep"]
const BUILDABLE_KEYS := ["building", "name", "category", "cost", "turns", "available", "reason"]
const EFFECT_VALUE_KEYS := ["flat", "percent"]
const EFFECTS_KEYS := ["tax_income", "trade_income", "health", "unrest", "wealth", "goods_satisfaction", "growth", "garrison", "fortification_level", "recruit_cost", "supply"]
const FACTION_ECONOMY_KEYS := ["treasury", "income", "projected_income", "army_upkeep", "building_upkeep", "tax_rate", "goods", "goods_categories"]

var _failures: int = 0


func _init() -> void:
	_run()
	quit(1 if _failures > 0 else 0)


func _run() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		_fail("CampaignSim class not registered: GDExtension not loaded (run core/build.sh)")
		return

	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: CampaignSim = CampaignSim.new()
	_check(sim.get_turn() == -1, "turn before new_campaign should be -1, got %d" % sim.get_turn())
	_check(sim.get_date_label() == "", "date label before new_campaign should be empty")
	_check(sim.get_army_ids().is_empty(), "no armies before new_campaign")
	_check(not sim.new_campaign(data_dir, "fac_atlantis", 1), "unknown player faction should be refused")
	_check(sim.new_campaign(data_dir, PLAYER, 1337), "new_campaign(%s, %s) should return true" % [data_dir, PLAYER])
	if _failures > 0:
		return

	_check(sim.get_turn() == 0, "initial turn should be 0, got %d" % sim.get_turn())
	_check(sim.get_date_label() == "Printemps 1337", "initial date should be 'Printemps 1337', got '%s'" % sim.get_date_label())
	_check(sim.get_player_faction() == PLAYER, "player faction should be %s" % PLAYER)

	# Faction et province.
	var summary := sim.get_faction_summary(PLAYER)
	_check_keys(summary, FACTION_SUMMARY_KEYS, "faction summary")
	_check(summary.get("treasury", 0) > 0, "France treasury should be positive")
	_check(summary.get("provinces_count", 0) > 0, "France should own provinces")
	_check(summary.get("alive", false) == true, "France should be alive")
	_check(summary.get("at_war_with", []).has("fac_england"), "France should be at war with England")
	_check(sim.get_faction_summary("fac_atlantis").is_empty(), "unknown faction should give an empty dictionary")

	var paris := sim.get_province_state("prov_ile_de_france")
	_check_keys(paris, PROVINCE_STATE_KEYS, "province state")
	_check(paris.get("owner", "") == PLAYER, "prov_ile_de_france owner should be France")
	_check(paris.get("controller", "") == PLAYER, "prov_ile_de_france controller should be France")
	_check(not paris.has("siege"), "prov_ile_de_france should not be under siege")
	for unit in paris.get("garrison", []):
		_check_keys(unit, UNIT_KEYS, "garrison unit")
	_check(sim.get_province_state("prov_atlantis").is_empty(), "unknown province should give an empty dictionary")

	# Recrutement.
	var recruitable := sim.get_recruitable("prov_ile_de_france")
	_check(recruitable.size() > 0, "France should have recruitable unit types in its capital")
	for option in recruitable:
		_check_keys(option, RECRUIT_KEYS, "recruit option")

	# Armées.
	var army_ids := sim.get_army_ids()
	_check(army_ids.size() > 0, "there should be armies at the start")
	var player_army_id := ""
	for army_id in army_ids:
		var army := sim.get_army(army_id)
		_check_keys(army, ARMY_KEYS, "army %s" % army_id)
		for unit in army.get("units", []):
			_check_keys(unit, UNIT_KEYS, "army unit")
		if army.get("faction", "") == PLAYER and player_army_id == "":
			player_army_id = army_id
	_check(player_army_id != "", "France should have at least one army")
	_check(sim.get_army("army_atlantis").is_empty(), "unknown army should give an empty dictionary")
	if player_army_id == "":
		return
	var player_army := sim.get_army(player_army_id)
	print("campaign sim: %d armies, player army %s at %s led by %s (%d units, %d MP)" % [
		army_ids.size(), player_army_id, player_army["location"], player_army["general_name"],
		player_army["units"].size(), player_army["movement_points"]])

	# Déplacement.
	var reachable := sim.get_reachable(player_army_id)
	_check(reachable.size() > 0, "player army should reach at least one province")
	_check(sim.get_reachable("army_atlantis").is_empty(), "unknown army should reach nothing")
	var target := ""
	var best_cost := INF
	for province_id in reachable:
		_check(reachable[province_id] > 0, "reachable cost of %s should be positive" % province_id)
		if reachable[province_id] < best_cost:
			best_cost = reachable[province_id]
			target = province_id
	var path := sim.find_path(player_army_id, target)
	_check(path.size() > 0 and path[path.size() - 1] == target, "find_path should end at %s, got %s" % [target, path])
	_check(sim.find_path(player_army_id, player_army["location"]).is_empty(), "path to current location should be empty")

	var bad := sim.submit_order({"type": "move_army", "army": "army_atlantis", "path": path})
	_check_keys(bad, ["ok", "error"], "order result")
	_check(bad.get("ok", true) == false and bad.get("error", "") != "", "moving an unknown army should be refused with an error")
	var garbage := sim.submit_order({"type": "teleport"})
	_check(garbage.get("ok", true) == false, "unknown order type should be refused")
	var result := sim.submit_order({"type": "move_army", "army": player_army_id, "path": path})
	_check(result.get("ok", false) == true, "move_army should be accepted, got error '%s'" % result.get("error", ""))
	var ordered := sim.get_army(player_army_id)
	_check(Array(ordered.get("path", [])).size() == path.size(), "army path should be recorded")

	# Fins de tour.
	for i in range(TURNS):
		var events := sim.end_turn()
		for event in events:
			_check_keys(event, EVENT_KEYS, "event")
		_check(sim.get_events().size() == events.size(), "get_events should return the last turn's events")
		_check(sim.get_turn() == i + 1, "turn should be %d after end_turn" % (i + 1))
	_check(sim.get_date_label() == "Printemps 1338", "date after %d turns should be 'Printemps 1338', got '%s'" % [TURNS, sim.get_date_label()])
	var moved := sim.get_army(player_army_id)
	_check(moved.is_empty() or moved.get("location", "") == target, "player army should have arrived at %s (or been destroyed), got %s" % [target, moved.get("location", "")])

	# Ville de province (spec M3 § 2).
	var city := sim.get_province_city("prov_ile_de_france")
	_check_keys(city, PROVINCE_CITY_KEYS, "province city")
	var classes: Dictionary = city.get("classes", {})
	_check_keys(classes, ["peasants", "burghers", "clergy", "nobility"], "province city classes")
	for class_name in classes:
		_check_keys(classes[class_name], CLASS_KEYS, "population class %s" % class_name)
	for building in city.get("buildings", []):
		_check_keys(building, BUILDING_SUMMARY_KEYS, "city building")
	var buildable: Array = city.get("buildable", [])
	_check(buildable.size() > 0, "prov_ile_de_france should have buildable options")
	var buildable_id := ""
	for option in buildable:
		_check_keys(option, BUILDABLE_KEYS, "buildable option")
		if bool(option.get("available", false)) and buildable_id == "":
			buildable_id = str(option.get("building", ""))
	_check(buildable_id != "", "prov_ile_de_france should have at least one available buildable option")
	_check_keys(city.get("effects", {}), EFFECTS_KEYS, "province city effects")
	for effect_name in city.get("effects", {}):
		_check_keys(city["effects"][effect_name], EFFECT_VALUE_KEYS, "effect %s" % effect_name)
	_check(sim.get_province_city("prov_atlantis").is_empty(), "unknown province should give an empty dictionary for get_province_city")

	if buildable_id != "":
		var build_result := sim.submit_order({"type": "build", "province": "prov_ile_de_france", "building": buildable_id})
		_check(build_result.get("ok", false) == true, "build order should be accepted, got error '%s'" % build_result.get("error", ""))
		var built := false
		for i in range(6):
			sim.end_turn()
			var after_build := sim.get_province_city("prov_ile_de_france")
			var building_ids: Array = []
			for b in after_build.get("buildings", []):
				building_ids.append(b.get("id", ""))
			if building_ids.has(buildable_id):
				built = true
				break
		_check(built, "%s should appear in buildings within 6 turns" % buildable_id)

	# Économie de faction (spec M3 § 2).
	var economy := sim.get_faction_economy(PLAYER)
	_check_keys(economy, FACTION_ECONOMY_KEYS, "faction economy")
	_check(economy.get("projected_income", 0) > 0, "France projected_income should be positive")
	_check(sim.get_faction_economy("fac_atlantis").is_empty(), "unknown faction should give an empty dictionary for get_faction_economy")

	var tax_high := sim.submit_order({"type": "set_tax_rate", "rate": "high"})
	_check(tax_high.get("ok", false) == true, "set_tax_rate high should be accepted, got error '%s'" % tax_high.get("error", ""))
	var economy_after := sim.get_faction_economy(PLAYER)
	_check(economy_after.get("tax_rate", "") == "high", "tax_rate should be 'high' after set_tax_rate high")

	var tax_bad := sim.submit_order({"type": "set_tax_rate", "rate": "absurd"})
	_check(tax_bad.get("ok", true) == false, "set_tax_rate 'absurd' should be refused")

	# Sauvegarde / rechargement.
	var saved := sim.save_to_string()
	_check(saved.length() > 0, "save_to_string should not be empty")
	var reloaded: CampaignSim = CampaignSim.new()
	_check(not reloaded.load_from_string("{not json"), "load_from_string should refuse garbage")
	_check(reloaded.load_from_string(saved), "load_from_string should accept a save")
	_check(reloaded.get_turn() == sim.get_turn(), "reloaded turn should match")
	_check(reloaded.get_date_label() == sim.get_date_label(), "reloaded date should match: '%s' vs '%s'" % [reloaded.get_date_label(), sim.get_date_label()])
	_check(reloaded.get_player_faction() == PLAYER, "reloaded player faction should match")
	_check(reloaded.get_army_ids() == sim.get_army_ids(), "reloaded armies should match")
	_check(reloaded.save_to_string() == saved, "save should round-trip byte for byte")
	_check(reloaded.end_turn().size() == sim.end_turn().size(), "reloaded campaign should replay identically")
	_check(reloaded.get_date_label() == sim.get_date_label(), "dates should still match after one more turn")

	if _failures == 0:
		print("campaign sim OK: %s after %d turns, %d events last turn" % [sim.get_date_label(), sim.get_turn(), sim.get_events().size()])


func _check_keys(dict: Dictionary, expected: Array, label: String) -> void:
	var keys := dict.keys()
	for key in expected:
		_check(dict.has(key), "%s should have key '%s' (got %s)" % [label, key, keys])
	for key in keys:
		_check(expected.has(key) or (key == "siege" and label == "province state"), "%s has unexpected key '%s'" % [label, key])


func _check(condition: bool, message: String) -> void:
	if not condition:
		_fail(message)


func _fail(message: String) -> void:
	_failures += 1
	push_error("campaign sim FAIL: " + message)
	printerr("campaign sim FAIL: " + message)
