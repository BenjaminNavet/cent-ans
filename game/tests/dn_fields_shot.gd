extends SceneTree

## DN-CHAMPS : vue des champs en modèles générés et coût (images, appels de dessin, instances).
## Usage : tools/godot_bg.sh --path game --resolution 1280x720 --script res://tests/dn_fields_shot.gd -- \
##   --out=<dossier> --x=2377 --z=3340 --distance=14 [--name=bourgogne]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _measure(frames: int) -> Dictionary:
	var start := Time.get_ticks_usec()
	var draws := 0.0
	for _i in frames:
		await process_frame
		draws += Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	return {"ms": (Time.get_ticks_usec() - start) / 1000.0 / frames, "draws": draws / frames}


func _init() -> void:
	var out := CmdArgs.value("--out", "user://dn_fields")
	var distance := CmdArgs.number("--distance", 14.0)
	var view_name := CmdArgs.value("--name", "champs")
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
	for _i in 5:
		await process_frame
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var map_data: MapData = map.get("map_data")
	var fields: FieldLayer = root.find_child("Fields", true, false)
	DirAccess.make_dir_recursive_absolute(out)
	var x := CmdArgs.number("--x", 2377.0)
	var z := CmdArgs.number("--z", 3340.0)
	rig.look_at_point(Vector3(x, map_data.surface_world_at(x, z), z), distance)
	rig.snap()
	for _i in 400:
		await process_frame
	await RenderingServer.frame_post_draw
	var on: Dictionary = await _measure(120)
	print("dn_fields_shot: fields ON  %.2f ms/frame, %.0f draw calls, %d instances, %d nodes, rebuild max %.1f ms, plan max cell %.1f ms" % [on["ms"], on["draws"], fields.instance_count(), fields.node_count(), fields.stats["build_ms_max"], fields.stats["plan_ms_max"]])
	var path := out.path_join("%s.png" % view_name)
	root.get_texture().get_image().save_png(path)
	fields.enabled = false
	fields.visible = false
	for _i in 10:
		await process_frame
	var off: Dictionary = await _measure(120)
	print("dn_fields_shot: fields OFF %.2f ms/frame, %.0f draw calls" % [off["ms"], off["draws"]])
	map.queue_free()
	await process_frame
	quit(0)
