extends SceneTree

## SG4 : captures d'un assaut d'Avignon par l'IA (armée anglaise, trébuchet et bombarde ajoutés,
## comme l'ancienne sonde `sg3_assault_probe` avec ENGINES=1) : le bélier à la porte et son équipage relevé,
## les échelles dressées sur plusieurs pans à la fois, l'infanterie qui entre par la porte enfoncée.
## Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --script res://tests/sg4_assault_shot.gd -- \
##     --out=<dossier> [--landmark=avignon] [--prefix=sg4]

var _out := ""
var _prefix := "sg4"
var _landmark := "avignon"
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
	if _out == "":
		push_error("sg4_assault_shot: --out=<dossier> required")
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
	var index: int = sim.call("debug_stage_landmark_siege", armies[1], _landmark)
	if index < 0:
		push_error("sg4_assault_shot: cannot stage the siege")
		quit(1)
		return
	BattleScene.demo_args = PackedStringArray(["--siege-engines=unit_trebuchet,unit_bombard", "--no-speech"])
	scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 11)
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
	var siege: Dictionary = battle.call("get_siege")
	var gate := Vector3.ZERO
	for piece in siege.get("pieces", []):
		if str(piece["kind"]) == "gate":
			var a: Vector2 = piece["a"]
			var b: Vector2 = piece["b"]
			gate = Vector3((a.x + b.x) * 0.5, 0.0, (a.y + b.y) * 0.5)
	var ram_shot := false
	var ladders_shot := false
	var storm_shot := false
	var limit := 900.0
	while float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		await _step(1.0)
		var units: Array = battle.call("get_units")
		if not ram_shot:
			for unit in units:
				if bool(unit.get("ram", false)) and bool(unit["present"]) and int(unit["soldiers"]) < 12 \
						and Vector3(float(unit["x"]), 0.0, float(unit["z"])).distance_to(gate) < 20.0:
					scene.paused = true
					scene.camera_rig.look_at_point(gate + Vector3(0.0, 0.0, -18.0), 45.0, PI + 0.5)
					await _frames(6)
					await _shot("belier_releve.png")
					scene.paused = false
					ram_shot = true
					break
		if not ladders_shot:
			var pieces := {}
			for unit in units:
				if str(unit["side"]) == "attacker" and int(unit.get("climbing", -1)) >= 0:
					pieces[int(unit["climbing"])] = true
			if pieces.size() >= 2:
				scene.paused = true
				scene.camera_rig.look_at_point(gate + Vector3(0.0, 0.0, -30.0), 150.0, PI + 0.3)
				await _frames(6)
				await _shot("echelles_plusieurs_pans.png")
				scene.paused = false
				ladders_shot = true
		if not storm_shot:
			siege = battle.call("get_siege")
			for piece in siege.get("pieces", []):
				if str(piece["kind"]) == "gate" and not bool(piece["intact"]):
					await _step(12.0)
					scene.paused = true
					scene.camera_rig.look_at_point(gate, 70.0, PI + 0.8)
					await _frames(6)
					await _shot("assaut_porte.png")
					scene.paused = false
					storm_shot = true
		if ram_shot and ladders_shot and storm_shot:
			break
	print("sg4_assault_shot: done at %.0f s (ram %s, ladders %s, storm %s)" % [float(battle.call("get_elapsed")), ram_shot, ladders_shot, storm_shot])
	quit(0)


func _step(seconds: float) -> void:
	scene._fast_forward(float(battle.call("get_elapsed")) + seconds)
	await process_frame


func _frames(n: int) -> void:
	for _i in n:
		await process_frame


func _shot(file: String) -> void:
	await RenderingServer.frame_post_draw
	var img := root.get_viewport().get_texture().get_image()
	var path := _out.path_join("%s_%s" % [_prefix, file])
	img.save_png(path)
	print("sg4_assault_shot: ", path)
