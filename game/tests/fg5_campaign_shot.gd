extends SceneTree

## Lot FG5 : capture d'une armée figurée sur la carte de campagne (figurines de bataille,
## `ArmyFigures`), vue rapprochée, pour vérifier le rendu fin par défaut (non headless).
## Usage : godot --path game --resolution 1600x900 --script res://tests/fg5_campaign_shot.gd --
##   --out=<png> [--distance=<unités>] [--army=<rang>] [--coarse-figures]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	var out := "user://fg5_campaign.png"
	var distance := 30.0
	var army := 0
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--distance="):
			distance = float(arg.trim_prefix("--distance="))
		elif arg.begins_with("--army="):
			army = int(arg.trim_prefix("--army="))
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
	var ids: PackedStringArray = map.player_army_ids()
	if ids.is_empty():
		print("FG5 campaign shot: no army")
		quit(1)
		return
	var id := ids[clampi(army, 0, ids.size() - 1)]
	var world: Vector3 = map.armies.world_position_of(id)
	map.camera_rig.edge_pan_enabled = false
	map.camera_rig.look_at_point(world, distance)
	map.camera_rig.snap()
	for i in 120:
		await process_frame
	var image := root.get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	var err := image.save_png(out)
	print("FG5 campaign shot: army %s at %s, distance %.0f -> %s (%s)" % [id, world, distance, out, error_string(err)])
	quit(0 if err == OK else 1)
