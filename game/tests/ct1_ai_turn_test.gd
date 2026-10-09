extends TestCase

## Test du lot CT1 (relecture du tour de l'IA) sur la vraie simulation et les vraies données :
## une fin de tour dans chaque mode des Réglages « Mouvements de l'IA » :
##  - « Masquer » : rien d'enregistré ni de rejoué, la fin de tour reste synchrone et le rapport
##    de saison est ouvert au retour de `_on_end_turn` ;
##  - « Montrer » : les marches des armées IA vues sont rejouées (début et fin signalés), la
##    caméra ne bouge pas, le rapport de saison apparaît après ;
##  - « Suivre » : idem, au plus `max_followed_moves` suivis, la caméra revient où elle était ;
##  - Espace (`skip`) coupe la relecture.
## Le brouillard est coupé pour que les armées IA aient un marqueur quel que soit le tirage.
## Affiche `CT1_PERF` : durée synchrone de la fin de tour et durée totale jusqu'au rapport.
## Usage : godot --headless --path game --script res://tests/ct1_ai_turn_test.gd

const TIMEOUT_MS := 90000

var _started := 0
var _finished := 0


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_ai_turn_moves"),
			"CampaignSim.get_ai_turn_moves missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "game/autosave_interval", 0, false)
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "interface/season_report", true, false)
	settings.call("set_value", "interface/confirm_end_turn", false, false)
	settings.call("set_value", "map/fog_of_war", false, false)
	settings.call("set_value", "camera/edge_pan", false, false)  # souris headless en (0, 0) : bord de l'écran
	settings.call("set_value", "map/ai_moves_speed", 4.0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	AiTurnReplay.allow_headless = true
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var replay: AiTurnReplay = map.ai_replay
	if not check(replay != null, "CampaignMap.ai_replay missing"):
		return
	replay.replay_started.connect(func(_shown: int, _followed: int) -> void: _started += 1)
	replay.replay_finished.connect(func() -> void: _finished += 1)
	var tuning := AiTurnReplay.tuning()
	check(int(tuning.get("max_followed_moves", -1)) >= 0, "tuning data loaded")

	# 1. Masquer : synchrone, rien d'enregistré.
	settings.call("set_value", "map/ai_moves", "hide", false)
	var t := Time.get_ticks_usec()
	map.call("_on_end_turn")
	var sync_ms := (Time.get_ticks_usec() - t) / 1000.0
	check(not replay.playing, "hide: no replay running")
	check(str(replay.last_stats.get("mode", "")) == "hide", "hide: mode recorded")
	check((map.sim.call("get_ai_turn_moves") as Array).is_empty(), "hide: the core recorded nothing")
	check(_report_visible(map), "hide: season report shown when _on_end_turn returns")
	check(_started == 0, "hide: replay never started")
	print("CT1_PERF hide sync %.1f ms total %.1f ms" % [sync_ms, sync_ms])
	_close_report(map)

	# 2. Montrer et 3. Suivre.
	for mode: String in ["show", "follow"]:
		settings.call("set_value", "map/ai_moves", mode, false)
		var rig: CampaignCamera = map.camera_rig
		var focus_before: Vector3 = rig.target_focus
		var started_before := _started
		var finished_before := _finished
		t = Time.get_ticks_usec()
		map.call("_on_end_turn")
		sync_ms = (Time.get_ticks_usec() - t) / 1000.0
		var stats: Dictionary = replay.last_stats
		check(int(stats.get("moves", 0)) > 0, "%s: the core recorded AI moves" % mode)
		var shown := int(stats.get("shown", 0))
		check(shown > 0, "%s: AI moves shown (fog off)" % mode)
		if shown > 0:
			check(replay.playing and _started == started_before + 1, "%s: replay started" % mode)
			check(not _report_visible(map), "%s: season report waits for the replay" % mode)
			check(replay.caption_text().begins_with("Tour de l'IA"), "%s: caption shown (%s)" % [mode, replay.caption_text()])
		if mode == "show":
			check(int(stats.get("followed", -1)) == 0, "show: the camera follows nothing")
		else:
			check(int(stats.get("followed", -1)) <= int(tuning["max_followed_moves"]), "follow: capped followed moves")
		var deadline := Time.get_ticks_msec() + TIMEOUT_MS
		while replay.playing and Time.get_ticks_msec() < deadline:
			await process_frame
		var total_ms := (Time.get_ticks_usec() - t) / 1000.0
		check(not replay.playing, "%s: replay ended" % mode)
		check(_finished == finished_before + (1 if shown > 0 else 0), "%s: replay_finished emitted" % mode)
		await process_frame
		check(_report_visible(map), "%s: season report shown after the replay" % mode)
		var back := Vector2(rig.target_focus.x, rig.target_focus.z).distance_to(Vector2(focus_before.x, focus_before.z))
		check(back < 0.5, "%s: camera back where the player was (%.2f off)" % [mode, back])
		check(replay.caption_text() == "", "%s: caption hidden" % mode)
		print("CT1_PERF %s sync %.1f ms total %.1f ms (moves %d, shown %d, followed %d, speed x4)" % [
			mode, sync_ms, total_ms, int(stats.get("moves", 0)), shown, int(stats.get("followed", 0))])
		_close_report(map)

	# 4. Espace : la relecture s'arrête tout de suite.
	settings.call("set_value", "map/ai_moves", "follow", false)
	settings.call("set_value", "map/ai_moves_speed", 1.0, false)
	map.call("_on_end_turn")
	if replay.playing:
		await process_frame
		var key := InputEventKey.new()
		key.keycode = KEY_SPACE
		key.pressed = true
		replay._unhandled_input(key)
		var frames := 0
		while replay.playing and frames < 10:
			await process_frame
			frames += 1
		check(not replay.playing, "skip: replay stopped within 10 frames")
		check(bool(replay.last_stats.get("skipped", false)), "skip: recorded as skipped")
		await process_frame
		check(_report_visible(map), "skip: season report shown")
	else:
		check(false, "skip: no replay to skip")
	map.queue_free()
	await process_frame


func _report_visible(map: Node) -> bool:
	var flow: Node = map.get("flow")
	var report: Control = flow.get("season_report") if flow != null else null
	return report != null and report.visible


func _close_report(map: Node) -> void:
	var flow: Node = map.get("flow")
	var report: Control = flow.get("season_report") if flow != null else null
	if report != null and report.visible:
		report.hide()
	if map.has_method("_close_battle_dialog"):
		map.call("_close_battle_dialog")
