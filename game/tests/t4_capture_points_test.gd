extends SceneTree

## T4 (TW2, ADR 0108) : points de capture d'une bataille de siège.
## 1. `SiegeCapturePoints.states` (fonction pure) : barre masquée tant que la prise n'est pas
##    entamée, « Place du marché : 24/60 s », drapeau à l'assaillant une fois le point pris.
## 2. Nœud seul : un drapeau et un cercle par point, barre de capture créée à la première
##    progression, couleur de l'assaillant, étiquette visible.
## 3. Colonne d'alertes : « La place est menacée » a un libellé et passe avant « Porte détruite ».
## 4. Intégration : siège de Guyenne (`debug_stage_siege`) — `get_siege().points` expose la place
##    et la porte ; la vue de siège plante leurs drapeaux.
##
## Usage : godot --headless --path game --script res://tests/t4_capture_points_test.gd

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_states()
	await _check_node()
	_check_alert()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("t4_capture_points_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("t4_capture_points_test: " + message)
	return condition


func _fake_siege(square_progress: float, square_status: String, gate_status: String) -> Dictionary:
	return {
		"points": [
			{"kind": "square", "x": 600.0, "z": 560.0, "radius": 35.0, "progress": square_progress, "hold_s": 60.0,
				"share": square_progress / 60.0, "status": square_status, "attackers": 200.0, "defenders": 80.0},
			{"kind": "gate", "x": 600.0, "z": 430.0, "radius": 25.0, "progress": 30.0 if gate_status == "taken" else 0.0,
				"hold_s": 30.0, "share": 1.0 if gate_status == "taken" else 0.0, "status": gate_status,
				"attackers": 0.0, "defenders": 0.0},
		],
	}


func _check_states() -> void:
	var ground := func(_x: float, _z: float) -> float: return 3.0
	var quiet := SiegeCapturePoints.states(_fake_siege(0.0, "held", "held"), ground)
	_check(quiet.has("square") and quiet.has("gate"), "both points read")
	_check(not quiet["square"]["shown"], "no capture bar while the square is quiet")
	_check(str(quiet["square"]["holder"]) == "defender", "the garrison holds the square")
	var s := SiegeCapturePoints.states(_fake_siege(24.4, "capturing", "taken"), ground)
	_check(s["square"]["shown"], "capture bar shown while the attacker takes the square")
	_check(absf(float(s["square"]["share"]) - 24.4 / 60.0) < 1e-4, "square share: %s" % s["square"]["share"])
	_check(str(s["square"]["text"]) == "Place du marché : 24/60 s", "square caption: %s" % s["square"]["text"])
	_check(bool(s["square"]["contested"]), "a point being taken is marked contested")
	_check(str(s["gate"]["holder"]) == "attacker" and str(s["gate"]["text"]) == "Porte prise", "gate taken: %s" % s["gate"])
	var world: Vector3 = s["square"]["world"]
	_check(absf(world.y - (3.0 + SiegeCapturePoints.BAR_LIFT)) < 1e-4, "bar above the flag: %s" % world)


func _check_node() -> void:
	var holder := Node3D.new()
	root.add_child(holder)
	var camera := Camera3D.new()
	holder.add_child(camera)
	camera.position = Vector3(600, 80, 700)
	camera.look_at(Vector3(600, 0, 560))
	camera.current = true
	var points := SiegeCapturePoints.new()
	points.height_at = func(_x: float, _z: float) -> float: return 0.0
	points.side_colors = {"attacker": Color(0.1, 0.2, 0.7), "defender": Color(0.7, 0.1, 0.1)}
	holder.add_child(points)
	points.sync(_fake_siege(0.0, "held", "held"))
	await process_frame
	_check(points.flag("square") != null and points.flag("gate") != null, "a flag planted on each point")
	_check(points.bar("square") == null, "no capture bar before any progress")
	var square_flag := points.flag("square")
	if square_flag != null:
		_check(square_flag.get_node_or_null("Ring") != null, "ground circle of the point")
		_check(absf(square_flag.position.x - 600.0) < 1e-3, "flag at the point")
	points.sync(_fake_siege(30.0, "capturing", "held"))
	await process_frame
	var bar: Control = points.bar("square")
	if _check(bar != null, "capture bar built once the square is being taken"):
		_check(bar.visible, "capture bar on screen")
		_check(absf(float(bar.get("ratio")) - 0.5) < 1e-4, "capture bar at 50 %%: %s" % bar.get("ratio"))
		_check((bar.get("fill") as Color).is_equal_approx(Color(0.1, 0.2, 0.7)), "attacker colour")
		_check((bar.get("caption") as Label).visible, "caption shown")
	points.sync(_fake_siege(0.0, "held", "held"))
	await process_frame
	if bar != null:
		_check(not bar.visible, "capture bar hidden once the garrison pushes back")
	holder.queue_free()
	await process_frame


func _check_alert() -> void:
	_check(str(BattleAlertsColumn.LABELS.get("square_threatened", "")) == "La place est menacée", "alert caption")
	var weights: Dictionary = BattleAlertsColumn.IMPORTANCE_FALLBACK
	_check(int(weights.get("square_threatened", 0)) > int(weights.get("gate_destroyed", 0)), "square alert before the gate")


func _check_integration() -> void:
	if not ClassDB.class_exists("CampaignSim") or not ClassDB.instantiate("CampaignSim").has_method("debug_stage_siege"):
		_check(false, "CampaignSim.debug_stage_siege not registered (run core/build.sh)")
		return
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", data_dir, "fac_france", 1337), "new_campaign"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.configure(sim, index, 7)
	root.add_child(_scene)
	for _i in 20:
		await process_frame
	var battle: Object = _scene.battle
	var siege: Dictionary = battle.call("get_siege")
	var points: Array = siege.get("points", [])
	_check(points.size() == 2, "two capture points: %d" % points.size())
	var kinds := []
	for point in points:
		kinds.append(str(point["kind"]))
		_check(float(point["hold_s"]) > 0.0 and float(point["radius"]) > 0.0, "point entry: %s" % point)
	_check(kinds.has("square") and kinds.has("gate"), "square and gate: %s" % [kinds])
	_check(float(siege.get("hold_to_win", 0.0)) > 0.0, "hold_to_win from the rules")
	var view: BattleSiege = _scene.siege_view
	if not _check(view != null and view.capture_points != null, "siege view has capture points"):
		return
	_check(view.capture_points.flag("square") != null, "square flag planted in the battle")
	_check(view.capture_points.side_colors.get("attacker") == _scene.side_colors.get("attacker"), "flags use the scene's side colours")
