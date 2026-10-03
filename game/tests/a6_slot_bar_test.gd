extends SceneTree

## Test headless A6-L15 (ADR 0181) : barre des emplacements de colonie. Sélectionne Paris
## (vraie simulation) : la barre existe, une case par emplacement du cœur, elle tient dans
## 1280×720 sans chevaucher la cloche de fin de tour ni la minicarte, et un clic ouvre l'onglet
## des bâtiments du panneau de colonie.
## Usage : godot --headless --path game --script res://tests/a6_slot_bar_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("a6_slot_bar_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("a6_slot_bar_test: " + message)
	return condition


func _wait(frames: int) -> void:
	for i in frames:
		await process_frame


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("settlement_slots"),
			"CampaignSim.settlement_slots missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "interface/ui_size", 1.0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	root.size = Vector2i(1280, 720)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	if not _check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var ctl: Node = map.settlements_ctl
	var bar: Control = ctl.slot_bar  # sans type : ce script se compile avant les autoloads
	if not _check(bar != null, "no slot bar on the settlement controller"):
		map.queue_free()
		return
	_check(not bar.visible, "bar must stay hidden until a settlement is selected")
	var expected: Array = map.sim.call("settlement_slots", "set_paris")
	_check(expected.size() >= 10, "Paris should expose many slots, got %d" % expected.size())
	map.settlement_layer.select("set_paris")
	await _wait(10)
	_check(ctl.panel.visible, "settlement panel should be open")
	if not _check(bar.is_visible_in_tree(), "slot bar should be visible once Paris is selected"):
		map.queue_free()
		return
	_check(bar.call("cell_count") == expected.size(), "bar has %d cells, core has %d slots" % [bar.call("cell_count"), expected.size()])
	var states := {}
	for slot in expected:
		states[str(slot["state"])] = true
	print("a6_slot_bar: %d slots, states %s" % [expected.size(), states.keys()])
	# Placement : dans l'écran, hors cloche de fin de tour, hors minicarte, hors panneau latéral.
	var view := Rect2(Vector2.ZERO, root.get_visible_rect().size)
	var rect := bar.get_global_rect()
	_check(view.encloses(rect), "bar %s leaves the screen %s" % [rect, view])
	var end_turn: Control = map.ui.end_turn_cluster
	# La cloche (bord gauche réel : `fan_left_edge`) ; le reste du cadre du nœud est transparent.
	var bell_left: float = end_turn.get_global_rect().position.x + float(end_turn.call("fan_left_edge"))
	_check(rect.end.x <= bell_left, "bar %s overlaps the end-turn bell (left edge %.1f)" % [rect, bell_left])
	print("a6_slot_bar: view %s bar %s bell_left %.1f" % [view, rect, bell_left])
	if map.ui.minimap != null and map.ui.minimap.is_visible_in_tree():
		_check(not rect.intersects(map.ui.minimap.get_global_rect()), "bar overlaps the minimap")
	if ctl.panel.is_visible_in_tree():
		_check(not rect.intersects(ctl.panel.get_global_rect()), "bar overlaps the settlement panel")
	# Chaque case offre une infobulle ; clic sur une case utile → onglet des bâtiments.
	var clicked := false
	for cell in (bar.get("cells_box") as Node).get_children():
		var button := cell as Button
		_check(button.tooltip_text != "", "cell %s has no tooltip" % button.name)
		if not clicked and not button.disabled:
			ctl.panel.tabs.current_tab = 0
			button.pressed.emit()
			_check(ctl.panel.tabs.current_tab == 1, "click should open the buildings tab")
			clicked = true
	_check(clicked or ctl.panel.is_player_owner == false, "no clickable cell in a French city")
	# Fermeture du panneau : la barre disparaît.
	ctl.panel.hide()
	await _wait(3)
	_check(not bar.visible, "bar should hide with the panel")
	map.queue_free()
	await process_frame
