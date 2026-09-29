extends SceneTree

## Test headless du lot NT3 (missions de campagne) sur la vraie simulation et les vraies données,
## vérifié par l'état des nœuds (pas de capture) :
##  1. `get_missions` / `get_mission_notices` : une mission proposée après la première fin de
##     tour, champs remplis (objectif, progression, échéance, récompense) ;
##  2. l'avis de la mission obtenue part en toast ;
##  3. le panneau d'objectifs montre la section « Missions » avec une ligne par mission.
## Usage : godot --headless --path game --script res://tests/nt3_missions_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("nt3_missions_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("nt3_missions_test: " + message)
	return condition


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_missions"),
			"CampaignSim.get_missions missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	_check((sim.call("get_missions") as Array).is_empty(), "no mission before the first end of turn")

	# 1. Pont.
	sim.call("end_turn")
	var missions: Array = sim.call("get_missions")
	if _check(missions.size() == 1, "one mission offered after the first turn, got %d" % missions.size()):
		var m: Dictionary = missions[0]
		for key in ["title", "objective", "progress", "deadline", "reward", "kind"]:
			_check(str(m.get(key, "")) != "", "mission field %s empty" % key)
		_check(int(m.get("turns_left", 0)) >= 3 and int(m.get("turns_left", 0)) <= 12, "turns_left in 3-12")
	var notices: Array = sim.call("get_mission_notices")
	_check(notices.size() == 1 and str(notices[0].get("kind", "")) == "offered", "an 'offered' notice")

	# 2. Avis.
	map.victory.show_mission_notices()
	var toast_text := str(map.ui.toast.text)
	_check(toast_text.begins_with("Nouvelle mission"), "the notice goes to a toast: %s" % toast_text)

	# 3. Panneau d'objectifs.
	map.victory.open_panel()
	var heading: Label = map.victory.list.find_child("MissionsHeading", true, false) as Label
	_check(heading != null and heading.text == "Missions", "the objectives panel has a « Missions » section")
	if missions.size() == 1:
		var row: Node = map.victory.list.find_child("Mission%d" % int(missions[0]["id"]), true, false)
		if _check(row != null, "one row per mission"):
			var terms: Label = row.find_child("Terms", true, false) as Label
			_check(terms != null and terms.text.contains("Échéance") and terms.text.contains("Récompense"),
				"the row shows deadline and reward")
			var head: Label = row.find_child("Head", true, false) as Label
			_check(head != null and head.text.contains(str(missions[0]["title"])), "the row shows the title")
	map.victory.panel.hide()
	map.queue_free()
	await process_frame
