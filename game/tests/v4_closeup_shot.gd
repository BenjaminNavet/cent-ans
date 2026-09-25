extends SceneTree

## Gros plan de contrôle du lot V4 (ponts, gués, arbres ; fenêtre réelle) : la caméra de campagne
## est autorisée à descendre sous sa distance minimale pour regarder un point de près.
## Usage : godot --path game --script res://tests/v4_closeup_shot.gd -- --at=<x>,<y>,<dist>[,<yaw°>] --out=<png>


func _init() -> void:
	var at := Vector4(2122.07, 2082.48, 6.0, 30.0)
	var out := "user://v4_closeup.png"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--at="):
			var parts := arg.substr(5).split(",")
			at = Vector4(float(parts[0]), float(parts[1]), float(parts[2]), float(parts[3]) if parts.size() > 3 else 30.0)
		elif arg.begins_with("--out="):
			out = arg.substr(6)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	rig.min_distance = 2.0
	rig.look_at_point(Vector3(at.x, data.surface_world_at(at.x, at.y), at.y), at.z)
	rig.target_yaw = deg_to_rad(at.w)
	rig.snap()
	var terrain: TerrainBuilder = map.get("terrain")
	var vegetation: Vegetation = map.get_node("Vegetation")
	for i in 40:
		await process_frame
	var guard := 0
	while (vegetation.pending_jobs() > 0 or not terrain.fine_ready()) and guard < 900:
		guard += 1
		await process_frame
	vegetation.flush_ground()
	for i in 30:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	image.save_png(out)
	print("v4 closeup saved: %s" % out)
	quit(0)
