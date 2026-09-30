extends SceneTree

## Figurines sur un tablier de pont et sur un chemin de ronde (`sim/footing.rs`) : un régiment
## traverse un pont de la rivière, une garnison tient la muraille ; capture de chaque cas.
## Usage (avec affichage) :
##   godot --path game --resolution 1280x720 --script res://tests/footing_shot.gd -- \
##     --out=<dossier> --mode=bridge --river
##   godot --path game --resolution 1280x720 --script res://tests/footing_shot.gd -- \
##     --out=<dossier> --mode=wall

var _out := ""
var _mode := "bridge"
var _seed := 7
var scene: Node
var battle: Object


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--mode="):
			_mode = arg.trim_prefix("--mode=")
		elif arg.begins_with("--seed="):
			_seed = int(arg.trim_prefix("--seed="))
	if _out == "":
		push_error("footing_shot: --out=<dossier> required")
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
	var index: int
	if _mode == "wall":
		index = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	else:
		index = sim.call("debug_stage_battle", armies[0], armies[1])
	if index < 0:
		push_error("footing_shot: cannot stage the battle")
		quit(1)
		return
	BattleScene.demo_args = PackedStringArray(["--no-speech"])
	scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, _seed)
	root.add_child(scene)
	for _i in 30:
		await process_frame
	battle = scene.battle
	if scene.deployment != null and scene.deployment.active:
		scene.deployment.finish()
	if battle.call("is_deploying"):
		battle.call("start_battle")
	scene.camera_rig.edge_pan_enabled = false
	if _mode == "wall":
		await _wall()
	else:
		await _bridge()
	quit(0)


func _bridge() -> void:
	var bridges: Array = battle.call("get_terrain").get("bridges", [])
	if bridges.is_empty():
		push_error("footing_shot: no bridge (seed %d)" % _seed)
		return
	var bridge: Dictionary = bridges[0]
	var centre := Vector2(float(bridge["x"]), float(bridge["z"]))
	var yaw := float(bridge["yaw"])
	var dir := Vector2(cos(yaw), sin(yaw))
	var half := float(bridge["length"]) * 0.5
	# Le régiment à pied le plus proche du pont, envoyé de l'autre côté.
	var best := -1
	var best_d := INF
	var from := Vector2.ZERO
	for unit in battle.call("get_units"):
		if bool(unit.get("mounted", false)) or str(unit.get("category", "")) != "infantry":
			continue
		var p := Vector2(float(unit["x"]), float(unit["z"]))
		var d := p.distance_to(centre)
		if d < best_d:
			best_d = d
			best = int(unit["id"])
			from = p
	var ahead := 1.0 if (centre - from).dot(dir) >= 0.0 else -1.0
	var goal := centre + dir * ahead * (half + 60.0)
	battle.call("set_ai", "attacker", false)
	battle.call("set_ai", "defender", false)
	print("footing_shot: unit %d from %s to bridge %s, goal %s" % [best, from, centre, goal])
	var result: Dictionary = battle.call("issue_command", {"type": "move", "units": [best], "x": goal.x, "z": goal.y, "run": true})
	print("footing_shot: move ", result)
	var on_deck := false
	for _i in 6000:
		battle.call("tick", 0.1)
		var unit: Dictionary = _unit(best)
		var local := Vector2(float(unit["x"]), float(unit["z"])) - centre
		if absf(local.dot(dir)) < half * 0.5 and absf(local.dot(Vector2(-dir.y, dir.x))) < float(bridge["width"]):
			on_deck = true
			break
	print("footing_shot: on deck ", on_deck, " at t=", battle.call("get_elapsed"))
	await _frames(6)
	scene.paused = true
	var unit: Dictionary = _unit(best)
	var focus := Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
	# Caméra : direction (sin yaw, cos yaw) depuis la cible ; l'axe du pont est (cos, sin).
	var axis_yaw := atan2(dir.x, dir.y)
	_look(focus, 45.0, axis_yaw + PI * 0.5 + 0.3, 35.0)
	await _frames(8)
	await _shot("pont_oblique.png")
	_look(focus, 40.0, axis_yaw + 0.2, 30.0)
	await _frames(8)
	await _shot("pont_axe.png")


func _wall() -> void:
	battle.call("set_ai", "attacker", false)
	battle.call("set_ai", "defender", false)
	var target: Dictionary = {}
	for unit in battle.call("get_units"):
		if bool(unit.get("on_wall", false)):
			target = unit
			break
	if target.is_empty():
		push_error("footing_shot: nobody on the wall")
		return
	await _frames(6)
	scene.paused = true
	var focus := Vector3(float(target["x"]), float(target.get("y", 0.0)), float(target["z"]))
	var facing := float(target["facing"])
	_look(focus, 45.0, facing + 0.5, 35.0)
	await _frames(8)
	await _shot("muraille_dehors.png")
	_look(focus, 40.0, facing + PI - 0.5, 40.0)
	await _frames(8)
	await _shot("muraille_dedans.png")


## Cadrage à inclinaison fixe, interface masquée (seul le rendu est jugé).
func _look(focus: Vector3, distance: float, yaw: float, pitch_deg: float) -> void:
	for child in scene.get_children():
		if child is CanvasLayer:
			child.visible = false
	scene.camera_rig.look_at_point(focus, distance, yaw)
	scene.camera_rig.manual_pitch = true
	scene.camera_rig.manual_pitch_deg = pitch_deg
	scene.camera_rig._apply()


func _unit(id: int) -> Dictionary:
	for unit in battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}


func _frames(n: int) -> void:
	for _i in n:
		await process_frame


func _shot(file: String) -> void:
	await RenderingServer.frame_post_draw
	var img := root.get_viewport().get_texture().get_image()
	var path := _out.path_join("footing_%s" % file)
	img.save_png(path)
	print("footing_shot: ", path)
