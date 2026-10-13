extends SceneTree

## PF : planche des champs peints à plusieurs hauteurs de caméra autour d'un point (Paris par
## défaut), pour juger leur lisibilité et leur absence dans les villes.
## Usage : tools/godot_bg.sh --path game --resolution 960x600 --script res://tests/pf_fields_views_shot.gd -- \
##   [--out=<fichier.png>] [--lon=2.35] [--lat=48.85] [--distances=8,16,30,55]
## Écrit hors dépôt (par défaut ~/dev/cent-ans-raw/textures/planches/pf_fields_views.png).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _settle(frames: int) -> Image:
	for _i in frames:
		await process_frame
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()


func _init() -> void:
	var out := CmdArgs.value("--out", OS.get_environment("HOME") + "/dev/cent-ans-raw/textures/planches/pf_fields_views.png")
	var distances: Array[float] = []
	for part in CmdArgs.value("--distances", "8,16,30,55").split(","):
		distances.append(float(part))
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
	RenderingServer.global_shader_parameter_set("campaign_season", Vector4(0.0, 1.0, 0.0, 0.0))
	var map_data: MapData = map.get("map_data")
	var center := FaunaLayer.lonlat_to_px(CmdArgs.number("--lon", 2.35), CmdArgs.number("--lat", 48.85), map_data)
	var shots: Array[Image] = []
	for index in distances.size():
		rig.look_at_point(Vector3(center.x, map_data.surface_world_at(center.x, center.y), center.y), distances[index])
		rig.snap()
		shots.append(await _settle(300 if index == 0 else 120))
		print("pf_fields_views_shot: distance %.1f" % distances[index])
	var w := shots[0].get_width()
	var h := shots[0].get_height()
	var cols := 2
	var rows := int(ceil(shots.size() / float(cols)))
	var board := Image.create(w * cols, h * rows, false, shots[0].get_format())
	for index in shots.size():
		board.blit_rect(shots[index], Rect2i(Vector2i.ZERO, Vector2i(w, h)), Vector2i((index % cols) * w, (index / cols) * h))
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	board.save_png(out)
	print("pf_fields_views_shot: planche -> ", out)
	map.queue_free()
	await process_frame
	quit(0)
