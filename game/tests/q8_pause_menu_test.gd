extends SceneTree

## Q8 : le menu pause (Échap sur la carte) doit être visible et utilisable : son panneau est
## rangé dans la zone modale de l'UI ; si la pause le fige, le fondu d'entrée reste à alpha 0
## (écran assombri, aucun bouton). Ouvre le menu, attend, lit l'opacité et le traitement.
## Usage : godot --headless --path game --script res://tests/q8_pause_menu_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	_run.call_deferred()


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("q8_pause_menu_test: " + message)


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	root.size = Vector2i(1280, 720)
	var map: Node = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for _i in 10:
		await process_frame
	for attempt in 2:
		map.flow.call("open_pause")
		for _i in 40:
			await process_frame
		var menu: PauseMenu = map.flow.pause_menu
		_check(menu != null and paused, "attempt %d: pause menu not open" % attempt)
		if menu == null:
			break
		var panel: Control = menu.get("_menu_panel")
		print("q8 pause attempt %d: panel visible %s alpha %.2f can_process %s" % [attempt, panel.is_visible_in_tree(), panel.modulate.a, panel.can_process()])
		_check(panel.is_visible_in_tree() and panel.modulate.a > 0.95, "attempt %d: pause panel invisible (alpha %.2f)" % [attempt, panel.modulate.a])
		_check(panel.can_process(), "attempt %d: pause panel frozen by the pause" % attempt)
		map.flow.call("close_pause")
		for _i in 5:
			await process_frame
	print("q8_pause_menu_test: %s" % ("OK" if _failures == 0 else "%d FAILURES" % _failures))
	quit(1 if _failures > 0 else 0)
