extends SceneTree

## Test headless du lot C5 (interface des colonies) sur la vraie simulation et les vraies
## données `data/` :
##  1. sélection d'une ville française (non-cité) → panneau de colonie ouvert, bons champs ;
##  2. onglet « Colonies » du panneau de province : une ligne par colonie, clic → panneau de
##     colonie ;
##  3. recrutement et construction depuis le panneau, ordres adressés à la colonie (file de
##     recrutement et chantier de la colonie, pas de la cité) ;
##  4. armée sélectionnée : lot M4, la bulle du mouvement libre remplace les anneaux C5 ;
##  5. lot C7d : bouton « Garnison » du bandeau d'ost — une armée du joueur sur une colonie
##     qu'il contrôle laisse une unité en garnison (la garnison grandit, l'armée rétrécit) ;
##  6. clic droit sur une colonie amie atteignable → marche immédiate (M4), l'armée y stationne.
## Usage : godot --headless --path game --script res://tests/c5_settlements_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	pass
	print("c5_settlements_ui_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("c5_settlements_ui_test: " + message)
	return condition


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("settlement_buildable"),
			"CampaignSim.settlement_buildable missing (run core/build.sh)"):
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
	var ctl: Node = map.settlements_ctl
	if not _check(ctl != null and ctl.available(), "SettlementController missing or inactive"):
		map.queue_free()
		return
	var sim: Object = map.sim
	var town := _french_settlement(sim, "town")
	if not _check(town != "", "no French town"):
		map.queue_free()
		return
	var detail: Dictionary = sim.call("settlement_detail", town)
	var province_id := str(detail["province"])
	print("c5: town %s (%s) in %s" % [town, detail["name"], province_id])

	# 1. Sélection sur la carte → panneau de colonie.
	map.settlement_layer.select(town)
	var panel: Control = ctl.panel
	_check(panel.visible, "settlement panel should open on settlement_selected")
	_check(panel.settlement_id == town, "panel shows %s, expected %s" % [panel.settlement_id, town])
	_check(panel.name_label.text == str(detail["name"]), "panel name %s" % panel.name_label.text)
	_check(panel.kind_value.text.begins_with("Ville"), "panel kind %s" % panel.kind_value.text)
	_check(not map.ui.province_panel.visible, "province panel should be hidden")
	_check(panel.actions.visible, "France owns the town: actions should be visible")

	# 2. Onglet « Colonies » du panneau de province.
	map._on_province_selected(map.map_data.index_of_id(province_id))
	var province_panel: Control = map.ui.province_panel
	_check(province_panel.visible and not panel.visible, "province panel should replace the settlement panel")
	var expected: PackedStringArray = sim.call("province_settlements", province_id)
	var rows: Array = province_panel.settlements_list.get_children()
	_check(rows.size() == expected.size(), "Colonies tab: %d rows, expected %d" % [rows.size(), expected.size()])
	var row: Node = province_panel.settlements_list.get_node_or_null(NodePath(town))
	if _check(row is Button, "Colonies tab: no row for %s" % town):
		_check(str(row.text).contains("garnison"), "Colonies tab row should show the garrison")
		row.emit_signal("pressed")
		_check(panel.visible and panel.settlement_id == town and not province_panel.visible,
			"clicking a Colonies row should open the settlement panel")
		_check(map.settlement_layer.selected_id == town, "clicking a Colonies row should select the settlement")

	# 3. Recrutement et construction adressés à la colonie.
	var city := str(sim.call("get_province_state", province_id).get("city", ""))
	var city_queue := (sim.call("settlement_detail", city).get("recruit_queue", PackedStringArray()) as PackedStringArray).size()
	var recruit_button := _first_enabled_button(panel.recruit_list)
	if _check(recruit_button != null, "no recruitable unit in %s" % town):
		recruit_button.emit_signal("pressed")
		var after: Dictionary = sim.call("settlement_detail", town)
		_check((after["recruit_queue"] as PackedStringArray).size() == 1, "recruit order should queue in the town")
		_check((sim.call("settlement_detail", city)["recruit_queue"] as PackedStringArray).size() == city_queue, "the city queue should not change")
		_check(panel.queue_label.text.contains("1 place") or panel.queue_label.text.contains("Recrues attendues : "), "queue label %s" % panel.queue_label.text)
	var build_button := _first_enabled_button(panel.buildable_list)
	if _check(build_button != null, "nothing buildable in %s" % town):
		build_button.emit_signal("pressed")
		var built: Dictionary = sim.call("settlement_detail", town)
		_check(built.has("construction"), "build order should start a construction in the town")
		_check(panel.construction_box.visible, "panel should show the construction")
		_check(_first_enabled_button(panel.buildable_list) == null, "one construction at a time")

	# 4. Armée du joueur : sélection. Lot M4 : la bulle du mouvement libre remplace les
	# anneaux C5 et l'aperçu sur le graphe des colonies.
	var army_id := ""
	for id in map.player_army_ids():
		army_id = id
		break
	if not _check(army_id != "", "France has no army"):
		map.queue_free()
		return
	map.select_army(army_id)
	var movement: Node = map.movement_ctl
	_check(movement != null and movement.active(), "the free movement controller should drive the selected army")
	_check(ctl.markers.marker_count() == 0, "the C5 rings are replaced by the reachable bubble")
	_check(movement.bubble.visible, "the reachable bubble should be shown")
	var here := str(sim.call("get_army", army_id)["settlement"])

	# 5. Lot C7d : bouton « Garnison » du bandeau d'ost, avant tout déplacement (M4 : un ordre
	# de marche est exécuté aussitôt, l'armée quitterait sa colonie).
	var here_detail := sim.call("settlement_detail", here) as Dictionary
	if _check(here != "" and str(here_detail.get("controller", "")) == "fac_france", "%s should be a French settlement" % here):
		var strip: Control = map.ui.army_strip
		_check(strip.visible and strip.can_garrison, "the garrison button should be offered on the army's own settlement")
		_check(str(strip.garrison_disabled_reason) == "", "the garrison button should be enabled: %s" % strip.garrison_disabled_reason)
		var before_army := sim.call("get_army", army_id) as Dictionary
		var before_units: int = (before_army.get("units", []) as Array).size()
		var before_garrison: int = (here_detail.get("garrison", []) as Array).size()
		if _check(before_units > 1, "the test army should have more than one unit to garrison only one"):
			strip.select([0])
			strip._garrison_button.emit_signal("pressed")
			var after_army := sim.call("get_army", army_id) as Dictionary
			var after_detail := sim.call("settlement_detail", here) as Dictionary
			_check((after_detail.get("garrison", []) as Array).size() == before_garrison + 1,
				"garrisoning a regiment should grow %s's garrison" % here)
			_check((after_army.get("units", []) as Array).size() == before_units - 1,
				"the army should shrink by the garrisoned regiment")

	# 6. Clic droit sur une colonie amie atteignable → marche immédiate, l'armée y stationne.
	var reachable: Dictionary = sim.call("get_reachable_settlements", army_id)
	var target := ""
	var best := -1
	for id in reachable:
		var entry: Dictionary = sim.call("settlement_detail", str(id))
		if str(id) != here and str(entry.get("controller", "")) == "fac_france" and int(reachable[id]) > best:
			best = int(reachable[id])
			target = str(id)
	if not _check(target != "", "no reachable French settlement for %s" % army_id):
		map.queue_free()
		return
	# Clic droit à l'écran sur la colonie visée (caméra au palier moyen, icônes visibles).
	var world: Vector3 = map.settlement_layer.world_position_of(target)
	map.camera_rig.look_at_point(world, 300.0)
	map.camera_rig.snap()
	for _i in 4:
		await process_frame
	var screen: Vector2 = map.camera.unproject_position(world + Vector3(0.0, 0.5, 0.0))
	var picked: String = map.settlement_layer.pick_screen(screen)
	var ordered := false
	if picked == target:
		ordered = map.picker.right_click_interceptor.call(screen)
	else:
		print("c5: screen picking gave '%s' instead of %s (headless viewport), ordering directly" % [picked, target])
		ordered = bool(movement.order_move_settlement(army_id, target).get("ok", false))
	_check(ordered, "move order to %s refused" % target)
	var moved: Dictionary = sim.call("get_army", army_id)
	_check(str(moved.get("settlement", "")) == target, "the army should stand in %s, got '%s' at %s" % [target, moved.get("settlement", ""), moved.get("position", "")])
	print("c5: army %s marched from %s to %s (cost %d)" % [army_id, here, target, best])

	map.queue_free()
	await process_frame


## Première colonie du type donné possédée et contrôlée par la France.
func _french_settlement(sim: Object, kind: String) -> String:
	for entry in sim.call("settlements"):
		if str(entry["kind"]) == kind and str(entry["owner"]) == "fac_france" and str(entry["controller"]) == "fac_france":
			return str(entry["id"])
	return ""


func _first_enabled_button(list: Node) -> Button:
	for line in list.get_children():
		for child in line.get_children():
			if child is Button and not (child as Button).disabled:
				return child
	return null
