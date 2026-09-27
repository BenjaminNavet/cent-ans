extends SceneTree

## Banc PB1 (fenêtre réelle, vsync coupée) : chargement de la carte de campagne, temps de
## stabilisation après chaque saut de zoom (relief fin et végétation prêts), temps d'image à l'arrêt
## par vue (médiane de plusieurs échantillons espacés), puis un zoom animé France → Paris → France
## (médiane, p95 et pire image : à-coups de chargement des tuiles). Temps GPU seulement avec
## `--rendering-driver vulkan` (Metal ne les mesure pas).
## Usage : godot --path game --script res://tests/pb1_bench.gd [-- --views=2600,1250,491,150,40]
## Sortie : une ligne `PB1_JSON {...}`.

const PARIS := Vector2(2213, 1924)
const DEFAULT_VIEWS: Array[float] = [2600.0, 1250.0, 491.0, 150.0, 40.0]
const SAMPLES := 3
const SAMPLE_FRAMES := 90
const SETTLE_TIMEOUT_MS := 20000
const SWEEP_FRAMES := 240

var _viewport_rid: RID
var _result: Dictionary = {}
## `--trace` : temps cumulé par écouteur de `chunk_surface_changed` (µs) et nombre d'appels.
var _trace: Dictionary = {}
var _trace_frame: Dictionary = {}
var _last_trace_s := -1


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	_viewport_rid = root.get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(_viewport_rid, true)
	var views := DEFAULT_VIEWS.duplicate()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--views="):
			views.clear()
			for token in arg.trim_prefix("--views=").split(","):
				views.append(float(token))
	await process_frame
	var t_load := Time.get_ticks_msec()
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	while not map.get("load_ok"):
		if Time.get_ticks_msec() - t_load > 60000:
			print("PB1_JSON ", JSON.stringify({"ok": false, "error": "map load timeout"}))
			quit(1)
			return
		await process_frame
	_result["map_load_ms"] = Time.get_ticks_msec() - t_load
	_result["startup"] = map.get("startup_stats")
	var vegetation: Vegetation = map.get_node("Vegetation")
	var terrain: TerrainBuilder = map.get("terrain")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var settled := func() -> bool: return vegetation.pending_jobs() == 0 and terrain.fine_ready()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--veg-jobs="):
			vegetation.max_concurrent_jobs = int(arg.trim_prefix("--veg-jobs="))
	if "--trace" in OS.get_cmdline_user_args():
		_wrap_listeners(terrain)
	# Temps depuis le début du chargement jusqu'à la vue initiale complète (relief et végétation).
	await _settle(settled)
	_result["first_settle_ms"] = Time.get_ticks_msec() - t_load
	var surface_y := data.surface_world_at(PARIS.x, PARIS.y)
	var per_view: Dictionary = {}
	for d in views:
		rig.look_at_point(Vector3(PARIS.x, surface_y, PARIS.y), d)
		rig.snap()
		var settle_stats := await _settle_measured(settled)
		for i in 20:
			await process_frame
		var samples: Array = []
		for s in SAMPLES:
			samples.append(await _measure(SAMPLE_FRAMES))
			for i in 30:
				await process_frame
		samples.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a["median_ms"] < b["median_ms"])
		var best: Dictionary = samples[SAMPLES / 2]
		best["settle_ms"] = settle_stats["ms"]
		best["settle_worst_frame_ms"] = settle_stats["worst"]
		per_view[str(int(d))] = best
	_result["views"] = per_view
	# Zoom animé (molette continue) : France entière → Paris au plus près → France entière.
	rig.look_at_point(Vector3(PARIS.x, surface_y, PARIS.y), 2600.0)
	rig.snap()
	await _settle(settled)
	var frame_times: Array[float] = []
	var last := Time.get_ticks_usec()
	for i in SWEEP_FRAMES:
		var t := float(i) / float(SWEEP_FRAMES - 1)
		var phase := 1.0 - absf(2.0 * t - 1.0)  # 0 → 1 → 0
		rig.target_distance = exp(lerpf(log(2600.0), log(30.0), phase))
		await process_frame
		var now := Time.get_ticks_usec()
		frame_times.append((now - last) / 1000.0)
		last = now
	frame_times.sort()
	_result["sweep"] = {
		"median_ms": snappedf(frame_times[frame_times.size() / 2], 0.01),
		"p95_ms": snappedf(frame_times[int(frame_times.size() * 0.95)], 0.01),
		"max_ms": snappedf(frame_times[-1], 0.01),
		"total_ms": snappedf(_sum(frame_times), 1.0),
	}
	_result["vegetation"] = vegetation.stats
	_result["ok"] = true
	if not _trace.is_empty():
		_result["trace"] = _trace
	print("PB1_JSON ", JSON.stringify(_result))
	map.queue_free()
	await process_frame
	quit(0)


func _settle(condition: Callable) -> int:
	var stats := await _settle_measured(condition)
	return stats["ms"]


func _settle_measured(condition: Callable) -> Dictionary:
	var t0 := Time.get_ticks_msec()
	var worst := 0.0
	var last := Time.get_ticks_usec()
	await process_frame
	_trace_frame.clear()
	while not condition.call() and Time.get_ticks_msec() - t0 < SETTLE_TIMEOUT_MS:
		await process_frame
		var now := Time.get_ticks_usec()
		var frame_ms := (now - last) / 1000.0
		worst = maxf(worst, frame_ms)
		last = now
		if "--trace" in OS.get_cmdline_user_args() and Time.get_ticks_msec() / 1000 != _last_trace_s:
			_last_trace_s = Time.get_ticks_msec() / 1000
			var map_now := root.get_child(-1)
			print("PB1_T %d ms veg_pending=%d fine_ready=%s frame=%.1f" % [Time.get_ticks_msec() - t0,
				(map_now.get_node("Vegetation") as Vegetation).pending_jobs(), (map_now.get("terrain") as TerrainBuilder).fine_ready(), frame_ms])
		if frame_ms > 60.0 and root.get_child(-1).get("_PT") != null:
			var map_times: Dictionary = {}
			var pt: Dictionary = root.get_child(-1).get("_PT")
			for key in pt:
				if int(pt[key]) > 5000:
					map_times[key] = int(pt[key]) / 1000
			print("PB1_SPIKE %.1f ms: listeners %s map %s" % [frame_ms, _big(_trace_frame), map_times])
		_trace_frame.clear()
		if root.get_child(-1).get("_PT") != null:
			(root.get_child(-1).get("_PT") as Dictionary).clear()
	var map_node := root.get_child(-1)
	if Time.get_ticks_msec() - t0 > 3000 and map_node.get("terrain") != null:
		var quadtree: Variant = (map_node.get("terrain") as TerrainBuilder).get("quadtree")
		print("PB1_SLOW %d ms: vegetation pending=%d fine_ready=%s quadtree=%s missing=%s" % [Time.get_ticks_msec() - t0,
			(map_node.get_node("Vegetation") as Vegetation).pending_jobs(),
			(map_node.get("terrain") as TerrainBuilder).fine_ready(),
			(quadtree as Object).call("perf_stats") if quadtree != null else "none",
			(quadtree as Object).get("_missing_wanted") if quadtree != null else -1])
	if not condition.call():
		print("PB1_UNSETTLED vegetation pending=%d fine_ready=%s" % [
			(root.get_child(-1).get_node("Vegetation") as Vegetation).pending_jobs(),
			(root.get_child(-1).get("terrain") as TerrainBuilder).fine_ready()])
	return {"ms": Time.get_ticks_msec() - t0, "worst": snappedf(worst, 0.01)}


func _measure(count: int) -> Dictionary:
	var times: Array[float] = []
	var gpu := 0.0
	var last := Time.get_ticks_usec()
	for i in count:
		await process_frame
		var now := Time.get_ticks_usec()
		times.append((now - last) / 1000.0)
		last = now
		gpu += RenderingServer.viewport_get_measured_render_time_gpu(_viewport_rid)
	times.sort()
	return {
		"median_ms": snappedf(times[times.size() / 2], 0.01),
		"p95_ms": snappedf(times[int(times.size() * 0.95)], 0.01),
		"gpu_ms": snappedf(gpu / count, 0.01),
		"process_ms": snappedf(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0, 0.01),
		"primitives": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
		"draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
	}


func _sum(values: Array[float]) -> float:
	var total := 0.0
	for v in values:
		total += v
	return total


func _wrap_listeners(terrain: TerrainBuilder) -> void:
	for connection in terrain.get_signal_connection_list("chunk_surface_changed"):
		var callable: Callable = connection["callable"]
		var label := "%s.%s" % [(callable.get_object() as Node).name, callable.get_method()]
		terrain.chunk_surface_changed.disconnect(callable)
		terrain.chunk_surface_changed.connect(func(index: int) -> void:
			var t := Time.get_ticks_usec()
			callable.call(index)
			var dt := Time.get_ticks_usec() - t
			var entry: Array = _trace.get(label, [0, 0])
			_trace[label] = [entry[0] + dt, entry[1] + 1]
			_trace_frame[label] = int(_trace_frame.get(label, 0)) + dt)


func _big(times: Dictionary) -> Dictionary:
	var out: Dictionary = {}
	for key in times:
		if int(times[key]) > 3000:
			out[key] = int(times[key]) / 1000
	return out
