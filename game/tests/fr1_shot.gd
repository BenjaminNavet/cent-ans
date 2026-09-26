extends SceneTree

## Captures et mesures du lot FR1 (frontières de faction) : frontière France / Angleterre au plus
## haut relief trouvé, à trois zooms (Europe, comté, vallée ~2 km) et en vue parchemin ; pour
## chacune, temps GPU médian avec et sans les frontières (A/B entrelacé, --rendering-driver
## vulkan : Metal ne mesure pas le GPU). Fenêtre obligatoire.
## Usage : godot --path game --script res://tests/fr1_shot.gd -- <dossier> [--quality=high]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const VIEW := Vector2i(1920, 1080)
const MEASURE_FRAMES := 30
const ROUNDS := 6

var _map: Node3D


func _init() -> void:
	await process_frame
	var args := OS.get_cmdline_user_args()
	var folder: String = args[0] if not args.is_empty() and not args[0].begins_with("--") else "user://"
	root.size = VIEW
	DisplayServer.window_set_size(VIEW)
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	_map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(_map)
	for i in 30:
		await process_frame
	_map.camera_rig.min_distance = 0.3
	_map.camera_rig.close_min_distance = 0.3
	RenderingServer.viewport_set_measure_render_time(root.get_viewport_rid(), true)
	var target := _border_point("fac_france", "fac_england")
	print("fr1_shot: border point %s" % target)
	var ground := Vector3(target.x, _map.terrain.surface_height_at(target.x, target.y), target.y)
	var views := [["europe", 700.0], ["comte", 60.0], ["vallee", 2.8], ["parchemin", 1800.0]]
	for view in views:
		_map.camera_rig.look_at_point(ground, float(view[1]))
		_map.camera_rig.snap()
		for i in 40:
			await process_frame
		if _map.terrain.has_method("wait_fine_jobs"):
			_map.terrain.wait_fine_jobs()
		for i in 20:
			await process_frame
		_shot(folder.path_join("fr1-%s.png" % view[0]))
		_map.faction_borders.set_enabled(false)
		for i in 3:
			await process_frame
		_shot(folder.path_join("fr1-%s-sans.png" % view[0]))
		# A/B entrelacé (dérive thermique, streaming) : médiane par état sur ROUNDS alternances.
		var on_samples: Array[float] = []
		var off_samples: Array[float] = []
		for round in ROUNDS:
			_map.faction_borders.set_enabled(true)
			on_samples.append_array(await _gpu_samples())
			_map.faction_borders.set_enabled(false)
			off_samples.append_array(await _gpu_samples())
		_map.faction_borders.set_enabled(true)
		var on_ms := _median(on_samples)
		var off_ms := _median(off_samples)
		print("fr1_shot: %s distance=%.1f alpha=%.2f gpu_on=%.3f ms gpu_off=%.3f ms delta=%.3f ms" % [
				view[0], _map.camera_rig.distance, _map.faction_borders.effective_alpha, on_ms, off_ms, on_ms - off_ms])
	quit(0)


func _gpu_samples() -> Array[float]:
	for i in 4:
		await process_frame
	var samples: Array[float] = []
	for i in MEASURE_FRAMES:
		await process_frame
		samples.append(RenderingServer.viewport_get_measured_render_time_gpu(root.get_viewport_rid()))
	return samples


static func _median(values: Array[float]) -> float:
	var sorted := values.duplicate()
	sorted.sort()
	return sorted[sorted.size() / 2] if not sorted.is_empty() else 0.0


func _shot(path: String) -> void:
	var image := root.get_texture().get_image()
	print("fr1_shot: %s → %s" % [path, error_string(image.save_png(path))])


## Point de frontière entre deux factions au relief le plus haut (drapé sur les collines).
func _border_point(a: String, b: String) -> Vector2:
	var data: MapData = _map.map_data
	var owner_of := {}
	for index in range(1, data.province_count + 1):
		var id := str(data.get_province(index).get("id", ""))
		var state: Dictionary = _map.sim.call("get_province_state", id)
		owner_of[index] = str(state.get("owner", ""))
	var best := Vector2(data.size) * 0.5
	var best_h := -1e9
	for y in range(8, data.size.y - 8, 3):
		for x in range(8, data.size.x - 8, 3):
			var p := data.province_index_at(x, y)
			var q := data.province_index_at(x + 3, y)
			if p <= 0 or q <= 0 or p == q:
				continue
			var pair := [owner_of.get(p, ""), owner_of.get(q, "")]
			if not (a in pair and b in pair):
				continue
			var h := data.height_m_at(x, y)
			if h > best_h:
				best_h = h
				best = Vector2(x + 1.5, y)
	return best
