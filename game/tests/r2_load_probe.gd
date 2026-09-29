extends SceneTree

## Sonde du lot OMR-R2 (protocole OM-I1) : temps de chargement de la carte de campagne réelle
## (`campaign_map.tscn`) jusqu'à la pose des textures de relief (`ReliefLandcover.pending()` faux),
## et mémoire statique Godot. Réglages : fichier de test (`user://settings.cfg` intact).
## Usage : /usr/bin/time -l godot --headless --path game --script res://tests/r2_load_probe.gd
## (RSS max lu dans la sortie de `time -l`). Sortie : une ligne `R2_LOAD {...}`.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const TIMEOUT_MS := 120000


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
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
	await process_frame
	var t_load := Time.get_ticks_msec()
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	var ready_ms := Time.get_ticks_msec() - t_load
	var frames: Array = []
	var t_frame := Time.get_ticks_msec()
	while not map.get("load_ok") or ReliefLandcover.pending():
		if Time.get_ticks_msec() - t_load > TIMEOUT_MS:
			print("R2_LOAD ", JSON.stringify({"ok": false, "error": "timeout"}))
			quit(1)
			return
		await process_frame
		frames.append(Time.get_ticks_msec() - t_frame)
		t_frame = Time.get_ticks_msec()
	var wall_ms := Time.get_ticks_msec() - t_load
	var stats: Dictionary = map.get("startup_stats")
	var result := {
		"ok": true,
		"wall_ms": wall_ms,
		"ready_ms": ready_ms,
		"frames_ms": frames.slice(0, 12),
		"frame_count": frames.size(),
		"total_ms": stats.get("total_ms"),
		"load_ms": stats.get("load_ms"),
		"terrain_ms": stats.get("terrain_ms"),
		"decor_ms": stats.get("decor_ms"),
		"controllers_ms": stats.get("controllers_ms"),
		"campaign_ms": stats.get("campaign_ms"),
		"after_campaign_ms": stats.get("after_campaign_ms"),
		"data": stats.get("data"),
		"static_mem_mb": snappedf(OS.get_static_memory_usage() / 1048576.0, 0.1),
		"static_peak_mb": snappedf(OS.get_static_memory_peak_usage() / 1048576.0, 0.1),
	}
	print("R2_LOAD ", JSON.stringify(result))
	quit(0)
