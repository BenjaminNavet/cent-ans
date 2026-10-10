extends SceneTree

## Banc FPS de bataille : France contre Angleterre (armées principales de 1337), IA des deux camps,
## caméra posée sur le barycentre des régiments présents, à plusieurs distances. Fenêtré (le GPU
## compte), synchronisation verticale coupée. Imprime `battle_fps {...}` par vue puis quitte.
## Usage (fenêtre en arrière-plan, jamais en direct) :
##   tools/godot_bg.sh --path game --script res://tests/battle_fps_bench.gd -- \
##       [--views=overview:120:0.6,mid:55:0.6,close:22:0.6] [--seconds=12] [--warmup=8]
##       [--window=2624x1644] [--units=N]
## Vue : nom:distance(m):lacet(rad). `--warmup` : secondes de bataille avant la première vue
## (approche des lignes). `--bench-probe` : sections `PerfProbe` de l'image. `--window` : taille
## de fenêtre imposée (sinon celle du projet). `--units` est lu par la scène de bataille.

var _views := "overview:120:0.6,mid:55:0.6,close:22:0.6"
var _seconds := 12.0
var _warmup := 8.0
var _scene: Node


func _init() -> void:
	_views = CmdArgs.value("--views", _views)
	_seconds = CmdArgs.number("--seconds", _seconds)
	_warmup = CmdArgs.number("--warmup", _warmup)
	PerfProbe.enabled = CmdArgs.has("--bench-probe")
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("battle_fps_bench: campagne introuvable")
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.set("autoplay", true)
	_scene.configure(sim, index, 11)
	root.add_child(_scene)
	var window := CmdArgs.value("--window", "")
	if window.contains("x"):
		DisplayServer.window_set_size(Vector2i(int(window.get_slice("x", 0)), int(window.get_slice("x", 1))))
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	RenderingServer.viewport_set_measure_render_time(root.get_viewport_rid(), true)
	_scene.camera_rig.edge_pan_enabled = false
	await _hold(_warmup, false)
	for view in _views.split(",", false):
		var parts := view.split(":")
		await _hold(1.0, false, float(parts[1]), float(parts[2]))  # cadrage, chargement
		var report := await _hold(_seconds, true, float(parts[1]), float(parts[2]))
		report["view"] = parts[0]
		print("battle_fps ", JSON.stringify(report))
	quit(0)


## Barycentre des régiments présents (plan du sol).
func _centre() -> Vector3:
	var sum := Vector3.ZERO
	var count := 0
	for unit: Dictionary in _scene.get("units"):
		if bool(unit.get("present", true)):
			sum += Vector3(float(unit["x"]), 0.0, float(unit["z"]))
			count += 1
	return sum / maxf(count, 1.0)


## Tient `seconds` en recadrant la caméra sur les régiments ; avec `measure`, rend le rapport.
func _hold(seconds: float, measure: bool, distance := 0.0, yaw := 0.0) -> Dictionary:
	var frame_ms := PackedFloat32Array()
	var process_ms := PackedFloat32Array()
	var draws := PackedFloat32Array()
	var prims := PackedFloat32Array()
	var probe_sum: Dictionary = {}
	var probe_max: Dictionary = {}
	var start := Time.get_ticks_usec()
	var last := start
	PerfProbe.take_frame()
	while Time.get_ticks_usec() - start < int(seconds * 1.0e6):
		if distance > 0.0:
			_scene.camera_rig.look_at_point(_centre(), distance, yaw)
		await process_frame
		var now := Time.get_ticks_usec()
		if measure:
			frame_ms.append((now - last) / 1000.0)
			process_ms.append(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0)
			draws.append(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME))
			prims.append(Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME))
			var sections := PerfProbe.take_frame()
			for label: String in sections:
				var ms := int(sections[label]) / 1000.0
				probe_sum[label] = float(probe_sum.get(label, 0.0)) + ms
				probe_max[label] = maxf(float(probe_max.get(label, 0.0)), ms)
		last = now
	if not measure:
		return {}
	var n := frame_ms.size()
	var sorted := frame_ms.duplicate()
	sorted.sort()
	var probe: Dictionary = {}
	for label: String in probe_sum:
		probe[label] = {"mean_ms": snappedf(float(probe_sum[label]) / maxf(n, 1.0), 0.01), "max_ms": snappedf(float(probe_max[label]), 0.1)}
	return {
		"frames": n, "fps_avg": snappedf(n / maxf(seconds, 0.001), 0.1),
		"frame_ms_p50": snappedf(sorted[n / 2], 0.01), "frame_ms_p99": snappedf(sorted[int(n * 0.99)], 0.01),
		"frame_ms_max": snappedf(sorted[n - 1], 0.01), "process_ms_p50": _median(process_ms),
		"draw_calls_p50": _median(draws), "primitives_p50": _median(prims),
		"window": DisplayServer.window_get_size(), "scale_3d": root.scaling_3d_scale,
		"units": (_scene.get("units") as Array).size(), "probe": probe,
		"battle_elapsed_s": snappedf(float(_scene.battle.call("get_elapsed")), 0.1),
		"finished": bool(_scene.battle.call("is_finished")), "paused": bool(_scene.get("paused")),
	}


func _median(values: PackedFloat32Array) -> float:
	if values.is_empty():
		return 0.0
	var sorted := values.duplicate()
	sorted.sort()
	return snappedf(sorted[sorted.size() / 2], 0.01)
