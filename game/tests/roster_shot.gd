extends SceneTree

## Capture de la liste « Mes unités » (non headless). Usage : godot --path game --script res://tests/roster_shot.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 30:
		await process_frame
	map.units_ctl.toggle()
	var ids: PackedStringArray = map.player_army_ids()
	if ids.size() > 1:
		map.units_ctl.focus_entry("army:" + ids[1])
	for i in 90:
		await process_frame
	var path := OS.get_environment("ROSTER_SHOT")
	root.get_viewport().get_texture().get_image().save_png(path if path != "" else "user://roster.png")
	for key in map.units_ctl._rows:
		print("row ", key)
	quit(0)
