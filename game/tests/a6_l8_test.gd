extends TestCase

## Lot A6-L8 (audit joueur), campagne France, simulation réelle :
##  U19 : un clic sur la ville où stationne l'ost du joueur sélectionne l'armée ; le second clic au
##        même endroit ouvre la ville ; le clic au point écran de `world_position_of` aussi ;
##  U21 : l'anneau de sélection d'une ville est plafonné ;
##  U4  : chaque bouton de la barre du haut porte une infobulle ;
##  U9  : un clic sur l'écu ou le nom de faction ouvre la fiche de faction ;
##  M9  : l'infobulle d'une armée vassale nomme son suzerain.
## Usage : godot --headless --path game --script res://tests/a6_l8_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _frames(count: int) -> void:
	for i in count:
		await process_frame


func _click(control: Control) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = true
	control.gui_input.emit(event)


func _run() -> void:
	root.size = Vector2i(1280, 720)
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
	await _frames(10)
	if not check(map.load_ok and map.sim != null, "campaign map failed to start"):
		return
	map.camera_rig.edge_pan_enabled = false

	# U19
	var army_id := ""
	var settlement_id := ""
	for id in map.player_army_ids():
		var army: Dictionary = map.sim.call("get_army", id)
		if str(army.get("settlement", "")) != "":
			army_id = id
			settlement_id = str(army["settlement"])
			break
	if check(army_id != "", "no garrisoned player army"):
		var city: Vector3 = map.settlement_layer.world_position_of(settlement_id)
		map.camera_rig.look_at_point(city, 60.0)
		map.camera_rig.snap()
		await _frames(6)
		for point in [map.camera.unproject_position(city), map.camera.unproject_position(map.armies.world_position_of(army_id))]:
			map.deselect_army()
			map._last_pick_position = Vector2(-1.0e6, -1.0e6)
			map._last_pick_target = ""
			check(map._try_select_army(point) and map.selected_army == army_id, "clicking at %s should select the army (selected '%s')" % [point, map.selected_army])
		# Second clic au même endroit : la ville.
		var again: Vector2 = map.camera.unproject_position(city)
		map.deselect_army()
		map._last_pick_position = Vector2(-1.0e6, -1.0e6)
		map._last_pick_target = ""
		map._try_select_army(again)
		map._try_select_army(again)
		check(map.selected_army == "" and map.settlement_layer.selected_id == settlement_id, "second click should open the town (army '%s', town '%s')" % [map.selected_army, map.settlement_layer.selected_id])

		# U21
		var radius: float = map.settlement_layer.model_radius_of(settlement_id)
		var ring: MeshInstance3D = map.settlement_layer._selection_ring
		check(ring.scale.x <= map.settlement_layer._ring_max_radius + 1e-4, "selection ring should be capped (%.2f > %.2f, town radius %.2f)" % [ring.scale.x, map.settlement_layer._ring_max_radius, radius])

	# U4
	var bar: Node = map.ui.get_node("TopBar")
	var buttons: Array = bar.find_children("*", "Button", true, false)
	var shown := 0
	for button: Button in buttons:
		if button.is_visible_in_tree():
			shown += 1
			check(button.tooltip_text.strip_edges() != "", "top bar button %s has no tooltip" % button.name)
	check(shown >= 5, "expected the top bar buttons, found %d" % shown)

	# U9
	for control: Control in [map.ui.faction_swatch, map.ui.faction_label]:
		map.ui.hide_faction()
		_click(control)
		await _frames(2)
		check(map.ui.faction_panel_visible(), "clicking %s should open the faction panel" % control.name)

	# M9
	var vassal_text := ""
	for id in map.sim.call("get_army_ids"):
		var faction := str(map.sim.call("get_army", id).get("faction", ""))
		var text: String = map.army_hover_text(id)
		if text.contains("vassal de"):
			vassal_text = text
			print("a6_l8: %s (%s)" % [text, faction])
			break
	check(vassal_text != "", "some army should show a liege in its tooltip")
	map.queue_free()
	await _frames(2)
