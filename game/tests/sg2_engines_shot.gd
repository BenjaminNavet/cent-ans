extends SceneTree

## SG2 : captures des engins animés (trébuchet à mi-tir, séquence du tir, mangonneau, bombarde),
## du bélier à la porte, de l'huile bouillante et des impacts sur la muraille. La bataille avance
## vite jusqu'au moment voulu, puis la scène est mise en pause pour la capture (les poses sont
## celles de l'instant, calées sur les tirs du cœur).
## Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --script res://tests/sg2_engines_shot.gd -- \
##     --out=<dossier> [--landmark=avignon --attacker=fac_england] \
##     [--engines=unit_trebuchet,unit_mangonel,unit_bombard,unit_siege_tower] [--prefix=sg2] [--only=treb,ram,oil,marks,demo]

var _out := ""
var _prefix := "sg2"
var _landmark := ""
var _attacker := "fac_france"
var _engines := "unit_trebuchet,unit_bombard,unit_mangonel,unit_siege_tower"
var _only: PackedStringArray = []
var scene: Node
var battle: Object


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--landmark="):
			_landmark = arg.trim_prefix("--landmark=")
		elif arg.begins_with("--attacker="):
			_attacker = arg.trim_prefix("--attacker=")
		elif arg.begins_with("--engines="):
			_engines = arg.trim_prefix("--engines=")
		elif arg.begins_with("--only="):
			_only = arg.trim_prefix("--only=").split(",", false)
	if _out == "":
		push_error("sg2_engines_shot: --out=<dossier> required")
		quit(1)
		return
	DirAccess.make_dir_recursive_absolute(_out)
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index := -1
	if _landmark != "":
		index = sim.call("debug_stage_landmark_siege", armies[1] if _attacker == "fac_england" else armies[0], _landmark)
	else:
		index = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	if index < 0:
		push_error("sg2_engines_shot: cannot stage the siege")
		quit(1)
		return
	BattleScene.demo_args = PackedStringArray(["--siege-engines=" + _engines, "--no-speech"])
	scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 7)
	root.add_child(scene)
	for _i in 30:
		await process_frame
	battle = scene.battle
	if scene.deployment != null and scene.deployment.active:
		scene.deployment.finish()
	if battle.call("is_deploying"):
		battle.call("start_battle")
	battle.call("set_ai", "attacker", true)
	battle.call("set_ai", "defender", true)
	scene.camera_rig.edge_pan_enabled = false
	var engines: SiegeEnginesFx = scene.engines_fx
	var fx: SiegeAssaultFx = scene.assault_fx
	if engines == null or fx == null:
		push_error("sg2_engines_shot: no engine or assault fx")
		quit(1)
		return
	if _landmark != "":
		await _demo_overview()
	if _wants("treb"):
		await _engine_throw(engines, "unit_trebuchet", 0.55, "trebuchet_mi_tir")
		await _engine_sequence(engines, "unit_trebuchet", "trebuchet_seq")
	if _wants("mangonel"):
		await _engine_throw(engines, "unit_mangonel", 0.22, "mangonneau_tir")
	if _wants("bombard"):
		await _engine_throw(engines, "unit_bombard", 0.05, "bombarde_tir")
	if _wants("marks"):
		await _wall_marks(fx)
	if _wants("ram"):
		await _ram_at_gate(fx)
	if _wants("oil"):
		await _oil(fx)
	print("sg2_engines_shot: done at %.0f s, %d swings (%d anticipated), %d shots, %d ram blows, %d oil, %d marks" % [float(battle.call("get_elapsed")), engines.swings_started, engines.predicted_swings, fx.shots_seen, fx.strikes_seen, fx.oils_seen, fx.marks.mark_count()])
	quit(0)


func _wants(key: String) -> bool:
	return _only.is_empty() or _only.has(key)


func _unit_of(type: String) -> Dictionary:
	for unit in battle.call("get_units"):
		if str(unit["type"]) == type and bool(unit["present"]):
			return unit
	return {}


## Avance jusqu'à ce que la figurine 0 de l'engin `type` soit à `at` s de son basculement, puis
## capture de profil.
func _engine_throw(engines: SiegeEnginesFx, type: String, at: float, name: String) -> void:
	var limit := float(battle.call("get_elapsed")) + 240.0
	while float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 0.05)
		var unit := _unit_of(type)
		if unit.is_empty():
			continue
		var entry: Dictionary = engines._engines.get(int(unit["id"]), {})
		if entry.is_empty() or (entry["figures"] as Array).is_empty():
			continue
		var k := _end_figure(entry, unit)
		var tau := engines.time_now - float(entry["swing"]) - float(engines._kind_cfg(str(entry["model"])).get("stagger_s", 0.3)) * k
		if tau >= at and tau < at + 0.08:
			var node: Node3D = entry["figures"][k]
			var facing := float(unit["facing"])
			scene.paused = true
			var focus := node.global_position
			print("sg2_engines_shot: %s at %s (unit %.0f, %.0f), tau %.2f" % [type, str(focus), float(unit["x"]), float(unit["z"]), tau])
			focus.y = 0.0
			scene.camera_rig.look_at_point(focus, 32.0 if type == "unit_trebuchet" else 16.0, facing + PI * 0.5 + 0.35)
			await _frames(4)
			await _shot(name + ".png")
			scene.paused = false
			return
	print("sg2_engines_shot: no %s throw caught" % type)


## Six images du tir du trébuchet (0,25 s d'écart), caméra fixe de profil.
func _engine_sequence(engines: SiegeEnginesFx, type: String, prefix: String) -> void:
	var limit := float(battle.call("get_elapsed")) + 60.0
	while float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 0.05)
		var unit := _unit_of(type)
		if unit.is_empty():
			continue
		var entry: Dictionary = engines._engines.get(int(unit["id"]), {})
		if entry.is_empty() or (entry["figures"] as Array).is_empty():
			continue
		var k := _end_figure(entry, unit)
		var tau := engines.time_now - float(entry["swing"]) - float(engines._kind_cfg(str(entry["model"])).get("stagger_s", 0.3)) * k
		if tau >= 0.0 and tau < 0.06:
			var node: Node3D = entry["figures"][k]
			scene.paused = true
			var focus := node.global_position
			focus.y = 0.0
			scene.camera_rig.look_at_point(focus, 34.0, float(unit["facing"]) + PI * 0.5 + 0.25)
			for i in 6:
				await _frames(3)
				await _shot("%s_%d.png" % [prefix, i])
				scene._fast_forward(float(battle.call("get_elapsed")) + 0.3)
			scene.paused = false
			return


## Rang de la figurine d'engin au bout gauche de la ligne (la caméra de profil s'y place).
func _end_figure(entry: Dictionary, unit: Dictionary) -> int:
	var facing := float(unit["facing"])
	var right := Vector3(cos(facing), 0.0, -sin(facing))
	var best := 0
	var best_d := INF
	var figures: Array = entry["figures"]
	for i in figures.size():
		var node: Node3D = figures[i]
		if node.visible and -node.global_position.dot(right) < best_d:
			best_d = -node.global_position.dot(right)
			best = i
	return best


func _wall_marks(fx: SiegeAssaultFx) -> void:
	var limit := float(battle.call("get_elapsed")) + 200.0
	while fx.marks.mark_count() < 6 and float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 1.0)
	if fx.marks.mark_count() == 0:
		print("sg2_engines_shot: no impact mark")
		return
	var mark: Decal = fx.marks._marks[fx.marks._marks.size() - 1]["node"]
	var out := mark.global_basis.y
	scene.paused = true
	scene.camera_rig.look_at_point(mark.global_position * Vector3(1, 0, 1), 38.0, atan2(out.x, out.z) + 0.4)
	await _frames(6)
	await _shot("impacts_muraille.png")
	scene.paused = false


func _ram_at_gate(fx: SiegeAssaultFx) -> void:
	var limit := float(battle.call("get_elapsed")) + 900.0
	while fx.strikes_seen < 2 and float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 0.5)
	if fx.strikes_seen < 2:
		print("sg2_engines_shot: the ram never struck")
		return
	var siege: Dictionary = battle.call("get_siege")
	var gate: Dictionary = siege["pieces"][int(siege["gate"])]
	var mid: Vector2 = ((gate["a"] as Vector2) + (gate["b"] as Vector2)) * 0.5
	var out := fx._outward(gate)
	# Poutre ramenée en arrière (0,6 période après un coup).
	var strikes := fx.strikes_seen
	while fx.strikes_seen == strikes and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 0.1)
	scene._fast_forward(float(battle.call("get_elapsed")) + fx.ram_period * 0.6)
	scene.paused = true
	var focus := Vector3(mid.x, 0.0, mid.y) + out * 7.0
	scene.camera_rig.look_at_point(focus, 22.0, atan2(out.x, out.z) + 1.2)
	await _frames(6)
	await _shot("belier_porte.png")
	scene.paused = false


func _oil(fx: SiegeAssaultFx) -> void:
	var limit := float(battle.call("get_elapsed")) + 900.0
	var oils := fx.oils_seen
	while fx.oils_seen == oils and float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 0.1)
	if fx.oils_seen == oils:
		print("sg2_engines_shot: no boiling oil")
		return
	var siege: Dictionary = battle.call("get_siege")
	var gate: Dictionary = siege["pieces"][int(siege["gate"])]
	var mid: Vector2 = ((gate["a"] as Vector2) + (gate["b"] as Vector2)) * 0.5
	var out := fx._outward(gate)
	var yaw := atan2(out.x, out.z) + 0.75
	scene.camera_rig.look_at_point(Vector3(mid.x, 0.0, mid.y) + out * 6.0, 30.0, yaw)
	# Temps réel : la coulée (0,5 s), puis la vapeur (1,6 s).
	await _wait(0.5)
	await _shot("huile_coulee.png")
	await _wait(1.3)
	await _shot("huile_vapeur.png")


func _demo_overview() -> void:
	var siege: Dictionary = battle.call("get_siege")
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	scene.camera_rig.look_at_point(Vector3(center.x, 0.0, center.y - 150.0), 330.0, PI + 0.35)
	await _wait(1.5)
	await _shot("vue_ensemble.png")


func _frames(n: int) -> void:
	for _i in n:
		await process_frame


func _wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		await process_frame


func _shot(file: String) -> void:
	await RenderingServer.frame_post_draw
	var img := root.get_viewport().get_texture().get_image()
	var path := _out.path_join("%s_%s" % [_prefix, file])
	img.save_png(path)
	print("sg2_engines_shot: ", path)
