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
var _map_ab := 0.0


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
		elif arg.begins_with("--map-ab="):
			_map_ab = float(arg.trim_prefix("--map-ab="))
	_isolate()
	_run.call_deferred()


## Réglages, codex et sauvegardes du parcours dans un dossier à part (effacé à la fin) : le
## joueur ne retrouve ni sauvegarde automatique ni réglage modifié, et le parcours mesure les
## réglages par défaut.
var _root_dir := ""


func _isolate() -> void:
	_root_dir = "user://rl1_journey_%d" % OS.get_process_id()
	DirAccess.make_dir_recursive_absolute(_root_dir.path_join("saves"))
	var root := get_tree().root
	var facade := root.get_node_or_null("SimFacade")
	if facade != null:
		facade.call("use_test_saves_dir", _root_dir.path_join("saves"))
	SaveSlots.use_test_dir(_root_dir.path_join("saves"))
	var settings := root.get_node_or_null("Settings")
	if settings != null:
		settings.call("use_test_file", _root_dir.path_join("settings.cfg"))
	var codex := root.get_node_or_null("CodexStore")
	if codex != null:
		codex.call("use_test_file", _root_dir.path_join("codex.json"))
	RenderQuality.reapply(get_tree())


func _cleanup() -> void:
	if _root_dir == "":
		return
	for sub in ["saves", ""]:
		var dir_path := _root_dir.path_join(sub) if sub != "" else _root_dir
		for file in DirAccess.get_files_at(dir_path):
			DirAccess.remove_absolute(dir_path.path_join(file))
		DirAccess.remove_absolute(dir_path)


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
	_result["window"] = [DisplayServer.window_get_size(), DisplayServer.screen_get_scale(), get_tree().root.size]
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
	if _map_ab > 0.0:
		var y0: float = map_data.call("surface_world_at", PARIS.x, PARIS.y)
		rig.call("look_at_point", Vector3(PARIS.x, y0, PARIS.y), _map_ab)
		rig.call("snap")
		await _frames_passed(120)
		_result["map_ab"] = await _ab(map)
		_result["ok"] = true
		print("JOURNEY_JSON %s" % JSON.stringify(_result))
		_cleanup()
		get_tree().quit(0)
		return
	var zooms: Dictionary = {}
	for d: float in ZOOMS:
		var y: float = map_data.call("surface_world_at", PARIS.x, PARIS.y)
		rig.call("look_at_point", Vector3(PARIS.x, y, PARIS.y), d)
		rig.call("snap")
		await _frames_passed(90)
		var terrain: Node = map.get("terrain")
		if terrain != null and terrain.has_method("fine_ready"):
			await _wait_for(func() -> bool: return bool(terrain.call("fine_ready")), 20.0)
		var overlay := map.find_child("ParchmentOverlay", true, false)
		var draws_before := int(overlay.get("draw_count")) if overlay != null else 0
		var measured := await _measure(_frames)
		if overlay != null:
			measured["parchment_draws"] = int(overlay.get("draw_count")) - draws_before
		zooms[str(int(d))] = measured
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
		var t_save := Time.get_ticks_usec()
		var saved := bool(facade.call("save_game", "journey"))
		_result["save_ms"] = (Time.get_ticks_usec() - t_save) / 1000.0
		_result["save_ok"] = saved and FileAccess.file_exists(str(facade.call("save_path", "journey")))
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
		# Sortie propre : la bataille et la carte libérées avant de quitter (pas de fuite au journal).
		for child in get_tree().root.get_children():
			if child is BattleScene:
				child.queue_free()
		get_tree().current_scene.queue_free()
		await _frames_passed(5)
	_result["ok"] = true
	_result["wall_ms"] = Time.get_ticks_msec()
	print("JOURNEY_JSON %s" % JSON.stringify(_result))
	_cleanup()
	get_tree().quit(0)


func _fail(message: String) -> void:
	_result["error"] = message
	_result["wall_ms"] = Time.get_ticks_msec()
	print("JOURNEY_JSON %s" % JSON.stringify(_result))
	_cleanup()
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


## `--map-ab=<distance>` : coûts isolés à cette distance, configurations alternées dans le même
## processus (même charge machine pour toutes) ; médianes du temps d'image et du GPU (Vulkan).
## `--ab-configs=base,medium,hide:Sea,…` : liste des configurations (voir `_apply_config`).
func _ab(map: Node) -> Dictionary:
	var configs: Array = ["base", "medium", "low", "no_ssil", "no_ssao", "no_glow", "msaa_off", "no_dof", "no_shadows", "scale75"]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--ab-configs="):
			configs = Array(arg.trim_prefix("--ab-configs=").split(","))
	var samples: Dictionary = {}
	var gpu: Dictionary = {}
	for config: String in configs:
		samples[config] = []
		gpu[config] = []
	var vp := get_tree().root.get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(vp, true)
	for round_index in 4:
		for config: String in configs:
			_apply_config(map, config)
			await _frames_passed(12)
			var last := Time.get_ticks_usec()
			for i in 30:
				if config == "no_shadows":
					(map.get_node("Sun") as DirectionalLight3D).shadow_enabled = false
				await get_tree().process_frame
				var now := Time.get_ticks_usec()
				samples[config].append((now - last) / 1000.0)
				gpu[config].append(RenderingServer.viewport_get_measured_render_time_gpu(vp))
				last = now
	_apply_config(map, "base")
	var out: Dictionary = {}
	for config: String in configs:
		var frame: Array = samples[config]
		var g: Array = gpu[config]
		frame.sort()
		g.sort()
		out[config] = {"frame_ms": snappedf(frame[frame.size() / 2], 0.01), "gpu_ms": snappedf(g[g.size() / 2], 0.01)}
	return out


var _hidden: Array[Node] = []


func _apply_config(map: Node, config: String) -> void:
	RenderQuality.override_level = config if config in ["low", "medium", "ultra"] else ""
	RenderQuality.reapply(get_tree())
	for node in _hidden:
		if is_instance_valid(node):
			node.set("visible", true)
	_hidden.clear()
	var env := (map.get_node("WorldEnvironment") as WorldEnvironment).environment
	var camera := map.get("camera") as Camera3D
	var attributes := camera.attributes as CameraAttributesPractical
	if attributes == null:
		attributes = (map.get_node("WorldEnvironment") as WorldEnvironment).camera_attributes as CameraAttributesPractical
	if attributes != null:
		if not attributes.has_meta("rl1_dof"):
			attributes.set_meta("rl1_dof", attributes.dof_blur_far_enabled)
		attributes.dof_blur_far_enabled = bool(attributes.get_meta("rl1_dof")) and config != "no_dof"
	get_tree().root.scaling_3d_scale = 0.75 if config == "scale75" else 1.0
	var terrain: Node = map.get("terrain")
	if terrain != null:
		terrain.set("fine_enabled", config != "no_fine")
	match config:
		"no_ssil":
			env.ssil_enabled = false
		"no_ssao":
			env.ssao_enabled = false
		"no_glow":
			env.glow_enabled = false
		"no_fog":
			env.fog_enabled = false
		"msaa_off":
			get_tree().root.msaa_3d = Viewport.MSAA_DISABLED
		"shadow_4096":
			RenderingServer.directional_shadow_atlas_set_size(4096, true)
	if config.begins_with("hide:"):
		var node := map.find_child(config.trim_prefix("hide:"), true, false)
		if node != null and node.get("visible") != null:
			node.set("visible", false)
			_hidden.append(node)
