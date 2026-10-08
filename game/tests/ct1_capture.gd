extends SceneTree

## Captures du lot CT1 (fenêtre, pas headless) : la caméra qui suit les armées IA pendant la
## relecture du tour de l'IA. Écrit `docs/audit/captures/ct1/suivi_*.png` (trois marches suivies).
## Usage : godot --path game --script res://tests/ct1_capture.gd -- [--no-fog] [--turns=N]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const OUT_DIR := "docs/audit/captures/ct1"


func _init() -> void:
	_run.call_deferred()


func _save(name: String) -> void:
	var image := root.get_viewport().get_texture().get_image()
	var path := MAP_PATHS.project_root().path_join(OUT_DIR).path_join(name)
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	image.save_png(path)
	print("CT1_CAPTURE ", path)


func _run() -> void:
	var fog := not CmdArgs.has("--no-fog")
	var turns := int(CmdArgs.number("--turns", 3))
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "game/autosave_interval", 0, false)
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "interface/season_report", true, false)
	settings.call("set_value", "map/fog_of_war", fog, false)
	settings.call("set_value", "camera/edge_pan", false, false)
	settings.call("set_value", "map/ai_moves", "follow", false)
	settings.call("set_value", "map/ai_moves_speed", 1.0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	var t0 := Time.get_ticks_msec()
	while not map.get("load_ok") and Time.get_ticks_msec() - t0 < 60000:
		await process_frame
	for i in 60:
		await process_frame
	var replay: AiTurnReplay = map.ai_replay
	var shots := 0
	for turn in turns:
		var ui: Node = map.get("ui")
		var stack: Object = ui.get("panels")
		for i in 8:  # panneaux ouverts par la fin de tour précédente (diplomatie, chronique…)
			if stack == null or not bool(stack.call("close_top")):
				break
		var notice := ui.find_child("ReliefCacheNotice", true, false) as Control
		if notice != null:
			notice.hide()
		map.call("_on_end_turn")
		print("CT1_CAPTURE turn %d stats %s" % [turn, replay.last_stats])
		var last_follow := ""
		while replay.playing:
			await process_frame
			var following: String = replay.get("_follow_army")
			if following != "" and following != last_follow and shots < 3:
				last_follow = following
				for i in 45:  # au milieu de la marche suivie
					await process_frame
				if replay.playing:
					shots += 1
					_save("suivi_%d.png" % shots)
		for i in 30:
			await process_frame
		var flow: Node = map.get("flow")
		if flow != null and flow.get("season_report") != null:
			(flow.get("season_report") as Control).hide()
		map.call("_close_battle_dialog")
		if shots >= 3:
			break
	quit(0)
