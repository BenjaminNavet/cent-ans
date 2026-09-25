extends SceneTree

## SG1 : captures d'un assaut de siège (Guyenne, `debug_stage_siege`), IA des deux camps, avec des
## engins ajoutés à l'assiégeant (`--siege-engines=`). La bataille avance vite jusqu'à chaque
## moment attendu, puis tourne en temps réel une seconde pour que les animations jouent :
## tir d'engin en vol, bélier à la porte, échelles, beffroi accosté, huile, porte enfoncée, place.
## Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --script res://tests/sg1_siege_shot.gd -- \
##     --out=<dossier> --siege-engines=unit_trebuchet,unit_bombard,unit_siege_tower [--units=10] [--seq]
## (`--units=10` : garnison étoffée, pour voir le repli sur la place ; l'escalade n'a alors
## souvent pas lieu, la porte tombant d'abord).
## `--seq` : ajoute une séquence de 6 images du bélier et de l'escalade (0,25 s d'écart).

var _out := ""
var _seq := false
var scene: Node
var battle: Object


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg == "--seq":
			_seq = true
	if _out == "":
		push_error("sg1_siege_shot: --out=<dossier> required")
		quit(1)
		return
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
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
	var fx: SiegeAssaultFx = scene.assault_fx
	if fx == null:
		push_error("sg1_siege_shot: no assault fx")
		quit(1)
		return
	var pending := ["engin", "belier", "echelles", "beffroi", "huile", "porte"]
	var oils := 0
	var last_oil := -1.0
	while not pending.is_empty() and float(battle.call("get_elapsed")) < 1500.0 and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 0.5)
		var units: Array = battle.call("get_units")
		var siege: Dictionary = battle.call("get_siege")
		if fx.oils_seen > oils:
			oils = fx.oils_seen
			last_oil = float(battle.call("get_elapsed"))
		if pending.has("engin") and fx.shots_seen >= 2:
			pending.erase("engin")
			await _engine_shot(fx, siege)
		elif pending.has("belier") and fx.strikes_seen >= 3 and not fx._gate_broken:
			pending.erase("belier")
			await _view(_gate_point(siege, 6.0), 26.0, _yaw_out(siege, int(siege["gate"])) + 1.25, 0.6)
			await _shot("sg1_belier.png")
			if _seq:
				await _sequence("sg1_belier_seq", 6)
		elif pending.has("echelles") and _climber(units, 0.25, 0.8) != {}:
			pending.erase("echelles")
			var unit := _climber(units, 0.25, 0.8)
			var lines: PackedVector3Array = unit["ladder_lines"]
			var mid := (lines[0] + lines[lines.size() - 2]) * 0.5
			var yaw := _yaw_out(siege, int(unit["climbing"])) + 0.55
			print("sg1_siege_shot: climber %s %s progress %.2f shown %d lines %s" % [unit["name"], unit["render"], float(unit["climb_progress"]), int(unit.get("climbers_shown", 0)), str(lines)])
			await _view(mid, 70.0, yaw, 2.5)
			await _shot("sg1_echelles_haut.png")
			await _view(mid, 38.0, yaw, 0.3)
			await _shot("sg1_echelles.png")
			await _view(lines[0].lerp(lines[1], 0.4), 16.0, yaw - 0.2, 0.3)
			await _shot("sg1_echelles_gros_plan.png")
			if _seq:
				await _sequence("sg1_escalade_seq", 6)
		elif pending.has("beffroi") and _docked_tower(units, siege) != {}:
			pending.erase("beffroi")
			var tower := _docked_tower(units, siege)
			await _view(Vector3(float(tower["x"]), 6.0, float(tower["z"])), 48.0, float(tower["facing"]) + PI * 0.5 + 0.35, 1.8)
			await _shot("sg1_beffroi.png")
		elif pending.has("huile") and last_oil > 0.0 and float(battle.call("get_elapsed")) - last_oil < 0.6:
			pending.erase("huile")
			await _view(_gate_point(siege, 6.0), 34.0, _yaw_out(siege, int(siege["gate"])) + 0.7, 0.9)
			await _shot("sg1_huile.png")
		elif pending.has("porte") and not pending.has("belier") and not fx._gate_broken and fx.strikes_seen > 0 and (not pending.has("echelles") or float(battle.call("get_elapsed")) > 600.0):
			# Accélère la chute de la porte (le bélier donne le dernier coup) : un coup de plus.
			battle.call("debug_set_piece_hp", int(siege["gate"]), 1.0)
			for _k in 40:
				scene._fast_forward(float(battle.call("get_elapsed")) + 0.1)
				if fx._gate_broken:
					break
			if fx._gate_broken:
				pending.erase("porte")
				await _view(_gate_point(siege, 8.0), 34.0, _yaw_out(siege, int(siege["gate"])) + 0.5, 0.15)
				await _sequence("sg1_porte_seq", 6)
				await _wait(3.0)
				await _shot("sg1_porte_enfoncee.png")
				var center: Vector2 = siege.get("center", Vector2(600, 560))
				scene._fast_forward(float(battle.call("get_elapsed")) + 40.0)
				var on_square := 0
				for u in battle.call("get_units"):
					if str(u["side"]) == "defender" and bool(u["present"]) and Vector2(float(u["x"]), float(u["z"])).distance_to(center) < float(siege.get("square_radius", 35.0)) + 10.0:
						on_square += 1
				print("sg1_siege_shot: %d defender regiments on the square 40 s after the gate fell" % on_square)
				await _view(Vector3(center.x, 0.0, center.y - 30.0), 120.0, 0.35, 0.8)
				await _shot("sg1_place.png")
		elif pending.has("porte") and fx._gate_broken:
			pending.erase("porte")
			await _view(_gate_point(siege, 8.0), 34.0, _yaw_out(siege, int(siege["gate"])) + 0.4, 1.2)
			await _shot("sg1_porte_enfoncee.png")
			var center: Vector2 = siege.get("center", Vector2(600, 560))
			scene._fast_forward(float(battle.call("get_elapsed")) + 25.0)
			await _view(Vector3(center.x, 0.0, center.y), 90.0, PI + 0.5, 0.5)
			await _shot("sg1_place.png")
	print("sg1_siege_shot: done at %.0f s, missing %s, %d shots, %d ram blows, %d oil" % [float(battle.call("get_elapsed")), str(pending), fx.shots_seen, fx.strikes_seen, fx.oils_seen])
	quit(0)


func _engine_shot(fx: SiegeAssaultFx, siege: Dictionary) -> void:
	# Attendre (temps réel) qu'une pierre soit à mi-course.
	var end := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < end:
		for f in fx._flights:
			var t := (fx.time_now - float(f["t0"])) / float(f["flight"])
			if t > 0.45 and t < 0.7:
				var a: Vector3 = f["start"]
				var b: Vector3 = f["end"]
				# Lacet de la caméra : elle se place à l'opposé de (sin, cos) du point visé.
				var yaw := atan2(b.x - a.x, b.z - a.z) + 0.7
				scene.camera_rig.look_at_point(b.lerp(a, 0.15), 60.0, yaw)
				await process_frame
				await process_frame
				await _shot("sg1_engin_tir.png")
				await _wait(float(f["flight"]) * 0.5)
				await _shot("sg1_engin_impact.png")
				return
		await process_frame
	print("sg1_siege_shot: no stone in flight")


func _view(focus: Vector3, distance: float, yaw: float, wait: float) -> void:
	focus.y = 0.0
	scene.camera_rig.look_at_point(focus, distance, yaw)
	await _wait(wait)


func _sequence(prefix: String, count: int) -> void:
	for i in count:
		await _wait(0.25)
		await _shot("%s_%d.png" % [prefix, i])


func _gate_point(siege: Dictionary, offset: float) -> Vector3:
	var gate: Dictionary = siege["pieces"][int(siege["gate"])]
	var a: Vector2 = gate["a"]
	var b: Vector2 = gate["b"]
	var mid := (a + b) * 0.5
	var out := _out_dir(siege, gate)
	return Vector3(mid.x + out.x * offset, 0.0, mid.y + out.y * offset)


func _out_dir(siege: Dictionary, piece: Dictionary) -> Vector2:
	var a: Vector2 = piece["a"]
	var b: Vector2 = piece["b"]
	var d := (b - a).normalized()
	var out := Vector2(d.y, -d.x)
	if out.dot((a + b) * 0.5 - (siege.get("center", Vector2(600, 560)) as Vector2)) < 0.0:
		out = -out
	return out


func _yaw_out(siege: Dictionary, piece: int) -> float:
	var out := _out_dir(siege, siege["pieces"][piece])
	return atan2(out.x, out.y)


func _climber(units: Array, lo: float, hi: float) -> Dictionary:
	for unit in units:
		if unit.has("ladder_lines") and float(unit.get("climb_progress", 0.0)) > lo and float(unit.get("climb_progress", 0.0)) < hi:
			return unit
	return {}


func _docked_tower(units: Array, siege: Dictionary) -> Dictionary:
	for piece in siege.get("pieces", []):
		var id := int(piece.get("docked_tower", -1))
		if id >= 0:
			for unit in units:
				if int(unit["id"]) == id:
					return unit
	return {}


func _wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		await process_frame


func _shot(file_name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := _out.path_join(file_name)
	DirAccess.make_dir_recursive_absolute(_out)
	print("sg1_siege_shot: %s (%s)" % [path, error_string(image.save_png(path))])
