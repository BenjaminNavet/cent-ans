class_name ReleaseJourney
extends Node

## RL1 : parcours de vérification du jeu (exporté ou éditeur), sans intervention. Lancé par le
## menu principal quand `--journey` suit `--` (`--script` n'existe pas dans un jeu exporté) :
##
##   "Cent Ans.app/Contents/MacOS/Cent Ans" -- --journey [options]
##   godot --path game -- --journey [options]
##   … -- --journey=battle --benchmark --units=20   (banc de bataille T8 dans le jeu exporté)
##
## Étapes : menu 3D → nouvelle campagne (France) → carte (3 zooms) → fins de tour → sauvegarde
## (dossier isolé, effacé ensuite) → bataille France–Angleterre lancée depuis la carte → sortie.
## Imprime une ligne `JOURNEY_JSON {...}` (temps de démarrage, i/s médianes, fins de tour) puis
## quitte avec le code 0 (succès) ou 1 (étape échouée).
## Options : `--turns=<n>` (3), `--frames=<n>` par mesure (180), `--no-battle`, `--uncapped`
## (vsync coupée, i/s non plafonnées : temps d'image réels).

const CAMPAIGN_SCENE := "res://scenes/campaign_map.tscn"
const PARIS := Vector2(2213.0, 1924.0)
const ZOOMS := [1500.0, 1250.0, 491.0, 150.0]
const TIMEOUT_S := 240.0

var _result: Dictionary = {"ok": false}
var _turns := 3
var _frames := 180
var _battle := true
var _uncapped := false


## Appelé par `StartMenu._ready` : vrai si la ligne de commande demande le parcours (le nœud
## est alors ajouté à la racine et survit aux changements de scène).
static func maybe_start(tree: SceneTree) -> bool:
	for arg in OS.get_cmdline_user_args():
		if arg == "--journey=battle":
			tree.change_scene_to_file.call_deferred("res://scenes/battle/battle.tscn")
			return true
		if arg == "--journey":
			if tree.root.get_node_or_null("ReleaseJourney") != null:
				return false  # retour au menu pendant le parcours
			var journey := ReleaseJourney.new()
			journey.name = "ReleaseJourney"
			tree.root.add_child.call_deferred(journey)
			return true
	return false


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--turns="):
			_turns = int(arg.trim_prefix("--turns="))
		elif arg.begins_with("--frames="):
			_frames = int(arg.trim_prefix("--frames="))
		elif arg == "--no-battle":
			_battle = false
		elif arg == "--uncapped":
			_uncapped = true
	_run.call_deferred()


func _run() -> void:
	var started := Time.get_ticks_msec()
	if _uncapped:
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		Engine.max_fps = 0
	_result["uncapped"] = _uncapped
	var tree := get_tree()
	var root := tree.root
	_result["engine_startup_ms"] = started  # moteur, autoloads et menu jusqu'au parcours
	_result["quality"] = RenderQuality.current()
	_result["adapter"] = RenderingServer.get_video_adapter_name()
	_result["template"] = OS.has_feature("template")
	var facade: Node = root.get_node_or_null("SimFacade")
	_result["real_sim"] = facade != null and bool(facade.get("is_real"))
	# Menu
	await _frames_passed(2)
	_result["menu_first_frame_ms"] = Time.get_ticks_msec()
	await _frames_passed(60)
	_result["menu"] = await _measure(_frames)
	# Campagne
	var t0 := Time.get_ticks_msec()
	if facade != null:
		facade.set("pending_faction", "fac_france")
		facade.set("pending_seed", 1337)
		facade.set("pending_load_path", "")
	tree.change_scene_to_file(CAMPAIGN_SCENE)
	var map: Node = await _wait_for(func() -> bool: return tree.current_scene != null and tree.current_scene.get("load_ok") == true)
	if map == null:
		return _fail("campaign map did not load")
	_result["campaign_load_ms"] = Time.get_ticks_msec() - t0
	_result["campaign_startup"] = map.get("startup_stats")
	var sim: Object = map.get("sim")
	var rig: Node = map.get("camera_rig")
	var map_data: Object = map.get("map_data")
	var zooms: Dictionary = {}
	for d: float in ZOOMS:
		var y: float = map_data.call("surface_world_at", PARIS.x, PARIS.y)
		rig.call("look_at_point", Vector3(PARIS.x, y, PARIS.y), d)
		rig.call("snap")
		await _frames_passed(90)
		var terrain: Node = map.get("terrain")
		if terrain != null and terrain.has_method("fine_ready"):
			await _wait_for(func() -> bool: return bool(terrain.call("fine_ready")), 20.0)
		zooms[str(int(d))] = await _measure(_frames)
	_result["map"] = zooms
	# Fins de tour : temps du cœur seul et de la fin de tour complète de la carte.
	var core_ms: Array[float] = []
	for i in _turns:
		var t := Time.get_ticks_usec()
		sim.call("end_turn")
		core_ms.append((Time.get_ticks_usec() - t) / 1000.0)
		await _frames_passed(2)
	_result["end_turn_core_ms"] = core_ms
	var t_full := Time.get_ticks_usec()
	map.call("_on_end_turn")
	_result["end_turn_full_ms"] = (Time.get_ticks_usec() - t_full) / 1000.0
	await _frames_passed(10)
	# Sauvegarde dans un dossier isolé (les sauvegardes du joueur ne sont pas touchées).
	if facade != null:
		var saves_dir := "user://rl1_journey_%d" % OS.get_process_id()
		facade.call("use_test_saves_dir", saves_dir)
		var t_save := Time.get_ticks_usec()
		var saved := bool(facade.call("save_game", "journey"))
		_result["save_ms"] = (Time.get_ticks_usec() - t_save) / 1000.0
		_result["save_ok"] = saved and FileAccess.file_exists(str(facade.call("save_path", "journey")))
		DirAccess.remove_absolute(str(facade.call("save_path", "journey")))
		DirAccess.remove_absolute(saves_dir)
		if not _result["save_ok"]:
			return _fail("save failed")
	# Bataille lancée depuis la carte (même chemin que le bouton « Combattre »).
	if _battle:
		if not sim.has_method("debug_stage_battle"):
			return _fail("no debug_stage_battle")
		var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
		if armies.size() < 2:
			return _fail("no armies to stage a battle")
		sim.call("debug_stage_battle", armies[0], armies[1])
		var pending: Array = sim.call("get_pending_battles")
		if pending.is_empty():
			return _fail("no pending battle")
		var t_battle := Time.get_ticks_msec()
		map.call("_on_battle_fight", 0, 1337)
		await _frames_passed(3)
		_result["battle_load_ms"] = Time.get_ticks_msec() - t_battle
		await _frames_passed(120)
		_result["battle"] = await _measure(_frames)
	_result["ok"] = true
	_result["wall_ms"] = Time.get_ticks_msec()
	print("JOURNEY_JSON %s" % JSON.stringify(_result))
	get_tree().quit(0)


func _fail(message: String) -> void:
	_result["error"] = message
	_result["wall_ms"] = Time.get_ticks_msec()
	print("JOURNEY_JSON %s" % JSON.stringify(_result))
	get_tree().quit(1)


func _frames_passed(count: int) -> void:
	for i in count:
		await get_tree().process_frame


func _wait_for(condition: Callable, timeout_s: float = TIMEOUT_S) -> Variant:
	var deadline := Time.get_ticks_msec() + int(timeout_s * 1000.0)
	while Time.get_ticks_msec() < deadline:
		if condition.call():
			return get_tree().current_scene
		await get_tree().process_frame
	return null


## Médiane et p95 du temps d'image (ms), plus temps CPU/GPU de rendu mesurés.
func _measure(count: int) -> Dictionary:
	var vp := get_tree().root.get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(vp, true)
	var times: Array[float] = []
	var gpu := 0.0
	var cpu := 0.0
	var last := Time.get_ticks_usec()
	for i in count:
		await get_tree().process_frame
		var now := Time.get_ticks_usec()
		times.append((now - last) / 1000.0)
		last = now
		gpu += RenderingServer.viewport_get_measured_render_time_gpu(vp)
		cpu += RenderingServer.viewport_get_measured_render_time_cpu(vp) + RenderingServer.get_frame_setup_time_cpu()
	times.sort()
	return {
		"median_ms": snappedf(times[times.size() / 2], 0.01),
		"p95_ms": snappedf(times[int(times.size() * 0.95)], 0.01),
		"max_ms": snappedf(times[-1], 0.01),
		"gpu_ms": snappedf(gpu / count, 0.01),
		"render_cpu_ms": snappedf(cpu / count, 0.01),
		"process_ms": snappedf(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0, 0.01),
		"primitives": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
		"draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
	}
