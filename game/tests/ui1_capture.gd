extends SceneTree

## Lot UI1 (enluminures) — captures fenêtrées (pas en headless) de l'UI de campagne :
##   godot --path game --script res://tests/ui1_capture.gd -- --out=<dossier>
## Écrit `campagne.png` (HUD, armée sélectionnée) et `province.png` (panneau de province ouvert).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const VIEW := Vector2i(1440, 900)

var _out := ""


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
	_run.call_deferred()
	create_timer(150.0).timeout.connect(func() -> void:
		print("ui1_capture: timeout")
		quit(1))


func _run() -> void:
	await process_frame
	root.size = VIEW
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
	current_scene = map
	for _i in 90:
		await process_frame
	for id in map.player_army_ids():
		map.select_army(id)
		break
	await _wait(2.0)
	_save("campagne.png")
	map.call("_stage_screenshot_province")
	await _wait(1.5)
	_save("province.png")
	quit()


func _wait(seconds: float) -> void:
	var until := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < until:
		await process_frame
	await RenderingServer.frame_post_draw


func _save(name: String) -> void:
	if _out == "":
		return
	DirAccess.make_dir_recursive_absolute(_out)
	var image := root.get_texture().get_image()
	print("ui1_capture: %s (%s)" % [name, error_string(image.save_png(_out.path_join(name)))])
