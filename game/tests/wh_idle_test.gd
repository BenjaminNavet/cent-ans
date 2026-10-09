extends TestCase

## Test headless du lot WH `idle` (ne jamais oublier une armée) :
##  1. fonctions pures : prédicat d'armée inactive, cycle, texte de confirmation, migration du
##     réglage booléen, touches listées dans la fiche et réaffectables ;
##  2. vraie simulation : alertes `idle_army` (une par armée, puis moins après un ordre de marche),
##     cycle Tab / Maj+Tab, marque « inactive » de la plaque, confirmation `off|warnings|always` ;
##  3. fuite du brouillard : une armée ennemie hors de vue ne produit aucune alerte.
## Usage : godot --headless --path game --script res://tests/wh_idle_test.gd


func _init() -> void:
	await process_frame
	_test_pure()
	await _test_real_map()
	finish()


func _flow_script() -> GDScript:
	return load("res://scripts/map/flow_controller.gd")


func _test_pure() -> void:
	check(CampaignAlerts.army_is_idle({"path": [], "movement_points": 5}), "no path + movement left = idle")
	check(not CampaignAlerts.army_is_idle({"path": [], "movement_points": 0}), "no movement left")
	check(not CampaignAlerts.army_is_idle({"path": ["p"], "movement_points": 5}), "a march order")
	check(not CampaignAlerts.army_is_idle({"path": [], "movement_points": 5, "stance": "siege"}), "besieging army is busy")
	check(not CampaignAlerts.army_is_idle({"path": [], "movement_points": 5, "embarked": true}), "embarked army is busy")
	var ids := ["a", "b", "c"]
	check(CampaignHotkeys.next_in_cycle(ids, "a", 1) == "b" and CampaignHotkeys.next_in_cycle(ids, "c", 1) == "a", "forward cycle wraps")
	check(CampaignHotkeys.next_in_cycle(ids, "a", -1) == "c", "backward cycle wraps")
	check(CampaignHotkeys.next_in_cycle(ids, "zz", 1) == "a" and CampaignHotkeys.next_in_cycle(ids, "zz", -1) == "c", "unknown current")
	check(CampaignHotkeys.next_in_cycle([], "a", 1) == "", "empty cycle")
	check(CampaignHotkeys.stance_of("army_stance_raid") == "raid" and CampaignHotkeys.stance_of("army_split") == "", "stance_of")
	var text: String = _flow_script().confirm_text("Printemps 1337", [
		{"kind": "idle_army"}, {"kind": "idle_army"}, {"kind": "research_idle", "text": "Aucune recherche en cours"}])
	check(text.contains("2 armées sans ordre") and text.contains("Aucune recherche"), "confirm text lists the oversights: %s" % text)
	# Migration du réglage booléen.
	var settings: Node = root.get_node_or_null("/root/Settings")
	if check(settings != null, "Settings autoload"):
		check(settings.call("_coerce", "interface/confirm_end_turn", true) == "always", "true -> always")
		check(settings.call("_coerce", "interface/confirm_end_turn", false) == "warnings", "false -> warnings")
		check(settings.call("_coerce", "interface/confirm_end_turn", "off") == "off", "off kept")
		check(settings.call("_coerce", "interface/confirm_end_turn", "bogus") == "warnings", "unknown -> default")
		check(settings.DEFAULTS["interface/confirm_end_turn"] == "warnings", "default is warnings")
	# Fiche des raccourcis et réaffectation.
	var rebindable := KeyBindings.all_actions()
	var sheet := ShortcutSheet.bbcode()
	for action in CampaignHotkeys.ACTIONS:
		check(InputMap.has_action(action), "InputMap has %s" % action)
		check(rebindable.has(action), "%s is rebindable" % action)
		check(ShortcutSheet.action_keys(action) != "", "%s has a key label" % action)
	check(sheet.contains("Armée inactive suivante"), "sheet lists the idle cycle")
	check(ShortcutSheet.action_keys("campaign_prev_idle").contains("Maj"), "Shift+Tab label: %s" % ShortcutSheet.action_keys("campaign_prev_idle"))
	# Tab et Maj+Tab sont deux actions distinctes (correspondance exacte).
	var tab := InputEventKey.new()
	tab.physical_keycode = KEY_TAB
	tab.pressed = true
	check(tab.is_action("campaign_next_idle", true) and not tab.is_action("campaign_prev_idle", true), "Tab = next only")
	tab.shift_pressed = true
	check(tab.is_action("campaign_prev_idle", true) and not tab.is_action("campaign_next_idle", true), "Shift+Tab = previous only")


func _test_real_map() -> void:
	if not check(ClassDB.class_exists("CampaignSim"), "CampaignSim missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
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
	var sim: Object = map.sim
	var flow = map.flow
	var own: PackedStringArray = map.player_army_ids()
	check(own.size() >= 1, "French player has armies")

	# Alertes : une `idle_army` par armée inactive, même liste que le prédicat partagé.
	var idle: Array = CampaignAlerts.idle_armies(map)
	var alerts: Array = CampaignAlerts.collect(map, [])
	var idle_alerts := alerts.filter(func(a: Dictionary) -> bool: return a["kind"] == "idle_army")
	check(idle_alerts.size() == idle.size() and not idle.is_empty(), "one idle_army alert per idle army: %d / %d" % [idle_alerts.size(), idle.size()])
	var first_alert: Dictionary = idle_alerts[0]
	check(str(first_alert["id"]).begins_with("idle:") and str(first_alert["army_id"]) in idle, "alert id / army_id")
	var research_idle := (sim.call("get_research", "fac_france") as Dictionary).is_empty() and int(sim.call("get_research_points", "fac_france")) > 0
	check(alerts.any(func(a: Dictionary) -> bool: return a["kind"] == "research_idle") == research_idle, "research_idle alert follows the research state")
	print("wh_idle: %d idle armies, free_slot=%s research_idle=%s" % [idle.size(),
		alerts.any(func(a: Dictionary) -> bool: return a["kind"] == "free_slot"), research_idle])

	# Plaque : « inactive » pour une armée du joueur sans ordre.
	var marker = map.armies.marker_of(idle[0])
	if check(marker != null, "marker of the idle army"):
		check(marker.status == "idle", "marker status idle, got '%s'" % marker.status)
		var plate = load("res://scripts/map/army_plate.gd").build(marker)
		var has_text := false
		for label in plate.find_children("*", "Label", true, false):
			has_text = has_text or (label as Label).text == "inactive"
		check(has_text, "the plate says 'inactive'")
		plate.free()

	# Cycle Tab / Maj+Tab : parcourt toutes les armées inactives puis revient au début.
	map.deselect_army()
	var visited: Array = []
	for i in idle.size():
		visited.append(flow.focus_next_idle(1))
	check(visited == idle, "Tab visits the idle armies in order: %s vs %s" % [visited, idle])
	check(flow.focus_next_idle(1) == idle[0], "Tab wraps to the first")
	check(str(map.selected_army) == idle[0], "the army is selected")
	check(flow.focus_next_idle(-1) == idle[idle.size() - 1], "Shift+Tab goes back (wraps)")

	# Un ordre de marche retire l'armée des inactives.
	var army_id := str(idle[0])
	var army: Dictionary = sim.call("get_army", army_id)
	var destination := ""
	for neighbor in map.map_data.get_province(map.map_data.index_of_id(str(army.get("location_province", army.get("location", ""))))).get("neighbors", []):
		var path: PackedStringArray = sim.call("find_path", army_id, str(neighbor))
		if not path.is_empty():
			destination = str(neighbor)
			break
	if check(destination != "", "a reachable neighbour province"):
		var result: Dictionary = sim.call("submit_order", {"type": "move_army", "army": army_id, "path": Array(sim.call("find_path", army_id, destination))})
		check(result.get("ok", false), "move order accepted: %s" % result)
		map.refresh_all()
		check(not (army_id in CampaignAlerts.idle_armies(map)), "an army with orders is no longer idle")
		check(CampaignAlerts.idle_armies(map).size() == idle.size() - 1, "one fewer idle army")

	# Confirmation de fin de tour.
	settings.call("set_value", "interface/confirm_end_turn", "off", false)
	check(flow.end_turn_would_proceed(), "off: the turn ends without question")
	settings.call("set_value", "interface/confirm_end_turn", "always", false)
	check(not flow.end_turn_would_proceed(), "always: confirmation required")
	settings.call("set_value", "interface/confirm_end_turn", "warnings", false)
	var warnings: Array = flow.end_turn_warnings()
	check(flow.end_turn_would_proceed() == warnings.is_empty(), "warnings: confirmation iff oversights (%d)" % warnings.size())
	check(not warnings.is_empty() and warnings.all(func(a: Dictionary) -> bool: return a["kind"] in _flow_script().FORGOTTEN_KINDS), "only oversights are listed")

	# Fuite du brouillard : l'ennemi hors de vue ne produit aucune alerte, en vue si.
	var enemies: PackedStringArray = sim.call("get_faction_summary", "fac_france").get("at_war_with", PackedStringArray())
	var enemy_army := ""
	for id in sim.call("get_army_ids"):
		if str(sim.call("get_army", id).get("faction", "")) in enemies:
			enemy_army = str(id)
			break
	if check(enemy_army != "", "an enemy army exists"):
		var at: Vector2 = sim.call("get_army", army_id).get("position", Vector2.ZERO)
		sim.call("debug_place_army", enemy_army, at.x, at.y)
		var fog: Object = map.minimap_ctl
		fog.set("fog_active", true)
		fog.set("fog_by_cell", true)
		fog.set("visible_armies", {})
		var hidden := CampaignAlerts.collect(map, []).filter(func(a: Dictionary) -> bool: return a["kind"] == "enemy_army" and a.get("army_id", "") == enemy_army)
		check(hidden.is_empty(), "no alert for an enemy army hidden by the fog")
		fog.set("visible_armies", {enemy_army: true})
		var seen := CampaignAlerts.collect(map, []).filter(func(a: Dictionary) -> bool: return a["kind"] == "enemy_army" and a.get("army_id", "") == enemy_army)
		check(seen.size() == 1, "the alert returns once the army is in view (%d)" % seen.size())
	map.queue_free()
