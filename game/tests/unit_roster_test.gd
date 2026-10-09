extends TestCase

## Test headless de la liste « Mes unités » (touche U) sur la vraie simulation :
##  1. U ouvre la liste ; une ligne par armée du joueur, avec la portée en km ;
##  2. replier la section « Armées » masque ses lignes, la dérouler les rend ;
##  3. clic sur une ligne d'armée : l'armée est sélectionnée et la caméra vise sa position ;
##  4. clic sur une ligne d'agent (s'il y en a) : l'agent est sélectionné, l'armée non ;
##  5. formatage des entrées (épuisée, en mer) sur des dictionnaires synthétiques.
## Usage : godot --headless --path game --script res://tests/unit_roster_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
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
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var roster: Node = map.units_ctl  # non typé : la classe dépend des autoloads
	if not check(roster != null and roster.available(), "roster controller missing"):
		return

	# 1. Ouverture par l'action U.
	var press := InputEventAction.new()
	press.action = "map_toggle_units"
	press.pressed = true
	roster._unhandled_input(press)
	check(roster.is_open(), "U should open the roster")
	var army_ids: PackedStringArray = map.player_army_ids()
	check(not army_ids.is_empty(), "the player should start with armies")
	var agent_count := 0
	for agent in map.sim.call("get_agents"):
		if str(agent.get("faction", "")) == map.player_faction:
			agent_count += 1
	check(roster.row_count() == army_ids.size() + agent_count,
		"one row per army and agent: %d vs %d + %d" % [roster.row_count(), army_ids.size(), agent_count])
	var first: Dictionary = roster.army_entry(army_ids[0], map.sim.call("get_army", army_ids[0]))
	check(str(first["range"]).contains("km"), "army range should be given in km: %s" % first["range"])
	check(float(first["ratio"]) > 0.0, "a fresh army has movement left")

	# 2. Replier / dérouler.
	roster._expanded["armies"] = false
	roster.refresh()
	check(roster.row_count() == agent_count, "collapsed armies section hides its rows")
	roster._expanded["armies"] = true
	roster.refresh()
	check(roster.row_count() == army_ids.size() + agent_count, "expanded again")

	# 3. Clic sur une armée.
	roster.focus_entry("army:" + army_ids[0])
	check(map.selected_army == army_ids[0], "clicking a row selects the army")
	var target: Vector3 = map.camera_rig.target_focus
	var world: Vector3 = map.armies.world_position_of(army_ids[0])
	check(Vector2(target.x, target.z).distance_to(Vector2(world.x, world.z)) < 1.0, "camera should aim at the army")
	await process_frame
	var pressed := 0
	for key in roster._rows:
		if (roster._rows[key] as Button).button_pressed:
			pressed += 1
			check(key == "army:" + army_ids[0], "selected row highlighted")
	check(pressed == 1, "exactly one highlighted row, got %d" % pressed)

	# 4. Clic sur un agent.
	if agent_count > 0:
		var agent_key := ""
		for key in roster._rows:
			if str(key).begins_with("agent:"):
				agent_key = key
				break
		roster.focus_entry(agent_key)
		check(map.agents_ctl.selected_agent == agent_key.trim_prefix("agent:"), "clicking an agent row selects it")
		check(map.selected_army == "", "army deselected when an agent is picked")

	# 5. Formatage.
	var spent: Dictionary = roster.army_entry("x", {"general_name": "Jean", "movement_left": 0, "movement_max": 100, "units": []})
	check(float(spent["ratio"]) == 0.0 and str(spent["range"]).begins_with("Plus de mouvement"), "spent army: %s" % spent)
	var at_sea: Dictionary = roster.army_entry("x", {"movement_left": 50, "movement_max": 100, "embarked": true})
	check(str(at_sea["range"]) == "En mer", "embarked army")
	var agent: Dictionary = roster.agent_entry({"id": "a", "kind": "spy", "name": "Gilles", "kind_name": "Espion", "location": "s0", "movement_points": 20, "max_movement_points": 30}, {"s0": 0, "s1": 10, "s2": 20})
	check(str(agent["range"]).contains("2 colonies"), "agent reach excludes its own place: %s" % agent["range"])
	map.queue_free()
