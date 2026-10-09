extends TestCase

## Régression : carte de campagne → « Menu principal » (FlowController.go_to_main_menu) → menu
## titre, sans plantage. Usage : godot --path game --script res://tests/menu_return_test.gd
## (fenêtré de préférence : le plantage visé touche le rendu), ou --headless.



func _init() -> void:
	await process_frame
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
	LoadingScreen.start(self, "res://scenes/campaign_map.tscn")
	var flow: Node = null
	for i in 3000:
		await process_frame
		if current_scene != null and current_scene.get("flow") != null:
			flow = current_scene.get("flow")
			break
	for i in 120:
		await process_frame
	# Carte « jouée » : zooms successifs sur Paris (villes 1:1, clutter, forêts en construction
	# sur les tâches de fond) et une fin de tour, puis sortie pendant que ces tâches tournent.
	var map: Node = current_scene
	var rig: Node = map.get("camera_rig")
	var map_data: Object = map.get("map_data")
	for distance: float in [1500.0, 491.0, 150.0, 40.0, 15.0]:
		var y: float = map_data.call("surface_world_at", 2213.0, 3204.0)
		rig.call("look_at_point", Vector3(2213.0, y, 3204.0), distance)
		for i in 30:
			await process_frame
	map.call("_on_end_turn")
	for i in 5:
		await process_frame
	print("menu_return_test: map loaded, going to main menu")
	if flow == null:
		push_error("menu_return_test: FlowController not found")
		failures += 1
		finish()
		return
	flow.call("go_to_main_menu")
	var start := Time.get_ticks_msec()
	for i in 300:
		await process_frame
	print("menu_return_test: 300 frames in %d ms" % (Time.get_ticks_msec() - start))
	print("menu_return_test: now on %s" % current_scene.scene_file_path if current_scene != null else "none")
	print("menu_return_test: OK")
	finish()
