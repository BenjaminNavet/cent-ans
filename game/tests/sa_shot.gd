extends SceneTree

## Lot SA (ADR 0160) : captures de l'ost du roi à Paris à plusieurs distances caméra (fenêtre
## réelle), pour juger l'échelle, l'écart à la ville et la surbrillance de survol.
## Usage : godot --path game --resolution 1280x720 --script res://tests/sa_shot.gd --
##   --out=<préfixe> [--distances=24,60,150,400] [--hover=army|town|none] [--select]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	var out := "user://sa"
	var distances := PackedFloat32Array([24.0, 60.0, 150.0, 400.0])
	var hover := "none"
	var select := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--distances="):
			distances = PackedFloat32Array()
			for part in arg.trim_prefix("--distances=").split(","):
				distances.append(float(part))
		elif arg.begins_with("--hover="):
			hover = arg.trim_prefix("--hover=")
		elif arg == "--select":
			select = true
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
		print("SA shot: no army")
		quit(1)
		return
	var id := ids[0]
	var army: Dictionary = map.sim.call("get_army", id)
	var settlement := str(army.get("settlement", ""))
	map.camera_rig.edge_pan_enabled = false
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	for distance in distances:
		var world: Vector3 = map.armies.world_position_of(id)
		map.camera_rig.look_at_point(world, distance)
		map.camera_rig.snap()
		for i in 90:
			await process_frame
		if select:
			map.select_army(id)
		for i in 5:
			var marker: ArmyMarker = map.armies._markers[id]
			if hover == "army":
				map.hover_at(marker.screen_rect(map.camera).get_center())
			elif hover == "town" and settlement != "":
				map.hover_at(map.camera.unproject_position(map.settlement_layer.world_position_of(settlement)))
			await process_frame
		var path := "%s_%d.png" % [out, int(distance)]
		var err := root.get_viewport().get_texture().get_image().save_png(path)
		print("SA shot: army %s, distance %.0f, scale %.2f -> %s (%s)" % [id, distance, map.armies._current_scale, path, error_string(err)])
	quit(0)
