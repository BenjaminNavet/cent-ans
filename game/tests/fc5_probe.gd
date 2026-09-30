extends SceneTree

## Sonde et capture du lot FC5 (fenêtre réelle) : caméra de campagne posée comme
## `v4_closeup_shot.gd`, puis primitives et appels de dessin de l'image, recensement des arbres
## (`Vegetation.lod_census`), forêt dense et herbe ; capture PNG si `--out=` est donné.
## Usage : godot --path game --resolution 1280x720 --script res://tests/fc5_probe.gd --
##   --at=<x>,<y>,<dist>[,<yaw°>] [--out=<png>] [--no-fc5] [--no-fc2]


func _init() -> void:
	var at := Vector4(2122.07, 3362.48, 25.0, 30.0)
	var out := ""
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
	rig.edge_pan_enabled = false
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
	var primitives := 0.0
	var draws := 0.0
	for i in 10:
		await process_frame
		primitives += Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME)
		draws += Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	print("fc5 probe d=%.0f: primitives %.0f, draw calls %.0f" % [at.z, primitives / 10.0, draws / 10.0])
	print("fc5 probe census %s" % vegetation.lod_census())
	if vegetation.forest_detail != null:
		print("fc5 probe forest detail visible %d" % vegetation.forest_detail.visible_count())
	var clutter: Node = map.find_child("GroundClutter", true, false)
	if clutter != null and clutter.get("stats") != null:
		print("fc5 probe clutter %s" % clutter.get("stats"))
	if out != "":
		await RenderingServer.frame_post_draw
		root.get_viewport().get_texture().get_image().save_png(out)
		print("fc5 probe saved: %s" % out)
	quit(0)
