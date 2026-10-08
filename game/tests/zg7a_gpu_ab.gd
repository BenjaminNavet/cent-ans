extends SceneTree

## Banc ZG7a (ADR 0036) : coût GPU et CPU des couches de la vue rapprochée, par comparaison A/B
## alternée dans le même créneau (machine partagée) : parcellaire ZG5b (`fp_quality` 0), relief
## exagéré ZG8 (`campaign_relief_gain` 0 : plus de lecture du fond), villes ZG6 (couche masquée).
## Temps GPU seulement avec `--rendering-driver vulkan` (Metal ne les mesure pas).
## Usage : godot --path game --rendering-driver vulkan --script res://tests/zg7a_gpu_ab.gd
##         [-- --rounds=3 --configs=base,no_parcels,no_relief,no_towns]
## Sortie : une ligne `ZG7A_AB {...}` (médianes par vue et par configuration).

## Vues (x, y carte, distance) : Amiens palier vallée et site, Paris vallée, Grande Chartreuse.
const VIEWS := {
	"amiens_2": [Vector2(2224.0, 3043.6), 2.0],
	"amiens_site": [Vector2(2224.0, 3043.6), 0.0],
	"paris_5": [Vector2(2212.9, 3204.5), 5.0],
	"chartreuse_3": [Vector2(2537.6, 3772.0), 3.0],
}
const FRAMES := 60
const SETTLE_TIMEOUT_MS := 20000

var _viewport_rid: RID
var _map: Node3D
var _terrain: TerrainBuilder


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	_viewport_rid = root.get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(_viewport_rid, true)
	var rounds := 3
	var configs: Array = ["base", "no_parcels", "no_relief", "no_towns"]
	rounds = int(CmdArgs.number("--rounds", rounds))
	if CmdArgs.has("--configs"):
		configs = Array(CmdArgs.list("--configs"))
	await process_frame
	_map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(_map)
	var t0 := Time.get_ticks_msec()
	while not _map.get("load_ok"):
		if Time.get_ticks_msec() - t0 > 60000:
			print("ZG7A_AB ", JSON.stringify({"ok": false, "error": "map load timeout"}))
			quit(1)
			return
		await process_frame
	_terrain = _map.get("terrain")
	var rig: CampaignCamera = _map.camera_rig
	var data: MapData = _map.map_data
	var result := {}
	for view_name: String in VIEWS:
		var p: Vector2 = VIEWS[view_name][0]
		var d: float = VIEWS[view_name][1]
		var point := Vector3(p.x, data.surface_world_at(p.x, p.y), p.y)
		if d <= 0.0:
			d = rig.min_distance_at(point)
		rig.look_at_point(point, d)
		rig.snap()
		await _settle()
		var samples := {}
		for r in rounds:
			for config: String in configs:
				_apply(config, true)
				for i in 10:
					await process_frame
				var m := await _measure()
				_apply(config, false)
				if not samples.has(config):
					samples[config] = []
				(samples[config] as Array).append(m)
		var per := {}
		for config: String in samples:
			per[config] = _median_of(samples[config])
		result[view_name] = per
	print("ZG7A_AB ", JSON.stringify({"ok": true, "rounds": rounds, "views": result}))
	_map.queue_free()
	await process_frame
	quit(0)


var _saved := {}


func _apply(config: String, on: bool) -> void:
	# `mat:nom=valeur;nom=valeur` : uniformes du matériau du terrain (restaurés ensuite).
	if config.begins_with("mat:"):
		for pair in config.trim_prefix("mat:").split(";"):
			var kv := pair.split("=")
			if on:
				_saved[kv[0]] = _terrain.material.get_shader_parameter(kv[0])
				_terrain.material.set_shader_parameter(kv[0], int(kv[1]) if kv[1].is_valid_int() else float(kv[1]))
			else:
				_terrain.material.set_shader_parameter(kv[0], _saved[kv[0]])
		return
	match config:
		"no_parcels":
			_terrain.material.set_shader_parameter("fp_quality", 0 if on else 2)
		"no_relief":
			RenderingServer.global_shader_parameter_set("campaign_relief_gain", 0.0 if on else MapData.relief_gain())
		"no_towns":
			var towns := _map.find_child("Towns", true, false) as Node3D
			if towns != null:
				towns.visible = not on


func _settle() -> void:
	var t0 := Time.get_ticks_msec()
	var vegetation: Vegetation = _map.get_node("Vegetation")
	await process_frame
	while not (vegetation.pending_jobs() == 0 and _terrain.fine_ready()) and Time.get_ticks_msec() - t0 < SETTLE_TIMEOUT_MS:
		await process_frame
	# Villes et rubans fins : construction étalée, quelques secondes de plus.
	var t1 := Time.get_ticks_msec()
	while Time.get_ticks_msec() - t1 < 6000:
		await process_frame


func _measure() -> Dictionary:
	var frames: Array[float] = []
	var gpus: Array[float] = []
	var last := Time.get_ticks_usec()
	for i in FRAMES:
		await process_frame
		var now := Time.get_ticks_usec()
		frames.append((now - last) / 1000.0)
		last = now
		gpus.append(RenderingServer.viewport_get_measured_render_time_gpu(_viewport_rid))
	frames.sort()
	gpus.sort()
	return {
		"frame_ms": frames[frames.size() / 2],
		"gpu_ms": gpus[gpus.size() / 2],
		"draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		"primitives": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
	}


func _median_of(list: Array) -> Dictionary:
	var out := {}
	for key in ["frame_ms", "gpu_ms", "draw_calls", "primitives"]:
		var values: Array = []
		for m: Dictionary in list:
			values.append(float(m[key]))
		values.sort()
		out[key] = snappedf(values[values.size() / 2], 0.01)
	return out
