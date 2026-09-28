extends SceneTree

## Banc d'essai du lot V4 (fleuves, ponts, forêts ; fenêtre réelle, pas headless) : charge la carte
## de campagne, place la caméra au zoom le plus large et au plus proche (plus une forêt et la
## Loire), attend relief fin et végétation, puis mesure le temps GPU et le temps de trame (vsync
## coupée). Chaque vue est mesurée `ROUNDS` fois ; on garde la meilleure (machine partagée).
## Usage : godot --path game --script res://tests/v4_map_bench.gd

const VIEWS := [
	["large (France)", Vector2(2213, 3204), 2600.0],
	["très proche (Paris)", Vector2(2213, 3204), 22.0],
	["forêt (Orléanais)", Vector2(2185, 3320), 90.0],
	["Loire (Orléans)", Vector2(2152, 3346), 40.0],
]
const WARMUP_FRAMES := 30
const MEASURE_FRAMES := 150
const ROUNDS := 3


func _init() -> void:
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	var viewport_rid := root.get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(viewport_rid, true)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("v4 bench: campaign map failed to load")
		quit(1)
		return
	var vegetation: Vegetation = map.get_node("Vegetation")
	# --hide=Rivers,Vegetation,... : masque des nœuds de la carte pour attribuer le coût GPU.
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--hide="):
			for node_name in arg.substr(7).split(","):
				var node := map.get_node_or_null(NodePath(node_name))
				if node is Vegetation:
					(node as Vegetation).enabled = false
				elif node is Node3D:
					(node as Node3D).visible = false
					print("V4BENCH hidden: ", node_name)
	var terrain: TerrainBuilder = map.get("terrain")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	for view in VIEWS:
		var point: Vector2 = view[1]
		rig.look_at_point(Vector3(point.x, data.surface_world_at(point.x, point.y), point.y), view[2])
		rig.snap()
		for i in WARMUP_FRAMES:
			await process_frame
		var guard := 0
		while (vegetation.pending_jobs() > 0 or not terrain.fine_ready()) and guard < 900:
			guard += 1
			await process_frame
		for i in WARMUP_FRAMES:
			await process_frame
		var best_gpu := INF
		var best_frame := INF
		for round_index in ROUNDS:
			var t0 := Time.get_ticks_usec()
			var gpu_total := 0.0
			for i in MEASURE_FRAMES:
				await process_frame
				gpu_total += RenderingServer.viewport_get_measured_render_time_gpu(viewport_rid)
			best_frame = minf(best_frame, (Time.get_ticks_usec() - t0) / 1000.0 / MEASURE_FRAMES)
			best_gpu = minf(best_gpu, gpu_total / MEASURE_FRAMES)
		var prims := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
		var draws := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
		print("V4BENCH %s (d=%d): GPU %.2f ms, frame %.2f ms (%.0f fps), %d primitives, %d draw calls" % [
			view[0], view[2], best_gpu, best_frame, 1000.0 / best_frame, prims, draws])
	map.queue_free()
	await process_frame
	quit(0)
