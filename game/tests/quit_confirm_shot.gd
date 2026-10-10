extends TestCase

## Capture de la confirmation « Quitter le jeu » (partie non sauvegardée) depuis la carte.
## Usage : tools/godot_bg.sh --path game --script res://tests/quit_confirm_shot.gd
## Écrit user://quit_confirm_shot.png.


func _init() -> void:
	_run.call_deferred()


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
	for _i in 60:
		await process_frame
	map.flow.call("open_pause")
	var menu: PauseMenu = map.flow.pause_menu
	menu.unsaved_turns = 3
	menu.call("_request_exit", "quit")
	for _i in 60:
		await process_frame
	var panel: Control = menu.get("_confirm_panel")
	print("quit confirm: visible %s alpha %.2f rect %s" % [panel.is_visible_in_tree(), panel.modulate.a, panel.get_global_rect()])
	await RenderingServer.frame_post_draw
	var path := ProjectSettings.globalize_path("user://quit_confirm_shot.png")
	root.get_texture().get_image().save_png(path)
	print("quit confirm shot: ", path)
	finish()
