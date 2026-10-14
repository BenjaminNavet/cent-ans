extends SceneTree
## CV (ADR 0354) : captures de la carte vivante à plusieurs distances. --views=name:lon:lat:dist;... --out=dir
const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
func _init() -> void:
	var out := CmdArgs.value("--out", "user://v")
	var views := CmdArgs.value("--views", "").split(";", false)
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
	var data: MapData = map.get("map_data")
	DirAccess.make_dir_recursive_absolute(out)
	for v in views:
		var p := v.split(":")
		var px := FaunaLayer.lonlat_to_px(float(p[1]), float(p[2]), data)
		rig.look_at_point(Vector3(px.x, data.surface_world_at(px.x, px.y), px.y), float(p[3]))
		rig.snap()
		for _i in 90:
			await process_frame
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png(out.path_join(p[0] + ".png"))
		print("views_shot: ", p[0], " px=", px, " dist=", rig.distance)
	quit(0)
