extends SceneTree

## Banc d'essai visuel du lot V3 (fenêtre réelle, pas headless) : charge la carte de campagne,
## place la caméra à plusieurs zooms, attend la fin du semis de la végétation puis mesure le
## temps de trame (vsync coupée), les primitives et les appels de dessin.
## Usage : godot --path game --script res://tests/vegetation_bench.gd [-- --no-vegetation]

const VIEWS := [
	["France entière", Vector2(2000, 1900), 1400.0],
	["zoom moyen", Vector2(2000, 2000), 400.0],
	["proche", Vector2(1950, 1850), 150.0],
	["forêt d'Orléans", Vector2(2144, 2054), 150.0],
	["très proche", Vector2(1950, 1850), 40.0],
	["bocage", Vector2(1900, 2098), 60.0],
]
const WARMUP_FRAMES := 20
const MEASURE_FRAMES := 180


func _init() -> void:
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	var viewport_rid := root.get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(viewport_rid, true)
	# Les autoloads (MapPaths, SimFacade) ne sont enregistrés qu'après _init.
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("bench: campaign map failed to load")
		quit(1)
		return
	var vegetation: Vegetation = map.get_node("Vegetation")
	# `-- --no-vegetation` : référence sans arbres.
	vegetation.enabled = not OS.get_cmdline_user_args().has("--no-vegetation")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	for view in VIEWS:
		var point: Vector2 = view[1]
		rig.look_at_point(Vector3(point.x, data.surface_world_at(point.x, point.y), point.y), view[2])
		rig.snap()
		for i in WARMUP_FRAMES:
			await process_frame
		var guard := 0
		while vegetation.pending_jobs() > 0 and guard < 600:
			guard += 1
			await process_frame
		for i in WARMUP_FRAMES:
			await process_frame
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		var t0 := Time.get_ticks_usec()
		var worst := 0
		var last := t0
		var gpu_total := 0.0
		var gpu_worst := 0.0
		for i in MEASURE_FRAMES:
			await process_frame
			var now := Time.get_ticks_usec()
			worst = maxi(worst, now - last)
			last = now
			var gpu := RenderingServer.viewport_get_measured_render_time_gpu(viewport_rid)
			gpu_total += gpu
			gpu_worst = maxf(gpu_worst, gpu)
		var avg_ms := (Time.get_ticks_usec() - t0) / 1000.0 / MEASURE_FRAMES
		var prims := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
		var draws := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
		print("BENCH %s (d=%d): GPU %.2f ms (max %.1f), frame %.2f ms (%.0f fps), worst %.1f ms, %d primitives, %d draw calls, vegetation %d tiles / %d instances" % [
			view[0], view[2], gpu_total / MEASURE_FRAMES, gpu_worst, avg_ms, 1000.0 / avg_ms, worst / 1000.0, prims, draws, vegetation.tile_count(), vegetation.instance_count()])
	print("BENCH vegetation stats: %s" % JSON.stringify(vegetation.stats))
	map.queue_free()
	await process_frame
	quit(0)
