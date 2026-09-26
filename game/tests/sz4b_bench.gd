extends SceneTree

## Banc SZ4b (fenêtre réelle, vsync coupée) : i/s et instances d'arbres au palier vallée.
## Fonctionne aussi sur `main` (sans `forest_detail`, les instances denses valent 0).
##   godot --path game --script res://tests/sz4b_bench.gd -- --map-weather=clear
## Une ligne `SZ4b bench <vue> fps=… frame_ms=… trees=… dense=… prims=…` par vue.

const VIEWS := [
	["foret_orleans_d6", Vector2(2180.0, 2053.0), 6.0],
	["foret_orleans_d10", Vector2(2180.0, 2053.0), 10.0],
	["foret_compiegne_d6", Vector2(2285.0, 1844.0), 6.0],
	["crecy_d10", Vector2(2191.5, 1706.0), 10.0],
	["amiens_d14", Vector2(2224.0, 1763.6), 14.0],
]
const WARMUP_FRAMES := 30
const MEASURE_FRAMES := 240


func _init() -> void:
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 180:
		await process_frame
	if not map.get("load_ok"):
		push_error("sz4b bench: campaign map failed to load")
		quit(1)
		return
	var vegetation: Node = map.get_node_or_null("Vegetation")
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	for view: Array in VIEWS:
		var point: Vector2 = view[1]
		var distance: float = view[2]
		rig.look_at_point(Vector3(point.x, data.surface_world_at(point.x, point.y), point.y), distance)
		rig.snap()
		for i in 30:
			await process_frame
		var guard := 0
		while guard < 1500:
			guard += 1
			var busy := false
			if vegetation != null and vegetation.has_method("pending_jobs") and vegetation.pending_jobs() > 0:
				busy = true
			if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
				busy = true
			if not busy:
				break
			await process_frame
		var forest: Variant = vegetation.get("forest_detail") if vegetation != null else null
		if forest != null:
			(forest as Object).call("flush", point, distance)
		for i in WARMUP_FRAMES:
			await process_frame
		var start := Time.get_ticks_usec()
		for i in MEASURE_FRAMES:
			await process_frame
		var elapsed_ms := (Time.get_ticks_usec() - start) / 1000.0
		var dense := 0
		if forest != null:
			dense = int(((forest as Object).get("stats") as Dictionary).get("visible", 0))
		var prims := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
		var draws := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
		print("SZ4b bench %s fps=%.1f frame_ms=%.2f trees=%d dense=%d prims=%d draws=%d" % [
			view[0], MEASURE_FRAMES * 1000.0 / elapsed_ms, elapsed_ms / MEASURE_FRAMES,
			vegetation.instance_count() if vegetation != null else 0, dense, prims, draws])
	map.queue_free()
	await process_frame
	quit(0)
