extends TestCase

## Test headless du lot WH hover (ADR 0271) sur la vraie simulation : numéros de tour du chemin,
## ZOC de toutes les armées ennemies en vue, bulles d'armée et de colonie, fantôme d'une armée
## perdue de vue. Aucune capture.
## Usage : godot --headless --path game --script res://tests/wh_hover_map_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim"), "CampaignSim missing (run core/build.sh)"):
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
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var own_id := ""
	for id in map.player_army_ids():
		own_id = id
		break
	if not check(own_id != "", "France has no army"):
		map.queue_free()
		return
	_turn_numbers(map)
	_zoc(map, own_id, settings)
	_bubbles(map, own_id)
	_ghost(map)
	map.queue_free()


func _turn_numbers(map: Node3D) -> void:
	var path: ArmyMovementPath = map.movement_ctl.path_line
	var points := PackedVector2Array([Vector2(100, 100), Vector2(150, 100), Vector2(200, 100), Vector2(250, 100), Vector2(300, 100)])
	path.show_plan(points, 1, PackedInt32Array([1, 2, 4]), 100.0)
	check(path.turn_label_count() == 3, "three turn ends give three numbers (%d)" % path.turn_label_count())
	check(path.turn_label_texts() == PackedStringArray(["1", "2", "T3"]), "labels read 1, 2, T3: %s" % [path.turn_label_texts()])
	path.hide_path()
	check(path.turn_label_count() == 0, "no number after hide_path")


func _zoc(map: Node3D, own_id: String, settings: Node) -> void:
	var ctl: ArmyMovementController = map.movement_ctl
	var others: Array[String] = []
	for id in map.sim.call("get_army_ids"):
		if str(map.sim.call("get_army", id).get("faction", "")) != map.player_faction and map.armies.marker_of(str(id)) != null:
			others.append(str(id))
	if not check(others.size() >= 2, "need two foreign armies on the map (%d)" % others.size()):
		return
	for id in others:
		map.armies.marker_of(id).set_cue(StanceCues.ENEMY)
	var expected := mini(others.size(), StanceCues.zoc_max_rings())
	# Une armée ennemie hors de vue n'a pas de marqueur : retirée du compte.
	map.select_army(own_id)
	check(ctl.active(), "ordering is active for the player army")
	check(ctl.enemy_zoc_count() == expected, "one ring per visible enemy army (%d vs %d)" % [ctl.enemy_zoc_count(), expected])
	var hidden_id := others[0]
	var marker: ArmyMarker = map.armies._markers[hidden_id]
	map.armies._markers.erase(hidden_id)
	ctl.refresh_enemy_zoc()
	check(ctl.enemy_zoc_count() == expected - 1, "an army out of sight gets no ring (%d)" % ctl.enemy_zoc_count())
	map.armies._markers[hidden_id] = marker
	if settings != null:
		settings.call("set_value", "map/show_zoc", false, false)
		ctl.refresh_enemy_zoc()
		check(ctl.enemy_zoc_count() == 0, "map/show_zoc off hides every ring")
		settings.call("set_value", "map/show_zoc", true, false)
	ctl.refresh_enemy_zoc()
	check(ctl.enemy_zoc_count() == expected, "rings back with the setting on")
	map.deselect_army()
	check(ctl.enemy_zoc_count() == 0, "rings cleared on deselect")


func _bubbles(map: Node3D, own_id: String) -> void:
	var army: Dictionary = map.sim.call("get_army", own_id)
	var men := 0
	for unit in army.get("units", []):
		men += int(unit.get("strength", 0))
	var text: String = map.army_bubble_text(own_id)
	check(text.contains(ArmyPlate.format_men(men)), "own army bubble lists its men (%d): %s" % [men, text])
	check(text.contains("Vivres"), "own army bubble shows supplies")
	for id in map.sim.call("get_army_ids"):
		var foreign: Dictionary = map.sim.call("get_army", id)
		if str(foreign.get("faction", "")) != map.player_faction:
			var foreign_text: String = map.army_bubble_text(str(id))
			check(not foreign_text.contains("Vivres") and not foreign_text.contains("Embuscade"), "foreign bubble hides supplies and ambush: %s" % foreign_text)
			break
	var settlement_id := ""
	for entry in map.settlement_data.settlements:
		if str(entry.get("owner", "")) == map.player_faction and str(entry.get("kind", "")) == "city":
			settlement_id = str(entry["id"])
			break
	if check(settlement_id != "", "France owns a city"):
		var detail: Dictionary = map.sim.call("settlement_detail", settlement_id)
		var settlement_text: String = map.settlement_bubble_text(settlement_id)
		check(settlement_text.contains(str(detail.get("name", "?"))) and settlement_text.contains("Garnison"), "own settlement bubble: %s" % settlement_text)


func _ghost(map: Node3D) -> void:
	var armies: ArmyMarkers = map.armies
	armies.ghosts = [{"id": "ghost_x", "faction": "fac_england", "general_name": "Fantôme", "pos": Vector2(500, 500), "men": 800, "turn": 1, "ago": 2}]
	armies._sync_ghost_labels()
	check(armies.ghost_label_count() == 1, "a ghost label is shown")
	var text: String = map.ghost_bubble_text(armies.ghosts[0])
	check(text.contains("Vue il y a 2 saisons") and text.contains("800"), "ghost bubble: %s" % text)
	armies.ghosts = []
	armies._sync_ghost_labels()
	check(armies.ghost_label_count() == 0, "ghost label removed when the army reappears")
